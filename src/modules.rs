//! Many small graphs that reason separately, and a fact promoted when enough
//! of them agree.
//!
//! One global graph forces every context to be consistent with every other at
//! once. That is the qualification problem in its practical form: birds fly,
//! penguins do not, and a single graph holding both is inconsistent unless
//! somebody writes the exception into the rule. The usual answers are
//! non-monotonic and give up machine-checkability.
//!
//! This gives up neither, by refusing the premise. Each module is its own
//! store and reasons ALONE. Within one module it is still ordinary monotonic
//! OWL 2 RL, so every module's entailments are exactly as checkable as they
//! were before, and `onto_reason --certificate` still produces a certificate
//! `oo-cert` accepts. What changes is that two modules are ALLOWED to
//! disagree: that is a fact about them, queryable, rather than an error state.
//!
//! "True everywhere" then stops being an assumption baked into the schema and
//! becomes something a fact has to earn: entailed independently in at least
//! `k` of `n` modules.
//!
//! # What this is not
//!
//! It is not a consensus protocol and there is no cryptography in it. `k` of
//! `n` modules agreeing is a COUNT, and the report says so. Ten modules that
//! all copied the same mistaken axiom agree ten times; agreement is evidence
//! about independence only to the degree the modules really are independent,
//! which this cannot check and does not claim to.
//!
//! It also inherits every limit of the rule table underneath. A fact no rule
//! fires on is entailed in zero modules and is not "disagreed about", it is
//! unseen. `onto_dlp_boundary` is the tool that says which.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::graph::GraphStore;
use crate::reason::{InferenceTarget, Reasoner};

/// One context: a name and the axioms that hold in it.
#[derive(Clone, Debug)]
pub struct Module {
    pub name: String,
    pub ttl: String,
}

/// Everything one module entails, on its own.
fn entailed(m: &Module) -> anyhow::Result<BTreeSet<String>> {
    let g = Arc::new(GraphStore::new());
    g.load_turtle(&m.ttl, None)?;
    // materialize, because the question is what the module ENTAILS and not
    // what its author wrote down.
    Reasoner::run_full(&g, "owl-rl", true, InferenceTarget::DefaultGraph, None)?;
    let nt = g.snapshot("ntriples")?;
    Ok(nt
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        // Blank nodes are labelled per store, so the same structure in two
        // modules would compare unequal for a reason that has nothing to do
        // with agreement. Counted separately would be worse than not counted.
        .filter(|l| !l.contains("_:"))
        .map(|l| l.to_string())
        .collect())
}

/// Reason every module alone, then promote what enough of them agree on.
pub fn distributed(modules: &[Module], k: usize) -> anyhow::Result<serde_json::Value> {
    if modules.is_empty() {
        anyhow::bail!("no modules: there is nothing for a threshold to count");
    }
    if k == 0 || k > modules.len() {
        anyhow::bail!(
            "a threshold of {k} over {} modules is not a question that has an answer",
            modules.len()
        );
    }

    let mut per: Vec<(String, BTreeSet<String>)> = Vec::new();
    for m in modules {
        per.push((m.name.clone(), entailed(m)?));
    }

    // Who entails what.
    let mut who: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (name, set) in &per {
        for t in set {
            who.entry(t.as_str()).or_default().push(name.as_str());
        }
    }

    let mut promoted = Vec::new();
    let mut contested = Vec::new();
    for (t, names) in &who {
        let row = serde_json::json!({
            "triple": t,
            "agreed": names.len(),
            "modules": names,
        });
        if names.len() >= k {
            promoted.push(row);
        } else {
            contested.push(row);
        }
    }

    Ok(serde_json::json!({
        "modules": per.iter().map(|(n, s)| serde_json::json!({
            "name": n, "entails": s.len(),
        })).collect::<Vec<_>>(),
        "threshold": k,
        "promoted": promoted.len(),
        "contested": contested.len(),
        "promoted_facts": promoted,
        "contested_facts": contested,
        "verdict": "agreed_by_k_of_n_modules",
        "means": "each module was reasoned ALONE under OWL 2 RL, in its own store, so no \
                  module's axioms answered another's questions. A promoted fact is one at \
                  least k modules entailed independently. This is a COUNT and not a proof: \
                  modules that copied the same mistaken axiom agree just as loudly, and \
                  agreement is evidence only to the degree the modules really are \
                  independent, which nothing here checks. A contested fact is not an error; \
                  it is the disagreement made visible, which one graph would have hidden by \
                  refusing to hold both.",
    }))
}

/// The promoted facts as N-Triples, ready to load as a module of their own.
pub fn promoted_ntriples(report: &serde_json::Value) -> String {
    report["promoted_facts"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|r| r["triple"].as_str())
                .map(|t| format!("{t}\n"))
                .collect::<String>()
        })
        .unwrap_or_default()
}
