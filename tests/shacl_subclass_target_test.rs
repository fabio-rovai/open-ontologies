//! `sh:targetClass` must select SHACL instances, not only direct `rdf:type` matches.
//!
//! SHACL defines a SHACL instance as a node reachable by `rdf:type` followed by zero or
//! more `rdfs:subClassOf` steps. Matching the direct type alone makes every shape that
//! targets a superclass select nothing, and the validator then reports `conforms: true`
//! having evaluated no focus nodes at all. That is the one failure mode this validator
//! is not allowed to have, and it was present until this test was written.

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

const ONTOLOGY_AND_DATA: &str = r#"
    @prefix owl:  <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix ex:   <http://example.org/> .

    ex:Assertion           a owl:Class .
    ex:IdentifierAssertion a owl:Class ; rdfs:subClassOf ex:Assertion .
    ex:StructureAssertion  a owl:Class ; rdfs:subClassOf ex:IdentifierAssertion .

    # Typed with the concrete leaf class only, which is how generated data looks.
    ex:a1 a ex:StructureAssertion ; ex:value "ok" .
    ex:a2 a ex:StructureAssertion .
"#;

const SHAPES: &str = r#"
    @prefix sh:   <http://www.w3.org/ns/shacl#> .
    @prefix ex:   <http://example.org/> .

    ex:AssertionShape a sh:NodeShape ;
        sh:targetClass ex:Assertion ;
        sh:property [ sh:path ex:value ; sh:minCount 1 ] .
"#;

fn report(data: &str, shapes: &str) -> serde_json::Value {
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(data, None).expect("load data");
    let json = ShaclValidator::validate(&graph, shapes).expect("validate");
    serde_json::from_str(&json).expect("parse report")
}

#[test]
fn target_class_reaches_instances_of_a_transitive_subclass() {
    let r = report(ONTOLOGY_AND_DATA, SHAPES);
    // ex:a2 is a StructureAssertion, therefore an IdentifierAssertion, therefore an
    // Assertion, and it has no ex:value. The shape must fire.
    assert_eq!(
        r["conforms"], serde_json::json!(false),
        "a shape targeting a superclass must evaluate instances of its subclasses: {r}"
    );
    let violations = r["violations"].as_array().expect("violations array");
    assert_eq!(violations.len(), 1, "expected exactly one violation: {r}");
    assert!(
        violations[0]["focus_node"].as_str().unwrap_or_default().ends_with("a2"),
        "the violation should be on ex:a2: {r}"
    );
}

#[test]
fn a_superclass_shape_is_not_reported_as_unmatched() {
    let r = report(ONTOLOGY_AND_DATA, SHAPES);
    let unmatched = r["unmatched_shapes"].as_array().cloned().unwrap_or_default();
    assert!(
        unmatched.is_empty(),
        "the shape has two focus nodes via subclass closure, so it is not unmatched: {r}"
    );
    assert_eq!(r["focus_nodes"], serde_json::json!(2), "report: {r}");
}

#[test]
fn direct_type_targeting_still_works() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:LeafShape a sh:NodeShape ;
            sh:targetClass ex:StructureAssertion ;
            sh:property [ sh:path ex:value ; sh:minCount 1 ] .
    "#;
    let r = report(ONTOLOGY_AND_DATA, shapes);
    assert_eq!(r["conforms"], serde_json::json!(false), "report: {r}");
    assert_eq!(r["focus_nodes"], serde_json::json!(2), "report: {r}");
}
