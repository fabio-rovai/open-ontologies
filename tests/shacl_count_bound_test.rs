//! A malformed count bound must never turn a shape into a no-op (#321).
//!
//! `sh:minCount` was read with `unwrap_or(0)` and `sh:maxCount` with
//! `unwrap_or(u64::MAX)`, so `sh:minCount "one"`, `sh:minCount 1.0` or
//! `sh:maxCount -1` became a bound no data can breach, and a focus node with no
//! value came back `conforms: true`. A malformed `sh:qualifiedMinCount` /
//! `sh:qualifiedMaxCount` made the row `continue` with no record. The same file
//! already refused a malformed `sh:minLength` by naming it in
//! `skipped_constraints`, which nulls the verdict; the counts now do the same.
//!
//! Pinned:
//!   1. each malformed spelling of each of the four count bounds yields
//!      `conforms: null` and a named `skipped_constraints` entry, never true;
//!   2. a well-formed bound still produces its violation (the fix did not
//!      turn every count into a skip);
//!   3. `onto_shacl_check` (`check_shapes`) reports each ill-formed literal
//!      with its reason;
//!   4. the verified evaluator refuses the same bounds (UNDETERMINED), when
//!      the `oo-shacl` binary is available.

mod common;

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

const PREFIXES: &str = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://ex.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
"#;

/// A Person with no ex:name at all, which is exactly what a minimum count
/// exists to catch.
const DATA: &str = "ex:Person a owl:Class . ex:name a owl:DatatypeProperty . ex:bob a ex:Person .";

fn graph() -> Arc<GraphStore> {
    let g = Arc::new(GraphStore::new());
    g.load_turtle(&format!("{PREFIXES}{DATA}"), None).unwrap();
    g
}

fn shape(constraint: &str) -> String {
    format!(
        "{PREFIXES} ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
             sh:property [ sh:path ex:name ; {constraint} ] ."
    )
}

fn qualified_shape(constraint: &str) -> String {
    format!(
        "{PREFIXES} ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
             sh:property [ sh:path ex:name ; sh:qualifiedValueShape [ sh:datatype xsd:string ] ; {constraint} ] ."
    )
}

fn validate(shapes: &str) -> serde_json::Value {
    serde_json::from_str(&ShaclValidator::validate(&graph(), shapes).unwrap()).unwrap()
}

fn assert_refused(shapes: &str, constraint: &str) {
    let r = validate(shapes);
    assert_ne!(r["conforms"], serde_json::json!(true), "{constraint} passed vacuously: {r}");
    assert!(r["conforms"].is_null(), "a skipped bound must null the verdict: {r}");
    let skipped = r["skipped_constraints"].as_array().unwrap_or_else(|| panic!("no skipped_constraints: {r}"));
    assert!(
        skipped.iter().any(|e| e["constraint"] == format!("sh:{constraint}")
            && e["reason"].as_str().unwrap_or("").contains("not a non-negative integer")),
        "sh:{constraint} must be named in skipped_constraints: {r}"
    );
}

const MALFORMED: [&str; 4] = ["\"one\"", "1.0", "-1", "\"1.5\"^^xsd:decimal"];

#[test]
fn malformed_min_count_never_conforms() {
    for v in MALFORMED {
        assert_refused(&shape(&format!("sh:minCount {v}")), "minCount");
    }
}

#[test]
fn malformed_max_count_never_conforms() {
    for v in MALFORMED {
        assert_refused(&shape(&format!("sh:maxCount {v}")), "maxCount");
    }
}

#[test]
fn malformed_qualified_counts_never_conform() {
    for v in MALFORMED {
        assert_refused(&qualified_shape(&format!("sh:qualifiedMinCount {v}")), "qualifiedMinCount");
        assert_refused(&qualified_shape(&format!("sh:qualifiedMaxCount {v}")), "qualifiedMaxCount");
    }
}

/// The control: a well-formed bound is still evaluated, and the same data
/// violates it. Without this the four tests above would pass on a validator
/// that skipped every count.
#[test]
fn well_formed_bounds_still_run() {
    for (shapes, constraint) in [
        (shape("sh:minCount 1"), "minCount"),
        (shape("sh:minCount \"1\"^^xsd:integer"), "minCount"),
        (qualified_shape("sh:qualifiedMinCount 1"), "qualifiedMinCount"),
    ] {
        let r = validate(&shapes);
        assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
        assert!(
            r["violations"].as_array().unwrap().iter().any(|v| v["constraint"] == constraint),
            "{r}"
        );
        assert!(r.get("skipped_constraints").is_none(), "{r}");
    }
    let r = validate(&shape("sh:maxCount 0"));
    assert_eq!(r["conforms"], serde_json::json!(true), "no value, maxCount 0 holds: {r}");
}

#[test]
fn shacl_check_reports_ill_formed_count_and_length_literals() {
    let shapes = format!(
        "{PREFIXES} ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
             sh:property [ sh:path ex:name ; sh:minCount \"one\" ] ;
             sh:property [ sh:path ex:name ; sh:maxCount -1 ] ;
             sh:property [ sh:path ex:name ; sh:minLength 2.5 ] ;
             sh:property [ sh:path ex:name ; sh:maxLength 4 ; sh:minCount 1 ] ."
    );
    let r: serde_json::Value =
        serde_json::from_str(&ShaclValidator::check_shapes(&graph(), &shapes).unwrap()).unwrap();
    assert_eq!(r["ok"], serde_json::json!(false), "{r}");
    let ill: Vec<&serde_json::Value> = r["issues"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["kind"] == "ill_formed_literal")
        .collect();
    assert_eq!(ill.len(), 3, "exactly the three malformed bounds, not the good ones: {r}");
    let reason_for = |c: &str| {
        ill.iter()
            .find(|i| i["constraint"] == c)
            .unwrap_or_else(|| panic!("{c} not reported: {r}"))["message"]
            .as_str()
            .unwrap()
            .to_string()
    };
    assert!(reason_for("sh:minCount").contains("xsd:integer"), "{r}");
    assert!(reason_for("sh:maxCount").contains("negative"), "{r}");
    assert!(reason_for("sh:minLength").contains("xsd:integer"), "{r}");
}

#[test]
fn shacl_check_passes_well_formed_bounds() {
    let shapes = format!(
        "{PREFIXES} ex:S a sh:NodeShape ; sh:targetClass ex:Person ;
             sh:property [ sh:path ex:name ; sh:minCount 1 ; sh:maxCount \"3\"^^xsd:integer ; sh:maxLength 0 ] ."
    );
    let r: serde_json::Value =
        serde_json::from_str(&ShaclValidator::check_shapes(&graph(), &shapes).unwrap()).unwrap();
    assert_eq!(r["ok"], serde_json::json!(true), "{r}");
}

/// The verified path refuses the same bounds. `readNat` in
/// `lean/Shacl/Compile.lean` declined "one" and -1 on its own but evaluated
/// `1.0` as 1, because the store hands it `"1"^^xsd:decimal`; the pre-check in
/// `validate_verified` closes that, so the two paths cannot drift apart.
#[test]
fn verified_evaluator_refuses_the_same_bounds() {
    if let Err(why) = open_ontologies::shacl_verified::find_checker()
        && common::skip_unless(false, &why, "the verified half of this test needs the oo-shacl binary")
    {
        return;
    }
    for constraint in ["sh:minCount \"one\"", "sh:maxCount -1", "sh:maxCount 1.0"] {
        let v = open_ontologies::shacl_verified::validate_verified(&graph(), &shape(constraint)).unwrap();
        assert!(v["conforms"].is_null(), "{constraint} must be refused: {v}");
        assert!(v["ill_formed"].as_array().is_some_and(|a| !a.is_empty()), "{constraint}: {v}");
    }
    // Control: a well-formed bound still reaches the evaluator and is judged.
    let v = open_ontologies::shacl_verified::validate_verified(&graph(), &shape("sh:minCount 1")).unwrap();
    assert_eq!(v["conforms"], serde_json::json!(false), "{v}");
}
