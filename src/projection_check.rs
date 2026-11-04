// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Graph-projection loss audit (#35).
//!
//! Per the IJCAI 2025 paper "How to Mitigate Information Loss in Knowledge
//! Graphs for GraphRAG", lossy graph projections silently degrade downstream
//! retrieval quality. This module's `check_projection_loss` audits whether a
//! projected Turtle slice has dropped predicates / objects / structural
//! patterns vs the full neighbourhood of the seed IRIs in the loaded ontology.
//!
//! Designed as a complementary primitive to `onto_segment_retrieve` (#34) — the
//! retriever produces the slice; this auditor reports what it left behind.
//!
//! # `coverage_ratio` is a PROXY and it is demoted, not deleted
//!
//! It is neither necessary nor sufficient for the property that matters. A
//! slice at 0.99 can have dropped the one triple an answer rests on, and a
//! slice at 0.60 can preserve every conclusion a report asks about. It also
//! moves the wrong way: it rises as the projection grows, so a retriever tuned
//! on it learns to fetch MORE rather than to fetch the right thing.
//!
//! Measured on this repository's own `benchmark/reference/pizza-reference.owl`:
//! the whole ontology minus the single triple `NamedPizza rdfs:subClassOf
//! Pizza` scores `aggregate_coverage_ratio: 1.0` and `ok: true` over the 23
//! named-pizza seeds, while `Veneziana rdfs:subClassOf Food` — a conclusion the
//! source derives — is gone. The number cannot see the damage at all.
//!
//! The property is entailment preservation, and it is asked by
//! [`crate::projection_entailment`] (goal-directed, one answer) and
//! [`crate::closure_diff`] (goal-free, a whole retrieval strategy). This module
//! stays because a cheap signal is worth keeping; the label stays with it
//! because an unlabelled one is a trap.

use crate::graph::GraphStore;
use serde::{Deserialize, Serialize};

/// The sentence that travels with the number, in the payload rather than only
/// in the docs. Re-exported from [`crate::projection_entailment`] so there is
/// one spelling of it in the crate.
pub use crate::projection_entailment::COVERAGE_LABEL;
use std::collections::BTreeSet;
use std::sync::Arc;

/// Per-seed loss report.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeedLossReport {
    pub seed_iri: String,
    /// Predicates appearing in the source but absent from the projection.
    pub dropped_predicates: Vec<String>,
    /// Object IRIs appearing in the source but absent from the projection.
    pub dropped_objects: Vec<String>,
    /// Total source-side triples touching this seed.
    pub source_triples: u64,
    /// Total projection-side triples touching this seed.
    pub projected_triples: u64,
    /// `projected_triples / source_triples` (0.0 if source is empty).
    pub coverage_ratio: f64,
}

/// Top-level audit report aggregating per-seed losses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectionLossReport {
    /// Whether projection parsed as Turtle.
    pub projection_parses: bool,
    /// Aggregate coverage ratio across all seeds (mean of per-seed ratios).
    pub aggregate_coverage_ratio: f64,
    /// Per-seed details.
    pub per_seed: Vec<SeedLossReport>,
    /// True iff projection_parses AND all per-seed coverage_ratio == 1.0.
    pub ok: bool,
    /// Total dropped predicates across all seeds (deduped).
    pub total_dropped_predicates: usize,
    /// Total dropped objects across all seeds (deduped).
    pub total_dropped_objects: usize,
    /// Always false. A FIELD rather than a doc comment, so a consumer reading
    /// the JSON sees it without reading prose, and a renderer that walks only
    /// the data still emits it.
    pub is_a_warrant: bool,
    /// [`COVERAGE_LABEL`], carried in the payload.
    pub warning: &'static str,
    /// Seeds whose projection side holds MORE triples than the source side.
    ///
    /// `coverage_ratio` clamps with `.min()`, so such a seed reads 1.0 with
    /// empty dropped lists, and `ok` used to be true on exactly the input that
    /// should stop a pipeline: a projection that is not a subset of the source,
    /// which is either a hallucinating retriever or a slice of a different
    /// graph. The clamp stays, because an unclamped "ratio" above 1.0 is not a
    /// ratio; the surplus is named instead and `ok` now requires it to be
    /// empty.
    pub seeds_with_surplus: Vec<String>,
}

/// Audit a projected Turtle slice against the loaded ontology's full
/// neighbourhood of the seed IRIs.
///
/// Returns a `ProjectionLossReport` with per-seed dropped predicates/objects
/// and aggregate coverage. The loaded graph (`graph`) is the source of truth;
/// `projected_ttl` is the slice being audited.
pub fn check_projection_loss(
    graph: &Arc<GraphStore>,
    seed_iris: &[String],
    projected_ttl: &str,
) -> anyhow::Result<ProjectionLossReport> {
    // Load the projection into a temp store. Parse failure ⇒ projection_parses=false.
    let projected = GraphStore::new();
    let parse_ok = projected.load_turtle(projected_ttl, None).is_ok();
    if !parse_ok {
        // NOTE: `aggregate_coverage_ratio: 0.0` on a parse failure is this
        // module's oldest trap: "we could not read the file" renders as "0%
        // coverage". `projection_parses` is the field that separates them, and
        // `projection_entailment::coverage_proxy` drops the number entirely
        // rather than pass a 0.0 on to a reader.
        return Ok(ProjectionLossReport {
            projection_parses: false,
            aggregate_coverage_ratio: 0.0,
            per_seed: Vec::new(),
            ok: false,
            total_dropped_predicates: 0,
            total_dropped_objects: 0,
            is_a_warrant: false,
            warning: COVERAGE_LABEL,
            seeds_with_surplus: Vec::new(),
        });
    }

    let projected_ref: Arc<GraphStore> = Arc::new(projected);

    let mut per_seed = Vec::with_capacity(seed_iris.len());
    let mut all_dropped_preds: BTreeSet<String> = BTreeSet::new();
    let mut all_dropped_objs: BTreeSet<String> = BTreeSet::new();
    let mut sum_ratio = 0.0;

    for iri in seed_iris {
        // Collect (predicate, object) pairs for this seed from both stores.
        let source_pairs = neighbourhood_pairs(graph, iri)?;
        let projected_pairs = neighbourhood_pairs(&projected_ref, iri)?;

        let source_preds: BTreeSet<&str> = source_pairs.iter().map(|(p, _)| p.as_str()).collect();
        let projected_preds: BTreeSet<&str> =
            projected_pairs.iter().map(|(p, _)| p.as_str()).collect();
        let dropped_preds: Vec<String> = source_preds
            .difference(&projected_preds)
            .map(|s| s.to_string())
            .collect();

        let source_objs: BTreeSet<&str> = source_pairs.iter().map(|(_, o)| o.as_str()).collect();
        let projected_objs: BTreeSet<&str> = projected_pairs.iter().map(|(_, o)| o.as_str()).collect();
        let dropped_objs: Vec<String> = source_objs
            .difference(&projected_objs)
            .map(|s| s.to_string())
            .collect();

        let source_n = source_pairs.len() as u64;
        let projected_n = projected_pairs.len() as u64;
        let coverage = if source_n == 0 {
            1.0
        } else {
            (projected_n.min(source_n) as f64) / (source_n as f64)
        };

        for d in &dropped_preds {
            all_dropped_preds.insert(d.clone());
        }
        for d in &dropped_objs {
            all_dropped_objs.insert(d.clone());
        }
        sum_ratio += coverage;

        per_seed.push(SeedLossReport {
            seed_iri: iri.clone(),
            dropped_predicates: dropped_preds,
            dropped_objects: dropped_objs,
            source_triples: source_n,
            projected_triples: projected_n,
            coverage_ratio: coverage,
        });
    }

    let n = seed_iris.len().max(1) as f64;
    let aggregate = sum_ratio / n;
    let surplus: Vec<String> = per_seed
        .iter()
        .filter(|r| r.projected_triples > r.source_triples)
        .map(|r| r.seed_iri.clone())
        .collect();
    let ok = surplus.is_empty()
        && per_seed.iter().all(|r| (r.coverage_ratio - 1.0).abs() < 1e-9);

    Ok(ProjectionLossReport {
        projection_parses: true,
        aggregate_coverage_ratio: aggregate,
        per_seed,
        ok,
        total_dropped_predicates: all_dropped_preds.len(),
        total_dropped_objects: all_dropped_objs.len(),
        is_a_warrant: false,
        warning: COVERAGE_LABEL,
        seeds_with_surplus: surplus,
    })
}

/// All `(predicate, object)` pairs where `iri` appears as the subject. We use
/// subject-only as the canonical neighbourhood definition for the projection
/// check; CIVeX-style structural-dependency closure would extend this to
/// inbound edges, but for "did the slice preserve this seed's outbound
/// description?" the subject view is the right unit.
fn neighbourhood_pairs(
    graph: &Arc<GraphStore>,
    iri: &str,
) -> anyhow::Result<Vec<(String, String)>> {
    let q = format!(
        r#"SELECT DISTINCT ?p ?o WHERE {{ <{iri}> ?p ?o }} LIMIT 1000"#
    );
    let mut out = Vec::new();
    let json_str = graph.sparql_select(&q)?;
    let parsed: serde_json::Value = serde_json::from_str(&json_str)?;
    if let Some(rows) = parsed["results"].as_array() {
        for row in rows {
            let p = row["p"].as_str().unwrap_or("").trim_matches(|c| c == '<' || c == '>').to_string();
            let o = row["o"].as_str().unwrap_or("").trim_matches(|c| c == '<' || c == '>').to_string();
            if !p.is_empty() {
                out.push((p, o));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loaded(ttl: &str) -> Arc<GraphStore> {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(ttl, None).expect("load");
        g
    }

    #[test]
    fn full_projection_reports_ok_and_full_coverage() {
        let source = r#"
            @prefix ex: <http://ex.org/> .
            ex:Cat ex:hasColour ex:Black ; ex:age 5 .
        "#;
        let projection = source; // identical
        let g = loaded(source);
        let report = check_projection_loss(&g, &["http://ex.org/Cat".to_string()], projection).unwrap();
        assert!(report.ok);
        assert_eq!(report.aggregate_coverage_ratio, 1.0);
        assert_eq!(report.total_dropped_predicates, 0);
    }

    #[test]
    fn dropped_predicate_is_reported() {
        let source = r#"
            @prefix ex: <http://ex.org/> .
            ex:Cat ex:hasColour ex:Black ; ex:age 5 ; ex:species "Felis catus" .
        "#;
        // Projection keeps only hasColour; drops age and species.
        let projection = r#"
            @prefix ex: <http://ex.org/> .
            ex:Cat ex:hasColour ex:Black .
        "#;
        let g = loaded(source);
        let report = check_projection_loss(&g, &["http://ex.org/Cat".to_string()], projection).unwrap();
        assert!(!report.ok);
        assert!(report.total_dropped_predicates >= 2);
        let cat_report = &report.per_seed[0];
        assert!(cat_report.dropped_predicates.iter().any(|p| p.contains("age")));
        assert!(cat_report.dropped_predicates.iter().any(|p| p.contains("species")));
    }

    /// The clamp used to report a clean bill of health on the one input that
    /// should stop a pipeline: a projection holding MORE about a seed than the
    /// source does, which is a hallucinating retriever or a slice of a
    /// different graph.
    #[test]
    fn a_projection_with_more_than_the_source_is_not_ok() {
        let source = r#"
            @prefix ex: <http://ex.org/> .
            ex:Cat ex:hasColour ex:Black .
        "#;
        let projection = r#"
            @prefix ex: <http://ex.org/> .
            ex:Cat ex:hasColour ex:Black ; ex:invented ex:Nonsense .
        "#;
        let g = loaded(source);
        let r = check_projection_loss(&g, &["http://ex.org/Cat".to_string()], projection).unwrap();
        assert_eq!(r.per_seed[0].coverage_ratio, 1.0, "the clamp still clamps");
        assert!(r.per_seed[0].dropped_predicates.is_empty(), "nothing was dropped");
        assert_eq!(r.seeds_with_surplus.len(), 1, "the surplus must be named: {r:?}");
        assert!(!r.ok, "a non-subset projection must not report ok: {r:?}");
    }

    #[test]
    fn the_label_travels_with_the_number() {
        let g = loaded(r#"@prefix ex: <http://ex.org/> . ex:X ex:p ex:Y ."#);
        for ttl in [r#"@prefix ex: <http://ex.org/> . ex:X ex:p ex:Y ."#, "not turtle <<<"] {
            let r = check_projection_loss(&g, &["http://ex.org/X".to_string()], ttl).unwrap();
            let json = serde_json::to_string(&r).unwrap();
            assert!(!r.is_a_warrant);
            assert!(json.contains("is neither necessary nor sufficient"), "{json}");
            assert!(
                !json.contains("\"ratio\"") || json.contains("PROXY"),
                "wherever the number appears the label must appear: {json}"
            );
        }
    }

    #[test]
    fn invalid_projection_turtle_reports_parses_false() {
        let g = loaded(r#"@prefix ex: <http://ex.org/> . ex:X ex:p ex:Y ."#);
        let report = check_projection_loss(&g, &["http://ex.org/X".to_string()], "this is not turtle: << >>").unwrap();
        assert!(!report.projection_parses);
        assert!(!report.ok);
    }

    #[test]
    fn missing_seed_in_source_has_full_coverage_by_definition() {
        // If the seed has no triples in source, there's nothing to drop —
        // coverage is trivially 1.0.
        let g = loaded(r#"@prefix ex: <http://ex.org/> . ex:Other ex:p ex:Z ."#);
        let report = check_projection_loss(
            &g,
            &["http://ex.org/UnknownSeed".to_string()],
            r#"@prefix ex: <http://ex.org/> . ex:Other ex:p ex:Z ."#,
        )
        .unwrap();
        assert!(report.ok);
        assert_eq!(report.per_seed[0].source_triples, 0);
    }
}
