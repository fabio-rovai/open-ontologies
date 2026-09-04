//! Inferred triples must stay distinguishable from asserted ones.
//!
//! The defect this pins (flaw hunt, 30 Aug 2026, D2): `reason` merged
//! materialised triples into the default graph with no marker, so after
//! reasoning an inferred statement was byte-identical in form to one a person
//! wrote, and `save` then published it. Source `laund.ttl` went in at 8 triples
//! and came out at 9, newly asserting `<http://ex.org/ghost> a
//! <http://ex.org/Person>`, which nobody ever wrote.
//!
//! The fix follows the failure-direction argument: a separate container, not a
//! marker on the shared one. A reader that forgets about inference sees fewer
//! triples; a reader that wants them asks for the union. A marker triple would
//! fail the other way, and the one consumer that forgot to filter would
//! publish a laundered assertion.

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::{InferenceTarget, Reasoner, INFERRED_GRAPH};
use oxigraph::io::RdfFormat;
use std::sync::Arc;

/// The D2 reproduction: a dangling reference plus a range axiom.
/// OWL-RL range entailment materialises `ex:ghost a ex:Person`.
const LAUNDERING_CASE: &str = r#"
    @prefix ex: <http://ex.org/> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    ex:knows rdfs:range ex:Person .
    ex:alice ex:knows ex:ghost .
"#;

const GHOST_IS_A_PERSON: &str = "ASK { <http://ex.org/ghost> a <http://ex.org/Person> }";

fn asks_true(json: &str) -> bool {
    let parsed: serde_json::Value = serde_json::from_str(json).unwrap();
    parsed["result"].as_bool().unwrap()
}

#[test]
fn inferred_triples_stay_out_of_the_default_graph() {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(LAUNDERING_CASE, None).unwrap();

    let result =
        Reasoner::run_with_target(&store, "owl-rl", true, InferenceTarget::Inferred).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(
        parsed["inferred_count"].as_u64().unwrap() >= 1,
        "the range axiom should have entailed at least the ghost's type: {result}"
    );

    assert!(
        !asks_true(&store.sparql_select(GHOST_IS_A_PERSON).unwrap()),
        "an inferred type must not appear in the default graph beside asserted statements"
    );
    assert!(
        asks_true(&store.sparql_select_union(GHOST_IS_A_PERSON).unwrap()),
        "the inference must still be in the store, reachable through the union dataset"
    );
}

#[test]
fn the_response_names_the_graph_the_inferences_went_to() {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(LAUNDERING_CASE, None).unwrap();

    let result =
        Reasoner::run_with_target(&store, "owl-rl", true, InferenceTarget::Inferred).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(
        parsed["inference_graph"].as_str(),
        Some(INFERRED_GRAPH),
        "a caller cannot ask for the inferences back unless the response says where they went"
    );
}

#[test]
fn a_turtle_save_does_not_bake_in_inferred_triples() {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(LAUNDERING_CASE, None).unwrap();
    Reasoner::run_with_target(&store, "owl-rl", true, InferenceTarget::Inferred).unwrap();

    // Turtle cannot carry a graph name, so the only honest thing it can hold is
    // what was asserted. Round-trip through a fresh store rather than matching
    // strings, because the question is what a consumer would read back.
    let turtle = store.serialize("turtle").unwrap();
    let reloaded = GraphStore::new();
    reloaded.load_turtle(&turtle, None).unwrap();

    assert!(
        !asks_true(&reloaded.sparql_select_union(GHOST_IS_A_PERSON).unwrap()),
        "saving to a triple format published an inference as an assertion:\n{turtle}"
    );
    assert!(
        asks_true(
            &reloaded
                .sparql_select_union("ASK { <http://ex.org/alice> <http://ex.org/knows> <http://ex.org/ghost> }")
                .unwrap()
        ),
        "the asserted statements must survive the save untouched"
    );
}

#[test]
fn a_trig_save_keeps_the_inferences_under_their_own_graph_name() {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(LAUNDERING_CASE, None).unwrap();
    Reasoner::run_with_target(&store, "owl-rl", true, InferenceTarget::Inferred).unwrap();

    // TriG can carry the graph name, so nothing is dropped: the inference is
    // kept and stays labelled as one.
    let trig = store.serialize("trig").unwrap();
    assert!(
        trig.contains(INFERRED_GRAPH),
        "a dataset format must name the inference graph it is carrying:\n{trig}"
    );

    let reloaded = GraphStore::new();
    reloaded
        .load_content(&trig, RdfFormat::TriG)
        .unwrap();
    assert!(
        !asks_true(&reloaded.sparql_select(GHOST_IS_A_PERSON).unwrap()),
        "after a TriG round trip the inference must still be outside the default graph"
    );
    assert!(
        asks_true(&reloaded.sparql_select_union(GHOST_IS_A_PERSON).unwrap()),
        "after a TriG round trip the inference must still be in the store"
    );
}

#[test]
fn the_default_target_is_unchanged_so_existing_callers_keep_their_contract() {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(LAUNDERING_CASE, None).unwrap();

    Reasoner::run(&store, "owl-rl", true).unwrap();

    assert!(
        asks_true(&store.sparql_select(GHOST_IS_A_PERSON).unwrap()),
        "Reasoner::run must keep materialising into the default graph until the \
         default is deliberately flipped"
    );
}
