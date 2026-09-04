//! Counting when a focus node reaches its target class by more than one path.
//!
//! `sh:targetClass` selects SHACL instances, which is `rdf:type` followed by zero
//! or more `rdfs:subClassOf` steps. That pattern binds a focus node once per
//! distinct path, so a node typed two ways under one class, or typed at two
//! levels of one chain, is bound twice. `COUNT(?val)` over that join then counts
//! every value once per binding.
//!
//! It breaks both count constraints, in opposite and equally bad directions:
//!
//!   * `sh:maxCount` fires on data that satisfies it. 258 such violations on the
//!     investment-fund vertical, which loads a FIBO alignment supplying the extra
//!     subclass paths. Found by the differential run against pyshacl.
//!   * `sh:minCount` is hidden by the same inflation: a node with one value and
//!     two bindings counts two, so `minCount 2` passes on data that breaks it.
//!     A false clean, which is the failure this validator must not have.
//!
//! Counting distinct value nodes is the fix, and it is what pyshacl reports.

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

fn report(store: &Arc<GraphStore>, shapes: &str) -> serde_json::Value {
    serde_json::from_str(&ShaclValidator::validate(store, shapes).unwrap()).unwrap()
}

/// `ex:a` is both an `ex:Equity` and an `ex:Bond`, and both are subclasses of
/// `ex:Instrument`, so it reaches the target class twice. It carries exactly one
/// `ex:isin`.
fn store() -> Arc<GraphStore> {
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
            @prefix ex: <http://example.org/> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            ex:Equity rdfs:subClassOf ex:Instrument .
            ex:Bond   rdfs:subClassOf ex:Instrument .
            ex:a a ex:Equity, ex:Bond ; ex:isin "GB0000000001" .
        "#,
            None,
        )
        .unwrap();
    store
}

#[test]
fn max_count_does_not_fire_on_a_node_reached_by_two_paths() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Instrument ;
            sh:property [ sh:path ex:isin ; sh:maxCount 1 ] .
    "#;
    let r = report(&store(), shapes);
    assert_eq!(
        r["conforms"],
        serde_json::json!(true),
        "ex:a has one isin; counting it twice is the bug: {r}"
    );
    assert_eq!(r["violation_count"], serde_json::json!(0), "{r}");
}

#[test]
fn min_count_still_fires_on_a_node_reached_by_two_paths() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Instrument ;
            sh:property [ sh:path ex:isin ; sh:minCount 2 ] .
    "#;
    let r = report(&store(), shapes);
    assert_eq!(
        r["conforms"],
        serde_json::json!(false),
        "one value cannot satisfy minCount 2, however many times the node is bound: {r}"
    );
    assert_eq!(r["violation_count"], serde_json::json!(1), "{r}");
    assert_eq!(r["violations"][0]["focus_node"], "http://example.org/a");
}

/// Two genuinely distinct values must still break `maxCount 1`, so the fix does
/// not simply silence the constraint.
#[test]
fn max_count_still_fires_on_two_real_values() {
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
            @prefix ex: <http://example.org/> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            ex:Equity rdfs:subClassOf ex:Instrument .
            ex:Bond   rdfs:subClassOf ex:Instrument .
            ex:a a ex:Equity, ex:Bond ; ex:isin "GB0000000001", "GB0000000002" .
        "#,
            None,
        )
        .unwrap();
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Instrument ;
            sh:property [ sh:path ex:isin ; sh:maxCount 1 ] .
    "#;
    let r = report(&store, shapes);
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], serde_json::json!(1), "{r}");
}
