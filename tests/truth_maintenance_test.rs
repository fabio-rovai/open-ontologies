// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Exercise the existing public reasoning/write interfaces, not a second
//! lifecycle service. Withdrawal must change query results in the SAME commit.

use open_ontologies::graph::{GraphStore, ReadScope};
use open_ontologies::reason::{InferenceTarget, Reasoner};
use open_ontologies::reason_incremental::IncrementalReasoner;
use open_ontologies::truth_maintenance::{LEDGER_GRAPH, Ledger};
use std::sync::Arc;

const PREFIX: &str =
    "@prefix : <http://example.org/> . @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .";
fn graph(ttl: &str) -> Arc<GraphStore> {
    let g = Arc::new(GraphStore::new());
    g.load_turtle(&format!("{PREFIX}\n{ttl}"), None).unwrap();
    g
}
fn t(s: &str, p: &str, o: &str) -> (String, String, String) {
    (s.into(), p.into(), o.into())
}
fn holds(g: &GraphStore, s: &str, class: &str) -> bool {
    g.all_triples().unwrap().contains(&t(
        &format!("<http://example.org/{s}>"),
        "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>",
        &format!("<http://example.org/{class}>"),
    ))
}
fn reason(g: &Arc<GraphStore>) {
    Reasoner::run_scoped_with_tracking(
        g,
        "rdfs",
        true,
        InferenceTarget::DefaultGraph,
        None,
        &open_ontologies::temporal::ScopeRequest::Unscoped,
        true,
    )
    .unwrap();
}

#[test]
fn existing_reasoner_persists_all_alternatives_and_delete_retracts_transitively() {
    let g = graph(
        ":A rdfs:subClassOf :C . :B rdfs:subClassOf :C . :C rdfs:subClassOf :D . :x a :A, :B .",
    );
    reason(&g);
    assert!(holds(&g, "x", "D"));
    g.sparql_update_with_retraction(
        "DELETE DATA { <http://example.org/x> a <http://example.org/A> }",
        true,
    )
    .unwrap();
    assert!(holds(&g, "x", "C"), "the independent B derivation survives");
    assert!(holds(&g, "x", "D"));
    g.sparql_update_with_retraction(
        "DELETE DATA { <http://example.org/x> a <http://example.org/B> }",
        true,
    )
    .unwrap();
    assert!(!holds(&g, "x", "C"));
    assert!(!holds(&g, "x", "D"));
    assert!(
        !g.ledger()
            .unwrap()
            .unwrap()
            .history
            .last()
            .unwrap()
            .withdrawn_facts
            .is_empty()
    );
}

#[test]
fn recursive_support_does_not_survive_withdrawal_of_its_last_seed() {
    let g = graph(":A rdfs:subClassOf :B . :B rdfs:subClassOf :A . :x a :A .");
    reason(&g);
    g.sparql_update_with_retraction(
        "DELETE DATA { <http://example.org/x> a <http://example.org/A> }",
        true,
    )
    .unwrap();
    assert!(!holds(&g, "x", "A"));
    assert!(!holds(&g, "x", "B"));
}

#[test]
fn reassertion_is_independent_of_the_derived_copy() {
    let g = graph(":A rdfs:subClassOf :B . :x a :A .");
    reason(&g);
    g.load_turtle(&format!("{PREFIX} :x a :B ."), None).unwrap();
    g.sparql_update_with_retraction(
        "DELETE DATA { <http://example.org/x> a <http://example.org/A> }",
        true,
    )
    .unwrap();
    assert!(holds(&g, "x", "B"));
}

#[test]
fn repeated_reasoning_never_promotes_inferences_to_assertions() {
    let g = graph(":A rdfs:subClassOf :B . :x a :A .");
    reason(&g);
    reason(&g);
    assert_eq!(
        g.triples_in_scope(&ReadScope::AllGraphs).unwrap().0.len(),
        2
    );
    let dir = tempfile::tempdir().unwrap();
    Reasoner::run_full(
        &g,
        "rdfs",
        false,
        InferenceTarget::DefaultGraph,
        Some(dir.path()),
    )
    .unwrap();
    let asserted = std::fs::read_to_string(dir.path().join("asserted.tsv")).unwrap();
    assert!(!asserted.contains("<http://example.org/x>\t<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>\t<http://example.org/B>"));
}

#[test]
fn independent_named_graph_evidence_is_withdrawn_separately() {
    let g = graph(":A rdfs:subClassOf :B .");
    g.load_nquads("<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> <http://example.org/source1> .\n<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> <http://example.org/source2> .").unwrap();
    reason(&g);
    g.sparql_update_with_retraction("CLEAR GRAPH <http://example.org/source1>", true)
        .unwrap();
    assert!(holds(&g, "x", "B"));
    g.sparql_update_with_retraction("CLEAR GRAPH <http://example.org/source2>", true)
        .unwrap();
    assert!(!holds(&g, "x", "B"));
}

#[test]
fn dataset_backup_restores_dependencies_and_they_still_retract() {
    let g = graph(":A rdfs:subClassOf :B . :x a :A .");
    reason(&g);
    let snapshot = g.snapshot("nquads").unwrap();
    let restored = Arc::new(GraphStore::new());
    restored.load_nquads(&snapshot).unwrap();
    assert_eq!(restored.triple_count(), g.triple_count());
    restored
        .sparql_update_with_retraction(
            "DELETE DATA { <http://example.org/x> a <http://example.org/A> }",
            true,
        )
        .unwrap();
    assert!(!holds(&restored, "x", "B"));
}

#[test]
fn legacy_interfaces_do_not_count_or_reason_over_metadata() {
    let g = graph(":A rdfs:subClassOf :B . :x a :A .");
    reason(&g);
    assert_eq!(g.triple_count(), 3);
    assert_eq!(g.all_triples().unwrap().len(), 3);
    assert!(!g.serialize("turtle").unwrap().contains(LEDGER_GRAPH));
    let v: serde_json::Value = serde_json::from_str(
        &g.sparql_select_union("SELECT ?s ?p ?o WHERE {?s ?p ?o}")
            .unwrap(),
    )
    .unwrap();
    assert_eq!(v["results"].as_array().unwrap().len(), 3);
}

#[test]
fn rule_retirement_withdraws_only_conclusions_without_independent_support() {
    let g = graph(":A rdfs:subClassOf :B . :x a :A . :y a :B .");
    reason(&g);
    let pack = open_ontologies::truth_maintenance::builtin_rule_pack_hash();
    g.invalidate_rule("rdfs9", &pack).unwrap();
    assert!(!holds(&g, "x", "B"));
    assert!(holds(&g, "y", "B"));
}

#[test]
fn incremental_inference_uses_the_same_durable_support_and_can_be_retracted() {
    let g = graph(":A rdfs:subClassOf :B . :B rdfs:subClassOf :C .");
    reason(&g);
    g.load_turtle(&format!("{PREFIX} :x a :A ."), None).unwrap();
    let delta = vec![t(
        "<http://example.org/x>",
        "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>",
        "<http://example.org/A>",
    )];
    IncrementalReasoner::run_scoped_with_tracking(&g, &delta, true, false, true).unwrap();
    assert!(holds(&g, "x", "C"));
    g.retract_assertions(&delta).unwrap();
    assert!(!holds(&g, "x", "B"));
    assert!(!holds(&g, "x", "C"));
}

#[test]
fn durable_proof_export_uses_existing_checker_inputs_and_keeps_all_alternatives() {
    let g = graph(":A rdfs:subClassOf :C . :B rdfs:subClassOf :C . :x a :A, :B .");
    reason(&g);
    let dir = tempfile::tempdir().unwrap();
    let ledger = g.ledger().unwrap().unwrap();
    let manifest = ledger.export_certificate(dir.path()).unwrap();
    assert_eq!(manifest["verdict"], "not_checked");
    let restored: Ledger =
        serde_json::from_slice(&std::fs::read(dir.path().join("ledger.json")).unwrap()).unwrap();
    assert_eq!(restored.derivations.len(), ledger.derivations.len());
    assert!(
        std::fs::read_to_string(dir.path().join("derivations.tsv"))
            .unwrap()
            .contains("rdfs9")
    );
}

#[test]
fn deleting_an_assertion_keeps_its_independent_derivation() {
    let g = graph(":A rdfs:subClassOf :B . :x a :A, :B .");
    reason(&g);
    g.sparql_update_with_retraction(
        "DELETE DATA { <http://example.org/x> a <http://example.org/B> }",
        true,
    )
    .unwrap();
    assert!(holds(&g, "x", "B"));
    g.sparql_update_with_retraction(
        "DELETE DATA { <http://example.org/x> a <http://example.org/A> }",
        true,
    )
    .unwrap();
    assert!(!holds(&g, "x", "B"));
}

#[test]
fn failed_load_and_update_preserve_both_data_and_support() {
    let g = graph(":A rdfs:subClassOf :B . :x a :A .");
    reason(&g);
    let before = g.snapshot("nquads").unwrap();
    assert!(
        g.load_ntriples(
            "<http://example.org/new> <http://example.org/p> <http://example.org/o> .\ninvalid"
        )
        .is_err()
    );
    assert!(
        g.sparql_update_with_retraction(
            "DELETE DATA { <http://example.org/x> a <http://example.org/A> }; INVALID",
            true
        )
        .is_err()
    );
    assert_eq!(g.snapshot("nquads").unwrap(), before);
}

#[test]
fn new_evidence_marks_the_recorded_closure_incomplete_until_reasoning_runs() {
    let g = graph(":A rdfs:subClassOf :B . :x a :A .");
    reason(&g);
    assert!(g.ledger().unwrap().unwrap().fixpoint_reached);
    g.load_turtle(&format!("{PREFIX} :y a :A ."), None).unwrap();
    assert!(!g.ledger().unwrap().unwrap().fixpoint_reached);
    reason(&g);
    assert!(holds(&g, "y", "B"));
    assert!(g.ledger().unwrap().unwrap().fixpoint_reached);
}

#[test]
fn persisted_dependencies_survive_store_reopen() {
    let dir = tempfile::tempdir().unwrap();
    {
        let g = Arc::new(GraphStore::open_persistent(dir.path()).unwrap());
        g.load_turtle(&format!("{PREFIX} :A rdfs:subClassOf :B . :x a :A ."), None)
            .unwrap();
        reason(&g);
    }
    let g = GraphStore::open_persistent(dir.path()).unwrap();
    assert!(holds(&g, "x", "B"));
    g.sparql_update_with_retraction(
        "DELETE DATA { <http://example.org/x> a <http://example.org/A> }",
        true,
    )
    .unwrap();
    assert!(!holds(&g, "x", "B"));
}

#[test]
fn blank_node_support_survives_dataset_backup() {
    let g = graph(":A rdfs:subClassOf :B . _:person a :A .");
    reason(&g);
    let snapshot = g.snapshot("nquads").unwrap();
    let restored = Arc::new(GraphStore::new());
    restored.load_nquads(&snapshot).unwrap();
    restored
        .sparql_update_with_retraction("DELETE WHERE { ?s a <http://example.org/A> }", true)
        .unwrap();
    assert!(!restored.all_triples().unwrap().iter().any(|(_, p, o)| p
        == "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"
        && o == "<http://example.org/B>"));
}

#[test]
fn sparql_reinsertion_after_withdrawal_creates_new_active_evidence() {
    let g = graph(":A rdfs:subClassOf :B . :B rdfs:subClassOf :A . :x a :A .");
    reason(&g);
    g.sparql_update_with_retraction(
        "DELETE DATA { <http://example.org/x> a <http://example.org/A> }",
        true,
    )
    .unwrap();
    assert!(!holds(&g, "x", "B"));
    g.sparql_update_with_retraction(
        "INSERT DATA { <http://example.org/x> a <http://example.org/A> }",
        true,
    )
    .unwrap();
    assert!(holds(&g, "x", "A"));
    assert!(holds(&g, "x", "B"));
}

#[test]
fn withdrawing_a_named_graph_assertion_reveals_its_independent_inference() {
    let g = graph(":A rdfs:subClassOf :B . :x a :A .");
    g.load_nquads("<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/B> <http://example.org/source> .").unwrap();
    reason(&g);
    assert_eq!(
        g.triple_count(),
        3,
        "no redundant inference copy is needed yet"
    );
    g.sparql_update_with_retraction("CLEAR GRAPH <http://example.org/source>", true)
        .unwrap();
    assert!(holds(&g, "x", "B"));
    assert!(
        g.triples_in_scope(&ReadScope::AllGraphs)
            .unwrap()
            .0
            .iter()
            .all(|t| t.2 != "<http://example.org/B>" || t.0 == "<http://example.org/A>")
    );
    g.sparql_update_with_retraction(
        "DELETE DATA { <http://example.org/x> a <http://example.org/A> }",
        true,
    )
    .unwrap();
    assert!(!holds(&g, "x", "B"));
}

#[test]
fn default_materialisation_and_delete_keep_legacy_semantics() {
    let g = graph(":A rdfs:subClassOf :B . :x a :A .");
    Reasoner::run(&g, "rdfs", true).unwrap();
    assert!(g.ledger().unwrap().is_none(), "capture must be opt-in");
    g.sparql_update("DELETE DATA { <http://example.org/x> a <http://example.org/A> }")
        .unwrap();
    assert!(holds(&g, "x", "B"), "ordinary DELETE must not cascade");
}

#[test]
fn untracked_bulk_updates_match_the_native_executor_and_net_counts() {
    use oxigraph::sparql::SparqlEvaluator;
    use oxigraph::store::Store;
    use std::collections::BTreeSet;

    // An opt-in withdrawal request on a store without a ledger must also keep
    // the native update semantics; there are no recorded supports to reconcile.
    for retract in [false, true] {
        let g = graph("");
        let native = Store::new().unwrap();
        let data = (0..1024)
            .map(|i| format!("<urn:s:{i}> a <urn:Class> ."))
            .collect::<String>();
        for update in [
            format!("INSERT DATA {{ {data} }}"),
            "INSERT { ?s <urn:typed> ?c } WHERE { ?s a ?c }".into(),
            "INSERT { ?s <urn:typed> ?c } WHERE { ?s a ?c }".into(),
            "DELETE { ?s <urn:typed> ?c } WHERE { ?s <urn:typed> ?c }".into(),
            "INSERT { ?s <urn:all> ?o } WHERE { ?s ?p ?o }".into(),
            "DELETE WHERE { ?s <urn:all> ?o }".into(),
            "INSERT { GRAPH <urn:source> { ?s a ?c } } WHERE { ?s a ?c }".into(),
            "COPY GRAPH <urn:source> TO GRAPH <urn:copy>; MOVE GRAPH <urn:copy> TO GRAPH <urn:moved>".into(),
            "WITH <urn:moved> DELETE { ?s a ?c } INSERT { ?s <urn:typed> ?c } WHERE { ?s a ?c }".into(),
            "CLEAR NAMED".into(),
            "DELETE { ?s a ?c } INSERT { ?s a ?c } WHERE { ?s a ?c }".into(),
        ] {
            let before = native.len().unwrap();
            SparqlEvaluator::new()
                .parse_update(&update).unwrap()
                .on_store(&native).execute().unwrap();
            let count = g.sparql_update_with_retraction(&update, retract).unwrap();
            assert_eq!(count, native.len().unwrap().saturating_sub(before), "{update}");
            let expected: BTreeSet<_> = native.iter()
                .map(|q| format!("{} .", q.unwrap())).collect();
            let actual: BTreeSet<_> = g.snapshot("nquads").unwrap()
                .lines().map(str::to_owned).collect();
            assert_eq!(actual, expected, "{update}");
            assert!(g.ledger().unwrap().is_none());
        }
    }
}

#[test]
fn untracked_native_update_rolls_back_on_an_execution_error() {
    for retract in [false, true] {
        let g = graph(":s :p :o .");
        g.sparql_update("CREATE GRAPH <urn:existing>").unwrap();
        let before = g.snapshot("nquads").unwrap();
        assert!(
            g.sparql_update_with_retraction(
                "INSERT DATA { <urn:new> <urn:p> <urn:o> }; CREATE GRAPH <urn:existing>",
                retract,
            )
            .is_err()
        );
        assert_eq!(g.snapshot("nquads").unwrap(), before);
        assert!(g.sparql_update("CREATE GRAPH <urn:existing>").is_err());
        assert!(g.ledger().unwrap().is_none());
    }
}

#[test]
fn default_delete_does_not_cascade_even_in_a_tracked_store() {
    let g = graph(":A rdfs:subClassOf :B . :x a :A .");
    reason(&g);
    g.sparql_update("DELETE DATA { <http://example.org/x> a <http://example.org/A> }")
        .unwrap();
    assert!(holds(&g, "x", "B"));
    let ledger = g.ledger().unwrap().unwrap();
    assert!(!ledger.fixpoint_reached);
    assert!(!ledger.grounded().contains(&t(
        "<http://example.org/x>",
        "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>",
        "<http://example.org/B>"
    )));
}

#[test]
fn metadata_is_hidden_from_named_graphs_and_explicit_from() {
    let g = graph(":A rdfs:subClassOf :B . :x a :A .");
    reason(&g);
    for query in [
        format!("SELECT * WHERE {{ GRAPH <{LEDGER_GRAPH}> {{ ?s ?p ?o }} }}"),
        "SELECT * WHERE { GRAPH ?g { ?s ?p ?o } }".into(),
    ] {
        for raw in [
            g.sparql_select(&query).unwrap(),
            g.sparql_select_union(&query).unwrap(),
        ] {
            let result: serde_json::Value = serde_json::from_str(&raw).unwrap();
            assert!(
                result["results"].as_array().unwrap().is_empty(),
                "{query}: {raw}"
            );
        }
    }
    let query = format!("SELECT * FROM <{LEDGER_GRAPH}> WHERE {{ ?s ?p ?o }}");
    let plain: serde_json::Value = serde_json::from_str(&g.sparql_select(&query).unwrap()).unwrap();
    assert!(plain["results"].as_array().unwrap().is_empty());
    // The union API deliberately overrides FROM; it must still expose only data.
    let union: serde_json::Value =
        serde_json::from_str(&g.sparql_select_union(&query).unwrap()).unwrap();
    assert_eq!(union["results"].as_array().unwrap().len(), 3);
}

fn metadata(g: &GraphStore) -> std::collections::BTreeSet<String> {
    g.snapshot("nquads")
        .unwrap()
        .lines()
        .filter(|line| line.ends_with(&format!("<{LEDGER_GRAPH}> .")))
        .map(str::to_owned)
        .collect()
}

#[test]
fn an_unrelated_single_quad_write_changes_only_its_records() {
    let g = graph(":A rdfs:subClassOf :B . :x a :A .");
    reason(&g);
    let before = metadata(&g);
    assert!(
        before.len() > 10,
        "records and dependency indexes must be separate"
    );
    assert!(before.iter().all(|q| !q.contains("provenance#ledger>")));
    g.load_turtle(&format!("{PREFIX} :unrelated :p :o ."), None)
        .unwrap();
    let after = metadata(&g);
    assert_eq!(
        before.difference(&after).count(),
        1,
        "only the header is replaced"
    );
    assert_eq!(
        after.difference(&before).count(),
        4,
        "header, evidence, index and event"
    );
    assert_eq!(
        g.triple_count(),
        4,
        "record count must not leak into the data count"
    );
}

#[test]
fn update_where_journals_the_same_bindings_and_preserves_blank_nodes() {
    let g = graph(":A rdfs:subClassOf :B . _:person a :A . :y a :A .");
    reason(&g);
    g.sparql_update_with_retraction(
        "DELETE { ?s a <http://example.org/A> } INSERT { ?s <http://example.org/retired> true } WHERE { ?s a <http://example.org/A> }", true
    ).unwrap();
    assert!(!holds(&g, "y", "B"));
    let triples = g.all_triples().unwrap();
    assert_eq!(
        triples
            .iter()
            .filter(|(_, p, _)| p == "<http://example.org/retired>")
            .count(),
        2
    );
    assert!(!triples.iter().any(|(_, p, o)| p
        == "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"
        && o == "<http://example.org/B>"));
    assert!(
        triples
            .iter()
            .any(|(s, p, _)| s.starts_with("_:") && p == "<http://example.org/retired>")
    );
    assert_eq!(
        metadata(&g).len(),
        g.snapshot("nquads").unwrap().lines().count() - g.triple_count()
    );
}

#[test]
fn failed_reserved_graph_updates_roll_back_the_entire_operation() {
    for tracked in [false, true] {
        let g = graph(":A rdfs:subClassOf :B . :x a :A .");
        if tracked {
            reason(&g);
        }
        let before = g.snapshot("nquads").unwrap();
        for update in [
            format!(
                "INSERT DATA {{ <http://example.org/new> <http://example.org/p> <http://example.org/o> }}; CLEAR GRAPH <{LEDGER_GRAPH}>"
            ),
            format!(
                "INSERT {{ GRAPH ?g {{ <http://example.org/s> <http://example.org/p> <http://example.org/o> }} }} WHERE {{ VALUES ?g {{ <{LEDGER_GRAPH}> }} }}"
            ),
            format!(
                "INSERT DATA {{ GRAPH <{LEDGER_GRAPH}> {{ <https://open-ontologies.org/provenance#state> <https://open-ontologies.org/provenance#header> \"forged\" }} }}"
            ),
            format!("CREATE GRAPH <{LEDGER_GRAPH}>"),
        ] {
            assert!(g.sparql_update(&update).is_err());
            assert_eq!(g.snapshot("nquads").unwrap(), before);
        }
    }
}

#[test]
fn clear_all_preserves_the_reserved_ledger_and_withdraws_user_sources() {
    let g = graph(":A rdfs:subClassOf :B . :x a :A .");
    reason(&g);
    g.sparql_update_with_retraction("CLEAR ALL", true).unwrap();
    assert_eq!(g.triple_count(), 0);
    assert!(g.ledger().unwrap().is_some());
}

#[test]
fn old_single_literal_backups_migrate_to_records_before_further_writes() {
    let g = graph(":A rdfs:subClassOf :B . :x a :A .");
    reason(&g);
    let mut old = g.serialize("ntriples").unwrap();
    old.push_str(&format!(
        "{} .\n",
        g.ledger().unwrap().unwrap().as_quad().unwrap()
    ));
    let restored = GraphStore::new();
    restored.load_nquads(&old).unwrap();
    assert!(
        metadata(&restored)
            .iter()
            .all(|line| !line.contains("provenance#ledger>"))
    );
    restored
        .sparql_update_with_retraction(
            "DELETE DATA { <http://example.org/x> a <http://example.org/A> }",
            true,
        )
        .unwrap();
    assert!(!holds(&restored, "x", "B"));
}

#[test]
fn insert_data_uses_a_fresh_blank_node_scope_for_each_operation() {
    let g = graph("");
    for _ in 0..2 {
        g.sparql_update("INSERT DATA { _:person <http://example.org/p> <http://example.org/o> }")
            .unwrap();
    }
    assert_eq!(g.triple_count(), 2);
    g.sparql_update("INSERT { _:a <http://example.org/p> ?o } WHERE { VALUES ?o { <http://example.org/one> <http://example.org/two> } }").unwrap();
    let triples = g.all_triples().unwrap();
    assert_eq!(
        triples
            .iter()
            .map(|(s, _, _)| s)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4
    );
}

#[test]
fn write_counts_track_actual_data_changes_without_counting_metadata() {
    let g = graph(":A rdfs:subClassOf :B . :x a :A .");
    reason(&g);
    let quad = "<http://example.org/new> <http://example.org/p> <http://example.org/o>";
    assert_eq!(
        g.load_turtle(&format!("{quad} . {quad} ."), None).unwrap(),
        1
    );
    assert_eq!(g.load_turtle(&format!("{quad} ."), None).unwrap(), 0);
    assert_eq!(
        g.sparql_update(&format!("INSERT DATA {{ {quad} }}"))
            .unwrap(),
        0
    );
    assert_eq!(
        g.sparql_update(&format!(
            "DELETE {{ {quad} }} INSERT {{ {quad} }} WHERE {{ {quad} }}"
        ))
        .unwrap(),
        0
    );
    assert_eq!(
        g.sparql_update(&format!(
            "DELETE DATA {{ {quad} }}; INSERT DATA {{ {quad} }}"
        ))
        .unwrap(),
        0
    );
    assert_eq!(g.triple_count(), 4);
}

#[test]
fn update_templates_skip_unbound_terms_and_invalid_graph_names() {
    let g = graph("");
    assert_eq!(
        g.sparql_update(
            "INSERT { <http://example.org/s> <http://example.org/p> ?missing } WHERE {}"
        )
        .unwrap(),
        0
    );
    assert_eq!(g.sparql_update("INSERT { GRAPH ?g { <http://example.org/s> <http://example.org/p> <http://example.org/o> } } WHERE { VALUES ?g { \"\" } }").unwrap(), 0);
    assert_eq!(g.triple_count(), 0);
    assert_eq!(g.sparql_update("INSERT { _:a <http://example.org/p> ?o; <http://example.org/q> ?o } WHERE { VALUES ?o { <http://example.org/one> <http://example.org/two> } }").unwrap(), 4);
    assert_eq!(
        g.all_triples()
            .unwrap()
            .iter()
            .map(|(s, _, _)| s)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        2
    );
}

#[test]
fn repeated_withdrawal_inputs_count_each_source_once() {
    let g = graph(":A rdfs:subClassOf :B . :x a :A .");
    reason(&g);
    let fact = t(
        "<http://example.org/x>",
        "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>",
        "<http://example.org/A>",
    );
    let result = g.retract_assertions(&[fact.clone(), fact]).unwrap();
    assert_eq!(result["removed_assertions"], 1);
    assert!(!holds(&g, "x", "B"));
}
