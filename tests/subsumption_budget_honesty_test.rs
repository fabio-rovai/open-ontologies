//! **A classification that says it is complete must be the classification.**
//!
//! The subsumption sweep decides `A ⊑ B` by asking whether `A ⊓ ¬B` is
//! unsatisfiable, and it read the three-valued answer with `.is_unsat()`. A test
//! that ran out of budget came back `Unknown`, `.is_unsat()` turned that into
//! `false`, and the pair was dropped exactly as if it had been refuted. Nothing
//! recorded it: `subsumption_cut_short` was set only when the run's GLOBAL clock
//! ran out, never when one test's own budget did. So a run could hand back a
//! hierarchy with subsumptions missing, `complete: true`, and an empty
//! `budget_exhausted_in`.
//!
//! Measured on ORE 2015 `ore_ont_8937` (37 classes): two runs of the same
//! binary on the same file returned 36 and then 57 subsumptions, both with
//! `budget_exhausted_in: []`, against 329 from HermiT.
//!
//! That defect is pinned by a deterministic unit test beside `sweep_pairs` in
//! `src/tableaux.rs`, which hands the sweep an `Unknown` directly. The real
//! failure needs a phase CLOCK to run out partway through a sweep, and a test
//! that depends on a wall clock is the flake `#161` removed. This file guards
//! the property the defect broke, end to end, against the deterministic step
//! ceiling instead: it sweeps that ceiling from tiny to unbounded and asserts,
//! at every setting, that the hierarchy is a SUBSET of the unbounded one,
//! because running out of budget may lose answers but must never invent them,
//! and that a run reporting `complete: true` is EQUAL to the unbounded one,
//! because that is what the word means. It also requires that some setting in
//! the sweep actually cuts the run short, or it would prove nothing.
//!
//! It did not fail against the old sweep, and the reason is worth keeping: on
//! this fixture the reverse half of `ex:B`'s definition is a GCI on every node,
//! so every satisfiability test costs at least as many steps as every pair
//! test, and no per-test step budget can cut the pairs without also cutting a
//! class, which the old code did report. It is a guard on classification as a
//! whole, which the absorption and model-reading changes rewrite.
//!
//! This file moves a PROCESS-WIDE setting, so it holds one test and restores
//! the setting on the way out, for the reason `reasoner_phase_budget_test`
//! gives.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::Reasoner;
use open_ontologies::runtime;

/// `ex:A ⊑ ex:B` holds, and proving it means refuting every disjunct of
/// `¬ex:B`, which is a five-way disjunction. Deciding that `ex:A` is
/// satisfiable needs none of that, so there is a band of step budgets in
/// which every class is decided and the subsumption test is not.
const ONTOLOGY: &str = r#"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ex: <http://example.org/> .
ex:r a owl:ObjectProperty .
ex:A a owl:Class ; rdfs:subClassOf ex:E1 , ex:E2 , ex:E3 , ex:E4 ,
    [ a owl:Restriction ; owl:onProperty ex:r ; owl:someValuesFrom ex:C ] .
ex:C a owl:Class ; rdfs:subClassOf ex:D .
ex:D a owl:Class .
ex:E1 a owl:Class . ex:E2 a owl:Class . ex:E3 a owl:Class . ex:E4 a owl:Class .
ex:B a owl:Class ; owl:equivalentClass [ owl:intersectionOf (
    ex:E1 ex:E2 ex:E3 ex:E4
    [ a owl:Restriction ; owl:onProperty ex:r ; owl:someValuesFrom ex:D ] ) ] .
"#;

type Hierarchy = BTreeMap<String, BTreeSet<String>>;

fn classify(store: &Arc<GraphStore>) -> (Hierarchy, bool, serde_json::Value) {
    let out = Reasoner::run(store, "owl-dl", false).expect("reason must run");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let mut h: Hierarchy = BTreeMap::new();
    for row in v["classification"].as_array().unwrap() {
        let sups: BTreeSet<String> = row["superclasses"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_string())
            .collect();
        h.insert(row["class"].as_str().unwrap().to_string(), sups);
    }
    let complete = v["complete"].as_bool().unwrap();
    (h, complete, v)
}

#[test]
fn a_run_that_reports_complete_is_the_unbounded_classification() {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(ONTOLOGY, None).unwrap();

    let previous = runtime::tableaux_max_steps().unwrap_or(0) as usize;

    runtime::set_tableaux_max_steps(0);
    let (reference, ref_complete, _) = classify(&store);
    assert!(ref_complete, "the unbounded run must finish");
    let a = "<http://example.org/A>";
    let b = "<http://example.org/B>";
    assert!(
        reference.get(a).is_some_and(|s| s.contains(b)),
        "the fixture is meant to entail A ⊑ B and the unbounded run must find it: {reference:?}"
    );

    let mut saw_incomplete = false;
    let mut failures: Vec<String> = Vec::new();
    for steps in (1..=60).chain((70..=400).step_by(10)).chain([600, 1_000, 5_000, 50_000]) {
        runtime::set_tableaux_max_steps(steps);
        let (h, complete, v) = classify(&store);
        for (cls, sups) in &h {
            let allowed = reference.get(cls).cloned().unwrap_or_default();
            let invented: Vec<&String> = sups.difference(&allowed).collect();
            if !invented.is_empty() {
                failures.push(format!(
                    "steps={steps}: {cls} gained {invented:?}, which the unbounded run does not entail"
                ));
            }
        }
        if complete && h != reference {
            failures.push(format!(
                "steps={steps}: complete=true but the hierarchy differs from the unbounded run; \
                 budget_exhausted_in={} subsumption_agent={}",
                v["budget_exhausted_in"], v["agents"]["subsumption_agent"]
            ));
        }
        if !complete {
            saw_incomplete = true;
            let named = v["budget_exhausted_in"].as_array().unwrap();
            if named.is_empty() {
                failures.push(format!(
                    "steps={steps}: complete=false and budget_exhausted_in is empty"
                ));
            }
        }
    }

    runtime::set_tableaux_max_steps(previous);

    assert!(
        saw_incomplete,
        "no step budget in the sweep cut the run short, so this test proved nothing"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
