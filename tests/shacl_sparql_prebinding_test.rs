// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Regression tests for issue #132.
//!
//! A `sh:sparql` constraint was evaluated by wrapping the author's SELECT as a
//! subquery under a `VALUES ?this { ... }` clause. SPARQL evaluates a subquery
//! bottom-up with no outer variable in scope, so inside it `$this` was unbound:
//! a `FILTER NOT EXISTS { $this ... }` asked "does ANY node satisfy this", the
//! single empty solution either survived (joining with every focus node, so
//! every node was reported) or died (so none was). One clean record therefore
//! hid every dirty one. SHACL-SPARQL section 5.3 says `$this` is pre-bound per
//! focus node; Oxigraph's `substitute_variable` is that mechanism.

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

const RECORDS: &str = r#"
    @prefix ex: <http://example.org/> .
    ex:r1 a ex:Record ; ex:has ex:e1 .
    ex:e1 ex:field ex:F ; ex:status "present" .
    ex:r2 a ex:Record ; ex:has ex:e2 .
    ex:e2 ex:field ex:F ; ex:status "absent" .
    ex:r3 a ex:Record .
"#;

const NEEDS_F: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    ex:NeedsF a sh:NodeShape ; sh:targetClass ex:Record ;
      sh:sparql [ sh:message "record lacks a present F" ;
        sh:select """SELECT $this WHERE { FILTER NOT EXISTS {
            $this <http://example.org/has> ?e .
            ?e <http://example.org/field> <http://example.org/F> ;
               <http://example.org/status> "present" . } }""" ] .
"#;

fn store(ttl: &str) -> Arc<GraphStore> {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(ttl, None).unwrap();
    store
}

fn report(store: &Arc<GraphStore>, shapes: &str) -> serde_json::Value {
    serde_json::from_str(&ShaclValidator::validate(store, shapes).unwrap()).unwrap()
}

fn focus_nodes(r: &serde_json::Value) -> Vec<String> {
    let mut v: Vec<String> = r["violations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["focus_node"].as_str().unwrap().to_string())
        .collect();
    v.sort();
    v
}

#[test]
fn one_clean_record_does_not_hide_the_dirty_ones() {
    // The exact reproduction from issue #132. pyshacl 0.40.1: conforms False,
    // focus nodes r2 and r3.
    let r = report(&store(RECORDS), NEEDS_F);
    assert_eq!(r["conforms"], serde_json::json!(false), "issue #132: {r}");
    assert_eq!(
        focus_nodes(&r),
        vec!["http://example.org/r2".to_string(), "http://example.org/r3".to_string()],
        "{r}"
    );
    assert_eq!(r["violation_count"], 2);
}

#[test]
fn every_dirty_record_is_still_reported_when_none_is_clean() {
    // The degenerate case that made the old evaluation look right.
    let all_dirty = RECORDS.replace("\"present\"", "\"absent\"");
    let r = report(&store(&all_dirty), NEEDS_F);
    assert_eq!(r["conforms"], serde_json::json!(false));
    assert_eq!(r["violation_count"], 3, "{r}");
}

#[test]
fn a_clean_data_set_still_conforms() {
    let all_clean = r#"
        @prefix ex: <http://example.org/> .
        ex:r1 a ex:Record ; ex:has ex:e1 .
        ex:e1 ex:field ex:F ; ex:status "present" .
    "#;
    let r = report(&store(all_clean), NEEDS_F);
    assert_eq!(r["conforms"], serde_json::json!(true), "{r}");
    assert_eq!(r["violation_count"], 0);
}

#[test]
fn blank_node_focus_nodes_are_evaluated_not_skipped() {
    // Blank nodes could not be named in a VALUES clause, so they were excluded
    // and the run was marked undetermined. Substitution binds a term, and a
    // blank node is a term.
    let data = r#"
        @prefix ex: <http://example.org/> .
        [] a ex:Record .
        ex:r1 a ex:Record ; ex:has ex:e1 .
        ex:e1 ex:field ex:F ; ex:status "present" .
    "#;
    let r = report(&store(data), NEEDS_F);
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], 1, "{r}");
    assert!(
        r["violations"][0]["focus_node"].as_str().unwrap().starts_with("_:"),
        "the blank node is the violator: {r}"
    );
    assert!(r["skipped_constraints"].is_null(), "nothing was skipped: {r}");
}

#[test]
fn solution_bindings_flow_into_the_result() {
    // SHACL-SPARQL 5.3.2: ?value becomes sh:value, ?path becomes sh:resultPath
    // and ?message overrides the constraint's sh:message.
    let data = r#"
        @prefix ex: <http://example.org/> .
        ex:big   a ex:Widget ; ex:size 99 .
        ex:small a ex:Widget ; ex:size 1 .
    "#;
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Widget ;
          sh:sparql [ sh:message "oversized" ;
            sh:select """SELECT $this ?value ?path ?message WHERE {
                $this <http://example.org/size> ?value .
                BIND(<http://example.org/size> AS ?path)
                BIND(CONCAT("too big: ", STR(?value)) AS ?message)
                FILTER(?value > 10) }""" ] .
    "#;
    let r = report(&store(data), shapes);
    assert_eq!(r["violation_count"], 1, "{r}");
    let v = &r["violations"][0];
    assert_eq!(v["focus_node"], "http://example.org/big");
    assert!(v["value"].as_str().unwrap().contains("99"), "{v}");
    assert_eq!(v["result_path"], "http://example.org/size");
    assert_eq!(v["message"], "too big: 99");
}
