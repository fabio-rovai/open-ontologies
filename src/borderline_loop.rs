//! Generalised iterative borderline-pair review loop (#37, NORA NeurIPS
//! 2025).
//!
//! Generalises the borderline-pair pattern from `onto_align` (#16) to any
//! candidate set: given a list of `(item, score)` candidates plus two
//! thresholds, partition into `auto_accept` (above high), `borderline`
//! (between low and high), `auto_reject` (below low), and emit a summary
//! prompt instructing the orchestrator to judge the borderline bucket.
//!
//! The judgement persists via `record_verdict`, and `apply_verdicts`
//! merges accepted-borderline items back into the auto-accept set.

use crate::state::StateDb;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Candidate {
    pub id: String,
    pub score: f64,
    /// Free-form context for the LLM's judgement (e.g. labels, parents).
    #[serde(default)]
    pub context: serde_json::Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct PartitionReport {
    pub auto_accept: Vec<Candidate>,
    pub borderline: Vec<Candidate>,
    pub auto_reject: Vec<Candidate>,
    pub summary_for_review: String,
}

/// Partition `candidates` by two thresholds. Items with `score >= high`
/// land in `auto_accept`; `[low, high)` in `borderline`; `< low` in
/// `auto_reject`.
pub fn partition(candidates: Vec<Candidate>, low: f64, high: f64) -> PartitionReport {
    let (mut auto_a, mut bord, mut auto_r) = (Vec::new(), Vec::new(), Vec::new());
    for c in candidates {
        if c.score >= high {
            auto_a.push(c);
        } else if c.score >= low {
            bord.push(c);
        } else {
            auto_r.push(c);
        }
    }
    let summary = format!(
        "Review {} borderline candidate(s) in range [{:.3}, {:.3}). For each, call \
         `borderline_record_verdict` with verdict `\"accept\"` or `\"reject\"`. \
         {} auto-accepted, {} auto-rejected.",
        bord.len(),
        low,
        high,
        auto_a.len(),
        auto_r.len()
    );
    PartitionReport {
        auto_accept: auto_a,
        borderline: bord,
        auto_reject: auto_r,
        summary_for_review: summary,
    }
}

// ─── Verdict persistence ────────────────────────────────────────────────────

const ENSURE_TABLE: &str = "
CREATE TABLE IF NOT EXISTS borderline_verdicts (
    candidate_id TEXT NOT NULL,
    namespace TEXT NOT NULL DEFAULT 'default',
    verdict TEXT NOT NULL CHECK (verdict IN ('accept', 'reject')),
    rationale TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (candidate_id, namespace, created_at)
)";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BorderlineVerdict {
    pub candidate_id: String,
    /// Logical namespace ("default" if unspecified). Lets a single server
    /// host multiple independent borderline loops without collision.
    #[serde(default = "default_namespace")]
    pub namespace: String,
    /// Either "accept" or "reject".
    pub verdict: String,
    #[serde(default)]
    pub rationale: Option<String>,
}

fn default_namespace() -> String {
    "default".to_string()
}

fn ensure(db: &StateDb) -> anyhow::Result<()> {
    db.conn().execute(ENSURE_TABLE, [])?;
    Ok(())
}

pub fn record_verdict(db: &StateDb, v: &BorderlineVerdict) -> anyhow::Result<()> {
    if !matches!(v.verdict.as_str(), "accept" | "reject") {
        anyhow::bail!("invalid verdict `{}`: must be accept/reject", v.verdict);
    }
    ensure(db)?;
    db.conn().execute(
        "INSERT INTO borderline_verdicts (candidate_id, namespace, verdict, rationale) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![v.candidate_id, v.namespace, v.verdict, v.rationale],
    )?;
    Ok(())
}

/// Return the most-recent verdict for a candidate id in the given namespace,
/// or `None` if no verdict has been recorded.
pub fn latest_verdict(db: &StateDb, namespace: &str, candidate_id: &str) -> anyhow::Result<Option<BorderlineVerdict>> {
    ensure(db)?;
    let conn = db.conn();
    let row: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT verdict, rationale FROM borderline_verdicts \
             WHERE candidate_id = ?1 AND namespace = ?2 ORDER BY created_at DESC LIMIT 1",
            rusqlite::params![candidate_id, namespace],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();
    Ok(row.map(|(verdict, rationale)| BorderlineVerdict {
        candidate_id: candidate_id.to_string(),
        namespace: namespace.to_string(),
        verdict,
        rationale,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn fresh_db() -> StateDb {
        StateDb::open(Path::new(":memory:")).unwrap()
    }

    fn cand(id: &str, score: f64) -> Candidate {
        Candidate { id: id.to_string(), score, context: serde_json::Value::Null }
    }

    #[test]
    fn partition_splits_by_two_thresholds() {
        let cs = vec![cand("a", 0.95), cand("b", 0.7), cand("c", 0.2)];
        let r = partition(cs, 0.4, 0.85);
        assert_eq!(r.auto_accept.len(), 1);
        assert_eq!(r.auto_accept[0].id, "a");
        assert_eq!(r.borderline.len(), 1);
        assert_eq!(r.borderline[0].id, "b");
        assert_eq!(r.auto_reject.len(), 1);
        assert_eq!(r.auto_reject[0].id, "c");
    }

    #[test]
    fn summary_text_reflects_counts() {
        let cs = vec![cand("a", 0.9), cand("b", 0.6), cand("c", 0.6), cand("d", 0.1)];
        let r = partition(cs, 0.5, 0.85);
        assert!(r.summary_for_review.contains("2 borderline"));
        assert!(r.summary_for_review.contains("1 auto-accepted"));
        assert!(r.summary_for_review.contains("1 auto-rejected"));
    }

    #[test]
    fn record_verdict_round_trip() {
        let db = fresh_db();
        record_verdict(
            &db,
            &BorderlineVerdict {
                candidate_id: "pair_42".to_string(),
                namespace: "alignment".to_string(),
                verdict: "accept".to_string(),
                rationale: Some("labels matched after stemming".to_string()),
            },
        )
        .unwrap();
        let v = latest_verdict(&db, "alignment", "pair_42").unwrap().unwrap();
        assert_eq!(v.verdict, "accept");
    }

    #[test]
    fn record_verdict_rejects_invalid_label() {
        let db = fresh_db();
        let err = record_verdict(
            &db,
            &BorderlineVerdict {
                candidate_id: "x".to_string(),
                namespace: "default".to_string(),
                verdict: "maybe".to_string(),
                rationale: None,
            },
        )
        .expect_err("should reject");
        assert!(format!("{}", err).contains("invalid verdict"));
    }

    #[test]
    fn namespaces_isolate_verdicts() {
        let db = fresh_db();
        record_verdict(
            &db,
            &BorderlineVerdict {
                candidate_id: "x".to_string(),
                namespace: "ns_a".to_string(),
                verdict: "accept".to_string(),
                rationale: None,
            },
        )
        .unwrap();
        record_verdict(
            &db,
            &BorderlineVerdict {
                candidate_id: "x".to_string(),
                namespace: "ns_b".to_string(),
                verdict: "reject".to_string(),
                rationale: None,
            },
        )
        .unwrap();
        let va = latest_verdict(&db, "ns_a", "x").unwrap().unwrap();
        let vb = latest_verdict(&db, "ns_b", "x").unwrap().unwrap();
        assert_eq!(va.verdict, "accept");
        assert_eq!(vb.verdict, "reject");
    }
}
