//! Vacuous conformance: shapes that matched no focus nodes.
//!
//! A SHACL run whose shapes target classes that appear nowhere in the data
//! produces `conforms: true, violation_count: 0` — byte-for-byte the report of a
//! run where every constraint was checked and passed. The caller cannot tell the
//! two apart, so a pipeline that validates before publishing gets a green light
//! on data nothing looked at.
//!
//! This is the same failure the `sh:sparql` fix (#98) pinned down from the other
//! direction: there, constraints were never executed; here, they execute against
//! an empty target set. `ShaclValidator`'s stated contract is that reporting
//! success for rules that were never run is the one failure mode it must not
//! have, so these tests pin:
//!
//!   1. shapes matching zero focus nodes report which shapes matched nothing;
//!   2. a run where nothing at all matched does not claim conformance;
//!   3. an ordinary passing run still reports `conforms: true`, with the count;
//!   4. a partial match keeps a real verdict but still names the empty shape.

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

fn store_with_people() -> Arc<GraphStore> {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:alice a ex:Person ; ex:fullName "Alice" .
    "#;
    store.load_turtle(ttl, None).unwrap();
    store
}

fn report(store: &Arc<GraphStore>, shapes: &str) -> serde_json::Value {
    serde_json::from_str(&ShaclValidator::validate(store, shapes).unwrap()).unwrap()
}

/// Shapes minted in one namespace, data in another: the exact shape of the
/// mismatch that makes generated shapes silently target nothing.
const SHAPES_WRONG_NAMESPACE: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/shapes/> .
    ex:PersonShape a sh:NodeShape ;
        sh:targetClass ex:Person ;
        sh:property [ sh:path ex:fullName ; sh:minCount 1 ] .
"#;

const SHAPES_RIGHT_NAMESPACE: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    ex:PersonShape a sh:NodeShape ;
        sh:targetClass ex:Person ;
        sh:property [ sh:path ex:fullName ; sh:minCount 1 ] .
"#;

#[test]
fn shapes_that_match_nothing_are_named_in_the_report() {
    let r = report(&store_with_people(), SHAPES_WRONG_NAMESPACE);

    assert_eq!(
        r["focus_nodes"],
        serde_json::json!(0),
        "a report that hides a zero target count cannot be distinguished from a real pass"
    );
    let unmatched = r["unmatched_shapes"]
        .as_array()
        .expect("unmatched_shapes must be present");
    assert_eq!(unmatched.len(), 1);
    assert_eq!(
        unmatched[0]["target_class"],
        "http://example.org/shapes/Person"
    );
}

#[test]
fn a_run_that_matched_nothing_does_not_claim_conformance() {
    let r = report(&store_with_people(), SHAPES_WRONG_NAMESPACE);

    assert_eq!(
        r["conforms"],
        serde_json::Value::Null,
        "nothing was evaluated, so there is no conformance verdict to give"
    );
    assert_eq!(r["violation_count"], 0);
}

#[test]
fn an_ordinary_passing_run_still_conforms() {
    let r = report(&store_with_people(), SHAPES_RIGHT_NAMESPACE);

    assert_eq!(r["conforms"], serde_json::json!(true));
    assert_eq!(r["focus_nodes"], serde_json::json!(1));
    assert!(
        r["unmatched_shapes"].as_array().unwrap().is_empty(),
        "the shape matched a focus node, so nothing is unmatched"
    );
}

#[test]
fn a_partial_match_keeps_its_verdict_and_still_names_the_empty_shape() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:PersonShape a sh:NodeShape ;
            sh:targetClass ex:Person ;
            sh:property [ sh:path ex:fullName ; sh:minCount 1 ] .
        ex:GadgetShape a sh:NodeShape ;
            sh:targetClass ex:Gadget ;
            sh:property [ sh:path ex:serial ; sh:minCount 1 ] .
    "#;
    let r = report(&store_with_people(), shapes);

    assert_eq!(
        r["conforms"],
        serde_json::json!(true),
        "one shape did evaluate, so the verdict stands"
    );
    assert_eq!(r["focus_nodes"], serde_json::json!(1));
    let unmatched = r["unmatched_shapes"].as_array().unwrap();
    assert_eq!(unmatched.len(), 1);
    assert_eq!(unmatched[0]["target_class"], "http://example.org/Gadget");
}
