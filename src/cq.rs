//! Competency-question runner + LLM-assisted verification (#29 + #39).
//!
//! Competency questions (CQs) are the OBO/Foundry standard for documenting
//! what an ontology is supposed to answer. A CQ is typically a natural-
//! language question paired with a SPARQL query whose result set
//! constitutes the answer.
//!
//! This module provides:
//!
//!   - `run_cq_suite` (#29): run a batch of CQs, return pass/fail per CQ
//!     plus VSPO-pitfall hints when a CQ returns empty or returns
//!     surprising shape.
//!   - `verify_cq` (#39, ISWC 2025 Lippolis): take a CQ result + an
//!     LLM-supplied judgement (was the answer correct?) and persist it as
//!     feedback. The server doesn't run the LLM — it stores + retrieves
//!     verdicts, mirroring the borderline-pair convention from `onto_align`.

use crate::graph::GraphStore;
use crate::state::StateDb;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompetencyQuestion {
    pub id: String,
    pub question: String,
    pub sparql: String,
    /// Optional expected row count (None = any non-empty result is a pass).
    #[serde(default)]
    pub expected_min_rows: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CqRunReport {
    pub total: usize,
    pub passed: usize,
    pub results: Vec<CqResult>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CqResult {
    pub id: String,
    pub passed: bool,
    pub row_count: usize,
    /// VSPO-style pitfall hints (Villalón-Suárez-Poveda-Otero pitfall list).
    pub pitfalls: Vec<String>,
    /// Truncated raw result for inspection.
    pub raw_excerpt: String,
}

const VSPO_HINT_EMPTY: &str = "P10: ontology returns no answer (possible missing rdfs:domain/range or sub-ontology declaration)";
const VSPO_HINT_NO_LABELS: &str = "P11: no rdfs:label on returned IRIs (LLM cannot phrase the answer)";
const VSPO_HINT_INFINITE: &str = "P12: result row count > 10000 (possible cycle in rdfs:subClassOf or no LIMIT clause)";

/// Run a batch of competency questions against the loaded graph.
pub fn run_cq_suite(graph: &Arc<GraphStore>, cqs: &[CompetencyQuestion]) -> CqRunReport {
    let mut results: Vec<CqResult> = Vec::with_capacity(cqs.len());
    let mut passed = 0usize;
    for cq in cqs {
        let mut pitfalls: Vec<String> = Vec::new();
        let (row_count, excerpt) = match graph.sparql_select(&cq.sparql) {
            Ok(js) => {
                let v: serde_json::Value =
                    serde_json::from_str(&js).unwrap_or(serde_json::Value::Null);
                let rows = v["results"].as_array().cloned().unwrap_or_default();
                if rows.is_empty() {
                    pitfalls.push(VSPO_HINT_EMPTY.to_string());
                }
                if rows.len() > 10000 {
                    pitfalls.push(VSPO_HINT_INFINITE.to_string());
                }
                // No-labels heuristic: scan first 5 rows for any rdfs:label.
                let snip: String = rows
                    .iter()
                    .take(5)
                    .map(|r| r.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                if !snip.is_empty() && !snip.contains("label") {
                    pitfalls.push(VSPO_HINT_NO_LABELS.to_string());
                }
                let excerpt = if js.len() > 800 {
                    format!("{}...", &js[..800])
                } else {
                    js
                };
                (rows.len(), excerpt)
            }
            Err(e) => (0, format!("error: {}", e)),
        };
        let pass = match cq.expected_min_rows {
            Some(n) => row_count >= n,
            None => row_count > 0,
        };
        if pass {
            passed += 1;
        }
        results.push(CqResult {
            id: cq.id.clone(),
            passed: pass,
            row_count,
            pitfalls,
            raw_excerpt: excerpt,
        });
    }
    CqRunReport {
        total: cqs.len(),
        passed,
        results,
    }
}

// ─── Verification feedback persistence (#39) ────────────────────────────────

const ENSURE_CQ_VERDICT_TABLE: &str = "
CREATE TABLE IF NOT EXISTS cq_verdicts (
    cq_id TEXT NOT NULL,
    verdict TEXT NOT NULL CHECK (verdict IN ('correct', 'incorrect', 'partial')),
    rationale TEXT,
    judge TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (cq_id, judge, created_at)
)";

fn ensure_cq_verdicts(db: &StateDb) -> anyhow::Result<()> {
    db.conn().execute(ENSURE_CQ_VERDICT_TABLE, [])?;
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CqVerdict {
    pub cq_id: String,
    /// One of `"correct"`, `"incorrect"`, `"partial"`.
    pub verdict: String,
    #[serde(default)]
    pub rationale: Option<String>,
    /// Identifier of the judging entity (e.g. `"claude"`, `"alice@org"`).
    #[serde(default)]
    pub judge: Option<String>,
}

/// Persist an LLM-supplied (or human-supplied) verdict on a CQ result.
pub fn verify_cq(db: &StateDb, verdict: &CqVerdict) -> anyhow::Result<()> {
    if !matches!(verdict.verdict.as_str(), "correct" | "incorrect" | "partial") {
        anyhow::bail!(
            "invalid verdict `{}`: must be correct/incorrect/partial",
            verdict.verdict
        );
    }
    ensure_cq_verdicts(db)?;
    db.conn().execute(
        "INSERT INTO cq_verdicts (cq_id, verdict, rationale, judge) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![
            verdict.cq_id,
            verdict.verdict,
            verdict.rationale,
            verdict.judge.clone().unwrap_or_else(|| "anonymous".to_string()),
        ],
    )?;
    Ok(())
}

/// List all verdicts for a given CQ, most-recent first.
pub fn list_cq_verdicts(db: &StateDb, cq_id: &str) -> anyhow::Result<Vec<CqVerdict>> {
    ensure_cq_verdicts(db)?;
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT cq_id, verdict, rationale, judge FROM cq_verdicts WHERE cq_id = ?1 ORDER BY created_at DESC",
    )?;
    let rows: Vec<CqVerdict> = stmt
        .query_map(rusqlite::params![cq_id], |r| {
            Ok(CqVerdict {
                cq_id: r.get(0)?,
                verdict: r.get(1)?,
                rationale: r.get(2)?,
                judge: r.get(3)?,
            })
        })?
        .filter_map(Result::ok)
        .collect();
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn fresh_db() -> StateDb {
        StateDb::open(Path::new(":memory:")).unwrap()
    }

    fn graph_with_classes() -> Arc<GraphStore> {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(
            r#"
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            @prefix ex: <http://ex.org/> .
            ex:Cat a owl:Class ; rdfs:label "Cat" .
            ex:Dog a owl:Class ; rdfs:label "Dog" .
        "#,
            None,
        )
        .unwrap();
        g
    }

    #[test]
    fn run_cq_suite_marks_non_empty_result_as_pass() {
        let g = graph_with_classes();
        let cqs = vec![CompetencyQuestion {
            id: "cq1".to_string(),
            question: "What classes are declared?".to_string(),
            sparql: "SELECT ?c WHERE { ?c a <http://www.w3.org/2002/07/owl#Class> }".to_string(),
            expected_min_rows: None,
        }];
        let report = run_cq_suite(&g, &cqs);
        assert_eq!(report.passed, 1);
        assert_eq!(report.results[0].id, "cq1");
        assert!(report.results[0].passed);
    }

    #[test]
    fn run_cq_suite_flags_empty_with_vspo_p10() {
        let g = graph_with_classes();
        let cqs = vec![CompetencyQuestion {
            id: "cq_empty".to_string(),
            question: "Find non-existent class.".to_string(),
            sparql: "SELECT ?c WHERE { ?c a <http://ex.org/NoSuchClass> }".to_string(),
            expected_min_rows: None,
        }];
        let report = run_cq_suite(&g, &cqs);
        assert_eq!(report.passed, 0);
        assert!(report.results[0].pitfalls.iter().any(|p| p.starts_with("P10")));
    }

    #[test]
    fn verify_cq_persists_and_lists_verdicts() {
        let db = fresh_db();
        verify_cq(
            &db,
            &CqVerdict {
                cq_id: "cq1".to_string(),
                verdict: "correct".to_string(),
                rationale: Some("Returned both Cat and Dog as expected".to_string()),
                judge: Some("claude".to_string()),
            },
        )
        .unwrap();
        let listed = list_cq_verdicts(&db, "cq1").unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].verdict, "correct");
    }

    #[test]
    fn verify_cq_rejects_invalid_verdict_label() {
        let db = fresh_db();
        let err = verify_cq(
            &db,
            &CqVerdict {
                cq_id: "cq1".to_string(),
                verdict: "maybe".to_string(),
                rationale: None,
                judge: None,
            },
        )
        .expect_err("should reject");
        assert!(format!("{}", err).contains("invalid verdict"));
    }

    #[test]
    fn run_cq_suite_respects_expected_min_rows() {
        let g = graph_with_classes();
        let cqs = vec![CompetencyQuestion {
            id: "cq_min".to_string(),
            question: "At least 5 classes?".to_string(),
            sparql: "SELECT ?c WHERE { ?c a <http://www.w3.org/2002/07/owl#Class> }".to_string(),
            expected_min_rows: Some(5),
        }];
        let report = run_cq_suite(&g, &cqs);
        // Only 2 classes in seed; min 5 → fail.
        assert!(!report.results[0].passed);
    }
}
