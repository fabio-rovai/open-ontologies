//! Axiom pinpointing: which ASSERTED triples are responsible for a conclusion.
//!
//! A justification (a MinA, a minimal axiom set) for a conclusion `T` is a
//! subset `S` of the asserted graph with `T` in `closure(S)` and `T` not in
//! `closure(S')` for any proper subset `S'`. The second half is the whole
//! point. A support set that is not minimal is a lie about responsibility: it
//! blames triples that had nothing to do with the conclusion, and an engineer
//! deleting one of them and watching the conclusion survive learns to distrust
//! the tool rather than the ontology.
//!
//! # Where the answer comes from, and what checked it
//!
//! Three different things are going on and they carry three different words.
//!
//! * **Sufficiency is re-run, and can be machine-checked.** Every justification
//!   reported here was produced by running the engine over exactly that subset
//!   and seeing the conclusion appear. With `certificate_dir` the run is
//!   repeated with `--certificate`, so `lake exe oo-cert` can verify under
//!   `OOCert.certificate_sound` that the subset really does entail the
//!   conclusion. That is the one claim here a theorem stands behind.
//! * **Minimality is re-run, and is NOT machine-checked.** For every element
//!   `e` of every reported justification, the engine is run again over
//!   `S \ {e}` and the conclusion must be gone. It is a property of this
//!   engine's rule table verified by execution, and no Lean theorem says a set
//!   is minimal.
//! * **Completeness of the LIST is an algorithm's claim, bounded and flagged.**
//!   All justifications are enumerated by Reiter's hitting-set tree over the
//!   same oracle. The search is bounded by `max_justifications` and
//!   `max_oracle_calls`, and `truncated` says when a bound fired and which one.
//!   Reiter's construction is complete for a MONOTONE oracle; see the
//!   monotonicity note below for the one corner of this engine that is not.
//!
//! # The first justification is nearly free, the rest are not
//!
//! [`crate::reason::Reasoner::derivation_graph`] hands back every applicable
//! ground rule instance. Walking one derivation tree from the conclusion down
//! to asserted leaves gives a support set with no re-run at all. It is not
//! necessarily minimal, so it is shrunk by the oracle, and it seeds the
//! hitting-set tree. Everything after that costs one fixpoint per node.
//!
//! # Monotonicity, precisely
//!
//! The rule table is monotone: no rule reads the ABSENCE of a triple. Two
//! places in [`crate::reason`] are not, and they are named here rather than
//! assumed away. `owl:onProperty`, `owl:someValuesFrom`, `owl:allValuesFrom`
//! and `owl:hasValue` are read into maps keyed by the restriction node, and
//! `rdf:first` / `rdf:rest` likewise, so a node carrying TWO values for one of
//! them contributes only one, chosen by hash order. Removing a triple can
//! therefore change which value is read rather than only removing
//! consequences. Where that bites, the hitting-set tree may miss a
//! justification; it cannot report a false one, because every reported set is
//! verified by re-running the engine over it and over each of its subsets.
//!
//! # Inconsistency
//!
//! The same machinery, with the target being "this engine finds a clash" rather
//! than a triple. That is the operationally valuable half: an inconsistent
//! ontology entails everything, and the question is never which conclusion to
//! doubt but which axioms to remove. The verdict word is
//! `clash_found_by_this_engine` exactly as [`crate::reason`] reports it: ten of
//! the seventeen OWL 2 RL rules that conclude `false` are looked for, so a
//! justification here explains a clash this engine found and NOT
//! unsatisfiability.

use crate::graph::GraphStore;
use crate::provenance::{parse_triple, triple_out};
use crate::reason::{DerivationGraph, InferenceTarget, Reasoner, Spelled};
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Justifications returned before the search gives up.
pub const DEFAULT_MAX_JUSTIFICATIONS: usize = 16;

/// Fixpoint re-runs allowed before the search gives up.
///
/// Every node of the hitting-set tree and every minimality check is one full
/// re-run of the reasoner over a subset of the graph. On a large ontology that
/// is the entire cost of this tool, and a run that hits this bound reports the
/// fact rather than a shorter list wearing the word "all".
///
/// It is checked BETWEEN nodes, so the reported `oracle_calls` can overshoot it
/// by one minimisation pass, and the final verification pass runs on top of it
/// whatever it says. The reported number is always the count of runs actually
/// made, never the bound.
pub const DEFAULT_MAX_ORACLE_CALLS: usize = 400;

/// What is being explained.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// One conclusion, in the store's own N-Triples spelling.
    Triple(Spelled),
    /// Any clash this engine detects. Not unsatisfiability; see the module
    /// docs.
    Inconsistency,
}

impl Target {
    fn label(&self) -> &'static str {
        match self {
            Target::Triple(_) => "triple",
            Target::Inconsistency => "inconsistency",
        }
    }

    /// Does this run reach the target?
    fn holds(&self, dg: &DerivationGraph) -> bool {
        match self {
            Target::Triple(t) => dg.holds(t),
            Target::Inconsistency => !dg.clashes.is_empty(),
        }
    }
}

pub struct JustifyOptions {
    pub profile: String,
    pub max_justifications: usize,
    pub max_oracle_calls: usize,
    /// Where to write one derivation certificate per justification, so that
    /// `oo-cert` can verify the SUFFICIENCY half against a machine-checked
    /// soundness theorem.
    pub certificate_dir: Option<PathBuf>,
}

impl Default for JustifyOptions {
    fn default() -> Self {
        Self {
            profile: "owl-rl".to_string(),
            max_justifications: DEFAULT_MAX_JUSTIFICATIONS,
            max_oracle_calls: DEFAULT_MAX_ORACLE_CALLS,
            certificate_dir: None,
        }
    }
}

/// The sentence that travels with every answer here.
pub const ENGINE_CAVEAT: &str =
    "a justification is a statement about THIS ENGINE's rule table, 29 of OWL 2 RL's 78 rules, \
     and about the graph as loaded. A set that no longer yields the conclusion under this table \
     may still yield it under the full profile, and a conclusion this engine never reaches is not \
     a non-entailment result.";

// ───────────────────────────────────────────────────────────────────────────
// The oracle: one fixpoint per question
// ───────────────────────────────────────────────────────────────────────────

struct Oracle<'a> {
    asserted: Vec<Spelled>,
    target: &'a Target,
    profile: &'a str,
    calls: usize,
    /// Set once if a re-run ever stopped at the iteration cap. Every answer
    /// after that is about a partial closure and the report has to say so.
    hit_iteration_cap: bool,
}

impl<'a> Oracle<'a> {
    fn new(asserted: Vec<Spelled>, target: &'a Target, profile: &'a str) -> Self {
        Self {
            asserted,
            target,
            profile,
            calls: 0,
            hit_iteration_cap: false,
        }
    }

    fn store_of(keep: &BTreeSet<Spelled>) -> anyhow::Result<Arc<GraphStore>> {
        let mut nt = String::with_capacity(keep.len() * 96);
        for (s, p, o) in keep {
            nt.push_str(s);
            nt.push(' ');
            nt.push_str(p);
            nt.push(' ');
            nt.push_str(o);
            nt.push_str(" .\n");
        }
        let g = GraphStore::new();
        g.load_ntriples(&nt)?;
        Ok(Arc::new(g))
    }

    /// Run the engine over `keep` and return the DAG when the target is
    /// reached, `None` when it is not.
    fn ask(&mut self, keep: &BTreeSet<Spelled>) -> anyhow::Result<Option<DerivationGraph>> {
        self.calls += 1;
        let store = Self::store_of(keep)?;
        let dg = Reasoner::derivation_graph(&store, self.profile)?;
        if !dg.fixpoint_reached {
            self.hit_iteration_cap = true;
        }
        Ok(self.target.holds(&dg).then_some(dg))
    }

    fn without(&self, removed: &BTreeSet<Spelled>) -> BTreeSet<Spelled> {
        self.asserted
            .iter()
            .filter(|t| !removed.contains(*t))
            .cloned()
            .collect()
    }
}

// ───────────────────────────────────────────────────────────────────────────
// One justification, read off the DAG
// ───────────────────────────────────────────────────────────────────────────

/// The asserted leaves of ONE derivation tree for the target.
///
/// Free: no re-run, just a walk. The result is a support set and is not
/// necessarily minimal, which is why nothing calls this without
/// [`minimise`] afterwards.
///
/// The instance chosen for each fact is the FIRST one the fixpoint found, and
/// that choice is what makes the walk terminate. A rule reads its premises out
/// of the closure as it stood at the start of a round, so the first instance
/// concluding a fact has every premise concluded in a strictly earlier round.
/// The visited set is belt and braces.
fn leaves(dg: &DerivationGraph, target: &Target) -> Option<BTreeSet<Spelled>> {
    let mut first: HashMap<&Spelled, &Vec<Spelled>> = HashMap::new();
    for inst in &dg.instances {
        first.entry(&inst.conclusion).or_insert(&inst.premises);
    }
    let asserted: HashSet<&Spelled> = dg.asserted.iter().collect();

    let mut work: Vec<Spelled> = match target {
        Target::Triple(t) => vec![t.clone()],
        Target::Inconsistency => {
            // Prefer a clash the Lean refutation checker can judge, so that a
            // certificate written for this justification carries a
            // `refutation.tsv` worth checking rather than only the derivation
            // steps.
            let clash = dg
                .clashes
                .iter()
                .find(|c| c.certifiable)
                .or_else(|| dg.clashes.first())?;
            clash.premises.clone()
        }
    };
    let mut seen: HashSet<Spelled> = HashSet::new();
    let mut out: BTreeSet<Spelled> = BTreeSet::new();
    while let Some(t) = work.pop() {
        if asserted.contains(&t) {
            out.insert(t);
            continue;
        }
        if !seen.insert(t.clone()) {
            continue;
        }
        let premises = first.get(&t)?;
        work.extend(premises.iter().cloned());
    }
    Some(out)
}

/// Shrink a support set to a minimal one, one element at a time.
///
/// `|candidate|` oracle calls. Linear minimisation is exact for a monotone
/// oracle: once an element has been shown removable it stays removable, because
/// removing more can only take conclusions away.
fn minimise(
    oracle: &mut Oracle<'_>,
    candidate: &BTreeSet<Spelled>,
) -> anyhow::Result<BTreeSet<Spelled>> {
    let mut cur = candidate.clone();
    for e in candidate.iter() {
        let mut trial = cur.clone();
        trial.remove(e);
        if trial.len() == cur.len() {
            continue;
        }
        if oracle.ask(&trial)?.is_some() {
            cur = trial;
        }
    }
    Ok(cur)
}

// ───────────────────────────────────────────────────────────────────────────
// All justifications: Reiter's hitting-set tree
// ───────────────────────────────────────────────────────────────────────────

struct Search {
    minas: Vec<BTreeSet<Spelled>>,
    truncated_by: Option<&'static str>,
    nodes_expanded: usize,
    reused: usize,
}

/// Enumerate justifications by hitting-set tree.
///
/// A node is a set `H` of removed triples. If the target survives `G \ H` there
/// is a justification disjoint from `H`, and every OTHER justification must
/// contain one of its elements, so the node branches on them. If the target
/// does not survive, `H` is a hitting set of all justifications and nothing
/// below it can be new.
fn hitting_set_tree(oracle: &mut Oracle<'_>, seed: BTreeSet<Spelled>, opts: &JustifyOptions) -> anyhow::Result<Search> {
    let mut s = Search {
        minas: vec![seed.clone()],
        truncated_by: None,
        nodes_expanded: 0,
        reused: 0,
    };
    let mut visited: HashSet<BTreeSet<Spelled>> = HashSet::new();
    let mut queue: VecDeque<BTreeSet<Spelled>> = VecDeque::new();
    for e in &seed {
        queue.push_back(BTreeSet::from([e.clone()]));
    }

    while let Some(h) = queue.pop_front() {
        if s.minas.len() >= opts.max_justifications {
            s.truncated_by = Some("max_justifications");
            break;
        }
        if oracle.calls >= opts.max_oracle_calls {
            s.truncated_by = Some("max_oracle_calls");
            break;
        }
        if !visited.insert(h.clone()) {
            continue;
        }
        s.nodes_expanded += 1;

        // Reuse: a known justification disjoint from `H` is still present in
        // `G \ H`, so the answer is known without a fixpoint. This is the
        // optimisation that makes the tree affordable at all.
        if let Some(m) = s.minas.iter().find(|m| m.is_disjoint(&h)).cloned() {
            s.reused += 1;
            for e in &m {
                let mut next = h.clone();
                next.insert(e.clone());
                queue.push_back(next);
            }
            continue;
        }

        let keep = oracle.without(&h);
        let Some(dg) = oracle.ask(&keep)? else {
            // `H` hits every justification. Closed.
            continue;
        };
        let Some(raw) = leaves(&dg, oracle.target) else {
            // The target holds but no derivation tree could be walked for it.
            // On a correct run this is unreachable; swallowing it would turn a
            // producer bug into a silently short list.
            anyhow::bail!(
                "the target holds over a subset but no derivation tree could be walked for it, \
                 so the derivation DAG and the closure disagree. This is a bug in the producer; \
                 please report it"
            );
        };
        let m = minimise(oracle, &raw)?;
        if !s.minas.contains(&m) {
            s.minas.push(m.clone());
        }
        for e in &m {
            let mut next = h.clone();
            next.insert(e.clone());
            queue.push_back(next);
        }
    }
    s.minas.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.iter().cmp(b.iter())));
    Ok(s)
}

// ───────────────────────────────────────────────────────────────────────────
// Checking a candidate someone else produced
// ───────────────────────────────────────────────────────────────────────────

/// What a candidate support set turned out to be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CandidateVerdict {
    /// The engine does not reach the target from this set at all.
    NotASupport,
    /// The target survives removing each of these, so the set blames triples
    /// that are not responsible.
    NotMinimal(Vec<Spelled>),
    /// Sufficient, and every element is necessary. Both halves re-run.
    MinimalJustification,
}

impl CandidateVerdict {
    pub fn name(&self) -> &'static str {
        match self {
            CandidateVerdict::NotASupport => "not_a_justification_target_not_reached",
            CandidateVerdict::NotMinimal(_) => "not_a_justification_not_minimal",
            CandidateVerdict::MinimalJustification => "minimal_justification",
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
// The entry point
// ───────────────────────────────────────────────────────────────────────────

/// Explain a conclusion, or a clash, in terms of the asserted triples.
pub fn justify(
    graph: &Arc<GraphStore>,
    target_text: Option<&str>,
    inconsistency: bool,
    candidate: Option<&[String]>,
    opts: &JustifyOptions,
) -> anyhow::Result<serde_json::Value> {
    if target_text.is_some() == inconsistency {
        anyhow::bail!(
            "ask about exactly one of `triple` and `inconsistency`. Explaining everything at once \
             is a different question with a different cost"
        );
    }
    if opts.profile == "owl-dl" {
        anyhow::bail!(
            "owl-dl records no rule applications, so there is no derivation to walk and no \
             justification to extract. Run rdfs, owl-rl or owl-rl-ext"
        );
    }

    // Blank nodes do not survive being written out and read back: every oracle
    // call rebuilds a store from a subset of triples, and the parser may
    // relabel. Skolemising once, up front, is what makes a subset comparable to
    // the whole. `crate::projection_entailment` owns that function and the
    // reasons are in its docstring.
    let (sk, _map) = crate::projection_entailment::skolemise(graph)?;

    let target = match target_text {
        Some(t) => Target::Triple(parse_triple(t)?),
        None => Target::Inconsistency,
    };

    let full = Reasoner::derivation_graph(&sk, &opts.profile)?;
    let asserted: Vec<Spelled> = full.asserted.clone();

    let target_json = match &target {
        Target::Triple(t) => triple_out(t),
        Target::Inconsistency => serde_json::json!("inconsistency"),
    };
    let mut out = serde_json::json!({
        "target": target_json,
        "target_kind": target.label(),
        "profile_used": full.profile_used,
        "asserted": asserted.len(),
        "closure": full.closure.len(),
        "rule_instances": full.instances.len(),
        "fixpoint_reached": full.fixpoint_reached,
        "means": ENGINE_CAVEAT,
    });
    if let Target::Inconsistency = target {
        out["verdict_explained"] = serde_json::json!("clash_found_by_this_engine");
        out["verdict_means"] = serde_json::json!(
            "the justifications below explain a contradiction THIS ENGINE found, under ten of the \
             seventeen OWL 2 RL rules that conclude false. They are not an explanation of \
             unsatisfiability, and `oo-refute`'s `unsatisfiable_under_disjointness` is a different \
             word for a different, checked, result"
        );
        out["clashes_found"] = serde_json::json!(full.clashes.len());
    }
    if !full.fixpoint_reached {
        out["fixpoint_warning"] = serde_json::json!(
            "the reasoner stopped at its iteration cap rather than a fixpoint, so the closure is \
             a LOWER BOUND. A justification search over a partial closure can miss justifications \
             and can call a set minimal that is not, because the removal test is run against the \
             same partial closure"
        );
    }

    if !target.holds(&full) {
        out["reached"] = serde_json::json!(false);
        out["justifications"] = serde_json::json!([]);
        let note = match target {
            Target::Triple(_) => {
                "this engine does not derive that triple from the loaded graph under this \
                 profile, so there is nothing to explain. That is not a non-entailment result"
            }
            Target::Inconsistency => {
                "this engine found no clash in the closure it computed. That is NOT a consistency \
                 result: seven of the seventeen rules that conclude false are not looked for at \
                 all, and a TBox unsatisfiable with no individual asserted is invisible to this \
                 route"
            }
        };
        out["note"] = serde_json::json!(note);
        return Ok(out);
    }
    out["reached"] = serde_json::json!(true);

    let mut oracle = Oracle::new(asserted.clone(), &target, &opts.profile);

    // ── The candidate path: check, do not search ────────────────────────
    if let Some(cand) = candidate {
        let mut parsed: BTreeSet<Spelled> = BTreeSet::new();
        let asserted_set: HashSet<&Spelled> = asserted.iter().collect();
        let mut not_asserted: Vec<Spelled> = Vec::new();
        for c in cand {
            let t = parse_triple(c)?;
            if !asserted_set.contains(&t) {
                not_asserted.push(t.clone());
            }
            parsed.insert(t);
        }
        let verdict = if oracle.ask(&parsed)?.is_none() {
            CandidateVerdict::NotASupport
        } else {
            let mut removable: Vec<Spelled> = Vec::new();
            for e in parsed.iter() {
                let mut trial = parsed.clone();
                trial.remove(e);
                if oracle.ask(&trial)?.is_some() {
                    removable.push(e.clone());
                }
            }
            if removable.is_empty() {
                CandidateVerdict::MinimalJustification
            } else {
                CandidateVerdict::NotMinimal(removable)
            }
        };
        let removable: Vec<serde_json::Value> = match &verdict {
            CandidateVerdict::NotMinimal(rs) => rs.iter().map(triple_out).collect(),
            _ => Vec::new(),
        };
        let means = match &verdict {
            CandidateVerdict::NotASupport => {
                "running the engine over exactly this set does not reach the target, so it is \
                 not a support at all"
            }
            CandidateVerdict::NotMinimal(_) => {
                "the target survives removing each triple under `removable`, so this set blames \
                 triples that are not responsible for it. Every element of a justification must \
                 be necessary"
            }
            CandidateVerdict::MinimalJustification => {
                "the engine reaches the target from this set, and loses it when any one element \
                 is removed. Both halves were re-run, neither is machine-checked"
            }
        };
        out["candidate"] = serde_json::json!({
            "size": parsed.len(),
            "triples": parsed.iter().map(triple_out).collect::<Vec<_>>(),
            "verdict": verdict.name(),
            "not_in_the_asserted_graph": not_asserted.iter().map(triple_out).collect::<Vec<_>>(),
            "removable": removable,
            "means": means,
            "checks_run": parsed.len() + 1,
        });
        out["oracle_calls"] = serde_json::json!(oracle.calls);
        out["searched"] = serde_json::json!(false);
        if oracle.hit_iteration_cap {
            out["iteration_cap_hit_during_search"] = serde_json::json!(true);
        }
        return Ok(out);
    }

    // ── The search path ─────────────────────────────────────────────────
    let Some(raw) = leaves(&full, &target) else {
        anyhow::bail!(
            "the target holds but no derivation tree could be walked for it, so the derivation \
             DAG and the closure disagree. This is a bug in the producer; please report it"
        );
    };
    let from_dag = raw.len();
    let seed = minimise(&mut oracle, &raw)?;
    let search = hitting_set_tree(&mut oracle, seed, opts)?;

    // ── Minimality, verified rather than argued ─────────────────────────
    //
    // Every element of every reported justification is removed and the engine
    // is run again. The claim in the payload is exactly what these runs show
    // and never more.
    let mut verified = true;
    let mut failures: Vec<serde_json::Value> = Vec::new();
    let mut justifications: Vec<serde_json::Value> = Vec::new();
    for m in &search.minas {
        let sufficient = oracle.ask(m)?.is_some();
        if !sufficient {
            verified = false;
            failures.push(serde_json::json!({
                "kind": "not_sufficient",
                "justification": m.iter().map(triple_out).collect::<Vec<_>>(),
            }));
        }
        let mut necessary = true;
        for e in m.iter() {
            let mut trial = m.clone();
            trial.remove(e);
            if oracle.ask(&trial)?.is_some() {
                necessary = false;
                verified = false;
                failures.push(serde_json::json!({
                    "kind": "element_not_necessary",
                    "triple": triple_out(e),
                }));
            }
        }
        let sufficiency = if sufficient {
            "re_run_and_reached"
        } else {
            "RE_RUN_AND_NOT_REACHED"
        };
        let minimality = if necessary {
            "verified_by_re_running_the_fixpoint_without_each_element"
        } else {
            "FAILED: the target survived removing an element"
        };
        justifications.push(serde_json::json!({
            "size": m.len(),
            "triples": m.iter().map(triple_out).collect::<Vec<_>>(),
            "sufficiency": sufficiency,
            "minimality": minimality,
            "removal_checks": m.len(),
        }));
    }

    out["searched"] = serde_json::json!(true);
    out["justifications"] = serde_json::json!(justifications);
    out["justification_count"] = serde_json::json!(search.minas.len());
    out["first_justification_from_the_dag"] = serde_json::json!({
        "size": from_dag,
        "means": "the asserted leaves of ONE derivation tree, read off the DAG with no re-run. \
                  It is a support and is not necessarily minimal; it was shrunk by the oracle \
                  before being reported, and it seeded the hitting-set tree",
    });
    out["truncated"] = serde_json::json!(search.truncated_by.is_some());
    out["complete"] = serde_json::json!(search.truncated_by.is_none());
    match search.truncated_by {
        Some(bound) => {
            let value = if bound == "max_justifications" {
                opts.max_justifications
            } else {
                opts.max_oracle_calls
            };
            out["truncation"] = serde_json::json!({
                "bound": bound,
                "value": value,
                "means": "the hitting-set tree stopped at a bound rather than because it had \
                          closed, so there may be justifications this list does not contain. \
                          Every justification IN it is still sufficient and minimal, both \
                          re-run. Raise the bound to continue",
            });
        }
        None => {
            out["completeness_means"] = serde_json::json!(
                "the hitting-set tree closed without hitting a bound, so under Reiter's \
                 construction this is EVERY justification, provided the oracle is monotone. This \
                 engine has one non-monotone corner: a restriction node or list node carrying two \
                 values for a functional position contributes one, chosen by hash order. Where \
                 that occurs a justification can be missed. No listed justification can be \
                 wrong, because each was re-run"
            );
        }
    }
    out["minimality_verified"] = serde_json::json!(verified);
    if !verified {
        out["minimality_failures"] = serde_json::json!(failures);
        out["minimality_failure_means"] = serde_json::json!(
            "a justification failed its own re-run check. That is a defect in this tool or a \
             non-monotone corner of the engine, it is reported rather than hidden, and the list \
             above must not be trusted. Please report it"
        );
    }
    out["oracle_calls"] = serde_json::json!(oracle.calls);
    out["oracle"] = serde_json::json!(
        "one full re-run of this engine's fixpoint over a subset of the asserted graph. Nodes \
         expanded and justifications reused are reported so the cost is legible. oracle_calls is \
         the number of runs actually made and can exceed max_oracle_calls, which is checked \
         between nodes and does not stop the verification pass"
    );
    out["nodes_expanded"] = serde_json::json!(search.nodes_expanded);
    out["nodes_answered_without_a_re_run"] = serde_json::json!(search.reused);
    if oracle.hit_iteration_cap {
        out["iteration_cap_hit_during_search"] = serde_json::json!(true);
        out["iteration_cap_means"] = serde_json::json!(
            "at least one re-run stopped at the iteration cap instead of a fixpoint, so at least \
             one answer above is about a partial closure. Raise reasoner.max_iterations and run \
             again before believing the list"
        );
    }

    // ── Certificates, the one half a theorem covers ─────────────────────
    if let Some(dir) = &opts.certificate_dir {
        let mut written: Vec<serde_json::Value> = Vec::new();
        for (i, m) in search.minas.iter().enumerate() {
            let sub = dir.join(format!("justification-{i}"));
            let store = Oracle::store_of(m)?;
            let report = Reasoner::run_full(
                &store,
                &opts.profile,
                false,
                InferenceTarget::DefaultGraph,
                Some(&sub),
            )?;
            let parsed: serde_json::Value = serde_json::from_str(&report)?;
            written.push(serde_json::json!({
                "index": i,
                "dir": sub.display().to_string(),
                "derivations": parsed["certificate"]["derivations"],
                "refutation": parsed["inconsistency"]["refutation"]["written"],
                "check_with": check_command(&sub, &target),
            }));
        }
        out["certificates"] = serde_json::json!(written);
        out["certificates_mean"] = serde_json::json!(
            "one certified run per justification, over exactly that justification's triples. \
             `oo-cert` accepting one establishes SUFFICIENCY under OOCert.certificate_sound: the \
             conclusion really does follow from that subset. It says NOTHING about minimality, \
             which only the removal re-runs above support, and nothing about the list being \
             complete"
        );
    }

    Ok(out)
}

fn check_command(dir: &Path, target: &Target) -> String {
    match target {
        Target::Triple(_) => format!(
            "cd lean && lake exe oo-cert {a} {d}",
            a = dir.join("asserted.tsv").display(),
            d = dir.join("derivations.tsv").display()
        ),
        Target::Inconsistency => format!(
            "cd lean && lake exe oo-refute check {a} {r}  (only when refutation is true; \
             cax-dw is the one clash rule the checker has a semantic condition for)",
            a = dir.join("asserted.tsv").display(),
            r = dir.join("refutation.tsv").display()
        ),
    }
}
