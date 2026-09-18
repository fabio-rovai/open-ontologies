//! Axiom pinpointing, on graphs whose answer is known by hand.
//!
//! Every fixture here is small enough that the minimal set can be worked out on
//! paper, which is the only way to test a minimiser: a test that asserts
//! whatever the implementation returned tests nothing at all.
//!
//! The negative tests matter more than the positive ones. A pinpointer that
//! returns a superset is not a slightly worse pinpointer, it is a tool that
//! blames the wrong axiom, so there is a test that a non-minimal candidate is
//! REJECTED and a test that a set which does not reach the target is rejected
//! for a different reason and with a different word.

use open_ontologies::graph::GraphStore;
use open_ontologies::justify::{justify, JustifyOptions};
use open_ontologies::reason::Reasoner;
use std::collections::BTreeSet;
use std::sync::Arc;

const NS: &str = "https://example.org/";
const TYPE: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
const SUBCLASS: &str = "<http://www.w3.org/2000/01/rdf-schema#subClassOf>";
const DISJOINT: &str = "<http://www.w3.org/2002/07/owl#disjointWith>";
const TRANSITIVE: &str = "<http://www.w3.org/2002/07/owl#TransitiveProperty>";

fn t(name: &str) -> String {
    format!("<{NS}{name}>")
}

fn store(nt: &str) -> Arc<GraphStore> {
    let g = GraphStore::new();
    g.load_ntriples(nt).expect("the fixture must parse");
    Arc::new(g)
}

fn opts(profile: &str) -> JustifyOptions {
    JustifyOptions {
        profile: profile.to_string(),
        ..Default::default()
    }
}

/// The justifications as sets of `"s p o"` strings, sorted, so an assertion can
/// name the expected set literally.
fn sets(v: &serde_json::Value) -> Vec<BTreeSet<String>> {
    v["justifications"]
        .as_array()
        .expect("justifications must be an array")
        .iter()
        .map(|j| {
            j["triples"]
                .as_array()
                .unwrap()
                .iter()
                .map(|t| {
                    let a = t.as_array().unwrap();
                    format!(
                        "{} {} {}",
                        a[0].as_str().unwrap(),
                        a[1].as_str().unwrap(),
                        a[2].as_str().unwrap()
                    )
                })
                .collect()
        })
        .collect()
}

fn expect(items: &[String]) -> BTreeSet<String> {
    items.iter().cloned().collect()
}

// ───────────────────────────────────────────────────────────────────────────
// A chain, with noise beside it
// ───────────────────────────────────────────────────────────────────────────

/// Six asserted triples, three of which are responsible. A pinpointer that
/// returned all six would be correct about sufficiency and useless.
fn chain_fixture() -> Arc<GraphStore> {
    store(&format!(
        "{a} {TYPE} {c} .\n\
         {c} {SUBCLASS} {d} .\n\
         {d} {SUBCLASS} {e} .\n\
         {b} {TYPE} {f} .\n\
         {f} {SUBCLASS} {g} .\n\
         {g} {SUBCLASS} {h} .\n",
        a = t("a"),
        b = t("b"),
        c = t("C"),
        d = t("D"),
        e = t("E"),
        f = t("F"),
        g = t("G"),
        h = t("H"),
    ))
}

#[test]
fn the_justification_is_the_chain_and_not_the_graph() {
    let g = chain_fixture();
    let target = format!("{} {TYPE} {}", t("a"), t("E"));
    let out = justify(&g, Some(&target), false, None, &opts("rdfs")).unwrap();

    assert_eq!(out["reached"], true, "{out:#}");
    assert_eq!(out["asserted"], 6);
    let js = sets(&out);
    assert_eq!(js.len(), 1, "one derivation path, one justification: {out:#}");
    assert_eq!(
        js[0],
        expect(&[
            format!("{} {TYPE} {}", t("a"), t("C")),
            format!("{} {SUBCLASS} {}", t("C"), t("D")),
            format!("{} {SUBCLASS} {}", t("D"), t("E")),
        ]),
        "{out:#}"
    );
    // The point of the tool, stated as a number: strictly fewer than the
    // asserted graph.
    assert!(js[0].len() < 6);
    assert_eq!(out["minimality_verified"], true);
    assert_eq!(out["truncated"], false);
    assert_eq!(out["complete"], true);
    assert_eq!(out["justifications"][0]["removal_checks"], 3);
    assert_eq!(
        out["justifications"][0]["minimality"],
        "verified_by_re_running_the_fixpoint_without_each_element"
    );
}

/// The whole tool in one line of evidence: the FIRST justification costs no
/// re-run, because the derivation DAG already carries it.
#[test]
fn the_first_support_set_is_read_off_the_dag_before_any_re_run() {
    let g = chain_fixture();
    let target = format!("{} {TYPE} {}", t("a"), t("E"));
    let out = justify(&g, Some(&target), false, None, &opts("rdfs")).unwrap();
    assert_eq!(out["first_justification_from_the_dag"]["size"], 3, "{out:#}");
}

// ───────────────────────────────────────────────────────────────────────────
// The negative tests
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn a_non_minimal_candidate_is_rejected_and_the_useless_triple_is_named() {
    let g = chain_fixture();
    let target = format!("{} {TYPE} {}", t("a"), t("E"));
    // The real justification, plus one triple that has nothing to do with it.
    let candidate = vec![
        format!("{} {TYPE} {}", t("a"), t("C")),
        format!("{} {SUBCLASS} {}", t("C"), t("D")),
        format!("{} {SUBCLASS} {}", t("D"), t("E")),
        format!("{} {TYPE} {}", t("b"), t("F")),
    ];
    let out = justify(&g, Some(&target), false, Some(&candidate), &opts("rdfs")).unwrap();

    assert_eq!(
        out["candidate"]["verdict"], "not_a_justification_not_minimal",
        "a superset of a justification is NOT a justification: {out:#}"
    );
    let removable = out["candidate"]["removable"].as_array().unwrap();
    assert_eq!(removable.len(), 1, "{out:#}");
    assert_eq!(removable[0][0].as_str().unwrap(), t("b"));
    assert_eq!(out["searched"], false, "a candidate is checked, not searched");
}

#[test]
fn a_candidate_that_does_not_reach_the_target_is_rejected_under_a_different_word() {
    let g = chain_fixture();
    let target = format!("{} {TYPE} {}", t("a"), t("E"));
    // The chain with its last link missing.
    let candidate = vec![
        format!("{} {TYPE} {}", t("a"), t("C")),
        format!("{} {SUBCLASS} {}", t("C"), t("D")),
    ];
    let out = justify(&g, Some(&target), false, Some(&candidate), &opts("rdfs")).unwrap();
    assert_eq!(
        out["candidate"]["verdict"], "not_a_justification_target_not_reached",
        "{out:#}"
    );
    assert!(
        out["candidate"]["removable"].as_array().unwrap().is_empty(),
        "nothing is removable from a set that is not a support"
    );
}

#[test]
fn the_real_justification_passes_the_same_check_that_rejects_the_others() {
    // A gate that only ever says no is not a gate. The same code path must
    // accept the set that IS minimal.
    let g = chain_fixture();
    let target = format!("{} {TYPE} {}", t("a"), t("E"));
    let candidate = vec![
        format!("{} {TYPE} {}", t("a"), t("C")),
        format!("{} {SUBCLASS} {}", t("C"), t("D")),
        format!("{} {SUBCLASS} {}", t("D"), t("E")),
    ];
    let out = justify(&g, Some(&target), false, Some(&candidate), &opts("rdfs")).unwrap();
    assert_eq!(out["candidate"]["verdict"], "minimal_justification", "{out:#}");
    assert_eq!(out["candidate"]["checks_run"], 4);
}

// ───────────────────────────────────────────────────────────────────────────
// Two independent derivations are two justifications
// ───────────────────────────────────────────────────────────────────────────

fn two_ways_fixture() -> Arc<GraphStore> {
    store(&format!(
        "{a} {TYPE} {c} .\n\
         {c} {SUBCLASS} {e} .\n\
         {a} {TYPE} {d} .\n\
         {d} {SUBCLASS} {e} .\n",
        a = t("a"),
        c = t("C"),
        d = t("D"),
        e = t("E"),
    ))
}

#[test]
fn a_triple_derived_two_ways_has_two_justifications() {
    let g = two_ways_fixture();
    let target = format!("{} {TYPE} {}", t("a"), t("E"));
    let out = justify(&g, Some(&target), false, None, &opts("rdfs")).unwrap();

    let js = sets(&out);
    assert_eq!(
        js.len(),
        2,
        "the certificate records ONE derivation and there are two; this is the test that \
         reading the certificate back would fail: {out:#}"
    );
    assert!(js.contains(&expect(&[
        format!("{} {TYPE} {}", t("a"), t("C")),
        format!("{} {SUBCLASS} {}", t("C"), t("E")),
    ])));
    assert!(js.contains(&expect(&[
        format!("{} {TYPE} {}", t("a"), t("D")),
        format!("{} {SUBCLASS} {}", t("D"), t("E")),
    ])));
    assert_eq!(out["minimality_verified"], true);
    assert_eq!(out["complete"], true);
}

/// The substrate claim, tested where it lives: the DAG carries both rule
/// instances, and `derivations.tsv` would carry one.
#[test]
fn the_derivation_dag_carries_both_instances_where_a_certificate_carries_one() {
    let g = two_ways_fixture();
    let dg = Reasoner::derivation_graph(&g, "rdfs").unwrap();
    let target = (
        t("a"),
        TYPE.to_string(),
        t("E"),
    );
    let n = dg
        .instances
        .iter()
        .filter(|i| i.conclusion == target)
        .count();
    assert_eq!(n, 2, "two applicable rule instances conclude the same triple");

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    let json = Reasoner::run_full(
        &g,
        "rdfs",
        false,
        open_ontologies::reason::InferenceTarget::DefaultGraph,
        Some(&dir),
    )
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    let lines = std::fs::read_to_string(dir.join("derivations.tsv")).unwrap();
    let recorded = lines
        .lines()
        .filter(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            f.len() > 3 && f[1] == t("a") && f[3] == t("E")
        })
        .count();
    assert_eq!(
        recorded, 1,
        "the certificate records the first derivation only, which is why explanation does not \
         read it: {}",
        v["certificate"]
    );
}

// ───────────────────────────────────────────────────────────────────────────
// The bound, shown firing
// ───────────────────────────────────────────────────────────────────────────

fn six_ways_fixture() -> Arc<GraphStore> {
    let mut nt = String::new();
    for i in 1..=6 {
        nt.push_str(&format!(
            "{a} {TYPE} {c} .\n{c} {SUBCLASS} {e} .\n",
            a = t("a"),
            c = t(&format!("C{i}")),
            e = t("E"),
        ));
    }
    store(&nt)
}

#[test]
fn six_independent_derivations_give_six_justifications() {
    let g = six_ways_fixture();
    let target = format!("{} {TYPE} {}", t("a"), t("E"));
    let out = justify(&g, Some(&target), false, None, &opts("rdfs")).unwrap();
    assert_eq!(out["justification_count"], 6, "{out:#}");
    assert_eq!(out["truncated"], false, "{out:#}");
    assert_eq!(out["complete"], true);
    assert_eq!(out["minimality_verified"], true);
    for j in out["justifications"].as_array().unwrap() {
        assert_eq!(j["size"], 2);
    }
}

#[test]
fn the_truncation_flag_fires_when_the_bound_is_hit() {
    let g = six_ways_fixture();
    let target = format!("{} {TYPE} {}", t("a"), t("E"));
    let out = justify(
        &g,
        Some(&target),
        false,
        None,
        &JustifyOptions {
            max_justifications: 3,
            ..opts("rdfs")
        },
    )
    .unwrap();
    assert_eq!(out["truncated"], true, "{out:#}");
    assert_eq!(out["complete"], false);
    assert_eq!(out["truncation"]["bound"], "max_justifications");
    assert_eq!(out["truncation"]["value"], 3);
    assert_eq!(out["justification_count"], 3);
    // Truncated is not the same as wrong: every justification returned is
    // still sufficient and still minimal, and both were re-run.
    assert_eq!(out["minimality_verified"], true);
}

#[test]
fn the_other_bound_fires_under_its_own_name() {
    let g = six_ways_fixture();
    let target = format!("{} {TYPE} {}", t("a"), t("E"));
    let out = justify(
        &g,
        Some(&target),
        false,
        None,
        &JustifyOptions {
            max_oracle_calls: 6,
            ..opts("rdfs")
        },
    )
    .unwrap();
    assert_eq!(out["truncated"], true, "{out:#}");
    assert_eq!(out["truncation"]["bound"], "max_oracle_calls");
    assert_eq!(out["truncation"]["value"], 6);
}

// ───────────────────────────────────────────────────────────────────────────
// Inconsistency, the half that is worth more
// ───────────────────────────────────────────────────────────────────────────

fn clash_fixture() -> Arc<GraphStore> {
    store(&format!(
        "{c} {DISJOINT} {d} .\n\
         {a} {TYPE} {c0} .\n\
         {c0} {SUBCLASS} {c} .\n\
         {a} {TYPE} {d} .\n\
         {b} {TYPE} {c} .\n",
        a = t("a"),
        b = t("b"),
        c = t("C"),
        c0 = t("C0"),
        d = t("D"),
    ))
}

#[test]
fn the_axioms_responsible_for_a_clash_are_four_of_the_five() {
    let g = clash_fixture();
    let out = justify(&g, None, true, None, &opts("owl-rl")).unwrap();

    assert_eq!(out["reached"], true, "{out:#}");
    assert_eq!(
        out["verdict_explained"], "clash_found_by_this_engine",
        "never the checker's word"
    );
    let js = sets(&out);
    assert_eq!(js.len(), 1, "{out:#}");
    assert_eq!(
        js[0],
        expect(&[
            format!("{} {DISJOINT} {}", t("C"), t("D")),
            format!("{} {TYPE} {}", t("a"), t("C0")),
            format!("{} {SUBCLASS} {}", t("C0"), t("C")),
            format!("{} {TYPE} {}", t("a"), t("D")),
        ]),
        "{out:#}"
    );
    // The individual that is in one of the two classes and not the other is
    // not responsible for anything.
    assert!(!js[0].contains(&format!("{} {TYPE} {}", t("b"), t("C"))));
    assert_eq!(out["minimality_verified"], true);
}

#[test]
fn a_consistent_graph_gets_the_sentence_that_is_not_a_consistency_result() {
    let g = chain_fixture();
    let out = justify(&g, None, true, None, &opts("owl-rl")).unwrap();
    assert_eq!(out["reached"], false, "{out:#}");
    assert!(
        out["note"].as_str().unwrap().contains("NOT a consistency result"),
        "{out:#}"
    );
    assert!(out["justifications"].as_array().unwrap().is_empty());
}

// ───────────────────────────────────────────────────────────────────────────
// Recursion terminates
// ───────────────────────────────────────────────────────────────────────────

/// A transitive property round a three-cycle. Every derived edge's support
/// contains a cycle, which is the case a naive proof-tree walk does not come
/// back from.
pub fn cycle_fixture() -> Arc<GraphStore> {
    store(&format!(
        "{p} {TYPE} {TRANSITIVE} .\n\
         {x} {p} {y} .\n\
         {y} {p} {z} .\n\
         {z} {p} {x} .\n",
        p = t("p"),
        x = t("x"),
        y = t("y"),
        z = t("z"),
    ))
}

#[test]
fn pinpointing_terminates_on_a_cyclic_derivation() {
    let g = cycle_fixture();
    let target = format!("{} {} {}", t("x"), t("p"), t("z"));
    let out = justify(&g, Some(&target), false, None, &opts("owl-rl")).unwrap();

    assert_eq!(out["reached"], true, "{out:#}");
    let js = sets(&out);
    assert_eq!(js.len(), 1, "{out:#}");
    assert_eq!(
        js[0],
        expect(&[
            format!("{} {TYPE} {TRANSITIVE}", t("p")),
            format!("{} {} {}", t("x"), t("p"), t("y")),
            format!("{} {} {}", t("y"), t("p"), t("z")),
        ]),
        "the third edge closes the cycle and is not needed for this conclusion: {out:#}"
    );
    assert_eq!(out["minimality_verified"], true);
    assert_eq!(out["complete"], true);
}

// ───────────────────────────────────────────────────────────────────────────
// Refusals
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn owl_dl_is_refused_rather_than_answered_emptily() {
    let g = chain_fixture();
    let target = format!("{} {TYPE} {}", t("a"), t("E"));
    let err = justify(&g, Some(&target), false, None, &opts("owl-dl")).unwrap_err();
    assert!(
        err.to_string().contains("no rule applications"),
        "got: {err}"
    );
}

#[test]
fn asking_for_both_targets_at_once_is_refused() {
    let g = chain_fixture();
    let target = format!("{} {TYPE} {}", t("a"), t("E"));
    let err = justify(&g, Some(&target), true, None, &opts("rdfs")).unwrap_err();
    assert!(err.to_string().contains("exactly one"), "got: {err}");
    let err = justify(&g, None, false, None, &opts("rdfs")).unwrap_err();
    assert!(err.to_string().contains("exactly one"), "got: {err}");
}

#[test]
fn a_triple_the_engine_does_not_derive_is_said_so_and_not_explained() {
    let g = chain_fixture();
    let target = format!("{} {TYPE} {}", t("b"), t("E"));
    let out = justify(&g, Some(&target), false, None, &opts("rdfs")).unwrap();
    assert_eq!(out["reached"], false, "{out:#}");
    assert!(
        out["note"]
            .as_str()
            .unwrap()
            .contains("not a non-entailment result"),
        "{out:#}"
    );
}

// ───────────────────────────────────────────────────────────────────────────
// Certificates: the one half a theorem covers
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn a_certificate_is_written_per_justification_over_that_justification_alone() {
    let g = two_ways_fixture();
    let target = format!("{} {TYPE} {}", t("a"), t("E"));
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_path_buf();
    let out = justify(
        &g,
        Some(&target),
        false,
        None,
        &JustifyOptions {
            certificate_dir: Some(dir.clone()),
            ..opts("rdfs")
        },
    )
    .unwrap();

    let certs = out["certificates"].as_array().unwrap();
    assert_eq!(certs.len(), 2, "{out:#}");
    for (i, c) in certs.iter().enumerate() {
        let d = dir.join(format!("justification-{i}"));
        let asserted = std::fs::read_to_string(d.join("asserted.tsv")).unwrap();
        assert_eq!(
            asserted.lines().count(),
            2,
            "the certificate is over the justification and nothing else"
        );
        let derivations = std::fs::read_to_string(d.join("derivations.tsv")).unwrap();
        assert!(
            derivations.lines().any(|l| {
                let f: Vec<&str> = l.split('\t').collect();
                f.len() > 3 && f[1] == t("a") && f[3] == t("E")
            }),
            "the target must appear as a CONCLUSION, or the certificate proves nothing about it"
        );
        assert!(c["check_with"].as_str().unwrap().contains("oo-cert"));
    }
}

// ───────────────────────────────────────────────────────────────────────────
// A real ontology
// ───────────────────────────────────────────────────────────────────────────

mod common;

fn repo() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Six-triple fixtures prove the answer is right. This one proves the thing
/// runs at all on an ontology somebody wrote, and it measures the claim the
/// whole design rests on: the DAG carries MORE hyperedges than a certificate
/// carries lines, because a certificate keeps one derivation per conclusion.
#[test]
fn on_a_real_ontology_the_dag_carries_more_than_the_certificate_would() {
    let path = repo().join("benchmark/reference/pizza-reference.owl");
    if common::skip_unless(
        path.is_file(),
        "benchmark/reference/pizza-reference.owl",
        "it ships with the repository; a sparse checkout may have skipped benchmark/",
    ) {
        return;
    }
    let g = Arc::new(GraphStore::new());
    g.load_file(&path.display().to_string()).unwrap();

    let dg = Reasoner::derivation_graph(&g, "rdfs").unwrap();
    assert!(dg.fixpoint_reached, "the fixture must reach a fixpoint");
    let conclusions: BTreeSet<_> = dg.instances.iter().map(|i| &i.conclusion).collect();
    assert!(
        dg.instances.len() > conclusions.len(),
        "a certificate would have {} lines and the DAG has {} hyperedges; if these were equal \
         there would be nothing here that reading derivations.tsv could not do",
        conclusions.len(),
        dg.instances.len()
    );

    // Explain one triple with more than one derivation, since that is the case
    // the certificate cannot represent. Deterministic: the sorted first one.
    let multi: BTreeSet<_> = dg
        .instances
        .iter()
        .map(|i| i.conclusion.clone())
        .filter(|c| dg.instances.iter().filter(|i| &i.conclusion == c).count() > 1)
        .collect();
    let target = multi.iter().next().expect("some triple is derived twice").clone();

    let out = justify(
        &g,
        Some(&format!("{} {} {}", target.0, target.1, target.2)),
        false,
        None,
        &JustifyOptions {
            profile: "rdfs".to_string(),
            max_justifications: 4,
            max_oracle_calls: 60,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(out["reached"], true, "{out:#}");
    assert_eq!(
        out["minimality_verified"], true,
        "every element of every justification was removed and re-run: {out:#}"
    );
    let js = sets(&out);
    assert!(!js.is_empty(), "{out:#}");
    for j in &js {
        assert!(
            j.len() < dg.asserted.len(),
            "a justification the size of the ontology explains nothing"
        );
    }
}
