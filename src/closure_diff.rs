//! Entailment preservation under graph projection, by closure difference.
//!
//! [`crate::projection_entailment`] asks the question of a SUPPLIED set of
//! goals: the online form, for auditing one answer. This module supplies no
//! goals and computes the whole thing: reason both graphs to a fixpoint under
//! the same rule table and report `closure(G) \ closure(P)`. That is the form
//! an offline audit of a retrieval STRATEGY needs, because a strategy is not
//! evaluated against one question, it is evaluated against every conclusion it
//! could ever be asked to ground.
//!
//! Everything shared with the goal-directed form is shared rather than
//! re-implemented: the certificate index, the checker runner, the subset
//! precondition, the skolemiser and the monotonicity differential all live in
//! [`crate::projection_entailment`], so there is exactly ONE place in this
//! crate where a verdict word is produced.
//!
//! # Replacing a ratio with a count would reproduce the disease
//!
//! `closure(G) \ closure(P)` is enormous by construction: `P` is a slice, so
//! the difference is nearly all of `closure(G)`, and it grows with `|G|`.
//! Minimising it means retrieving MORE, which is the same perverse gradient
//! wearing a better name. The headline is therefore
//! `lost_in_projection_vocabulary`: a lost entailment is dangerous exactly when
//! every one of its terms occurs in `P`, because then an answer grounded in `P`
//! is a claim over those terms and could have rested on it. That number has its
//! own gaming direction, shrinking `terms(P)` shrinks it, and the report says
//! so in the payload rather than only in the docs.
//!
//! # Cost
//!
//! `closure(G)` is the expensive half and `G` does not change between retrieval
//! strategies, so [`SourceClosure::build`] is paid once and [`SourceClosure::diff`]
//! is run N times. The cache is the certificate directory: `asserted.tsv` and
//! `derivations.tsv` already exist, are already the format, and `oo-cert` has
//! already been run over them. [`SourceClosure::load`] RE-RUNS the checker
//! rather than trusting a verdict read off disk, because a cached verdict would
//! be an unchecked result wearing the checked word one indirection away.

use crate::graph::GraphStore;
use crate::projection_entailment as pe;
use crate::projection_entailment::{
    CertKind, CertificateIndex, CheckerStatus, CoverageProxy, Guards, Spelled, LAKE_INSTALL,
    STOP_THE_LINE,
};
use crate::verdict::{Certified, ClosureVerdict};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// An N-Triples-spelled triple, exactly as the store's interner holds it and
/// exactly as `asserted.tsv` writes it.
pub type NtTriple = Spelled;

// ───────────────────────────────────────────────────────────────────────────
// Verdict vocabulary
// ───────────────────────────────────────────────────────────────────────────

/// Why the source is entitled to a conclusion. NEVER COLLAPSED.
///
/// `Checked` carries the evidence, so the row below cannot wear the word on a
/// run where nothing was checked:
///
/// ```compile_fail
/// use open_ontologies::closure_diff::Warrant;
/// use open_ontologies::verdict::Certified;
/// let w = Warrant::Checked(Certified { theorem: "OOCert.certificate_sound" });
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Warrant {
    /// The triple IS in the asserted graph. No rule fired, nothing was proved,
    /// and nothing needed to be: a lookup, not an entailment claim. Listed
    /// separately from `Checked` so the checked count cannot rise without the
    /// checker running: 35,000 assertions and 3 inferences reporting "35,003
    /// checked conclusions" on a machine with no Lean is the laundering shape.
    AssertedInSource,
    /// A derivation step for it appears in a certificate `oo-cert` ACCEPTED.
    /// `OOCert.certificate_sound` applies. The only word that may be spoken
    /// when a machine-checked theorem stands behind the row.
    Checked(Certified),
    /// The engine says it derived this and no checker has looked: the checker
    /// was not built, was not run, or REJECTED the certificate.
    EngineOpinion,
}

impl Warrant {
    pub fn name(self) -> &'static str {
        match self {
            Warrant::AssertedInSource => "asserted_in_source",
            Warrant::Checked(_) => "checked",
            Warrant::EngineOpinion => "engine_opinion",
        }
    }
    /// The theorem, named only where one stands behind the word. It comes out
    /// of the evidence now, not out of a match arm, so an unchecked run has no
    /// way to print it.
    pub fn theorem(self) -> Option<&'static str> {
        match self {
            Warrant::Checked(c) => Some(c.theorem()),
            _ => None,
        }
    }
    pub fn is_checked(self) -> bool {
        matches!(self, Warrant::Checked(_))
    }
}

impl Serialize for Warrant {
    /// The snake_case word the derive used to write, unchanged on the wire.
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.name())
    }
}

impl PartialEq<&str> for Warrant {
    fn eq(&self, other: &&str) -> bool {
        self.name() == *other
    }
}

/// Whether a term's spelling can be trusted to name the same thing on both
/// sides. Blank nodes do not compare across graphs by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Comparability {
    /// Every term is an IRI or a literal. Two spellings name the same thing.
    Ground,
    /// A term is a Skolem IRI THIS RUN minted for a source blank node, and the
    /// projection was produced from the skolemised source, so the binding is
    /// this run's own and is exact.
    SkolemBound,
    /// A blank node with no binding across the two graphs. NOT COMPARED, and no
    /// verdict of any kind is computed from it.
    Unbound,
}

pub fn comparability_of(t: &NtTriple, skolem_map: &BTreeMap<String, String>) -> Comparability {
    if pe::is_blank(&t.0) || pe::is_blank(&t.1) || pe::is_blank(&t.2) {
        return Comparability::Unbound;
    }
    let skolem = |x: &String| x.starts_with(&format!("<{}", pe::SKOLEM_PREFIX));
    if !skolem_map.is_empty() && (skolem(&t.0) || skolem(&t.2)) {
        return Comparability::SkolemBound;
    }
    Comparability::Ground
}

// ───────────────────────────────────────────────────────────────────────────
// What checked the closure this diff is computed from
// ───────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize)]
pub struct CertificateVerdict {
    /// `"checked"` | `"engine_opinion"` | `"rejected"`. The checked variant
    /// carries the evidence a zero exit code minted, so no other path in this
    /// file or any other can reach the word — it is not a `&'static str` any
    /// more and there is nothing to assign.
    pub verdict: ClosureVerdict,
    /// `Some("OOCert.certificate_sound")` on `"checked"`, `None` otherwise,
    /// and taken from the evidence rather than written beside it.
    pub theorem: Option<&'static str>,
    pub asserted: usize,
    pub derivations: usize,
    /// The checker's own output, verbatim, when it ran.
    pub checker_report: Option<String>,
    /// `None` means the checker was never run.
    pub checker_exit: Option<i32>,
    /// Why it was not run, or why it said no. Carries the install line. Loud,
    /// never silent.
    pub skipped: Option<String>,
    /// False when the reasoner stopped at `runtime::reasoner_max_iterations()`
    /// with the closure still growing, so the closure may be TRUNCATED. No
    /// monotonicity verdict may be computed from a truncated source closure.
    pub reached_fixpoint: bool,
    pub iterations: usize,
    pub iteration_cap: usize,
}

impl CertificateVerdict {
    fn from_status(
        status: &CheckerStatus,
        asserted: usize,
        derivations: usize,
        reached_fixpoint: bool,
        iterations: usize,
    ) -> Self {
        // The ONE place `"checked"` is produced. It requires an accepted run,
        // which required exit code 0 — and now the type says so: the arm below
        // is the only one that has a `Certified` to put in the variant.
        let (verdict, exit, skipped): (ClosureVerdict, Option<i32>, Option<String>) = match status {
            CheckerStatus::Accepted(a) => (ClosureVerdict::Checked(a.certified()), Some(0), None),
            CheckerStatus::Rejected { stdout } => (
                ClosureVerdict::Rejected,
                Some(1),
                Some(format!(
                    "the verified checker REJECTED this certificate, so nothing in this report is \
                     checked: {}",
                    stdout.trim()
                )),
            ),
            // Exit 2 is "a file could not be read or parsed" and is NOT a
            // rejection: `lean/Main.lean` reserves the two codes for exactly
            // that distinction, and mapping 2 to "rejected" would turn a
            // missing file into a soundness finding.
            CheckerStatus::Unreadable { stdout } => (
                ClosureVerdict::EngineOpinion,
                Some(2),
                Some(format!("the checker could not read the certificate: {}", stdout.trim())),
            ),
            CheckerStatus::Absent { what, install } => (
                ClosureVerdict::EngineOpinion,
                None,
                Some(format!("{what}. {install}")),
            ),
            // The closure diff always has a whole certificate to check, so
            // this arm is unreachable here. It is still mapped to the weakest
            // word rather than left to a wildcard, because a wildcard is how a
            // new status silently becomes "checked" one refactor later.
            CheckerStatus::NotNeeded { what } => (ClosureVerdict::EngineOpinion, None, Some(what.clone())),
        };
        CertificateVerdict {
            verdict,
            theorem: verdict.theorem(),
            asserted,
            derivations,
            checker_report: match status {
                CheckerStatus::Accepted(a) => Some(a.stdout().to_string()),
                _ => None,
            },
            checker_exit: exit,
            skipped,
            reached_fixpoint,
            iterations,
            iteration_cap: crate::runtime::reasoner_max_iterations(),
        }
    }

    fn warrant_for_derived(&self) -> Warrant {
        match self.verdict {
            ClosureVerdict::Checked(c) => Warrant::Checked(c),
            _ => Warrant::EngineOpinion,
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Findings
// ───────────────────────────────────────────────────────────────────────────

/// A conclusion of the source that the projection does not reach.
/// Not `Deserialize`: it carries a [`Warrant`], whose checked variant is
/// evidence and not a word, so parsing one back out of JSON would be a public
/// constructor for the certified state. Read a report as `serde_json::Value`.
#[derive(Clone, Debug, Serialize)]
pub struct LostEntailment {
    pub triple: NtTriple,
    pub warrant: Warrant,
    pub warrant_word: &'static str,
    /// Named only when `warrant.theorem()` is `Some`.
    pub theorem: Option<&'static str>,
    /// The rule that first produced it in the source, when it was inferred.
    pub rule: Option<String>,
    /// The premises of that derivation the PROJECTION's closure does not hold.
    /// This is the actionable half: the retriever dropped these, and the
    /// conclusion went with them. Printed rather than summarised.
    pub blocking_premises: Vec<NtTriple>,
    /// True when every term of the triple occurs in the projection, so an
    /// answer grounded in the projection is a claim over those terms and could
    /// have rested on this conclusion. THE headline partition.
    pub in_projection_vocabulary: bool,
    pub comparability: Comparability,
}

/// The stop-the-line case: a conclusion of the projection that is not a
/// conclusion of the source.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MonotonicityViolation {
    pub triple: NtTriple,
    pub projection_rule: Option<String>,
    /// The premises the projection's rule read, so the step can be replayed by
    /// hand against `asserted.tsv`.
    pub projection_premises: Vec<NtTriple>,
    pub severity: &'static str,
    pub means: &'static str,
}

/// The dual, and the safe direction. Every premise of the source's derivation
/// is present in the projection's closure, the projection reached a fixpoint,
/// and it still did not derive the conclusion. That is an INCOMPLETENESS bug in
/// the engine or a certificate-recording bug. Deriving less is the sound
/// direction (decision 0002 item 8), so this is WARN and never stop-the-line.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReachableButNotDerived {
    pub triple: NtTriple,
    pub rule: Option<String>,
    pub premises: Vec<NtTriple>,
    pub severity: &'static str,
    pub means: &'static str,
}

/// Triples no verdict was computed from, with the reason. Counted, never folded
/// into a denominator: if these were dropped from the count rather than
/// reported, `lost_in_projection_vocabulary == 0` would mean "nothing was lost"
/// on a graph where a quarter of the triples were never looked at.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NotCompared {
    pub source_triples: usize,
    pub projection_triples: usize,
    pub distinct_source_blank_nodes: usize,
    pub distinct_projection_blank_nodes: usize,
    pub why: &'static str,
}

pub const NOT_COMPARED_WHY: &str =
    "a blank node has no name that survives a graph boundary, and deciding whether one graph is a \
     subgraph of another up to blank-node renaming is simple entailment, which is NP-complete in \
     general and polynomial only when the target is ground. This bucket is a consequence of that \
     complexity, not an unfinished feature. Skolemise the source before the retriever sees it and \
     it empties.";

pub const NOT_DERIVED_MEANS: &str =
    "every premise of the source's derivation IS in the projection's closure and the projection \
     reached a fixpoint, yet it did not draw the conclusion. That is an incompleteness bug in the \
     engine or a certificate-recording bug. Deriving less is the sound direction, so this is a \
     warning and never a stop-the-line.";

// ───────────────────────────────────────────────────────────────────────────
// The report
// ───────────────────────────────────────────────────────────────────────────

/// Not `Deserialize`, for the reason on [`LostEntailment`].
#[derive(Clone, Debug, Serialize)]
pub struct ClosureDiffReport {
    pub format: &'static str,
    /// One sentence the reader cannot miss, first after `format` in the JSON
    /// and the first line of any human rendering.
    pub headline: String,
    pub profile: String,

    /// What licenses `Warrant::Checked` on the rows below.
    pub source_certificate: CertificateVerdict,
    /// What licenses a monotonicity claim.
    pub projection_certificate: CertificateVerdict,

    pub subset: pe::SubsetReport,

    pub entailments_lost: Vec<LostEntailment>,
    /// Exact totals. `entailments_lost` is capped for rendering; these are not.
    pub lost_total: usize,
    pub lost_in_projection_vocabulary: usize,
    pub lost_by_warrant: BTreeMap<String, usize>,
    /// The gaming direction of the headline number, in the payload rather than
    /// only in the docs, because the docs are not what a dashboard renders.
    pub headline_gaming_direction: &'static str,

    /// Empty except when the engine is wrong.
    pub monotonicity_violations: Vec<MonotonicityViolation>,
    /// Non-null whenever the gate did NOT run, with the reason, so a green
    /// `monotonicity_violations: []` can never mean "we did not look".
    pub monotonicity_gate_skipped: Option<String>,

    pub incompleteness_warnings: Vec<ReachableButNotDerived>,

    pub not_compared: NotCompared,

    pub coverage_proxy: CoverageProxy,

    pub skolemised: usize,
    pub asserted_digest: String,
    /// Set when the run could not do what it says. Loud, never silent.
    pub skipped: Option<String>,
    pub seconds: f64,
    /// 0 clean, 1 something was lost, 2 stop-the-line.
    pub exit_code: i32,
}

pub const HEADLINE_GAMING_DIRECTION: &str =
    "lost_in_projection_vocabulary has its own gaming direction: shrinking the projection's \
     vocabulary shrinks it. It is the right headline because a lost conclusion whose terms the \
     projection never mentions cannot ground an answer the projection supports, and it is not a \
     score to tune on.";

// ───────────────────────────────────────────────────────────────────────────
// Options
// ───────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct DiffOptions {
    /// `rdfs`, `owl-rl` or `owl-rl-ext`. `owl-dl` is REFUSED: the tableaux path
    /// emits no rule trace, so neither side could be checked and the whole
    /// verdict vocabulary would collapse to `engine_opinion` while still
    /// looking like a certified run.
    pub profile: String,
    /// Where both certificates and the report land. A run is reproducible by
    /// hand from what it leaves behind.
    pub out: PathBuf,
    /// Path to the checker. `None` looks at `$OO_CERT`, then the repository's
    /// `lean/.lake/build/bin/`, then `$PATH`.
    pub checker: Option<PathBuf>,
    /// Replace every source blank node with a Skolem IRI before the retriever
    /// ever sees the graph. Without this, every triple touching a blank node
    /// lands in `not_compared` and the monotonicity gate is suppressed for it.
    pub skolemise_source: bool,
    /// Rows rendered per list. Totals are always exact.
    pub max_rows: usize,
    /// Seeds for the demoted coverage proxy.
    pub seed_iris: Vec<String>,
}

impl Default for DiffOptions {
    fn default() -> Self {
        DiffOptions {
            profile: "owl-rl-ext".into(),
            out: PathBuf::from("closure-diff"),
            checker: None,
            skolemise_source: true,
            max_rows: 200,
            seed_iris: Vec::new(),
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
// The source side, computed once and reused
// ───────────────────────────────────────────────────────────────────────────

/// FNV-1a 64, written out here rather than taken from a library, over a
/// canonical re-serialisation of the asserted set.
///
/// Offset basis `0xcbf29ce484222325`, prime `0x100000001b3`, the same
/// specification `src/vecstore.rs` already writes down for its own fingerprint.
/// `String::hash` is an opaque extern with no specification a second
/// implementation could target, so it is not used.
///
/// It IDENTIFIES so two runs can be compared; it does not COMMIT, and anyone
/// who needs that should hash the file. That is decision 0003 item 5, in the
/// same words, for the same reason.
///
/// The first version of this function grouped the prime's digits wrong and
/// multiplied by `0x1000000001b3`, one zero too many. Nothing would have
/// looked broken: the digest is only compared against itself. The published
/// test vectors below are what caught it, which is why they are pinned rather
/// than left to a round trip that would agree with any constant at all.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

fn digest_of(asserted: &HashSet<Spelled>) -> u64 {
    let mut lines: Vec<String> =
        asserted.iter().map(|t| format!("{}\t{}\t{}", t.0, t.1, t.2)).collect();
    lines.sort();
    fnv1a64(lines.join("\n").as_bytes())
}

/// The source side of a diff: its closure, its certificate verdict, and the
/// skolem map that made it comparable.
pub struct SourceClosure {
    closure: HashSet<Spelled>,
    asserted: HashSet<Spelled>,
    /// For a derived triple, the rule and premises of its FIRST derivation.
    derivation_of: HashMap<Spelled, (String, Vec<Spelled>)>,
    /// Copied onto every report this cache serves. A cache built without a
    /// checker serves `engine_opinion` for ever; it cannot be upgraded in
    /// place, because the upgrade would be a claim nobody made about the file
    /// that was actually diffed. Private, with no setter.
    verdict: CertificateVerdict,
    skolem_map: BTreeMap<String, String>,
    asserted_digest: u64,
    store: Arc<GraphStore>,
}

impl SourceClosure {
    /// Reason the source to a fixpoint, write the certificate under
    /// `opts.out/source`, and run the checker over it.
    pub fn build(source: &Arc<GraphStore>, opts: &DiffOptions) -> anyhow::Result<SourceClosure> {
        refuse_owl_dl(&opts.profile)?;
        let (store, skolem_map) = if opts.skolemise_source {
            pe::skolemise(source)?
        } else {
            (source.clone(), BTreeMap::new())
        };
        let dir = opts.out.join("source");
        let run = reason_side(&store, &opts.profile, &dir)?;
        let cert = CertificateIndex::read(&dir)?;
        let status = pe::run_oo_cert(
            opts.checker.as_deref(),
            &cert.asserted_path(),
            &dir.join("derivations.tsv"),
        );
        Ok(Self::assemble(&cert, status, run, store, skolem_map))
    }

    /// Reload from a certificate directory a previous run wrote, RE-RUNNING the
    /// checker rather than trusting the file.
    pub fn load(dir: &Path, opts: &DiffOptions) -> anyhow::Result<SourceClosure> {
        let cert = CertificateIndex::read(dir)?;
        let status = pe::run_oo_cert(
            opts.checker.as_deref(),
            &cert.asserted_path(),
            &dir.join("derivations.tsv"),
        );
        // A reloaded cache carries no live store and no iteration record. The
        // fixpoint flag is unknown, and an unknown fixpoint disarms the gate
        // rather than arming it on an assumption.
        let run = SideRun { reached_fixpoint: false, iterations: 0 };
        let store = Arc::new(GraphStore::new());
        Ok(Self::assemble(&cert, status, run, store, BTreeMap::new()))
    }

    fn assemble(
        cert: &CertificateIndex,
        status: CheckerStatus,
        run: SideRun,
        store: Arc<GraphStore>,
        skolem_map: BTreeMap<String, String>,
    ) -> SourceClosure {
        let asserted = cert.asserted_set().clone();
        let closure = cert.closure();
        let mut derivation_of = HashMap::new();
        for i in 0..cert.derivation_count() {
            if let (Some(rule), Some(prem)) = (cert.rule_of(i), cert.premises_of(i)) {
                // The index records the FIRST derivation of each conclusion, so
                // inserting only when absent keeps that choice.
                let concl = cert
                    .conclusion_of_line(i)
                    .expect("a line has a conclusion")
                    .clone();
                derivation_of
                    .entry(concl)
                    .or_insert_with(|| (rule.to_string(), prem.to_vec()));
            }
        }
        let verdict = CertificateVerdict::from_status(
            &status,
            cert.asserted_count(),
            cert.derivation_count(),
            run.reached_fixpoint,
            run.iterations,
        );
        let asserted_digest = digest_of(&asserted);
        SourceClosure { closure, asserted, derivation_of, verdict, skolem_map, asserted_digest, store }
    }

    pub fn verdict(&self) -> &CertificateVerdict {
        &self.verdict
    }
    pub fn asserted_digest(&self) -> u64 {
        self.asserted_digest
    }
    pub fn skolem_map(&self) -> &BTreeMap<String, String> {
        &self.skolem_map
    }
    /// The (possibly skolemised) store the diff is computed against. A
    /// retriever must be pointed at THIS store, not the original, or the slice
    /// will carry blank nodes the source no longer has.
    pub fn store(&self) -> &Arc<GraphStore> {
        &self.store
    }
    pub fn closure_size(&self) -> usize {
        self.closure.len()
    }

    /// Assemble a closure from parts.
    ///
    /// The hook the monotonicity gate's failure test needs: the engine cannot be
    /// made unsound on demand, so the test builds a source closure with a hole
    /// the projection's closure fills.
    #[doc(hidden)]
    pub fn from_parts(
        closure: HashSet<Spelled>,
        asserted: HashSet<Spelled>,
        derivation_of: HashMap<Spelled, (String, Vec<Spelled>)>,
        verdict: CertificateVerdict,
    ) -> SourceClosure {
        let asserted_digest = digest_of(&asserted);
        SourceClosure {
            closure,
            asserted,
            derivation_of,
            verdict,
            skolem_map: BTreeMap::new(),
            asserted_digest,
            store: Arc::new(GraphStore::new()),
        }
    }

    /// Diff one projection against this closure.
    pub fn diff(&self, projection_ttl: &str, opts: &DiffOptions) -> anyhow::Result<ClosureDiffReport> {
        refuse_owl_dl(&opts.profile)?;
        let started = std::time::Instant::now();
        let pstore = Arc::new(GraphStore::new());
        pstore.load_turtle(projection_ttl, None).map_err(|e| {
            let hint = if projection_ttl.contains("<_:") {
                ". The slice contains `<_:`, an angle-bracketed blank node label, which is not a \
                 legal IRI: that is what onto_segment_retrieve emits for a blank-node object. \
                 Skolemise the source before retrieving"
            } else {
                ""
            };
            anyhow::anyhow!("the projection is not readable as Turtle: {e}{hint}")
        })?;

        let pdir = opts.out.join("projection");
        let prun = reason_side(&pstore, &opts.profile, &pdir)?;
        let pcert = CertificateIndex::read(&pdir)?;
        let pstatus =
            pe::run_oo_cert(opts.checker.as_deref(), &pcert.asserted_path(), &pdir.join("derivations.tsv"));
        let pverdict = CertificateVerdict::from_status(
            &pstatus,
            pcert.asserted_count(),
            pcert.derivation_count(),
            prun.reached_fixpoint,
            prun.iterations,
        );
        let pclosure = pcert.closure();

        // ── Subsethood, the ANTECEDENT of the monotonicity argument ──────
        let proj_asserted = pcert.asserted_set();
        let bn_src = self.asserted.iter().filter(|t| bn(t)).count();
        let bn_prj = proj_asserted.iter().filter(|t| bn(t)).count();
        let ground_only = bn_src > 0 || bn_prj > 0;
        let mut extras: Vec<String> = Vec::new();
        let mut extra_count = 0usize;
        for t in proj_asserted {
            if ground_only && bn(t) {
                continue;
            }
            if !self.asserted.contains(t) {
                extra_count += 1;
                if extras.len() < opts.max_rows.min(25) {
                    extras.push(format!("{} {} {}", t.0, t.1, t.2));
                }
            }
        }
        extras.sort();
        let subset = pe::SubsetReport {
            verified: extra_count == 0,
            decided_over: if ground_only { "ground triples only" } else { "all triples" },
            extra_triples: extras,
            extra_count,
            blank_node_bearing_source: bn_src,
            blank_node_bearing_projection: bn_prj,
            blank_nodes_unmatched: ground_only,
        };

        let not_compared = NotCompared {
            source_triples: bn_src,
            projection_triples: bn_prj,
            distinct_source_blank_nodes: distinct_blanks(&self.asserted),
            distinct_projection_blank_nodes: distinct_blanks(proj_asserted),
            why: NOT_COMPARED_WHY,
        };

        // ── The free differential, through the shared pure function ──────
        let both_fixpoint = self.verdict.reached_fixpoint && prun.reached_fixpoint;
        let guards = Guards {
            subset_verified: subset.verified,
            blank_nodes_unmatched: subset.blank_nodes_unmatched,
            both_reached_fixpoint: both_fixpoint,
        };
        let mono = pe::differential(&self.closure, &pclosure, guards);
        let gate_skipped = if mono.status == "disarmed" {
            Some(match mono.reason {
                "projection_is_not_a_subset" => format!(
                    "the monotonicity gate did NOT run: the projection is not a subset of the \
                     source ({} extra asserted triple(s)). A conclusion the projection reaches \
                     and the source does not is then ordinary, and reporting it as an engine \
                     soundness bug would be a spectacular false alarm. This is a RETRIEVAL \
                     finding",
                    subset.extra_count
                ),
                "blank_nodes_unmatched" => format!(
                    "the monotonicity gate did NOT run: {} source and {} projection triples carry \
                     a blank node with no binding across the two graphs. Set skolemise_source to \
                     empty this bucket",
                    bn_src, bn_prj
                ),
                _ => format!(
                    "the monotonicity gate did NOT run: a closure that stopped at the \
                     {}-iteration cap is a LOWER BOUND and cannot refute anything (source \
                     fixpoint {}, projection fixpoint {})",
                    crate::runtime::reasoner_max_iterations(),
                    self.verdict.reached_fixpoint,
                    prun.reached_fixpoint
                ),
            })
        } else {
            None
        };
        let mut violations: Vec<MonotonicityViolation> = Vec::new();
        if mono.status == "armed" {
            for t in pclosure.difference(&self.closure) {
                let i = pcert.conclusion_index(t);
                violations.push(MonotonicityViolation {
                    triple: t.clone(),
                    projection_rule: i.and_then(|i| pcert.rule_of(i)).map(|s| s.to_string()),
                    projection_premises: i
                        .and_then(|i| pcert.premises_of(i))
                        .map(|p| p.to_vec())
                        .unwrap_or_default(),
                    severity: STOP_THE_LINE,
                    means: pe::MONOTONICITY_MEANS,
                });
            }
            violations.sort_by(|a, b| a.triple.cmp(&b.triple));
        }

        // ── The difference, partitioned by projection vocabulary ─────────
        let pterms: HashSet<&str> = proj_asserted
            .iter()
            .flat_map(|t| [t.0.as_str(), t.1.as_str(), t.2.as_str()])
            .chain(
                pclosure
                    .iter()
                    .flat_map(|t| [t.0.as_str(), t.1.as_str(), t.2.as_str()]),
            )
            .collect();
        let mut lost: Vec<LostEntailment> = Vec::new();
        let mut lost_in_vocab = 0usize;
        let mut by_warrant: BTreeMap<String, usize> = BTreeMap::new();
        let mut incompleteness: Vec<ReachableButNotDerived> = Vec::new();
        let mut ordered: Vec<&Spelled> = self.closure.difference(&pclosure).collect();
        ordered.sort();
        for t in ordered {
            let derivation = self.derivation_of.get(t);
            let warrant = if self.asserted.contains(t) {
                Warrant::AssertedInSource
            } else {
                self.verdict.warrant_for_derived()
            };
            *by_warrant.entry(warrant.name().to_string()).or_default() += 1;
            let in_vocab = [&t.0, &t.1, &t.2].iter().all(|x| pterms.contains(x.as_str()));
            if in_vocab {
                lost_in_vocab += 1;
            }
            let blocking: Vec<Spelled> = derivation
                .map(|(_, prem)| prem.iter().filter(|p| !pclosure.contains(*p)).cloned().collect())
                .unwrap_or_default();
            // The dual detector, free in the same pass.
            if let Some((rule, prem)) = derivation
                && blocking.is_empty()
                && prun.reached_fixpoint
                && incompleteness.len() < opts.max_rows
            {
                incompleteness.push(ReachableButNotDerived {
                    triple: t.clone(),
                    rule: Some(rule.clone()),
                    premises: prem.clone(),
                    severity: "WARN",
                    means: NOT_DERIVED_MEANS,
                });
            }
            if lost.len() < opts.max_rows {
                lost.push(LostEntailment {
                    triple: t.clone(),
                    warrant,
                    warrant_word: warrant.name(),
                    theorem: warrant.theorem(),
                    rule: derivation.map(|(r, _)| r.clone()),
                    blocking_premises: blocking,
                    in_projection_vocabulary: in_vocab,
                    comparability: comparability_of(t, &self.skolem_map),
                });
            }
        }
        let lost_total = self.closure.difference(&pclosure).count();

        let coverage_proxy = pe::coverage_proxy(&self.store, &opts.seed_iris, projection_ttl);
        let exit_code = if !violations.is_empty() {
            2
        } else if lost_in_vocab > 0 {
            1
        } else {
            0
        };
        let headline = format!(
            "{lost_total} conclusion(s) of the source are not conclusions of the projection, \
             {lost_in_vocab} of them over terms the projection MENTIONS. Source certificate: {}{}. \
             Monotonicity gate: {}{}. {}",
            self.verdict.verdict,
            match self.verdict.theorem {
                Some(t) => format!(" ({t})"),
                None => String::new(),
            },
            mono.status,
            if mono.reason.is_empty() { String::new() } else { format!(" ({})", mono.reason) },
            pe::COVERAGE_LABEL,
        );

        Ok(ClosureDiffReport {
            format: "oo-closure-diff/1",
            headline,
            profile: opts.profile.clone(),
            source_certificate: self.verdict.clone(),
            projection_certificate: pverdict,
            subset,
            entailments_lost: lost,
            lost_total,
            lost_in_projection_vocabulary: lost_in_vocab,
            lost_by_warrant: by_warrant,
            headline_gaming_direction: HEADLINE_GAMING_DIRECTION,
            monotonicity_violations: violations,
            monotonicity_gate_skipped: gate_skipped,
            incompleteness_warnings: incompleteness,
            not_compared,
            coverage_proxy,
            skolemised: self.skolem_map.len(),
            asserted_digest: format!("{:016x}", self.asserted_digest),
            skipped: None,
            seconds: started.elapsed().as_secs_f64(),
            exit_code,
        })
    }
}

fn bn(t: &Spelled) -> bool {
    pe::is_blank(&t.0) || pe::is_blank(&t.1) || pe::is_blank(&t.2)
}

fn distinct_blanks(set: &HashSet<Spelled>) -> usize {
    let mut s: BTreeSet<&str> = BTreeSet::new();
    for t in set {
        for x in [&t.0, &t.1, &t.2] {
            if pe::is_blank(x) {
                s.insert(x.as_str());
            }
        }
    }
    s.len()
}

fn refuse_owl_dl(profile: &str) -> anyhow::Result<()> {
    if profile == "owl-dl" {
        anyhow::bail!(
            "owl-dl emits no rule trace, so neither certificate could be written and every row \
             would be engine_opinion over a run that still looked like a closure diff. Run rdfs, \
             owl-rl or owl-rl-ext"
        );
    }
    Ok(())
}

struct SideRun {
    reached_fixpoint: bool,
    iterations: usize,
}

fn reason_side(store: &Arc<GraphStore>, profile: &str, dir: &Path) -> anyhow::Result<SideRun> {
    std::fs::create_dir_all(dir)?;
    let out = crate::reason::Reasoner::run_full(
        store,
        profile,
        false,
        crate::reason::InferenceTarget::DefaultGraph,
        Some(dir),
    )?;
    let v: serde_json::Value = serde_json::from_str(&out)?;
    Ok(SideRun {
        reached_fixpoint: v["fixpoint_reached"].as_bool().unwrap_or(false),
        iterations: v["iterations"].as_u64().unwrap_or(0) as usize,
    })
}

/// One-shot: reason both graphs, check both certificates, subtract.
pub fn closure_diff(
    source: &Arc<GraphStore>,
    projection_ttl: &str,
    opts: &DiffOptions,
) -> anyhow::Result<ClosureDiffReport> {
    let src = SourceClosure::build(source, opts)?;
    src.diff(projection_ttl, opts)
}

/// `oo-cert`'s install line, re-exported so a caller does not have to reach
/// into the sibling module for it.
pub const OO_CERT_INSTALL_LINE: &str = LAKE_INSTALL;

/// The kind of certificate this module reads. Fixed: the closure diff is over
/// a built-in profile, so a supplied Horn table has no arm here.
pub const CERT_KIND: CertKind = CertKind::OoCert;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a64_matches_the_specified_constant() {
        // Pinned against the published FNV-1a 64 vectors, not against a round
        // trip through this function, which would agree with any constant.
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
    }

    /// The acceptance row used to be written by hand:
    ///
    /// ```text
    /// CheckerStatus::Accepted { theorem: "OOCert.certificate_sound", stdout: "{}".into() }
    /// ```
    ///
    /// That line no longer compiles, and that is the point of this change.
    /// `CheckerStatus::Accepted` now carries a `projection_entailment::
    /// Accepted`, whose only constructor takes a `verdict::Certified`, which
    /// nothing outside `src/verdict.rs` can build. A test that wants an
    /// acceptance has to EARN one, so this one runs a process that exits zero
    /// — which is exactly the condition the test is named after.
    #[test]
    fn a_checked_word_needs_exit_zero() {
        let dir = std::env::temp_dir().join("oo-closure-diff-exit-zero");
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("asserted.tsv");
        let d = dir.join("derivations.tsv");
        std::fs::write(&a, "").unwrap();
        std::fs::write(&d, "").unwrap();
        // A script rather than `/bin/true`, which is `/usr/bin/true` on macOS
        // and absent from `/bin` entirely. The suite found that itself. A `.sh`
        // is not executable on Windows either, which CI then found.
        let ok = dir.join(if cfg!(windows) { "exit0.cmd" } else { "exit0.sh" });
        let body = if cfg!(windows) { "@echo off\r\nexit /b 0\r\n" } else { "#!/bin/sh\nexit 0\n" };
        std::fs::write(&ok, body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&ok, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let accepted = pe::run_checker(CertKind::OoCert, Some(ok.as_path()), &a, &d, None);
        assert!(matches!(accepted, CheckerStatus::Accepted(_)), "{accepted:?}");

        for (status, want) in [
            (accepted, "checked"),
            (CheckerStatus::Rejected { stdout: "no".into() }, "rejected"),
            (CheckerStatus::Unreadable { stdout: "boom".into() }, "engine_opinion"),
            (
                CheckerStatus::Absent { what: "not built".into(), install: LAKE_INSTALL },
                "engine_opinion",
            ),
        ] {
            let v = CertificateVerdict::from_status(&status, 1, 1, true, 1);
            assert_eq!(v.verdict, want, "{status:?}");
            assert_eq!(v.theorem.is_some(), want == "checked");
            if want != "checked" {
                assert!(v.skipped.is_some(), "a non-checked verdict must say why: {v:?}");
            }
        }
    }

    #[test]
    fn exit_two_is_not_a_rejection() {
        let v = CertificateVerdict::from_status(
            &CheckerStatus::Unreadable { stdout: "cannot read".into() },
            0,
            0,
            true,
            1,
        );
        assert_ne!(v.verdict, "rejected", "an unreadable file is not a rejected certificate");
        assert_eq!(v.checker_exit, Some(2));
    }
}
