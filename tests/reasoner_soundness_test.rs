// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

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

// ── Inverse and symmetric roles over asserted ABox edges ────────────────────
//
// An asserted role edge a R b means b has an R-inverse edge to a. A ForAll or
// cardinality constraint that must propagate BACKWARD across the edge relies on
// that. check_abox installed asserted edges one-directional with no inverse
// neighbour, so an inconsistency reachable only through an inverse (or, since a
// symmetric role is its own inverse, a symmetric) role was silently missed and
// the KB reported consistent. The direct-role control isolates the inverse
// handling as the sole cause.
const RS_PREFIXES: &str = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
    @prefix ex: <http://example.org/> .\n\
    ex:D a owl:Class . ex:E a owl:Class . ex:D owl:disjointWith ex:E .\n\
    ex:r a owl:ObjectProperty . ex:s a owl:ObjectProperty ; owl:inverseOf ex:r .\n";

/// Read the ABox verdict, refusing to accept one the reasoner never proved.
///
/// `consistent` starts true and is falsified by finding a clash, so an
/// unfinished run reports `consistent: true` with `undecided: true`. Reading the
/// first field and ignoring the second treats "I ran out of budget" as "I proved
/// it", which is the exact confusion `incomplete_runs_are_declared_in_the_output`
/// exists to prevent, and it made these tests fail intermittently on a loaded
/// machine with a message accusing the reasoner of unsoundness. The engine was
/// right; the reading was wrong. Fail on the unfinished run and say what
/// actually happened.
///
/// This asks `check_abox` directly rather than going through `DlReasoner::run`,
/// and that is not a shortcut, it is what these tests are about. `run` also
/// classifies every class and every ordered pair of classes across a rayon pool
/// sized to the machine, and none of these tests reads the classification. The
/// cost was not just wasted: every tableau in a run shares ONE wall-clock
/// deadline fixed when the reasoner is built, `cargo test` runs this file's
/// tests concurrently, and each opens a pool of its own, so on a machine with
/// fewer cores than that product the subsumption sweep starved, burned the whole
/// deadline on six trivial pairs, and the ABox check that runs after it reported
/// `undecided` on an ontology it decides in under a millisecond. The three tests
/// below failed about half the time on `fol-export` for that reason alone, with
/// no defect in the reasoner. Asking the ABox question directly removes the
/// dependency on a sweep nobody reads. The headline `consistent` flag, which
/// must carry an ABox contradiction rather than report around it, is covered by
/// `tests/tableaux_test.rs::test_dl_asserted_edge_applies_its_domain`, which
/// keeps the whole pipeline in the loop and lives in a binary that does not
/// deliberately exhaust budgets.
fn abox_is_consistent(body: &str) -> bool {
    let ttl = format!("{RS_PREFIXES}{body}");
    let store = Arc::new(GraphStore::new());
    store.load_turtle(&ttl, None).expect("ontology must parse");
    let reasoner = DlReasoner::from_graph(&store).expect("reasoner must build");
    let abox = reasoner.check_abox();
    assert!(
        !abox.undecided,
        "the ABox check hit a budget before finishing, so this run proves nothing \
         either way. Raise [reasoner] tableaux_max_nodes / tableaux_max_depth, or \
         give the run more time, rather than reading an unfinished run as a verdict."
    );
    assert!(
        abox.individuals_checked > 0,
        "no individual was checked, so this test would pass on an empty ABox"
    );
    abox.consistent
}

#[test]
fn direct_role_clash_is_detected_control() {
    // B forces its r-fillers to be D; b is E; D disjoint E. a r b => clash on b.
    let inconsistent = !abox_is_consistent(
        "ex:B a owl:Class ; rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:r ; owl:allValuesFrom ex:D ] .\n\
         ex:a a ex:B ; ex:r ex:b .\n\
         ex:b a ex:E .\n",
    );
    assert!(inconsistent, "a direct-role clash must be detected, or the probe below proves nothing");
}

#[test]
fn inverse_role_clash_is_detected() {
    // Same clash but reachable only through the inverse role s = r-inverse.
    // b:B forces its s-fillers to be D; a r b => b s a => a:D; a also E => clash.
    let inconsistent = !abox_is_consistent(
        "ex:B a owl:Class ; rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:s ; owl:allValuesFrom ex:D ] .\n\
         ex:b a ex:B .\n\
         ex:a a ex:E ; ex:unused ex:noop .\n\
         ex:a ex:r ex:b .\n",
    );
    assert!(inconsistent, "an inconsistency reachable only through an inverse role must be detected");
}

#[test]
fn symmetric_role_clash_is_detected() {
    // A symmetric role is its own inverse. p symmetric, a p b => b p a.
    // B forces p-fillers to D; a:B and a p b => b:D; b:E; D disjoint E => clash.
    let inconsistent = !abox_is_consistent(
        "ex:p a owl:SymmetricProperty .\n\
         ex:B a owl:Class ; rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ; owl:allValuesFrom ex:D ] .\n\
         ex:m a ex:B, ex:E ; ex:p ex:n .\n\
         ex:n a ex:B .\n",
    );
    assert!(inconsistent, "a clash reachable only through a symmetric role's own-inverse edge must be detected");
}

// ── Min-cardinality is not an allocation weapon ─────────────────────────────
//
// The >=-rule materialises n successors from an owl:minCardinality literal that
// has no magnitude cap. The node budget was checked only between fixpoint passes,
// so the inner loop would allocate billions of nodes before the outer guard ran.
// A per-iteration guard turns this into the honest "undecided" answer. (The
// pre-fix behaviour is an out-of-memory kill, not safely run in CI.)
#[test]
fn a_giant_min_cardinality_is_bounded_not_an_oom() {
    let ttl = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
        @prefix ex: <http://example.org/> .\n\
        ex:R a owl:ObjectProperty .\n\
        ex:C a owl:Class ; rdfs:subClassOf \
            [ a owl:Restriction ; owl:onProperty ex:R ; owl:minCardinality 2000000000 ] .\n\
        ex:x a ex:C .\n";
    let result = classify(ttl);
    // Budget exhaustion is never a proof of inconsistency, and it is not a
    // proof of consistency either. This asserted `true` until #355, which was
    // the undecided ABox's default printed as a verdict; the headline is now
    // undetermined, and still never `false`.
    assert_eq!(
        result["consistent"],
        serde_json::Value::Null,
        "a giant min-cardinality must not be reported inconsistent, nor consistent on a run \
         that did not finish: {result}"
    );
    // And the reasoner reports it could not decide the ABox rather than fabricating one.
    assert_eq!(
        result["abox"]["undecided"], true,
        "a giant min-cardinality must be reported undecided, not silently completed: {result}"
    );
}

// ── Anonymous class types on individuals are checked, not dropped ────────────
//
// `:a rdf:type [ owl:Restriction ; owl:onProperty :r ; owl:allValuesFrom :C ]`
// directly types an individual with an anonymous class expression. The collector
// kept only NAMED-class types, so the restriction never reached the tableau and
// an inconsistency arising from it was reported consistent.
#[test]
fn anonymous_class_type_on_individual_is_enforced() {
    // a is directly typed by the anonymous (∀r.D); a r b; b is E.
    // ∀r.D forces b:D; b is E; D disjoint E => clash. Requires the anonymous type
    // to be enforced, which it was not before the fix.
    let inconsistent = !abox_is_consistent(
        "ex:a a [ a owl:Restriction ; owl:onProperty ex:r ; owl:allValuesFrom ex:D ] ; ex:r ex:b .\n\
         ex:b a ex:E .\n\
         ex:a2 a ex:E .\n",
    );
    assert!(
        inconsistent,
        "an inconsistency from an anonymous class type on an individual must be detected"
    );
}

#[test]
fn anonymous_class_type_only_individual_is_still_checked() {
    // b typed ONLY by an anonymous restriction (∀r.D). No named class on b at all.
    // b r c ; c is E ; ∀r.D => c:D ; D disjoint E => clash.
    let inconsistent = !abox_is_consistent(
        "ex:b a [ a owl:Restriction ; owl:onProperty ex:r ; owl:allValuesFrom ex:D ] ; ex:r ex:c .\n\
         ex:c a ex:E .\n",
    );
    assert!(
        inconsistent,
        "an individual typed only by an anonymous class expression must still be checked"
    );
}

// ── rdfs:domain and rdfs:range on an ASSERTED edge ───────────────────────────
//
// `create_successor` was the only place the tableau consulted `role_domain` and
// `role_range`, and `build_abox_tableau` wrote asserted role assertions straight
// into the edge map without going through it. A GENERATED successor therefore
// got its domain and range and an ASSERTED edge got neither, so an ABox made
// inconsistent by an asserted edge alone came back `consistent: true` with
// `undecided: false`, the strongest answer this checker can give. Both paths now
// go through `Tableau::add_role_edge`.

/// The reported shape was `consistent: true` with `undecided: false`, the
/// strongest answer this checker can give, for an ABox with a contradiction in
/// it. That report is asserted on the whole pipeline in
/// `tests/tableaux_test.rs::test_dl_asserted_edge_applies_its_domain`, which
/// lives there because this file deliberately exhausts budgets and would starve
/// it. Here the same repair is checked against `check_abox` directly.
#[test]
fn an_asserted_edge_applies_its_domain_to_the_subject() {
    // domain(p) = D, a is E, D disjoint E, and a has an asserted p-edge out.
    // The edge alone forces a into D, which clashes with E.
    let inconsistent = !abox_is_consistent(
        "ex:p a owl:ObjectProperty ; rdfs:domain ex:D .\n\
         ex:a a ex:E ; ex:p ex:b .\n",
    );
    assert!(
        inconsistent,
        "an asserted edge must apply its rdfs:domain to the subject"
    );
}

#[test]
fn an_asserted_edge_applies_its_range_to_the_object() {
    // range(q) = D, b is E, D disjoint E, and b is the object of an asserted
    // q-edge. The edge alone forces b into D, which clashes with E.
    let inconsistent = !abox_is_consistent(
        "ex:q a owl:ObjectProperty ; rdfs:range ex:D .\n\
         ex:a a ex:E ; ex:q ex:b .\n\
         ex:b a ex:E .\n",
    );
    assert!(
        inconsistent,
        "an asserted edge must apply its rdfs:range to the object"
    );
}

/// A GENERATED successor, for contrast. This path always applied domain and
/// range, and is here so that a regression in the shared primitive cannot be
/// mistaken for the asserted-edge case coming back.
#[test]
fn a_generated_successor_still_applies_its_range_control() {
    // x is a C, C needs an r-successor in E, range(r) = D, D disjoint E. The
    // successor the tableau invents for x cannot be both.
    let inconsistent = !abox_is_consistent(
        "ex:r rdfs:range ex:D .\n\
         ex:C a owl:Class ; rdfs:subClassOf \
             [ a owl:Restriction ; owl:onProperty ex:r ; owl:someValuesFrom ex:E ] .\n\
         ex:x a ex:C .\n",
    );
    assert!(
        inconsistent,
        "a generated successor must apply its rdfs:range"
    );
}

// ── The same constraints through a role's INVERSE ────────────────────────────
//
// `a r b` entails `b inv(r) a` in every model, so domain(inv(r)) binds b and
// range(inv(r)) binds a. NEITHER path consulted the inverse role's domain or
// range at all, which is the same defect one step removed: a constraint stated
// on `s owl:inverseOf r` was invisible to every r-edge in the ontology, asserted
// or generated. A symmetric role is its own inverse in `inverse_roles`, so it
// rides on the same clause.

#[test]
fn a_generated_successor_applies_the_inverse_roles_domain() {
    // x is a C, C needs an r-successor in E. s is the inverse of r and has
    // domain D, so that successor is also a D, and D is disjoint from E.
    let inconsistent = !abox_is_consistent(
        "ex:s rdfs:domain ex:D .\n\
         ex:C a owl:Class ; rdfs:subClassOf \
             [ a owl:Restriction ; owl:onProperty ex:r ; owl:someValuesFrom ex:E ] .\n\
         ex:x a ex:C .\n",
    );
    assert!(
        inconsistent,
        "the inverse role's rdfs:domain must bind the target of the edge"
    );
}

#[test]
fn an_asserted_edge_applies_the_inverse_roles_range() {
    // s is the inverse of r and has range D, so `a r b` makes a a D. a is an E,
    // and D is disjoint from E.
    let inconsistent = !abox_is_consistent(
        "ex:s rdfs:range ex:D .\n\
         ex:a a ex:E ; ex:r ex:b .\n",
    );
    assert!(
        inconsistent,
        "the inverse role's rdfs:range must bind the source of an asserted edge"
    );
}

// ── GCIs reach every node in the ABox tableau ────────────────────────────────
//
// Found in the sweep for the same shape of mistake as the domain/range split,
// and it is the same shape: a node created outside the path that applies the
// constraint. `build_abox_tableau` gives the GCIs to every TYPED individual and
// `create_successor` gives them to every generated successor, but an individual
// named only as the OBJECT of a role assertion got a bare node carrying nothing
// but its own nominal. A GCI holds of every element of the domain, so that node
// was a hole the check could not see into, and an ABox whose only contradiction
// landed there was reported consistent.

#[test]
fn gcis_reach_an_individual_named_only_as_the_object_of_an_edge() {
    // X is equivalent to owl:Thing, which is a GCI saying every element is an X,
    // and no X is an E. B's r-fillers are all E. a is a B with an r-edge to b,
    // so b is an E and an X at once. b has no rdf:type of its own.
    let body = "ex:X a owl:Class ; owl:equivalentClass owl:Thing .\n\
                ex:X owl:disjointWith ex:E .\n\
                ex:B a owl:Class ; rdfs:subClassOf \
                    [ a owl:Restriction ; owl:onProperty ex:r ; owl:allValuesFrom ex:E ] .\n\
                ex:a a ex:B ; ex:r ex:b .\n";
    assert!(
        !abox_is_consistent(body),
        "the GCIs must reach an individual that is only ever an edge's object"
    );

    // Control: the identical ontology with b given a type of its own, so it goes
    // down the path that always applied the GCIs. Both must answer the same way,
    // because whether b happens to carry an rdf:type changes nothing about
    // whether the ontology has a model.
    let typed = "ex:X a owl:Class ; owl:equivalentClass owl:Thing .\n\
                 ex:X owl:disjointWith ex:E .\n\
                 ex:Neutral a owl:Class .\n\
                 ex:B a owl:Class ; rdfs:subClassOf \
                     [ a owl:Restriction ; owl:onProperty ex:r ; owl:allValuesFrom ex:E ] .\n\
                 ex:a a ex:B ; ex:r ex:b .\n\
                 ex:b a ex:Neutral .\n";
    assert!(
        !abox_is_consistent(typed),
        "the control must be inconsistent too, or the test is measuring the wrong thing"
    );
}
