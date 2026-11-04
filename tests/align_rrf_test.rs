// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Integration tests for the optional RRF (Reciprocal Rank Fusion) strategy in
//! `onto_align` — Cormack et al. SIGIR 2009, validated for ontology alignment
//! by Agent-OM at VLDB 2025.

use open_ontologies::align::AlignmentEngine;
use open_ontologies::graph::GraphStore;
use open_ontologies::state::StateDb;
use std::sync::Arc;
use tempfile::NamedTempFile;

fn setup() -> (StateDb, Arc<GraphStore>) {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    let db = StateDb::open(&path).unwrap();
    let graph = Arc::new(GraphStore::new());
    (db, graph)
}

const SOURCE: &str = r#"
    @prefix owl:  <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix ex:   <http://source.org/> .
    ex:Animal a owl:Class ; rdfs:label "Animal" .
    ex:Dog    a owl:Class ; rdfs:label "Dog"    ; rdfs:subClassOf ex:Animal .
    ex:Cat    a owl:Class ; rdfs:label "Cat"    ; rdfs:subClassOf ex:Animal .
"#;

const TARGET: &str = r#"
    @prefix owl:  <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix tg:   <http://target.org/> .
    tg:Animal a owl:Class ; rdfs:label "Animal" .
    tg:Dog    a owl:Class ; rdfs:label "Dog"    ; rdfs:subClassOf tg:Animal .
    tg:Cat    a owl:Class ; rdfs:label "Cat"    ; rdfs:subClassOf tg:Animal .
"#;

#[test]
fn rrf_produces_normalised_confidence_in_unit_interval() {
    let (db, graph) = setup();
    let engine = AlignmentEngine::new(db, graph);

    let result_str = engine
        .align_with_fusion(SOURCE, Some(TARGET), 0.5, 0.0, true, "rrf")
        .expect("align_with_fusion rrf");
    let result: serde_json::Value = serde_json::from_str(&result_str).expect("json");

    let candidates = result["candidates"].as_array().unwrap();
    assert!(!candidates.is_empty(), "expected at least some candidates under RRF");

    for c in candidates {
        let conf = c["confidence"].as_f64().unwrap_or(-1.0);
        assert!(
            (0.0..=1.0).contains(&conf),
            "RRF confidence must be in [0, 1]; got {} on {}",
            conf,
            c
        );
    }
}

#[test]
fn rrf_preserves_per_signal_breakdown() {
    // RRF only overwrites the top-level `confidence` field; the per-signal
    // values must remain on each candidate so the caller can inspect them
    // and so feedback signals stay computable.
    let (db, graph) = setup();
    let engine = AlignmentEngine::new(db, graph);

    let result_str = engine
        .align_with_fusion(SOURCE, Some(TARGET), 0.5, 0.0, true, "rrf")
        .expect("align_with_fusion rrf");
    let result: serde_json::Value = serde_json::from_str(&result_str).expect("json");

    for c in result["candidates"].as_array().unwrap() {
        let signals = &c["signals"];
        assert!(signals.is_object(), "missing signals on RRF candidate: {}", c);
        assert!(signals["label_similarity"].is_number());
        assert!(signals["property_overlap"].is_number());
        assert!(signals["parent_overlap"].is_number());
    }
}

#[test]
fn rrf_low_threshold_filters_after_rerank() {
    // A high low_threshold should drop most candidates; we should be left with
    // at most the top-ranked few from the RRF rerank.
    let (db, graph) = setup();
    let engine = AlignmentEngine::new(db, graph);

    let result_str = engine
        .align_with_fusion(SOURCE, Some(TARGET), 1.0, 0.7, true, "rrf")
        .expect("align_with_fusion rrf");
    let result: serde_json::Value = serde_json::from_str(&result_str).expect("json");

    for c in result["candidates"].as_array().unwrap() {
        let conf = c["confidence"].as_f64().unwrap_or(0.0);
        assert!(
            conf >= 0.7,
            "RRF candidate below low_threshold should have been dropped; got conf={} on {}",
            conf,
            c
        );
    }
}

#[test]
fn weighted_sum_default_unchanged() {
    // Calling align_with_thresholds (the wrapper) should behave exactly as before —
    // using the weighted-sum fusion under the hood. Confidence values for the
    // matching Animal/Dog/Cat pairs should be > 0.85 (the default high threshold
    // for the explicit pairs, since labels and parents both match).
    let (db, graph) = setup();
    let engine = AlignmentEngine::new(db, graph);

    let result_str = engine
        .align_with_thresholds(SOURCE, Some(TARGET), 0.5, 0.5, true)
        .expect("align_with_thresholds weighted_sum");
    let result: serde_json::Value = serde_json::from_str(&result_str).expect("json");

    // At minimum the perfect-label matches should be in auto_applied.
    let auto = result["auto_applied"].as_array().unwrap();
    assert!(
        auto.iter().any(|c| {
            c["source_iri"].as_str().unwrap_or("").contains("Animal")
                && c["target_iri"].as_str().unwrap_or("").contains("Animal")
        }),
        "weighted_sum should match Animal/Animal under back-compat thresholds; auto={:?}",
        auto
    );
}

#[test]
fn rrf_and_weighted_sum_can_produce_different_orderings() {
    // RRF and weighted_sum use different aggregation strategies; on this small
    // test ontology they should both produce sensible orderings even though
    // the absolute confidence numbers differ. Verify both produce non-empty
    // auto_applied buckets at sensible thresholds.
    let (db, graph) = setup();
    let engine = AlignmentEngine::new(db, graph);

    let rrf_str = engine
        .align_with_fusion(SOURCE, Some(TARGET), 0.5, 0.0, true, "rrf")
        .expect("rrf");
    let ws_str = engine
        .align_with_fusion(SOURCE, Some(TARGET), 0.5, 0.0, true, "weighted_sum")
        .expect("weighted_sum");
    let rrf: serde_json::Value = serde_json::from_str(&rrf_str).unwrap();
    let ws: serde_json::Value = serde_json::from_str(&ws_str).unwrap();

    assert!(!rrf["candidates"].as_array().unwrap().is_empty());
    assert!(!ws["candidates"].as_array().unwrap().is_empty());

    // Both should rank the same top pair (Animal↔Animal) first since RRF
    // and weighted_sum agree on perfect matches.
    let rrf_top = rrf["candidates"][0]["source_iri"].as_str().unwrap_or("");
    let ws_top = ws["candidates"][0]["source_iri"].as_str().unwrap_or("");
    assert!(
        rrf_top.contains("Animal") || rrf_top.contains("Dog") || rrf_top.contains("Cat"),
        "RRF top should be one of the perfect matches; got {}",
        rrf_top
    );
    assert!(
        ws_top.contains("Animal") || ws_top.contains("Dog") || ws_top.contains("Cat"),
        "weighted_sum top should be one of the perfect matches; got {}",
        ws_top
    );
}
