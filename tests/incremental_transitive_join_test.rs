// SPDX-FileCopyrightText: 2026 Gqqqqgg
// SPDX-License-Identifier: MIT

//! Newly inferred edges must participate in the same incremental closure (#416).

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::{InferenceTarget, Reasoner, Spelled};
use open_ontologies::reason_incremental::IncrementalReasoner;
use open_ontologies::temporal::ScopeRequest;
use serde_json::Value;
use std::collections::BTreeSet;
use std::sync::Arc;

const PREFIXES: &str = "@prefix : <http://example.org/> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .";
const SCHEMA: &str = ":q rdfs:subPropertyOf :p . :p a owl:TransitiveProperty .";

fn edge(s: &str, p: &str, o: &str) -> Spelled {
    (
        format!("<http://example.org/{s}>"),
        format!("<http://example.org/{p}>"),
        format!("<http://example.org/{o}>"),
    )
}

fn ntriples(edges: &[Spelled]) -> String {
    edges
        .iter()
        .map(|(s, p, o)| format!("{s} {p} {o} .\n"))
        .collect()
}

fn prepared_graph(schema: &str, tracking: bool) -> Arc<GraphStore> {
    let graph = Arc::new(GraphStore::new());
    graph
        .load_turtle(&format!("{PREFIXES}\n{schema}"), None)
        .unwrap();
    Reasoner::run_scoped_with_tracking(
        &graph,
        "owl-rl",
        true,
        InferenceTarget::DefaultGraph,
        None,
        &ScopeRequest::Unscoped,
        tracking,
    )
    .unwrap();
    graph
}

fn incremental(graph: &Arc<GraphStore>, delta: &[Spelled], tracking: bool) -> Value {
    graph.load_ntriples(&ntriples(delta)).unwrap();
    let report: Value = serde_json::from_str(
        &IncrementalReasoner::run_scoped_with_tracking(graph, delta, true, false, tracking)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(report["ok"], true, "{report}");
    assert_eq!(report["complete"], true, "{report}");
    report
}

fn triples(graph: &GraphStore) -> BTreeSet<Spelled> {
    graph.all_triples().unwrap().into_iter().collect()
}

fn check_subproperty_join(tracking: bool) {
    let graph = prepared_graph(SCHEMA, tracking);
    let report = incremental(
        &graph,
        &[edge("a", "q", "b"), edge("b", "q", "c")],
        tracking,
    );
    assert!(
        triples(&graph).contains(&edge("a", "p", "c")),
        "a completed incremental run omitted the transitive consequence: {report}"
    );
    assert_eq!(report["inferred_count"], 3);
    if tracking {
        assert!(
            graph
                .ledger()
                .unwrap()
                .unwrap()
                .grounded()
                .contains(&edge("a", "p", "c"))
        );
    }
}

#[test]
fn new_subproperty_edges_join_in_one_untracked_run() {
    check_subproperty_join(false);
}

#[test]
fn new_subproperty_edges_join_in_one_tracked_run() {
    check_subproperty_join(true);
}

#[test]
fn new_inverse_edges_join_in_one_run() {
    for tracking in [false, true] {
        let graph = prepared_graph(
            ":q owl:inverseOf :p . :p a owl:TransitiveProperty .",
            tracking,
        );
        incremental(
            &graph,
            &[edge("b", "q", "a"), edge("c", "q", "b")],
            tracking,
        );
        assert!(triples(&graph).contains(&edge("a", "p", "c")));
    }
}

#[test]
fn several_rounds_join_new_edges_with_both_sides_of_the_stored_closure() {
    for tracking in [false, true] {
        let schema = format!("{SCHEMA} :left :p :a . :e :p :right .");
        let graph = prepared_graph(&schema, tracking);
        incremental(
            &graph,
            &[
                edge("a", "q", "b"),
                edge("b", "q", "c"),
                edge("c", "q", "d"),
                edge("d", "q", "e"),
                edge("right", "other", "unrelated"),
            ],
            tracking,
        );
        let actual = triples(&graph);
        assert!(actual.contains(&edge("left", "p", "right")));
        assert!(!actual.contains(&edge("left", "p", "unrelated")));

        let expected = prepared_graph(&schema, tracking);
        expected
            .load_ntriples(&ntriples(&[
                edge("a", "q", "b"),
                edge("b", "q", "c"),
                edge("c", "q", "d"),
                edge("d", "q", "e"),
                edge("right", "other", "unrelated"),
            ]))
            .unwrap();
        Reasoner::run(&expected, "owl-rl", true).unwrap();
        assert_eq!(actual, triples(&expected));
    }
}

#[test]
fn dry_run_reports_the_new_join_without_publishing_data_or_support() {
    let graph = prepared_graph(SCHEMA, false);
    let delta = [edge("a", "q", "b"), edge("b", "q", "c")];
    graph.load_ntriples(&ntriples(&delta)).unwrap();
    let before = graph.snapshot("nquads").unwrap();
    let report: Value = serde_json::from_str(
        &IncrementalReasoner::run_scoped_with_tracking(&graph, &delta, false, false, false)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(report["complete"], true);
    assert_eq!(report["materialized"], false);
    assert_eq!(report["inferred_count"], 3);
    assert_eq!(graph.snapshot("nquads").unwrap(), before);
    assert!(graph.ledger().unwrap().is_none());
}

#[test]
fn new_edges_in_separate_named_graphs_join_with_durable_support() {
    let graph = prepared_graph(SCHEMA, true);
    let delta = [edge("a", "q", "b"), edge("b", "q", "c")];
    let dataset = delta
        .iter()
        .enumerate()
        .map(|(i, (s, p, o))| format!("{s} {p} {o} <http://example.org/source{i}> .\n"))
        .collect::<String>();
    graph.load_nquads(&dataset).unwrap();
    IncrementalReasoner::run_scoped_with_tracking(&graph, &delta, true, false, true).unwrap();
    assert!(triples(&graph).contains(&edge("a", "p", "c")));
    graph
        .sparql_update_with_retraction("CLEAR GRAPH <http://example.org/source0>", true)
        .unwrap();
    assert!(!triples(&graph).contains(&edge("a", "p", "c")));
    assert!(triples(&graph).contains(&edge("b", "p", "c")));
}

#[test]
fn alternative_new_paths_survive_until_their_last_source_is_withdrawn() {
    let graph = prepared_graph(SCHEMA, true);
    let delta = [
        edge("a", "q", "b"),
        edge("b", "q", "c"),
        edge("a", "q", "d"),
        edge("d", "q", "c"),
    ];
    incremental(&graph, &delta, true);
    let target = edge("a", "p", "c");
    let ledger = graph.ledger().unwrap().unwrap();
    assert!(ledger.grounded().contains(&target));
    assert!(
        ledger
            .derivations
            .values()
            .filter(|d| d.enabled && d.rule == "prp-trp" && d.conclusion == target)
            .count()
            >= 2
    );
    graph.retract_assertions(&[delta[0].clone()]).unwrap();
    assert!(triples(&graph).contains(&target));
    graph.retract_assertions(&[delta[2].clone()]).unwrap();
    assert!(!triples(&graph).contains(&target));
    assert!(triples(&graph).contains(&edge("b", "p", "c")));
    assert!(triples(&graph).contains(&edge("d", "p", "c")));
}

#[test]
fn a_new_transitive_cycle_cannot_support_itself_after_withdrawal() {
    let graph = prepared_graph(SCHEMA, true);
    let delta = [edge("a", "q", "b"), edge("b", "q", "a")];
    incremental(&graph, &delta, true);
    assert!(triples(&graph).contains(&edge("a", "p", "a")));
    assert!(triples(&graph).contains(&edge("b", "p", "b")));
    graph.retract_assertions(&[delta[0].clone()]).unwrap();
    let remaining = triples(&graph);
    assert!(!remaining.contains(&edge("a", "p", "a")));
    assert!(!remaining.contains(&edge("b", "p", "b")));
    assert!(!remaining.contains(&edge("a", "p", "b")));
    assert!(remaining.contains(&edge("b", "p", "a")));
}

#[test]
fn batching_the_same_additions_does_not_change_the_closure() {
    for tracking in [false, true] {
        let together = prepared_graph(SCHEMA, tracking);
        let separate = prepared_graph(SCHEMA, tracking);
        let delta = [edge("a", "q", "b"), edge("b", "q", "c")];
        incremental(&together, &delta, tracking);
        for addition in &delta {
            incremental(&separate, std::slice::from_ref(addition), tracking);
        }
        assert_eq!(triples(&together), triples(&separate));
    }
}
