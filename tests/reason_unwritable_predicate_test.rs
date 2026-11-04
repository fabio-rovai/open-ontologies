// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! A conclusion whose PREDICATE is not an IRI.
//!
//! # The defect
//!
//! `rdfs7` reads `s sub o` and `sub rdfs:subPropertyOf super` and concludes
//! `s super o`. Nothing required `super` to be an IRI, and `rdfs:subPropertyOf`
//! takes any term as its object, so a blank node there produced a conclusion
//! with a blank node in predicate position. No RDF serialisation can write one.
//!
//! The materialiser turns the inferred set into one N-Triples document and
//! hands it to `GraphStore::load_ntriples`, which streams: it inserts each quad
//! as it parses it. So the bad line did not make the batch fail cleanly. It
//! made it fail PARTWAY, after an arbitrary number of good triples were already
//! in the store, and `run_full` then returned the parser's error and never
//! wrote the certificate. The store was left holding uncertified inferences and
//! the caller was told the run had failed. The number kept was hash-iteration
//! order, so it differed between runs on identical input: measured at 40, 9 and
//! 24 of 40 on three consecutive runs of the same graph.
//!
//! The same defect for the SUBJECT position was found on 30 August 2026 and
//! guarded at four rule sites (`tests/reason_literal_subject_test.rs`). The
//! predicate position was left open, and `run_horn` was the only path that
//! refused it. Found by `tests/certificate_boundary_proptest.rs` on 14
//! September 2026, at case 115, shrunk to two triples.
//!
//! # Why it matters beyond the crash
//!
//! It is a certificate defect, not only a robustness one. In `--dry-run` the
//! materialiser never runs, so nothing failed and the certificate was written
//! WITH the unserialisable conclusion in it. The Lean checker holds terms as
//! opaque strings and its semantics puts properties in the domain, so it
//! accepts a blank node in predicate position and the certificate checks. It is
//! sound and it describes a triple the store cannot hold.
//!
//! # The fix
//!
//! One guard, in `emit`, covering both positions, shared with `run_horn`
//! (`writable_triple` in `src/reason.rs`). A refused conclusion is not
//! materialised, not certified, and not available as a premise, and the run
//! reports how many it refused.

use std::sync::Arc;

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::{InferenceTarget, Reasoner};

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("oo-unwritable-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

const TYPE: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
const SUBCLASS: &str = "<http://www.w3.org/2000/01/rdf-schema#subClassOf>";
const SUBPROP: &str = "<http://www.w3.org/2000/01/rdf-schema#subPropertyOf>";

/// The shrunken counterexample, exactly as proptest reported it.
#[test]
fn a_blank_node_super_property_does_not_break_the_run() {
    let nt = format!("<http://e/A> <http://e/q> <http://e/A> .\n<http://e/q> {SUBPROP} _:b0 .\n");
    let g = Arc::new(GraphStore::new());
    g.load_ntriples(&nt).unwrap();
    let before = g.triple_count();
    let dir = scratch("blank");

    let out = Reasoner::run_full(&g, "rdfs", true, InferenceTarget::DefaultGraph, Some(&dir))
        .expect("a blank node as the object of rdfs:subPropertyOf is legal RDF");
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();

    assert_eq!(json["skipped_unserialisable"], 1, "{out}");
    assert!(
        json["skipped_examples"][0].as_str().unwrap().contains("(by rdfs7)"),
        "{out}"
    );
    assert_eq!(g.triple_count(), before, "an unwritable triple was materialised");
    assert!(dir.join("asserted.tsv").is_file(), "the certificate was not written");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The realistic route in. `SubObjectPropertyOf(:p ObjectInverseOf(:q))` maps to
/// RDF as `:p rdfs:subPropertyOf [ owl:inverseOf :q ]`, which is a blank node in
/// exactly the position that breaks `rdfs7`. An ordinary OWL 2 ontology was
/// enough to make `reason` fail.
#[test]
fn the_owl_anonymous_inverse_property_idiom_reasons() {
    let ttl = r#"
@prefix : <http://e/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
:p a owl:ObjectProperty ; rdfs:subPropertyOf [ owl:inverseOf :q ] .
:a :p :b .
"#;
    let g = Arc::new(GraphStore::new());
    g.load_turtle(ttl, None).unwrap();
    let dir = scratch("owlidiom");
    let out = Reasoner::run_full(&g, "owl-rl", true, InferenceTarget::DefaultGraph, Some(&dir))
        .expect("a standard OWL 2 ontology must not make the reasoner fail");
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["skipped_unserialisable"], 1, "{out}");
    assert!(dir.join("derivations.tsv").is_file());
    let _ = std::fs::remove_dir_all(&dir);
}

/// Every rule that can reach the defect, each from a fixture that is legal RDF
/// and, in four of the five cases, an ordinary OWL 2 idiom. Measured against the
/// unfixed engine: each of these returned
/// `Parser error: The predicate of a triple must be an IRI` or
/// `... The subject of a triple must be an IRI or a blank node`.
///
/// `cls-avf` is the one to read. It needs no anonymous property expression at
/// all: an `owl:allValuesFrom` restriction plus a literal value is enough, and
/// it concludes `"x" rdf:type :D`. That is the SUBJECT defect, at a fifth rule
/// the 30 August 2026 fix did not reach, and it went unnoticed because that fix
/// was four targeted guards rather than one central one.
///
/// `prp-symp` and `prp-trp` are NOT reachable and are here to say so: both
/// require their property to already be the predicate of a stored triple, so it
/// is an IRI by construction.
#[test]
fn every_rule_that_can_reach_the_defect_is_covered() {
    let prefixes = "@prefix : <http://e/> .\n\
                    @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
                    @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n\
                    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n";
    let cases: [(&str, &str); 5] = [
        // SubObjectPropertyOf(:p ObjectInverseOf(:q)).
        ("rdfs7", ":p rdfs:subPropertyOf [ owl:inverseOf :q ] .\n:a :p :b .\n"),
        ("prp-inv1", ":p owl:inverseOf [ owl:inverseOf :q ] .\n:a :p :b .\n"),
        ("prp-inv2", "[ owl:inverseOf :q ] owl:inverseOf :r .\n:a :q :b .\n"),
        // owl:onProperty on an anonymous inverse property expression.
        (
            "cls-hv1",
            ":C rdfs:subClassOf [ a owl:Restriction ; owl:onProperty [ owl:inverseOf :q ] ; \
             owl:hasValue :v ] .\n:a a :C .\n",
        ),
        // allValuesFrom plus a literal value. No anonymous anything.
        (
            "cls-avf",
            ":C rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :p ; \
             owl:allValuesFrom :D ] .\n:a a :C ; :p \"x\" .\n",
        ),
    ];
    for (rule, body) in cases {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(&format!("{prefixes}{body}"), None).unwrap();
        let dir = scratch(rule);
        let out =
            Reasoner::run_full(&g, "owl-rl-ext", true, InferenceTarget::DefaultGraph, Some(&dir))
                .unwrap_or_else(|e| panic!("{rule}: legal RDF made the reasoner fail: {e}"));
        let json: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(json["skipped_unserialisable"], 1, "{rule}: {out}");
        assert!(
            json["skipped_examples"][0].as_str().unwrap().contains(&format!("(by {rule})")),
            "{rule}: {out}"
        );
        assert!(dir.join("asserted.tsv").is_file(), "{rule}: no certificate");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // Not reachable, and pinned so that a future change that makes them
    // reachable shows up as a surprise rather than as silence.
    for (what, body) in [
        ("prp-symp", "[] a owl:SymmetricProperty .\n:a :p :b .\n"),
        ("prp-trp", "[] a owl:TransitiveProperty .\n:a :p :b .\n:b :p :c .\n"),
    ] {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(&format!("{prefixes}{body}"), None).unwrap();
        let dir = scratch(what);
        let out =
            Reasoner::run_full(&g, "owl-rl-ext", true, InferenceTarget::DefaultGraph, Some(&dir))
                .unwrap();
        let json: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(
            json["skipped_unserialisable"].is_null(),
            "{what} became reachable: {out}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// `owl:inverseOf` whose object is a literal. Garbage as OWL, legal as RDF, and
/// `prp-inv1` used to turn it into a literal in predicate position.
#[test]
fn a_literal_inverse_property_does_not_break_the_run() {
    let nt = "<http://e/p> <http://www.w3.org/2002/07/owl#inverseOf> \"junk\" .\n\
              <http://e/a> <http://e/p> <http://e/b> .\n";
    let g = Arc::new(GraphStore::new());
    g.load_ntriples(nt).unwrap();
    let dir = scratch("litpred");
    let out = Reasoner::run_full(&g, "owl-rl", true, InferenceTarget::DefaultGraph, Some(&dir))
        .expect("a literal object of owl:inverseOf is legal RDF");
    let json: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(json["skipped_unserialisable"], 1, "{out}");
    assert!(
        json["skipped_examples"][0].as_str().unwrap().contains("(by prp-inv1)"),
        "{out}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The consequence that made this more than a crash: the good inferences used to
/// land in the store and the run still reported failure, with no certificate.
/// How many landed depended on hash iteration order, so the store's contents
/// were not a function of its input.
#[test]
fn the_good_inferences_survive_and_the_run_is_deterministic() {
    let mut nt = String::new();
    for i in 0..40 {
        nt.push_str(&format!("<http://e/x{i}> {TYPE} <http://e/A> .\n"));
    }
    nt.push_str(&format!("<http://e/A> {SUBCLASS} <http://e/B> .\n"));
    // The poison: two triples that make rdfs7 conclude a blank-node predicate.
    nt.push_str("<http://e/A> <http://e/p> <http://e/A> .\n");
    nt.push_str(&format!("<http://e/p> {SUBPROP} _:b0 .\n"));

    let mut sizes = Vec::new();
    for trial in 0..5 {
        let g = Arc::new(GraphStore::new());
        g.load_ntriples(&nt).unwrap();
        let dir = scratch(&format!("partial{trial}"));
        let out = Reasoner::run_full(&g, "rdfs", true, InferenceTarget::DefaultGraph, Some(&dir))
            .expect("one unwritable conclusion must not fail the whole batch");
        let json: serde_json::Value = serde_json::from_str(&out).unwrap();
        // The 40 rdfs9 conclusions are unaffected by the one refusal.
        assert_eq!(json["inferred_count"], 40, "{out}");
        assert!(dir.join("asserted.tsv").is_file());
        sizes.push(g.triple_count());
        let _ = std::fs::remove_dir_all(&dir);
    }
    assert!(
        sizes.windows(2).all(|w| w[0] == w[1]),
        "the store's size after the run depends on hash iteration order: {sizes:?}"
    );
}

/// A dry run must not certify what a materialising run refuses. Before the fix
/// the two disagreed: `--dry-run` wrote a certificate containing the
/// unserialisable conclusion, and materialising failed on it.
#[test]
fn a_dry_run_certifies_exactly_what_a_materialising_run_does() {
    let nt = format!("<http://e/A> <http://e/q> <http://e/A> .\n<http://e/q> {SUBPROP} _:b0 .\n");

    let wet = Arc::new(GraphStore::new());
    wet.load_ntriples(&nt).unwrap();
    let dw = scratch("wet");
    Reasoner::run_full(&wet, "rdfs", true, InferenceTarget::DefaultGraph, Some(&dw)).unwrap();

    let dry = Arc::new(GraphStore::new());
    dry.load_ntriples(&nt).unwrap();
    let dd = scratch("dry");
    Reasoner::run_full(&dry, "rdfs", false, InferenceTarget::DefaultGraph, Some(&dd)).unwrap();

    let a = std::fs::read_to_string(dw.join("derivations.tsv")).unwrap();
    let b = std::fs::read_to_string(dd.join("derivations.tsv")).unwrap();
    assert_eq!(a, b, "a dry run and a materialising run certified different things");
    assert!(
        !a.contains("_:b0\t"),
        "an unserialisable conclusion was certified: {a:?}"
    );
    let _ = std::fs::remove_dir_all(&dw);
    let _ = std::fs::remove_dir_all(&dd);
}
