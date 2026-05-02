use open_ontologies::graph::GraphStore;
use open_ontologies::plan::Planner;
use open_ontologies::monitor::Monitor;
use open_ontologies::state::StateDb;
use std::sync::Arc;
use tempfile::NamedTempFile;

fn setup() -> (StateDb, Arc<GraphStore>) {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    let db = StateDb::open(&path).unwrap();
    let graph = Arc::new(GraphStore::new());
    (db, graph)
}

#[test]
fn test_plan_additions_only() {
    let (db, graph) = setup();

    let new_turtle = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class ; rdfs:label "Dog" .
        ex:Cat a owl:Class ; rdfs:label "Cat" .
    "#;

    let planner = Planner::new(db, graph);
    let plan = planner.plan(new_turtle).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&plan).unwrap();

    assert!(parsed["added_classes"].as_array().unwrap().len() >= 2);
    assert_eq!(parsed["removed_classes"].as_array().unwrap().len(), 0);
    assert_eq!(parsed["risk_score"].as_str().unwrap(), "low");
}

#[test]
fn test_plan_detects_removals() {
    let (db, graph) = setup();

    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
        ex:Cat a owl:Class .
        ex:Bird a owl:Class .
    "#, None).unwrap();

    let new_turtle = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
        ex:Cat a owl:Class .
    "#;

    let planner = Planner::new(db, graph);
    let plan = planner.plan(new_turtle).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&plan).unwrap();

    let removed = parsed["removed_classes"].as_array().unwrap();
    assert!(removed.iter().any(|v| v.as_str().unwrap().contains("Bird")));
    assert!(parsed["risk_score"].as_str().unwrap() != "low");
}

#[test]
fn test_plan_detects_property_changes() {
    let (db, graph) = setup();

    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:hasName a owl:DatatypeProperty .
        ex:hasAge a owl:DatatypeProperty .
    "#, None).unwrap();

    let new_turtle = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:hasName a owl:DatatypeProperty .
        ex:hasEmail a owl:DatatypeProperty .
    "#;

    let planner = Planner::new(db, graph);
    let plan = planner.plan(new_turtle).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&plan).unwrap();

    assert!(parsed["removed_properties"].as_array().unwrap().iter().any(|v| v.as_str().unwrap().contains("hasAge")));
    assert!(parsed["added_properties"].as_array().unwrap().iter().any(|v| v.as_str().unwrap().contains("hasEmail")));
}

#[test]
fn test_plan_blast_radius_counts_triples() {
    let (db, graph) = setup();

    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Animal a owl:Class .
        ex:Dog a owl:Class ; rdfs:subClassOf ex:Animal .
        ex:Cat a owl:Class ; rdfs:subClassOf ex:Animal .
        ex:Poodle a owl:Class ; rdfs:subClassOf ex:Dog .
    "#, None).unwrap();

    // Remove Animal — everything depends on it
    let new_turtle = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
        ex:Cat a owl:Class .
        ex:Poodle a owl:Class ; rdfs:subClassOf ex:Dog .
    "#;

    let planner = Planner::new(db, graph);
    let plan = planner.plan(new_turtle).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&plan).unwrap();

    assert!(parsed["blast_radius"]["triples_affected"].as_u64().unwrap() > 0);
    assert_eq!(parsed["risk_score"].as_str().unwrap(), "high");
}

#[test]
fn test_apply_safe_mode() {
    let (db, graph) = setup();

    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#, None).unwrap();

    let new_turtle = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
        ex:Cat a owl:Class .
    "#;

    let planner = Planner::new(db.clone(), graph.clone());
    let _ = planner.plan(new_turtle).unwrap();
    let result = planner.apply("safe").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert!(parsed["ok"].as_bool().unwrap());
    // Cat should now be in the store
    let stats = graph.get_stats().unwrap();
    assert!(stats.contains("\"classes\":2") || stats.contains("\"classes\": 2"));
}

#[test]
fn test_apply_blocked_by_monitor() {
    let (db, graph) = setup();
    let monitor = Monitor::new(db.clone(), graph.clone());
    monitor.set_blocked(true);

    let planner = Planner::new(db, graph);
    let result = planner.apply("safe");
    assert!(result.is_err() || {
        let r = result.unwrap();
        r.contains("blocked")
    });
}

#[test]
fn test_migrate_generates_bridges() {
    let (db, graph) = setup();

    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:authoredBy a owl:ObjectProperty .
    "#, None).unwrap();

    let new_turtle = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:writtenBy a owl:ObjectProperty .
    "#;

    let planner = Planner::new(db, graph.clone());
    let _ = planner.plan(new_turtle).unwrap();
    let result = planner.apply("migrate").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert!(parsed["migration_triples"].as_u64().unwrap() > 0);

    // Check that equivalentProperty bridge was created
    let query = "ASK { <http://example.org/authoredBy> <http://www.w3.org/2002/07/owl#equivalentProperty> <http://example.org/writtenBy> }";
    let ask_result = graph.sparql_select(query).unwrap();
    assert!(ask_result.contains("true"));
}

#[test]
fn test_lock_prevents_plan() {
    let (db, graph) = setup();

    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Person a owl:Class .
    "#, None).unwrap();

    let planner = Planner::new(db, graph);
    planner.lock_iri("http://example.org/Person", "production");

    // Try to remove Person — should be rejected
    let new_turtle = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#;

    let plan = planner.plan(new_turtle).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&plan).unwrap();
    assert!(!parsed["locked_violations"].as_array().unwrap().is_empty());
}
