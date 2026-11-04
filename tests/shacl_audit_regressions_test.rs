// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Defects found by an adversarial audit of the SHACL validator, 13 Sep 2026.
//!
//! All four predate the `sh:sparql` pre-binding work and were found while
//! auditing it. Each is pinned here against the answer pyshacl 0.40.1 gives on
//! the same input, which is the oracle `tools/shacl_differential.py` uses.
//!
//!   1. `strip_quotes` searched the whole string for the literal suffix markers
//!      `^^` and `"@` and truncated there, so a constraint comparing against a
//!      typed literal was cut mid-query and never ran.
//!   2. A blank-node node shape was spliced into SPARQL as `_:label`, which is
//!      a non-distinguished variable, so it collected every property shape in
//!      the file and reported conforming data as non-conforming.
//!   3. `sh:deactivated true` was honoured on node and property shapes and
//!      ignored on a `sh:sparql` constraint node.
//!   4. Every `sh:declare` in the shapes graph was merged into one prologue and
//!      `sh:prefixes` was never read, so a prefix bound twice resolved by store
//!      row order. That is a false clean: the constraint pointing at the losing
//!      binding matched nothing and the run reported `conforms: true`.

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

fn store(ttl: &str) -> Arc<GraphStore> {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(ttl, None).unwrap();
    store
}

fn report(store: &Arc<GraphStore>, shapes: &str) -> serde_json::Value {
    serde_json::from_str(&ShaclValidator::validate(store, shapes).unwrap()).unwrap()
}

const DATED: &str = r#"
    @prefix ex: <http://example.org/> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
    ex:a a ex:Thing ; ex:when "2020-01-01"^^xsd:date .
    ex:b a ex:Thing ; ex:when "2030-01-01"^^xsd:date .
"#;

#[test]
fn a_constraint_comparing_against_a_typed_literal_runs() {
    // pyshacl: conforms False, one result, focus ex:a.
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
          sh:sparql [ sh:message "date is in the past" ;
            sh:select """SELECT $this WHERE {
                $this <http://example.org/when> ?d .
                FILTER(?d < "2025-01-01"^^<http://www.w3.org/2001/XMLSchema#date>) }""" ] .
    "#;
    let r = report(&store(DATED), shapes);
    assert!(
        r["skipped_constraints"].is_null(),
        "the query is valid SPARQL and must not be reported as unrunnable: {r}"
    );
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], 1, "{r}");
    assert_eq!(r["violations"][0]["focus_node"], "http://example.org/a");
}

#[test]
fn a_constraint_comparing_against_a_language_tagged_literal_runs() {
    let data = r#"
        @prefix ex: <http://example.org/> .
        ex:a a ex:Thing ; ex:label "hello"@en .
    "#;
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
          sh:sparql [ sh:message "english label" ;
            sh:select """SELECT $this WHERE {
                $this <http://example.org/label> ?l . FILTER(?l = "hello"@en) }""" ] .
    "#;
    let r = report(&store(data), shapes);
    assert!(r["skipped_constraints"].is_null(), "{r}");
    assert_eq!(r["violation_count"], 1, "{r}");
}

#[test]
fn a_pattern_containing_the_literal_suffix_markers_survives() {
    // The same helper served sh:pattern. `^^` is a plausible regex.
    let data = r#"
        @prefix ex: <http://example.org/> .
        ex:a a ex:Thing ; ex:code "x" .
    "#;
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
          sh:property [ sh:path ex:code ; sh:pattern "^[a-z]@[a-z]$" ] .
    "#;
    let r = report(&store(data), shapes);
    assert_eq!(r["conforms"], serde_json::json!(false), "the pattern must not match: {r}");
    assert_eq!(r["violation_count"], 1, "{r}");
}

#[test]
fn two_blank_node_shapes_do_not_borrow_each_others_constraints() {
    // pyshacl: conforms True. Each blank shape applies only its own property
    // shape to its own target.
    let data = r#"
        @prefix ex: <http://example.org/> .
        ex:a a ex:Alpha ; ex:name "n" .
        ex:b a ex:Beta  ; ex:code "c" .
    "#;
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        [] a sh:NodeShape ; sh:targetClass ex:Alpha ; sh:property [ sh:path ex:name ; sh:minCount 1 ] .
        [] a sh:NodeShape ; sh:targetClass ex:Beta  ; sh:property [ sh:path ex:code ; sh:minCount 1 ] .
    "#;
    let r = report(&store(data), shapes);
    assert_eq!(
        r["conforms"],
        serde_json::json!(true),
        "a blank node shape must not collect the other shape's constraints: {r}"
    );
    assert_eq!(r["violation_count"], 0, "{r}");
}

#[test]
fn a_blank_node_shape_still_enforces_its_own_constraint() {
    // The other direction, so the fix cannot pass by evaluating nothing.
    let data = r#"
        @prefix ex: <http://example.org/> .
        ex:a a ex:Alpha .
    "#;
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        [] a sh:NodeShape ; sh:targetClass ex:Alpha ; sh:property [ sh:path ex:name ; sh:minCount 1 ] .
    "#;
    let r = report(&store(data), shapes);
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], 1, "{r}");
}

#[test]
fn a_deactivated_sparql_constraint_produces_no_results() {
    // SHACL 5.3: "There are no validation results if the SPARQL-based
    // constraint has true as a value for the property sh:deactivated."
    // pyshacl: conforms True.
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
          sh:sparql [ sh:deactivated true ; sh:message "off" ;
            sh:select "SELECT $this WHERE { $this a <http://example.org/Thing> }" ] .
    "#;
    let r = report(&store(DATED), shapes);
    assert_eq!(r["conforms"], serde_json::json!(true), "{r}");
    assert_eq!(r["violation_count"], 0, "{r}");
    assert!(r["skipped_constraints"].is_null(), "deactivated is honoured, not skipped: {r}");
}

#[test]
fn a_live_sparql_constraint_still_fires() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
          sh:sparql [ sh:deactivated false ; sh:message "on" ;
            sh:select "SELECT $this WHERE { $this a <http://example.org/Thing> }" ] .
    "#;
    let r = report(&store(DATED), shapes);
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], 2, "{r}");
}

#[test]
fn an_unimplemented_predicate_on_a_constraint_node_is_recorded() {
    // There was no complement over sh:sparql constraint nodes at all, so any
    // sh: predicate written there was invisible to the "every constraint is
    // either executed or recorded" invariant.
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
          sh:sparql [ sh:message "x" ; sh:nodeKind sh:IRI ;
            sh:select "SELECT $this WHERE { $this a <http://example.org/Nothing> }" ] .
    "#;
    let r = report(&store(DATED), shapes);
    assert!(r["conforms"].is_null(), "an unevaluated constraint suppresses the verdict: {r}");
    let skipped = r["skipped_constraints"].as_array().unwrap();
    assert!(
        skipped.iter().any(|e| e["constraint"]
            .as_str()
            .unwrap_or("")
            .ends_with("nodeKind")),
        "{r}"
    );
}

const FLAGGED: &str = r#"
    @prefix ex: <http://example.org/> .
    ex:a a ex:Thing ; ex:flagged "yes" .
"#;

#[test]
fn a_prefix_bound_twice_is_refused_rather_than_guessed() {
    // Both declaration sets are in the shapes graph and the constraint names
    // neither, so which binding wins would be decided by store row order. The
    // old behaviour emitted two PREFIX lines, SPARQL took the last, and the
    // constraint pointing at the loser matched nothing and reported
    // `conforms: true` with nothing recorded. A false clean.
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        @prefix other: <http://other.example/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
          sh:sparql [ sh:message "flagged" ;
            sh:select "SELECT $this WHERE { $this p:flagged ?v }" ] .
        ex:decls    sh:declare [ sh:prefix "p" ; sh:namespace "http://example.org/"^^<http://www.w3.org/2001/XMLSchema#anyURI> ] .
        other:decls sh:declare [ sh:prefix "p" ; sh:namespace "http://unrelated.example/"^^<http://www.w3.org/2001/XMLSchema#anyURI> ] .
    "#;
    let r = report(&store(FLAGGED), shapes);
    assert!(
        r["conforms"].is_null(),
        "an ambiguous prologue must not resolve to a verdict: {r}"
    );
    let skipped = r["skipped_constraints"].as_array().unwrap();
    assert!(
        skipped.iter().any(|e| e["reason"].as_str().unwrap_or("").contains("different namespaces")),
        "the reason must name the ambiguity: {r}"
    );
}

#[test]
fn sh_prefixes_selects_the_declarations_that_apply() {
    // The same collision, but the constraint says which set it means. SHACL
    // 5.2.1 scopes declarations this way, and scoping resolves the ambiguity.
    // pyshacl: conforms False, one result.
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        @prefix other: <http://other.example/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
          sh:sparql [ sh:message "flagged" ; sh:prefixes ex:decls ;
            sh:select "SELECT $this WHERE { $this p:flagged ?v }" ] .
        ex:decls    sh:declare [ sh:prefix "p" ; sh:namespace "http://example.org/"^^<http://www.w3.org/2001/XMLSchema#anyURI> ] .
        other:decls sh:declare [ sh:prefix "p" ; sh:namespace "http://unrelated.example/"^^<http://www.w3.org/2001/XMLSchema#anyURI> ] .
    "#;
    let r = report(&store(FLAGGED), shapes);
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], 1, "{r}");
}

#[test]
fn the_whole_graph_prologue_still_serves_shapes_that_declare_no_prefixes() {
    // Backwards compatibility: no sh:prefixes and no collision keeps working.
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
          sh:sparql [ sh:message "flagged" ;
            sh:select "SELECT $this WHERE { $this p:flagged ?v }" ] .
        ex:decls sh:declare [ sh:prefix "p" ; sh:namespace "http://example.org/"^^<http://www.w3.org/2001/XMLSchema#anyURI> ] .
    "#;
    let r = report(&store(FLAGGED), shapes);
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], 1, "{r}");
}
