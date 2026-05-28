//! mmRAG benchmark adapter (#41, ISWC 2025).
//!
//! Implements the multi-modal RAG evaluation scoring convention from
//! the ISWC 2025 mmRAG paper. Given a set of QA pairs + the retriever's
//! answers + the gold answers, compute:
//!
//!   - **Hit@k**: fraction of questions whose gold-IRI appears in the
//!     retriever's top-k.
//!   - **MRR**: Mean Reciprocal Rank — `1/(rank of first correct hit)`
//!     averaged over questions.
//!   - **Exact-match accuracy**: fraction of questions where the
//!     retriever's top-1 equals the gold.
//!
//! Pairs with `onto_segment_retrieve` (#34) for the retriever side and
//! `graph_projection_lossy_check` (#35) for slice quality audit.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RagQa {
    pub question_id: String,
    /// Gold-standard answer IRI.
    pub gold_iri: String,
    /// Retriever's ranked list of candidate IRIs (most-relevant first).
    pub retrieved: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RagEvalReport {
    pub total: usize,
    pub exact_match_at_1: f64,
    pub hit_at_3: f64,
    pub hit_at_5: f64,
    pub hit_at_10: f64,
    pub mrr: f64,
    /// Per-question rank of the gold IRI; `0` means "not retrieved within
    /// the supplied top-k".
    pub per_question_rank: Vec<usize>,
}

/// Score a batch of QA results.
pub fn evaluate(qas: &[RagQa]) -> RagEvalReport {
    if qas.is_empty() {
        return RagEvalReport {
            total: 0,
            exact_match_at_1: 0.0,
            hit_at_3: 0.0,
            hit_at_5: 0.0,
            hit_at_10: 0.0,
            mrr: 0.0,
            per_question_rank: Vec::new(),
        };
    }
    let n = qas.len() as f64;
    let mut em1 = 0usize;
    let mut h3 = 0usize;
    let mut h5 = 0usize;
    let mut h10 = 0usize;
    let mut mrr_sum = 0.0_f64;
    let mut ranks: Vec<usize> = Vec::with_capacity(qas.len());

    for qa in qas {
        let rank = qa
            .retrieved
            .iter()
            .position(|iri| iri == &qa.gold_iri)
            .map(|i| i + 1)
            .unwrap_or(0);
        ranks.push(rank);
        if rank == 1 {
            em1 += 1;
        }
        if (1..=3).contains(&rank) {
            h3 += 1;
        }
        if (1..=5).contains(&rank) {
            h5 += 1;
        }
        if (1..=10).contains(&rank) {
            h10 += 1;
        }
        if rank > 0 {
            mrr_sum += 1.0 / rank as f64;
        }
    }

    RagEvalReport {
        total: qas.len(),
        exact_match_at_1: em1 as f64 / n,
        hit_at_3: h3 as f64 / n,
        hit_at_5: h5 as f64 / n,
        hit_at_10: h10 as f64 / n,
        mrr: mrr_sum / n,
        per_question_rank: ranks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qa(id: &str, gold: &str, retrieved: &[&str]) -> RagQa {
        RagQa {
            question_id: id.to_string(),
            gold_iri: gold.to_string(),
            retrieved: retrieved.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn perfect_top_1_gives_exact_match_1() {
        let qas = vec![
            qa("q1", "A", &["A", "B", "C"]),
            qa("q2", "X", &["X", "Y"]),
        ];
        let r = evaluate(&qas);
        assert_eq!(r.exact_match_at_1, 1.0);
        assert_eq!(r.mrr, 1.0);
        assert_eq!(r.hit_at_3, 1.0);
    }

    #[test]
    fn gold_not_retrieved_gives_rank_0() {
        let qas = vec![qa("q1", "Z", &["A", "B", "C"])];
        let r = evaluate(&qas);
        assert_eq!(r.exact_match_at_1, 0.0);
        assert_eq!(r.mrr, 0.0);
        assert_eq!(r.per_question_rank[0], 0);
    }

    #[test]
    fn mrr_weights_top_ranks_higher() {
        // q1: rank 1 → 1.0; q2: rank 4 → 0.25. MRR = (1.0 + 0.25) / 2 = 0.625.
        let qas = vec![
            qa("q1", "A", &["A", "B", "C", "D"]),
            qa("q2", "X", &["W", "Y", "Z", "X"]),
        ];
        let r = evaluate(&qas);
        assert!((r.mrr - 0.625).abs() < 1e-9, "got mrr={}", r.mrr);
        assert_eq!(r.per_question_rank, vec![1, 4]);
    }

    #[test]
    fn hit_at_k_buckets_are_monotonic() {
        // Rank 5: hit@5=1, hit@3=0, hit@10=1.
        let qas = vec![qa("q1", "X", &["A", "B", "C", "D", "X"])];
        let r = evaluate(&qas);
        assert_eq!(r.hit_at_3, 0.0);
        assert_eq!(r.hit_at_5, 1.0);
        assert_eq!(r.hit_at_10, 1.0);
    }

    #[test]
    fn empty_input_gives_zero_metrics() {
        let r = evaluate(&[]);
        assert_eq!(r.total, 0);
        assert_eq!(r.mrr, 0.0);
        assert_eq!(r.exact_match_at_1, 0.0);
    }
}
