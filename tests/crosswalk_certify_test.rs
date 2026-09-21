//! A crosswalk's match TYPE is a claim, and this is the thing that checks it.
//!
//! `src/align.rs` picks `exactMatch` when label similarity exceeds 0.8 and
//! `closeMatch` otherwise. That is a guess from surface features, and once it
//! is written into a mapping file it is indistinguishable from a fact. Every
//! crosswalk export in this space has that property.
//!
//! The case this repository already measured is the expensive one: the Foundry
//! crosswalk found IES 5.0.3 losing 202 `rdfs:subPropertyOf` axioms against
//! HQDM. A row asserting `exactMatch` across a gap like that is not slightly
//! wrong; it claims two concepts are interchangeable when one side cannot
//! express what the other says.
//!
//! # The direction is pinned by a worked example, not by a docstring
//!
//! `skos:broadMatch` is a sub-property of `skos:broader`, and `A skos:broader
//! B` means **B is the broader concept**. So when the subject is the more
//! specific of the two, the correct predicate is `broadMatch`. Getting that
//! backwards would silently invert every downgrade this tool produces, which
//! is worse than not checking at all, so
//! `the_skos_direction_is_pinned_by_a_worked_example` asserts it on a
//! three-class hierarchy a reader can check by hand.

use std::sync::Arc;

use open_ontologies::crosswalk::{certify, report, to_sssom, Mapping};
use open_ontologies::graph::GraphStore;

const P: &str = "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
                 @prefix owl:  <http://www.w3.org/2002/07/owl#> .\n";

/// The richer side. `Employee` is under `Person`, and `Person` under `Agent`.
/// `Contractor` is deliberately left out of every mapping below, so there is
/// always something for the gap report to find.
const SOURCE: &str = "@prefix s: <http://src.example/> .\n\
    s:Employee   a owl:Class ; rdfs:subClassOf s:Person .\n\
    s:Contractor a owl:Class ; rdfs:subClassOf s:Person .\n\
    s:Person     a owl:Class ; rdfs:subClassOf s:Agent .\n\
    s:Agent      a owl:Class .\n";

/// The poorer side. It HAS a `Person`, but it does not put `Employee` under
/// it: `Employee` is only an `Agent`. So the target says strictly less.
const TARGET: &str = "@prefix t: <http://tgt.example/> .\n\
    t:Employee a owl:Class ; rdfs:subClassOf t:Agent .\n\
    t:Person   a owl:Class ; rdfs:subClassOf t:Agent .\n\
    t:Agent    a owl:Class .\n";

fn store(ttl: &str) -> Arc<GraphStore> {
    let g = Arc::new(GraphStore::new());
    g.load_turtle(&format!("{P}{ttl}"), None).expect("turtle parses");
    g
}

fn m(s: &str, p: &str, o: &str) -> Mapping {
    Mapping { subject: s.to_string(), predicate: p.to_string(), object: o.to_string() }
}

const EXACT: &str = "http://www.w3.org/2004/02/skos/core#exactMatch";

fn full_mapping() -> Vec<Mapping> {
    vec![
        m("http://src.example/Employee", EXACT, "http://tgt.example/Employee"),
        m("http://src.example/Person", EXACT, "http://tgt.example/Person"),
        m("http://src.example/Agent", EXACT, "http://tgt.example/Agent"),
    ]
}

#[test]
fn the_skos_direction_is_pinned_by_a_worked_example() {
    // Worked by hand: s:Employee is under Person AND Agent. t:Employee is
    // under Agent only. So the SUBJECT is the more specific concept, which
    // means subject ⊑ object, which in SKOS is `subject broadMatch object`
    // because broadMatch points AT the broader term.
    let (rows, _) = certify(&store(SOURCE), &store(TARGET), &full_mapping()).expect("certifies");
    let emp = rows.iter().find(|r| r.mapping.subject.ends_with("Employee")).expect("row");
    assert_eq!(
        emp.supported, "broadMatch",
        "the subject is the more specific term, so the object is BROADER and the predicate \
         points at it. If this ever reads narrowMatch the direction has been inverted and \
         every downgrade this tool produces is backwards: {}",
        emp.because
    );
    assert!(emp.downgraded, "the file claimed exactMatch, which the entailments do not support");
}

#[test]
fn a_truthful_exact_match_is_upheld() {
    // s:Person is under Agent; t:Person is under Agent. Same, once translated.
    let (rows, _) = certify(&store(SOURCE), &store(TARGET), &full_mapping()).expect("certifies");
    let p = rows.iter().find(|r| r.mapping.subject.ends_with("/Person")).expect("row");
    assert_eq!(p.supported, "exactMatch", "{}", p.because);
    assert!(!p.downgraded, "a claim the evidence supports must not be reported as downgraded");
}

#[test]
fn an_ancestor_the_target_cannot_express_forces_a_related_match() {
    // THE FOUNDRY CASE. The mapping carries no image for s:Person, so the
    // target cannot express something the source says about s:Employee. No
    // comparison of the two is defensible beyond relatedMatch.
    let partial = vec![
        m("http://src.example/Employee", EXACT, "http://tgt.example/Employee"),
        m("http://src.example/Agent", EXACT, "http://tgt.example/Agent"),
    ];
    let (rows, _) = certify(&store(SOURCE), &store(TARGET), &partial).expect("certifies");
    let emp = rows.iter().find(|r| r.mapping.subject.ends_with("Employee")).expect("row");
    assert_eq!(emp.supported, "relatedMatch", "{}", emp.because);
    assert!(
        emp.untranslatable.iter().any(|u| u.ends_with("/Person")),
        "the report must NAME the ancestor that has no image, or a reader cannot act on it: \
         {:?}",
        emp.untranslatable
    );
    assert!(emp.because.contains("no image"), "and say so in words: {}", emp.because);
}

#[test]
fn a_source_term_with_no_row_is_reported_as_a_gap() {
    // The half that vanishes from every crosswalk export.
    let one = vec![m("http://src.example/Agent", EXACT, "http://tgt.example/Agent")];
    let (_, gaps) = certify(&store(SOURCE), &store(TARGET), &one).expect("certifies");
    let terms: Vec<&str> = gaps.iter().map(|g| g.term.as_str()).collect();
    assert!(
        terms.iter().any(|t| t.ends_with("Employee"))
            && terms.iter().any(|t| t.ends_with("/Person"))
            && terms.iter().any(|t| t.ends_with("Contractor")),
        "every unmapped source class must be listed, not silently absent: {terms:?}"
    );
    assert!(gaps.iter().all(|g| !g.reason.is_empty()), "a gap without a reason is a blank line");
}

#[test]
fn the_output_is_valid_sssom_and_carries_the_supported_predicate() {
    let (rows, gaps) = certify(&store(SOURCE), &store(TARGET), &full_mapping()).expect("certifies");
    let tsv = to_sssom(&rows, &gaps);
    let header = tsv.lines().next().expect("a header");
    for col in ["subject_id", "predicate_id", "object_id", "mapping_justification"] {
        assert!(header.contains(col), "SSSOM requires {col}: {header}");
    }
    // The predicate column must carry what is SUPPORTED. A file still saying
    // exactMatch after this ran would be worse than no check at all, because
    // it would now carry a certification column vouching for it.
    let emp = tsv.lines().find(|l| l.contains("src.example/Employee")).expect("the row");
    assert!(emp.contains("skos:broadMatch"), "the supported predicate belongs in the file: {emp}");
    assert!(emp.contains(EXACT), "and the original claim is kept, so the change is auditable");
    assert!(
        tsv.lines().any(|l| l.contains("sssom:NoMapping")),
        "the gaps travel in the same file; that is the point of them"
    );
}

#[test]
fn the_report_does_not_claim_a_word_it_has_not_earned() {
    // The laundering guard, in the pattern lean_horn_certificate_test set. No
    // Lean checker has a theorem about SKOS match types, so nothing here may
    // wear a checker's word.
    let (rows, gaps) = certify(&store(SOURCE), &store(TARGET), &full_mapping()).expect("certifies");
    let js = report(&rows, &gaps).to_string();
    for forbidden in [
        "certificate_sound",
        "validate_spec",
        "unsat_of_check",
        "\"verdict\":\"certified\"",
        "machine-checked",
    ] {
        assert!(!js.contains(forbidden), "the report claims {forbidden}, which nothing proved");
    }
    assert!(js.contains("checked_by_this_engine"), "it must say whose opinion this is");
    assert!(js.contains("29 of OWL 2 RL's 78 rules"), "and how much of the profile it read");
}

// ───────────────────────────────────────────────────────────────────────────
// The contradiction probe: a disagreement, not a granularity gap
// ───────────────────────────────────────────────────────────────────────────

/// A target that actively DENIES what the mapping carries in.
///
/// `t:Employee` and `t:Machine` are disjoint here. The mapping will say
/// `s:Employee` is `t:Machine`, and the source entails `s:Employee ⊑ s:Person`
/// which maps to `t:Employee`. So the probe carries in `t:Machine ⊑
/// t:Employee`, and the target's own disjointness denies it.
const HOSTILE_TARGET: &str = "@prefix t: <http://tgt.example/> .\n\
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
    @prefix owl:  <http://www.w3.org/2002/07/owl#> .\n\
    t:Employee a owl:Class ; rdfs:subClassOf t:Agent ; owl:disjointWith t:Machine .\n\
    t:Machine  a owl:Class .\n\
    t:Person   a owl:Class ; rdfs:subClassOf t:Agent .\n\
    t:Agent    a owl:Class .\n\
    t:m1 a t:Machine , t:Employee .\n";

#[test]
fn a_target_that_denies_what_the_mapping_carries_is_reported_as_a_contradiction() {
    use open_ontologies::crosswalk::contradiction_probe;
    let bad = vec![
        m("http://src.example/Employee", EXACT, "http://tgt.example/Machine"),
        m("http://src.example/Person", EXACT, "http://tgt.example/Employee"),
        m("http://src.example/Agent", EXACT, "http://tgt.example/Agent"),
    ];
    let js = contradiction_probe(&store(SOURCE), HOSTILE_TARGET, &bad).expect("probes");
    assert_eq!(js["contradicted"], true, "the target denies the carried claim: {js}");
    assert!(
        js["carried"].as_u64().unwrap_or(0) > 0,
        "and the probe must say WHAT it carried across, or the result is unactionable: {js}"
    );
    assert_eq!(js["verdict"], "the_target_denies_what_the_mapping_carried");
}

#[test]
fn a_probe_that_finds_nothing_says_only_that_and_names_its_limit() {
    use open_ontologies::crosswalk::contradiction_probe;
    let target_ttl = format!("{P}{TARGET}");
    let js = contradiction_probe(&store(SOURCE), &target_ttl, &full_mapping()).expect("probes");
    assert_eq!(js["contradicted"], false, "{js}");
    assert_eq!(js["verdict"], "no_contradiction_among_the_rules_this_engine_looks_for");
    // A clean probe that read as "compatible" would be the laundering this
    // repository exists to refuse.
    let means = js["means"].as_str().unwrap_or_default();
    assert!(means.contains("NOT proof of compatibility"), "{means}");
    assert!(means.contains("ten of the seventeen"), "it must state how much it looked at: {means}");
}
