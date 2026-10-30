//! `sh:datatype` naming a type `DATATYPE()` cannot report.
//!
//! The twelve XSD integer-derived datatypes and `xsd:dateTimeStamp`. Oxigraph
//! 0.5.9's store dropped their IRIs; the pinned store keeps them since #335, but
//! the SPARQL evaluator still reads each as the type it derives from, so
//! `DATATYPE("0"^^xsd:nonNegativeInteger)` is `xsd:integer`. See
//! `datatype_is_indistinguishable_in_store`, and `datatype_preservation_test`
//! for the isolation of parser, store and evaluator.
//!
//! The consequence for validation is that a `sh:datatype xsd:nonNegativeInteger`
//! constraint cannot be decided: by the time the test runs, a conforming literal
//! and one of the base type answer alike. Asking anyway reported a violation
//! against every value that satisfied the shape — nine of them in
//! `jsonld-escaping-conformance`, which is how the storage defect was found.
//!
//! A wrong answer is worse than no answer, so the constraint is recorded as
//! unevaluated and the verdict is suppressed, exactly as for any other constraint
//! this validator cannot execute.

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
            ex:a a ex:Profile ; ex:passes "0"^^xsd:nonNegativeInteger ; ex:name "extruct" .
            ex:b a ex:Profile ; ex:passes "1"^^xsd:nonNegativeInteger ; ex:name "nodejs" .
        "#,
            None,
        )
        .unwrap();
    store
}

#[test]
fn a_datatype_the_store_cannot_preserve_is_not_answered() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Profile ;
            sh:property [ sh:path ex:passes ; sh:datatype xsd:nonNegativeInteger ] .
    "#;
    let r = report(&store(), shapes);
    assert_eq!(
        r["violation_count"],
        serde_json::json!(0),
        "the data satisfies this shape; reporting a violation is the bug: {r}"
    );
    assert!(
        r["conforms"].is_null(),
        "the constraint could not be decided, so there is no verdict to give: {r}"
    );
    let skipped = r["skipped_constraints"].as_array().cloned().unwrap_or_default();
    assert_eq!(skipped.len(), 1, "{r}");
    assert_eq!(skipped[0]["constraint"], "sh:datatype");
    assert_eq!(
        skipped[0]["datatype"],
        "http://www.w3.org/2001/XMLSchema#nonNegativeInteger"
    );
}

/// The datatypes the store does keep must still be checked, and still catch a
/// real mismatch. Suppressing the whole constraint family would trade a false
/// violation for a blind spot.
#[test]
fn a_datatype_the_store_preserves_is_still_evaluated() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Profile ;
            sh:property [ sh:path ex:name ; sh:datatype xsd:string ] .
    "#;
    let r = report(&store(), shapes);
    assert_eq!(r["conforms"], serde_json::json!(true), "{r}");

    let wrong = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Profile ;
            sh:property [ sh:path ex:name ; sh:datatype xsd:date ] .
    "#;
    let r = report(&store(), wrong);
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], serde_json::json!(2), "{r}");
}

/// `xsd:integer` itself is the type the others collapse into, so it is decidable
/// and must not be caught by the guard.
#[test]
fn plain_integer_is_still_evaluated() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Profile ;
            sh:property [ sh:path ex:passes ; sh:datatype xsd:integer ] .
    "#;
    let r = report(&store(), shapes);
    assert!(!r["conforms"].is_null(), "must give a verdict: {r}");
}

// ── The same guard on every path that tests a datatype ───────────────────────
//
// The direct `sh:datatype` constraint (on a property shape and on a node shape)
// consults `datatype_is_indistinguishable_in_store` and refuses to answer. The
// paths that test a datatype as a MEMBER of something else did not, so the
// question the direct path refuses was answered anyway, wrongly, by them: every
// value `DATATYPE()` read as `xsd:integer` failed a member of
// `xsd:nonNegativeInteger`. Each test below names its path, asserts the refusal
// the direct path gives, and asserts that no violation is reported.

const PREFIXES: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
    @prefix ex: <http://example.org/> .
"#;

fn refused(shapes_body: &str, constraint: &str) {
    let r = report(&store(), &format!("{PREFIXES}{shapes_body}"));
    assert_eq!(
        r["violation_count"],
        serde_json::json!(0),
        "the data satisfies this shape; reporting a violation is the bug: {r}"
    );
    assert!(r["conforms"].is_null(), "the datatype cannot be decided, so there is no verdict: {r}");
    let skipped = r["skipped_constraints"].as_array().cloned().unwrap_or_default();
    assert!(
        skipped.iter().any(|s| s["constraint"] == constraint),
        "{constraint} must be recorded as not evaluated: {r}"
    );
}

/// `sh:or` over a property shape's values, one member naming the datatype.
#[test]
fn an_or_member_naming_an_unpreserved_datatype_is_not_answered() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetClass ex:Profile ;
           sh:property [ sh:path ex:passes ;
             sh:or ( [ sh:datatype xsd:nonNegativeInteger ] [ sh:datatype xsd:string ] ) ] .",
        "sh:or",
    );
}

/// `sh:not [ sh:datatype D ]`. Here the wrong answer was a FALSE CLEAN: every
/// value is an xsd:nonNegativeInteger and so violates the negation, but the
/// store said xsd:integer and the run came back `conforms: true`.
#[test]
fn a_not_member_naming_an_unpreserved_datatype_is_not_answered() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetClass ex:Profile ;
           sh:property [ sh:path ex:passes ; sh:not [ sh:datatype xsd:nonNegativeInteger ] ] .",
        "sh:not",
    );
}

/// `sh:qualifiedValueShape [ sh:datatype D ]`: no value counted as conforming.
#[test]
fn a_qualified_shape_naming_an_unpreserved_datatype_is_not_answered() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetClass ex:Profile ;
           sh:property [ sh:path ex:passes ;
             sh:qualifiedValueShape [ sh:datatype xsd:nonNegativeInteger ] ; sh:qualifiedMinCount 1 ] .",
        "sh:qualifiedValueShape",
    );
}

/// `sh:node` to a shape whose OWN constraint is the datatype. The compiler
/// already refused the same datatype on a property inside the referenced shape
/// and not on the shape itself.
#[test]
fn a_node_shape_reference_naming_an_unpreserved_datatype_is_not_answered() {
    refused(
        "ex:S a sh:NodeShape ; sh:targetClass ex:Profile ;
           sh:property [ sh:path ex:passes ; sh:node ex:Count ] .
         ex:Count a sh:NodeShape ; sh:datatype xsd:nonNegativeInteger .",
        "sh:node",
    );
}

/// The guard must not swallow the decidable members: the same four paths over
/// `xsd:integer` and `xsd:string` still give a verdict and still catch a value
/// that matches none of them.
#[test]
fn members_naming_a_preserved_datatype_are_still_evaluated() {
    let cases = [
        "ex:S a sh:NodeShape ; sh:targetClass ex:Profile ;
           sh:property [ sh:path ex:name ; sh:or ( [ sh:datatype xsd:integer ] [ sh:datatype xsd:date ] ) ] .",
        "ex:S a sh:NodeShape ; sh:targetClass ex:Profile ;
           sh:property [ sh:path ex:name ; sh:not [ sh:datatype xsd:string ] ] .",
        "ex:S a sh:NodeShape ; sh:targetClass ex:Profile ;
           sh:property [ sh:path ex:name ;
             sh:qualifiedValueShape [ sh:datatype xsd:integer ] ; sh:qualifiedMinCount 1 ] .",
        "ex:S a sh:NodeShape ; sh:targetClass ex:Profile ;
           sh:property [ sh:path ex:name ; sh:node ex:Int ] .
         ex:Int a sh:NodeShape ; sh:datatype xsd:integer .",
    ];
    for body in cases {
        let r = report(&store(), &format!("{PREFIXES}{body}"));
        assert_eq!(r["conforms"], serde_json::json!(false), "{body}\n{r}");
        assert_eq!(r["violation_count"], serde_json::json!(2), "{body}\n{r}");
    }
}
