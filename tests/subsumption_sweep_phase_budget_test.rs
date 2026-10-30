//! **A phase budget bounds the subsumption sweep, and the run says which
//! clock stopped it** (#340).
//!
//! The sweep checked only the run's GLOBAL ceiling before building each pair's
//! tableau. Its own phase deadline was handed to the tableau and nowhere else,
//! so once the phase budget had passed, every remaining pair still built a
//! tableau that returned `Unknown` from its first step. A class whose
//! satisfiability test ran out has no model to read, so it contributed a pair
//! for every satisfiable class, and all `k x n` of those were materialised and
//! swept. Measured on `main` 9e73d951 with the issue's 6,000-class ontology,
//! 200 ms phase budget, release build: 5,042 ms in the subsumption phase over
//! 27,454,934 pairs, 27,453,588 of them undetermined.
//!
//! The budget block then reported the bound the SETTINGS predicted: with five
//! phases at 200 ms against a 180 000 ms ceiling it said `binding_bound:
//! "phase"` and that the ceiling "cannot be the bound that stops this run",
//! whatever had actually stopped it.
//!
//! The fixture is the issue's generator scaled down: every class is a subclass
//! of `UNIONS` unions, so every model is built by choice and no class's
//! subsumers can be read off its model. No named class is entailed under any
//! other, so ANY reported subsumption is invented.
//!
//! The assertions are on counters, which a debug build on a slow machine
//! cannot make flaky, plus one wall-clock bound set generously. One test,
//! because it moves process-wide budget settings; see
//! `reasoner_phase_budget_test.rs`.

use std::sync::Arc;

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::Reasoner;
use open_ontologies::runtime;

const CLASSES: usize = 1_500;
const UNIONS: usize = 40;

fn unions() -> String {
    let mut ttl = String::from(
        "@prefix ex: <https://example.org/x/> .\n\
         @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
         @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
    );
    for j in 0..UNIONS {
        ttl.push_str(&format!("ex:P{j} a owl:Class .\nex:Q{j} a owl:Class .\n"));
    }
    for i in 0..CLASSES {
        let supers: Vec<String> = (0..UNIONS)
            .map(|j| format!("rdfs:subClassOf [ a owl:Class ; owl:unionOf ( ex:P{j} ex:Q{j} ) ]"))
            .collect();
        ttl.push_str(&format!("ex:C{i} a owl:Class ; {} .\n", supers.join(" ; ")));
    }
    ttl
}

fn run(store: &Arc<GraphStore>, phase_ms: usize, ceiling_ms: usize) -> serde_json::Value {
    runtime::set_tableaux_test_timeout_ms(phase_ms);
    runtime::set_classify_timeout_ms(ceiling_ms);
    let out = Reasoner::run(store, "owl-dl", false).expect("owl-dl must run");
    serde_json::from_str(&out).unwrap()
}

fn strings(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

#[test]
fn the_subsumption_sweep_stops_at_its_phase_budget_and_names_the_clock_that_stopped_it() {
    let restore_global = runtime::classify_timeout_ms().unwrap_or(0) as usize;
    let restore_phase = runtime::tableaux_test_timeout_ms().unwrap_or(0) as usize;

    let store = Arc::new(GraphStore::new());
    store.load_turtle(&unions(), None).unwrap();

    // ── 1. A 30 ms phase budget under a ceiling ten minutes away. The
    //       satisfiability phase runs out, leaving most classes with no model,
    //       and the sweep has well over a million pairs in front of it.
    const PHASE_MS: u64 = 30;
    let v = run(&store, PHASE_MS as usize, 600_000);
    let sat = &v["agents"]["satisfiability_agent"];
    let sub = &v["agents"]["subsumption_agent"];
    let context = format!("satisfiability_agent={sat} subsumption_agent={sub} budget={}", v["budget"]);
    eprintln!("{context}");

    let undetermined_classes = sat["undetermined"].as_u64().unwrap_or(0);
    assert!(
        undetermined_classes > (CLASSES / 2) as u64,
        "the fixture needs the satisfiability phase to run out of a {PHASE_MS} ms budget on \
         most classes, or the sweep has no k x n tail to outlive it; widen it. {context}"
    );

    let tested = sub["pairs_tested"].as_u64().unwrap_or(0);
    let not_reached = sub["pairs_not_reached"].as_u64();
    let scheduled = sub["pairs_scheduled"].as_u64();
    let (Some(not_reached), Some(scheduled)) = (not_reached, scheduled) else {
        panic!(
            "the sweep has to say how many pairs it never reached, apart from those it \
             tested: once its deadline passes it must stop building tableaux, so the two \
             differ. {context}"
        );
    };
    assert_eq!(tested + not_reached, scheduled, "every pair is tested or not reached: {context}");
    assert!(
        scheduled >= undetermined_classes * (CLASSES as u64 / 2),
        "each undetermined class still stands for a pair per satisfiable class: {context}"
    );
    assert!(
        tested * 10 < scheduled,
        "{tested} of {scheduled} pairs built a tableau under a {PHASE_MS} ms phase budget. \
         After the deadline a tableau can only return Unknown, so building one per \
         remaining pair is the defect: {context}"
    );
    let undetermined_pairs = sub["undetermined"].as_u64().unwrap_or(0);
    assert!(
        undetermined_pairs >= not_reached,
        "a pair that was never reached is undetermined, not refuted: {context}"
    );
    let sub_ms = sub["time_ms"].as_u64().unwrap_or(u64::MAX);
    // Generous for a debug build on a loaded machine, so the counters above are
    // the real guard. Measured on this fixture, debug build: 1 275 and 1 860 ms
    // on `main` 9e73d951, 41 ms with the sweep fixed.
    assert!(
        sub_ms <= PHASE_MS + 1_000,
        "the subsumption phase took {sub_ms} ms under a {PHASE_MS} ms budget: {context}"
    );

    // Soundness: nothing here is entailed, and a cut-short run says so.
    assert_eq!(v["inferred_subsumptions"], 0, "{context}");
    assert_eq!(sub["subsumptions_found"], 0, "{context}");
    assert_eq!(v["complete"], false, "{context}");
    assert_eq!(v["subsumption_sweep_cut_short"], true, "{context}");
    let exhausted = strings(&v["budget_exhausted_in"]);
    assert!(exhausted.contains(&"subsumption".to_string()), "{context}");

    assert_eq!(v["budget"]["binding_bound"], "phase", "{context}");
    assert_eq!(v["budget"]["configured_bound"], "phase", "{context}");
    let note = v["budget"]["note"].as_str().unwrap_or_default();
    assert!(note.contains("A phase budget (30 ms) stopped this run"), "{note}");
    assert!(note.contains("subsumption"), "{note}");

    // ── 2. The settings predict the CEILING: 500 ms is at or below five phases
    //       of 100 ms. But the satisfiability phase spends its 100 ms and the
    //       subsumption phase opens long before the ceiling is 100 ms away, so
    //       its own phase deadline comes first and is what stops it. The run
    //       used to report the prediction, `"global"`, as the bound in force.
    let v = run(&store, 100, 500);
    let b = &v["budget"];
    let ctx = format!("{b} subsumption_agent={}", v["agents"]["subsumption_agent"]);
    eprintln!("{ctx}");
    assert_eq!(b["configured_bound"], "global", "{ctx}");
    assert_eq!(
        b["binding_bound"], "phase",
        "the 100 ms phase budget stopped this run, 400 ms before the ceiling could, and \
         the output has to say so rather than repeat what the settings predicted: {ctx}"
    );
    let note = b["note"].as_str().unwrap_or_default();
    assert!(note.contains("the classify_timeout_ms ceiling did not"), "{note}");
    assert_eq!(v["inferred_subsumptions"], 0, "{ctx}");
    assert_eq!(v["complete"], false, "{ctx}");

    runtime::set_classify_timeout_ms(restore_global);
    runtime::set_tableaux_test_timeout_ms(restore_phase);
}
