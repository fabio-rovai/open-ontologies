//! `sh:qualifiedValueShape` with `sh:qualifiedMinCount` / `sh:qualifiedMaxCount`.
//!
//! Modelled on the investment-fund vertical, which is where the corpus run found
//! these unevaluated. Its `ifo:FundShape` carries two qualified constraints on the
//! SAME path: `ifo:identifiedBy` must hold exactly one SEC series identifier, and
//! should hold at least one LEI. That shape is why these are collected as
//! independent entries instead of keyed by path the way `sh:or` and `sh:in` are —
//! keying by path merges two different rules into one and silently drops a bound.

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

fn report(store: &Arc<GraphStore>, shapes: &str) -> serde_json::Value {
    serde_json::from_str(&ShaclValidator::validate(store, shapes).unwrap()).unwrap()
}

/// ex:ok has one series id and one LEI. ex:noLei has a series id but no LEI.
/// ex:twoSeries has two series ids, breaking the maximum.
fn store() -> Arc<GraphStore> {
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
            @prefix ex: <http://example.org/> .
            ex:s1 a ex:SeriesId . ex:s2 a ex:SeriesId . ex:s3 a ex:SeriesId .
            ex:l1 a ex:Lei .
            ex:ok        a ex:Fund ; ex:identifiedBy ex:s1, ex:l1 .
            ex:noLei     a ex:Fund ; ex:identifiedBy ex:s2 .
            ex:twoSeries a ex:Fund ; ex:identifiedBy ex:s3, ex:s1, ex:l1 .
        "#,
            None,
        )
        .unwrap();
    store
}

#[test]
fn qualified_min_count_reports_the_node_below_the_bound() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Fund ;
            sh:property [
                sh:path ex:identifiedBy ;
                sh:qualifiedValueShape [ sh:class ex:Lei ] ;
                sh:qualifiedMinCount 1 ;
                sh:message "no LEI" ;
            ] .
    "#;
    let r = report(&store(), shapes);
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], serde_json::json!(1), "{r}");
    assert_eq!(r["violations"][0]["focus_node"], "http://example.org/noLei");
    assert_eq!(r["violations"][0]["constraint"], "qualifiedMinCount");
    assert_eq!(r["violations"][0]["message"], "no LEI");
}

#[test]
fn qualified_max_count_reports_the_node_above_the_bound() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Fund ;
            sh:property [
                sh:path ex:identifiedBy ;
                sh:qualifiedValueShape [ sh:class ex:SeriesId ] ;
                sh:qualifiedMaxCount 1 ;
            ] .
    "#;
    let r = report(&store(), shapes);
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], serde_json::json!(1), "{r}");
    assert_eq!(
        r["violations"][0]["focus_node"],
        "http://example.org/twoSeries"
    );
    assert_eq!(r["violations"][0]["constraint"], "qualifiedMaxCount");
}

/// The shape that motivated the design: two qualified constraints, same path,
/// different nested shapes and different bounds. Both must be evaluated, and
/// neither may absorb the other.
#[test]
fn two_qualified_constraints_on_one_path_are_both_evaluated() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Fund ;
            sh:property [
                sh:path ex:identifiedBy ;
                sh:qualifiedValueShape [ sh:class ex:SeriesId ] ;
                sh:qualifiedMinCount 1 ; sh:qualifiedMaxCount 1 ;
                sh:message "exactly one series id" ;
            ] ;
            sh:property [
                sh:path ex:identifiedBy ;
                sh:qualifiedValueShape [ sh:class ex:Lei ] ;
                sh:qualifiedMinCount 1 ;
                sh:message "at least one LEI" ;
            ] .
    "#;
    let r = report(&store(), shapes);
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    let msgs: Vec<String> = r["violations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["message"].as_str().unwrap_or_default().to_string())
        .collect();
    assert!(
        msgs.contains(&"at least one LEI".to_string()),
        "the LEI rule must fire for ex:noLei: {r}"
    );
    assert!(
        msgs.contains(&"exactly one series id".to_string()),
        "the series-id rule must fire for ex:twoSeries: {r}"
    );
    assert_eq!(r["violation_count"], serde_json::json!(2), "{r}");
}

#[test]
fn data_satisfying_both_bounds_conforms() {
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
            @prefix ex: <http://example.org/> .
            ex:s1 a ex:SeriesId . ex:l1 a ex:Lei .
            ex:ok a ex:Fund ; ex:identifiedBy ex:s1, ex:l1 .
        "#,
            None,
        )
        .unwrap();
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Fund ;
            sh:property [
                sh:path ex:identifiedBy ;
                sh:qualifiedValueShape [ sh:class ex:SeriesId ] ;
                sh:qualifiedMinCount 1 ; sh:qualifiedMaxCount 1 ;
            ] .
    "#;
    let r = report(&store, shapes);
    assert_eq!(r["conforms"], serde_json::json!(true), "{r}");
}

/// A nested shape form that cannot be turned into a filter must suppress the
/// verdict rather than count zero matches and report a spurious minimum breach.
#[test]
fn an_unsupported_nested_shape_is_recorded_as_skipped() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Fund ;
            sh:property [
                sh:path ex:identifiedBy ;
                sh:qualifiedValueShape [ sh:closed true ] ;
                sh:qualifiedMinCount 1 ;
            ] .
    "#;
    let r = report(&store(), shapes);
    assert!(r["conforms"].is_null(), "must not claim a verdict: {r}");
    let skipped = r["skipped_constraints"].as_array().cloned().unwrap_or_default();
    assert!(!skipped.is_empty(), "{r}");
}
