//! **A verdict must not depend on how fast the machine is (#161).**
//!
//! `reasoner_soundness_test::direct_role_clash_is_detected_control` failed
//! about three runs in five against an unmodified baseline. The cause was that
//! the only bound on tableau BRANCHING was a wall clock: the node and depth
//! budgets cap how big and how deep a tableau gets, and cap nothing about how
//! many branches are explored, so a run that finished on an idle machine ran
//! out of time on a busy one and came back `undecided`.
//!
//! A wall-clock-dependent test in a repository that sells verification is worse
//! than a missing test, because it teaches a reader to discount red.
//!
//! The fix is a deterministic budget beside the clock: expansion steps, shared
//! across the branches of one run through an `Arc<AtomicU64>` so that cloning a
//! tableau per disjunct does not reset the count. These tests are the gate on
//! that. They assert the property directly — the same ontology takes the SAME
//! number of steps, every time — rather than running the flaky test a few more
//! times and hoping.

use open_ontologies::graph::GraphStore;
use open_ontologies::tableaux::DlReasoner;
use std::sync::Arc;

/// The same preamble `reasoner_soundness_test` uses, declarations included.
/// The first version of this file left `ex:D`, `ex:E` and `ex:r` undeclared and
/// the clash went undetected, which looked like the budget change having broken
/// the reasoner and was a broken fixture.
const PREFIXES: &str = r#"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ex: <http://example.org/> .
ex:D a owl:Class . ex:E a owl:Class . ex:D owl:disjointWith ex:E .
ex:r a owl:ObjectProperty . ex:s a owl:ObjectProperty ; owl:inverseOf ex:r .
"#;

/// The ontology from the flaky test, plus disjointness so there is a real
/// clash to find and real branching on the way.
const CLASHING: &str = r#"
ex:B a owl:Class ; rdfs:subClassOf
    [ a owl:Restriction ; owl:onProperty ex:r ; owl:allValuesFrom ex:D ] .
ex:a a ex:B ; ex:r ex:b .
ex:b a ex:E .
"#;

/// Something with genuine disjunction, so the count covers backtracking rather
/// than a straight line. A budget that only ever sees deterministic expansion
/// would not be measuring the thing the clock was there to bound.
const BRANCHING: &str = r#"
ex:P1 a owl:Class . ex:P2 a owl:Class .
ex:Q1 a owl:Class . ex:Q2 a owl:Class .
ex:R1 a owl:Class . ex:R2 a owl:Class .
ex:N a owl:Class .
ex:P a owl:Class ; owl:unionOf ( ex:P1 ex:P2 ) .
ex:Q a owl:Class ; owl:unionOf ( ex:Q1 ex:Q2 ) .
ex:R a owl:Class ; owl:unionOf ( ex:R1 ex:R2 ) .
ex:N owl:disjointWith ex:P1 . ex:N owl:disjointWith ex:P2 .
ex:S a owl:Class ; rdfs:subClassOf ex:P , ex:Q , ex:R , ex:N .
ex:thing a ex:S .
"#;

fn run(body: &str) -> (bool, bool, u64) {
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(&format!("{PREFIXES}{body}"), None)
        .expect("ontology must parse");
    let r = DlReasoner::from_graph(&store).expect("reasoner");
    let a = r.check_abox();
    (a.consistent, a.undecided, a.steps)
}

#[test]
fn the_same_ontology_takes_the_same_number_of_steps_every_time() {
    for (name, body) in [("clashing", CLASHING), ("branching", BRANCHING)] {
        let runs: Vec<_> = (0..6).map(|_| run(body)).collect();
        let first = runs[0];
        assert!(
            runs.iter().all(|r| *r == first),
            "{name}: six runs of one ontology did not agree. A verdict that moves between \
             runs of the same input is the defect #161 recorded, and the step count is what \
             makes it visible: {runs:?}"
        );
        assert!(
            first.2 > 0,
            "{name}: the run took zero steps, so this test is measuring nothing"
        );
    }
}

/// The counter has to survive branching. It lives in an `Arc` because `Tableau`
/// is cloned per disjunct, and a per-clone counter would reset at every branch
/// and bound nothing at all. If that regresses, the branching ontology stops
/// costing more than the straight-line one.
#[test]
fn branching_costs_more_steps_than_a_straight_line() {
    let (_, _, straight) = run(CLASHING);
    let (_, _, branching) = run(BRANCHING);
    assert!(
        branching > straight,
        "a disjunctive ontology took {branching} steps against {straight} for a \
         straight-line one. The counter is not surviving the per-disjunct clone, so it \
         bounds nothing."
    );
}

/// The clash is still found. A deterministic budget that made the reasoner stop
/// answering would be a worse bug than the one it fixes.
#[test]
fn the_clash_is_still_detected_and_the_run_is_not_cut_short() {
    let (consistent, undecided, steps) = run(CLASHING);
    assert!(!undecided, "the run was cut short after {steps} steps");
    assert!(
        !consistent,
        "the direct-role clash was not found, so the budget change broke the reasoner"
    );
}

/// The corpus fits far inside the default, and the number is here so a future
/// reader can see the headroom rather than take 5,000,000 on trust.
#[test]
fn the_default_step_budget_has_real_headroom() {
    let (_, _, a) = run(CLASHING);
    let (_, _, b) = run(BRANCHING);
    let worst = a.max(b);
    assert!(
        worst < 50_000,
        "the worst ontology here now takes {worst} steps. The default is a backstop \
         against exponential backtracking, not a working limit; if real work is \
         approaching it, raise the default deliberately rather than letting runs start \
         coming back undecided."
    );
}
