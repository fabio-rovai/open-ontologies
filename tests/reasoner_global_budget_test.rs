//! `classify_timeout_ms` bounds a real run, or the run says that it does not.
//!
//! The setting defaults to 180 000 ms and its own documentation calls it "the
//! budget that actually bounds the run". It has never bounded one. Two separate
//! reasons, and both are here:
//!
//!   1. **It could not fire.** Each phase of a classification opens its own
//!      deadline from `tableaux_test_timeout_ms`, which defaults to 10 000 ms.
//!      Four phases had one — consistency, satisfiability, subsumption, ABox —
//!      for a worst case of 40 000 ms, so a 180 000 ms global deadline was dead
//!      arithmetic. Explanation is the fifth and had no clock at all, which is
//!      point 2 below; with it under the same budget the worst case is 50 000 ms
//!      and the ceiling is still dead arithmetic, which is what the run now says
//!      in words. An earlier attempt to fix this by minting a deadline per
//!      TABLEAU did hand the global budget the run, and took a 20-ontology
//!      corpus from 67s to over 600s. It was abandoned, correctly, and the knob
//!      stayed. (That 67s is unreproducible — its corpus was never written down.
//!      `tests/reasoner_budget_corpus_bench.rs` pins two that were.)
//!   2. **It did not cover the run.** `classify_timeout_ms` was read inside
//!      `classify_parallel` and nowhere else, so the TBox consistency check
//!      before it, the ABox check after it and the explanation loop after that
//!      each opened a fresh budget of their own. A run could therefore exceed
//!      `classify_timeout_ms` outright, which is the opposite of a ceiling. The
//!      explanation loop had no wall-clock budget at all: `Tableau::
//!      new_with_tracing` left `Budget::deadline` at `None`.
//!
//! What a knob that reads as a safety limit may not do is enforce nothing. It
//! now enforces a ceiling over every phase, at no cost, because a phase deadline
//! is intersected with the global one rather than minted beside it — and the
//! output SAYS which bound the settings make binding (`configured_bound`) and
//! which bound actually stopped the run (`binding_bound`, #340), instead of
//! leaving a reader to infer either from two settings and a phase count.
//!
//! This file lives on its own and holds ONE test, for the reason
//! `reasoner_phase_budget_test.rs` gives: it moves process-wide budget settings,
//! and `cargo test` runs the tests inside one binary on a shared thread pool.

use std::sync::Arc;

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::Reasoner;
use open_ontologies::runtime;

/// Decided in microseconds under any budget, including none at all.
const TRIVIAL: &str = r#"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix ex: <http://example.org/> .
ex:A a owl:Class .
ex:B a owl:Class .
ex:a a ex:A .
"#;

/// Enough classes that a classification sweep cannot finish inside a
/// millisecond, so a 1 ms global deadline is provably spent before the ABox
/// check is reached. 80 classes is 6320 ordered pairs.
fn wide() -> String {
    let mut ttl = String::from(
        "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
         @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
         @prefix ex: <http://example.org/> .\n",
    );
    for i in 0..80 {
        ttl.push_str(&format!("ex:C{i} a owl:Class .\n"));
        if i > 0 {
            ttl.push_str(&format!("ex:C{i} rdfs:subClassOf ex:C{}.\n", i - 1));
        }
    }
    ttl.push_str("ex:a a ex:C79 .\n");
    ttl
}

fn run(ttl: &str) -> serde_json::Value {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(ttl, None).unwrap();
    let out = Reasoner::run(&store, "owl-dl", false).expect("owl-dl must run");
    serde_json::from_str(&out).unwrap()
}

fn exhausted(v: &serde_json::Value) -> Vec<String> {
    v["budget_exhausted_in"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn the_classification_budget_bounds_the_run_or_the_run_says_it_does_not() {
    let restore_global = runtime::classify_timeout_ms().unwrap_or(0) as usize;
    let restore_phase = runtime::tableaux_test_timeout_ms().unwrap_or(0) as usize;

    // ── 1. The shipped defaults. The global budget cannot fire, and the run
    //       has to say so rather than present 180s as the bound.
    runtime::set_classify_timeout_ms(180_000);
    runtime::set_tableaux_test_timeout_ms(10_000);
    let v = run(TRIVIAL);
    let b = v["budget"].clone();
    assert!(
        b.is_object(),
        "a run has to report which budget was in force. Two settings and a phase \
         count is not a report; it is a puzzle. got {v}"
    );
    assert_eq!(b["classify_timeout_ms"], 180_000, "{b}");
    assert_eq!(b["phase_timeout_ms"], 10_000, "{b}");
    assert_eq!(
        b["phases"].as_array().map(Vec::len),
        Some(5),
        "consistency, satisfiability, subsumption, ABox and explanation each \
         open a phase budget, and the worst case is their sum: {b}"
    );
    assert_eq!(b["phase_worst_case_ms"], 50_000, "{b}");
    assert_eq!(
        b["configured_bound"], "phase",
        "50 000 ms of phase budget under a 180 000 ms ceiling means the settings \
         do not expect the ceiling to stop the run: {b}"
    );
    // What the settings predict is not what happened. Nothing stopped this run,
    // and `binding_bound` reports the bound that actually stopped it (#340): it
    // used to repeat the prediction, and said "phase" over a run the ceiling
    // had stopped.
    assert_eq!(b["binding_bound"], "none", "nothing stopped a trivial run: {b}");
    let note = b["note"].as_str().unwrap_or_default().to_string();
    assert!(
        note.contains("No bound stopped this run") && note.contains("not expected to be the bound"),
        "the note has to say what stopped the run, and state plainly that at these \
         settings the global bound is not the one expected to fire, got {note:?}"
    );

    // ── 2. A global budget smaller than a phase budget is the bound, and it
    //       covers the WHOLE run. The ABox check used to open a fresh deadline
    //       of its own after classification had spent the global one, so a run
    //       could outlast the ceiling it was given.
    runtime::set_classify_timeout_ms(1);
    runtime::set_tableaux_test_timeout_ms(10_000);
    let v = run(&wide());
    assert_eq!(v["budget"]["configured_bound"], "global", "{v}");
    assert_eq!(v["budget"]["binding_bound"], "global", "the ceiling stopped this run: {v}");
    let ms = v["agents"]["total_time_ms"].as_u64().unwrap_or(0);
    assert!(
        ms >= 1,
        "this test needs classification to outlast a 1 ms deadline, and it took \
         {ms} ms. Widen the ontology."
    );
    assert!(
        exhausted(&v).contains(&"abox".to_string()),
        "the global deadline was spent before the ABox check began, so the ABox \
         check ran outside the budget that is supposed to bound the run: {v}"
    );

    // ── 3. The global budget switched off. Say so; do not report a bound.
    runtime::set_classify_timeout_ms(0);
    runtime::set_tableaux_test_timeout_ms(10_000);
    let v = run(TRIVIAL);
    assert_eq!(v["budget"]["classify_timeout_ms"], serde_json::Value::Null, "{v}");
    assert_eq!(v["budget"]["configured_bound"], "phase", "{v}");
    assert_eq!(v["budget"]["binding_bound"], "none", "{v}");
    let note = v["budget"]["note"].as_str().unwrap_or_default().to_string();
    assert!(
        note.contains("No global bound"),
        "with classify_timeout_ms off, the output must state in words that no \
         global bound is in force, got {note:?}"
    );

    // ── 4. No wall-clock bound at all. That is a legitimate configuration and
    //       the least excusable one to leave a reader guessing about.
    runtime::set_classify_timeout_ms(0);
    runtime::set_tableaux_test_timeout_ms(0);
    let v = run(TRIVIAL);
    assert_eq!(v["budget"]["configured_bound"], "none", "{v}");
    assert_eq!(v["budget"]["binding_bound"], "none", "{v}");
    let note = v["budget"]["note"].as_str().unwrap_or_default().to_string();
    assert!(
        note.contains("node") && note.contains("depth"),
        "with both clocks off the only bounds left are the node and depth caps, \
         and the note has to name them, got {note:?}"
    );

    // ── 5. The explanation loop is a phase like any other, and it had no clock
    //       at all. `Tableau::new_with_tracing` leaves `Budget::deadline` at
    //       `None`, and `run` performs one such tableau per unsatisfiable class
    //       in a loop, immediately after a classification that may have just
    //       been cut short for want of exactly that budget.
    runtime::set_classify_timeout_ms(0);
    runtime::set_tableaux_test_timeout_ms(10_000);
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:A a owl:Class . ex:B a owl:Class .
        ex:A owl:disjointWith ex:B .
        ex:Bad a owl:Class ; rdfs:subClassOf ex:A , ex:B .
    "#,
            None,
        )
        .unwrap();
    let reasoner =
        open_ontologies::tableaux::DlReasoner::from_graph(&store).expect("reasoner builds");
    let bad = reasoner
        .named_class_id("http://example.org/Bad")
        .expect("ex:Bad is a named class");
    assert!(
        reasoner.explain_unsatisfiable(bad).is_some(),
        "ex:Bad is a subclass of two disjoint classes and this engine explains it"
    );
    let spent = std::time::Instant::now() + std::time::Duration::from_millis(1);
    std::thread::sleep(std::time::Duration::from_millis(5));
    assert!(
        reasoner.explain_unsatisfiable_within(bad, Some(spent)).is_none(),
        "an explanation is a tableau expansion, and an expired deadline has to \
         stop it. A tableau that ran out of budget returns Unknown, and Unknown \
         is not a proof of unsatisfiability, so there is no clash trace to report."
    );

    // ── 6. The knob has to BE a knob. Neither budget appeared in
    //       `ReasonerConfig`, so neither could be set from `config.toml` at
    //       all: `apply_reasoner` wrote the depth cap, the node cap and the
    //       iteration cap and touched neither clock. Three comments in
    //       `src/tableaux.rs` referred to "`[reasoner] classify_timeout_ms`" as
    //       though it were a setting. The only way to move either was the pair
    //       of setters above, which exist for tests — and the phase one was
    //       documented as "used by the CLI `--reason-timeout-ms` flag", which
    //       does not exist: grepping the whole tree for that string returned one
    //       hit, the sentence claiming it. A ceiling nobody can lower is the
    //       strongest form of a limit that enforces nothing, and the note this
    //       run now prints tells a reader to lower it.
    //
    //       Worse than an unreachable key: `Config` does not deny unknown
    //       fields, so a user who wrote `classify_timeout_ms = 30000` in
    //       `config.toml` got a clean parse, no warning, and no effect. That is
    //       what the assertions below would have caught, and did: run this test
    //       against the pre-fix engine and `classify_timeout_ms()` comes back
    //       holding whatever the previous line set, not 4321.
    let toml = r#"
        [reasoner]
        classify_timeout_ms = 4321
        tableaux_test_timeout_ms = 1234
    "#;
    let cfg: open_ontologies::config::Config =
        toml::from_str(toml).expect("[reasoner] must accept both budgets");
    runtime::init_from_config(&cfg);
    assert_eq!(
        runtime::classify_timeout_ms(),
        Some(4321),
        "config.toml has to be able to set the ceiling"
    );
    assert_eq!(
        runtime::tableaux_test_timeout_ms(),
        Some(1234),
        "and the phase budget"
    );

    // Zero means OFF for a clock, not "use the default". The other three
    // `[reasoner]` numbers read 0 as "unset, use the default", because a depth
    // cap of zero is not a configuration anyone wants; a timeout of zero is.
    let cfg: open_ontologies::config::Config = toml::from_str(
        "[reasoner]\nclassify_timeout_ms = 0\ntableaux_test_timeout_ms = 0\n",
    )
    .unwrap();
    runtime::init_from_config(&cfg);
    assert_eq!(runtime::classify_timeout_ms(), None, "0 is off, not default");
    assert_eq!(runtime::tableaux_test_timeout_ms(), None, "0 is off, not default");

    // And an omitted section leaves the shipped defaults, not zero.
    let cfg: open_ontologies::config::Config = toml::from_str("").unwrap();
    runtime::init_from_config(&cfg);
    assert_eq!(runtime::classify_timeout_ms(), Some(180_000));
    assert_eq!(runtime::tableaux_test_timeout_ms(), Some(10_000));

    runtime::set_classify_timeout_ms(restore_global);
    runtime::set_tableaux_test_timeout_ms(restore_phase);
}
