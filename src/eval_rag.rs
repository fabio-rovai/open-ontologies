//! mmRAG benchmark adapter (#41, ISWC 2025).
//!
//! Implements the full multi-modal RAG evaluation scoring convention from
//! the ISWC 2025 mmRAG paper. Three scoring layers:
//!
//!   1. **Retrieval IR metrics** (always computable from `retrieved` +
//!      `gold_iri`):
//!        - Hit@k for k ∈ {3, 5, 10}.
//!        - MRR (Mean Reciprocal Rank).
//!        - Exact-match-at-1.
//!
//!   2. **Faithfulness** (when `generated_answer` + `retrieved` are both
//!      supplied): fraction of generated-answer tokens that appear in the
//!      concatenated text of the retrieved IRIs' rdfs:labels (or in the
//!      caller-supplied `retrieved_text`). Detects hallucination — a high
//!      faithfulness score means the generated answer is supported by what
//!      the retriever returned.
//!
//!   3. **Answer relevance** (when both `gold_answer` and `generated_answer`
//!      are supplied): token-Jaccard + ROUGE-1 between the two strings.
//!      Detects whether the LLM's answer actually addressed the question.
//!
//! The `mmRAG` dataset adapter (`parse_mmrag_dataset`) loads the standard
//! JSON format the ISWC 2025 paper releases, where each record carries
//! `{question, gold_iris, gold_answer, retrieved_iris, generated_answer}`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RagQa {
    pub question_id: String,
    /// Gold-standard answer IRI.
    pub gold_iri: String,
    /// Retriever's ranked list of candidate IRIs (most-relevant first).
    pub retrieved: Vec<String>,
    /// Optional natural-language answer the LLM generated. Required for
    /// faithfulness + answer-relevance scoring.
    #[serde(default)]
    pub generated_answer: Option<String>,
    /// Optional natural-language gold answer. Required for answer-relevance.
    #[serde(default)]
    pub gold_answer: Option<String>,
    /// Optional concatenated text of the retrieved IRIs' labels/comments,
    /// used for faithfulness scoring. When `None`, faithfulness is `None`
    /// in the per-question report.
    #[serde(default)]
    pub retrieved_text: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RagEvalReport {
    pub total: usize,
    // ── IR metrics ─────────────────────────────────────────────────────
    pub exact_match_at_1: f64,
    pub hit_at_3: f64,
    pub hit_at_5: f64,
    pub hit_at_10: f64,
    pub mrr: f64,
    /// Per-question rank of the gold IRI; `0` means "not retrieved within
    /// the supplied top-k".
    pub per_question_rank: Vec<usize>,
    // ── Answer-quality metrics (mean over questions that supplied the
    //    relevant fields; `None` when no question was scoreable) ────────
    pub mean_faithfulness: Option<f64>,
    pub mean_answer_jaccard: Option<f64>,
    pub mean_answer_rouge1: Option<f64>,
    /// Per-question detail; aligns with `per_question_rank` by index.
    pub per_question_scores: Vec<PerQuestionScores>,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct PerQuestionScores {
    pub question_id: String,
    pub rank: usize,
    /// Fraction of generated-answer tokens that appear in retrieved_text.
    pub faithfulness: Option<f64>,
    /// Jaccard similarity between gold-answer and generated-answer tokens.
    pub answer_jaccard: Option<f64>,
    /// ROUGE-1 F1 between gold-answer and generated-answer.
    pub answer_rouge1: Option<f64>,
}

/// Score a batch of QA results across all three layers (IR metrics +
/// faithfulness + answer-relevance).
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
            mean_faithfulness: None,
            mean_answer_jaccard: None,
            mean_answer_rouge1: None,
            per_question_scores: Vec::new(),
        };
    }
    let n = qas.len() as f64;
    let mut em1 = 0usize;
    let mut h3 = 0usize;
    let mut h5 = 0usize;
    let mut h10 = 0usize;
    let mut mrr_sum = 0.0_f64;
    let mut ranks: Vec<usize> = Vec::with_capacity(qas.len());
    let mut per_q: Vec<PerQuestionScores> = Vec::with_capacity(qas.len());

    let mut faith_sum = 0.0_f64;
    let mut faith_count = 0usize;
    let mut jacc_sum = 0.0_f64;
    let mut jacc_count = 0usize;
    let mut rouge_sum = 0.0_f64;
    let mut rouge_count = 0usize;

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

        let mut scores = PerQuestionScores {
            question_id: qa.question_id.clone(),
            rank,
            ..Default::default()
        };

        if let (Some(generated), Some(ret_txt)) = (&qa.generated_answer, &qa.retrieved_text) {
            let f = faithfulness(generated, ret_txt);
            scores.faithfulness = Some(f);
            faith_sum += f;
            faith_count += 1;
        }
        if let (Some(gold), Some(generated)) = (&qa.gold_answer, &qa.generated_answer) {
            let j = token_jaccard(gold, generated);
            let r = rouge_1_f1(gold, generated);
            scores.answer_jaccard = Some(j);
            scores.answer_rouge1 = Some(r);
            jacc_sum += j;
            jacc_count += 1;
            rouge_sum += r;
            rouge_count += 1;
        }

        per_q.push(scores);
    }

    let mean_or_none = |sum: f64, count: usize| -> Option<f64> {
        if count == 0 {
            None
        } else {
            Some(sum / count as f64)
        }
    };

    RagEvalReport {
        total: qas.len(),
        exact_match_at_1: em1 as f64 / n,
        hit_at_3: h3 as f64 / n,
        hit_at_5: h5 as f64 / n,
        hit_at_10: h10 as f64 / n,
        mrr: mrr_sum / n,
        per_question_rank: ranks,
        mean_faithfulness: mean_or_none(faith_sum, faith_count),
        mean_answer_jaccard: mean_or_none(jacc_sum, jacc_count),
        mean_answer_rouge1: mean_or_none(rouge_sum, rouge_count),
        per_question_scores: per_q,
    }
}

// ─── Text-similarity helpers ────────────────────────────────────────────────

/// Tokenise a string into lowercase alphanumeric tokens. Punctuation is
/// dropped; multi-char tokens preserved. Stopwords kept (mmRAG paper notes
/// that aggressive stopword removal hurts on short answers).
fn tokenise(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() {
            current.push(c.to_ascii_lowercase());
        } else if !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// Faithfulness: fraction of generated-answer tokens that also appear in
/// the retrieved-text. Reports `0.0` when generated is empty.
pub fn faithfulness(generated: &str, retrieved_text: &str) -> f64 {
    let gen_tokens: Vec<String> = tokenise(generated);
    if gen_tokens.is_empty() {
        return 0.0;
    }
    let ret: BTreeSet<String> = tokenise(retrieved_text).into_iter().collect();
    let supported = gen_tokens.iter().filter(|t| ret.contains(*t)).count();
    supported as f64 / gen_tokens.len() as f64
}

/// Token Jaccard between two strings: `|A ∩ B| / |A ∪ B|`.
pub fn token_jaccard(a: &str, b: &str) -> f64 {
    let sa: BTreeSet<String> = tokenise(a).into_iter().collect();
    let sb: BTreeSet<String> = tokenise(b).into_iter().collect();
    if sa.is_empty() && sb.is_empty() {
        return 1.0;
    }
    let inter = sa.intersection(&sb).count();
    let union = sa.union(&sb).count();
    if union == 0 {
        0.0
    } else {
        inter as f64 / union as f64
    }
}

/// ROUGE-1 F1: harmonic mean of token recall (fraction of gold tokens
/// appearing in candidate) and token precision (fraction of candidate
/// tokens appearing in gold). Multiset-aware: a candidate token counts
/// only once per matching gold token, per ROUGE convention.
pub fn rouge_1_f1(gold: &str, candidate: &str) -> f64 {
    let mut gold_tokens = tokenise(gold);
    let mut cand_tokens = tokenise(candidate);
    if gold_tokens.is_empty() && cand_tokens.is_empty() {
        return 1.0;
    }
    if gold_tokens.is_empty() || cand_tokens.is_empty() {
        return 0.0;
    }
    // Multiset intersection.
    gold_tokens.sort();
    cand_tokens.sort();
    let mut i = 0;
    let mut j = 0;
    let mut overlap = 0;
    while i < gold_tokens.len() && j < cand_tokens.len() {
        match gold_tokens[i].cmp(&cand_tokens[j]) {
            std::cmp::Ordering::Equal => {
                overlap += 1;
                i += 1;
                j += 1;
            }
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
        }
    }
    let recall = overlap as f64 / gold_tokens.len() as f64;
    let precision = overlap as f64 / cand_tokens.len() as f64;
    if recall + precision == 0.0 {
        0.0
    } else {
        2.0 * recall * precision / (recall + precision)
    }
}

// ─── mmRAG dataset adapter ──────────────────────────────────────────────────

/// One record in an mmRAG dataset file. Maps to the publication's JSON
/// schema (loose: extra fields ignored, missing optional fields tolerated).
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MmRagRecord {
    pub question_id: String,
    pub question: String,
    /// Gold IRI for retrieval scoring (the first one is treated as the
    /// canonical gold for ranking-style metrics).
    pub gold_iris: Vec<String>,
    pub gold_answer: Option<String>,
    pub retrieved_iris: Vec<String>,
    pub generated_answer: Option<String>,
    pub retrieved_text: Option<String>,
}

/// Parse a JSON-array mmRAG dataset and convert each record into a
/// `RagQa` suitable for `evaluate`.
pub fn parse_mmrag_dataset(json: &str) -> anyhow::Result<Vec<RagQa>> {
    let records: Vec<MmRagRecord> = serde_json::from_str(json)?;
    let mut out: Vec<RagQa> = Vec::with_capacity(records.len());
    for r in records {
        let gold = r
            .gold_iris
            .first()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("mmRAG record `{}` has empty gold_iris", r.question_id))?;
        out.push(RagQa {
            question_id: r.question_id,
            gold_iri: gold,
            retrieved: r.retrieved_iris,
            generated_answer: r.generated_answer,
            gold_answer: r.gold_answer,
            retrieved_text: r.retrieved_text,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qa(id: &str, gold: &str, retrieved: &[&str]) -> RagQa {
        RagQa {
            question_id: id.to_string(),
            gold_iri: gold.to_string(),
            retrieved: retrieved.iter().map(|s| s.to_string()).collect(),
            generated_answer: None,
            gold_answer: None,
            retrieved_text: None,
        }
    }

    fn qa_full(
        id: &str,
        gold: &str,
        retrieved: &[&str],
        gold_ans: &str,
        gen_ans: &str,
        ret_txt: &str,
    ) -> RagQa {
        RagQa {
            question_id: id.to_string(),
            gold_iri: gold.to_string(),
            retrieved: retrieved.iter().map(|s| s.to_string()).collect(),
            generated_answer: Some(gen_ans.to_string()),
            gold_answer: Some(gold_ans.to_string()),
            retrieved_text: Some(ret_txt.to_string()),
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
        assert!(r.mean_faithfulness.is_none());
    }

    #[test]
    fn tokenise_lowercases_and_strips_punctuation() {
        let t = tokenise("The Cat sat on a Mat. (Yes!)");
        assert_eq!(t, vec!["the", "cat", "sat", "on", "a", "mat", "yes"]);
    }

    #[test]
    fn faithfulness_full_when_every_token_supported() {
        let f = faithfulness("the cat sat on the mat", "The cat sat on the mat in the room.");
        assert!((f - 1.0).abs() < 1e-9, "got {}", f);
    }

    #[test]
    fn faithfulness_partial_when_some_tokens_missing() {
        // Generated says "cat dragon mat"; retrieved has "cat" and "mat".
        // 2/3 generated tokens supported = 0.667.
        let f = faithfulness("cat dragon mat", "the cat sat on the mat");
        assert!((f - 2.0 / 3.0).abs() < 1e-9, "got {}", f);
    }

    #[test]
    fn faithfulness_zero_when_generated_empty() {
        assert_eq!(faithfulness("", "anything"), 0.0);
    }

    #[test]
    fn token_jaccard_identical_strings_score_1() {
        assert_eq!(token_jaccard("hello world", "hello world"), 1.0);
    }

    #[test]
    fn token_jaccard_disjoint_strings_score_0() {
        assert_eq!(token_jaccard("alpha beta", "gamma delta"), 0.0);
    }

    #[test]
    fn token_jaccard_partial_overlap() {
        // {a, b, c} vs {b, c, d} → 2/4 = 0.5.
        let j = token_jaccard("a b c", "b c d");
        assert!((j - 0.5).abs() < 1e-9);
    }

    #[test]
    fn rouge_1_f1_perfect_match() {
        assert!((rouge_1_f1("the cat sat", "the cat sat") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn rouge_1_f1_partial_with_repeated_tokens() {
        // Gold: "the cat the dog" (4 tokens). Candidate: "the cat" (2 tokens).
        // Multiset overlap = 2 ("the","cat" each appear once in min of both).
        // recall = 2/4 = 0.5; precision = 2/2 = 1.0; F1 = 2*0.5*1/(1.5) ≈ 0.667.
        let r = rouge_1_f1("the cat the dog", "the cat");
        assert!((r - 2.0 / 3.0).abs() < 1e-9, "got {}", r);
    }

    #[test]
    fn rouge_1_f1_no_overlap_scores_0() {
        assert_eq!(rouge_1_f1("alpha", "beta"), 0.0);
    }

    #[test]
    fn evaluate_computes_faithfulness_and_relevance_when_supplied() {
        let qas = vec![qa_full(
            "q1",
            "http://gold/A",
            &["http://gold/A", "http://other/B"],
            "Cats are mammals that purr",
            "Cats are mammals",
            "Cats are mammals and purr loudly when content.",
        )];
        let r = evaluate(&qas);
        assert!(r.mean_faithfulness.unwrap() > 0.9,
            "all 3 generated tokens are supported; got {:?}",
            r.mean_faithfulness);
        // gold tokens: {cats, are, mammals, that, purr}; generated: {cats, are, mammals}
        // jaccard = 3/5 = 0.6
        assert!((r.mean_answer_jaccard.unwrap() - 0.6).abs() < 1e-6,
            "got {:?}", r.mean_answer_jaccard);
        let pq = &r.per_question_scores[0];
        assert_eq!(pq.question_id, "q1");
        assert_eq!(pq.rank, 1);
        assert!(pq.faithfulness.is_some());
        assert!(pq.answer_jaccard.is_some());
        assert!(pq.answer_rouge1.is_some());
    }

    #[test]
    fn evaluate_leaves_answer_metrics_none_when_fields_absent() {
        let qas = vec![qa("q1", "http://gold/A", &["http://gold/A"])];
        let r = evaluate(&qas);
        // IR metrics still populated.
        assert_eq!(r.exact_match_at_1, 1.0);
        // No generated_answer / gold_answer / retrieved_text → None.
        assert!(r.mean_faithfulness.is_none());
        assert!(r.mean_answer_jaccard.is_none());
        assert!(r.mean_answer_rouge1.is_none());
    }

    #[test]
    fn evaluate_mixes_questions_with_and_without_answer_fields() {
        let qas = vec![
            qa_full("q1", "A", &["A"], "a b", "a b", "a b c"),
            qa("q2", "B", &["X", "B"]),
        ];
        let r = evaluate(&qas);
        // Faithfulness mean comes from only q1.
        assert!((r.mean_faithfulness.unwrap() - 1.0).abs() < 1e-6);
        // IR metrics span both: q1 rank=1, q2 rank=2.
        assert!(r.per_question_rank == vec![1, 2]);
    }

    #[test]
    fn parse_mmrag_dataset_loads_records_and_picks_first_gold_iri() {
        let dataset = r#"[
            {
                "question_id": "mm1",
                "question": "What is a cat?",
                "gold_iris": ["http://ex.org/Cat", "http://ex.org/Feline"],
                "gold_answer": "A small domesticated felid.",
                "retrieved_iris": ["http://ex.org/Cat", "http://ex.org/Tiger"],
                "generated_answer": "A small felid that is domesticated.",
                "retrieved_text": "The cat is a small felid commonly kept as a pet."
            }
        ]"#;
        let qas = parse_mmrag_dataset(dataset).unwrap();
        assert_eq!(qas.len(), 1);
        assert_eq!(qas[0].gold_iri, "http://ex.org/Cat");
        assert!(qas[0].generated_answer.is_some());
        assert!(qas[0].retrieved_text.is_some());
    }

    #[test]
    fn parse_mmrag_dataset_rejects_record_with_empty_gold_iris() {
        let dataset = r#"[
            {"question_id": "bad", "question": "?", "gold_iris": [],
             "retrieved_iris": []}
        ]"#;
        let err = parse_mmrag_dataset(dataset).expect_err("should reject");
        assert!(format!("{}", err).contains("empty gold_iris"));
    }

    #[test]
    fn parse_mmrag_then_evaluate_end_to_end() {
        let dataset = r#"[
            {
                "question_id": "mm1",
                "question": "?",
                "gold_iris": ["http://gold/X"],
                "gold_answer": "alpha beta gamma",
                "retrieved_iris": ["http://gold/X", "http://other/Y"],
                "generated_answer": "alpha beta",
                "retrieved_text": "alpha beta gamma delta"
            },
            {
                "question_id": "mm2",
                "question": "?",
                "gold_iris": ["http://gold/Z"],
                "gold_answer": "one two three",
                "retrieved_iris": ["http://other/W"],
                "generated_answer": "one two",
                "retrieved_text": "completely different"
            }
        ]"#;
        let qas = parse_mmrag_dataset(dataset).unwrap();
        let r = evaluate(&qas);
        assert_eq!(r.total, 2);
        // mm1: rank 1, mm2: rank 0.
        assert_eq!(r.per_question_rank, vec![1, 0]);
        assert!((r.exact_match_at_1 - 0.5).abs() < 1e-9);
        // mm2 faithfulness is 0 (tokens "one","two" not in "completely different").
        // mm1 faithfulness is 1.0.
        // Mean = 0.5.
        assert!((r.mean_faithfulness.unwrap() - 0.5).abs() < 1e-6);
    }
}
