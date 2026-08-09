//! Soundness regressions for the OWL-DL tableaux reasoner.
//!
//! These tests use ontologies whose satisfiability is a mathematical certainty,
//! so they need no reference reasoner to establish ground truth:
//!
//!   An ontology containing no negation, no disjointness, no owl:Nothing and no
//!   max-cardinality restriction is ALWAYS satisfiable, and every class in it is
//!   satisfiable. Witness model: a single element x, every role interpreted as
//!   {(x,x)}, and x a member of every class. Every GCI C ⊑ D holds because x is
//!   in both; every ∃R.C holds because R(x,x) and x ∈ C.
//!
//! So if the reasoner reports "unsatisfiable" for any class in such an ontology,
//! it is wrong, full stop.

use std::sync::Arc;

use open_ontologies::graph::GraphStore;
use open_ontologies::tableaux::DlReasoner;

/// Build a Turtle ontology that is a chain of `n` existential restrictions:
/// C0 ⊑ ∃R.C1, C1 ⊑ ∃R.C2, ... No negation anywhere.
fn existential_chain(n: usize) -> String {
    let mut s = String::from(
        "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
         @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
         @prefix ex: <http://example.org/> .\n\
         ex:R a owl:ObjectProperty .\n",
    );
    for i in 0..=n {
        s.push_str(&format!("ex:C{i} a owl:Class .\n"));
    }
    for i in 0..n {
        s.push_str(&format!(
            "ex:C{i} rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:R ; \
             owl:someValuesFrom ex:C{next} ] .\n",
            next = i + 1
        ));
    }
    s
}

fn classify(ttl: &str) -> serde_json::Value {
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(ttl, None).expect("ontology must parse");
    let out = DlReasoner::run(&graph, false).expect("reasoner must return a result");
    serde_json::from_str(&out).expect("reasoner output must be JSON")
}

/// A negation-free ontology is satisfiable by construction. Every class in it
/// must be reported satisfiable.
#[test]
fn negation_free_chain_has_no_unsatisfiable_classes() {
    let result = classify(&existential_chain(40));

    let unsat = result
        .get("unsatisfiable_classes")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    assert!(
        unsat.is_empty(),
        "negation-free ontology cannot have unsatisfiable classes, but the \
         reasoner reported {unsat:?}. Full result: {result}"
    );

    assert_eq!(
        result.get("consistent").and_then(|v| v.as_bool()),
        Some(true),
        "negation-free ontology must be consistent. Full result: {result}"
    );
}

/// The important one. Resource exhaustion is NOT evidence of unsatisfiability.
///
/// `Tableau::expand` returns `bool`, where `false` means "clash found, this
/// branch is unsatisfiable". It also returns `false` when the node budget or
/// branch-depth budget is exhausted. Those two situations are not the same
/// thing, and conflating them makes the reasoner assert subsumptions that do
/// not hold.
///
/// This test drives a chain long enough to stress the expansion and asserts the
/// reasoner never converts "I ran out of budget" into "this class is impossible".
#[test]
fn resource_exhaustion_is_not_reported_as_unsatisfiability() {
    // Long enough to be expensive, short enough to keep the test quick.
    let result = classify(&existential_chain(200));

    let unsat = result
        .get("unsatisfiable_classes")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    assert!(
        unsat.is_empty(),
        "reasoner reported unsatisfiable classes {unsat:?} in an ontology with \
         no negation, no disjointness and no max-cardinality. Every class here \
         is satisfiable in a one-element model. This is a soundness failure, \
         most likely resource exhaustion in Tableau::expand being returned as a \
         clash. Full result: {result}"
    );
}

/// Build a branching ontology: C0 ⊑ ≥3 R.C1, C1 ⊑ ≥3 R.C2, ...
///
/// Expanding C0 forces 3^depth blockable successors, blowing past the 10,000
/// node budget with only a handful of classes. Still negation-free, so still
/// satisfiable by construction: take a 3-element domain, interpret R as the
/// complete relation on it, and put all three elements in every class. Every
/// ≥3 R.C is then satisfied and no axiom can be violated because none is
/// negative.
fn branching_chain(depth: usize) -> String {
    let mut s = String::from(
        "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
         @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
         @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
         @prefix ex: <http://example.org/> .\n\
         ex:R a owl:ObjectProperty .\n",
    );
    for i in 0..=depth {
        s.push_str(&format!("ex:C{i} a owl:Class .\n"));
    }
    for i in 0..depth {
        s.push_str(&format!(
            "ex:C{i} rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:R ; \
             owl:minQualifiedCardinality \"3\"^^xsd:nonNegativeInteger ; \
             owl:onClass ex:C{next} ] .\n",
            next = i + 1
        ));
    }
    s
}

/// THE WITNESS. Exceeds the tableau node budget on a provably satisfiable
/// ontology.
///
/// `Tableau::expand` opens with:
///
/// ```ignore
/// if depth > max_depth || self.nodes.len() > max_nodes {
///     return false;
/// }
/// ```
///
/// `false` is the same value the function returns for a genuine clash, so the
/// caller reads "I exhausted my budget" as "this concept is unsatisfiable".
/// In classification that becomes a spurious `owl:Nothing` subsumption: the
/// reasoner asserts, with no hedging, that a perfectly consistent class cannot
/// have any instances.
#[test]
fn node_budget_exhaustion_does_not_fabricate_unsatisfiability() {
    // 3^12 successors dwarfs the 10,000-node default budget.
    let result = classify(&branching_chain(12));

    let unsat = result
        .get("unsatisfiable_classes")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    assert!(
        unsat.is_empty(),
        "SOUNDNESS FAILURE: reasoner reported {unsat:?} as unsatisfiable. This \
         ontology has no negation, no disjointness, no max-cardinality and no \
         owl:Nothing, so it is satisfiable in a 3-element model where R is the \
         complete relation and every class contains every element. The reasoner \
         has converted resource exhaustion into a claim of impossibility. \
         Full result: {result}"
    );
}

/// Whatever the reasoner decides, it must not silently claim a complete answer
/// when it gave up. If any budget was hit, that has to be visible in the output.
#[test]
fn incomplete_runs_are_declared_in_the_output() {
    let result = classify(&existential_chain(200));

    // The reasoner must expose SOME field telling the caller whether the run
    // was complete. Absence of such a field means a caller cannot distinguish
    // "proved" from "gave up", which is the defect this test guards.
    let has_completeness_signal = result.get("complete").is_some()
        || result.get("incomplete").is_some()
        || result.get("exhausted").is_some()
        || result.get("limits_hit").is_some();

    assert!(
        has_completeness_signal,
        "reasoner output carries no completeness signal, so a caller cannot \
         tell a proof from a timeout. Full result: {result}"
    );
}
