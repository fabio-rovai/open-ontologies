//! `sh:not`, the constraint that zeroed 198 real violations.
//!
//! In the Scottish land register build a layer-2 shapes graph expressed its
//! rule as `sh:not [ sh:hasValue ... ]`. The validator collected every other
//! constraint on that property shape, never collected this one, and returned a
//! clean report over data that broke the rule. The skip complement later made
//! that visible — `conforms` became null with `sh:not` named in
//! `skipped_constraints` — which is honest, but a shapes graph written to the
//! specification still gets no verdict.
//!
//! These pin `sh:not` over the nested constraint forms the validator already
//! evaluates in their positive sense, and pin that a nested form it cannot
//! evaluate still reaches `skipped_constraints` rather than passing quietly.

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

fn report(store: &Arc<GraphStore>, shapes: &str) -> serde_json::Value {
    serde_json::from_str(&ShaclValidator::validate(store, shapes).unwrap()).unwrap()
}

fn store() -> Arc<GraphStore> {
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
            @prefix ex: <http://example.org/> .
            @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
            ex:a a ex:Thing ; ex:mech ex:bad ; ex:code "ABC" ; ex:n "7"^^xsd:integer .
            ex:b a ex:Thing ; ex:mech ex:good ; ex:code "123" ; ex:n "x" .
        "#,
            None,
        )
        .unwrap();
    store
}

#[test]
fn not_has_value_reports_the_node_that_carries_the_forbidden_value() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:mech ; sh:not [ sh:hasValue ex:bad ] ;
                          sh:message "must not be bad" ] .
    "#;
    let r = report(&store(), shapes);
    assert_eq!(
        r["conforms"],
        serde_json::json!(false),
        "pyshacl reports conforms=False, violations=1 for this case"
    );
    assert_eq!(r["violation_count"], serde_json::json!(1));
    assert_eq!(r["violations"][0]["focus_node"], "http://example.org/a");
    assert_eq!(r["violations"][0]["constraint"], "not");
    assert_eq!(r["violations"][0]["message"], "must not be bad");
}

/// The positive form of the same shape is already evaluated correctly. Pinning
/// both directions is what isolates a fault to `sh:not` rather than to the
/// nested constraint.
#[test]
fn the_positive_form_of_the_same_constraint_is_unchanged() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:mech ; sh:hasValue ex:bad ] .
    "#;
    let r = report(&store(), shapes);
    assert_eq!(r["conforms"], serde_json::json!(false));
    assert_eq!(r["violation_count"], serde_json::json!(1));
    assert_eq!(r["violations"][0]["focus_node"], "http://example.org/b");
}

#[test]
fn not_datatype_reports_the_value_of_the_forbidden_type() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:n ; sh:not [ sh:datatype xsd:integer ] ] .
    "#;
    let r = report(&store(), shapes);
    assert_eq!(r["conforms"], serde_json::json!(false));
    assert_eq!(r["violation_count"], serde_json::json!(1));
    assert_eq!(r["violations"][0]["focus_node"], "http://example.org/a");
}

#[test]
fn not_pattern_reports_the_value_that_matches() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:code ; sh:not [ sh:pattern "^[0-9]+$" ] ] .
    "#;
    let r = report(&store(), shapes);
    assert_eq!(r["conforms"], serde_json::json!(false));
    assert_eq!(r["violation_count"], serde_json::json!(1));
    assert_eq!(r["violations"][0]["focus_node"], "http://example.org/b");
}

/// Data that satisfies the negation must come back clean, not merely
/// violation-free with a suppressed verdict.
#[test]
fn a_satisfied_negation_conforms() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:mech ; sh:not [ sh:hasValue ex:absent ] ] .
    "#;
    let r = report(&store(), shapes);
    assert_eq!(r["conforms"], serde_json::json!(true));
    assert_eq!(r["violation_count"], serde_json::json!(0));
}

/// `sh:not` is no longer wholesale unimplemented, but a nested form the
/// validator cannot evaluate must still suppress the verdict. Implementing the
/// common cases must not convert an unevaluated rule into a silent pass.
#[test]
fn a_nested_form_that_cannot_be_evaluated_still_reaches_skipped() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:mech ; sh:not [ sh:closed true ] ] .
    "#;
    let r = report(&store(), shapes);
    assert!(
        r["conforms"].is_null(),
        "an unevaluated negation must not report a verdict, got {}",
        r["conforms"]
    );
    let skipped = r["skipped_constraints"].as_array().cloned().unwrap_or_default();
    assert!(
        !skipped.is_empty(),
        "the unevaluated nested form must be named in skipped_constraints"
    );
}
