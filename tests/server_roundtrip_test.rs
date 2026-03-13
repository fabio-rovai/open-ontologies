//! Round-trip test: construct OpenOntologiesServer, then exercise
//! the domain functions it delegates to (graph store, ontology service).
//! This validates that the server wiring is correct without needing
//! an actual MCP transport.

use open_ontologies::graph::GraphStore;
use open_ontologies::ontology::OntologyService;
use open_ontologies::state::StateDb;
use std::sync::Arc;

fn setup() -> (StateDb, Arc<GraphStore>) {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let db = StateDb::open(tmp.path()).unwrap();
    let graph = Arc::new(GraphStore::new());
    (db, graph)
}

const MINI_TURTLE: &str = r#"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ex: <http://example.org/test#> .

ex:Animal a owl:Class ;
    rdfs:label "Animal" .

ex:Dog a owl:Class ;
    rdfs:subClassOf ex:Animal ;
    rdfs:label "Dog" .

ex:Cat a owl:Class ;
    rdfs:subClassOf ex:Animal ;
    rdfs:label "Cat" .

ex:hasName a owl:DatatypeProperty ;
    rdfs:domain ex:Animal ;
    rdfs:range rdfs:Literal ;
    rdfs:label "has name" .
"#;

#[test]
fn validate_load_query_roundtrip() {
    let (_db, graph) = setup();

    // Validate inline
    let valid = GraphStore::validate_turtle(MINI_TURTLE);
    assert!(valid.is_ok(), "Validation failed: {:?}", valid.err());

    // Load
    let count = graph.load_turtle(MINI_TURTLE, None).unwrap();
    assert!(count > 0, "Should load triples");

    // Stats
    let stats = graph.get_stats().unwrap();
    let stats_val: serde_json::Value = serde_json::from_str(&stats).unwrap();
    assert!(stats_val["classes"].as_u64().unwrap() >= 3, "Should have at least 3 classes");

    // SPARQL query
    let result = graph.sparql_select(
        "SELECT ?c WHERE { ?c a <http://www.w3.org/2002/07/owl#Class> } ORDER BY ?c"
    ).unwrap();
    assert!(result.contains("Animal"), "Should find Animal class");
    assert!(result.contains("Dog"), "Should find Dog class");
    assert!(result.contains("Cat"), "Should find Cat class");
}

#[test]
fn lint_detects_issues() {
    // Turtle with a class missing a label — lint should flag it
    let no_label = r#"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix ex: <http://example.org/test#> .

ex:Unlabeled a owl:Class .
"#;
    let report = OntologyService::lint(no_label).unwrap();
    let val: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert!(val["issue_count"].as_u64().unwrap() > 0, "Should detect missing label");
}

#[test]
fn version_save_and_rollback() {
    let (db, graph) = setup();

    // Load initial ontology
    graph.load_turtle(MINI_TURTLE, None).unwrap();
    let initial_count = graph.triple_count();
    assert!(initial_count > 0);

    // Save version
    let save_result = OntologyService::save_version(&db, &graph, "v1");
    assert!(save_result.is_ok(), "Version save failed: {:?}", save_result.err());

    // Clear and verify empty
    graph.clear().unwrap();
    assert_eq!(graph.triple_count(), 0);

    // Rollback
    let rollback_result = OntologyService::rollback_version(&db, &graph, "v1");
    assert!(rollback_result.is_ok(), "Rollback failed: {:?}", rollback_result.err());
    assert!(graph.triple_count() > 0, "Should have triples after rollback");
}

#[test]
fn diff_detects_changes() {
    let original = MINI_TURTLE;
    let modified = r#"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ex: <http://example.org/test#> .

ex:Animal a owl:Class ;
    rdfs:label "Animal" .

ex:Dog a owl:Class ;
    rdfs:subClassOf ex:Animal ;
    rdfs:label "Dog" .

ex:Fish a owl:Class ;
    rdfs:subClassOf ex:Animal ;
    rdfs:label "Fish" .
"#;
    let diff = OntologyService::diff(original, modified).unwrap();
    let val: serde_json::Value = serde_json::from_str(&diff).unwrap();
    // Cat removed, Fish added
    assert!(val["removed_count"].as_u64().unwrap() > 0, "Should detect removals");
    assert!(val["added_count"].as_u64().unwrap() > 0, "Should detect additions");
}

#[test]
fn serialize_formats() {
    let (_db, graph) = setup();
    graph.load_turtle(MINI_TURTLE, None).unwrap();

    // N-Triples
    let nt = graph.serialize("ntriples").unwrap();
    assert!(nt.contains("Animal"), "N-Triples should contain Animal");

    // Round-trip: load N-Triples into a fresh store
    let graph2 = GraphStore::new();
    let count = graph2.load_ntriples(&nt).unwrap();
    assert!(count > 0, "Should load N-Triples back");
}
