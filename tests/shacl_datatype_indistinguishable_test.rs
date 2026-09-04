//! `sh:datatype` naming a type the store does not preserve.
//!
//! oxigraph 0.5 encodes twelve XSD integer-derived datatypes, and
//! `xsd:dateTimeStamp`, into the encoding of the type they derive from, dropping
//! the datatype IRI. See `datatype_is_indistinguishable_in_store`, and
//! `datatype_preservation_test` for the isolation showing the parser is correct
//! and the store is not.
//!
//! The consequence for validation is that a `sh:datatype xsd:nonNegativeInteger`
//! constraint cannot be decided: by the time the query runs, a conforming literal
//! and a widened one are the same term. Asking anyway reported a violation
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
