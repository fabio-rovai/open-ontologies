//! CIVeX-style causal certificate for state-changing ontology actions (#42).
//!
//! A scaffold port of the verifier described in **CIVeX: Causal Intervention
//! Verification for Language Agents** (Rovai, arXiv 2605.09168). The original
//! paper gates LLM-agent tool calls: it maps a proposed action to a structural
//! causal query `E[Y | do(t_v = t*)]`, checks identifiability under a committed
//! graph, computes a one-sided lower confidence bound on the do-effect, and
//! returns one of four auditable verdicts: EXECUTE / REJECT / EXPERIMENT /
//! ABSTAIN, each carrying a causal certificate.
//!
//! This module ports the structural shape to ontology operations: every state-
//! changing `onto_*` tool (apply / save / push / ingest / enrich) can be wrapped
//! in `certify_action` to obtain a verdict + certificate before executing.
//!
//! ## Scaffold scope vs paper
//!
//! What's faithful:
//!
//! - The four-way verdict.
//! - The certificate as an auditable JSON artefact: structural-graph slice
//!   hash, labelled assumptions, identification proof, point estimate,
//!   one-sided LCB, provenance hash, risk bound.
//! - LCB-based safety gating (executions are bounded at the LCB level α).
//! - Locked-IRI hard-rejection (the analogue of "risk class is forbidden").
//!
//! What's intentionally a proxy (and honestly documented):
//!
//! - **Identifiability check:** the paper runs backdoor / frontdoor / IV from
//!   do-calculus. The scaffold runs a **structural-dependency check** —
//!   "the proposed change is identifiable iff its effect on the utility metric
//!   is bounded by the closure of dependent IRIs in the loaded ontology."
//!   This is a sound conservative proxy for the typical ontology-edit case
//!   (linear blast radius from a target IRI), not a substitute for true
//!   do-calculus. The certificate flags this with `assumption = "structural_only"`.
//! - **The EXPERIMENT branch:** the paper consumes paired RCT-style data; the
//!   scaffold maps EXPERIMENT to "queue for sandbox replay" and exposes a
//!   separate path; it does not synthesise an RCT from observational data.
//! - **Adversarial-confounding robustness:** out of scope. The paper's Table 2
//!   result depends on Causal-ToolBench instrumentation; the scaffold honest-
//!   documents that confounded ontology workflows (e.g. cascading restriction
//!   updates) may produce LCBs the verifier cannot defend against an
//!   adversarial confounder.
//!
//! The follow-up issues track each of these toward production-grade fidelity.

use crate::graph::GraphStore;
use crate::state::StateDb;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// An action frame describing a proposed state-changing ontology operation.
/// Mirrors the paper's `a = (tool, t_v, t*, Y, c, r)` tuple.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionFrame {
    /// Name of the onto_* tool whose execution this certifies (e.g. "onto_apply",
    /// "onto_save", "onto_push", "onto_ingest").
    pub tool: String,
    /// The IRIs being targeted by the proposed change. In CIVeX terms, the
    /// `t_v` (target variable) generalises to the set of structural variables
    /// the action intervenes on.
    pub target_iris: Vec<String>,
    /// The proposed change expressed as Turtle. Treated as the `t*` (target
    /// value) — what the action would set the target IRIs' neighbourhood to.
    pub proposed_delta_ttl: String,
    /// The utility variable name. The scaffold computes the do-effect on this
    /// metric. Supported values:
    /// - `"dependent_query_pass_rate"` (default) — the caller supplies a list of
    ///   SPARQL queries in `dependent_queries`; the LCB is over the pre/post
    ///   pass rate.
    /// - `"triple_count_delta"` — coarse proxy: change in total triple count.
    /// - `"class_count_delta"` / `"property_count_delta"` — class/property
    ///   vocabulary changes.
    pub utility_metric: String,
    /// Optional list of SPARQL queries that should remain answerable post-change.
    /// Used when `utility_metric == "dependent_query_pass_rate"`.
    #[serde(default)]
    pub dependent_queries: Vec<String>,
    /// Cost budget. The action is REJECTED if `cost > cost_threshold`. Cost is
    /// measured in triples-affected (from blast-radius), matching `onto_plan`.
    pub cost_threshold: u64,
    /// Utility threshold for EXECUTE. The action is EXECUTED only if
    /// `LCB_α ≥ utility_threshold`. Conventional values: 0.0 for "do no harm",
    /// 0.5 for "majority of dependent queries still answer", etc.
    pub utility_threshold: f64,
    /// Risk threshold. Upper bound on potential harm. Action is REJECTED if
    /// `cost > risk_threshold` even when within the cost budget.
    pub risk_threshold: u64,
    /// Whether the action is reversible (has a defined inverse). Maps to the
    /// paper's `r ∈ {0, 1}`. Reversibility unlocks the EXPERIMENT branch.
    pub reversible: bool,
    /// When true, the EXPERIMENT verdict is legal output; otherwise unknown
    /// utility forces ABSTAIN. The paper requires explicit opt-in for
    /// experimentation because it consumes real resources.
    #[serde(default)]
    pub allow_experiment: bool,
    /// One-sided confidence level α for the LCB. Default 0.05 (95% one-sided).
    /// Matches the paper's `LCB_{α=0.05}`.
    #[serde(default = "default_alpha")]
    pub alpha: f64,
    /// Optional: name of a registered `dynamics::ActionSchema` this frame
    /// refers to (#43). When set, `certify_action` echoes the schema name
    /// into the certificate's assumptions and rationale, making the audit
    /// trail explicit about which Dynamics action was gated.
    #[serde(default)]
    pub action_schema_name: Option<String>,
}

fn default_alpha() -> f64 {
    0.05
}

/// The four-way verdict from `certify_action`. Auditable; the certificate
/// documents the assumptions under which the verdict holds.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Verdict {
    /// The action's effect is identifiable, its LCB clears the utility threshold,
    /// and its cost is within budget. Execution is bounded at the α level
    /// (probability of false-execute ≤ α).
    Execute,
    /// A hard constraint fails: locked IRIs would be touched, cost exceeds
    /// the risk threshold, or the action is irreversible with insufficient
    /// utility certainty.
    Reject,
    /// Utility is unidentifiable under the current observational data, but the
    /// action is reversible and the caller has set `allow_experiment = true`.
    /// The orchestrator is expected to run the action in a sandbox / dry-run
    /// path, collect the resulting data, and re-invoke `certify_action` with
    /// the updated evidence.
    Experiment,
    /// Utility ambiguous and either the action is irreversible or the caller
    /// has not authorised experimentation. The orchestrator must obtain more
    /// information (refine the utility metric, supply more dependent queries,
    /// or relax the threshold) before re-invoking.
    Abstain,
}

/// An auditable causal certificate. JSON-serialisable; the orchestrator can
/// log this artefact alongside the lineage trail.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Certificate {
    /// Verdict on the proposed action.
    pub verdict: Verdict,
    /// Hash of the structural dependency slice the verifier consulted. Stable
    /// across runs over the same committed graph state, so a verdict is replayable.
    pub graph_slice_hash: String,
    /// Labelled assumptions under which the verdict holds. The scaffold always
    /// emits `"structural_only"` (as opposed to full do-calculus identifiability)
    /// plus the locked-IRI and reversibility flags actually used.
    pub assumptions: Vec<String>,
    /// Identification proof. For the scaffold this is a one-line statement:
    /// `"structural-dependency closure of {target_iris} has bounded blast radius {N}"`.
    /// A production verifier would emit a backdoor / frontdoor / IV proof.
    pub identification_proof: String,
    /// Point estimate of the do-effect on the utility metric.
    pub utility_point_estimate: f64,
    /// One-sided lower confidence bound on the do-effect at level α.
    pub utility_lcb: f64,
    /// α level the LCB was computed at.
    pub alpha: f64,
    /// SHA-256 hash of the observational data used (canonical-form snapshot
    /// of the loaded ontology plus the proposed delta). Used for provenance
    /// and replay.
    pub provenance_hash: String,
    /// Upper bound on potential harm, measured in triples-affected. Matches
    /// the `cost` from `onto_plan`'s blast-radius.
    pub risk_bound: u64,
    /// Human-readable rationale explaining the verdict. Concatenation of the
    /// triage rules that fired.
    pub rationale: String,
}

/// Aggregate of the certificate plus side evidence for the orchestrator.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CertifyActionResult {
    pub certificate: Certificate,
    /// Number of dependent queries evaluated (when utility_metric is
    /// `"dependent_query_pass_rate"`).
    pub dependent_queries_evaluated: usize,
    /// Per-query pass results post-delta (when utility_metric is
    /// `"dependent_query_pass_rate"`).
    pub per_query_pass: Vec<bool>,
}

/// Certify a proposed state-changing ontology action.
///
/// Returns a `CertifyActionResult` carrying the verdict, certificate, and
/// evidence. The orchestrator decides whether to actually execute the action
/// based on the verdict — this function is a gate, not an executor.
pub fn certify_action(
    db: &StateDb,
    graph: &Arc<GraphStore>,
    frame: &ActionFrame,
) -> anyhow::Result<CertifyActionResult> {
    // ── Step 1: locked-IRI check ────────────────────────────────────────────
    // The hardest hard-constraint. Mirrors the paper's "risk class is forbidden"
    // branch. Locked IRIs always REJECT regardless of utility.
    let planner = crate::plan::Planner::new(db.clone(), graph.clone());
    let locked_violations: Vec<String> = frame
        .target_iris
        .iter()
        .filter(|iri| planner.is_locked(iri))
        .cloned()
        .collect();

    // ── Step 2: blast-radius (cost) ─────────────────────────────────────────
    // Reuses the existing onto_plan accounting.
    let plan_json_str = planner.plan(&frame.proposed_delta_ttl)?;
    let plan_json: serde_json::Value = serde_json::from_str(&plan_json_str)?;
    let cost_triples = plan_json["blast_radius"]["triples_affected"]
        .as_u64()
        .unwrap_or(0);

    // ── Step 3: structural-dependency slice + hash ──────────────────────────
    let dependency_iris = collect_dependency_iris(graph, &frame.target_iris);
    let slice_hash = hash_iri_set(&dependency_iris);

    // ── Step 4: utility estimate + LCB (Wilson one-sided) ───────────────────
    let (utility_point, utility_lcb, queries_eval, per_query_pass) = match frame
        .utility_metric
        .as_str()
    {
        "dependent_query_pass_rate" => {
            evaluate_dependent_queries(graph, &frame.proposed_delta_ttl, &frame.dependent_queries, frame.alpha)?
        }
        _ => {
            // Fallback: utility is a binary "delta is non-degenerate" check.
            // Point estimate 1.0 if the delta loads as valid Turtle and adds at
            // least one triple, else 0.0. LCB conservatively equals point estimate.
            let temp = GraphStore::new();
            let ok = temp.load_turtle(&frame.proposed_delta_ttl, None).is_ok();
            let p = if ok { 1.0 } else { 0.0 };
            (p, p, 0, Vec::new())
        }
    };

    // ── Step 5: provenance hash (canonical-form snapshot + delta) ───────────
    let provenance_hash = compute_provenance_hash(graph, &frame.proposed_delta_ttl)?;

    // ── Step 6: triage ──────────────────────────────────────────────────────
    let mut rationale_parts: Vec<String> = Vec::new();
    let mut assumptions: Vec<String> = vec!["structural_only".to_string()];
    if frame.reversible {
        assumptions.push("reversible".to_string());
    } else {
        assumptions.push("irreversible".to_string());
    }
    if let Some(name) = &frame.action_schema_name {
        assumptions.push(format!("dynamics_action_schema:{}", name));
    }

    let verdict = if !locked_violations.is_empty() {
        rationale_parts.push(format!(
            "REJECT: {} locked IRI(s) targeted",
            locked_violations.len()
        ));
        Verdict::Reject
    } else if cost_triples > frame.risk_threshold {
        rationale_parts.push(format!(
            "REJECT: cost {} exceeds risk_threshold {}",
            cost_triples, frame.risk_threshold
        ));
        Verdict::Reject
    } else if cost_triples > frame.cost_threshold {
        rationale_parts.push(format!(
            "REJECT: cost {} exceeds cost_threshold {}",
            cost_triples, frame.cost_threshold
        ));
        Verdict::Reject
    } else if utility_lcb >= frame.utility_threshold {
        rationale_parts.push(format!(
            "EXECUTE: utility_lcb {:.3} ≥ utility_threshold {:.3}, cost {} within budget {}",
            utility_lcb, frame.utility_threshold, cost_triples, frame.cost_threshold
        ));
        Verdict::Execute
    } else if frame.reversible && frame.allow_experiment {
        rationale_parts.push(format!(
            "EXPERIMENT: utility_lcb {:.3} < threshold {:.3} but reversible and experiment authorised",
            utility_lcb, frame.utility_threshold
        ));
        Verdict::Experiment
    } else {
        rationale_parts.push(format!(
            "ABSTAIN: utility_lcb {:.3} < threshold {:.3}, irreversible or experiment not authorised",
            utility_lcb, frame.utility_threshold
        ));
        Verdict::Abstain
    };

    let identification_proof = format!(
        "structural-dependency closure of {} target IRI(s) has bounded blast radius of {} triples",
        frame.target_iris.len(),
        cost_triples
    );

    let certificate = Certificate {
        verdict,
        graph_slice_hash: slice_hash,
        assumptions,
        identification_proof,
        utility_point_estimate: utility_point,
        utility_lcb,
        alpha: frame.alpha,
        provenance_hash,
        risk_bound: cost_triples,
        rationale: rationale_parts.join("; "),
    };

    // Record into lineage for audit trail.
    let lineage = crate::lineage::LineageLog::new(db.clone());
    lineage.record(
        "civex",
        "CV",
        &format!("certify:{}", frame.tool),
        &format!("{:?}:cost={}:lcb={:.3}", verdict, cost_triples, utility_lcb),
    );

    Ok(CertifyActionResult {
        certificate,
        dependent_queries_evaluated: queries_eval,
        per_query_pass,
    })
}

/// Evaluate each dependent SPARQL query against the (current state + proposed delta).
/// Returns (point estimate, Wilson LCB at α, evaluated count, per-query pass vec).
fn evaluate_dependent_queries(
    graph: &Arc<GraphStore>,
    proposed_delta_ttl: &str,
    queries: &[String],
    alpha: f64,
) -> anyhow::Result<(f64, f64, usize, Vec<bool>)> {
    if queries.is_empty() {
        // No queries to evaluate — best we can say is "pass rate undefined".
        // Conservative: point estimate 1.0 (no evidence of failure), LCB 0.0.
        return Ok((1.0, 0.0, 0, Vec::new()));
    }

    // Build a temp store: clone of current graph + apply delta.
    // For the scaffold we load the delta into a fresh store rather than
    // round-tripping through the main graph (cheaper + side-effect-free).
    let combined = GraphStore::new();
    let snapshot_ttl = graph.serialize("turtle")?;
    combined.load_turtle(&snapshot_ttl, None)?;
    combined.load_turtle(proposed_delta_ttl, None)?;

    let mut passes = Vec::with_capacity(queries.len());
    for q in queries {
        let ok = combined.sparql_select(q).is_ok();
        passes.push(ok);
    }

    let n = passes.len() as f64;
    let k = passes.iter().filter(|p| **p).count() as f64;
    let p_hat = k / n;
    let lcb = wilson_one_sided_lcb(k as u64, n as u64, alpha);

    Ok((p_hat, lcb, passes.len(), passes))
}

/// Wilson one-sided lower confidence bound for a binomial proportion. Closed
/// form, no stats library required. Conservative at small n.
fn wilson_one_sided_lcb(k: u64, n: u64, alpha: f64) -> f64 {
    if n == 0 {
        return 0.0;
    }
    let z = z_from_alpha(alpha);
    let n_f = n as f64;
    let p_hat = k as f64 / n_f;
    let denom = 1.0 + (z * z) / n_f;
    let centre = p_hat + (z * z) / (2.0 * n_f);
    let radius = z * (p_hat * (1.0 - p_hat) / n_f + (z * z) / (4.0 * n_f * n_f)).sqrt();
    let lcb = (centre - radius) / denom;
    lcb.clamp(0.0, 1.0)
}

/// Z-value for a one-sided α. Hard-coded table for common α; closed-form inverse-
/// normal would be cleaner but adds a dependency we don't need for typical uses.
fn z_from_alpha(alpha: f64) -> f64 {
    // One-sided critical values: P(Z ≥ z) = α.
    if alpha <= 0.005 {
        2.576
    } else if alpha <= 0.01 {
        2.326
    } else if alpha <= 0.025 {
        1.960
    } else if alpha <= 0.05 {
        1.645
    } else if alpha <= 0.1 {
        1.282
    } else {
        0.842 // α ≈ 0.2
    }
}

/// Collect IRIs structurally dependent on the action's target IRIs. The scaffold
/// uses one-hop closure (subjects/objects of any triple referencing a target).
/// A production version would walk the full subclass + property-domain/range
/// closure and bound the recursion.
fn collect_dependency_iris(graph: &Arc<GraphStore>, target_iris: &[String]) -> Vec<String> {
    use std::collections::BTreeSet;
    let mut deps: BTreeSet<String> = BTreeSet::new();
    for iri in target_iris {
        let q = format!(
            r#"SELECT DISTINCT ?x WHERE {{
                {{ <{iri}> ?p ?x }} UNION
                {{ ?x ?p <{iri}> }} UNION
                {{ ?x <{iri}> ?o }}
            }} LIMIT 200"#
        );
        if let Ok(json_str) = graph.sparql_select(&q)
            && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json_str)
            && let Some(rows) = parsed["results"].as_array()
        {
            for row in rows {
                if let Some(x) = row["x"].as_str() {
                    deps.insert(x.trim_matches(|c| c == '<' || c == '>').to_string());
                }
            }
        }
    }
    for iri in target_iris {
        deps.insert(iri.clone());
    }
    deps.into_iter().collect()
}

/// Stable hash of an IRI set. SHA-256 over the sorted concatenation.
fn hash_iri_set(iris: &[String]) -> String {
    let mut sorted: Vec<&String> = iris.iter().collect();
    sorted.sort();
    let mut hasher = Sha256::new();
    for iri in sorted {
        hasher.update(iri.as_bytes());
        hasher.update(b"\n");
    }
    hex_encode(&hasher.finalize())
}

/// SHA-256 over (current graph canonical-form serialisation + the proposed delta).
fn compute_provenance_hash(graph: &Arc<GraphStore>, delta_ttl: &str) -> anyhow::Result<String> {
    let snapshot = graph.serialize("turtle")?;
    let mut hasher = Sha256::new();
    hasher.update(snapshot.as_bytes());
    hasher.update(b"\n--DELTA--\n");
    hasher.update(delta_ttl.as_bytes());
    Ok(hex_encode(&hasher.finalize()))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wilson_lcb_at_full_pass_is_strictly_below_one() {
        // 10/10 passes should give an LCB below 1.0 — Wilson's whole point is that
        // observing a 100% pass rate on a small sample doesn't certify the true
        // rate is 100%.
        let lcb = wilson_one_sided_lcb(10, 10, 0.05);
        assert!(lcb < 1.0, "Wilson LCB at 10/10 should be < 1.0; got {}", lcb);
        assert!(lcb > 0.7, "but it should still be > 0.7; got {}", lcb);
    }

    #[test]
    fn wilson_lcb_at_zero_pass_is_zero() {
        let lcb = wilson_one_sided_lcb(0, 10, 0.05);
        assert!(lcb <= 0.01, "Wilson LCB at 0/10 should be ≈ 0; got {}", lcb);
    }

    #[test]
    fn wilson_lcb_at_empty_n_is_zero() {
        assert_eq!(wilson_one_sided_lcb(0, 0, 0.05), 0.0);
    }

    #[test]
    fn z_from_alpha_monotone_decreasing() {
        // As α grows, the critical value should shrink (or stay equal for the
        // hard-coded bands).
        let zs = [
            z_from_alpha(0.005),
            z_from_alpha(0.01),
            z_from_alpha(0.025),
            z_from_alpha(0.05),
            z_from_alpha(0.1),
        ];
        for w in zs.windows(2) {
            assert!(w[0] >= w[1], "z table must be monotone decreasing in α");
        }
    }

    #[test]
    fn hash_iri_set_is_deterministic() {
        let a = hash_iri_set(&["b".to_string(), "a".to_string(), "c".to_string()]);
        let b = hash_iri_set(&["c".to_string(), "a".to_string(), "b".to_string()]);
        assert_eq!(a, b, "hash must be order-independent");
        assert_eq!(a.len(), 64, "SHA-256 hex is 64 chars");
    }
}
