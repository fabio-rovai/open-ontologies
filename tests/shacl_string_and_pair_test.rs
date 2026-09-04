//! `sh:minLength`, `sh:maxLength`, `sh:lessThan`, `sh:lessThanOrEquals`.
//!
//! Chosen by census rather than by taste: across 88 shapes files in 39 of this
//! machine's vertical repositories, `sh:minLength` is the most-used constraint
//! the validator did not evaluate (23 occurrences). Every shapes graph carrying
//! one was getting `conforms: null` and no verdict at all.
//!
//! Every expectation here was checked against pyshacl on the same input.

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
            ex:short  a ex:Thing ; ex:code "AB"    ; ex:low "3"^^xsd:integer ; ex:high "9"^^xsd:integer .
            ex:exact  a ex:Thing ; ex:code "ABC"   ; ex:low "5"^^xsd:integer ; ex:high "5"^^xsd:integer .
            ex:long   a ex:Thing ; ex:code "ABCDE" ; ex:low "8"^^xsd:integer ; ex:high "2"^^xsd:integer .
        "#,
            None,
        )
        .unwrap();
    store
}

#[test]
fn min_length_reports_only_the_value_below_the_bound() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:code ; sh:minLength 3 ; sh:message "too short" ] .
    "#;
    let r = report(&store(), shapes);
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], serde_json::json!(1), "{r}");
    assert_eq!(r["violations"][0]["focus_node"], "http://example.org/short");
    assert_eq!(r["violations"][0]["constraint"], "minLength");
    assert_eq!(r["violations"][0]["message"], "too short");
}

#[test]
fn max_length_reports_only_the_value_above_the_bound() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:code ; sh:maxLength 3 ] .
    "#;
    let r = report(&store(), shapes);
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], serde_json::json!(1), "{r}");
    assert_eq!(r["violations"][0]["focus_node"], "http://example.org/long");
}

/// The bound is inclusive at both ends, so the exact-length value passes both.
#[test]
fn a_value_exactly_on_both_bounds_conforms() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:code ; sh:minLength 2 ; sh:maxLength 5 ] .
    "#;
    let r = report(&store(), shapes);
    assert_eq!(r["conforms"], serde_json::json!(true), "{r}");
}

#[test]
fn less_than_compares_two_properties_of_the_same_focus_node() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:low ; sh:lessThan ex:high ; sh:message "low must be under high" ] .
    "#;
    let r = report(&store(), shapes);
    // ex:short 3<9 passes; ex:exact 5<5 fails; ex:long 8<2 fails.
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], serde_json::json!(2), "{r}");
    let nodes: Vec<_> = r["violations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["focus_node"].as_str().unwrap().to_string())
        .collect();
    assert!(nodes.contains(&"http://example.org/exact".to_string()), "{r}");
    assert!(nodes.contains(&"http://example.org/long".to_string()), "{r}");
}

#[test]
fn less_than_or_equals_admits_the_equal_case() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:low ; sh:lessThanOrEquals ex:high ] .
    "#;
    let r = report(&store(), shapes);
    // Only ex:long (8 <= 2) fails now; ex:exact (5 <= 5) is admitted.
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], serde_json::json!(1), "{r}");
    assert_eq!(r["violations"][0]["focus_node"], "http://example.org/long");
}

/// Implementing these must remove them from `skipped_constraints`. A stale skip
/// suppresses the verdict of a run in which nothing was missed, and a null that
/// fires on a complete run teaches the reader to ignore null.
#[test]
fn the_implemented_constraints_no_longer_suppress_the_verdict() {
    for constraint in [
        "sh:minLength 2",
        "sh:maxLength 9",
        "sh:lessThanOrEquals ex:high",
    ] {
        let shapes = format!(
            r#"
            @prefix sh: <http://www.w3.org/ns/shacl#> .
            @prefix ex: <http://example.org/> .
            ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
                sh:property [ sh:path ex:low ; {constraint} ] .
        "#
        );
        let r = report(&store(), &shapes);
        assert!(
            !r["conforms"].is_null(),
            "{constraint} must yield a verdict: {r}"
        );
    }
}
