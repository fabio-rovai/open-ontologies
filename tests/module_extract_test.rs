//! A module is not a smaller slice, it is a slice with a theorem.
//!
//! The tests that matter here are the ones that would pass for a heuristic too,
//! and the one that would not. `a_naive_signature_slice_drops_an_entailment_the
//! _module_keeps` is the second kind: it builds the slice a sensible retriever
//! produces, shows it is BIGGER than the module, and shows it has lost a
//! conclusion the module still reaches. Without that test, "we extract modules"
//! and "we extract neighbourhoods and call them modules" look identical from
//! the outside.

use open_ontologies::closure_diff::DiffOptions;
use open_ontologies::graph::GraphStore;
use open_ontologies::module_extract as me;
use open_ontologies::reason::{InferenceTarget, Reasoner};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

type Triple = (String, String, String);

fn loaded(ttl: &str) -> Arc<GraphStore> {
    let g = Arc::new(GraphStore::new());
    g.load_turtle(ttl, None)
        .unwrap_or_else(|e| panic!("fixture must parse: {e}\n{ttl}"));
    g
}

/// Everything the profile derives from `ttl`, as N-Triples-spelled triples.
fn closure_of(ttl: &str, profile: &str) -> HashSet<Triple> {
    let g = loaded(ttl);
    Reasoner::run_full(&g, profile, true, InferenceTarget::DefaultGraph, None)
        .expect("the reasoner must run");
    g.all_triples().expect("triples").into_iter().collect()
}

fn t(s: &str, p: &str, o: &str) -> Triple {
    (format!("<{s}>"), format!("<{p}>"), format!("<{o}>"))
}

const EX: &str = "http://ex.org/";
const SUBCLASS: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";

fn ex(local: &str) -> String {
    format!("{EX}{local}")
}

/// The namespace `benchmark/reference/pizza-reference.owl` actually declares.
/// Its `xml:base` is the raw.githubusercontent URL the file was fetched from,
/// not the co-ode.org IRI the ontology is usually cited by, and a signature
/// spelled the usual way selects nothing at all.
const PIZZA: &str =
    "https://raw.githubusercontent.com/owlcs/pizza-ontology/refs/heads/master/pizza.owl#";

fn pizza(local: &str) -> String {
    format!("{PIZZA}{local}")
}

/// A four-link chain with two dead branches hanging off its top, one instance
/// and one unrelated ABox edge. Every number below is hand-computed from it.
const ONTOLOGY: &str = r#"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ex: <http://ex.org/> .

ex:Cat    rdfs:subClassOf ex:Mammal .
ex:Mammal rdfs:subClassOf ex:Animal .
ex:Animal rdfs:subClassOf ex:LivingThing .
ex:Plant  rdfs:subClassOf ex:LivingThing .
ex:Fungus rdfs:subClassOf ex:LivingThing .
ex:tom    a               ex:Cat .
ex:tom    ex:eats         ex:fish .
"#;

/// `{Cat, LivingThing}`.
fn signature() -> Vec<String> {
    vec![ex("Cat"), ex("LivingThing")]
}

fn extract(locality: me::Locality) -> me::ModuleReport {
    me::extract_module(
        &loaded(ONTOLOGY),
        &me::ModuleOptions { signature: signature(), locality, ..Default::default() },
    )
    .expect("extraction must succeed")
}

/// The slice a sensible retriever produces: every triple that MENTIONS a term
/// of the signature. It is the honest baseline, not a straw man, and it is the
/// shape `onto_segment_retrieve` produces at one hop.
fn naive_signature_slice() -> String {
    let g = loaded(ONTOLOGY);
    let wanted: HashSet<String> = signature().into_iter().map(|s| format!("<{s}>")).collect();
    g.all_triples()
        .unwrap()
        .into_iter()
        .filter(|(s, _, o)| wanted.contains(s) || wanted.contains(o))
        .map(|(s, p, o)| format!("{s} {p} {o} .\n"))
        .collect()
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("oo-module-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

// ───────────────────────────────────────────────────────────────────────────
// The module is strictly smaller and still says everything
// ───────────────────────────────────────────────────────────────────────────

/// Hand-computed. Over `{Cat, LivingThing}`:
///
/// - the `⊥` pass keeps the three chain links (each has a non-`⊥` left-hand
///   side once the previous link has grown the signature), `tom a Cat` and
///   `tom eats fish`, and drops the `Plant` and `Fungus` branches, whose
///   left-hand sides are external and therefore `⊥`;
/// - the `⊤` pass over that set keeps `Animal ⊑ LivingThing` on the first
///   sweep, then `Mammal ⊑ Animal`, then `Cat ⊑ Mammal` as the signature grows,
///   keeps `tom a Cat` because `Cat` is in `Σ`, and drops `tom eats fish`
///   because `eats` is external and the universal property makes the assertion
///   a tautology;
/// - a second `⊥⊤` round changes nothing, so the fixpoint is 4 of 7.
#[test]
fn the_star_module_is_strictly_smaller_than_the_ontology() {
    let r = extract(me::Locality::Star);
    assert_eq!(r.ontology_axioms, 7, "the fixture is seven axioms");
    assert_eq!(r.module_axioms, 4, "hand-computed ⊥⊤* module:\n{}", r.module_ttl);
    assert!(r.module_axioms < r.ontology_axioms);
    assert_eq!(r.rounds, 2, "one round to shrink, one to prove it stopped");
    assert!(!r.module_ttl.contains("Plant"), "{}", r.module_ttl);
    assert!(!r.module_ttl.contains("Fungus"), "{}", r.module_ttl);
    assert!(!r.module_ttl.contains("eats"), "{}", r.module_ttl);
}

/// The point of the smaller set: the conclusion over the signature survives it.
#[test]
fn the_module_still_entails_the_goal_over_the_signature() {
    let r = extract(me::Locality::Star);
    let goal = t(&ex("Cat"), SUBCLASS, &ex("LivingThing"));
    assert!(
        closure_of(ONTOLOGY, "owl-rl").contains(&goal),
        "the fixture must entail the goal, or the test proves nothing"
    );
    assert!(
        closure_of(&r.module_ttl, "owl-rl").contains(&goal),
        "the module lost a conclusion over its own signature:\n{}",
        r.module_ttl
    );
}

// ───────────────────────────────────────────────────────────────────────────
// The negative: a heuristic slice loses what the module keeps
// ───────────────────────────────────────────────────────────────────────────

/// This is the test that separates a module from a neighbourhood.
///
/// The naive slice keeps every triple MENTIONING a signature term. That is five
/// triples against the module's four, so it is not losing because it is smaller
/// or because it was built carelessly. It loses because `Mammal ⊑ Animal`
/// mentions neither `Cat` nor `LivingThing`, and the chain runs through it. The
/// module keeps it for exactly the reason the locality test exists: after
/// `Cat ⊑ Mammal` is taken, `Mammal` is in the working signature and the next
/// link stops being local.
#[test]
fn a_naive_signature_slice_drops_an_entailment_the_module_keeps() {
    let naive = naive_signature_slice();
    let module = extract(me::Locality::Star);
    let goal = t(&ex("Cat"), SUBCLASS, &ex("LivingThing"));

    let naive_triples = naive.lines().count();
    assert_eq!(naive_triples, 5, "the naive slice is:\n{naive}");
    assert!(
        naive_triples > module.module_triples,
        "the naive slice must be BIGGER than the module, or the comparison is about size: \
         naive {naive_triples}, module {}",
        module.module_triples
    );

    assert!(
        !closure_of(&naive, "owl-rl").contains(&goal),
        "the naive slice was supposed to lose the goal; if it stopped losing it the fixture \
         changed and this test no longer demonstrates anything:\n{naive}"
    );
    assert!(
        closure_of(&module.module_ttl, "owl-rl").contains(&goal),
        "the module must keep it:\n{}",
        module.module_ttl
    );
}

// ───────────────────────────────────────────────────────────────────────────
// Exercising the guarantee rather than asserting it
// ───────────────────────────────────────────────────────────────────────────

#[test]
fn the_closure_diff_finds_nothing_lost_over_the_signature() {
    let store = loaded(ONTOLOGY);
    let r = extract(me::Locality::Star);
    let opts = DiffOptions { out: scratch("verify"), ..Default::default() };
    let v = me::verify_module(&store, &r, &opts, 200_000)
        .expect("verification must run");
    assert_eq!(
        v.lost_over_signature_total, 0,
        "a locality module may not lose a conclusion over its own signature closure:\n{:#?}",
        v.lost_over_signature
    );
    assert_eq!(v.not_examined, 0, "the whole difference was examined");
    assert_eq!(v.engine_soundness_violations, 0);
    assert!(
        v.lost_outside_signature_total > 0,
        "the module must lose something OUTSIDE the signature, or it is the whole ontology and \
         the clean result is vacuous"
    );
}

/// The same thing at a size nobody hand-computes, on a file this repository
/// ships. A clean result over seven triples is cheap; a clean result over the
/// pizza ontology is the one worth reporting.
#[test]
fn the_guarantee_holds_on_the_pizza_ontology() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("benchmark")
        .join("reference")
        .join("pizza-reference.owl");
    let store = Arc::new(GraphStore::new());
    store.load_file(path.to_str().unwrap()).expect("the shipped pizza ontology must load");

    let signature = vec![pizza("Veneziana"), pizza("Food")];
    let r = me::extract_module(
        &store,
        &me::ModuleOptions {
            signature: signature.clone(),
            locality: me::Locality::Star,
            ..Default::default()
        },
    )
    .expect("extraction must succeed");

    assert!(r.signature_not_in_ontology.is_empty(), "{:?}", r.signature_not_in_ontology);
    assert!(
        r.module_triples < r.ontology_triples,
        "the module must be a proper subset: {} of {}",
        r.module_triples,
        r.ontology_triples
    );
    // Whatever it could not classify is IN the module, and the report says how
    // much. Printed rather than asserted to a number, which would pin a
    // property of the file rather than of this code.
    eprintln!(
        "pizza: {} of {} axioms, {} of {} triples, {} unclassified, kinds {:?}",
        r.module_axioms,
        r.ontology_axioms,
        r.module_triples,
        r.ontology_triples,
        r.unclassified_total,
        r.axioms_by_kind
    );

    let opts = DiffOptions { out: scratch("pizza"), ..Default::default() };
    let v = me::verify_module(&store, &r, &opts, 200_000)
        .expect("verification must run");
    assert_eq!(
        v.lost_over_signature_total, 0,
        "{}\nfirst rows: {:#?}",
        v.headline,
        v.lost_over_signature.iter().take(5).collect::<Vec<_>>()
    );
    assert_eq!(v.not_examined, 0, "the whole difference was examined: {}", v.headline);
    // The annotation partition has to be doing real work here, or a future
    // change that widened it into a blanket filter would go unnoticed: it
    // would make every module verify clean.
    assert!(
        v.lost_over_signature_annotation_only > 0,
        "pizza carries multilingual rdfs:labels the logical module drops, and they must be \
         COUNTED rather than silently filtered: {}",
        v.headline
    );
    eprintln!("pizza verification: {}", v.headline);
    // The blind spot is real on this file, because the module carries the
    // store's own blank node labels and the diff skolemises the source. It is
    // COUNTED, so the coverage of the clean result above can be read rather
    // than assumed.
    assert!(
        v.not_decided_blank_node_bearing > 0,
        "pizza is full of blank-node restrictions, so the blind spot must be non-zero and \
         reported: {}",
        v.headline
    );
    assert!(
        v.not_decided_blank_node_bearing <= v.lost_outside_signature_total,
        "the blind spot is a subset of the outside-signature count, not a fourth bucket"
    );

    // The conclusion the decision-0007 record names as the one a 99%-coverage
    // slice silently loses. It is over the module's signature closure, so the
    // module may not lose it.
    let goal = t(&pizza("Veneziana"), SUBCLASS, &pizza("Food"));
    assert!(
        closure_of(&r.module_ttl, "owl-rl").contains(&goal),
        "the module must still put Veneziana under Food"
    );
}

// ───────────────────────────────────────────────────────────────────────────
// Conservative where it cannot classify
// ───────────────────────────────────────────────────────────────────────────

/// An axiom type this file does not classify must be INCLUDED, never dropped,
/// and must be named in the report so the module's size can be read honestly.
///
/// `owl:hasSelf` is classified, so the unreadable case has to be a shape that
/// genuinely is not: a blank-node structure whose type is an OWL term with no
/// axiom reading here. Dropping it would be the silent failure, because the
/// module still parses and the count still looks small.
#[test]
fn an_unclassifiable_axiom_is_included_and_named() {
    let ttl = format!(
        "{ONTOLOGY}
         @prefix owl2: <http://www.w3.org/2002/07/owl#> .
         [] a owl2:SomeFutureAxiomKind ; owl2:members ( ex:Cat ex:Plant ) .
         ex:Cat owl2:someUnknownPredicate ex:Fungus ."
    );
    let r = me::extract_module(
        &loaded(&ttl),
        &me::ModuleOptions {
            signature: signature(),
            locality: me::Locality::Star,
            ..Default::default()
        },
    )
    .expect("extraction must succeed");

    assert_eq!(
        r.unclassified_total, 2,
        "both unreadable shapes must survive into the module, and be counted: {:#?}",
        r.unclassified_axioms
    );
    assert_eq!(r.included_conservatively.get("unclassified"), Some(&2));
    assert!(
        r.module_ttl.contains("SomeFutureAxiomKind"),
        "a blank-node structure of an unknown OWL type must be kept:\n{}",
        r.module_ttl
    );
    assert!(
        r.module_ttl.contains("someUnknownPredicate"),
        "an unrecognised owl: predicate must NOT be read as a droppable property assertion:\n{}",
        r.module_ttl
    );
    for u in &r.unclassified_axioms {
        assert!(!u.why.is_empty(), "every conservative inclusion says why: {u:?}");
    }
}

/// The dual of the test above, and the one that stops the conservative rule
/// from being vacuous: an unrecognised predicate in a USER namespace IS read as
/// a property assertion, and is `⊤`-local, so it does not survive. If every
/// unknown predicate were kept, the module would be the ontology and the
/// guarantee would be free.
#[test]
fn an_unrecognised_user_predicate_is_a_property_assertion_and_can_be_dropped() {
    let r = extract(me::Locality::Star);
    assert!(!r.module_ttl.contains("eats"), "{}", r.module_ttl);
    assert_eq!(r.axioms_by_kind.get("unclassified"), None);
}

// ───────────────────────────────────────────────────────────────────────────
// The report may not overstate what stands behind it
// ───────────────────────────────────────────────────────────────────────────

/// The house rule from decision 0002: a word with a machine-checked theorem
/// behind it and a word without one may never render the same. Locality has no
/// Lean theorem in this repository, so the serialised report must not name one.
#[test]
fn the_module_report_never_names_a_lean_theorem() {
    let r = extract(me::Locality::Star);
    let json = serde_json::to_string(&r).unwrap();
    for forbidden in ["OOCert.", "OwlLean.", "Fol.", "machine-checked theorem"] {
        assert!(
            !json.contains(forbidden),
            "the module report claimed {forbidden:?}, and no theorem in lean/ is about syntactic \
             locality"
        );
    }
    assert!(json.contains("CITED, not checked"), "the absence has to be stated, not inferred");
    assert!(json.contains("JAIR 31"), "the theorem that IS relied on must be named");
}

#[test]
fn every_axiom_kind_the_extractor_produces_is_declared_in_one_of_the_two_lists() {
    // A kind that is in neither list is a guarantee nobody wrote down. The
    // report's `axioms_by_kind` is the measurement; the two constants are the
    // claim.
    let ttl = format!(
        "{ONTOLOGY}
         @prefix owl2: <http://www.w3.org/2002/07/owl#> .
         ex:hasParent a owl2:TransitiveProperty ; rdfs:domain ex:Cat ; rdfs:range ex:Cat .
         ex:hasChild owl2:inverseOf ex:hasParent .
         ex:a owl2:sameAs ex:b .
         ex:Cat owl2:disjointWith ex:Plant .
         [] a owl2:SomeFutureAxiomKind ; owl2:members ( ex:Cat ) ."
    );
    let r = me::extract_module(
        &loaded(&ttl),
        &me::ModuleOptions {
            signature: vec![ex("Cat"), ex("LivingThing"), ex("hasParent"), ex("a")],
            locality: me::Locality::Bottom,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!r.axioms_by_kind.is_empty());
    for kind in r.axioms_by_kind.keys() {
        assert!(
            me::CLASSIFIED_KINDS.contains(&kind.as_str())
                || me::ALWAYS_INCLUDED_KINDS.contains(&kind.as_str()),
            "{kind:?} is produced by the extractor and declared in neither CLASSIFIED_KINDS nor \
             ALWAYS_INCLUDED_KINDS, so the tool description does not cover it"
        );
    }
}
