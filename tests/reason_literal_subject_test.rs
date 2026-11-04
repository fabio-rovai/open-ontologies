// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! No rule may conclude a triple whose subject is a literal.
//!
//! Found by an adversarial audit, 13 Sep 2026, on
//! `benchmark/ontoaxiom/data/ontoaxiom/ontologies/nordstream.ttl`. Four rules
//! conclude a triple whose SUBJECT comes from an object position: `prp-symp`,
//! `prp-inv1`, `prp-inv2` and `eq-sym`. Given a literal object, each produced a
//! triple no RDF serialisation can express. Materialisation then failed on the
//! whole batch with "The subject of a triple must be an IRI or a blank node",
//! and every inference from that run was lost, including the sound ones.
//!
//! The reported line number moved between runs (23, 54, 99 on three runs of one
//! file) because it indexes into a serialisation whose order comes from hash
//! iteration. A non-deterministic error message is what made this look like a
//! malformed input rather than a defect.

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::Reasoner;
use std::sync::Arc;

const PREFIXES: &str = r#"
    @prefix : <http://ex.org/> .
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
"#;

fn run(ttl: &str) -> (Arc<GraphStore>, serde_json::Value) {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(&format!("{PREFIXES}{ttl}"), None).unwrap();
    let out = Reasoner::run(&store, "owl-rl-ext", true).unwrap();
    (store, serde_json::from_str(&out).unwrap())
}

/// A symmetric, an inverse and a sameAs, each pointed at a literal.
const LITERAL_OBJECTS: &str = r#"
    :near a owl:SymmetricProperty .
    :a :near "a literal" .
    :parent owl:inverseOf :child .
    :b :parent "another literal" .
    :c owl:sameAs "a third literal" .
    :d :near :e .
"#;

#[test]
fn materialisation_succeeds_where_a_rule_would_have_made_a_literal_subject() {
    let (_store, r) = run(LITERAL_OBJECTS);
    assert!(r["error"].is_null(), "the run must not fail: {r}");
    assert!(
        r["inferred_count"].as_u64().is_some(),
        "a count means materialisation completed: {r}"
    );
}

#[test]
fn the_sound_inferences_on_the_same_input_survive() {
    // The guard must drop only the inexpressible conclusion, not the rule.
    let (store, r) = run(LITERAL_OBJECTS);
    let json: serde_json::Value = serde_json::from_str(
        &store
            .sparql_select_union("SELECT ?s WHERE { <http://ex.org/e> <http://ex.org/near> <http://ex.org/d> }")
            .unwrap(),
    )
    .unwrap();
    assert!(
        !json["results"].as_array().unwrap().is_empty(),
        "the IRI-valued symmetric pair must still be derived: {r}"
    );
}

#[test]
fn no_materialised_triple_has_a_literal_subject() {
    let (store, _) = run(LITERAL_OBJECTS);
    for (s, _, _) in store.all_triples().unwrap() {
        assert!(!s.starts_with('"'), "a literal cannot be a subject: {s}");
    }
}

#[test]
fn the_run_is_deterministic() {
    // The failure surfaced with a line number that changed between runs, so
    // repeat the run and require the same answer every time.
    let counts: Vec<u64> = (0..4)
        .map(|_| run(LITERAL_OBJECTS).1["inferred_count"].as_u64().unwrap())
        .collect();
    assert!(
        counts.windows(2).all(|w| w[0] == w[1]),
        "inference count must not depend on iteration order: {counts:?}"
    );
}
