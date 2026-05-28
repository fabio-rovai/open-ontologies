//! FLORA-style fuzzy-logic alignment adjudication (#38, ISWC 2025 Best
//! Paper).
//!
//! FLORA (Fuzzy Logic Over Relational Alignments) is embedding-free: it
//! adjudicates candidate alignment pairs using fuzzy-logic rules over
//! structural signals (label-token-Jaccard, parent overlap, sibling
//! overlap, datatype-property overlap), then aggregates via a configurable
//! t-norm.
//!
//! ## Per the strategic pivot recorded in project memory
//!
//! HNSW (currently the alignment engine's main matcher) is **demoted to a
//! candidate generator**. FLORA-style fuzzy adjudication becomes the
//! final-decision layer for borderline pairs. The ISWC 2025 Best Paper
//! result: embedding-free fuzzy alignment matches or beats embedding-based
//! pipelines on OAEI benchmarks while being interpretable.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct FuzzySignals {
    /// Token Jaccard over labels in `[0, 1]`.
    pub label_jaccard: f64,
    /// Fraction of parents shared in `[0, 1]`.
    pub parent_overlap: f64,
    /// Fraction of siblings shared in `[0, 1]`.
    pub sibling_overlap: f64,
    /// Fraction of datatype properties shared in `[0, 1]`.
    pub datatype_overlap: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TNorm {
    /// `min(a, b)` — Zadeh / Gödel.
    #[default]
    Min,
    /// `a * b` — product / probabilistic.
    Product,
    /// `max(0, a + b - 1)` — Łukasiewicz.
    Lukasiewicz,
}

#[derive(Clone, Debug, Serialize)]
pub struct FuzzyDecision {
    pub fuzzy_score: f64,
    pub verdict: String,
    pub rule_trace: Vec<String>,
}

/// Adjudicate a candidate pair using fuzzy logic over the supplied signals.
/// Returns a score in `[0, 1]` and one of `"accept"` / `"reject"` /
/// `"borderline"` based on thresholds.
pub fn adjudicate(
    signals: &FuzzySignals,
    tnorm: TNorm,
    low_threshold: f64,
    high_threshold: f64,
) -> FuzzyDecision {
    let mut trace: Vec<String> = Vec::new();

    // Rule 1: high label similarity AND high parent overlap → strong accept.
    let rule1 = combine(signals.label_jaccard, signals.parent_overlap, tnorm);
    trace.push(format!(
        "R1 (label ∧ parent, {:?}): label={:.3} parent={:.3} = {:.3}",
        tnorm, signals.label_jaccard, signals.parent_overlap, rule1
    ));

    // Rule 2: structural support (parent + sibling + datatype) for borderline labels.
    let mid = (signals.parent_overlap + signals.sibling_overlap + signals.datatype_overlap) / 3.0;
    let rule2 = combine(signals.label_jaccard, mid, tnorm);
    trace.push(format!(
        "R2 (label ∧ mean-structural, {:?}): label={:.3} mean-struct={:.3} = {:.3}",
        tnorm, signals.label_jaccard, mid, rule2
    ));

    // Final score: maximum activation (Mamdani-style aggregation).
    let fuzzy_score = rule1.max(rule2);
    trace.push(format!("aggregate (max of rules): {:.3}", fuzzy_score));

    let verdict = if fuzzy_score >= high_threshold {
        "accept"
    } else if fuzzy_score >= low_threshold {
        "borderline"
    } else {
        "reject"
    }
    .to_string();
    trace.push(format!(
        "verdict: {} (thresholds: low={:.3}, high={:.3})",
        verdict, low_threshold, high_threshold
    ));

    FuzzyDecision { fuzzy_score, verdict, rule_trace: trace }
}

fn combine(a: f64, b: f64, t: TNorm) -> f64 {
    match t {
        TNorm::Min => a.min(b),
        TNorm::Product => a * b,
        TNorm::Lukasiewicz => (a + b - 1.0).max(0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(l: f64, p: f64, sb: f64, d: f64) -> FuzzySignals {
        FuzzySignals {
            label_jaccard: l,
            parent_overlap: p,
            sibling_overlap: sb,
            datatype_overlap: d,
        }
    }

    #[test]
    fn high_label_and_parent_accepts_under_min_tnorm() {
        let d = adjudicate(&s(0.95, 0.9, 0.4, 0.3), TNorm::Min, 0.4, 0.85);
        assert_eq!(d.verdict, "accept");
        assert!(d.fuzzy_score >= 0.9);
    }

    #[test]
    fn moderate_label_borderline() {
        let d = adjudicate(&s(0.6, 0.5, 0.3, 0.2), TNorm::Min, 0.4, 0.85);
        assert_eq!(d.verdict, "borderline");
    }

    #[test]
    fn low_label_rejects_even_with_structural_support() {
        let d = adjudicate(&s(0.1, 0.95, 0.95, 0.95), TNorm::Min, 0.4, 0.85);
        assert_eq!(d.verdict, "reject");
    }

    #[test]
    fn product_tnorm_is_stricter_than_min() {
        let signals = s(0.8, 0.8, 0.4, 0.3);
        let d_min = adjudicate(&signals, TNorm::Min, 0.4, 0.85);
        let d_prod = adjudicate(&signals, TNorm::Product, 0.4, 0.85);
        assert!(d_min.fuzzy_score >= d_prod.fuzzy_score);
    }

    #[test]
    fn lukasiewicz_strictest_of_the_three() {
        let signals = s(0.5, 0.5, 0.5, 0.5);
        let d_min = adjudicate(&signals, TNorm::Min, 0.0, 1.0);
        let d_prod = adjudicate(&signals, TNorm::Product, 0.0, 1.0);
        let d_luka = adjudicate(&signals, TNorm::Lukasiewicz, 0.0, 1.0);
        assert!(d_luka.fuzzy_score <= d_prod.fuzzy_score);
        assert!(d_prod.fuzzy_score <= d_min.fuzzy_score);
    }

    #[test]
    fn rule_trace_records_each_rule_score_and_verdict() {
        let d = adjudicate(&s(0.7, 0.7, 0.7, 0.7), TNorm::Min, 0.4, 0.85);
        assert!(d.rule_trace.iter().any(|s| s.contains("R1")));
        assert!(d.rule_trace.iter().any(|s| s.contains("R2")));
        assert!(d.rule_trace.iter().any(|s| s.contains("verdict")));
    }
}
