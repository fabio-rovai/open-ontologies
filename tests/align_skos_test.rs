//! `align` against SKOS vocabularies, and against a mix of OWL and SKOS.
//!
//! The candidate generator used to read `?c a owl:Class` and nothing else, so a
//! pure SKOS taxonomy, which declares `skos:Concept` and never `owl:Class`,
//! produced no candidates at all. Nothing said why: the report was a clean
//! `total_candidates: 0`, which reads as "these two vocabularies share no
//! terms" when the truth was "one side was never read". A feasibility run
//! mapping a food-waste source list onto a SKOS taxonomy hit exactly that.
//!
//! A SKOS concept carries the same three things the scorer needs from a class,
//! under different predicates: its names (`skos:prefLabel`, `skos:altLabel`,
//! `skos:hiddenLabel`), its parents (`skos:broader`, `skos:broaderTransitive`)
//! and its definition (`skos:definition`). These tests pin each of them on
//! small synthetic fixtures. OWL behaviour is not touched: an entity typed
//! `owl:Class` is read exactly as before, which the OWL-only tests in
//! `src/align.rs` and the Anatomy hash in `docs/determinism.md` keep honest.

use open_ontologies::align::AlignmentEngine;
use open_ontologies::graph::GraphStore;
use open_ontologies::state::StateDb;
use serde_json::Value;
use std::sync::Arc;

fn engine() -> AlignmentEngine {
    let dir = tempfile::tempdir().unwrap();
    let db = StateDb::open(&dir.path().join("state.db")).unwrap();
    // The directory must outlive the engine's connection.
    std::mem::forget(dir);
    AlignmentEngine::new(db, Arc::new(GraphStore::new()))
}

fn run(source: &str, target: &str, high: f64, low: f64) -> Value {
    let out = engine()
        .align_with_thresholds(source, Some(target), high, low, true)
        .expect("align runs");
    serde_json::from_str(&out).expect("align returns JSON")
}

fn pair<'a>(result: &'a Value, source_tail: &str, target_tail: &str) -> Option<&'a Value> {
    result["candidates"].as_array().unwrap().iter().find(|c| {
        c["source_iri"].as_str().unwrap().ends_with(source_tail)
            && c["target_iri"].as_str().unwrap().ends_with(target_tail)
    })
}

/// A source list as a reporting template would publish it: flat, coded
/// concepts, one name each, some names reaching the target only through an
/// alternative or hidden label.
const SKOS_SOURCE: &str = r#"
    @prefix skos: <http://www.w3.org/2004/02/skos/core#> .
    @prefix src:  <http://source.example/list/> .
    src:c01 a skos:Concept ; skos:prefLabel "Bakery"@en .
    src:c02 a skos:Concept ; skos:prefLabel "Milk products"@en ; skos:altLabel "Dairy products"@en .
    src:c03 a skos:Concept ; skos:prefLabel "Frozen desserts"@en ; skos:hiddenLabel "Edible ices"@en .
"#;

/// A two-level taxonomy: every leaf has a `skos:broader` parent and a
/// definition. No `owl:Class` anywhere, which is what a SKOS scheme looks like.
const SKOS_TARGET: &str = r#"
    @prefix skos: <http://www.w3.org/2004/02/skos/core#> .
    @prefix tx:   <http://target.example/tax#> .
    tx:Scheme a skos:ConceptScheme .
    tx:PC  a skos:Concept ; skos:prefLabel "Product category"@en ; skos:topConceptOf tx:Scheme .
    tx:BAK a skos:Concept ; skos:prefLabel "Bakery"@en ; skos:broader tx:PC ;
           skos:definition "Bread and ordinary bakery wares."@en .
    tx:DRY a skos:Concept ; skos:prefLabel "Dairy products"@en ; skos:broader tx:PC .
    tx:ICE a skos:Concept ; skos:prefLabel "Edible ices"@en ; skos:broaderTransitive tx:PC .
"#;

#[test]
fn a_pure_skos_target_yields_candidates() {
    let r = run(SKOS_SOURCE, SKOS_TARGET, 0.85, 0.4);
    assert!(
        r["total_candidates"].as_u64().unwrap_or(0) >= 3,
        "three source concepts share a name with three target concepts; a SKOS target \
         read as empty would report none: {r}"
    );
    assert!(
        pair(&r, "c01", "#BAK").is_some(),
        "prefLabel to prefLabel: {r}"
    );
}

#[test]
fn alternative_and_hidden_labels_are_names() {
    let r = run(SKOS_SOURCE, SKOS_TARGET, 0.85, 0.4);
    let dairy = pair(&r, "c02", "#DRY").unwrap_or_else(|| panic!("altLabel route missing: {r}"));
    assert!(
        dairy["signals"]["label_similarity"].as_f64().unwrap() > 0.99,
        "{dairy}"
    );
    let ices = pair(&r, "c03", "#ICE").unwrap_or_else(|| panic!("hiddenLabel route missing: {r}"));
    assert!(
        ices["signals"]["label_similarity"].as_f64().unwrap() > 0.99,
        "{ices}"
    );
}

#[test]
fn broader_is_read_as_the_parent_and_definitions_reach_the_reviewer() {
    // A high threshold above 1 puts every candidate in the borderline bucket,
    // which is the bucket that carries the reviewer's context block.
    let r = run(SKOS_SOURCE, SKOS_TARGET, 1.01, 0.0);
    let bak = pair(&r, "c01", "#BAK").unwrap_or_else(|| panic!("no Bakery pair: {r}"));
    let parents: Vec<&str> = bak["context"]["target_parents"]
        .as_array()
        .unwrap_or_else(|| panic!("no target_parents: {bak}"))
        .iter()
        .filter_map(|p| p.as_str())
        .collect();
    assert_eq!(
        parents,
        vec!["http://target.example/tax#PC"],
        "skos:broader is the parent: {bak}"
    );
    let defs = bak["context"]["target_definitions"].to_string();
    assert!(
        defs.contains("Bread and ordinary bakery wares."),
        "skos:definition reaches the reviewer: {bak}"
    );

    let ice = pair(&r, "c03", "#ICE").unwrap_or_else(|| panic!("no ices pair: {r}"));
    assert!(
        ice["context"]["target_parents"]
            .to_string()
            .contains("tax#PC"),
        "skos:broaderTransitive is a parent too: {ice}"
    );
}

/// An OWL ontology on one side and a SKOS scheme on the other: the case of a
/// data model being mapped onto a reporting taxonomy.
const OWL_SOURCE: &str = r#"
    @prefix owl:  <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix m:    <http://model.example/> .
    m:Bakery        a owl:Class ; rdfs:label "Bakery" .
    m:DairyProducts a owl:Class ; rdfs:label "Dairy products" .
    m:Vehicle       a owl:Class ; rdfs:label "Vehicle" .
"#;

#[test]
fn owl_classes_align_onto_skos_concepts_and_back() {
    let forward = run(OWL_SOURCE, SKOS_TARGET, 0.85, 0.4);
    assert!(
        pair(&forward, "/Bakery", "#BAK").is_some(),
        "OWL to SKOS: {forward}"
    );
    assert!(
        pair(&forward, "/DairyProducts", "#DRY").is_some(),
        "OWL to SKOS: {forward}"
    );
    assert!(
        forward["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| !c["source_iri"].as_str().unwrap().ends_with("Vehicle")),
        "a class with no counterpart must not be matched: {forward}"
    );

    let backward = run(SKOS_TARGET, OWL_SOURCE, 0.85, 0.4);
    assert!(
        pair(&backward, "#BAK", "/Bakery").is_some(),
        "SKOS to OWL: {backward}"
    );
}

#[test]
fn an_entity_that_is_both_a_class_and_a_concept_is_read_once() {
    // Punning is legal and common in SKOS-OWL hybrids. The entity must appear
    // once, read as a class, rather than twice with a duplicate candidate.
    let punned = r#"
        @prefix owl:  <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix skos: <http://www.w3.org/2004/02/skos/core#> .
        @prefix p:    <http://punned.example/> .
        p:Bakery a owl:Class , skos:Concept ; rdfs:label "Bakery" ; skos:prefLabel "Bakery"@en .
    "#;
    let r = run(punned, SKOS_TARGET, 0.85, 0.4);
    let hits = r["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["source_iri"].as_str().unwrap().ends_with("/Bakery"))
        .count();
    assert_eq!(hits, 1, "{r}");
}
