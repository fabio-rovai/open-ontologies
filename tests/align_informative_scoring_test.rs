//! Confidence is a weighted mean over the signals that carry information for
//! the pair, not over every signal the engine knows.
//!
//! The old fusion had two branches. When every structural signal was zero it
//! returned `label_similarity * 0.85`, so a perfect name match between two flat
//! lists could never score above 0.85 however exact it was. When any
//! structural signal was non-zero it returned the plain weighted sum, in which
//! a signal with no data on either side still contributed its weight times
//! zero. The second branch made agreement COST confidence: two classes with
//! the same name and the same parent scored `0.25 + 0.15 = 0.40`, less than
//! half what the same pair scored with the parent deleted.
//!
//! Neither branch is a choice anyone would defend. A signal with no data on
//! one side is ignorance, not disagreement: a SKOS concept has no
//! `rdfs:domain` properties because SKOS does not model them, not because the
//! concept differs from an OWL class that has some. So a signal now enters the
//! mean only when BOTH sides carry data for it, and the mean is taken over the
//! weights of those signals alone. Each candidate names the signals that
//! counted (`informative_signals`) and the share of the total weight they hold
//! (`evidence_coverage`), so a 1.0 that rests on a name alone reads as exactly
//! that.

use open_ontologies::align::AlignmentEngine;
use open_ontologies::graph::GraphStore;
use open_ontologies::state::StateDb;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

fn engine_with(db: StateDb) -> AlignmentEngine {
    AlignmentEngine::new(db, Arc::new(GraphStore::new()))
}

fn fresh_db() -> StateDb {
    let dir = tempfile::tempdir().unwrap();
    let db = StateDb::open(&dir.path().join("state.db")).unwrap();
    std::mem::forget(dir);
    db
}

fn run(engine: &AlignmentEngine, source: &str, target: &str) -> Value {
    run_low(engine, source, target, 0.4)
}

fn run_low(engine: &AlignmentEngine, source: &str, target: &str, low: f64) -> Value {
    let out = engine
        .align_with_thresholds(source, Some(target), 0.85, low, true)
        .expect("align runs");
    serde_json::from_str(&out).expect("JSON")
}

fn only(result: &Value, source_tail: &str) -> Value {
    result["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["source_iri"].as_str().unwrap().ends_with(source_tail))
        .unwrap_or_else(|| panic!("no candidate for {source_tail}: {result}"))
        .clone()
}

const FLAT_SOURCE: &str = r#"
    @prefix skos: <http://www.w3.org/2004/02/skos/core#> .
    @prefix s: <http://flat-a.example/> .
    s:exact a skos:Concept ; skos:prefLabel "Landfill"@en .
    s:weak  a skos:Concept ; skos:prefLabel "Sewer treatment"@en .
"#;

const FLAT_TARGET: &str = r#"
    @prefix skos: <http://www.w3.org/2004/02/skos/core#> .
    @prefix t: <http://flat-b.example/> .
    t:LF a skos:Concept ; skos:prefLabel "Landfill"@en .
    t:SW a skos:Concept ; skos:prefLabel "Sewage"@en .
"#;

#[test]
fn an_exact_name_between_two_flat_lists_reaches_high_confidence() {
    let r = run(&engine_with(fresh_db()), FLAT_SOURCE, FLAT_TARGET);
    let exact = only(&r, "/exact");
    let conf = exact["confidence"].as_f64().unwrap();
    assert!(
        conf > 0.99,
        "two flat lists have nothing to compare but names, so an exact name is all the \
         evidence there is and it is perfect. The old fusion capped this at 0.85: {exact}"
    );
    assert_eq!(exact["applied"], false, "dry run");
    assert_eq!(exact["requires_review"], false, "{exact}");
}

#[test]
fn a_weak_name_between_two_flat_lists_cannot() {
    let r = run(&engine_with(fresh_db()), FLAT_SOURCE, FLAT_TARGET);
    let weak = only(&r, "/weak");
    let conf = weak["confidence"].as_f64().unwrap();
    let label = weak["signals"]["label_similarity"].as_f64().unwrap();
    assert!(label < 0.85, "the fixture is built for a weak name: {weak}");
    assert!(
        (conf - label).abs() < 1e-9,
        "with a name as the only evidence, confidence IS the name similarity: {weak}"
    );
    assert_eq!(
        weak["requires_review"], true,
        "a weak name alone must land in review, never auto-applied: {weak}"
    );
}

#[test]
fn the_output_names_the_signals_that_counted() {
    let r = run(&engine_with(fresh_db()), FLAT_SOURCE, FLAT_TARGET);
    assert_eq!(r["scoring"], "informative_renormalised", "{r}");
    let exact = only(&r, "/exact");
    let informative: Vec<&str> = exact["informative_signals"]
        .as_array()
        .unwrap_or_else(|| panic!("informative_signals missing: {exact}"))
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(informative, vec!["label_similarity"], "{exact}");
    let coverage = exact["evidence_coverage"].as_f64().unwrap();
    assert!(
        coverage > 0.0 && coverage < 0.5,
        "a confidence of 1.0 resting on the name alone must say how little of the \
         evidence it rests on: {exact}"
    );
}

const OWL_PARENT_SOURCE: &str = r#"
    @prefix owl:  <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix a: <http://parents-a.example/> .
    a:Animal a owl:Class ; rdfs:label "Animal" .
    a:Dog    a owl:Class ; rdfs:label "Dog" ; rdfs:subClassOf a:Animal .
"#;

const OWL_PARENT_TARGET: &str = r#"
    @prefix owl:  <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix b: <http://parents-b.example/> .
    b:Animal a owl:Class ; rdfs:label "Animal" .
    b:Dog    a owl:Class ; rdfs:label "Dog" ; rdfs:subClassOf b:Animal .
"#;

#[test]
fn agreeing_structure_never_lowers_confidence() {
    // Same name, same parent. The old weighted sum scored this 0.40, because
    // four signals with no data on either side still carried their weight.
    let r = run(
        &engine_with(fresh_db()),
        OWL_PARENT_SOURCE,
        OWL_PARENT_TARGET,
    );
    let dog = only(&r, "/Dog");
    assert!(
        dog["signals"]["parent_overlap"].as_f64().unwrap() > 0.99,
        "{dog}"
    );
    assert!(
        dog["confidence"].as_f64().unwrap() > 0.99,
        "a matching parent is more evidence for the match, not less: {dog}"
    );
}

/// The same flat names as `FLAT_*`, but each concept now sits under a parent,
/// and the two parents differ. Both parents exist on both sides, so each side
/// CAN name the other's parent: the two simply did not put the term in the
/// same place.
const DISAGREEING_SOURCE: &str = r#"
    @prefix skos: <http://www.w3.org/2004/02/skos/core#> .
    @prefix s: <http://dis-a.example/> .
    s:Destinations a skos:Concept ; skos:prefLabel "Waste destinations"@en .
    s:Sites a skos:Concept ; skos:prefLabel "Geological formations"@en .
    s:exact a skos:Concept ; skos:prefLabel "Landfill"@en ; skos:broader s:Destinations .
"#;

const DISAGREEING_TARGET: &str = r#"
    @prefix skos: <http://www.w3.org/2004/02/skos/core#> .
    @prefix t: <http://dis-b.example/> .
    t:Destinations a skos:Concept ; skos:prefLabel "Waste destinations"@en .
    t:Sites a skos:Concept ; skos:prefLabel "Geological formations"@en .
    t:LF a skos:Concept ; skos:prefLabel "Landfill"@en ; skos:broader t:Sites .
"#;

/// The same again, with a second parent on each side, one of them shared.
const PARTLY_AGREEING_SOURCE: &str = r#"
    @prefix skos: <http://www.w3.org/2004/02/skos/core#> .
    @prefix s: <http://part-a.example/> .
    s:Destinations a skos:Concept ; skos:prefLabel "Waste destinations"@en .
    s:Sites a skos:Concept ; skos:prefLabel "Geological formations"@en .
    s:Disposal a skos:Concept ; skos:prefLabel "Disposal"@en .
    s:exact a skos:Concept ; skos:prefLabel "Landfill"@en ; skos:broader s:Destinations , s:Disposal .
"#;

const PARTLY_AGREEING_TARGET: &str = r#"
    @prefix skos: <http://www.w3.org/2004/02/skos/core#> .
    @prefix t: <http://part-b.example/> .
    t:Destinations a skos:Concept ; skos:prefLabel "Waste destinations"@en .
    t:Sites a skos:Concept ; skos:prefLabel "Geological formations"@en .
    t:Disposal a skos:Concept ; skos:prefLabel "Disposal"@en .
    t:LF a skos:Concept ; skos:prefLabel "Landfill"@en ; skos:broader t:Sites , t:Disposal .
"#;

#[test]
fn a_parent_not_shared_is_not_a_denial_and_partial_agreement_never_scores_below_none() {
    // Open world: the target not putting Landfill under destinations is not
    // the target saying it is NOT a destination. Ontologies differ in
    // granularity and use multiple inheritance, and on OAEI Anatomy counting
    // such a zero as disagreement cost 0.07 F1. So it neither helps nor hurts.
    let flat = only(
        &run(&engine_with(fresh_db()), FLAT_SOURCE, FLAT_TARGET),
        "/exact",
    );
    let dis = only(
        &run(
            &engine_with(fresh_db()),
            DISAGREEING_SOURCE,
            DISAGREEING_TARGET,
        ),
        "/exact",
    );
    assert_eq!(
        dis["signals"]["parent_overlap"], 0.0,
        "the fixture shares no parent: {dis}"
    );
    assert_eq!(dis["confidence"], flat["confidence"], "{dis}");
    assert!(
        !dis["informative_signals"]
            .to_string()
            .contains("parent_overlap"),
        "a parent nobody shares carries no evidence and must not be listed as counting: {dis}"
    );

    // One shared parent out of three is MORE evidence than none, so it must
    // not score lower. A weighted mean that took the Jaccard value (1/3)
    // would put this pair below the one above.
    let part = only(
        &run(
            &engine_with(fresh_db()),
            PARTLY_AGREEING_SOURCE,
            PARTLY_AGREEING_TARGET,
        ),
        "/exact",
    );
    let jaccard = part["signals"]["parent_overlap"].as_f64().unwrap();
    assert!(
        jaccard > 0.0 && jaccard < 1.0,
        "the fixture shares one parent of three: {part}"
    );
    assert!(
        part["confidence"].as_f64().unwrap() >= dis["confidence"].as_f64().unwrap(),
        "agreement can only raise confidence: partly={part} none={dis}"
    );
    assert!(
        part["informative_signals"]
            .to_string()
            .contains("parent_overlap"),
        "{part}"
    );
}

#[test]
fn a_parent_on_one_side_only_neither_helps_nor_hurts() {
    let flat = only(
        &run(&engine_with(fresh_db()), FLAT_SOURCE, FLAT_TARGET),
        "/exact",
    );
    let one_sided = only(
        &run(&engine_with(fresh_db()), FLAT_SOURCE, DISAGREEING_TARGET),
        "/exact",
    );
    assert_eq!(one_sided["confidence"], flat["confidence"], "{one_sided}");
}

/// A near name under a shared parent: the case the learned weights decide.
const NEAR_SOURCE: &str = r#"
    @prefix skos: <http://www.w3.org/2004/02/skos/core#> .
    @prefix s: <http://near-a.example/> .
    s:Destinations a skos:Concept ; skos:prefLabel "Waste destinations"@en .
    s:near a skos:Concept ; skos:prefLabel "Sewer treatment"@en ; skos:broader s:Destinations .
"#;

const NEAR_TARGET: &str = r#"
    @prefix skos: <http://www.w3.org/2004/02/skos/core#> .
    @prefix t: <http://near-b.example/> .
    t:Destinations a skos:Concept ; skos:prefLabel "Waste destinations"@en .
    t:SW a skos:Concept ; skos:prefLabel "Sewage"@en ; skos:broader t:Destinations .
"#;

#[test]
fn learned_weights_still_move_the_score() {
    // Self-calibration must survive the change. A shared parent lifts a weak
    // name; train the weights to distrust parents (siblings share them too)
    // and the lift must shrink.
    let db = fresh_db();
    let before = only(
        &run_low(&engine_with(db.clone()), NEAR_SOURCE, NEAR_TARGET, 0.0),
        "/near",
    );
    assert!(
        before["informative_signals"]
            .to_string()
            .contains("parent_overlap"),
        "{before}"
    );

    let eng = engine_with(db.clone());
    for i in 0..6 {
        let accepted: HashMap<String, f64> = [
            ("label_similarity".to_string(), 1.0),
            ("parent_overlap".to_string(), 0.0),
        ]
        .into();
        eng.record_feedback(
            &format!("http://a/{i}"),
            "http://b/x",
            "skos:exactMatch",
            true,
            Some(&accepted),
        )
        .unwrap();
        let rejected: HashMap<String, f64> = [
            ("label_similarity".to_string(), 0.8),
            ("parent_overlap".to_string(), 1.0),
        ]
        .into();
        eng.record_feedback(
            &format!("http://c/{i}"),
            "http://d/x",
            "skos:exactMatch",
            false,
            Some(&rejected),
        )
        .unwrap();
    }
    let after = only(
        &run_low(&engine_with(db), NEAR_SOURCE, NEAR_TARGET, 0.0),
        "/near",
    );
    let (b, a) = (
        before["confidence"].as_f64().unwrap(),
        after["confidence"].as_f64().unwrap(),
    );
    let label = after["signals"]["label_similarity"].as_f64().unwrap();
    assert!((0.0..=1.0).contains(&a), "{after}");
    assert!(
        a < b,
        "weights that distrust parents must shrink the lift a shared parent gives: before={b} after={a}"
    );
    assert!(
        a >= label,
        "and the lift cannot turn into a penalty: {after}"
    );
}

#[test]
fn the_legacy_fusion_is_kept_behind_a_flag_and_reproduces_the_old_numbers() {
    // Every published alignment figure (OAEI Anatomy F1 0.829, Conference
    // 0.438) was measured with the old fusion, so it stays reachable. These
    // are the two numbers the old code produced on these fixtures: the 0.85
    // cap on a perfect flat name, and the weighted sum of name and parent for
    // the same name and same parent. That sum is 0.25 + 0.15 = 0.40 with the
    // six default weights, and 0.20 + 0.12 = 0.32 when the embeddings feature
    // adds a seventh signal and rescales the rest (no vectors are loaded here,
    // so the embedding term contributes 0).
    use open_ontologies::align::{AlignOptions, Scoring};
    let legacy = AlignOptions {
        scoring: Scoring::Legacy,
        dry_run: true,
        ..AlignOptions::default()
    };
    let eng = engine_with(fresh_db());
    let flat: Value = serde_json::from_str(
        &eng.align_with_options(FLAT_SOURCE, Some(FLAT_TARGET), &legacy)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(flat["scoring"], "legacy");
    assert_eq!(only(&flat, "/exact")["confidence"], 0.85, "{flat}");
    let parents: Value = serde_json::from_str(
        &eng.align_with_options(
            OWL_PARENT_SOURCE,
            Some(OWL_PARENT_TARGET),
            &AlignOptions {
                low_threshold: 0.0,
                ..legacy.clone()
            },
        )
        .unwrap(),
    )
    .unwrap();
    #[cfg(not(feature = "embeddings"))]
    let expected = 0.25 + 0.15;
    #[cfg(feature = "embeddings")]
    let expected = 0.20 + 0.12;
    let got = only(&parents, "/Dog")["confidence"].as_f64().unwrap();
    assert!((got - expected).abs() < 1e-9, "expected {expected}, got {got}: {parents}");
}

#[test]
fn the_command_line_takes_the_scoring_flag() {
    let dir = tempfile::tempdir().unwrap();
    let (s, t) = (dir.path().join("s.ttl"), dir.path().join("t.ttl"));
    std::fs::write(&s, FLAT_SOURCE).unwrap();
    std::fs::write(&t, FLAT_TARGET).unwrap();
    let run = |scoring: &str| -> (Option<i32>, Value) {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
            .arg("--data-dir")
            .arg(dir.path().join("data"))
            .args(["align"])
            .arg(&s)
            .arg(&t)
            .args(["--min-confidence", "0.5", "--dry-run", "--scoring", scoring])
            .output()
            .unwrap();
        let v = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
        (out.status.code(), v)
    };
    let (code, v) = run("legacy");
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(only(&v, "/exact")["confidence"], 0.85, "{v}");
    let (_, v) = run("informative");
    assert_eq!(only(&v, "/exact")["confidence"], 1.0, "{v}");
    let (code, v) = run("no-such-scoring");
    assert_eq!(
        code,
        Some(1),
        "an unknown scoring is an error, not a silent default: {v}"
    );
}
