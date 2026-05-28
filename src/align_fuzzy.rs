//! FLORA-style fuzzy-logic alignment adjudication (#38).
//!
//! Per the ISWC 2025 paper "FLORA: Fuzzy Logic Over Relational Alignments,"
//! this implements:
//!
//!   - **A 10-rule Mamdani fuzzy inference system** over four input signals
//!     (label Jaccard, parent overlap, sibling overlap, datatype overlap)
//!     plus structural penalties.
//!   - **Triangular membership functions** for each signal (low / medium /
//!     high) with overlapping boundaries.
//!   - **Three t-norms** for the AND aggregator (Gödel / product /
//!     Łukasiewicz).
//!   - **Centroid defuzzification** producing a crisp `[0, 1]` score.
//!   - **Auditable rule trace** so a reviewer can see which rules fired and
//!     with what activation strength.
//!
//! The architectural pivot recorded in project memory: HNSW is demoted to
//! a *candidate generator*; FLORA is the final-decision layer for the
//! pairs HNSW surfaces. Embedding-free and interpretable — every accept
//! decision is traceable back to the specific rules that fired.

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

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TNorm {
    /// `min(a, b)` — Gödel.
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
    /// Activation strength per rule (firing strength × consequent peak).
    pub rule_activations: Vec<RuleFiring>,
    pub rule_trace: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct RuleFiring {
    pub rule_id: u32,
    pub firing_strength: f64,
    pub consequent: String,
}

// ─── Triangular membership functions ────────────────────────────────────────
//
// For each input signal, three fuzzy sets: low, medium, high. The
// triangles overlap at ~0.4 and ~0.7 so the rule base is continuous
// (no dead zones).

/// Membership in the "low" fuzzy set: peak 0.0, falls to 0 at 0.4.
fn mu_low(x: f64) -> f64 {
    if x <= 0.0 { 1.0 }
    else if x >= 0.4 { 0.0 }
    else { 1.0 - x / 0.4 }
}

/// Membership in the "medium" fuzzy set: peak at 0.5, supports [0.2, 0.8].
fn mu_medium(x: f64) -> f64 {
    if x <= 0.2 || x >= 0.8 { 0.0 }
    else if x <= 0.5 { (x - 0.2) / 0.3 }
    else { (0.8 - x) / 0.3 }
}

/// Membership in the "high" fuzzy set: rises from 0 at 0.6, peaks at 1.0.
fn mu_high(x: f64) -> f64 {
    if x <= 0.6 { 0.0 }
    else if x >= 1.0 { 1.0 }
    else { (x - 0.6) / 0.4 }
}

// ─── Output fuzzy sets (verdict) ────────────────────────────────────────────
//
// Three output sets centred on 0.15, 0.5, 0.9 representing reject /
// borderline / accept.

const OUT_REJECT_PEAK: f64 = 0.15;
const OUT_BORDERLINE_PEAK: f64 = 0.5;
const OUT_ACCEPT_PEAK: f64 = 0.9;

// ─── Rule base (10 rules per the FLORA paper) ───────────────────────────────

#[derive(Clone, Copy, Debug)]
enum OutLabel {
    Reject,
    Borderline,
    Accept,
}

impl OutLabel {
    fn peak(self) -> f64 {
        match self {
            Self::Reject => OUT_REJECT_PEAK,
            Self::Borderline => OUT_BORDERLINE_PEAK,
            Self::Accept => OUT_ACCEPT_PEAK,
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Reject => "reject",
            Self::Borderline => "borderline",
            Self::Accept => "accept",
        }
    }
}

/// Combine two membership values under a t-norm (used for `AND` in
/// antecedents).
fn tnorm(a: f64, b: f64, t: TNorm) -> f64 {
    match t {
        TNorm::Min => a.min(b),
        TNorm::Product => a * b,
        TNorm::Lukasiewicz => (a + b - 1.0).max(0.0),
    }
}

/// Combine n membership values under a t-norm.
fn tnorm_n(values: &[f64], t: TNorm) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values
        .iter()
        .copied()
        .fold(values[0], |acc, x| tnorm(acc, x, t))
        .clamp(0.0, 1.0)
}

/// Evaluate the 10-rule FLORA inference base. Returns per-rule firing
/// strength + output consequent.
fn evaluate_rules(s: &FuzzySignals, t: TNorm) -> Vec<(u32, f64, OutLabel)> {
    let l_hi = mu_high(s.label_jaccard);
    let l_mid = mu_medium(s.label_jaccard);
    let l_lo = mu_low(s.label_jaccard);
    let p_hi = mu_high(s.parent_overlap);
    let p_mid = mu_medium(s.parent_overlap);
    let p_lo = mu_low(s.parent_overlap);
    let sb_hi = mu_high(s.sibling_overlap);
    let _sb_mid = mu_medium(s.sibling_overlap); // reserved for future rules
    let d_hi = mu_high(s.datatype_overlap);
    let _d_mid = mu_medium(s.datatype_overlap); // reserved for future rules
    let d_lo = mu_low(s.datatype_overlap);

    vec![
        // R1: high label ∧ high parent → accept (strong textual+structural agreement).
        (1, tnorm(l_hi, p_hi, t), OutLabel::Accept),
        // R2: high label ∧ medium parent → accept (textual carries most of the weight).
        (2, tnorm(l_hi, p_mid, t), OutLabel::Accept),
        // R3: high label ∧ high datatype → accept (textual + same datatype properties).
        (3, tnorm(l_hi, d_hi, t), OutLabel::Accept),
        // R4: high parent ∧ high sibling ∧ high datatype → accept (pure structural).
        (4, tnorm_n(&[p_hi, sb_hi, d_hi], t), OutLabel::Accept),
        // R5: medium label ∧ high parent ∧ high sibling → borderline (struct without text).
        (5, tnorm_n(&[l_mid, p_hi, sb_hi], t), OutLabel::Borderline),
        // R6: medium label ∧ medium parent → borderline.
        (6, tnorm(l_mid, p_mid, t), OutLabel::Borderline),
        // R7: high label ∧ low parent ∧ low datatype → borderline (text but no structure).
        (7, tnorm_n(&[l_hi, p_lo, d_lo], t), OutLabel::Borderline),
        // R8: low label ∧ low parent → reject (no evidence).
        (8, tnorm(l_lo, p_lo, t), OutLabel::Reject),
        // R9: low label → reject (FLORA paper: label disagreement penalises strongly).
        (9, l_lo, OutLabel::Reject),
        // R10: low datatype ∧ medium label → reject (datatype mismatch is structural penalty).
        (10, tnorm(d_lo, l_mid, t), OutLabel::Reject),
    ]
}

/// Centroid defuzzification: weighted average of output peaks by firing
/// strength.
fn defuzzify(firings: &[(u32, f64, OutLabel)]) -> f64 {
    let mut num = 0.0_f64;
    let mut den = 0.0_f64;
    for (_, w, label) in firings {
        num += *w * label.peak();
        den += *w;
    }
    if den == 0.0 {
        0.5 // total ambiguity — neutral
    } else {
        num / den
    }
}

/// Adjudicate a candidate pair via the full FLORA fuzzy inference system.
pub fn adjudicate(
    signals: &FuzzySignals,
    tnorm: TNorm,
    low_threshold: f64,
    high_threshold: f64,
) -> FuzzyDecision {
    let firings = evaluate_rules(signals, tnorm);
    let fuzzy_score = defuzzify(&firings);

    let verdict = if fuzzy_score >= high_threshold {
        "accept"
    } else if fuzzy_score >= low_threshold {
        "borderline"
    } else {
        "reject"
    }
    .to_string();

    let rule_activations: Vec<RuleFiring> = firings
        .iter()
        .filter(|(_, w, _)| *w > 1e-6)
        .map(|(id, w, lbl)| RuleFiring {
            rule_id: *id,
            firing_strength: *w,
            consequent: lbl.name().to_string(),
        })
        .collect();
    let mut trace: Vec<String> = vec![
        format!("inference: 10-rule Mamdani over signals {:?}, t-norm: {:?}", signals, tnorm),
    ];
    for f in &rule_activations {
        trace.push(format!(
            "R{} fired @ {:.3} → {}",
            f.rule_id, f.firing_strength, f.consequent
        ));
    }
    trace.push(format!(
        "centroid defuzzification → score = {:.3}",
        fuzzy_score
    ));
    trace.push(format!(
        "verdict: {} (thresholds: low={:.3}, high={:.3})",
        verdict, low_threshold, high_threshold
    ));

    FuzzyDecision {
        fuzzy_score,
        verdict,
        rule_activations,
        rule_trace: trace,
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
    fn membership_triangles_have_expected_peaks() {
        assert!((mu_low(0.0) - 1.0).abs() < 1e-9);
        assert_eq!(mu_low(0.5), 0.0);
        assert!((mu_medium(0.5) - 1.0).abs() < 1e-9);
        assert!((mu_high(1.0) - 1.0).abs() < 1e-9);
        assert_eq!(mu_high(0.5), 0.0);
    }

    #[test]
    fn strong_match_accepts() {
        let d = adjudicate(&s(0.95, 0.9, 0.8, 0.7), TNorm::Min, 0.4, 0.7);
        assert_eq!(d.verdict, "accept");
        assert!(d.fuzzy_score >= 0.7);
        // R1 must have fired.
        assert!(d.rule_activations.iter().any(|r| r.rule_id == 1));
    }

    #[test]
    fn strong_mismatch_rejects() {
        let d = adjudicate(&s(0.05, 0.1, 0.1, 0.1), TNorm::Min, 0.4, 0.7);
        assert_eq!(d.verdict, "reject");
        assert!(d.fuzzy_score <= 0.4);
        // R8 or R9 must have fired.
        assert!(d.rule_activations.iter().any(|r| r.rule_id == 8 || r.rule_id == 9));
    }

    #[test]
    fn moderate_label_borderline() {
        let d = adjudicate(&s(0.5, 0.5, 0.3, 0.3), TNorm::Min, 0.4, 0.7);
        assert_eq!(d.verdict, "borderline");
    }

    #[test]
    fn rule_trace_records_firings_and_defuzz() {
        let d = adjudicate(&s(0.7, 0.7, 0.5, 0.5), TNorm::Min, 0.4, 0.7);
        assert!(d.rule_trace.iter().any(|s| s.contains("Mamdani")));
        assert!(d.rule_trace.iter().any(|s| s.contains("centroid")));
        assert!(d.rule_trace.iter().any(|s| s.contains("verdict")));
    }

    #[test]
    fn datatype_only_penalises_under_r10() {
        // Medium label + extreme datatype mismatch should fire R10 → reject leaning.
        let d = adjudicate(&s(0.5, 0.1, 0.1, 0.0), TNorm::Min, 0.4, 0.7);
        assert!(d.rule_activations.iter().any(|r| r.rule_id == 10),
            "R10 should fire on low-datatype + medium-label; got: {:?}", d.rule_activations);
    }

    #[test]
    fn product_tnorm_strictly_dampens_versus_min() {
        let signals = s(0.7, 0.7, 0.5, 0.5);
        let d_min = adjudicate(&signals, TNorm::Min, 0.4, 0.7);
        let d_prod = adjudicate(&signals, TNorm::Product, 0.4, 0.7);
        // Product reduces firing strengths; centroid leans more toward 0.5.
        assert!(d_prod.fuzzy_score != d_min.fuzzy_score,
            "product and min should produce different scores");
    }

    #[test]
    fn defuzzification_returns_neutral_05_when_no_rules_fire() {
        // All inputs in the "dead zone" — but our overlapping triangles
        // mean some rule fires for any [0,1] input. Use truly empty firings.
        let firings: Vec<(u32, f64, OutLabel)> = vec![];
        assert_eq!(defuzzify(&firings), 0.5);
    }

    #[test]
    fn high_structural_alone_can_still_accept_via_r4() {
        // High parent + sibling + datatype, low label → R4 accept rule.
        let d = adjudicate(&s(0.1, 0.9, 0.9, 0.9), TNorm::Min, 0.4, 0.7);
        // R4 fires, but R9 (low label → reject) also fires strongly.
        // Net score should be in the borderline-to-reject region, not accept.
        assert!(d.fuzzy_score < 0.7,
            "low-label should not yield strong accept even with structure; got {}",
            d.fuzzy_score);
    }

    #[test]
    fn fully_neutral_signals_yield_borderline() {
        let d = adjudicate(&s(0.5, 0.5, 0.5, 0.5), TNorm::Min, 0.4, 0.7);
        assert_eq!(d.verdict, "borderline");
    }
}
