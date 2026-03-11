use open_ontologies::enforce::Enforcer;
use open_ontologies::graph::GraphStore;
use open_ontologies::state::StateDb;
use std::sync::Arc;
use tempfile::NamedTempFile;

fn setup_with_ontology(ttl: &str) -> (StateDb, Arc<GraphStore>) {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    let db = StateDb::open(&path).unwrap();
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(ttl, None).unwrap();
    (db, graph)
}

#[test]
fn test_enforce_generic_orphan_class() {
    let (db, graph) = setup_with_ontology(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Animal a owl:Class .
        ex:Dog a owl:Class ; rdfs:subClassOf ex:Animal .
        ex:OrphanClass a owl:Class .
    "#);

    let enforcer = Enforcer::new(db, graph);
    let result = enforcer.enforce("generic").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let violations = parsed["violations"].as_array().unwrap();
    assert!(violations.iter().any(|v| {
        v["rule"].as_str().unwrap() == "orphan_class"
            && v["entity"].as_str().unwrap().contains("OrphanClass")
    }));
}

#[test]
fn test_enforce_generic_missing_domain_range() {
    let (db, graph) = setup_with_ontology(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:hasName a owl:DatatypeProperty .
    "#);

    let enforcer = Enforcer::new(db, graph);
    let result = enforcer.enforce("generic").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let violations = parsed["violations"].as_array().unwrap();
    assert!(violations.iter().any(|v| v["rule"].as_str().unwrap() == "missing_domain"));
}

#[test]
fn test_enforce_boro_missing_state_class() {
    let (db, graph) = setup_with_ontology(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ies: <http://ies.data.gov.uk/ontology/ies4#> .
        @prefix ex: <http://example.org/> .
        ex:Building a owl:Class ; rdfs:subClassOf ies:Entity .
    "#);

    let enforcer = Enforcer::new(db, graph);
    let result = enforcer.enforce("boro").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let violations = parsed["violations"].as_array().unwrap();
    assert!(violations.iter().any(|v| {
        v["rule"].as_str().unwrap() == "missing_state_class"
            && v["entity"].as_str().unwrap().contains("Building")
    }));
}

#[test]
fn test_enforce_boro_passes_with_state() {
    let (db, graph) = setup_with_ontology(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ies: <http://ies.data.gov.uk/ontology/ies4#> .
        @prefix ex: <http://example.org/> .
        ex:Building a owl:Class ; rdfs:subClassOf ies:Entity .
        ex:BuildingState a owl:Class ; rdfs:subClassOf ies:State, ex:Building .
    "#);

    let enforcer = Enforcer::new(db, graph);
    let result = enforcer.enforce("boro").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let violations = parsed["violations"].as_array().unwrap();
    assert!(!violations.iter().any(|v| {
        v["rule"].as_str().unwrap() == "missing_state_class"
            && v["entity"].as_str().unwrap().contains("Building")
    }));
}

#[test]
fn test_enforce_value_partition_incomplete() {
    let (db, graph) = setup_with_ontology(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Spiciness a owl:Class .
        ex:Hot a owl:Class ; rdfs:subClassOf ex:Spiciness .
        ex:Medium a owl:Class ; rdfs:subClassOf ex:Spiciness .
        ex:Mild a owl:Class ; rdfs:subClassOf ex:Spiciness .
    "#);

    let enforcer = Enforcer::new(db, graph);
    let result = enforcer.enforce("value_partition").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let violations = parsed["violations"].as_array().unwrap();
    assert!(violations.iter().any(|v| v["rule"].as_str().unwrap() == "partition_not_disjoint"));
}

#[test]
fn test_enforce_value_partition_passes() {
    let (db, graph) = setup_with_ontology(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Spiciness a owl:Class .
        ex:Hot a owl:Class ; rdfs:subClassOf ex:Spiciness ; owl:disjointWith ex:Medium, ex:Mild .
        ex:Medium a owl:Class ; rdfs:subClassOf ex:Spiciness ; owl:disjointWith ex:Hot, ex:Mild .
        ex:Mild a owl:Class ; rdfs:subClassOf ex:Spiciness ; owl:disjointWith ex:Hot, ex:Medium .
        ex:Spiciness owl:equivalentClass [ a owl:Class ; owl:unionOf ( ex:Hot ex:Medium ex:Mild ) ] .
    "#);

    let enforcer = Enforcer::new(db, graph);
    let result = enforcer.enforce("value_partition").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let violations = parsed["violations"].as_array().unwrap();
    assert!(violations.is_empty() || !violations.iter().any(|v|
        v["rule"].as_str().unwrap() == "partition_not_disjoint"
            && v["entity"].as_str().unwrap().contains("Spiciness")
    ));
}

#[test]
fn test_enforce_custom_sparql_rule() {
    let (db, graph) = setup_with_ontology(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Drug a owl:Class .
        ex:aspirin a ex:Drug .
    "#);

    let enforcer = Enforcer::new(db.clone(), graph);

    enforcer.add_custom_rule(
        "drug_indication",
        "custom",
        "ASK { ?d a <http://example.org/Drug> . FILTER NOT EXISTS { ?d <http://example.org/hasIndication> ?i } }",
        "error",
        "Drug without indication",
    );

    let result = enforcer.enforce("custom").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let violations = parsed["violations"].as_array().unwrap();
    assert!(violations.iter().any(|v| v["rule"].as_str().unwrap() == "drug_indication"));
}

#[test]
fn test_enforce_compliance_score() {
    let (db, graph) = setup_with_ontology(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class ; rdfs:label "Dog" .
        ex:Cat a owl:Class .
    "#);

    let enforcer = Enforcer::new(db, graph);
    let result = enforcer.enforce("generic").unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let score = parsed["compliance"].as_f64().unwrap();
    assert!(score >= 0.0 && score <= 1.0);
}
