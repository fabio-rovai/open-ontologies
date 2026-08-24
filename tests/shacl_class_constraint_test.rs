//! `sh:class` must be evaluated, not skipped.
//!
//! Until this test was written the constraint was collected by no query, evaluated by no
//! code, and reported only as "constraint not implemented", which suppressed the verdict
//! to null. A caller checking the verdict got neither a pass nor a fail.

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

const DATA: &str = r#"
    @prefix owl:  <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix ex:   <http://example.org/> .

    ex:Person a owl:Class .
    ex:Address a owl:Class .
    ex:UkAddress a owl:Class ; rdfs:subClassOf ex:Address .

    ex:good  a ex:Person ; ex:addr ex:addr1 .
    ex:sub   a ex:Person ; ex:addr ex:addr2 .
    ex:wrong a ex:Person ; ex:addr ex:notAnAddress .
    ex:lit   a ex:Person ; ex:addr "12 High Street" .

    ex:addr1 a ex:Address .
    ex:addr2 a ex:UkAddress .
    ex:notAnAddress a ex:Person .
"#;

const SHAPES: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    ex:PersonShape a sh:NodeShape ;
        sh:targetClass ex:Person ;
        sh:property [ sh:path ex:addr ; sh:class ex:Address ] .
"#;

#[test]
fn class_constraint_is_evaluated_and_the_verdict_is_a_boolean() {
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(DATA, None).expect("load");
    let json = ShaclValidator::validate(&graph, SHAPES).expect("validate");
    let r: serde_json::Value = serde_json::from_str(&json).expect("parse");

    assert!(r["conforms"].is_boolean(), "verdict must not be null: {r}");
    assert_eq!(r["conforms"], serde_json::json!(false), "report: {r}");

    let skipped = r["skipped_constraints"].as_array().cloned().unwrap_or_default();
    assert!(
        !skipped.iter().any(|s| s["constraint"].as_str().unwrap_or_default().ends_with("#class")),
        "sh:class must no longer be reported as unimplemented: {r}"
    );

    let focus: Vec<String> = r["violations"].as_array().expect("violations")
        .iter()
        .filter(|v| v["constraint"] == serde_json::json!("class"))
        .map(|v| v["focus_node"].as_str().unwrap_or_default().to_string())
        .collect();

    // ex:wrong points at a Person, and ex:lit points at a literal, which is never a
    // SHACL instance of anything. ex:sub points at a UkAddress, which is an Address by
    // subclass closure and must pass.
    assert!(focus.iter().any(|f| f.ends_with("wrong")), "ex:wrong should fail: {focus:?}");
    assert!(focus.iter().any(|f| f.ends_with("lit")), "a literal is never a SHACL instance: {focus:?}");
    assert!(!focus.iter().any(|f| f.ends_with("sub")), "a subclass instance must pass: {focus:?}");
    assert!(!focus.iter().any(|f| f.ends_with("good")), "a direct instance must pass: {focus:?}");
}
