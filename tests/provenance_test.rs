//! Provenance semirings, on graphs whose annotation is known by hand.
//!
//! The polynomials here are checked EXACTLY, string and all. A test that
//! asserts "two monomials" and not which two would pass on an implementation
//! that lost one derivation and invented another.
//!
//! The recursion test is the one that matters. Datalog is recursive, the
//! counting semiring diverges on a cycle, and the failure mode this whole file
//! exists to prevent is a finite number printed with no indication that the
//! true one is infinite.

use open_ontologies::graph::GraphStore;
use open_ontologies::justify::{justify, JustifyOptions};
use open_ontologies::provenance::{annotate, ProvenanceOptions, Semiring};
use open_ontologies::reason::Spelled;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const NS: &str = "https://example.org/";
const TYPE: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
const SUBCLASS: &str = "<http://www.w3.org/2000/01/rdf-schema#subClassOf>";
const TRANSITIVE: &str = "<http://www.w3.org/2002/07/owl#TransitiveProperty>";

fn t(name: &str) -> String {
    format!("<{NS}{name}>")
}

fn store(nt: &str) -> Arc<GraphStore> {
    let g = GraphStore::new();
    g.load_ntriples(nt).expect("the fixture must parse");
    Arc::new(g)
}

fn rdfs(semirings: Vec<Semiring>) -> ProvenanceOptions {
    ProvenanceOptions {
        profile: "rdfs".to_string(),
        semirings,
        ..Default::default()
    }
}

/// `a rdf:type E` derived two independent ways, and nothing else.
fn two_ways() -> Arc<GraphStore> {
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

fn target_ae() -> String {
    format!("{} {TYPE} {}", t("a"), t("E"))
}

/// A monomial as a sorted set of `"s p o"` strings.
fn monomials(v: &serde_json::Value) -> Vec<BTreeSet<String>> {
    v["semirings"]["why"]["monomials"]
        .as_array()
        .expect("why must have monomials")
        .iter()
        .map(|m| {
            m["triples"]
                .as_array()
                .unwrap()
                .iter()
                .map(|tr| {
                    let a = tr.as_array().unwrap();
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

fn weight(map: &[(&str, &str, &str, f64)]) -> BTreeMap<Spelled, f64> {
    map.iter()
        .map(|(s, p, o, w)| ((s.to_string(), p.to_string(), o.to_string()), *w))
        .collect()
}

// ───────────────────────────────────────────────────────────────────────────
// Two derivations, two monomials, checked exactly
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn a_triple_derived_two_ways_has_a_polynomial_with_exactly_two_monomials() {
    let out = annotate(&two_ways(), Some(&target_ae()), &rdfs(vec![Semiring::Why])).unwrap();
    let why = &out["semirings"]["why"];

    assert_eq!(why["monomial_count"], 2, "{out:#}");
    // Variables are numbered over the asserted triples in sorted order, so the
    // polynomial is canonical and can be pinned as a string.
    assert_eq!(why["polynomial"], "x1*x3 + x2*x4", "{out:#}");
    assert_eq!(out["variables"]["x1"][0], t("C"));
    assert_eq!(out["variables"]["x2"][0], t("D"));
    assert_eq!(out["variables"]["x3"][2], t("C"));
    assert_eq!(out["variables"]["x4"][2], t("D"));

    let ms = monomials(&out);
    assert!(ms.contains(&BTreeSet::from([
        format!("{} {TYPE} {}", t("a"), t("C")),
        format!("{} {SUBCLASS} {}", t("C"), t("E")),
    ])));
    assert!(ms.contains(&BTreeSet::from([
        format!("{} {TYPE} {}", t("a"), t("D")),
        format!("{} {SUBCLASS} {}", t("D"), t("E")),
    ])));
    assert_eq!(why["stabilised"], true);
    assert_eq!(why["truncated"], false);
    assert_eq!(why["monomials_are_minimal_supports"], true);
}

#[test]
fn the_counting_semiring_counts_both_proof_trees_and_says_the_number_is_exact() {
    let out = annotate(&two_ways(), Some(&target_ae()), &rdfs(vec![Semiring::Counting])).unwrap();
    let c = &out["semirings"]["counting"];
    assert_eq!(c["value"], "2", "{out:#}");
    assert_eq!(c["value_is_exact"], true);
    assert_eq!(c["saturated"], false);
    assert_eq!(out["cycle_in_support"], false);
}

#[test]
fn lineage_is_the_union_and_is_therefore_not_a_justification() {
    // The honest limit of the coarsest semiring, tested rather than only
    // documented: lineage says four triples are involved, and neither
    // justification has four triples in it.
    let out = annotate(&two_ways(), Some(&target_ae()), &rdfs(vec![Semiring::Lineage])).unwrap();
    assert_eq!(out["semirings"]["lineage"]["size"], 4, "{out:#}");

    let j = justify(
        &two_ways(),
        Some(&target_ae()),
        false,
        None,
        &JustifyOptions {
            profile: "rdfs".to_string(),
            ..Default::default()
        },
    )
    .unwrap();
    for jj in j["justifications"].as_array().unwrap() {
        assert_eq!(jj["size"], 2);
    }
}

#[test]
fn the_boolean_semiring_is_derivability_and_nothing_more() {
    let out = annotate(&two_ways(), Some(&target_ae()), &rdfs(vec![Semiring::Boolean])).unwrap();
    assert_eq!(out["semirings"]["boolean"]["value"], true, "{out:#}");
    assert_eq!(out["semirings"]["boolean"]["absorptive"], false);
}

/// Feature 1 and feature 2 must agree on the same graph, or one of them is
/// wrong. The why-semiring's monomials and the verified justifications are the
/// same sets.
#[test]
fn the_why_monomials_are_the_justifications_the_re_runs_verify() {
    let g = two_ways();
    let prov = annotate(&g, Some(&target_ae()), &rdfs(vec![Semiring::Why])).unwrap();
    let algebra: BTreeSet<BTreeSet<String>> = monomials(&prov).into_iter().collect();

    let j = justify(
        &g,
        Some(&target_ae()),
        false,
        None,
        &JustifyOptions {
            profile: "rdfs".to_string(),
            ..Default::default()
        },
    )
    .unwrap();
    let verified: BTreeSet<BTreeSet<String>> = j["justifications"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| {
            x["triples"]
                .as_array()
                .unwrap()
                .iter()
                .map(|tr| {
                    let a = tr.as_array().unwrap();
                    format!(
                        "{} {} {}",
                        a[0].as_str().unwrap(),
                        a[1].as_str().unwrap(),
                        a[2].as_str().unwrap()
                    )
                })
                .collect()
        })
        .collect();

    assert_eq!(
        algebra, verified,
        "the algebra and the re-runs disagree about who is responsible"
    );
}

// ───────────────────────────────────────────────────────────────────────────
// The scalar semirings, with weights
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn min_plus_takes_the_cheapest_proof_tree_and_max_min_its_weakest_link() {
    let weights = weight(&[
        (&t("a"), TYPE, &t("C"), 5.0),
        (&t("C"), SUBCLASS, &t("E"), 5.0),
        (&t("a"), TYPE, &t("D"), 1.0),
        (&t("D"), SUBCLASS, &t("E"), 1.5),
    ]);
    let out = annotate(
        &two_ways(),
        Some(&target_ae()),
        &ProvenanceOptions {
            weights: weights.clone(),
            ..rdfs(vec![Semiring::TropicalMinPlus])
        },
    )
    .unwrap();
    // min(5 + 5, 1 + 1.5) = 2.5
    assert_eq!(out["semirings"]["tropical"]["value"], 2.5, "{out:#}");

    // The same weights with `trust` asked for are refused, because max-min is
    // a semiring on [0, 1] and a weight above its multiplicative identity
    // would make a conjunction come out smaller than the algebra says.
    let err = annotate(
        &two_ways(),
        Some(&target_ae()),
        &ProvenanceOptions {
            weights,
            ..rdfs(vec![Semiring::TrustMaxMin])
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("above 1.0"), "got: {err}");

    // With confidences in [0,1] the max-min reading is the natural one.
    let conf = weight(&[
        (&t("a"), TYPE, &t("C"), 0.4),
        (&t("C"), SUBCLASS, &t("E"), 0.9),
        (&t("a"), TYPE, &t("D"), 0.7),
        (&t("D"), SUBCLASS, &t("E"), 0.6),
    ]);
    let out = annotate(
        &two_ways(),
        Some(&target_ae()),
        &ProvenanceOptions {
            weights: conf,
            ..rdfs(vec![Semiring::TrustMaxMin])
        },
    )
    .unwrap();
    // max(min(0.4, 0.9), min(0.7, 0.6)) = max(0.4, 0.6) = 0.6
    assert_eq!(out["semirings"]["trust"]["value"], 0.6, "{out:#}");
}

#[test]
fn the_default_weight_makes_min_plus_the_size_of_the_smallest_proof_tree() {
    let out = annotate(
        &two_ways(),
        Some(&target_ae()),
        &rdfs(vec![Semiring::TropicalMinPlus]),
    )
    .unwrap();
    assert_eq!(out["semirings"]["tropical"]["value"], 2.0, "{out:#}");
}

#[test]
fn a_negative_weight_is_refused_rather_than_allowed_to_diverge() {
    // min-plus is absorptive only for non-negative weights: with a negative
    // one, going round a cycle keeps lowering the cost and the fixpoint does
    // not converge. Refusing is the honest answer; looping is not.
    let bad = weight(&[(&t("a"), TYPE, &t("C"), -1.0)]);
    let err = annotate(
        &two_ways(),
        Some(&target_ae()),
        &ProvenanceOptions {
            weights: bad,
            ..rdfs(vec![Semiring::TropicalMinPlus])
        },
    )
    .unwrap_err();
    assert!(
        err.to_string().contains("non-negative"),
        "got: {err}"
    );
}

// ───────────────────────────────────────────────────────────────────────────
// Recursion: the honest half
// ───────────────────────────────────────────────────────────────────────────

/// A transitive property round a three-cycle, so every derived edge's support
/// contains a cycle.
fn cycle() -> Arc<GraphStore> {
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

fn owl(semirings: Vec<Semiring>, depth: usize) -> ProvenanceOptions {
    ProvenanceOptions {
        profile: "owl-rl".to_string(),
        semirings,
        depth_bound: depth,
        ..Default::default()
    }
}

#[test]
fn a_cycle_is_named_as_a_cycle_before_any_number_is_read() {
    let target = format!("{} {} {}", t("x"), t("p"), t("z"));
    let out = annotate(&cycle(), Some(&target), &owl(vec![Semiring::Why], 8)).unwrap();
    assert_eq!(out["cycle_in_support"], true, "{out:#}");
    assert!(out["cycle_witness"].is_array(), "{out:#}");
    assert!(out["cycle_means"]
        .as_str()
        .unwrap()
        .contains("CANNOT be exact"));
}

#[test]
fn the_counting_semiring_reports_its_bound_instead_of_a_finite_lie() {
    let target = format!("{} {} {}", t("x"), t("p"), t("z"));

    let at3 = annotate(&cycle(), Some(&target), &owl(vec![Semiring::Counting], 3)).unwrap();
    let at5 = annotate(&cycle(), Some(&target), &owl(vec![Semiring::Counting], 5)).unwrap();

    // The numbers are the count of proof trees of height at most the bound,
    // which is why they are different. Pinned exactly: a run that returned the
    // same number for both bounds would not be bounding anything.
    assert_eq!(at3["semirings"]["counting"]["value"], "4", "{at3:#}");
    assert_eq!(at5["semirings"]["counting"]["value"], "289", "{at5:#}");

    for out in [&at3, &at5] {
        let c = &out["semirings"]["counting"];
        assert_eq!(c["stabilised"], false, "{out:#}");
        assert_eq!(c["value_is_exact"], false, "{out:#}");
        assert!(c["value_means"]
            .as_str()
            .unwrap()
            .contains("LOWER BOUND"));
    }
    assert_eq!(at3["semirings"]["counting"]["depth_bound"], 3);
    assert_eq!(at5["semirings"]["counting"]["depth_bound"], 5);
    assert_eq!(at3["semirings"]["counting"]["rounds_run"], 3);
}

#[test]
fn the_absorptive_semiring_converges_on_the_same_cycle() {
    let target = format!("{} {} {}", t("x"), t("p"), t("z"));
    let out = annotate(&cycle(), Some(&target), &owl(vec![Semiring::Why], 32)).unwrap();
    let why = &out["semirings"]["why"];
    assert_eq!(why["stabilised"], true, "{out:#}");
    assert_eq!(why["monomial_count"], 1, "{out:#}");
    assert_eq!(why["polynomial"], "x1*x2*x3");
    // It stopped long before the bound, which is what "converges" has to mean
    // if the word is to carry any weight.
    assert!(why["rounds_run"].as_u64().unwrap() < 32);
    assert_eq!(monomials(&out)[0].len(), 3);
}

#[test]
fn an_asserted_triple_absorbs_its_own_cyclic_derivation() {
    // `x p y` is asserted AND derivable, round the cycle, infinitely many
    // ways. Absorption is the reason the answer is one monomial of size one
    // rather than a series.
    let target = format!("{} {} {}", t("x"), t("p"), t("y"));
    let out = annotate(&cycle(), Some(&target), &owl(vec![Semiring::Why], 32)).unwrap();
    assert_eq!(out["target_is_asserted"], true, "{out:#}");
    assert_eq!(out["semirings"]["why"]["monomial_count"], 1, "{out:#}");
    assert_eq!(monomials(&out)[0].len(), 1);
    assert!(out["derivations_of_target"].as_u64().unwrap() >= 1);
}

// ───────────────────────────────────────────────────────────────────────────
// The monomial cap
// ───────────────────────────────────────────────────────────────────────────

fn six_ways() -> Arc<GraphStore> {
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
fn six_derivations_give_six_monomials_and_the_cap_says_when_it_bit() {
    let full = annotate(&six_ways(), Some(&target_ae()), &rdfs(vec![Semiring::Why])).unwrap();
    assert_eq!(full["semirings"]["why"]["monomial_count"], 6, "{full:#}");
    assert_eq!(full["semirings"]["why"]["truncated"], false);
    assert_eq!(full["semirings"]["why"]["monomials_are_minimal_supports"], true);

    let capped = annotate(
        &six_ways(),
        Some(&target_ae()),
        &ProvenanceOptions {
            max_monomials: 2,
            ..rdfs(vec![Semiring::Why])
        },
    )
    .unwrap();
    let why = &capped["semirings"]["why"];
    assert_eq!(why["monomial_count"], 2, "{capped:#}");
    assert_eq!(why["truncated"], true, "{capped:#}");
    assert_eq!(
        why["monomials_are_minimal_supports"], false,
        "a truncated run must withdraw the minimality claim, not keep it"
    );
    assert!(why["truncation_means"].as_str().unwrap().contains("INCOMPLETE"));
}

// ───────────────────────────────────────────────────────────────────────────
// Refusals and the empty cases
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn a_triple_outside_the_closure_gets_a_sentence_and_no_expression() {
    let target = format!("{} {TYPE} {}", t("zzz"), t("E"));
    let out = annotate(&two_ways(), Some(&target), &rdfs(vec![Semiring::Why])).unwrap();
    assert_eq!(out["in_closure"], false, "{out:#}");
    assert!(out.get("semirings").is_none(), "{out:#}");
    assert!(out["note"]
        .as_str()
        .unwrap()
        .contains("NOT a non-entailment result"));
}

#[test]
fn no_target_reports_the_shape_of_the_dag_and_annotates_nothing() {
    let out = annotate(&two_ways(), None, &rdfs(vec![Semiring::Why])).unwrap();
    assert_eq!(out["asserted"], 4, "{out:#}");
    assert_eq!(out["derived"], 1);
    assert_eq!(out["rule_instances"], 2);
    assert!(out.get("semirings").is_none());
}

#[test]
fn asking_for_no_semiring_is_refused() {
    let err = annotate(&two_ways(), Some(&target_ae()), &rdfs(vec![])).unwrap_err();
    assert!(err.to_string().contains("nothing to compute"), "got: {err}");
}

#[test]
fn owl_dl_is_refused_here_too() {
    let err = annotate(
        &two_ways(),
        Some(&target_ae()),
        &ProvenanceOptions {
            profile: "owl-dl".to_string(),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("derivation DAG"), "got: {err}");
}

#[test]
fn every_semiring_name_round_trips() {
    for s in Semiring::all() {
        assert_eq!(Semiring::parse(s.name()), Some(s), "{}", s.name());
        assert!(!s.convergence().is_empty());
        assert!(!s.means().is_empty());
    }
    assert_eq!(Semiring::parse("not_a_semiring"), None);
}
