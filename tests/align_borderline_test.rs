//! Integration tests for the MCP-native LLM-as-oracle pattern (#16).
//!
//! Verifies that `AlignmentEngine::align_with_thresholds` correctly partitions candidates
//! into `auto_applied` (≥ high_threshold) and `borderline` (in [low_threshold, high_threshold))
//! buckets, and that borderline candidates carry the context fields (labels, parents) the
//! calling LLM needs to judge them.

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
fn degenerate_thresholds_preserve_legacy_behaviour() {
    // low == high → no borderline bucket. Output shape matches the old align() semantics.
    let (db, graph) = setup();
    let engine = AlignmentEngine::new(db, graph);

    let result_str = engine
        .align_with_thresholds(SOURCE, Some(TARGET), 0.5, 0.5, true)
        .expect("align_with_thresholds");
    let result: serde_json::Value = serde_json::from_str(&result_str).expect("json");

    assert_eq!(
        result["borderline_count"].as_u64().unwrap_or(99),
        0,
        "degenerate range must produce empty borderline bucket"
    );
    assert!(
        result["borderline"].as_array().unwrap().is_empty(),
        "borderline array must be empty when low == high"
    );
    // All candidates that survived stable matching should be in auto_applied.
    let candidates = result["candidates"].as_array().unwrap();
    let auto_applied = result["auto_applied"].as_array().unwrap();
    assert_eq!(auto_applied.len(), candidates.len());
}

#[test]
fn borderline_bucket_collects_mid_confidence_candidates() {
    // With a wide range, mid-confidence matches land in borderline.
    let (db, graph) = setup();
    let engine = AlignmentEngine::new(db, graph);

    // high=0.95 is above what label-only matching produces here; low=0.4 catches
    // the label-similarity-only matches.
    let result_str = engine
        .align_with_thresholds(SOURCE, Some(TARGET), 0.95, 0.4, true)
        .expect("align_with_thresholds");
    let result: serde_json::Value = serde_json::from_str(&result_str).expect("json");

    let borderline = result["borderline"].as_array().unwrap();
    assert!(
        !borderline.is_empty(),
        "expected at least one borderline candidate with high=0.95, low=0.4; got result:\n{}",
        result_str
    );

    // Every borderline candidate must have the LLM-review marker and the context block.
    for cand in borderline {
        assert_eq!(
            cand["requires_review"].as_bool(),
            Some(true),
            "borderline candidate missing requires_review=true: {}",
            cand
        );
        let ctx = &cand["context"];
        assert!(
            ctx.is_object(),
            "borderline candidate missing `context` field: {}",
            cand
        );
        assert!(ctx["source_labels"].is_array(), "context.source_labels missing");
        assert!(ctx["target_labels"].is_array(), "context.target_labels missing");
        assert!(ctx["source_parents"].is_array(), "context.source_parents missing");
        assert!(ctx["target_parents"].is_array(), "context.target_parents missing");
    }
}

#[test]
fn auto_applied_candidates_lack_context_field() {
    // Context enrichment is for borderline only — auto-applied candidates skip the parent
    // lookups, keeping the hot path cheap on large ontologies.
    let (db, graph) = setup();
    let engine = AlignmentEngine::new(db, graph);

    let result_str = engine
        .align_with_thresholds(SOURCE, Some(TARGET), 0.5, 0.5, true)
        .expect("align_with_thresholds");
    let result: serde_json::Value = serde_json::from_str(&result_str).expect("json");

    for cand in result["auto_applied"].as_array().unwrap() {
        assert_eq!(cand["requires_review"].as_bool(), Some(false));
        assert!(
            cand["context"].is_null(),
            "auto_applied candidates must not carry context to keep enrichment cost off the hot path; got {}",
            cand
        );
    }
}

#[test]
fn summary_for_review_present_when_borderline_nonempty() {
    let (db, graph) = setup();
    let engine = AlignmentEngine::new(db, graph);

    let result_str = engine
        .align_with_thresholds(SOURCE, Some(TARGET), 0.95, 0.4, true)
        .expect("align_with_thresholds");
    let result: serde_json::Value = serde_json::from_str(&result_str).expect("json");

    let borderline_count = result["borderline_count"].as_u64().unwrap_or(0);
    let summary = result["summary_for_review"].as_str().unwrap_or("");
    if borderline_count > 0 {
        assert!(
            summary.contains("onto_align_feedback"),
            "summary must instruct the LLM to call onto_align_feedback; got: {}",
            summary
        );
        assert!(
            summary.contains("borderline"),
            "summary must mention borderline; got: {}",
            summary
        );
    }
}

#[test]
fn back_compat_align_method_still_works() {
    // The old align(source, target, min_confidence, dry_run) signature is still callable
    // and produces a sensible (degenerate-bucket) result.
    let (db, graph) = setup();
    let engine = AlignmentEngine::new(db, graph);

    let result_str = engine
        .align(SOURCE, Some(TARGET), 0.5, true)
        .expect("legacy align");
    let result: serde_json::Value = serde_json::from_str(&result_str).expect("json");

    // No borderline produced under back-compat (low_threshold = min_confidence).
    assert_eq!(result["borderline_count"].as_u64().unwrap_or(99), 0);
    // Existing keys still present.
    assert!(result["candidates"].is_array());
    assert!(result["total_candidates"].is_number());
    assert!(result["threshold"].is_number());
}
