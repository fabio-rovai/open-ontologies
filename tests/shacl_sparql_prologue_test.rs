//! A `sh:sparql` constraint whose SELECT declares its own prefixes.
//!
//! SHACL pre-binds `$this`, and this validator binds it by wrapping the author's
//! SELECT as a subquery under a VALUES clause. SPARQL allows PREFIX and BASE only
//! in the prologue, at the very start of a query, so an author's prologue carried
//! into that subquery position makes the whole query unparseable. The constraint
//! was then reported as skipped: honest, but it never ran.
//!
//! This is not a corner case. Declaring prefixes inside `sh:select` is the
//! portable way to write a SPARQL constraint, it is what pyshacl accepts, and
//! `sh:sparql` is the third most-used constraint across this machine's shapes
//! files (223 occurrences in 88 files). Every one of the seven in
//! `bank-register-ontology/shapes/bro-shapes.ttl` failed this way, found by the
//! differential run against pyshacl rather than by the unit tests.

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
            ex:a a ex:Assertion ; ex:conformant false .
            ex:b a ex:Assertion ; ex:conformant false ; ex:reason "truncated" .
        "#,
            None,
        )
        .unwrap();
    store
}

/// The shape from the banking vertical, reduced: a non-conformant assertion must
/// record why. `ex:a` does not, `ex:b` does.
const WITH_PROLOGUE: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    ex:S a sh:NodeShape ;
        sh:targetClass ex:Assertion ;
        sh:sparql [
            sh:message "non-conformant with no reason" ;
            sh:severity sh:Violation ;
            sh:select """
                PREFIX ex: <http://example.org/>
                SELECT $this WHERE {
                    $this ex:conformant false .
                    FILTER NOT EXISTS { $this ex:reason ?r }
                }
            """ ;
        ] .
"#;

#[test]
fn a_select_declaring_its_own_prefixes_is_executed() {
    let r = report(&store(), WITH_PROLOGUE);
    assert_eq!(
        r["conforms"],
        serde_json::json!(false),
        "the constraint must run, not be skipped: {r}"
    );
    assert_eq!(r["violation_count"], serde_json::json!(1), "{r}");
    assert_eq!(r["violations"][0]["focus_node"], "http://example.org/a");
    assert_eq!(r["violations"][0]["message"], "non-conformant with no reason");
    let skipped = r["skipped_constraints"].as_array().cloned().unwrap_or_default();
    assert!(skipped.is_empty(), "nothing should be skipped: {r}");
}

/// BASE is prologue too, and may be followed by further PREFIX lines.
#[test]
fn a_select_with_base_and_comments_in_the_prologue_is_executed() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ;
            sh:targetClass ex:Assertion ;
            sh:sparql [
                sh:message "non-conformant with no reason" ;
                sh:select """
                    # the rule, as the register states it
                    BASE <http://example.org/>
                    PREFIX ex: <http://example.org/>
                    SELECT $this WHERE {
                        $this ex:conformant false .
                        FILTER NOT EXISTS { $this ex:reason ?r }
                    }
                """ ;
            ] .
    "#;
    let r = report(&store(), shapes);
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], serde_json::json!(1), "{r}");
}

/// A SELECT with no prologue of its own must keep working exactly as before.
#[test]
fn a_select_without_a_prologue_is_unaffected() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ;
            sh:targetClass ex:Assertion ;
            sh:sparql [
                sh:message "non-conformant with no reason" ;
                sh:select """
                    SELECT $this WHERE {
                        $this <http://example.org/conformant> false .
                        FILTER NOT EXISTS { $this <http://example.org/reason> ?r }
                    }
                """ ;
            ] .
    "#;
    let r = report(&store(), shapes);
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], serde_json::json!(1), "{r}");
}

/// A genuinely broken query must still be reported as skipped and must still
/// suppress the verdict. Hoisting the prologue must not turn an unrunnable
/// constraint into a silent pass.
#[test]
fn a_query_that_cannot_parse_is_still_reported_as_skipped() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ;
            sh:targetClass ex:Assertion ;
            sh:sparql [
                sh:select """
                    PREFIX ex: <http://example.org/>
                    SELECT $this WHERE { this is not sparql (((
                """ ;
            ] .
    "#;
    let r = report(&store(), shapes);
    assert!(r["conforms"].is_null(), "must not claim a pass: {r}");
    let skipped = r["skipped_constraints"].as_array().cloned().unwrap_or_default();
    assert!(!skipped.is_empty(), "the failure must be named: {r}");
}
