//! OAEI-style alignment evaluation harness (#31).
//!
//! Given a reference alignment (gold standard) and a computed alignment
//! (e.g. from `onto_align`), compute precision / recall / F1 against the
//! reference. Mirrors the [OAEI](http://oaei.ontologymatching.org/) scoring
//! convention: alignments are sets of `(source_iri, target_iri, relation)`
//! triples; an entry matches the reference iff all three components match.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct AlignmentEntry {
    pub source: String,
    pub target: String,
    /// One of `"equivalent"`, `"subsumes"`, `"subsumed_by"`. Defaults to
    /// `"equivalent"` on deserialise if missing.
    #[serde(default = "default_relation")]
    pub relation: String,
}

fn default_relation() -> String {
    "equivalent".to_string()
}

#[derive(Clone, Debug, Serialize)]
pub struct EvaluationReport {
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    pub true_positive: usize,
    pub false_positive: usize,
    pub false_negative: usize,
    pub reference_size: usize,
    pub computed_size: usize,
}

/// Evaluate `computed` against `reference`. Entries are matched on
/// `(source, target, relation)` exact equality.
pub fn evaluate(reference: &[AlignmentEntry], computed: &[AlignmentEntry]) -> EvaluationReport {
    let ref_set: BTreeSet<&AlignmentEntry> = reference.iter().collect();
    let comp_set: BTreeSet<&AlignmentEntry> = computed.iter().collect();

    let tp = ref_set.intersection(&comp_set).count();
    let fp = comp_set.len() - tp;
    let fn_ = ref_set.len() - tp;

    let precision = if computed.is_empty() {
        0.0
    } else {
        tp as f64 / computed.len() as f64
    };
    let recall = if reference.is_empty() {
        0.0
    } else {
        tp as f64 / reference.len() as f64
    };
    let f1 = if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    };

    EvaluationReport {
        precision,
        recall,
        f1,
        true_positive: tp,
        false_positive: fp,
        false_negative: fn_,
        reference_size: reference.len(),
        computed_size: computed.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(s: &str, t: &str) -> AlignmentEntry {
        AlignmentEntry {
            source: s.to_string(),
            target: t.to_string(),
            relation: "equivalent".to_string(),
        }
    }

    #[test]
    fn perfect_match_gives_f1_1() {
        let r = vec![e("A", "X"), e("B", "Y")];
        let c = vec![e("A", "X"), e("B", "Y")];
        let report = evaluate(&r, &c);
        assert_eq!(report.precision, 1.0);
        assert_eq!(report.recall, 1.0);
        assert_eq!(report.f1, 1.0);
        assert_eq!(report.true_positive, 2);
    }

    #[test]
    fn complete_miss_gives_f1_0() {
        let r = vec![e("A", "X")];
        let c = vec![e("B", "Y")];
        let report = evaluate(&r, &c);
        assert_eq!(report.precision, 0.0);
        assert_eq!(report.recall, 0.0);
        assert_eq!(report.f1, 0.0);
        assert_eq!(report.false_positive, 1);
        assert_eq!(report.false_negative, 1);
    }

    #[test]
    fn partial_overlap_gives_intermediate_f1() {
        let r = vec![e("A", "X"), e("B", "Y"), e("C", "Z")];
        let c = vec![e("A", "X"), e("D", "W")];
        let report = evaluate(&r, &c);
        // P = 1/2 = 0.5, R = 1/3 ≈ 0.333, F1 = 2*0.5*0.333/(0.5+0.333) = 0.4
        assert!((report.precision - 0.5).abs() < 1e-9);
        assert!((report.recall - 1.0/3.0).abs() < 1e-9);
        assert!((report.f1 - 0.4).abs() < 1e-3);
    }

    #[test]
    fn empty_computed_gives_zero_precision_zero_recall() {
        let r = vec![e("A", "X")];
        let c: Vec<AlignmentEntry> = vec![];
        let report = evaluate(&r, &c);
        assert_eq!(report.precision, 0.0);
        assert_eq!(report.recall, 0.0);
        assert_eq!(report.false_negative, 1);
    }

    #[test]
    fn relation_mismatch_counts_as_miss() {
        let r = vec![e("A", "X")];
        let mut c = vec![e("A", "X")];
        c[0].relation = "subsumes".to_string();
        let report = evaluate(&r, &c);
        assert_eq!(report.precision, 0.0);
        assert_eq!(report.recall, 0.0);
        assert_eq!(report.true_positive, 0);
    }
}
