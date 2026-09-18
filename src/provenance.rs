//! Provenance semirings over the derivation DAG the reasoner already builds.
//!
//! Every derived triple carries an algebraic expression over the ASSERTED
//! triples, computed by evaluating the fixpoint a second time with the truth
//! values replaced by elements of a commutative semiring. That is Green,
//! Karvounarakis and Tannen's construction, applied to the rule table in
//! [`crate::reason`] rather than to relational algebra.
//!
//! # The substrate is the DAG, not the certificate
//!
//! `derivations.tsv` records ONE step per inferred triple, the first the
//! fixpoint reached. A polynomial built from that file would have one monomial
//! wherever the closure supports several, which is a false statement about
//! where the conclusion came from rather than a coarse one. So this module asks
//! [`Reasoner::derivation_graph`] for every applicable ground rule instance and
//! evaluates over that.
//!
//! # Recursion is where provenance goes wrong quietly
//!
//! Datalog is recursive, and under recursion the provenance series of the free
//! semiring N[X] is INFINITE: a triple on a cycle has arbitrarily many proof
//! trees, so its counting annotation diverges and its how-provenance polynomial
//! does not exist as a polynomial. Producing a finite answer anyway and not
//! saying so is the failure this module is written to avoid.
//!
//! Two honest answers exist and both are implemented.
//!
//! * **Absorptive and finite-lattice semirings converge.** `why`, `lineage`,
//!   `boolean`, `trust` (max-min) and `tropical` (min-plus over non-negative
//!   weights) all reach a fixpoint in finitely many rounds over a finite
//!   closure, and the report says the iteration stabilised. `why`, `trust` and
//!   `tropical` are absorptive, so a derivation that goes round a cycle is
//!   absorbed by the one that does not. `boolean` and `lineage` are NOT
//!   absorptive; they converge because their value lattices are finite and the
//!   iteration is monotone in them. Both reasons are stated per semiring rather
//!   than collapsed into the word "converges".
//! * **`counting` does not converge, and is bounded and reported.** Round `k`
//!   of the iteration computes the annotation over proof trees of HEIGHT AT
//!   MOST `k`, so stopping at `depth_bound` gives a LOWER BOUND, the bound is
//!   in the payload, and `value_is_exact` is false unless the iteration
//!   stabilised on its own. `cycle_in_support` says whether the target's own
//!   support contains a cycle, which is the structural reason a count cannot
//!   be exact.
//!
//! # What an annotation is a claim about
//!
//! The rule table is this engine's 29 of OWL 2 RL's 78 rules, so a monomial is
//! a claim about what THIS engine derived and not about OWL 2 RL entailment.
//! The soundness of each step is the Lean layer's business
//! (`OOCert.certificate_sound`); nothing here is machine-checked, and the
//! separate `onto_justify` is where a support set is verified by re-running the
//! engine rather than read off an algebra.

use crate::graph::GraphStore;
use crate::projection_entailment::SKOLEM_PREFIX;
use crate::reason::{DerivationGraph, Reasoner, Spelled};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

/// Rounds of the annotated fixpoint run by default.
///
/// A round is a derivation-depth step, so this is the maximum proof-tree height
/// a non-convergent semiring's answer covers. Absorptive semirings stabilise
/// long before it on every ontology this repository ships.
pub const DEFAULT_DEPTH_BOUND: usize = 32;

/// Monomials kept per fact in the `why` semiring before truncation.
///
/// Why-provenance is worst-case exponential in the size of the closure. When
/// the cap bites, the monomials KEPT are the smallest ones, which is what makes
/// the surviving set still an antichain: a dropped monomial cannot be a proper
/// subset of a kept one, because a proper subset is strictly smaller and would
/// have been kept first.
pub const DEFAULT_MAX_MONOMIALS: usize = 64;

// ───────────────────────────────────────────────────────────────────────────
// The semirings
// ───────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Semiring {
    /// `({false, true}, ∨, ∧, false, true)`. Derivability and nothing else.
    Boolean,
    /// Why-provenance: sets of sets of asserted triples, `⊕` union then
    /// absorption, `⊗` pairwise union. The minimal elements are the
    /// justifications.
    Why,
    /// Lineage, Green et al.'s `Which(X)`: `(𝒫(X) ∪ {⊥}, ∪, ∪, ⊥, ∅)`. Which
    /// asserted triples contributed AT ALL, with no record of how they combine.
    Lineage,
    /// `(ℕ, +, ×, 0, 1)`. The number of proof trees. DIVERGES under recursion.
    Counting,
    /// Min-plus over `[0, ∞]`: `⊕ = min`, `⊗ = +`, `0̄ = ∞`, `1̄ = 0`. The
    /// cheapest proof tree, summing the weights of its leaves with
    /// multiplicity. Absorptive only because weights are required to be
    /// non-negative.
    TropicalMinPlus,
    /// Max-min over `[0, 1]`: `⊕ = max`, `⊗ = min`, `0̄ = 0`, `1̄ = 1`. The
    /// confidence of the best derivation, which is the confidence of its
    /// weakest premise. Absorptive.
    TrustMaxMin,
}

impl Semiring {
    pub fn name(self) -> &'static str {
        match self {
            Semiring::Boolean => "boolean",
            Semiring::Why => "why",
            Semiring::Lineage => "lineage",
            Semiring::Counting => "counting",
            Semiring::TropicalMinPlus => "tropical",
            Semiring::TrustMaxMin => "trust",
        }
    }

    pub fn parse(s: &str) -> Option<Semiring> {
        match s {
            "boolean" | "bool" => Some(Semiring::Boolean),
            "why" | "why_provenance" => Some(Semiring::Why),
            "lineage" | "which" => Some(Semiring::Lineage),
            "counting" | "count" | "n" => Some(Semiring::Counting),
            "tropical" | "min_plus" | "min-plus" => Some(Semiring::TropicalMinPlus),
            "trust" | "max_min" | "max-min" | "fuzzy" => Some(Semiring::TrustMaxMin),
            _ => None,
        }
    }

    pub fn all() -> Vec<Semiring> {
        vec![
            Semiring::Boolean,
            Semiring::Why,
            Semiring::Lineage,
            Semiring::Counting,
            Semiring::TropicalMinPlus,
            Semiring::TrustMaxMin,
        ]
    }

    /// Whether `a ⊕ (a ⊗ b) = a`, which is what makes a recursive fixpoint
    /// stop: a derivation that passes through a cycle is absorbed by the
    /// shorter one it contains.
    pub fn absorptive(self) -> bool {
        matches!(
            self,
            Semiring::Why | Semiring::TropicalMinPlus | Semiring::TrustMaxMin
        )
    }

    /// Why this semiring's fixpoint terminates, or that it does not. One
    /// sentence, carried in the payload so a reader never has to infer it.
    pub fn convergence(self) -> &'static str {
        match self {
            Semiring::Boolean =>
                "converges: not absorptive, but its value lattice has two elements and the \
                 iteration is monotone in it",
            Semiring::Why =>
                "converges: absorptive, so a derivation that goes round a cycle is absorbed by \
                 the one that does not. Worst-case exponential in the size of the closure, which \
                 is what max_monomials bounds",
            Semiring::Lineage =>
                "converges: NOT absorptive (a + a·b = a ∪ b, not a), but its values are subsets \
                 of a finite set and the iteration only grows them",
            Semiring::Counting =>
                "DOES NOT CONVERGE under recursion: a triple on a cycle has arbitrarily many \
                 proof trees. Round k counts proof trees of height at most k, so the answer is a \
                 LOWER BOUND whenever the iteration did not stabilise on its own, and \
                 depth_bound is in the payload",
            Semiring::TropicalMinPlus =>
                "converges: absorptive PROVIDED every weight is non-negative, since min(a, a+b) \
                 = a needs b >= 0. A negative weight is refused rather than silently diverging",
            Semiring::TrustMaxMin =>
                "converges: absorptive, max(a, min(a, b)) = a for all a and b",
        }
    }

    /// What the annotation means, in the payload beside the number.
    pub fn means(self) -> &'static str {
        match self {
            Semiring::Boolean => "true when this engine derives the triple at all",
            Semiring::Why =>
                "each monomial is the set of asserted triples at the leaves of one proof tree, \
                 and the set of monomials is an antichain: no monomial contains another. Under \
                 an untruncated, stabilised run over a MONOTONE rule table they coincide with the \
                 minimal supports. They are computed from the derivation DAG rather than by \
                 re-running the engine, and onto_justify is the tool that re-runs it and names \
                 the one corner of this engine that is not monotone",
            Semiring::Lineage =>
                "the union of every asserted triple that contributes to any derivation. It is \
                 the coarsest useful answer: it says WHICH triples are involved and never how \
                 they combine, so it is NOT a justification and is usually far from minimal",
            Semiring::Counting =>
                "the number of distinct proof trees of height at most depth_bound. Read \
                 value_is_exact before reading the number",
            Semiring::TropicalMinPlus =>
                "the total weight of the cheapest proof tree, summing the weight of each leaf \
                 once per occurrence. Default weight 1.0 per asserted triple, so the default \
                 reading is the size of the smallest proof tree counted with multiplicity",
            Semiring::TrustMaxMin =>
                "the confidence of the best derivation, where a derivation is only as good as \
                 its weakest premise. Default weight 1.0 per asserted triple, so the default \
                 reading is trivially 1.0 and the semiring is only interesting with weights",
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Values
// ───────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
enum Val {
    Bool(bool),
    /// An antichain of monomials over asserted-variable indices, sorted by
    /// (size, contents) so the representation is canonical.
    Why(Vec<BTreeSet<u32>>),
    /// `None` is `⊥`, the zero.
    Lineage(Option<BTreeSet<u32>>),
    /// The count, and whether it hit the ceiling of the counter.
    Count(u128, bool),
    /// Min-plus. `f64::INFINITY` is the zero.
    Cost(f64),
    /// Max-min. `0.0` is the zero.
    Trust(f64),
}

impl Val {
    fn zero(sem: Semiring) -> Val {
        match sem {
            Semiring::Boolean => Val::Bool(false),
            Semiring::Why => Val::Why(Vec::new()),
            Semiring::Lineage => Val::Lineage(None),
            Semiring::Counting => Val::Count(0, false),
            Semiring::TropicalMinPlus => Val::Cost(f64::INFINITY),
            Semiring::TrustMaxMin => Val::Trust(0.0),
        }
    }

    fn is_zero(&self) -> bool {
        match self {
            Val::Bool(b) => !b,
            Val::Why(ms) => ms.is_empty(),
            Val::Lineage(l) => l.is_none(),
            Val::Count(n, sat) => *n == 0 && !sat,
            Val::Cost(c) => c.is_infinite(),
            Val::Trust(t) => *t == 0.0,
        }
    }

    /// The annotation an asserted triple starts with: its own variable, or its
    /// weight.
    fn base(sem: Semiring, var: u32, weight: f64) -> Val {
        match sem {
            Semiring::Boolean => Val::Bool(true),
            Semiring::Why => Val::Why(vec![BTreeSet::from([var])]),
            Semiring::Lineage => Val::Lineage(Some(BTreeSet::from([var]))),
            Semiring::Counting => Val::Count(1, false),
            Semiring::TropicalMinPlus => Val::Cost(weight),
            Semiring::TrustMaxMin => Val::Trust(weight),
        }
    }

    /// The multiplicative identity, which is what an empty body would
    /// evaluate to. No rule in the table has an empty body; the fold starts
    /// here regardless, so the code says what it means.
    fn one(sem: Semiring) -> Val {
        match sem {
            Semiring::Boolean => Val::Bool(true),
            Semiring::Why => Val::Why(vec![BTreeSet::new()]),
            Semiring::Lineage => Val::Lineage(Some(BTreeSet::new())),
            Semiring::Counting => Val::Count(1, false),
            Semiring::TropicalMinPlus => Val::Cost(0.0),
            Semiring::TrustMaxMin => Val::Trust(1.0),
        }
    }
}

/// `⊕`, with the why-semiring's absorption and truncation folded in. Returns
/// whether the cap discarded a monomial.
fn plus(a: &Val, b: &Val, max_monomials: usize) -> (Val, bool) {
    match (a, b) {
        (Val::Bool(x), Val::Bool(y)) => (Val::Bool(*x || *y), false),
        (Val::Why(x), Val::Why(y)) => {
            let mut all = x.clone();
            all.extend(y.iter().cloned());
            let (v, t) = normalise_why(all, max_monomials);
            (Val::Why(v), t)
        }
        (Val::Lineage(x), Val::Lineage(y)) => (
            Val::Lineage(match (x, y) {
                (None, other) => other.clone(),
                (some, None) => some.clone(),
                (Some(p), Some(q)) => Some(p.union(q).copied().collect()),
            }),
            false,
        ),
        (Val::Count(x, sx), Val::Count(y, sy)) => {
            let (n, over) = x.overflowing_add(*y);
            let saturated = *sx || *sy || over;
            (
                Val::Count(if over { u128::MAX } else { n }, saturated),
                false,
            )
        }
        (Val::Cost(x), Val::Cost(y)) => (Val::Cost(x.min(*y)), false),
        (Val::Trust(x), Val::Trust(y)) => (Val::Trust(x.max(*y)), false),
        // Unreachable: one semiring is chosen per evaluation and every value in
        // it is built by this module. A mismatch would be a bug here, and
        // returning `a` would hide it, so it is loud.
        _ => unreachable!("provenance values of two different semirings were combined"),
    }
}

/// `⊗`.
fn times(a: &Val, b: &Val, max_monomials: usize) -> (Val, bool) {
    match (a, b) {
        (Val::Bool(x), Val::Bool(y)) => (Val::Bool(*x && *y), false),
        (Val::Why(x), Val::Why(y)) => {
            if x.is_empty() || y.is_empty() {
                return (Val::Why(Vec::new()), false);
            }
            let mut prod: Vec<BTreeSet<u32>> = Vec::with_capacity(x.len() * y.len());
            for m in x {
                for n in y {
                    prod.push(m.union(n).copied().collect());
                }
            }
            let (v, t) = normalise_why(prod, max_monomials);
            (Val::Why(v), t)
        }
        (Val::Lineage(x), Val::Lineage(y)) => (
            Val::Lineage(match (x, y) {
                (None, _) | (_, None) => None,
                (Some(p), Some(q)) => Some(p.union(q).copied().collect()),
            }),
            false,
        ),
        (Val::Count(x, sx), Val::Count(y, sy)) => {
            let (n, over) = x.overflowing_mul(*y);
            let zero = *x == 0 || *y == 0;
            let saturated = !zero && (*sx || *sy || over);
            (
                Val::Count(if over { u128::MAX } else { n }, saturated),
                false,
            )
        }
        (Val::Cost(x), Val::Cost(y)) => (Val::Cost(x + y), false),
        (Val::Trust(x), Val::Trust(y)) => (Val::Trust(x.min(*y)), false),
        _ => unreachable!("provenance values of two different semirings were combined"),
    }
}

/// Absorb and cap a bag of monomials.
///
/// Sorting by size first is what makes both halves correct at once. Absorption
/// then only ever has to test a candidate against sets no larger than itself,
/// and the cap keeps the SMALLEST monomials, so a discarded monomial can never
/// be a proper subset of a kept one and the survivors are still an antichain.
fn normalise_why(mut ms: Vec<BTreeSet<u32>>, max_monomials: usize) -> (Vec<BTreeSet<u32>>, bool) {
    ms.sort_by(|a, b| {
        a.len()
            .cmp(&b.len())
            .then_with(|| a.iter().cmp(b.iter()))
    });
    ms.dedup();
    let mut kept: Vec<BTreeSet<u32>> = Vec::new();
    for m in ms {
        if kept.iter().any(|k| k.is_subset(&m)) {
            continue;
        }
        kept.push(m);
    }
    let truncated = kept.len() > max_monomials;
    if truncated {
        kept.truncate(max_monomials);
    }
    (kept, truncated)
}

// ───────────────────────────────────────────────────────────────────────────
// The DAG, indexed
// ───────────────────────────────────────────────────────────────────────────

/// The derivation DAG with triples replaced by indices, which is the shape the
/// annotated fixpoint iterates over.
pub struct Dag {
    /// Every triple in the closure, sorted, so ids are deterministic.
    pub facts: Vec<Spelled>,
    index: HashMap<Spelled, u32>,
    /// For each fact, the asserted-variable id when it was asserted.
    var_of: Vec<Option<u32>>,
    /// The asserted triples, sorted. A variable id indexes this.
    pub vars: Vec<Spelled>,
    /// For each fact, one entry per applicable rule instance concluding it.
    bodies: Vec<Vec<Vec<u32>>>,
    /// The rule name of each body, parallel to `bodies`.
    body_rules: Vec<Vec<&'static str>>,
    /// Instances dropped because a premise was not in the closure. Always zero
    /// on a run of this engine, reported because a non-zero value would mean
    /// the capture and the closure disagree and every answer below would be
    /// computed over a DAG missing hyperedges.
    pub instances_dropped: usize,
}

impl Dag {
    pub fn build(dg: &DerivationGraph) -> Dag {
        let mut facts: Vec<Spelled> = dg.closure.iter().cloned().collect();
        facts.sort();
        facts.dedup();
        let index: HashMap<Spelled, u32> = facts
            .iter()
            .enumerate()
            .map(|(i, f)| (f.clone(), i as u32))
            .collect();

        let mut vars: Vec<Spelled> = dg.asserted.clone();
        vars.sort();
        vars.dedup();
        let mut var_of: Vec<Option<u32>> = vec![None; facts.len()];
        for (vi, v) in vars.iter().enumerate() {
            if let Some(&fi) = index.get(v) {
                var_of[fi as usize] = Some(vi as u32);
            }
        }

        let mut bodies: Vec<Vec<Vec<u32>>> = vec![Vec::new(); facts.len()];
        let mut body_rules: Vec<Vec<&'static str>> = vec![Vec::new(); facts.len()];
        let mut dropped = 0usize;
        for inst in &dg.instances {
            let Some(&head) = index.get(&inst.conclusion) else {
                dropped += 1;
                continue;
            };
            let mut body: Vec<u32> = Vec::with_capacity(inst.premises.len());
            let mut ok = true;
            for p in &inst.premises {
                match index.get(p) {
                    Some(&pi) => body.push(pi),
                    None => {
                        ok = false;
                        break;
                    }
                }
            }
            if !ok {
                dropped += 1;
                continue;
            }
            bodies[head as usize].push(body);
            body_rules[head as usize].push(inst.rule);
        }

        Dag {
            facts,
            index,
            var_of,
            vars,
            bodies,
            body_rules,
            instances_dropped: dropped,
        }
    }

    pub fn id(&self, t: &Spelled) -> Option<u32> {
        self.index.get(t).copied()
    }

    pub fn rules_for(&self, fact: u32) -> &[&'static str] {
        &self.body_rules[fact as usize]
    }

    pub fn bodies_for(&self, fact: u32) -> &[Vec<u32>] {
        &self.bodies[fact as usize]
    }

    /// Does the support of `target` contain a cycle?
    ///
    /// The structural reason a counting annotation cannot be exact. Answered
    /// over the sub-DAG reachable from the target through bodies, with an
    /// explicit stack rather than recursion, because a closure is not bounded
    /// in depth by anything.
    pub fn cycle_in_support(&self, target: u32) -> Option<Spelled> {
        #[derive(Clone, Copy, PartialEq)]
        enum Mark {
            White,
            Grey,
            Black,
        }
        let mut mark = vec![Mark::White; self.facts.len()];
        // (fact, index of the next premise to visit, flattened premise list)
        let mut stack: Vec<(u32, usize, Vec<u32>)> = Vec::new();
        let flat = |f: u32| -> Vec<u32> {
            let mut v: Vec<u32> = self.bodies[f as usize].iter().flatten().copied().collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        mark[target as usize] = Mark::Grey;
        stack.push((target, 0, flat(target)));
        while let Some((f, i, prem)) = stack.pop() {
            if i == prem.len() {
                mark[f as usize] = Mark::Black;
                continue;
            }
            let next = prem[i];
            stack.push((f, i + 1, prem));
            match mark[next as usize] {
                Mark::Grey => return Some(self.facts[next as usize].clone()),
                Mark::Black => {}
                Mark::White => {
                    mark[next as usize] = Mark::Grey;
                    let p = flat(next);
                    stack.push((next, 0, p));
                }
            }
        }
        None
    }
}

// ───────────────────────────────────────────────────────────────────────────
// The annotated fixpoint
// ───────────────────────────────────────────────────────────────────────────

/// What one semiring's evaluation produced.
pub struct Eval {
    values: Vec<Val>,
    /// The iteration reached a round that changed nothing.
    pub stabilised: bool,
    pub rounds: usize,
    pub depth_bound: usize,
    /// The why-semiring's monomial cap discarded something, somewhere.
    pub truncated: bool,
    /// A counter hit its ceiling, so the number below it is meaningless rather
    /// than merely a bound.
    pub saturated: bool,
}

/// Evaluate the whole closure under one semiring.
///
/// Round `k` computes the annotation over proof trees of height at most `k`,
/// which is what makes `depth_bound` a derivation-depth bound and not an
/// implementation detail.
pub fn evaluate(
    dag: &Dag,
    sem: Semiring,
    depth_bound: usize,
    max_monomials: usize,
    weights: &BTreeMap<Spelled, f64>,
) -> Eval {
    let n = dag.facts.len();
    let base: Vec<Val> = (0..n)
        .map(|i| match dag.var_of[i] {
            // An asserted triple with no weight supplied is weighted 1.0 in
            // both scalar semirings. Under min-plus that makes the annotation
            // the size of the smallest proof tree; under max-min it makes every
            // annotation 1.0, which is the honest reading of "nobody said how
            // much they trust anything".
            Some(v) => Val::base(sem, v, weights.get(&dag.facts[i]).copied().unwrap_or(1.0)),
            None => Val::zero(sem),
        })
        .collect();

    let mut cur = base.clone();
    let mut stabilised = false;
    let mut rounds = 0usize;
    let mut truncated = false;
    for _ in 0..depth_bound {
        rounds += 1;
        let mut next = Vec::with_capacity(n);
        for (i, b) in base.iter().enumerate() {
            let mut acc = b.clone();
            for body in &dag.bodies[i] {
                let mut prod = Val::one(sem);
                let mut dead = false;
                for &p in body {
                    let (v, t) = times(&prod, &cur[p as usize], max_monomials);
                    truncated |= t;
                    prod = v;
                    if prod.is_zero() {
                        dead = true;
                        break;
                    }
                }
                if dead {
                    continue;
                }
                let (v, t) = plus(&acc, &prod, max_monomials);
                truncated |= t;
                acc = v;
            }
            next.push(acc);
        }
        if next == cur {
            stabilised = true;
            break;
        }
        cur = next;
    }
    let saturated = cur
        .iter()
        .any(|v| matches!(v, Val::Count(_, true)));
    Eval {
        values: cur,
        stabilised,
        rounds,
        depth_bound,
        truncated,
        saturated,
    }
}

impl Eval {
    /// The annotation of one fact, rendered for the report.
    fn render(&self, sem: Semiring, dag: &Dag, fact: u32, used: &mut BTreeSet<u32>) -> serde_json::Value {
        let v = &self.values[fact as usize];
        let mut out = serde_json::json!({
            "semiring": sem.name(),
            "absorptive": sem.absorptive(),
            "convergence": sem.convergence(),
            "means": sem.means(),
            "stabilised": self.stabilised,
            "rounds_run": self.rounds,
            "depth_bound": self.depth_bound,
        });
        match v {
            Val::Bool(b) => {
                out["value"] = serde_json::json!(b);
            }
            Val::Why(ms) => {
                for m in ms {
                    used.extend(m.iter().copied());
                }
                out["monomial_count"] = serde_json::json!(ms.len());
                out["monomials"] = serde_json::json!(
                    ms.iter()
                        .map(|m| serde_json::json!({
                            "variables": m.iter().map(|v| var_name(*v)).collect::<Vec<_>>(),
                            "size": m.len(),
                            "triples": m
                                .iter()
                                .map(|v| triple_json(&dag.vars[*v as usize]))
                                .collect::<Vec<_>>(),
                        }))
                        .collect::<Vec<_>>()
                );
                out["polynomial"] = serde_json::json!(polynomial(ms));
                out["truncated"] = serde_json::json!(self.truncated);
                out["monomials_are_minimal_supports"] = serde_json::json!(
                    self.stabilised && !self.truncated
                );
                if self.truncated {
                    out["truncation_means"] = serde_json::json!(
                        "the monomial cap discarded derivations somewhere in this closure, so \
                         this list may be INCOMPLETE and a monomial in it may not be minimal, \
                         because the smaller monomial that would have absorbed it may be one of \
                         the ones dropped at an intermediate fact. Every monomial listed is \
                         still the leaf set of a real derivation. Raise max_monomials, or use \
                         onto_justify, which verifies minimality by re-running the engine"
                    );
                }
            }
            Val::Lineage(l) => match l {
                None => {
                    out["value"] = serde_json::Value::Null;
                    out["contributing"] = serde_json::json!([]);
                }
                Some(set) => {
                    used.extend(set.iter().copied());
                    out["size"] = serde_json::json!(set.len());
                    out["contributing"] = serde_json::json!(
                        set.iter()
                            .map(|v| triple_json(&dag.vars[*v as usize]))
                            .collect::<Vec<_>>()
                    );
                }
            },
            Val::Count(n, sat) => {
                out["value"] = serde_json::json!(n.to_string());
                out["saturated"] = serde_json::json!(sat);
                out["value_is_exact"] = serde_json::json!(self.stabilised && !sat);
                if *sat {
                    out["saturation_means"] = serde_json::json!(
                        "the proof-tree count exceeded a 128-bit counter, so the number above is \
                         the counter's ceiling and not a count. It also makes `stabilised` \
                         meaningless, because saturating arithmetic has a fixed point of its own"
                    );
                } else if !self.stabilised {
                    out["value_means"] = serde_json::json!(
                        "a LOWER BOUND: the number of proof trees of height at most depth_bound. \
                         The iteration had not stopped changing when the bound was reached"
                    );
                }
            }
            Val::Cost(c) => {
                out["value"] = if c.is_finite() {
                    serde_json::json!(c)
                } else {
                    serde_json::Value::Null
                };
                out["zero_is"] = serde_json::json!("infinity, meaning no derivation");
            }
            Val::Trust(t) => {
                out["value"] = serde_json::json!(t);
                out["zero_is"] = serde_json::json!("0.0, meaning no derivation");
            }
        }
        out
    }
}

fn var_name(v: u32) -> String {
    format!("x{}", v + 1)
}

fn triple_json(t: &Spelled) -> serde_json::Value {
    serde_json::json!([t.0, t.1, t.2])
}

/// The canonical presentation of a why-provenance value, `x1*x2 + x3`.
fn polynomial(ms: &[BTreeSet<u32>]) -> String {
    if ms.is_empty() {
        return "0".to_string();
    }
    ms.iter()
        .map(|m| {
            if m.is_empty() {
                "1".to_string()
            } else {
                m.iter().map(|v| var_name(*v)).collect::<Vec<_>>().join("*")
            }
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

// ───────────────────────────────────────────────────────────────────────────
// Blank nodes and target parsing, shared with `crate::justify`
// ───────────────────────────────────────────────────────────────────────────

/// Rewrite `_:label` to the Skolem IRI [`crate::projection_entailment::skolemise`]
/// would give it.
///
/// Both explanation tools reason over a skolemised copy of the store, because
/// every oracle call rebuilds a store from a subset of triples and a blank node
/// does not survive that: the parser is free to relabel it, and a target naming
/// `_:b0` would then name a different node in every re-run.
pub fn skolem_term(term: &str) -> String {
    match term.strip_prefix("_:") {
        Some(label) => format!("<{SKOLEM_PREFIX}{label}>"),
        None => term.to_string(),
    }
}

/// The inverse, for output: a reader asked about `_:b0` and must be answered
/// about `_:b0`.
pub fn unskolem_term(term: &str) -> String {
    let inner = term.strip_prefix('<').and_then(|t| t.strip_suffix('>'));
    match inner.and_then(|i| i.strip_prefix(SKOLEM_PREFIX)) {
        Some(label) => format!("_:{label}"),
        None => term.to_string(),
    }
}

pub fn unskolem_triple(t: &Spelled) -> Spelled {
    (
        unskolem_term(&t.0),
        unskolem_term(&t.1),
        unskolem_term(&t.2),
    )
}

pub fn triple_out(t: &Spelled) -> serde_json::Value {
    let u = unskolem_triple(t);
    serde_json::json!([u.0, u.1, u.2])
}

/// Read one N-Triples line into the spelling the store uses.
///
/// Blank nodes are skolemised BEFORE parsing, so the parser never sees one and
/// never gets the chance to relabel it. The round trip through a real store is
/// what makes the result byte-identical to what `asserted.tsv` would carry: a
/// literal written `"1"^^<http://www.w3.org/2001/XMLSchema#integer>` and one
/// written `"1"^^xsd:integer` must not become two different targets.
pub fn parse_triple(text: &str) -> anyhow::Result<Spelled> {
    let skolemised: String = text
        .split_whitespace()
        .map(skolem_term)
        .collect::<Vec<_>>()
        .join(" ");
    let line = if skolemised.trim_end().ends_with('.') {
        skolemised
    } else {
        format!("{skolemised} .")
    };
    let store = GraphStore::new();
    store.load_ntriples(&line).map_err(|e| {
        anyhow::anyhow!(
            "the target is not one N-Triples triple: {e}. Write it as `<s> <p> <o>` with full \
             IRIs in angle brackets, or a literal in quotes; prefixed names are not read here"
        )
    })?;
    let mut triples = store.all_triples()?;
    match triples.len() {
        1 => Ok(triples.remove(0)),
        0 => anyhow::bail!("the target parsed to no triple at all"),
        n => anyhow::bail!(
            "the target parsed to {n} triples; ask about one conclusion at a time"
        ),
    }
}

// ───────────────────────────────────────────────────────────────────────────
// The report
// ───────────────────────────────────────────────────────────────────────────

pub struct ProvenanceOptions {
    pub profile: String,
    pub semirings: Vec<Semiring>,
    pub depth_bound: usize,
    pub max_monomials: usize,
    /// Weight per asserted triple for `tropical` and `trust`. Absent means
    /// 1.0. Keys are in the store's own N-Triples spelling.
    pub weights: BTreeMap<Spelled, f64>,
}

impl Default for ProvenanceOptions {
    fn default() -> Self {
        Self {
            profile: "owl-rl".to_string(),
            semirings: Semiring::all(),
            depth_bound: DEFAULT_DEPTH_BOUND,
            max_monomials: DEFAULT_MAX_MONOMIALS,
            weights: BTreeMap::new(),
        }
    }
}

/// The sentence that travels with every provenance answer.
pub const RULE_TABLE_CAVEAT: &str =
    "an annotation is a statement about THIS ENGINE's rule table, 29 of OWL 2 RL's 78 rules, and \
     never about OWL 2 RL entailment. Nothing in this report is machine-checked: the Lean layer \
     certifies that a derivation step is sound, and no theorem here says a monomial is minimal or \
     that the list of them is complete. onto_justify verifies a support set by re-running the \
     engine without each of its elements; this tool does algebra.";

/// Annotate the closure and report the target's expression.
pub fn annotate(
    graph: &Arc<GraphStore>,
    target: Option<&str>,
    opts: &ProvenanceOptions,
) -> anyhow::Result<serde_json::Value> {
    let wants_trust = opts.semirings.contains(&Semiring::TrustMaxMin);
    for (t, w) in &opts.weights {
        if *w < 0.0 || !w.is_finite() {
            anyhow::bail!(
                "weight {w} on <{} {} {}> is not a finite non-negative number. min-plus is \
                 absorptive only for non-negative weights, and with a negative one the recursive \
                 fixpoint does not converge; refusing rather than looping",
                t.0,
                t.1,
                t.2
            );
        }
        // Max-min is a semiring on [0, 1] and its multiplicative identity is
        // 1. A weight above 1 is not merely unusual: `min(1, w) = 1`, so the
        // identity stops being an identity and a conjunction of premises comes
        // out SMALLER than the semiring says it should. The answer would be
        // wrong rather than surprising, so it is refused.
        if wants_trust && *w > 1.0 {
            anyhow::bail!(
                "weight {w} on <{} {} {}> is above 1.0 and the `trust` semiring was asked for. \
                 Max-min is a semiring on [0, 1] whose multiplicative identity is 1, so a weight \
                 above it breaks the identity and the annotation would be wrong rather than \
                 merely odd. Use confidences in [0, 1] for `trust`, or drop `trust` and keep \
                 `tropical`, which takes any non-negative weight",
                t.0,
                t.1,
                t.2
            );
        }
    }
    if opts.semirings.is_empty() {
        anyhow::bail!("no semiring was asked for, so there is nothing to compute");
    }

    let (sk, _map) = crate::projection_entailment::skolemise(graph)?;
    let dg = Reasoner::derivation_graph(&sk, &opts.profile)?;
    let dag = Dag::build(&dg);

    let target_triple = match target {
        Some(t) => Some(parse_triple(t)?),
        None => None,
    };

    let mut out = serde_json::json!({
        "profile_used": dg.profile_used,
        "asserted": dag.vars.len(),
        "closure": dag.facts.len(),
        "derived": dag.facts.len().saturating_sub(dag.vars.len()),
        "rule_instances": dg.instances.len(),
        "fixpoint_reached": dg.fixpoint_reached,
        "iterations": dg.iterations,
        "depth_bound": opts.depth_bound,
        "max_monomials": opts.max_monomials,
        "means": RULE_TABLE_CAVEAT,
    });
    if !dg.fixpoint_reached {
        out["fixpoint_warning"] = serde_json::json!(
            "the reasoner stopped at its iteration cap rather than a fixpoint, so the closure is \
             a LOWER BOUND and every annotation below is computed over a partial DAG. Raise \
             reasoner.max_iterations"
        );
    }
    if dag.instances_dropped > 0 {
        out["instances_dropped"] = serde_json::json!(dag.instances_dropped);
        out["instances_dropped_means"] = serde_json::json!(
            "a captured rule instance cited a premise that is not in the closure, which cannot \
             happen on a correct run and means the DAG below is missing hyperedges. Please report \
             this."
        );
    }

    let Some(tt) = target_triple else {
        out["note"] = serde_json::json!(
            "no target triple was given, so this is the shape of the DAG and nothing was \
             annotated. Pass `triple` to get an expression"
        );
        return Ok(out);
    };

    let Some(fid) = dag.id(&tt) else {
        out["target"] = triple_out(&tt);
        out["in_closure"] = serde_json::json!(false);
        out["note"] = serde_json::json!(
            "this engine does not derive that triple under this profile, so its annotation is \
             the semiring zero in every semiring and there is nothing to show. A triple absent \
             from the closure is NOT a non-entailment result: the rule table is 29 of OWL 2 RL's \
             78 rules"
        );
        return Ok(out);
    };

    let cycle = dag.cycle_in_support(fid);
    let mut used: BTreeSet<u32> = BTreeSet::new();
    let mut sems = serde_json::Map::new();
    for &sem in &opts.semirings {
        let ev = evaluate(&dag, sem, opts.depth_bound, opts.max_monomials, &opts.weights);
        sems.insert(sem.name().to_string(), ev.render(sem, &dag, fid, &mut used));
    }

    out["target"] = triple_out(&tt);
    out["in_closure"] = serde_json::json!(true);
    out["target_is_asserted"] = serde_json::json!(dag.var_of[fid as usize].is_some());
    out["derivations_of_target"] = serde_json::json!(dag.bodies_for(fid).len());
    let mut by_rule: BTreeMap<&str, usize> = BTreeMap::new();
    for r in dag.rules_for(fid) {
        *by_rule.entry(r).or_default() += 1;
    }
    out["rules_concluding_target"] = serde_json::json!(by_rule);
    out["cycle_in_support"] = serde_json::json!(cycle.is_some());
    if let Some(c) = &cycle {
        out["cycle_witness"] = triple_out(c);
        out["cycle_means"] = serde_json::json!(
            "the target's own support contains a cycle, so it has proof trees of unbounded \
             height and the counting semiring CANNOT be exact here whatever depth_bound is set \
             to. The absorptive semirings are unaffected: a derivation through the cycle is \
             absorbed by the one that does not take it"
        );
    }
    out["variables"] = serde_json::json!(
        used.iter()
            .map(|v| (var_name(*v), triple_out(&dag.vars[*v as usize])))
            .collect::<serde_json::Map<String, serde_json::Value>>()
    );
    out["semirings"] = serde_json::Value::Object(sems);
    Ok(out)
}
