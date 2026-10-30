//! **The tableaux verdict, checked against a model rather than against an oracle.**
//!
//! `tests/owl2_conformance_test.rs` asserts what this engine's `owl-dl` path
//! says, and every expectation in it is sourced from a comment reading
//! `HermiT: consistent ✓`. Those assertions are real, so the file is a gate on
//! this engine. What it is not is a LIVE second opinion: HermiT is not a
//! dependency here, it is not installed, and nothing re-runs it. Twenty
//! expectations rest on a transcription nobody can re-check.
//!
//! Every other layer has a live differential. First-order has E and Vampire,
//! SHACL has pySHACL, the certificate layer has Isabelle and now Rocq. The
//! description-logic path had none.
//!
//! This closes that, and closes it BETTER than HermiT would. Decision 0006:
//! a model is a finite object a verified checker can validate, so a `sat`
//! answer becomes a certificate, while `unsat` stays testimony. So when the
//! tableaux says an ontology is consistent, this does not ask a second
//! reasoner whether it agrees. It asks for a MODEL and hands that model to
//! `oo-folmodel`, which re-evaluates every formula against the structure. The
//! verdict `model_checked` is a machine-checked statement that the ontology
//! has a model, and a second reasoner's agreement is not.
//!
//! It also needs no Java. HermiT would mean a JDK and the OWL API in CI; this
//! uses Z3, which the `lean` job already installs for exactly this layer.
//!
//! **The inconsistent direction is deliberately weaker, and says so.** When
//! the tableaux reports an inconsistency, no model exists, so there is nothing
//! to check and the solver's `unsat` is an oracle opinion. What is asserted
//! there is only the thing that would be a real defect: that no CHECKED MODEL
//! is produced for an ontology this engine called inconsistent. A checked
//! model for an inconsistent ontology would mean one of the two is wrong, and
//! that is worth catching even though its absence proves nothing.

mod common;

use open_ontologies::fol_solve::{SolveOptions, Solver, solve_export};
use open_ontologies::graph::GraphStore;
use open_ontologies::reason::Reasoner;
use std::sync::Arc;

const PREFIXES: &str = r#"
    @prefix ex: <http://example.org/> .
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
"#;

/// Cases the conformance file states with a frozen `HermiT:` comment, and what
/// this engine says about each.
fn consistent_cases() -> Vec<(&'static str, &'static str)> {
    vec![
        ("a subsumption chain", "ex:A a owl:Class . ex:B a owl:Class . ex:C a owl:Class . \
                                 ex:A rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:C ."),
        ("disjoint classes with no shared member", "ex:A a owl:Class . ex:B a owl:Class . \
                                 ex:A owl:disjointWith ex:B . ex:x a ex:A ."),
        ("a domain and a range", "ex:p a owl:ObjectProperty . ex:p rdfs:domain ex:A . \
                                 ex:p rdfs:range ex:B . ex:x ex:p ex:y ."),
    ]
}

fn inconsistent_cases() -> Vec<(&'static str, &'static str)> {
    vec![
        ("an individual in two disjoint classes", "ex:A a owl:Class . ex:B a owl:Class . \
                                 ex:A owl:disjointWith ex:B . ex:x a ex:A . ex:x a ex:B ."),
    ]
}

fn store(body: &str) -> Arc<GraphStore> {
    let g = Arc::new(GraphStore::new());
    g.load_turtle(&format!("{PREFIXES}{body}"), None).expect("ontology must parse");
    g
}

fn tableaux_says_consistent(g: &Arc<GraphStore>) -> bool {
    let out = Reasoner::run(g, "owl-dl", false).expect("owl-dl run");
    let v: serde_json::Value = serde_json::from_str(&out).expect("report is JSON");
    v["consistent"].as_bool().expect("the owl-dl report states consistency")
}

/// The ontology-level verdict from the model layer.
fn model_verdict(g: &Arc<GraphStore>, name: &str) -> String {
    let dir = std::env::temp_dir().join(format!("oo-dl-diff-{}-{}", std::process::id(), name.len()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let opts = SolveOptions {
        solver: Solver::Z3,
        // Small, because these are three-class ontologies and the ladder stops
        // at the first model. A large ceiling would only slow the negative
        // case, which has to exhaust it.
        max_domain: 4,
        timeout_secs: 20,
        unbounded_probe: false,
        checker: None,
    };
    let report = solve_export(g, &dir, &opts, None, 0).expect("solve_export");
    let v: serde_json::Value = serde_json::from_str(&report).expect("report is JSON");
    let verdict = v["ontology"]["verdict"].as_str().unwrap_or("<none>").to_string();
    let _ = std::fs::remove_dir_all(&dir);
    verdict
}

fn skip() -> bool {
    let has_z3 = std::process::Command::new("z3").arg("--version").output().is_ok();
    let checker = open_ontologies::fol_solve::find_checker(None).is_ok();
    common::skip_unless(
        has_z3 && checker,
        "z3 and the oo-folmodel checker",
        "install z3, and build lean/ with `cd lean && lake build`",
    )
}

#[test]
fn a_consistent_ontology_gets_a_checked_model_and_not_merely_a_second_opinion() {
    if skip() {
        return;
    }
    for (name, body) in consistent_cases() {
        let g = store(body);
        assert!(
            tableaux_says_consistent(&g),
            "{name}: this engine called it inconsistent, so the case is mislabelled here"
        );
        let verdict = model_verdict(&g, name);
        assert_eq!(
            verdict, "model_checked",
            "{name}: the tableaux says consistent and the model layer returned {verdict:?}. \
             `model_checked` is the only verdict that rests on a machine-checked theorem: the \
             structure was handed to oo-folmodel and every formula re-evaluated against it. \
             Anything else means the two halves of this engine disagree, or the model could \
             not be checked, and both are worth stopping for"
        );
    }
}

#[test]
fn an_inconsistent_ontology_never_gets_a_checked_model() {
    if skip() {
        return;
    }
    for (name, body) in inconsistent_cases() {
        let g = store(body);
        assert!(
            !tableaux_says_consistent(&g),
            "{name}: this engine called it consistent, so the case is mislabelled here"
        );
        let verdict = model_verdict(&g, name);
        assert_ne!(
            verdict, "model_checked",
            "{name}: this engine's tableaux says the ontology is inconsistent and the model \
             layer produced a CHECKED model of it. One of the two is wrong, and the checked \
             half is the one carrying a theorem. This is the assertion in this file worth \
             having: the other direction only establishes that no model was found, which is \
             not evidence of inconsistency"
        );
    }
}

/// Inconsistent ontologies whose inconsistency rests on a construct the FOL
/// reader cannot translate. Each is the smallest witness from the 29 Sep 2026
/// reproduction: the export is a strictly weaker theory, and it HAS a model.
fn out_of_fragment_inconsistent_cases() -> Vec<(&'static str, &'static str)> {
    vec![
        ("owl:AllDisjointProperties", "ex:p a owl:ObjectProperty . ex:q a owl:ObjectProperty . \
                                 [] a owl:AllDisjointProperties ; owl:members ( ex:p ex:q ) . \
                                 ex:a ex:p ex:b ; ex:q ex:b ."),
        ("owl:NegativePropertyAssertion", "ex:p a owl:ObjectProperty . ex:a ex:p ex:b . \
                                 [] a owl:NegativePropertyAssertion ; \
                                 owl:sourceIndividual ex:a ; owl:assertionProperty ex:p ; \
                                 owl:targetIndividual ex:b ."),
        ("a functional datatype property with two values", "ex:d a owl:DatatypeProperty , \
                                 owl:FunctionalProperty . ex:a ex:d 1 , 2 ."),
    ]
}

/// The report is about the EXPORT, and must say when that is not the ontology.
///
/// Before this, `solve_export` never read the reader's drop ledger. Each case
/// below came back `model_checked`, with `not_asked: []` and no flag, for a
/// graph with no models at all: a certified verdict about a different problem.
/// The verdict word may stay, because it is a true statement about the problem
/// the checker was handed. What may not happen is a report that lets it be
/// read as a statement about the ontology.
#[test]
fn a_model_of_a_weaker_export_is_never_reported_as_a_model_of_the_ontology() {
    if skip() {
        return;
    }
    for (name, body) in out_of_fragment_inconsistent_cases() {
        let g = store(body);
        let dir = std::env::temp_dir().join(format!(
            "oo-dl-weaker-{}-{}",
            std::process::id(),
            name.len()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let opts = SolveOptions {
            solver: Solver::Z3,
            max_domain: 4,
            timeout_secs: 20,
            unbounded_probe: false,
            checker: None,
        };
        let report = solve_export(&g, &dir, &opts, None, 0).expect("solve_export");
        let _ = std::fs::remove_dir_all(&dir);
        let v: serde_json::Value = serde_json::from_str(&report).expect("report is JSON");
        let o = &v["ontology"];
        assert_eq!(
            v["exports_a_weaker_axiom_set"], true,
            "{name}: the reader dropped the construct the inconsistency rests on, and the \
             report does not say so: {v}"
        );
        assert!(
            v["constructs_not_exported"]
                .as_array()
                .is_some_and(|a| !a.is_empty()),
            "{name}: the flag without the list names nothing a reader can act on: {v}"
        );
        // The case is only a test of the report if the weaker export really
        // does have a model. If a later change makes the reader translate the
        // construct, the verdict stops being model_checked and this case has
        // to be replaced rather than silently passing.
        assert_eq!(
            o["verdict"], "model_checked",
            "{name}: expected the weaker export to have a checked model. If the construct is \
             now translated, move this case to inconsistent_cases(): {v}"
        );
        assert_eq!(
            o["transfers_to_the_ontology"], false,
            "{name}: a model of a weaker theory was reported as carrying over to an \
             ontology that has no model: {v}"
        );
        assert_eq!(
            o["owl_reading"],
            serde_json::Value::Null,
            "{name}: an OWL-level reading was minted over a weaker export: {v}"
        );
    }
}

/// The control: in the fragment, nothing is dropped and a checked model does
/// transfer. Without this, `transfers_to_the_ontology: false` everywhere would
/// pass the test above.
#[test]
fn a_model_of_a_faithful_export_transfers() {
    if skip() {
        return;
    }
    let (name, body) = consistent_cases()[1];
    let g = store(body);
    let dir = std::env::temp_dir().join(format!("oo-dl-faithful-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let opts = SolveOptions {
        solver: Solver::Z3,
        max_domain: 4,
        timeout_secs: 20,
        unbounded_probe: false,
        checker: None,
    };
    let report = solve_export(&g, &dir, &opts, None, 0).expect("solve_export");
    let _ = std::fs::remove_dir_all(&dir);
    let v: serde_json::Value = serde_json::from_str(&report).expect("report is JSON");
    assert_eq!(v["exports_a_weaker_axiom_set"], false, "{name}: {v}");
    assert_eq!(v["ontology"]["verdict"], "model_checked", "{name}: {v}");
    assert_eq!(
        v["ontology"]["transfers_to_the_ontology"], true,
        "{name}: {v}"
    );
}
