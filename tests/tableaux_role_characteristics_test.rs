// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Role characteristics the SHIQ tableau claims to support, pinned against the
//! answer a reader would act on.
//!
//! Every test here is a FALSE CLEAN: an ontology that is genuinely inconsistent,
//! for which the reasoner reported `consistent: true` with `undecided: false` —
//! the strongest answer this checker can give. A budget-limited `undecided` is a
//! legitimate answer and is explicitly accepted nowhere below; each test refuses
//! to read an unfinished run as a verdict, because an unfinished run proves
//! nothing either way.

use std::sync::Arc;

use open_ontologies::graph::GraphStore;
use open_ontologies::tableaux::DlReasoner;

const PREFIXES: &str = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
     @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
     @prefix ex: <http://example.org/> .\n";

/// Read the ABox verdict, refusing to accept one the reasoner never proved.
///
/// `consistent` starts true and is falsified by finding a clash, so an
/// unfinished run reports `consistent: true` with `undecided: true`. Reading the
/// first field and ignoring the second treats "I ran out of budget" as "I proved
/// it", which is exactly the confusion these tests exist to catch.
fn abox_is_consistent(body: &str) -> bool {
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(&format!("{PREFIXES}{body}"), None)
        .expect("ontology must parse");
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

// ── 1. owl:AsymmetricProperty is not modelled, and must say so ──────────

/// A construct that is neither implemented nor declared is the worst of both.
///
/// `unmodelled_constructs` exists so a reader can see what the answer did NOT
/// take into account. `owl:AsymmetricProperty` was absent from the tableau AND
/// absent from that list, so an ontology leaning on it got a clean verdict with
/// no indication that a constraint had been dropped on the floor. Modelling it
/// or declaring it are both acceptable; saying nothing is not.
#[test]
fn asymmetric_property_is_declared_unmodelled() {
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            &format!(
                "{PREFIXES}\
                 ex:parentOf a owl:ObjectProperty , owl:AsymmetricProperty .\n\
                 ex:P a owl:Class .\n\
                 ex:a a ex:P ; ex:parentOf ex:b .\n\
                 ex:b a ex:P .\n"
            ),
            None,
        )
        .unwrap();

    let unmodelled = DlReasoner::unmodelled_constructs(&store);
    assert!(
        unmodelled.iter().any(|(label, _)| label == "owl:AsymmetricProperty"),
        "owl:AsymmetricProperty is used by this ontology and is not modelled by the \
         tableau, so it has to appear in the unmodelled list. Got: {unmodelled:?}"
    );
}

// ── 2. The role hierarchy is not closed under inverses ──────────────────

/// `r ⊑ s` entails `r⁻ ⊑ s⁻`, and a domain on `s⁻` has to reach an `r` edge.
///
/// Read the ontology below as a chain of four steps. `ex:a ex:r ex:b` is
/// asserted. `r ⊑ s` gives `ex:a ex:s ex:b`. The inverse of `s` is `ex:sInv`, so
/// `ex:b ex:sInv ex:a` holds in every model. `sInv` has domain `ex:D`, so `ex:b`
/// is a `D`. `ex:b` is asserted an `E`, and `D` and `E` are disjoint. The
/// ontology is inconsistent.
///
/// Only the INVERSE-of-a-super-role case is open: a domain stated directly on
/// `s` already reaches an `r` edge, because each role's domain list is closed
/// over its transitive super-roles when the TBox is processed. What was never
/// computed is that `rInv` is a sub-role of `sInv`, so `sInv`'s domain never
/// folded into `rInv`'s.
#[test]
fn role_hierarchy_is_closed_under_inverses() {
    let inconsistent = !abox_is_consistent(
        "ex:D a owl:Class . ex:E a owl:Class . ex:D owl:disjointWith ex:E .\n\
         ex:r a owl:ObjectProperty . ex:s a owl:ObjectProperty .\n\
         ex:r rdfs:subPropertyOf ex:s .\n\
         ex:rInv a owl:ObjectProperty ; owl:inverseOf ex:r .\n\
         ex:sInv a owl:ObjectProperty ; owl:inverseOf ex:s ; rdfs:domain ex:D .\n\
         ex:a a ex:E ; ex:r ex:b .\n\
         ex:b a ex:E .\n",
    );
    assert!(
        inconsistent,
        "r ⊑ s entails r⁻ ⊑ s⁻, so the domain D of s⁻ binds b, which is asserted E, \
         and D and E are disjoint"
    );
}

// ── 3. Role assertions from an untyped subject are dropped ──────────────

/// An IRI that is never `rdf:type`d still makes assertions, and they still bind.
///
/// `individual_subjects` was built only from subjects carrying an `rdf:type`, so
/// an IRI mentioned solely as the SUBJECT of role assertions contributed no
/// assertions at all — not the edges, not the individual. The object side was
/// already handled: `build_abox_tableau` creates a node for any individual named
/// as the object of a role assertion. The subject side was the hole.
///
/// Below, `ex:a` carries no type. It has two role assertions whose domains are
/// disjoint classes, so it is in both `D` and `E` and the ontology is
/// inconsistent. Nothing else in the ontology can produce the clash.
#[test]
fn role_assertions_from_an_untyped_subject_are_kept() {
    let inconsistent = !abox_is_consistent(
        "ex:D a owl:Class . ex:E a owl:Class . ex:D owl:disjointWith ex:E .\n\
         ex:p a owl:ObjectProperty ; rdfs:domain ex:D .\n\
         ex:q a owl:ObjectProperty ; rdfs:domain ex:E .\n\
         ex:Anchor a owl:Class .\n\
         ex:x a ex:Anchor . ex:y a ex:Anchor .\n\
         ex:a ex:p ex:x .\n\
         ex:a ex:q ex:y .\n",
    );
    assert!(
        inconsistent,
        "ex:a is the subject of a p-edge and a q-edge whose domains are disjoint, so \
         ex:a is in both D and E. It carries no rdf:type, which is not a licence to \
         ignore what it asserts."
    );
}

// ── 4. owl:FunctionalProperty and owl:InverseFunctionalProperty are inert ──

/// A functional property with two distinct fillers in disjoint classes.
///
/// Functionality is encoded as the GCI `≤1 r.⊤`. `add_label` returns early for
/// `Concept::Top` and never stores it, and the ≤-rule counts successors by
/// testing `labels.contains(&filler)`, which for `filler == Top` is false on
/// every node. So the bound was never violated, no merge ever fired, and the
/// module documentation claiming `Fun ✅` was claiming a rule that could not run.
#[test]
fn functional_property_forces_its_two_fillers_together() {
    let inconsistent = !abox_is_consistent(
        "ex:M a owl:Class . ex:W a owl:Class . ex:M owl:disjointWith ex:W .\n\
         ex:P a owl:Class .\n\
         ex:hasFather a owl:ObjectProperty , owl:FunctionalProperty .\n\
         ex:a a ex:P ; ex:hasFather ex:b ; ex:hasFather ex:c .\n\
         ex:b a ex:M .\n\
         ex:c a ex:W .\n",
    );
    assert!(
        inconsistent,
        "hasFather is functional, so b and c are the same individual, and that \
         individual is in both M and W, which are disjoint"
    );
}

/// The same, on `owl:InverseFunctionalProperty`.
///
/// `p` inverse-functional means `p⁻` is functional: two individuals with a
/// `p`-edge to the SAME object are the same individual.
#[test]
fn inverse_functional_property_forces_its_two_subjects_together() {
    let inconsistent = !abox_is_consistent(
        "ex:M a owl:Class . ex:W a owl:Class . ex:M owl:disjointWith ex:W .\n\
         ex:T a owl:Class .\n\
         ex:ssn a owl:ObjectProperty , owl:InverseFunctionalProperty .\n\
         ex:ssnInv a owl:ObjectProperty ; owl:inverseOf ex:ssn .\n\
         ex:n a ex:T .\n\
         ex:b a ex:M ; ex:ssn ex:n .\n\
         ex:c a ex:W ; ex:ssn ex:n .\n",
    );
    assert!(
        inconsistent,
        "ssn is inverse-functional, so b and c are the same individual, and that \
         individual is in both M and W, which are disjoint"
    );
}

/// THREE fillers under `≤1`, where no single merge resolves the bound.
///
/// This is the case that decides whether the ≤-rule may mark a bound processed
/// before it branches. With three successors and a bound of one, merging one
/// pair leaves two, so the bound is still violated and the rule has to fire
/// again on the same node. A rule that fires at most once per node accepts the
/// second state as a model of a constraint it plainly violates, which is the
/// same false clean in a new place. The ontology below is inconsistent: all
/// three fillers are one individual, and that individual is an `M` and a `W`.
#[test]
fn functional_property_merges_until_the_bound_is_met() {
    let inconsistent = !abox_is_consistent(
        "ex:M a owl:Class . ex:W a owl:Class . ex:M owl:disjointWith ex:W .\n\
         ex:N a owl:Class . ex:P a owl:Class .\n\
         ex:hasFather a owl:ObjectProperty , owl:FunctionalProperty .\n\
         ex:a a ex:P ; ex:hasFather ex:b , ex:c , ex:d .\n\
         ex:b a ex:M .\n\
         ex:c a ex:W .\n\
         ex:d a ex:N .\n",
    );
    assert!(
        inconsistent,
        "three fillers of a functional property are one individual, and that \
         individual is in both M and W"
    );
}

/// A functional property whose two fillers are CONSISTENT must stay consistent.
///
/// The failure mode of a merge rule that fires too eagerly is the opposite one:
/// reporting a contradiction that is not there. This is the control.
#[test]
fn functional_property_with_compatible_fillers_stays_consistent() {
    assert!(
        abox_is_consistent(
            "ex:M a owl:Class . ex:P a owl:Class .\n\
             ex:hasFather a owl:ObjectProperty , owl:FunctionalProperty .\n\
             ex:a a ex:P ; ex:hasFather ex:b ; ex:hasFather ex:c .\n\
             ex:b a ex:M .\n\
             ex:c a ex:M .\n"
        ),
        "b and c are both M, so identifying them is harmless and the ABox has a model"
    );
}
