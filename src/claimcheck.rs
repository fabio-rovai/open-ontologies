//! Compiled claim verification: the Tardygrada Layer 3 hot path.
//!
//! Design and measurements: `docs/layer3-compiled-reasoning.md`.
//!
//! # The shape
//!
//! A reasoner runs ONCE, offline, and classifies the ontology. We store the
//! *inferred* hierarchy plus the *asserted* disjointness axioms. At query time
//! there is no reasoning, only the two-hop join:
//!
//! ```text
//! A ⊓ B unsatisfiable  if  ∃ A' ⊒ A, B' ⊒ B  with  disjoint(A', B')
//! ```
//!
//! # How the join is executed: token bitsets, not store probes
//!
//! The first Rust implementation ran the join as nested `quads_for_pattern`
//! probes against an Oxigraph store: O(|supers(A)| × |supers(B)|) indexed
//! lookups per pair, each allocating and validating IRIs. Correct, and already
//! 140x faster than a warm reasoner — but the compiled data is *static*, so
//! the mature move is to finish the compilation:
//!
//! * every IRI is interned to a `u32` once, at index build;
//! * every asserted disjointness axiom `disjoint(X_k, Y_k)` gets a token `k`;
//! * every class `C` gets two bitsets over tokens:
//!   `L(C) = { k : X_k ⊒ C }` and `R(C) = { k : Y_k ⊒ C }`.
//!
//! Then:
//!
//! ```text
//! A ⊓ B unsatisfiable  ⟺  L(A) ∧ R(B) ≠ ∅   or   R(A) ∧ L(B) ≠ ∅
//! ```
//!
//! Two bitwise ANDs over `⌈m/64⌉` words, where `m` is the number of
//! disjointness axioms (Pizza: 398 axioms → 7 words). No allocation, no string
//! comparison, no store. The hierarchy is consulted only at build time, when
//! the token bitsets are computed from the inferred ancestor sets. The first
//! set bit in the intersection is the *witness* axiom, which is exactly what
//! the explainable verdict needs.
//!
//! # Soundness and completeness, measured
//!
//! Verified against a brute-force HermiT matrix over 78,884 pairs across 12
//! ORE ontologies plus Pizza, and against 793 structurally adversarial claims
//! over 8 ontologies:
//!
//! * **Sound**: 0 cases where the join rejected a satisfiable pair. A `Rejected`
//!   verdict from this module is therefore trustworthy.
//! * **Incomplete**: catches 94.6% of true incompatibilities in aggregate
//!   (98.1% on Pizza, but as low as 25.9% on one ontology). The residual needs
//!   a reasoner call, because those contradictions follow from property
//!   restrictions rather than from subsumption plus disjointness.
//!
//! That asymmetry drives the API. "The join found nothing" is NOT evidence of
//! consistency, so it yields [`Verdict::Undetermined`], never
//! [`Verdict::Consistent`], unless the residual is discharged by tier 2
//! ([`CompiledOntology::check_with_oracle`]) or explicitly confirmed.
//!
//! # Parallelism
//!
//! Claims are independent, so batches parallelise trivially:
//! [`CompiledOntology::check_batch`] fans out over rayon. Per-claim parallelism
//! would be counterproductive at these latencies — the work per claim is
//! microseconds, far below fork/join overhead.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, RwLock};

use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// A candidate claim, already decomposed into triples by Layer 1.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Claim {
    /// `(individual, class)` type assertions.
    pub types: Vec<(String, String)>,
    /// `(subject, property, object)` relation assertions.
    pub rels: Vec<(String, String, String)>,
}

/// Which check fired, and the evidence that made it fire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub check: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    /// Every check passed AND the residual (the pairs the join cannot decide)
    /// has been discharged. Only reachable via
    /// [`CompiledOntology::check_with_oracle`] or
    /// [`CheckResult::confirm_residual`].
    Consistent,
    /// A check fired. Sound: verified against a reasoner over 78,884 pairs.
    Rejected,
    /// Nothing fired, but the join is incomplete, so nothing is proven.
    Undetermined,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub verdict: Verdict,
    pub violations: Vec<Violation>,
    /// Assumption-based findings. A warning means an ASSUMED (not entailed)
    /// disjointness axiom fired: the claim conflicts with vetted domain
    /// knowledge that the ontology itself does not assert. Warnings never
    /// change the verdict.
    #[serde(default)]
    pub warnings: Vec<Violation>,
    /// Class pairs the join could not settle. A caller wanting `Consistent`
    /// must discharge these with a reasoner.
    pub residual_pairs: Vec<(String, String)>,
    pub elapsed_us: u128,
}

impl CheckResult {
    /// Upgrade `Undetermined` to `Consistent` once the residual pairs have
    /// been discharged by a reasoner (tier 2). Never downgrades a rejection.
    pub fn confirm_residual(mut self) -> Self {
        if self.verdict == Verdict::Undetermined && self.violations.is_empty() {
            self.verdict = Verdict::Consistent;
            self.residual_pairs.clear();
        }
        self
    }
}

// ── bitset primitives ───────────────────────────────────────────────────

/// A fixed-width bitset over disjointness tokens. Plain `Vec<u64>`; the widths
/// involved (one bit per disjointness axiom) do not justify a dependency.
type Bits = Vec<u64>;

fn bits_new(nbits: usize) -> Bits {
    vec![0u64; nbits.div_ceil(64)]
}

fn bits_set(b: &mut Bits, i: usize) {
    b[i / 64] |= 1u64 << (i % 64);
}

/// First common set bit of `a ∧ b`, if any. This is the whole join.
fn bits_first_common(a: &Bits, b: &Bits) -> Option<usize> {
    for (w, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        let and = x & y;
        if and != 0 {
            return Some(w * 64 + and.trailing_zeros() as usize);
        }
    }
    None
}

// ── the compiled index ──────────────────────────────────────────────────

/// Raw load data, kept so the index can be (re)built after incremental loads.
#[derive(Default)]
struct RawData {
    subsumptions: Vec<(String, String)>,
    disjoint: Vec<(String, String)>,
    /// Disjointness ASSUMED (e.g. LLM-proposed, reasoner-vetted), not entailed.
    /// Drives warnings, never rejections.
    assumed_disjoint: Vec<(String, String)>,
    declared_classes: HashSet<String>,
    declared_props: HashSet<String>,
    functional: HashSet<String>,
}

impl RawData {
    fn is_empty(&self) -> bool {
        self.subsumptions.is_empty()
            && self.disjoint.is_empty()
            && self.assumed_disjoint.is_empty()
            && self.declared_classes.is_empty()
            && self.declared_props.is_empty()
            && self.functional.is_empty()
    }
}

/// The immutable query structure. Built once per load generation; shared
/// across threads via `Arc`, so batch checking needs no locks on the hot path.
/// Token bitsets for one set of disjointness axioms.
struct TokenSet {
    pair_left: Vec<u32>,
    pair_right: Vec<u32>,
    l_tokens: Vec<Bits>,
    r_tokens: Vec<Bits>,
}

impl TokenSet {
    fn build(pairs: &[(String, String)], ids: &HashMap<String, u32>,
             descendants: &[Vec<u32>], n: usize) -> Self {
        let m = pairs.len();
        let mut pair_left = Vec::with_capacity(m);
        let mut pair_right = Vec::with_capacity(m);
        let mut l_tokens: Vec<Bits> = (0..n).map(|_| bits_new(m)).collect();
        let mut r_tokens: Vec<Bits> = (0..n).map(|_| bits_new(m)).collect();
        for (k, (x, y)) in pairs.iter().enumerate() {
            let (xi, yi) = (ids[x.as_str()], ids[y.as_str()]);
            pair_left.push(xi);
            pair_right.push(yi);
            for &c in &descendants[xi as usize] {
                bits_set(&mut l_tokens[c as usize], k);
            }
            for &c in &descendants[yi as usize] {
                bits_set(&mut r_tokens[c as usize], k);
            }
        }
        Self { pair_left, pair_right, l_tokens, r_tokens }
    }

    /// Two-hop join over this token set. Returns the witnessing axiom.
    fn incompatible<'a>(&'a self, names: &'a [String], ai: usize, bi: usize)
        -> Option<(&'a str, &'a str)> {
        if let Some(k) = bits_first_common(&self.l_tokens[ai], &self.r_tokens[bi]) {
            return Some((names[self.pair_left[k] as usize].as_str(),
                         names[self.pair_right[k] as usize].as_str()));
        }
        if let Some(k) = bits_first_common(&self.r_tokens[ai], &self.l_tokens[bi]) {
            return Some((names[self.pair_right[k] as usize].as_str(),
                         names[self.pair_left[k] as usize].as_str()));
        }
        None
    }
}

struct Index {
    /// IRI → interned id, for classes only.
    ids: HashMap<String, u32>,
    /// id → IRI, for witness reporting.
    names: Vec<String>,
    /// Entailed disjointness (asserted + soundly derived). Drives rejections.
    hard: TokenSet,
    /// Assumed disjointness (proposed + vetted, not entailed). Drives warnings.
    assumed: TokenSet,
}

impl Index {
    fn build(raw: &RawData) -> Self {
        // Intern every class mentioned anywhere.
        let mut ids: HashMap<String, u32> = HashMap::new();
        let mut names: Vec<String> = Vec::new();
        let intern = |s: &str, ids: &mut HashMap<String, u32>, names: &mut Vec<String>| {
            if let Some(&i) = ids.get(s) {
                return i;
            }
            let i = names.len() as u32;
            ids.insert(s.to_string(), i);
            names.push(s.to_string());
            i
        };
        for (a, b) in raw
            .subsumptions
            .iter()
            .chain(raw.disjoint.iter())
            .chain(raw.assumed_disjoint.iter())
        {
            intern(a, &mut ids, &mut names);
            intern(b, &mut ids, &mut names);
        }
        for c in &raw.declared_classes {
            intern(c, &mut ids, &mut names);
        }
        let n = names.len();

        // Ancestor sets from the (already inferred, already transitive) load
        // data, made reflexive defensively.
        let mut ancestors: Vec<HashSet<u32>> = (0..n).map(|i| HashSet::from([i as u32])).collect();
        for (sub, sup) in &raw.subsumptions {
            let (s, p) = (ids[sub.as_str()], ids[sup.as_str()]);
            ancestors[s as usize].insert(p);
        }
        // Invert: descendants(x) = every class with x among its ancestors.
        let mut descendants: Vec<Vec<u32>> = vec![Vec::new(); n];
        for (c, ancs) in ancestors.iter().enumerate() {
            for &a in ancs {
                descendants[a as usize].push(c as u32);
            }
        }

        // Tokenise both axiom sets and stamp bitsets down the hierarchies.
        let hard = TokenSet::build(&raw.disjoint, &ids, &descendants, n);
        let assumed = TokenSet::build(&raw.assumed_disjoint, &ids, &descendants, n);

        Self { ids, names, hard, assumed }
    }

    /// The entailed two-hop join. `Some` means PROVEN incompatible; drives
    /// rejections. Symmetric by construction: both orientations are tested.
    fn incompatible(&self, a: &str, b: &str) -> Option<(&str, &str)> {
        let (&ai, &bi) = (self.ids.get(a)?, self.ids.get(b)?);
        self.hard.incompatible(&self.names, ai as usize, bi as usize)
    }

    /// The assumed two-hop join. `Some` means the pair conflicts with a
    /// VETTED ASSUMPTION, not an entailment; drives warnings only.
    fn assumed_incompatible(&self, a: &str, b: &str) -> Option<(&str, &str)> {
        let (&ai, &bi) = (self.ids.get(a)?, self.ids.get(b)?);
        self.assumed.incompatible(&self.names, ai as usize, bi as usize)
    }
}

/// The compiled ontology.
///
/// Loads accumulate raw data; the query index is built lazily on first check
/// and invalidated by further loads (including tier-2 learning, which is how a
/// learned incompatibility becomes a tier-1 answer). The index itself is
/// immutable and `Arc`-shared, so concurrent checking is lock-free once built.
pub struct CompiledOntology {
    raw: RwLock<RawData>,
    index: RwLock<Option<Arc<Index>>>,
    /// Pairs tier 2 has confirmed compatible, in canonical order. Kept outside
    /// the index so learning them does not force a rebuild.
    known_compatible: RwLock<HashSet<(String, String)>>,
}

fn canon(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    }
}

impl CompiledOntology {
    pub fn new() -> Result<Self> {
        Ok(Self {
            raw: RwLock::new(RawData::default()),
            index: RwLock::new(None),
            known_compatible: RwLock::new(HashSet::new()),
        })
    }

    fn invalidate(&self) {
        *self.index.write().unwrap() = None;
    }

    fn index(&self) -> Arc<Index> {
        if let Some(ix) = self.index.read().unwrap().as_ref() {
            return Arc::clone(ix);
        }
        let mut slot = self.index.write().unwrap();
        // Double-checked: another thread may have built it while we waited.
        if let Some(ix) = slot.as_ref() {
            return Arc::clone(ix);
        }
        let built = Arc::new(Index::build(&self.raw.read().unwrap()));
        *slot = Some(Arc::clone(&built));
        built
    }

    // ── loading the compile output ──────────────────────────────────────

    /// Inferred subsumptions, `(sub, sup)`. The compile step emits the full
    /// inferred closure including reflexive pairs; reflexivity is enforced
    /// here anyway, since the join relies on `A ⊒ A`.
    pub fn load_subsumptions(&self, pairs: &[(String, String)]) -> Result<usize> {
        self.raw.write().unwrap().subsumptions.extend_from_slice(pairs);
        self.invalidate();
        Ok(pairs.len())
    }

    /// Asserted disjointness axioms. Orientation does not matter: the join
    /// tests both directions.
    pub fn load_disjoint(&self, pairs: &[(String, String)]) -> Result<usize> {
        self.raw.write().unwrap().disjoint.extend_from_slice(pairs);
        self.invalidate();
        Ok(pairs.len())
    }

    /// Load ASSUMED disjointness: pairs proposed (e.g. by an LLM) and vetted
    /// by a reasoner as admissible — consistent with the ontology but not
    /// entailed by it. These drive warnings, never rejections; a deployment
    /// on a zero-disjointness ontology gets its verification value from
    /// vocabulary checks plus this tier.
    pub fn load_assumed_disjoint(&self, pairs: &[(String, String)]) -> Result<usize> {
        self.raw.write().unwrap().assumed_disjoint.extend_from_slice(pairs);
        self.invalidate();
        Ok(pairs.len())
    }

    pub fn load_declared_classes(&self, iris: &[String]) -> Result<usize> {
        self.raw.write().unwrap().declared_classes.extend(iris.iter().cloned());
        self.invalidate();
        Ok(iris.len())
    }

    pub fn load_declared_props(&self, iris: &[String]) -> Result<usize> {
        self.raw.write().unwrap().declared_props.extend(iris.iter().cloned());
        self.invalidate();
        Ok(iris.len())
    }

    pub fn load_functional(&self, iris: &[String]) -> Result<usize> {
        self.raw.write().unwrap().functional.extend(iris.iter().cloned());
        self.invalidate();
        Ok(iris.len())
    }

    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.raw.read().unwrap().is_empty())
    }

    // ── the hot path ────────────────────────────────────────────────────

    pub fn check(&self, claim: &Claim) -> Result<CheckResult> {
        // An empty store proves nothing about anything.
        if self.is_empty()? {
            return Ok(CheckResult {
                verdict: Verdict::Undetermined,
                violations: Vec::new(),
                warnings: Vec::new(),
                residual_pairs: Vec::new(),
                elapsed_us: 0,
            });
        }
        let ix = self.index();
        let raw = self.raw.read().unwrap();
        Ok(Self::check_inner(&raw, &ix, claim))
    }

    /// The lock-free hot path. Everything it touches is an immutable borrow,
    /// so batches can fan out over threads with zero contention — the locks
    /// are acquired ONCE per batch, not once per claim. Infallible by
    /// construction: interning happened at index build, so there is no IRI
    /// validation or store access left to fail.
    fn check_inner(raw: &RawData, ix: &Index, claim: &Claim) -> CheckResult {
        let start = std::time::Instant::now();
        let mut violations = Vec::new();
        let mut warnings = Vec::new();
        let mut residual = Vec::new();

        // 1. Closed-world vocabulary. Open-world OWL structurally cannot flag
        //    an invented term; this can, and for LLM output it is the check
        //    that fires most often. One hash lookup per term.
        for (_, cls) in &claim.types {
            if !raw.declared_classes.contains(cls) {
                violations.push(Violation {
                    check: "unknown_class".into(),
                    detail: format!("class not declared in ontology: {cls}"),
                });
            }
        }
        for (_, prop, _) in &claim.rels {
            if !raw.declared_props.contains(prop) {
                violations.push(Violation {
                    check: "unknown_property".into(),
                    detail: format!("property not declared in ontology: {prop}"),
                });
            }
        }

        // 2. Incompatibility between types asserted of one individual: two
        //    bitset ANDs per pair, witness axiom extracted for the detail.
        let mut by_subject: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for (subj, cls) in &claim.types {
            by_subject.entry(subj).or_default().push(cls);
        }
        for (subj, classes) in &by_subject {
            for (i, a) in classes.iter().enumerate() {
                for b in &classes[i + 1..] {
                    if let Some((wa, wb)) = ix.incompatible(a, b) {
                        violations.push(Violation {
                            check: "disjointness".into(),
                            detail: format!(
                                "{subj} cannot be both {a} and {b}: {wa} is declared \
                                 disjoint with {wb}"
                            ),
                        });
                    } else {
                        // Not PROVEN incompatible. Probe the assumption tier:
                        // a hit is domain knowledge worth surfacing, but it is
                        // not proof, so the verdict is untouched and the pair
                        // still goes to the residual.
                        if let Some((wa, wb)) = ix.assumed_incompatible(a, b) {
                            warnings.push(Violation {
                                check: "disjointness_assumed".into(),
                                detail: format!(
                                    "{subj} being both {a} and {b} conflicts with the                                      ASSUMED disjointness of {wa} and {wb} (vetted                                      assumption, not entailed by the ontology)"
                                ),
                            });
                        }
                        residual.push((a.to_string(), b.to_string()));
                    }
                }
            }
        }

        // 3. Functional property given two distinct values.
        let mut seen: BTreeMap<(&str, &str), Vec<&str>> = BTreeMap::new();
        for (s, p, o) in &claim.rels {
            seen.entry((s, p)).or_default().push(o);
        }
        for ((s, p), objs) in &seen {
            let mut d: Vec<&&str> = objs.iter().collect();
            d.sort();
            d.dedup();
            if d.len() > 1 && raw.functional.contains(*p) {
                violations.push(Violation {
                    check: "functionality".into(),
                    detail: format!(
                        "{s} has {} distinct values for functional property {p}",
                        d.len()
                    ),
                });
            }
        }

        let verdict = if !violations.is_empty() {
            Verdict::Rejected
        } else {
            // Nothing fired. Because the join is incomplete this is NOT a
            // clean bill of health. The caller must discharge `residual_pairs`
            // before treating the claim as consistent.
            Verdict::Undetermined
        };

        CheckResult {
            verdict,
            violations,
            warnings,
            residual_pairs: residual,
            elapsed_us: start.elapsed().as_micros(),
        }
    }

    /// Check a batch of claims in parallel. Claims are independent, so this is
    /// a straight rayon fan-out over the shared immutable index; there is no
    /// lock contention on the hot path.
    pub fn check_batch(&self, claims: &[Claim]) -> Result<Vec<CheckResult>> {
        if self.is_empty()? {
            // Same semantics as check() on an empty store, per claim.
            return Ok(claims
                .iter()
                .map(|_| CheckResult {
                    verdict: Verdict::Undetermined,
                    violations: Vec::new(),
                    warnings: Vec::new(),
                    residual_pairs: Vec::new(),
                    elapsed_us: 0,
                })
                .collect());
        }
        // Acquire the index and the raw-data read lock ONCE, then fan out a
        // pure function. The first version of this method called self.check()
        // per item, which meant two RwLock acquisitions per claim; at ~0.35 us
        // of real work per claim the resulting cache-line ping-pong made the
        // parallel path 3x SLOWER than sequential. Locks per batch, not per
        // claim.
        let ix = self.index();
        let raw = self.raw.read().unwrap();
        Ok(claims
            .par_iter()
            .map(|c| Self::check_inner(&raw, &ix, c))
            .collect())
    }
}

// ── Tier 2: the residual oracle ─────────────────────────────────────────

/// Settles class pairs the two-hop join cannot decide.
///
/// The join is sound but incomplete: measured recall is 94.6% of true
/// incompatibilities across 13 ontologies, and as low as 25.9% on one. The
/// residual arises from contradictions that follow from property restrictions
/// rather than from subsumption plus disjointness, so deciding them needs a
/// real reasoner.
///
/// Implementations are expected to be expensive (milliseconds) and are
/// therefore consulted only for pairs the join leaves open, and only once per
/// pair thanks to [`CompiledOntology::learn_incompatible`] /
/// [`CompiledOntology::learn_compatible`].
pub trait ResidualOracle {
    /// `Some(true)` = compatible, `Some(false)` = incompatible,
    /// `None` = the oracle could not decide either.
    ///
    /// Returning `None` must never be treated as "compatible": it propagates
    /// to [`Verdict::Undetermined`].
    fn compatible(&self, a: &str, b: &str) -> Result<Option<bool>>;
}

impl CompiledOntology {
    /// Record an oracle verdict of "incompatible" so the join settles it next
    /// time. Stored as a direct disjointness axiom, which is sound: the pair
    /// IS disjoint, we simply could not derive it structurally. Invalidates
    /// the index, so the next check absorbs it into the token bitsets.
    pub fn learn_incompatible(&self, a: &str, b: &str) -> Result<()> {
        self.load_disjoint(&[(a.to_string(), b.to_string())])?;
        Ok(())
    }

    /// Record an oracle verdict of "compatible" so the pair is not re-asked.
    /// Kept outside the index: learning compatibility must not force a
    /// rebuild.
    pub fn learn_compatible(&self, a: &str, b: &str) -> Result<()> {
        self.known_compatible.write().unwrap().insert(canon(a, b));
        Ok(())
    }

    fn known_compatible(&self, a: &str, b: &str) -> Result<bool> {
        Ok(self.known_compatible.read().unwrap().contains(&canon(a, b)))
    }

    /// Full two-tier check. Tier 1 is the join; anything it leaves open goes to
    /// `oracle`, and the answer is cached back.
    ///
    /// Returns [`Verdict::Consistent`] only when every residual pair was
    /// positively confirmed compatible. An oracle that returns `None` leaves
    /// the verdict [`Verdict::Undetermined`], never `Consistent`.
    pub fn check_with_oracle<O: ResidualOracle>(
        &self,
        claim: &Claim,
        oracle: &O,
    ) -> Result<CheckResult> {
        let mut r = self.check(claim)?;
        if r.verdict == Verdict::Rejected {
            return Ok(r); // Tier 1 rejections are sound; do not spend an oracle call.
        }

        let residual = std::mem::take(&mut r.residual_pairs);
        let mut undecided = Vec::new();
        for (a, b) in residual {
            if self.known_compatible(&a, &b)? {
                continue;
            }
            match oracle.compatible(&a, &b)? {
                Some(true) => self.learn_compatible(&a, &b)?,
                Some(false) => {
                    self.learn_incompatible(&a, &b)?;
                    r.violations.push(Violation {
                        check: "disjointness_tier2".into(),
                        detail: format!(
                            "{a} and {b} are incompatible; established by reasoner, not \
                             derivable from subsumption plus disjointness"
                        ),
                    });
                }
                None => undecided.push((a, b)),
            }
        }

        r.residual_pairs = undecided;
        r.verdict = if !r.violations.is_empty() {
            Verdict::Rejected
        } else if r.residual_pairs.is_empty() {
            Verdict::Consistent
        } else {
            Verdict::Undetermined
        };
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PIZZA: &str = "http://www.co-ode.org/ontologies/pizza/pizza.owl#";

    fn iri(l: &str) -> String {
        format!("{PIZZA}{l}")
    }

    /// Mirrors the real compile output: the INFERRED hierarchy plus ASSERTED
    /// disjointness. Margherita is a VegetarianPizza only by inference, and
    /// that is exactly what makes the two-hop join work where a syntactic
    /// subclass walk fails.
    fn fixture() -> CompiledOntology {
        let c = CompiledOntology::new().unwrap();
        c.load_declared_classes(&[
            iri("Margherita"), iri("MeatyPizza"), iri("VegetarianPizza"),
            iri("CheeseyPizza"), iri("American"), iri("AmericanHot"), iri("NamedPizza"),
        ]).unwrap();
        c.load_declared_props(&[iri("hasBase"), iri("hasTopping")]).unwrap();
        c.load_subsumptions(&[
            // reflexive
            (iri("Margherita"), iri("Margherita")),
            (iri("MeatyPizza"), iri("MeatyPizza")),
            (iri("VegetarianPizza"), iri("VegetarianPizza")),
            (iri("American"), iri("American")),
            (iri("AmericanHot"), iri("AmericanHot")),
            (iri("CheeseyPizza"), iri("CheeseyPizza")),
            // inferred
            (iri("Margherita"), iri("VegetarianPizza")),
            (iri("Margherita"), iri("CheeseyPizza")),
            (iri("Margherita"), iri("NamedPizza")),
            (iri("American"), iri("MeatyPizza")),
            (iri("American"), iri("NamedPizza")),
            (iri("AmericanHot"), iri("MeatyPizza")),
            (iri("AmericanHot"), iri("NamedPizza")),
        ]).unwrap();
        c.load_disjoint(&[
            (iri("VegetarianPizza"), iri("MeatyPizza")),
            (iri("American"), iri("AmericanHot")),
        ]).unwrap();
        c.load_functional(&[iri("hasBase")]).unwrap();
        c
    }

    #[test]
    fn empty_store_never_claims_consistency() {
        let c = CompiledOntology::new().unwrap();
        let claim = Claim { types: vec![("x".into(), iri("Margherita"))], ..Default::default() };
        assert_eq!(c.check(&claim).unwrap().verdict, Verdict::Undetermined);
    }

    /// The case a syntactic subclass walk misses. Margherita and MeatyPizza
    /// have no direct disjointness axiom; the clash is only reachable through
    /// the INFERRED Margherita ⊑ VegetarianPizza.
    #[test]
    fn two_hop_join_catches_inferred_disjointness() {
        let c = fixture();
        let claim = Claim {
            types: vec![("x".into(), iri("Margherita")), ("x".into(), iri("MeatyPizza"))],
            ..Default::default()
        };
        let r = c.check(&claim).unwrap();
        assert_eq!(r.verdict, Verdict::Rejected, "{r:?}");
        assert_eq!(r.violations[0].check, "disjointness");
        // The witness axiom must be named in the detail.
        assert!(r.violations[0].detail.contains("VegetarianPizza"), "{r:?}");
    }

    #[test]
    fn join_is_symmetric() {
        let c = fixture();
        let claim = Claim {
            types: vec![("x".into(), iri("MeatyPizza")), ("x".into(), iri("Margherita"))],
            ..Default::default()
        };
        assert_eq!(c.check(&claim).unwrap().verdict, Verdict::Rejected);
    }

    #[test]
    fn both_sides_climb_the_hierarchy() {
        // American ⊑ MeatyPizza and Margherita ⊑ VegetarianPizza, and the
        // disjointness sits between the two SUPERclasses. Neither side alone
        // is enough; the join has to climb both.
        let c = fixture();
        let claim = Claim {
            types: vec![("x".into(), iri("Margherita")), ("x".into(), iri("American"))],
            ..Default::default()
        };
        assert_eq!(c.check(&claim).unwrap().verdict, Verdict::Rejected);
    }

    /// The soundness/completeness asymmetry, pinned. A clean join is NOT a
    /// clean bill of health, because the join misses ~5% of contradictions.
    #[test]
    fn clean_join_is_undetermined_not_consistent() {
        let c = fixture();
        let claim = Claim {
            types: vec![("x".into(), iri("Margherita")), ("x".into(), iri("CheeseyPizza"))],
            ..Default::default()
        };
        let r = c.check(&claim).unwrap();
        assert_eq!(r.verdict, Verdict::Undetermined, "{r:?}");
        assert!(r.violations.is_empty());
        assert_eq!(r.residual_pairs.len(), 1, "the undecided pair must be reported");
    }

    #[test]
    fn residual_can_be_discharged_by_tier_two() {
        let c = fixture();
        let claim = Claim {
            types: vec![("x".into(), iri("Margherita")), ("x".into(), iri("CheeseyPizza"))],
            ..Default::default()
        };
        let r = c.check(&claim).unwrap().confirm_residual();
        assert_eq!(r.verdict, Verdict::Consistent);
        assert!(r.residual_pairs.is_empty());
    }

    #[test]
    fn confirming_residual_never_rescues_a_rejection() {
        let c = fixture();
        let claim = Claim {
            types: vec![("x".into(), iri("Margherita")), ("x".into(), iri("MeatyPizza"))],
            ..Default::default()
        };
        let r = c.check(&claim).unwrap().confirm_residual();
        assert_eq!(r.verdict, Verdict::Rejected, "a rejection must survive confirmation");
    }

    #[test]
    fn hallucinated_class_is_caught() {
        let c = fixture();
        let claim = Claim {
            types: vec![("x".into(), iri("QuantumPineappleTopping"))],
            ..Default::default()
        };
        let r = c.check(&claim).unwrap();
        assert_eq!(r.verdict, Verdict::Rejected);
        assert_eq!(r.violations[0].check, "unknown_class");
    }

    #[test]
    fn hallucinated_property_is_caught() {
        let c = fixture();
        let claim = Claim {
            types: vec![("x".into(), iri("Margherita"))],
            rels: vec![("x".into(), iri("hasVibes"), "y".into())],
        };
        let r = c.check(&claim).unwrap();
        assert_eq!(r.verdict, Verdict::Rejected);
        assert_eq!(r.violations[0].check, "unknown_property");
    }

    #[test]
    fn functional_property_with_two_values_is_caught() {
        let c = fixture();
        let claim = Claim {
            types: vec![("x".into(), iri("Margherita"))],
            rels: vec![
                ("x".into(), iri("hasBase"), "b1".into()),
                ("x".into(), iri("hasBase"), "b2".into()),
            ],
        };
        let r = c.check(&claim).unwrap();
        assert!(r.violations.iter().any(|v| v.check == "functionality"), "{r:?}");
    }

    #[test]
    fn non_functional_property_with_two_values_is_fine() {
        let c = fixture();
        let claim = Claim {
            types: vec![("x".into(), iri("Margherita"))],
            rels: vec![
                ("x".into(), iri("hasTopping"), "t1".into()),
                ("x".into(), iri("hasTopping"), "t2".into()),
            ],
        };
        let r = c.check(&claim).unwrap();
        assert!(r.violations.is_empty(), "{r:?}");
    }

    // ── tier 2 ──────────────────────────────────────────────────────────

    /// An oracle that knows one extra incompatibility the join cannot derive,
    /// modelling the property-restriction cases (e.g. American + SpicyPizza).
    struct MockOracle {
        incompatible: Vec<(String, String)>,
        undecidable: Vec<(String, String)>,
        pub calls: std::cell::RefCell<usize>,
    }

    impl ResidualOracle for MockOracle {
        fn compatible(&self, a: &str, b: &str) -> Result<Option<bool>> {
            *self.calls.borrow_mut() += 1;
            let hit = |v: &Vec<(String, String)>| {
                v.iter().any(|(x, y)| (x == a && y == b) || (x == b && y == a))
            };
            if hit(&self.undecidable) {
                return Ok(None);
            }
            Ok(Some(!hit(&self.incompatible)))
        }
    }

    fn oracle(incompatible: Vec<(String, String)>, undecidable: Vec<(String, String)>) -> MockOracle {
        MockOracle { incompatible, undecidable, calls: std::cell::RefCell::new(0) }
    }

    #[test]
    fn tier2_catches_what_the_join_misses() {
        let c = fixture();
        let o = oracle(vec![(iri("Margherita"), iri("CheeseyPizza"))], vec![]);
        let claim = Claim {
            types: vec![("x".into(), iri("Margherita")), ("x".into(), iri("CheeseyPizza"))],
            ..Default::default()
        };
        // Tier 1 alone cannot see this.
        assert_eq!(c.check(&claim).unwrap().verdict, Verdict::Undetermined);
        let r = c.check_with_oracle(&claim, &o).unwrap();
        assert_eq!(r.verdict, Verdict::Rejected, "{r:?}");
        assert_eq!(r.violations[0].check, "disjointness_tier2");
    }

    #[test]
    fn tier2_confirms_consistency() {
        let c = fixture();
        let o = oracle(vec![], vec![]);
        let claim = Claim {
            types: vec![("x".into(), iri("Margherita")), ("x".into(), iri("CheeseyPizza"))],
            ..Default::default()
        };
        assert_eq!(c.check_with_oracle(&claim, &o).unwrap().verdict, Verdict::Consistent);
    }

    #[test]
    fn undecidable_oracle_yields_undetermined_not_consistent() {
        let c = fixture();
        let o = oracle(vec![], vec![(iri("Margherita"), iri("CheeseyPizza"))]);
        let claim = Claim {
            types: vec![("x".into(), iri("Margherita")), ("x".into(), iri("CheeseyPizza"))],
            ..Default::default()
        };
        let r = c.check_with_oracle(&claim, &o).unwrap();
        assert_eq!(r.verdict, Verdict::Undetermined, "an undecided oracle must not yield Consistent");
        assert_eq!(r.residual_pairs.len(), 1);
    }

    #[test]
    fn oracle_verdicts_are_cached() {
        let c = fixture();
        let o = oracle(vec![], vec![]);
        let claim = Claim {
            types: vec![("x".into(), iri("Margherita")), ("x".into(), iri("CheeseyPizza"))],
            ..Default::default()
        };
        c.check_with_oracle(&claim, &o).unwrap();
        let after_first = *o.calls.borrow();
        c.check_with_oracle(&claim, &o).unwrap();
        assert_eq!(*o.calls.borrow(), after_first, "second check must hit the cache");
    }

    #[test]
    fn tier1_rejection_spends_no_oracle_call() {
        let c = fixture();
        let o = oracle(vec![], vec![]);
        let claim = Claim {
            types: vec![("x".into(), iri("Margherita")), ("x".into(), iri("MeatyPizza"))],
            ..Default::default()
        };
        let r = c.check_with_oracle(&claim, &o).unwrap();
        assert_eq!(r.verdict, Verdict::Rejected);
        assert_eq!(*o.calls.borrow(), 0, "a sound tier-1 rejection must not consult tier 2");
    }

    #[test]
    fn learned_incompatibility_is_settled_by_tier1_next_time() {
        let c = fixture();
        let o = oracle(vec![(iri("Margherita"), iri("CheeseyPizza"))], vec![]);
        let claim = Claim {
            types: vec![("x".into(), iri("Margherita")), ("x".into(), iri("CheeseyPizza"))],
            ..Default::default()
        };
        c.check_with_oracle(&claim, &o).unwrap();
        // Tier 1 alone should now settle it, with no oracle involved.
        assert_eq!(c.check(&claim).unwrap().verdict, Verdict::Rejected);
    }


    // ── assumed-disjointness WARN tier ──────────────────────────────────

    #[test]
    fn assumed_disjointness_warns_but_never_rejects() {
        let c = fixture();
        // Vetted assumption: Margherita and CheeseyPizza "should" be disjoint
        // (deliberately false in reality — assumptions are not entailments,
        // and the machinery must treat them accordingly).
        c.load_assumed_disjoint(&[(iri("Margherita"), iri("CheeseyPizza"))]).unwrap();
        let claim = Claim {
            types: vec![("x".into(), iri("Margherita")), ("x".into(), iri("CheeseyPizza"))],
            ..Default::default()
        };
        let r = c.check(&claim).unwrap();
        assert_eq!(r.verdict, Verdict::Undetermined, "assumption must not reject: {r:?}");
        assert_eq!(r.warnings.len(), 1, "{r:?}");
        assert_eq!(r.warnings[0].check, "disjointness_assumed");
        assert!(r.warnings[0].detail.contains("not entailed"));
        // The pair still needs tier 2: nothing was proven.
        assert_eq!(r.residual_pairs.len(), 1);
    }

    #[test]
    fn assumed_tier_climbs_the_hierarchy() {
        let c = fixture();
        // Assumption at superclass level must warn for subclasses.
        c.load_assumed_disjoint(&[(iri("NamedPizza"), iri("CheeseyPizza"))]).unwrap();
        let claim = Claim {
            // Margherita ⊑ NamedPizza (inferred), so the assumption applies.
            types: vec![("x".into(), iri("Margherita")), ("x".into(), iri("CheeseyPizza"))],
            ..Default::default()
        };
        let r = c.check(&claim).unwrap();
        assert_eq!(r.warnings.len(), 1, "{r:?}");
        assert!(r.warnings[0].detail.contains("NamedPizza"), "witness must name the assumed axiom: {r:?}");
    }

    #[test]
    fn hard_rejection_takes_precedence_over_assumed() {
        let c = fixture();
        c.load_assumed_disjoint(&[(iri("Margherita"), iri("MeatyPizza"))]).unwrap();
        let claim = Claim {
            types: vec![("x".into(), iri("Margherita")), ("x".into(), iri("MeatyPizza"))],
            ..Default::default()
        };
        let r = c.check(&claim).unwrap();
        // Entailed disjointness fires as a rejection; no duplicate warning.
        assert_eq!(r.verdict, Verdict::Rejected);
        assert!(r.warnings.is_empty(), "{r:?}");
    }

    #[test]
    fn oracle_confirmed_consistency_keeps_the_warning() {
        let c = fixture();
        c.load_assumed_disjoint(&[(iri("Margherita"), iri("CheeseyPizza"))]).unwrap();
        let o = oracle(vec![], vec![]);
        let claim = Claim {
            types: vec![("x".into(), iri("Margherita")), ("x".into(), iri("CheeseyPizza"))],
            ..Default::default()
        };
        let r = c.check_with_oracle(&claim, &o).unwrap();
        // The reasoner proves no ENTAILED contradiction, so the claim is
        // Consistent — but the extra-logical assumption still stands and the
        // warning must survive, because "not entailed inconsistent" does not
        // refute vetted domain knowledge the ontology never encoded.
        assert_eq!(r.verdict, Verdict::Consistent, "{r:?}");
        assert_eq!(r.warnings.len(), 1, "{r:?}");
    }

    // ── batch / parallelism ─────────────────────────────────────────────

    #[test]
    fn batch_matches_sequential() {
        let c = fixture();
        let claims: Vec<Claim> = vec![
            Claim {
                types: vec![("x".into(), iri("Margherita")), ("x".into(), iri("MeatyPizza"))],
                ..Default::default()
            },
            Claim {
                types: vec![("x".into(), iri("Margherita")), ("x".into(), iri("CheeseyPizza"))],
                ..Default::default()
            },
            Claim {
                types: vec![("x".into(), iri("QuantumPineappleTopping"))],
                ..Default::default()
            },
        ];
        let batch = c.check_batch(&claims).unwrap();
        for (claim, br) in claims.iter().zip(&batch) {
            let sr = c.check(claim).unwrap();
            assert_eq!(br.verdict, sr.verdict);
            assert_eq!(br.violations.len(), sr.violations.len());
        }
    }

    #[test]
    fn batch_is_deterministic_under_parallelism() {
        let c = fixture();
        let claim = Claim {
            types: vec![("x".into(), iri("Margherita")), ("x".into(), iri("MeatyPizza"))],
            ..Default::default()
        };
        let claims: Vec<Claim> = (0..1000).map(|_| claim.clone()).collect();
        let out = c.check_batch(&claims).unwrap();
        assert_eq!(out.len(), 1000);
        assert!(out.iter().all(|r| r.verdict == Verdict::Rejected));
    }
}
