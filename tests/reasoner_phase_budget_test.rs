// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Each phase of a reasoning run gets its own budget.
//!
//! `DlReasoner` fixed ONE wall-clock deadline when it was constructed and shared
//! it across the satisfiability sweep, the subsumption sweep and the ABox check.
//! A classification that used the clock up left the ABox check starting already
//! expired, so it reported `undecided` on an ontology it decides in under a
//! millisecond. That is honest rather than false, which is why it survived, but
//! it means a loaded machine silently degrades an answer that was available.
//!
//! This file lives on its own because it moves a PROCESS-WIDE budget setting.
//! `cargo test` runs each test binary as its own process but runs the tests
//! inside one binary on a shared thread pool, so a file that shrinks the global
//! timeout must not contain anything else that a shrunken timeout would break.
//! Hence one test, and it restores the setting on the way out.

use std::sync::Arc;
use std::time::Duration;

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::Reasoner;
use open_ontologies::runtime;
use open_ontologies::tableaux::DlReasoner;

/// A budget older than the phase that spends it is not a budget.
///
/// The ontology is trivial: two classes, one individual, decidable in
/// microseconds. The test makes the per-test budget short, builds the reasoner,
/// and then lets more than that budget elapse before asking the ABox question —
/// which is what a real classification sweep does to the ABox check when it runs
/// long. A deadline minted at construction is already gone by then and the
/// answer comes back `undecided`. A budget minted when the phase STARTS is not,
/// and the answer comes back decided.
#[test]
fn a_phase_does_not_inherit_a_budget_an_earlier_phase_already_spent() {
    const BUDGET_MS: usize = 250;

    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:A a owl:Class .
        ex:B a owl:Class .
        ex:a a ex:A .
    "#,
            None,
        )
        .unwrap();

    let previous = runtime::tableaux_test_timeout_ms().unwrap_or(0) as usize;
    runtime::set_tableaux_test_timeout_ms(BUDGET_MS);

    let reasoner = DlReasoner::from_graph(&store).expect("reasoner must build");
    // Stand in for an earlier phase that ran long. Nothing about the ABox check
    // got slower; the clock it was handed simply belongs to somebody else.
    std::thread::sleep(Duration::from_millis(BUDGET_MS as u64 * 2));
    let abox = reasoner.check_abox();

    runtime::set_tableaux_test_timeout_ms(previous);

    assert!(
        !abox.undecided,
        "the ABox check reported undecided on an ontology it decides in microseconds, \
         because it inherited a deadline that was fixed when the reasoner was built and \
         had already elapsed. Each phase must get its own budget."
    );
    assert!(abox.consistent, "this ABox has a model");
    assert_eq!(abox.individuals_checked, 1);

    // The other half of the repair: a run that is not complete has to say WHICH
    // phase ran out, because the three take their budget from different settings
    // and "incomplete" on its own tells a reader nothing they can act on. On a
    // run that finishes, the list is present and empty — a reader must be able to
    // distinguish "no phase ran out" from "this build does not report it".
    let out = Reasoner::run(&store, "owl-dl", false).expect("reason must run");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["complete"], true, "this ontology is decided in microseconds: {v}");
    assert_eq!(
        v["budget_exhausted_in"],
        serde_json::json!([]),
        "a complete run names no phase, and must still carry the field: {v}"
    );
}
