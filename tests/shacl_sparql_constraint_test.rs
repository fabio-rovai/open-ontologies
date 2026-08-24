//! Regression tests for issue #98.
//!
//! `sh:sparql` constraints were not read at all, so a shapes graph built entirely
//! on SPARQL-based constraints returned `conforms: true` having evaluated nothing.
//! A validator that reports success on rules it never ran is more dangerous than
//! one that fails loudly, so these tests pin three behaviours:
//!
//!   1. a `sh:sparql` constraint that should fire, fires;
//!   2. a `sh:sparql` constraint that should not fire, does not;
//!   3. when a constraint cannot be executed, `conforms` is null rather than true.
//!
//! Test four covers the defect that made the first fix look broken: Oxigraph
//! renders literals in N-Triples form, so a multi-line `sh:select` arrived
//! carrying the two characters backslash and n where the author wrote a newline,
//! and every multi-line SPARQL constraint failed to parse.

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

fn store_with_widgets() -> Arc<GraphStore> {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:big   a ex:Widget ; ex:size 99 .
        ex:small a ex:Widget ; ex:size 1 .
    "#;
    store.load_turtle(ttl, None).unwrap();
    store
}

fn report(store: &Arc<GraphStore>, shapes: &str) -> serde_json::Value {
    serde_json::from_str(&ShaclValidator::validate(store, shapes).unwrap()).unwrap()
}

#[test]
fn sparql_constraint_that_should_fire_does_fire() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Widget ;
          sh:sparql [ sh:message "oversized widget" ;
            sh:select "SELECT $this WHERE { $this <http://example.org/size> ?s . FILTER(?s > 10) }" ] .
    "#;
    let r = report(&store_with_widgets(), shapes);

    assert_eq!(r["conforms"], serde_json::json!(false), "issue #98: sh:sparql must not silently pass");
    assert_eq!(r["violation_count"], 1);
    assert_eq!(r["violations"][0]["constraint"], "sparql");
    assert_eq!(r["violations"][0]["focus_node"], "http://example.org/big");
    assert_eq!(r["violations"][0]["message"], "oversized widget");
}

#[test]
fn sparql_constraint_that_should_not_fire_does_not() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Widget ;
          sh:sparql [ sh:message "impossible" ;
            sh:select "SELECT $this WHERE { $this <http://example.org/size> ?s . FILTER(?s > 1000) }" ] .
    "#;
    let r = report(&store_with_widgets(), shapes);

    assert_eq!(r["conforms"], serde_json::json!(true));
    assert_eq!(r["violation_count"], 0);
}

#[test]
fn unexecutable_constraint_yields_null_conformance_not_true() {
    // An undeclared prefix inside the author's SELECT. The engine cannot run it,
    // and must say so rather than report a clean bill of health.
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Widget ;
          sh:sparql [ sh:message "unrunnable" ;
            sh:select "SELECT $this WHERE { $this nosuchprefix:size ?s }" ] .
    "#;
    let r = report(&store_with_widgets(), shapes);

    assert!(r["conforms"].is_null(), "conformance must be undetermined, got {}", r["conforms"]);
    assert!(r["warning"].is_string());
    assert_eq!(r["skipped_constraints"].as_array().unwrap().len(), 1);
}

#[test]
fn multiline_select_parses() {
    // Regression for the literal-unescaping defect. Written across several lines
    // exactly as a human would write it in a real shapes file.
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Widget ;
          sh:sparql [ sh:message "oversized widget" ;
            sh:select """
                SELECT $this WHERE {
                  $this <http://example.org/size> ?s .
                  FILTER(?s > 10)
                }""" ] .
    "#;
    let r = report(&store_with_widgets(), shapes);

    assert_eq!(r["conforms"], serde_json::json!(false), "a multi-line sh:select must parse");
    assert_eq!(r["violation_count"], 1);
    assert!(r["skipped_constraints"].is_null(), "nothing should have been skipped");
}

#[test]
fn core_constraints_are_unaffected() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Widget ;
          sh:property [ sh:path ex:missing ; sh:minCount 1 ; sh:message "core constraint" ] .
    "#;
    let r = report(&store_with_widgets(), shapes);

    assert_eq!(r["conforms"], serde_json::json!(false));
    assert_eq!(r["violation_count"], 2);
    assert_eq!(r["violations"][0]["constraint"], "minCount");
}
