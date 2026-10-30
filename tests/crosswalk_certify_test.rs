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
    assert_eq!(p.certification, "upheld", "{}", p.because);
}

#[test]
fn an_uncovered_ancestor_alone_does_not_refute_an_exact_match() {
    // The mapping carries no row for s:Person, so nothing in the target can be
    // compared with it. That is a COVERAGE gap, and it is already reported:
    // s:Person is listed as unmapped. It is not evidence that s:Employee and
    // t:Employee differ, because over everything both sides CAN be compared on
    // (Agent) they agree. It used to force relatedMatch on every descendant,
    // which turned one missing row into a downgrade of the whole subtree; a
    // feasibility run on a food-waste taxonomy downgraded every row that way.
    let partial = vec![
        m("http://src.example/Employee", EXACT, "http://tgt.example/Employee"),
        m("http://src.example/Agent", EXACT, "http://tgt.example/Agent"),
    ];
    let (rows, gaps) = certify(&store(SOURCE), &store(TARGET), &partial).expect("certifies");
    let emp = rows.iter().find(|r| r.mapping.subject.ends_with("Employee")).expect("row");
    assert_eq!(emp.supported, "exactMatch", "{}", emp.because);
    assert!(!emp.downgraded, "{}", emp.because);
    assert!(
        emp.untranslatable.iter().any(|u| u.ends_with("/Person")),
        "the ancestor with no image must still be NAMED, or a reader cannot act on it: {:?}",
        emp.untranslatable
    );
    assert_eq!(emp.certification, "upheld_with_coverage_gap", "{}", emp.because);
    assert!(emp.because.contains("no image"), "and say so in words: {}", emp.because);
    assert!(
        gaps.iter().any(|g| g.term.ends_with("/Person")),
        "the gap is reported where it belongs, as an unmapped term"
    );
}

#[test]
fn a_real_disagreement_still_downgrades_when_an_ancestor_is_also_uncovered() {
    // s:Employee now also sits under s:Payroll, which nothing maps. That gap
    // must not mask the disagreement that IS visible: s:Employee is under
    // Person, t:Employee is not, and Person is mapped on both sides.
    let source = "@prefix s: <http://src.example/> .\n\
        s:Employee a owl:Class ; rdfs:subClassOf s:Person , s:Payroll .\n\
        s:Payroll  a owl:Class .\n\
        s:Person   a owl:Class ; rdfs:subClassOf s:Agent .\n\
        s:Agent    a owl:Class .\n";
    let (rows, _) = certify(&store(source), &store(TARGET), &full_mapping()).expect("certifies");
    let emp = rows.iter().find(|r| r.mapping.subject.ends_with("Employee")).expect("row");
    assert_eq!(emp.supported, "broadMatch", "{}", emp.because);
    assert!(emp.downgraded);
    assert_eq!(emp.certification, "downgraded");
    assert!(emp.untranslatable.iter().any(|u| u.ends_with("/Payroll")), "{:?}", emp.untranslatable);
}

#[test]
fn a_target_ancestor_with_no_source_counterpart_does_not_refute_either() {
    // A FLAT source list mapped onto a two-level taxonomy. The target puts
    // Bakery under a section the source has no term for. The old comparison
    // read that as "the object is the more specific concept" and downgraded a
    // correct exactMatch to narrowMatch. It is the same coverage gap seen from
    // the other side, and is named under `object_uncovered`.
    let flat = "@prefix s: <http://src.example/> .\n\
        s:Bakery a owl:Class .\n";
    let tax = "@prefix t: <http://tgt.example/> .\n\
        t:BAK a owl:Class ; rdfs:subClassOf t:ProductCategory .\n\
        t:ProductCategory a owl:Class .\n";
    let one = vec![m("http://src.example/Bakery", EXACT, "http://tgt.example/BAK")];
    let (rows, _) = certify(&store(flat), &store(tax), &one).expect("certifies");
    let r = &rows[0];
    assert_eq!(r.supported, "exactMatch", "{}", r.because);
    assert!(!r.downgraded);
    assert!(
        r.object_uncovered.iter().any(|u| u.ends_with("ProductCategory")),
        "{:?}",
        r.object_uncovered
    );
    assert_eq!(
        r.certification, "not_refuted_nothing_compared",
        "nothing on either side could be compared, so the row is not refuted and is not \
         confirmed either, and the word must say which: {}",
        r.because
    );
}

#[test]
fn a_weaker_claim_is_never_upgraded_in_the_output() {
    // Two siblings under one mapped parent have identical translated ancestor
    // sets, so the hierarchy cannot tell them apart and "supports" exactMatch
    // for ANY pairing of them, right or wrong. Writing that predicate into the
    // file would turn a cautious closeMatch into an exactMatch on evidence that
    // says nothing about the two terms themselves.
    let src = "@prefix s: <http://src.example/> .\n\
        s:Milk a owl:Class ; rdfs:subClassOf s:Food .\n\
        s:Food a owl:Class .\n";
    let tgt = "@prefix t: <http://tgt.example/> .\n\
        t:Beverages a owl:Class ; rdfs:subClassOf t:Food .\n\
        t:Dairy     a owl:Class ; rdfs:subClassOf t:Food .\n\
        t:Food a owl:Class .\n";
    let close = "http://www.w3.org/2004/02/skos/core#closeMatch";
    let maps = vec![
        m("http://src.example/Milk", close, "http://tgt.example/Beverages"),
        m("http://src.example/Food", EXACT, "http://tgt.example/Food"),
    ];
    let (rows, gaps) = certify(&store(src), &store(tgt), &maps).expect("certifies");
    let milk = rows.iter().find(|r| r.mapping.subject.ends_with("Milk")).expect("row");
    let tsv = to_sssom(&rows, &gaps);
    let line = tsv.lines().find(|l| l.contains("src.example/Milk")).expect("row");
    assert!(
        line.contains("\tskos:closeMatch\t"),
        "the predicate column may weaken a claim, never strengthen it: {line} ({})",
        milk.because
    );
    let js = report(&rows, &gaps).to_string();
    assert!(js.contains("siblings"), "the report must say the check cannot tell siblings apart: {js}");
}

// ───────────────────────────────────────────────────────────────────────────
// SKOS input: the gate must be able to fail
// ───────────────────────────────────────────────────────────────────────────

/// A source list with three axes, its hierarchy stated the SKOS way.
const SKOS_SOURCE: &str = "@prefix skos: <http://www.w3.org/2004/02/skos/core#> .\n\
    @prefix s: <http://src.example/> .\n\
    s:Destinations a skos:Concept .\n\
    s:WasteTypes   a skos:Concept .\n\
    s:Products     a skos:Concept .\n\
    s:Landfill   a skos:Concept ; skos:broader s:Destinations .\n\
    s:PlateWaste a skos:Concept ; skos:broader s:WasteTypes .\n\
    s:Bakery     a skos:Concept ; skos:broaderTransitive s:Products .\n";

/// The taxonomy it is mapped onto: the same three axes under other names.
const SKOS_TARGET: &str = "@prefix skos: <http://www.w3.org/2004/02/skos/core#> .\n\
    @prefix t: <http://tgt.example/> .\n\
    t:Scheme a skos:ConceptScheme .\n\
    t:WD  a skos:Concept ; skos:topConceptOf t:Scheme .\n\
    t:WT  a skos:Concept ; skos:topConceptOf t:Scheme .\n\
    t:PC  a skos:Concept ; skos:topConceptOf t:Scheme .\n\
    t:LF  a skos:Concept ; skos:broader t:WD .\n\
    t:PW  a skos:Concept ; skos:broader t:WT .\n\
    t:BAK a skos:Concept ; skos:broader t:PC .\n";

fn axes() -> Vec<Mapping> {
    vec![
        m("http://src.example/Destinations", EXACT, "http://tgt.example/WD"),
        m("http://src.example/WasteTypes", EXACT, "http://tgt.example/WT"),
        m("http://src.example/Products", EXACT, "http://tgt.example/PC"),
    ]
}

#[test]
fn a_deliberately_wrong_skos_crosswalk_is_rejected() {
    // Landfill is a destination and Plate waste a waste type; the rows cross
    // axes. Read through rdfs:subClassOf alone, both sides of every row had
    // no ancestors at all and this crosswalk certified clean, 0 downgrades:
    // the check could not fail on SKOS input.
    let mut maps = axes();
    maps.push(m("http://src.example/Landfill", EXACT, "http://tgt.example/PW"));
    maps.push(m("http://src.example/PlateWaste", EXACT, "http://tgt.example/BAK"));
    let (rows, _) = certify(&store(SKOS_SOURCE), &store(SKOS_TARGET), &maps).expect("certifies");
    for tail in ["/Landfill", "/PlateWaste"] {
        let r = rows.iter().find(|r| r.mapping.subject.ends_with(tail)).expect("row");
        assert!(r.downgraded, "{tail} crosses axes and must be refuted: {}", r.because);
        assert_eq!(r.supported, "relatedMatch", "{}", r.because);
        assert_eq!(r.certification, "downgraded");
    }
}

#[test]
fn a_correct_skos_crosswalk_is_upheld() {
    let mut maps = axes();
    maps.push(m("http://src.example/Landfill", EXACT, "http://tgt.example/LF"));
    maps.push(m("http://src.example/PlateWaste", EXACT, "http://tgt.example/PW"));
    maps.push(m("http://src.example/Bakery", EXACT, "http://tgt.example/BAK"));
    let (rows, _) = certify(&store(SKOS_SOURCE), &store(SKOS_TARGET), &maps).expect("certifies");
    for tail in ["/Landfill", "/PlateWaste", "/Bakery"] {
        let r = rows.iter().find(|r| r.mapping.subject.ends_with(tail)).expect("row");
        assert!(!r.downgraded, "{tail}: {}", r.because);
        assert_eq!(r.certification, "upheld", "{tail}: {}", r.because);
    }
}

#[test]
fn a_row_with_nothing_to_compare_is_not_certified() {
    // The same wrong rows WITHOUT the axis rows. Nothing either side says can
    // be compared through the mapping, so the honest answer is that nothing
    // was checked. The row must not read as upheld.
    let maps = vec![
        m("http://src.example/Landfill", EXACT, "http://tgt.example/PW"),
        m("http://src.example/PlateWaste", EXACT, "http://tgt.example/BAK"),
    ];
    let (rows, gaps) = certify(&store(SKOS_SOURCE), &store(SKOS_TARGET), &maps).expect("certifies");
    for r in &rows {
        assert_eq!(r.certification, "not_refuted_nothing_compared", "{}", r.because);
        assert!(r.because.contains("NOT certified"), "{}", r.because);
    }
    let tsv = to_sssom(&rows, &gaps);
    assert!(!tsv.contains("\tupheld\t"), "nothing in the file may say upheld: {tsv}");

    // And a term with no hierarchy at all, on both sides, is the same case.
    let flat = vec![m("http://src.example/Destinations", EXACT, "http://tgt.example/WD")];
    let (rows, _) = certify(&store(SKOS_SOURCE), &store(SKOS_TARGET), &flat).expect("certifies");
    assert_eq!(rows[0].certification, "not_refuted_nothing_compared", "{}", rows[0].because);
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
///
/// There is no individual here. This fixture used to carry `t:m1 a
/// t:Machine , t:Employee`, which made the target inconsistent ON ITS OWN, so
/// the probe reported a clash whatever the mapping said: the test could not
/// fail. A TBox alone is what a crosswalk usually connects, and the probe's
/// witness individuals are what let it see a clash there.
const HOSTILE_TARGET: &str = "@prefix t: <http://tgt.example/> .\n\
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
    @prefix owl:  <http://www.w3.org/2002/07/owl#> .\n\
    t:Employee a owl:Class ; rdfs:subClassOf t:Agent ; owl:disjointWith t:Machine .\n\
    t:Machine  a owl:Class .\n\
    t:Person   a owl:Class ; rdfs:subClassOf t:Agent .\n\
    t:Agent    a owl:Class .\n";

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
    assert_eq!(
        js["target_alone"]["clash_count"], 0,
        "the clash must come from the mapping, not from a target that clashes anyway: {js}"
    );
    assert_eq!(js["exit_code"], 1, "{js}");
}

#[test]
fn a_target_that_clashes_on_its_own_is_not_blamed_on_the_mapping() {
    use open_ontologies::crosswalk::contradiction_probe;
    let broken = format!("{HOSTILE_TARGET}@prefix t: <http://tgt.example/> .\nt:m1 a t:Machine , t:Employee .\n");
    let js = contradiction_probe(&store(SOURCE), &broken, &full_mapping()).expect("probes");
    assert_eq!(js["contradicted"], false, "an honest mapping into a broken target: {js}");
    assert!(js["target_alone"]["clash_count"].as_u64().unwrap_or(0) > 0, "{js}");
    assert_eq!(
        js["verdict"],
        "no_contradiction_added_by_the_mapping_but_the_target_clashes_on_its_own",
        "{js}"
    );
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
