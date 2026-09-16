//! Does a retrieved slice still support the claims an answer rests on?
//!
//! [`crate::projection_check`] answers "how much of the source neighbourhood
//! survived", which is not the question. A slice at 99% coverage can have
//! dropped the one triple an answer depends on, and a slice at 60% coverage can
//! preserve every conclusion that matters. Worse, the number moves the wrong
//! way: it rises as the projection grows, so a retriever tuned on it learns to
//! fetch MORE rather than to fetch the right thing, and a retrieval-augmented
//! answer grounded in such a slice can be false while every displayed metric is
//! green.
//!
//! This module asks the property instead. Let `G` be the loaded store, `P` the
//! retrieved slice and `Q` a set of ground positive triples the answer rests
//! on. For each `q` in `Q`, does `P` entail `q` exactly when `G` does, decided
//! under ONE pinned rule profile, with a machine-checked certificate for each
//! `q` the projection preserves?
//!
//! # Four cells, and they are four different answers
//!
//! | `G` derives `q` | `P` derives `q` | what it is |
//! |---|---|---|
//! | yes | yes | preserved. The only cell that can carry a certificate. |
//! | yes | no | lost by the projection. The retrieval finding. |
//! | no | yes | the projection is not a subset of the source, or OWL RL monotonicity has been violated and the engine is unsound. Never a retrieval finding. |
//! | no | no | `ungrounded_in_source`. The claim has no support in the graph at all. The fix is in the generator, not the retriever. |
//!
//! The fourth cell is where most of the real value sits, and it is why the two
//! negatives are never collapsed into "loss": a generator that invented a claim
//! and a retriever that dropped a triple have opposite fixes, and a tool that
//! reports both as "not preserved" sends every investigation to the wrong team.
//! `ungrounded_in_source` does NOT mean the claim is false. RDF entailment is
//! open-world and this engine covers 29 of OWL 2 RL's 78 rules, so it means
//! "not derivable from `G` under this profile" and nothing stronger.
//!
//! # Why membership is decided over the certificate FILES
//!
//! [`Reasoner::run_full`] is run twice with `materialize = false` and a
//! `certificate_dir`, and membership of `q` is decided by reading
//! `asserted.tsv` and `derivations.tsv` back off disk rather than by consulting
//! an in-memory closure. That is deliberate. The Lean checker only ever sees
//! those two files. If membership were decided against a set the checker never
//! reads, the engine could report "derived" over a certificate that does not
//! contain the derivation, and the verdict would be unchecked while wearing the
//! checked word. Deciding over the artefact the checker reads makes that class
//! of bug impossible rather than tested against.
//!
//! # The free differential
//!
//! OWL RL is monotone and `P` is a subset of `G`, so every entailment of `P`
//! must be an entailment of `G`. A conclusion of `P` that is not a conclusion of
//! `G` is therefore a SOUNDNESS BUG IN THE ENGINE, not a retrieval finding, and
//! it is reported as `STOP_THE_LINE`. It is worth nothing unless it is DISARMED
//! whenever its premises do not hold, and there are three such conditions, each
//! of which occurs on real input: the projection is not a subset (computed,
//! never assumed from provenance); blank nodes could not be matched; or either
//! run stopped at the iteration cap instead of a fixpoint, which makes the
//! larger closure a LOWER BOUND and manufactures spurious `P`-only entailments.
//! Disarming is reported, never silent.
//!
//! See [decision 0007](../../docs/decisions/0007-a-slice-preserves-a-conclusion-or-it-does-not.md).

use crate::graph::GraphStore;
use crate::projection_check::{check_projection_loss, ProjectionLossReport};
use crate::reason::{InferenceTarget, Reasoner};
use crate::verdict::{CheckerBinary, CheckerRun, Certified};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A triple in the store's own N-Triples spelling: byte-identical to what
/// [`GraphStore::all_triples`] yields and to what `asserted.tsv` carries.
/// Every comparison in this module is over this type and never over user text.
pub type Spelled = (String, String, String);

/// RDF 1.1 section 3.5 reserves `.well-known/genid/` for skolem constants.
pub const SKOLEM_PREFIX: &str = "https://open-ontologies.org/.well-known/genid/";

/// How to get the checker, printed wherever it is missing.
pub const LAKE_INSTALL: &str =
    "install elan from https://github.com/leanprover/elan; lean/lean-toolchain pins the version, \
     then `cd lean && lake build` (oo-cert and oo-horn are in defaultTargets). Or set OO_CERT to \
     the binary.";

/// The sentence that travels with the coverage number, in the payload rather
/// than only in the docs.
pub const COVERAGE_LABEL: &str =
    "coverage_ratio is a PROXY and is neither necessary nor sufficient for entailment \
     preservation: a slice at 0.99 can have dropped the one triple an answer rests on, and a \
     slice at 0.60 can preserve every claim in this report. It also moves the wrong way, rising \
     as the projection grows, so a retriever tuned on it learns to fetch MORE rather than the \
     right thing. Read the per-goal verdicts; this number is a cheap signal and never a warrant.";

/// The severity word `src/projection_check.rs`'s neighbours use for a finding
/// that must stop a pipeline rather than be filed under "other".
pub const STOP_THE_LINE: &str = "STOP_THE_LINE";

// ───────────────────────────────────────────────────────────────────────────
// Blank nodes
// ───────────────────────────────────────────────────────────────────────────

/// True when the term is a blank node in N-Triples spelling (`_:label`).
pub fn is_blank(term: &str) -> bool {
    term.starts_with("_:")
}

fn has_blank(t: &Spelled) -> bool {
    is_blank(&t.0) || is_blank(&t.1) || is_blank(&t.2)
}

/// Replace every blank node in `g` with a Skolem IRI, returning the new store
/// beside the map that did it.
///
/// THIS IS NOT [`GraphStore::canonicalize_blank_nodes`], and reaching for that
/// instead is the trap this function exists to stop. RDFC-1.0 labels are a
/// function of the WHOLE graph: a blank node with seven triples in the source
/// and three in a slice canonicalises to two DIFFERENT labels. Canonicalising
/// both sides separately and diffing is exactly as wrong as not canonicalising
/// at all, and it looks principled, which is worse.
///
/// Skolemising the SOURCE before a retriever ever sees it is the only thing
/// that makes a re-parsed slice comparable, because the slice then carries real
/// IRIs and nothing has to be matched.
pub fn skolemise(g: &Arc<GraphStore>) -> anyhow::Result<(Arc<GraphStore>, BTreeMap<String, String>)> {
    skolemise_with_prefix(g, SKOLEM_PREFIX)
}

/// [`skolemise`] under a caller-chosen prefix.
///
/// Two graphs skolemised SEPARATELY under the same prefix can collide: the map
/// is `_:label ↦ prefix + label`, and two unrelated graphs both containing
/// `_:b0` would then agree on an IRI that names two different things. Any
/// caller that skolemises one graph and later merges another into it has to
/// keep the two apart, and a distinct prefix is the only thing that does it.
/// `_` is not a legal character-run separator in an N-Triples blank node label
/// and `/` cannot appear in one at all, so a prefix ending in `/base/` cannot
/// be produced by the default prefix over any label.
pub fn skolemise_with_prefix(
    g: &Arc<GraphStore>,
    prefix: &str,
) -> anyhow::Result<(Arc<GraphStore>, BTreeMap<String, String>)> {
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    let mut nt = String::new();
    for (s, p, o) in g.all_triples()? {
        let mut one = |term: &str| -> String {
            if let Some(label) = term.strip_prefix("_:") {
                let iri = format!("<{prefix}{label}>");
                map.insert(iri.clone(), term.to_string());
                iri
            } else {
                term.to_string()
            }
        };
        let (s, p, o) = (one(&s), one(&p), one(&o));
        nt.push_str(&s);
        nt.push(' ');
        nt.push_str(&p);
        nt.push(' ');
        nt.push_str(&o);
        nt.push_str(" .\n");
    }
    let out = GraphStore::new();
    out.load_ntriples(&nt)?;
    Ok((Arc::new(out), map))
}

// ───────────────────────────────────────────────────────────────────────────
// Goals
// ───────────────────────────────────────────────────────────────────────────

/// One claim an answer rests on.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    /// Exactly what the caller wrote, where the caller's bytes are recoverable.
    ///
    /// They are on the TSV and bindings paths. On the Turtle path the document
    /// is parsed as a whole and there is no per-triple source text to keep, so
    /// this holds the parser's rendering and equals [`Goal::triple`] by
    /// construction; the field is still printed, because the case where the two
    /// differ is the one that bites.
    pub as_written: String,
    /// What is actually asked: the spelling that came back out of the parser.
    pub triple: Spelled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RefusedGoal {
    pub as_written: String,
    pub reason: GoalRefusal,
    pub means: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoalRefusal {
    /// A blank node in a goal is an existential, not a claim. This engine asks
    /// ground triples and cannot ask "there exists something such that".
    BlankNode,
    /// "no supplier is sanctioned", "exactly one registered address". Not a
    /// triple. Mapping absence-from-the-closure onto such a claim converts an
    /// open-world absence into a closed-world fact with a certificate stapled
    /// to it, which is the laundering this tool exists to attack, produced by
    /// the anti-laundering tool.
    NotAPositiveGroundTriple,
    /// A variable, a prefix that resolves to nothing, or text the parser
    /// refused.
    Unparseable,
}

impl GoalRefusal {
    pub fn means(self) -> &'static str {
        match self {
            GoalRefusal::BlankNode => {
                "a blank node in a goal is an existential rather than a claim. This layer asks \
                 whether a GROUND triple is entailed and has no way to ask 'there exists \
                 something such that'. Skolemise the source and ask about the Skolem constant, or \
                 ask a SHACL shape instead"
            }
            GoalRefusal::NotAPositiveGroundTriple => {
                "this is a closed-world or negative claim, not a positive ground triple. Reading \
                 'absent from the closure' as 'the claim holds' would turn an open-world absence \
                 into a fact with a certificate stapled to it. A closed-world question belongs in \
                 SHACL: use onto_shacl"
            }
            GoalRefusal::Unparseable => {
                "the parser refused this. A goal has to survive the SAME parser the store uses, \
                 because that is the only thing that makes the caller's spelling of a term \
                 comparable with the interner's"
            }
        }
    }
}

/// The OWL vocabulary for writing down a NEGATIVE fact. A caller reaching for
/// it is asking a closed-world question, and the refusal says where that
/// belongs. Nothing here is a general-purpose ban on the OWL namespace: a goal
/// asking whether `A owl:equivalentClass B` is entailed is a perfectly ordinary
/// positive ground triple and is accepted.
const NEGATIVE_VOCABULARY: [&str; 5] = [
    "<http://www.w3.org/2002/07/owl#NegativePropertyAssertion>",
    "<http://www.w3.org/2002/07/owl#sourceIndividual>",
    "<http://www.w3.org/2002/07/owl#assertionProperty>",
    "<http://www.w3.org/2002/07/owl#targetIndividual>",
    "<http://www.w3.org/2002/07/owl#targetValue>",
];

/// Prefixes a caller uses when trying to write "not this" into a goal line.
const NEGATION_MARKERS: [&str; 4] = ["NOT ", "not ", "!", "\u{00ac}"];

fn classify(t: &Spelled, as_written: &str) -> Result<(), GoalRefusal> {
    if has_blank(t) {
        return Err(GoalRefusal::BlankNode);
    }
    if t.0.starts_with('"') {
        // A literal subject is not writable RDF and no rule can conclude one.
        return Err(GoalRefusal::NotAPositiveGroundTriple);
    }
    if NEGATIVE_VOCABULARY.contains(&t.1.as_str()) || NEGATIVE_VOCABULARY.contains(&t.2.as_str()) {
        return Err(GoalRefusal::NotAPositiveGroundTriple);
    }
    if NEGATION_MARKERS.iter().any(|m| as_written.starts_with(m)) {
        return Err(GoalRefusal::NotAPositiveGroundTriple);
    }
    Ok(())
}

fn refuse(as_written: &str, reason: GoalRefusal) -> RefusedGoal {
    RefusedGoal { as_written: as_written.to_string(), reason, means: reason.means() }
}

fn accept(idx: usize, as_written: String, t: Spelled) -> Goal {
    Goal { id: format!("g{idx}"), as_written, triple: t }
}

/// Parse a Turtle document in which every triple is a goal.
///
/// Prefix resolution and literal canonicalisation come from the same parser the
/// store uses, which is the ONLY way a goal's spelling can match the
/// interner's. A hand-typed goal that skipped the parser matches nothing and is
/// reported lost, which is a false loss with a green-looking pipeline around
/// it.
pub fn parse_goals_turtle(goals_ttl: &str) -> anyhow::Result<(Vec<Goal>, Vec<RefusedGoal>)> {
    let parsed = match GraphStore::parse_triples_ordered(goals_ttl, "turtle") {
        Ok(v) => v,
        Err(e) => {
            // All or nothing: this parser collects the whole document before
            // yielding anything, so one syntax error means no goals at all.
            // Reported as a refusal rather than an error so the caller sees it
            // in the same place every other rejected goal appears.
            return Ok((
                Vec::new(),
                vec![RefusedGoal {
                    as_written: truncate(goals_ttl, 200),
                    reason: GoalRefusal::Unparseable,
                    means: Box::leak(
                        format!("{}: {e}", GoalRefusal::Unparseable.means()).into_boxed_str(),
                    ),
                }],
            ));
        }
    };
    // The parser's rendering is `as_written`; the STORE's is what is asked.
    // They differ: oxigraph's parser keeps a literal's lexical form and the
    // store normalises it, so a goal that skipped the store matches nothing.
    let raw: Vec<(String, Spelled)> = parsed
        .into_iter()
        .map(|t| (format!("{} {} {}", t.0, t.1, t.2), t))
        .collect();
    canonicalise_batch(raw, Vec::new())
}

/// The `fol --goals` shape, so `derivations.tsv` pipes in with
/// `skip_columns = 1` (its first column is the rule name).
pub fn parse_goals_tsv(text: &str, skip_columns: usize) -> anyhow::Result<(Vec<Goal>, Vec<RefusedGoal>)> {
    let mut raw: Vec<(String, Spelled)> = Vec::new();
    let mut refused: Vec<RefusedGoal> = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < skip_columns + 3 {
            refused.push(refuse(&truncate(line, 200), GoalRefusal::Unparseable));
            continue;
        }
        let (s, p, o) = (f[skip_columns], f[skip_columns + 1], f[skip_columns + 2]);
        let written = format!("{s} {p} {o}");
        if NEGATION_MARKERS.iter().any(|m| written.starts_with(m)) {
            refused.push(refuse(&written, GoalRefusal::NotAPositiveGroundTriple));
            continue;
        }
        raw.push((written, (s.to_string(), p.to_string(), o.to_string())));
    }
    canonicalise_batch(raw, refused)
}

/// Goals from the basic graph pattern of the query an answer was rendered
/// from, with the answer's bindings substituted.
///
/// The source that needs no trust in a generator at all: it is fully
/// mechanical, and available to any system that builds answers from SPARQL.
/// A variable with no binding is refused by name rather than dropped.
pub fn goals_from_bindings(
    bgp: &[(String, String, String)],
    bindings: &BTreeMap<String, String>,
) -> anyhow::Result<(Vec<Goal>, Vec<RefusedGoal>)> {
    let mut raw: Vec<(String, Spelled)> = Vec::new();
    let mut refused: Vec<RefusedGoal> = Vec::new();
    for (s, p, o) in bgp {
        let sub = |t: &String| -> Option<String> {
            match t.strip_prefix('?').or_else(|| t.strip_prefix('$')) {
                Some(v) => bindings.get(v).cloned(),
                None => Some(t.clone()),
            }
        };
        let written = format!("{s} {p} {o}");
        match (sub(s), sub(p), sub(o)) {
            (Some(s), Some(p), Some(o)) => raw.push((written, (s, p, o))),
            _ => refused.push(refuse(&written, GoalRefusal::Unparseable)),
        }
    }
    canonicalise_batch(raw, refused)
}

/// Round-trip every candidate through a STORE in ONE pass, in document order,
/// keeping the pairing between what was written and what came back.
///
/// The store, not just the parser: `"01"^^xsd:integer` survives
/// `RdfParser` unchanged and comes out of a store as `"1"^^xsd:integer`, so a
/// goal canonicalised only by the parser matches nothing in a certificate and
/// is reported LOST. That is a false loss with a green-looking pipeline round
/// it, and it is the reason `as_written` and `goal` are both printed.
fn canonicalise_batch(
    raw: Vec<(String, Spelled)>,
    mut refused: Vec<RefusedGoal>,
) -> anyhow::Result<(Vec<Goal>, Vec<RefusedGoal>)> {
    let inputs: Vec<Spelled> = raw.iter().map(|(_, t)| t.clone()).collect();
    let canonical = GraphStore::canonicalise_triples(&inputs)?;
    let mut goals = Vec::new();
    for (i, ((written, _), c)) in raw.iter().zip(canonical).enumerate() {
        let Some(t) = c else {
            // The store refused it. A literal subject is the realistic case and
            // it is not a positive ground triple; anything else the parser
            // rejected is unparseable.
            let why = if raw[i].1 .0.starts_with('"') {
                GoalRefusal::NotAPositiveGroundTriple
            } else {
                GoalRefusal::Unparseable
            };
            refused.push(refuse(written, why));
            continue;
        };
        match classify(&t, written) {
            Ok(()) => goals.push(accept(i, written.clone(), t)),
            Err(r) => refused.push(refuse(written, r)),
        }
    }
    Ok((goals, refused))
}

fn truncate(s: &str, n: usize) -> String {
    let t: String = s.chars().take(n).collect();
    if s.chars().count() > n { format!("{t}…") } else { t }
}

// ───────────────────────────────────────────────────────────────────────────
// The certificate, read back as the checker will read it
// ───────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Membership {
    /// In `asserted.tsv`. No derivation exists and none is needed.
    Asserted,
    /// Concluded by a line in the derivation file.
    Derived,
    /// In neither. Under this profile and this rule table only; the engine
    /// covers 29 of OWL 2 RL's 78 rules and this is not "false".
    NotDerivable,
}

/// Which certificate format a run wrote, and therefore which checker reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CertKind {
    /// `asserted.tsv` + `derivations.tsv`, checked by `oo-cert`.
    OoCert,
    /// `rules.tsv` + `asserted.tsv` + `horn.tsv`, checked by `oo-horn check`.
    OoHorn,
}

impl CertKind {
    pub fn derivations_file(self) -> &'static str {
        match self {
            CertKind::OoCert => "derivations.tsv",
            CertKind::OoHorn => "horn.tsv",
        }
    }
    pub fn binary(self) -> &'static str {
        match self {
            CertKind::OoCert => "oo-cert",
            CertKind::OoHorn => "oo-horn",
        }
    }
}

/// One run's certificate, indexed for lookup and slicing.
///
/// Raw lines are kept so a slice is byte-identical to what the emitter wrote.
pub struct CertificateIndex {
    dir: PathBuf,
    kind: CertKind,
    asserted: HashSet<Spelled>,
    lines: Vec<String>,
    conclusions: Vec<Spelled>,
    premises: Vec<Vec<Spelled>>,
    rules: Vec<String>,
    by_conclusion: HashMap<Spelled, usize>,
}

impl CertificateIndex {
    pub fn read(dir: &Path) -> anyhow::Result<Self> {
        let kind = if dir.join("horn.tsv").exists() { CertKind::OoHorn } else { CertKind::OoCert };
        let asserted_text = std::fs::read_to_string(dir.join("asserted.tsv"))
            .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", dir.join("asserted.tsv").display()))?;
        let mut asserted = HashSet::new();
        for line in asserted_text.lines() {
            if line.is_empty() {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() >= 3 {
                asserted.insert((f[0].to_string(), f[1].to_string(), f[2].to_string()));
            }
        }
        let dpath = dir.join(kind.derivations_file());
        let dtext = std::fs::read_to_string(&dpath)
            .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", dpath.display()))?;

        let mut lines = Vec::new();
        let mut conclusions = Vec::new();
        let mut premises = Vec::new();
        let mut rules = Vec::new();
        let mut by_conclusion: HashMap<Spelled, usize> = HashMap::new();
        for line in dtext.lines() {
            if line.is_empty() {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            // `oo-cert`: rule, then 3(1+k) term fields.
            // `oo-horn`: rule index, binding count b, 2b binding fields, then
            //            3(1+k) term fields.
            let (rule, start) = match kind {
                CertKind::OoCert => (f.first().copied().unwrap_or("").to_string(), 1),
                CertKind::OoHorn => {
                    let b: usize = f.get(1).and_then(|x| x.parse().ok()).unwrap_or(0);
                    (f.first().copied().unwrap_or("").to_string(), 2 + 2 * b)
                }
            };
            if f.len() < start + 3 || !(f.len() - start).is_multiple_of(3) {
                anyhow::bail!(
                    "{} line {} is not a {} step",
                    dpath.display(),
                    lines.len() + 1,
                    kind.binary()
                );
            }
            let terms: Vec<Spelled> = f[start..]
                .chunks(3)
                .map(|c| (c[0].to_string(), c[1].to_string(), c[2].to_string()))
                .collect();
            let idx = lines.len();
            let conclusion = terms[0].clone();
            by_conclusion.entry(conclusion.clone()).or_insert(idx);
            conclusions.push(conclusion);
            premises.push(terms[1..].to_vec());
            rules.push(rule);
            lines.push(line.to_string());
        }
        Ok(CertificateIndex {
            dir: dir.to_path_buf(),
            kind,
            asserted,
            lines,
            conclusions,
            premises,
            rules,
            by_conclusion,
        })
    }

    pub fn kind(&self) -> CertKind {
        self.kind
    }
    pub fn dir(&self) -> &Path {
        &self.dir
    }
    pub fn asserted_path(&self) -> PathBuf {
        self.dir.join("asserted.tsv")
    }
    pub fn rules_path(&self) -> PathBuf {
        self.dir.join("rules.tsv")
    }
    pub fn asserted_count(&self) -> usize {
        self.asserted.len()
    }
    pub fn derivation_count(&self) -> usize {
        self.lines.len()
    }
    pub fn rule_of(&self, i: usize) -> Option<&str> {
        self.rules.get(i).map(|s| s.as_str())
    }
    pub fn premises_of(&self, i: usize) -> Option<&[Spelled]> {
        self.premises.get(i).map(|v| v.as_slice())
    }
    pub fn conclusion_index(&self, t: &Spelled) -> Option<usize> {
        self.by_conclusion.get(t).copied()
    }
    pub fn conclusion_of_line(&self, i: usize) -> Option<&Spelled> {
        self.conclusions.get(i)
    }

    /// `asserted ⊎ {conclusions}`. The `emit` closure in `src/reason.rs`
    /// records a derivation only when the triple is not already in the closure,
    /// so the two sets are disjoint and this IS the closure the run reached.
    pub fn closure(&self) -> HashSet<Spelled> {
        let mut c = self.asserted.clone();
        for t in &self.conclusions {
            c.insert(t.clone());
        }
        c
    }

    pub fn asserted_set(&self) -> &HashSet<Spelled> {
        &self.asserted
    }

    pub fn membership(&self, t: &Spelled) -> Membership {
        // Asserted FIRST. The certificate holds only INFERRED triples, so a
        // goal that is literally in the slice has no derivation line at all,
        // and an implementation that searches the derivations first finds
        // nothing and reports the goal lost. That is inverted, and the
        // happy-path test would never catch it because it naturally uses a
        // derived goal.
        if self.asserted.contains(t) {
            return Membership::Asserted;
        }
        if self.by_conclusion.contains_key(t) {
            return Membership::Derived;
        }
        Membership::NotDerivable
    }

    /// Line indices of a certificate that concludes `t`, closed under premises
    /// and in ASCENDING original order.
    ///
    /// The order is valid and it is provable from the emitter rather than hoped
    /// for: `run_full` computes a round's conclusions from `triple_set` as it
    /// stood at the START of that round and inserts them only afterwards, so
    /// every premise of a line was asserted or concluded strictly earlier in
    /// the file. `oo-cert` rejects a premise that is "neither asserted nor
    /// derived earlier", so this is load-bearing and is pinned by a test.
    pub fn slice_for(&self, t: &Spelled) -> Option<Vec<usize>> {
        let root = *self.by_conclusion.get(t)?;
        let mut want: BTreeSet<usize> = BTreeSet::new();
        let mut stack = vec![root];
        while let Some(i) = stack.pop() {
            if !want.insert(i) {
                continue;
            }
            for p in &self.premises[i] {
                if self.asserted.contains(p) {
                    continue;
                }
                if let Some(&j) = self.by_conclusion.get(p)
                    && j < i
                    && !want.contains(&j)
                {
                    stack.push(j);
                }
            }
        }
        Some(want.into_iter().collect())
    }

    /// Write a sub-certificate's derivation lines.
    ///
    /// `asserted.tsv` is NOT copied: the caller passes the run's single
    /// [`asserted_path`](Self::asserted_path) to the checker, so 200 goals over
    /// a 37k-derivation run do not write 200 copies of the graph.
    pub fn write_slice(&self, lines: &[usize], out_derivations: &Path) -> anyhow::Result<()> {
        if let Some(parent) = out_derivations.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut body = String::new();
        for &i in lines {
            body.push_str(&self.lines[i]);
            body.push('\n');
        }
        std::fs::write(out_derivations, body)?;
        Ok(())
    }
}

// ───────────────────────────────────────────────────────────────────────────
// The checker
// ───────────────────────────────────────────────────────────────────────────

/// An acceptance. The fields are private and the only constructor is
/// [`Accepted::from_run`], which takes a [`Certified`] — and that type has no
/// public constructor at all, so this struct cannot be built by a caller that
/// did not run a checker and read a zero exit code.
///
/// It serialises as the two fields it always had, flattened under the `status`
/// tag by serde's internally-tagged newtype-variant handling, so the wire
/// format is byte-for-byte what it was.
#[derive(Clone, Debug)]
pub struct Accepted {
    certified: Certified,
    stdout: String,
}

impl Accepted {
    fn from_run(certified: Certified, stdout: String) -> Accepted {
        Accepted { certified, stdout }
    }
    /// The evidence, so a downstream verdict carries the token this run
    /// earned rather than re-asserting the acceptance on its own authority.
    pub fn certified(&self) -> Certified {
        self.certified
    }
    pub fn theorem(&self) -> &'static str {
        self.certified.theorem()
    }
    pub fn stdout(&self) -> &str {
        &self.stdout
    }
}

impl Serialize for Accepted {
    /// `{"theorem": ..., "stdout": ...}`, exactly the two fields the struct
    /// variant carried, so the tagged enum above still writes
    /// `{"status":"accepted","theorem":...,"stdout":...}`.
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct as _;
        let mut st = s.serialize_struct("Accepted", 2)?;
        st.serialize_field("theorem", self.certified.theorem())?;
        st.serialize_field("stdout", &self.stdout)?;
        st.end()
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CheckerStatus {
    Accepted(Accepted),
    /// Exit 1. This module built the slice, so a rejection is a defect HERE or
    /// in the emitter. It is never a downgrade to an unchecked verdict.
    Rejected { stdout: String },
    /// Exit 2: a file could not be read or parsed. Not a verdict in either
    /// direction, exactly as `lean/Main.lean` reserves the two codes.
    Unreadable { stdout: String },
    /// Skip loudly, with the install line, and report the unchecked verdict
    /// honestly. Never pass silently.
    Absent { what: String, install: &'static str },
    /// The checker was found and no goal needed it, because no preserved goal
    /// was DERIVED and a goal that is literally in the slice has no derivation
    /// to check. Its own word, because calling that `unreadable` would put a
    /// file-format failure's name on a run where nothing went wrong, and
    /// calling it `accepted` would be worse.
    NotNeeded { what: String },
}

impl CheckerStatus {
    pub fn is_absent(&self) -> bool {
        matches!(self, CheckerStatus::Absent { .. })
    }
    pub fn word(&self) -> &'static str {
        match self {
            CheckerStatus::Accepted(_) => "accepted",
            CheckerStatus::Rejected { .. } => "rejected",
            CheckerStatus::Unreadable { .. } => "unreadable",
            CheckerStatus::Absent { .. } => "absent",
            CheckerStatus::NotNeeded { .. } => "not_needed",
        }
    }
}

/// Find a Lean checker binary.
///
/// Mirrors the search every other certified path in this repository does, for
/// the reason those give: a run that quietly dropped to the engine's own
/// opinion because the checker was not built would look exactly like a run
/// where the checker had agreed.
pub fn find_checker(kind: CertKind, explicit: Option<&Path>) -> Result<PathBuf, String> {
    let name = kind.binary();
    if let Some(p) = explicit {
        return if p.exists() {
            Ok(p.to_path_buf())
        } else {
            Err(format!("{} does not exist", p.display()))
        };
    }
    let env_key = match kind {
        CertKind::OoCert => "OO_CERT",
        CertKind::OoHorn => "OO_HORN",
    };
    if let Ok(v) = std::env::var(env_key) {
        let p = PathBuf::from(&v);
        return if p.exists() { Ok(p) } else { Err(format!("${env_key} points at {v}, which does not exist")) };
    }
    let mut tried = Vec::new();
    for base in [
        PathBuf::from("."),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")),
    ] {
        let p = base.join("lean").join(".lake").join("build").join("bin").join(name);
        if p.exists() {
            return Ok(p);
        }
        tried.push(p.display().to_string());
    }
    if let Ok(out) = std::process::Command::new(name).arg("--help").output() {
        // A bare invocation with no arguments prints usage and exits 2, which
        // is enough to know the binary is on PATH.
        let _ = out;
        return Ok(PathBuf::from(name));
    }
    Err(format!("{name} not found (looked at ${env_key}, {}, and $PATH)", tried.join(", ")))
}

/// Run `oo-cert ASSERTED DERIVATIONS` and turn its exit code into a verdict.
pub fn run_oo_cert(checker: Option<&Path>, asserted: &Path, derivations: &Path) -> CheckerStatus {
    run_checker(CertKind::OoCert, checker, asserted, derivations, None)
}

/// The ONE place a checked verdict is produced, and it has just read an exit
/// code off a Lean binary.
///
/// "The one place" is no longer a promise in a comment. The acceptance it
/// returns carries a [`Certified`], and [`crate::verdict::CheckerRun::spawn`]
/// is the only thing in the crate that can mint one, so a second place would
/// have to run a checker too.
pub fn run_checker(
    kind: CertKind,
    checker: Option<&Path>,
    asserted: &Path,
    derivations: &Path,
    rules: Option<&Path>,
) -> CheckerStatus {
    let bin = match find_checker(kind, checker) {
        Ok(b) => b,
        Err(why) => {
            return CheckerStatus::Absent { what: why, install: LAKE_INSTALL };
        }
    };
    let mut cmd = std::process::Command::new(&bin);
    match kind {
        CertKind::OoCert => {
            cmd.arg(asserted).arg(derivations);
        }
        CertKind::OoHorn => {
            let Some(r) = rules else {
                return CheckerStatus::Absent {
                    what: "oo-horn needs the rules.tsv the run evaluated, and none was written"
                        .to_string(),
                    install: LAKE_INSTALL,
                };
            };
            cmd.arg("check").arg(r).arg(asserted).arg(derivations);
        }
    }
    let run = match CheckerRun::spawn(&CheckerBinary::found_at(bin.clone()), cmd) {
        Ok(r) => r,
        Err(e) => {
            return CheckerStatus::Absent {
                what: format!("{} could not be run: {e}", bin.display()),
                install: LAKE_INSTALL,
            };
        }
    };
    let text = run.output();
    // The theorem name is handed to the evidence rather than to the report, so
    // a status with no acceptance behind it has no way to name one.
    let theorem = match kind {
        CertKind::OoCert => "OOCert.certificate_sound",
        CertKind::OoHorn => "OOCert.horn_certificate_sound",
    };
    match run.accepted(theorem) {
        Some(cert) => CheckerStatus::Accepted(Accepted::from_run(cert, text)),
        // `lean/Main.lean` reserves 1 for a rejection and 2 for a file it
        // could not read. Anything else is not a verdict in either direction.
        None => match run.exit() {
            1 => CheckerStatus::Rejected { stdout: text },
            2 => CheckerStatus::Unreadable { stdout: text },
            // `{:?}` on the raw `Option`, byte for byte what this arm has
            // always printed: `None` for a signal death, `Some(n)` otherwise.
            _ => CheckerStatus::Unreadable {
                stdout: format!("{} exited with {:?}\n{text}", bin.display(), run.code()),
            },
        },
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Verdicts
// ───────────────────────────────────────────────────────────────────────────

/// The per-goal verdict.
///
/// The two CHECKED variants carry a [`Certified`], which has no public
/// constructor, so neither can be named on a path that did not run the Lean
/// checker. `Deserialize` is deliberately not implemented: reading
/// `"preserved_checked"` out of somebody else's JSON is not the same act as
/// earning it, and a derive would be a public constructor for both.
///
/// ```compile_fail
/// use open_ontologies::projection_entailment::GoalVerdict;
/// use open_ontologies::verdict::Certified;
/// let v = GoalVerdict::PreservedChecked(Certified { theorem: "OOCert.certificate_sound" });
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GoalVerdict {
    /// `P` entails `q`, machine-checked over `P`'s own asserted graph.
    PreservedChecked(Certified),
    /// The same, but the run evaluated a SUPPLIED Horn table. True in every
    /// model of `P` that ALSO satisfies those rules. Never shortened.
    PreservedUnderSuppliedRulesChecked(Certified),
    /// `q` is literally in the slice. Verified by set membership over the
    /// canonical spelling. Not a theorem and it does not pretend to be.
    PreservedAsserted,
    /// The ENGINE derived it and no checker looked.
    PreservedUnchecked,
    /// Not derivable from `P` under this profile. Bounded by a rule table
    /// implementing 29 of OWL 2 RL's 78 rules, so it is not "P does not entail
    /// q".
    LostUnderProfileUnchecked,
    /// Neither certificate has it. The source does not derive it either, so
    /// the projection did not lose it.
    UngroundedInSource,
    /// In `P`'s certificate and not in `G`'s. A defect report rather than a
    /// verdict; see the monotonicity block.
    ProjectionOnly,
    /// The Lean checker REJECTED the sub-certificate this module assembled for
    /// the goal. A defect HERE or in the emitter, never a weaker result about
    /// the ontology, and never a downgrade to an unchecked verdict.
    CertificateRejected,
    /// Not a positive ground triple.
    Refused,
}

impl GoalVerdict {
    /// The wire word. Byte-identical to what `#[serde(rename_all =
    /// "snake_case")]` used to derive, and the single source of the string now
    /// that the two checked variants carry a payload serde would otherwise
    /// have wrapped in an object.
    pub fn word(self) -> &'static str {
        match self {
            GoalVerdict::PreservedChecked(_) => "preserved_checked",
            GoalVerdict::PreservedUnderSuppliedRulesChecked(_) => {
                "preserved_under_supplied_rules_checked"
            }
            GoalVerdict::PreservedAsserted => "preserved_asserted",
            GoalVerdict::PreservedUnchecked => "preserved_unchecked",
            GoalVerdict::LostUnderProfileUnchecked => "lost_under_profile_unchecked",
            GoalVerdict::UngroundedInSource => "ungrounded_in_source",
            GoalVerdict::ProjectionOnly => "projection_only",
            GoalVerdict::CertificateRejected => "certificate_rejected",
            GoalVerdict::Refused => "refused",
        }
    }
    /// The theorem a verdict names, or `"none"`. A verdict with no theorem must
    /// never render a string containing "checked", and the suite asserts that
    /// over the SERIALISED report rather than over this function, because the
    /// enum is where the discipline is easy and the serialisation is where it
    /// leaks. The name now comes OUT OF THE EVIDENCE rather than out of a
    /// match arm, so a run that never ran the checker cannot name a theorem.
    pub fn warrant(self) -> &'static str {
        match self {
            GoalVerdict::PreservedChecked(c)
            | GoalVerdict::PreservedUnderSuppliedRulesChecked(c) => c.theorem(),
            _ => "none",
        }
    }
    pub fn is_checked(self) -> bool {
        self.warrant() != "none"
    }
    pub fn is_preserved(self) -> bool {
        matches!(
            self,
            GoalVerdict::PreservedChecked(_)
                | GoalVerdict::PreservedUnderSuppliedRulesChecked(_)
                | GoalVerdict::PreservedAsserted
                | GoalVerdict::PreservedUnchecked
        )
    }
    pub fn means(self) -> &'static str {
        match self {
            GoalVerdict::PreservedChecked(_) =>
                "the projection entails this goal: true in every model of the projection under the \
                 semantics in lean/OOCert/Semantics.lean. Machine-checked by oo-cert over the \
                 projection's own asserted.tsv",
            GoalVerdict::PreservedUnderSuppliedRulesChecked(_) =>
                "true in every model of the projection THAT ALSO SATISFIES the supplied rule \
                 table. The rules are assumed and never checked; a rule reading 'every supplier \
                 is compliant' produces certificates that check green for ever. Never shorten \
                 this to the plain preserved word",
            GoalVerdict::PreservedAsserted =>
                "the goal is literally in the slice, by exact-string membership in the \
                 projection's asserted.tsv. No rule fired, nothing was proved, and nothing needed \
                 to be. This is a lookup, not a theorem",
            GoalVerdict::PreservedUnchecked =>
                "the ENGINE derived it from the projection and no checker has looked. This is the \
                 engine's opinion about its own output, which is exactly what the certificate \
                 layer exists to stop being trusted",
            GoalVerdict::LostUnderProfileUnchecked =>
                "the source derives it and the projection does not, under this profile. The \
                 retrieval finding. The engine implements 29 of OWL 2 RL's 78 rules, so this is \
                 bounded by the rule table and is NOT 'the projection does not entail it'",
            GoalVerdict::UngroundedInSource =>
                "NEITHER the source nor the projection derives it. The projection did not lose \
                 it: the claim has no support in the knowledge graph under this profile, so the \
                 fix is in the generator and not the retriever. Open-world: this is not 'false'",
            GoalVerdict::ProjectionOnly =>
                "the projection derives it and the source does not. Either the projection is not \
                 a subset of the source, or OWL RL monotonicity has been violated and the engine \
                 is unsound. Read the monotonicity block; this is never a retrieval finding",
            GoalVerdict::CertificateRejected => CERTIFICATE_REJECTED_MEANS,
            GoalVerdict::Refused =>
                "not a positive ground triple, so it was never asked. Counted in the headline so \
                 a refusal cannot shrink the denominator unnoticed",
        }
    }
}

impl Serialize for GoalVerdict {
    /// A bare string, exactly as `#[serde(rename_all = "snake_case")]` wrote
    /// it before the checked variants grew a payload.
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.word())
    }
}

/// So the suite can go on asserting against the wire word without being able
/// to build a checked variant to compare with.
impl PartialEq<&str> for GoalVerdict {
    fn eq(&self, other: &&str) -> bool {
        self.word() == *other
    }
}

impl std::fmt::Display for GoalVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.word())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GoalCertificate {
    pub dir: String,
    pub format: &'static str,
    pub steps: usize,
    pub asserted: usize,
    /// The exact command a reader can run by hand.
    pub check_with: String,
}

/// Not `Deserialize`: it carries a [`GoalVerdict`], whose checked variants
/// are evidence and not words. Reading `"preserved_checked"` out of somebody
/// else's JSON is not the same act as earning it, and a derive here would be a
/// public constructor for the certified state. Reports are read back as
/// `serde_json::Value`, which is what every caller in the tree already does.
#[derive(Clone, Debug, Serialize)]
pub struct GoalReport {
    pub id: String,
    pub as_written: String,
    /// The canonical spelling actually asked. Printed even when it equals
    /// `as_written`, because the case where it differs is the one that bites.
    pub goal: String,
    pub source: Membership,
    pub projection: Membership,
    pub verdict: GoalVerdict,
    pub warrant: &'static str,
    pub means: &'static str,
    /// Present only for a preserved-and-checked goal.
    pub certificate: Option<GoalCertificate>,
    /// True when either run stopped at the iteration cap. Every NEGATIVE
    /// verdict from such a run is a lower bound, not a fact.
    pub bounded_by_iteration_cap: bool,
    /// The goal mentions no term any seed neighbourhood reaches. Usually a
    /// retriever misconfiguration rather than a loss, so it is named separately
    /// instead of being sent to the loss pile.
    pub goal_outside_seed_neighbourhood: bool,
}

// ───────────────────────────────────────────────────────────────────────────
// Subsethood and the differential
// ───────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubsetReport {
    /// Computed, never inferred from provenance. A retriever that normalises
    /// IRIs, re-prefixes, expands an abbreviation, inlines an imported
    /// vocabulary or adds a tidy `rdf:type owl:Class` declaration produces a
    /// non-subset that looks like a faithful slice.
    pub verified: bool,
    pub decided_over: &'static str,
    pub extra_triples: Vec<String>,
    pub extra_count: usize,
    pub blank_node_bearing_source: usize,
    pub blank_node_bearing_projection: usize,
    /// True when a blank node on either side has no binding across the two
    /// graphs, so subsethood over those triples is UNDECIDED rather than false.
    pub blank_nodes_unmatched: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Guards {
    pub subset_verified: bool,
    pub blank_nodes_unmatched: bool,
    pub both_reached_fixpoint: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MonotonicityReport {
    /// `"armed"` or `"disarmed"`. A green `violations: []` under `"disarmed"`
    /// means "we did not look", and the two must never render the same.
    pub status: &'static str,
    pub reason: &'static str,
    pub violations: Vec<String>,
    pub violation_count: usize,
    pub disagreement: Option<Disagreement>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Disagreement {
    pub severity: &'static str,
    pub what: &'static str,
    pub detail: String,
    pub means: &'static str,
}

pub const MONOTONICITY_MEANS: &str =
    "OWL RL is monotone and this projection IS a subset of the source, so every conclusion of the \
     projection must be a conclusion of the source. This one is not. That is a SOUNDNESS BUG IN \
     THE ENGINE, not a finding about the retrieval, and not something to file under 'other'. Do \
     not ship a run that reports this.";

pub const CERTIFICATE_REJECTED_MEANS: &str =
    "the Lean checker rejected a sub-certificate THIS MODULE assembled from the engine's own \
     derivation file. That is a defect in the slice extractor or in the emitter's premise order, \
     not a weaker result about the ontology. It does not downgrade to preserved_unchecked, \
     because that would turn a defect in this code into a slightly weaker verdict nobody \
     investigates.";

/// PURE, so the gate can be shown failing without an unsound engine to hand.
///
/// The engine cannot be made unsound on demand, so the only honest way to show
/// that this gate can fire is to hand it a pair of closures where the
/// projection's exceeds the source's with both guards true.
pub fn differential(
    source_closure: &HashSet<Spelled>,
    projection_closure: &HashSet<Spelled>,
    guards: Guards,
) -> MonotonicityReport {
    let disarm = if !guards.subset_verified {
        Some("projection_is_not_a_subset")
    } else if guards.blank_nodes_unmatched {
        Some("blank_nodes_unmatched")
    } else if !guards.both_reached_fixpoint {
        Some("fixpoint_not_reached")
    } else {
        None
    };
    if let Some(reason) = disarm {
        return MonotonicityReport {
            status: "disarmed",
            reason,
            violations: Vec::new(),
            violation_count: 0,
            disagreement: None,
        };
    }
    let mut extra: Vec<String> = projection_closure
        .difference(source_closure)
        .map(|t| format!("{} {} {}", t.0, t.1, t.2))
        .collect();
    extra.sort();
    let count = extra.len();
    let shown: Vec<String> = extra.into_iter().take(50).collect();
    let disagreement = if count == 0 {
        None
    } else {
        Some(Disagreement {
            severity: STOP_THE_LINE,
            what: "monotonicity_violated",
            detail: format!(
                "{count} conclusion(s) of the projection are not conclusions of the source, over a \
                 projection verified to be a subset, with both runs at a fixpoint"
            ),
            means: MONOTONICITY_MEANS,
        })
    };
    MonotonicityReport {
        status: "armed",
        reason: "",
        violations: shown,
        violation_count: count,
        disagreement,
    }
}

// ───────────────────────────────────────────────────────────────────────────
// The demoted proxy
// ───────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoverageProxy {
    /// `None` when no seeds were supplied or the slice did not parse, so
    /// "we did not compute it" can never render as "0% coverage".
    pub report: Option<ProjectionLossReport>,
    pub not_computed_because: Option<String>,
    /// Always false. A field rather than a doc comment, so a consumer reading
    /// JSON sees it without reading prose.
    pub is_a_warrant: bool,
    pub label: &'static str,
    /// Seeds whose neighbourhood hit the `LIMIT 1000` in
    /// `projection_check::neighbourhood_pairs`, so their ratio was computed on
    /// a truncated source. A truncated proxy reading 1.0 is the green-metric
    /// trap one level down.
    pub truncated_seeds: Vec<String>,
    /// The proxy is subject-only: an inbound-edge loss is invisible to it.
    /// Named here rather than fixed, because that is its own change.
    pub known_limits: &'static str,
}

const PROXY_LIMITS: &str =
    "the proxy counts a seed's OUTBOUND (predicate, object) pairs only, so a lost inbound edge is \
     invisible to it, and its SPARQL carries LIMIT 1000, so a seed with more than a thousand \
     outbound triples has its ratio computed against a truncated source. Both are pre-existing \
     and are named rather than fixed here.";

// ───────────────────────────────────────────────────────────────────────────
// Hygiene of the source side
// ───────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceHygiene {
    /// Non-zero means an earlier `reason` run materialised into this store, so
    /// `asserted.tsv` on the source side lists the engine's own output as an
    /// axiom. Every `ungrounded_in_source` then becomes unreachable and every
    /// goal looks supported. Decision 0005 item 7 records this exact defect,
    /// found inside this repository, in a different tool.
    pub materialised_inferences_in_source: usize,
    pub source_side_unreliable: bool,
    pub means: &'static str,
}

// ───────────────────────────────────────────────────────────────────────────
// Options and the report
// ───────────────────────────────────────────────────────────────────────────

/// Where the slice came from. The arms differ in what can be PROVED about them,
/// not in convenience.
#[derive(Clone, Copy, Debug)]
pub enum Projection<'a> {
    /// Turtle text, re-parsed into a fresh store. Blank nodes are relabelled on
    /// the way in, so subsethood is decidable only over ground triples and the
    /// differential is disarmed whenever either side carries one.
    Turtle(&'a str),
    /// A named graph of the SAME store the source is read from. Blank node
    /// identity survives because nothing is re-parsed, so `P ⊆ G` is decided by
    /// set membership over every triple and the differential can be armed.
    NamedGraph(&'a str),
}

#[derive(Clone, Debug)]
pub struct Opts {
    /// ONE profile for BOTH runs. Two profiles compare two rule sets, and the
    /// differential then fires on nothing at all.
    pub profile: String,
    /// Optional supplied Horn table. Its presence changes the verdict WORD for
    /// every preserved goal.
    pub rules: Option<PathBuf>,
    /// Where the two run certificates and the per-goal slices land. Every
    /// intermediate file stays, so a run is reproducible by hand.
    pub work_dir: PathBuf,
    /// Explicit checker path. Defaults to `$OO_CERT`, then the repository's
    /// `lean/.lake/build/bin/`, then `$PATH`.
    pub checker: Option<PathBuf>,
    /// Turn an absent checker into an error instead of an honest unchecked
    /// verdict. The CI leg sets this.
    pub require_checker: bool,
    /// Seeds for the demoted coverage proxy. Independent of the goals, and
    /// never derived from them: a goal can mention IRIs no seed neighbourhood
    /// reaches, and coverage over the seeds says nothing about such a goal.
    pub seed_iris: Vec<String>,
}

impl Default for Opts {
    fn default() -> Self {
        Opts {
            profile: "owl-rl".into(),
            rules: None,
            work_dir: PathBuf::from("projection-entailment"),
            checker: None,
            require_checker: false,
            seed_iris: Vec::new(),
        }
    }
}

/// Not `Deserialize`, for the reason on [`GoalReport`].
#[derive(Clone, Debug, Serialize)]
pub struct PreservationReport {
    pub format: &'static str,
    /// One sentence a reader cannot miss, first in the JSON after the format.
    pub headline: String,
    pub profile: String,
    pub rules: Option<String>,
    pub rules_sha256: Option<String>,
    pub goals_total: usize,
    pub preserved_checked: usize,
    pub preserved_asserted: usize,
    pub preserved_unchecked: usize,
    pub lost: usize,
    pub ungrounded_in_source: usize,
    pub projection_only: usize,
    /// Goals whose sub-certificate the checker rejected. Non-zero means a
    /// defect in this module or in the emitter, so it is counted apart from
    /// everything else and always comes with a stop-the-line block.
    pub certificate_rejected: usize,
    pub refused: usize,
    pub per_goal: Vec<GoalReport>,
    pub refused_goals: Vec<RefusedGoal>,
    pub subset: SubsetReport,
    pub monotonicity: MonotonicityReport,
    /// The stop-the-line block, if one fired. Either the monotonicity
    /// violation, or a sub-certificate the checker REJECTED, which is a defect
    /// in this module or in the emitter and not a weaker result about the
    /// ontology. Kept at the top level because a rejection is not a statement
    /// about monotonicity and must not be filed inside its block.
    pub disagreement: Option<Disagreement>,
    pub coverage_proxy: CoverageProxy,
    pub checker: CheckerStatus,
    pub source_hygiene: SourceHygiene,
    pub source_fixpoint_reached: bool,
    pub projection_fixpoint_reached: bool,
    pub source_iterations: usize,
    pub projection_iterations: usize,
    pub iteration_cap: usize,
    pub work_dir: String,
    /// Known limits of what a `preserved_checked` says, in the payload rather
    /// than only in the docs.
    pub limits: &'static str,
    /// True iff nothing was lost, nothing stopped the line, and no goal was
    /// refused. `ungrounded_in_source` does NOT clear it: a claim with no
    /// support in the graph is a finding, not a pass.
    pub ok: bool,
    /// 0 clean, 1 something was lost, ungrounded or refused, 2 stop-the-line.
    pub exit_code: i32,
}

const LIMITS: &str =
    "OOCert's entailment relation is not datatype-aware: \"1\"^^xsd:int and \"1\"^^xsd:integer are \
     two distinct terms, so a preserved_checked on one says nothing about the other. A lost_* \
     verdict is an engine opinion about a NEGATIVE and carries no certificate in this layer, \
     ever. The rule table is this engine's, not W3C's: 29 of OWL 2 RL's 78 rules.";

// ───────────────────────────────────────────────────────────────────────────
// The entry point
// ───────────────────────────────────────────────────────────────────────────

struct SideRun {
    fixpoint_reached: bool,
    iterations: usize,
    rules_sha256: Option<String>,
}

fn reason_side(store: &Arc<GraphStore>, opts: &Opts, dir: &Path) -> anyhow::Result<SideRun> {
    std::fs::create_dir_all(dir)?;
    let json: serde_json::Value = if let Some(rules) = &opts.rules {
        serde_json::from_str(&Reasoner::run_horn(store, rules, dir)?)?
    } else {
        // `materialize = false` ALWAYS. `run_full` computes the closure in
        // memory and writes the certificate regardless, so false costs nothing
        // and reasoning into the store would put this run's conclusions into
        // the next run's asserted.tsv.
        serde_json::from_str(&Reasoner::run_full(
            store,
            &opts.profile,
            false,
            InferenceTarget::DefaultGraph,
            Some(dir),
        )?)?
    };
    if let Some(e) = json.get("error").and_then(|v| v.as_str()) {
        anyhow::bail!("{e}");
    }
    Ok(SideRun {
        fixpoint_reached: json["fixpoint_reached"].as_bool().unwrap_or(false),
        iterations: json["iterations"].as_u64().unwrap_or(0) as usize,
        rules_sha256: json["certificate"]["rules_tsv_sha256"].as_str().map(|s| s.to_string()),
    })
}

/// Ask whether a retrieved slice still supports the claims an answer rests on.
///
/// Runs the reasoner twice with `materialize = false` into
/// `opts.work_dir/{source,projection}`, decides every goal over the certificate
/// FILES rather than an in-memory closure the checker never sees, extracts and
/// checks a sub-certificate per preserved goal, and runs the monotonicity
/// differential.
///
/// Errors rather than returning a green report when `goals` is empty: every
/// aggregate over zero goals is trivially perfect, which is decision 0006 item
/// 8's empty-problem shape, and "all my goals were refused" is the realistic
/// way to arrive at zero.
pub fn check_entailment_preservation(
    graph: &Arc<GraphStore>,
    projection: Projection<'_>,
    goals: &[Goal],
    refused: &[RefusedGoal],
    opts: &Opts,
) -> anyhow::Result<PreservationReport> {
    if goals.is_empty() {
        anyhow::bail!(
            "no goals to ask. {} goal(s) were refused as not positive ground triples or \
             unparseable, and every aggregate over zero goals is trivially perfect, so a report \
             here would be a green run that says nothing",
            refused.len()
        );
    }
    if opts.profile == "owl-dl" {
        anyhow::bail!(
            "owl-dl emits no rule trace, so neither side could be certified and every verdict \
             would collapse to the engine's opinion while still looking like a certified run. \
             Run rdfs, owl-rl or owl-rl-ext"
        );
    }

    // ── The projection store ────────────────────────────────────────────
    let (pstore, projected_ttl, projection_kind): (Arc<GraphStore>, String, &'static str) =
        match projection {
            Projection::Turtle(ttl) => {
                let s = GraphStore::new();
                s.load_turtle(ttl, None).map_err(|e| {
                    let hint = if ttl.contains("<_:") {
                        ". The slice contains `<_:`, an angle-bracketed blank node label, which \
                         is not a legal IRI. That is what onto_segment_retrieve emits for a \
                         blank-node object; skolemise the source before retrieving"
                    } else {
                        ""
                    };
                    anyhow::anyhow!(
                        "the projection is not readable as Turtle, so there is no slice to ask \
                         about: {e}{hint}"
                    )
                })?;
                (Arc::new(s), ttl.to_string(), "turtle")
            }
            Projection::NamedGraph(name) => {
                let s = graph.graph_store(name)?;
                if s.triple_count() == 0 {
                    anyhow::bail!(
                        "named graph <{name}> holds no triples in the loaded store, so there is \
                         no slice to ask about"
                    );
                }
                let ttl = s.serialize("turtle").unwrap_or_default();
                (Arc::new(s), ttl, "named_graph")
            }
        };

    // ── Hygiene of the source side, before anything is computed from it ──
    let materialised = graph.materialised_inference_count().unwrap_or(0);
    let hygiene = SourceHygiene {
        materialised_inferences_in_source: materialised,
        source_side_unreliable: materialised > 0,
        means: if materialised > 0 {
            "an earlier reason run materialised into this store, so the source side's \
             asserted.tsv lists the engine's own output as an axiom. ungrounded_in_source becomes \
             unreachable and every goal looks supported. Reload the source from its files"
        } else {
            "no materialised inferences in the source store, so asserted.tsv is what was written"
        },
    };

    // ── Subsethood, computed rather than assumed from provenance ────────
    let src_triples: HashSet<Spelled> = graph.all_triples()?.into_iter().collect();
    let proj_triples: Vec<Spelled> = pstore.all_triples()?;
    let bn_src = src_triples.iter().filter(|t| has_blank(t)).count();
    let bn_proj = proj_triples.iter().filter(|t| has_blank(t)).count();
    let ground_only = projection_kind == "turtle" && (bn_src > 0 || bn_proj > 0);
    let mut extras: Vec<String> = Vec::new();
    let mut extra_count = 0usize;
    for t in &proj_triples {
        if ground_only && has_blank(t) {
            continue;
        }
        if !src_triples.contains(t) {
            extra_count += 1;
            if extras.len() < 25 {
                extras.push(format!("{} {} {}", t.0, t.1, t.2));
            }
        }
    }
    extras.sort();
    let subset = SubsetReport {
        verified: extra_count == 0,
        decided_over: if ground_only { "ground triples only" } else { "all triples" },
        extra_triples: extras,
        extra_count,
        blank_node_bearing_source: bn_src,
        blank_node_bearing_projection: bn_proj,
        blank_nodes_unmatched: ground_only,
    };

    // ── Two runs, one profile ───────────────────────────────────────────
    let work = &opts.work_dir;
    let src_dir = work.join("source");
    let prj_dir = work.join("projection");
    let src_run = reason_side(graph, opts, &src_dir)?;
    let prj_run = reason_side(&pstore, opts, &prj_dir)?;
    let src_cert = CertificateIndex::read(&src_dir)?;
    let prj_cert = CertificateIndex::read(&prj_dir)?;

    // ── The free differential ───────────────────────────────────────────
    let both_fixpoint = src_run.fixpoint_reached && prj_run.fixpoint_reached;
    let mono = differential(
        &src_cert.closure(),
        &prj_cert.closure(),
        Guards {
            subset_verified: subset.verified,
            blank_nodes_unmatched: subset.blank_nodes_unmatched,
            both_reached_fixpoint: both_fixpoint,
        },
    );

    // ── Seed neighbourhood, for goal_outside_seed_neighbourhood ─────────
    let seed_terms = seed_neighbourhood_terms(graph, &opts.seed_iris);

    // ── Per goal ────────────────────────────────────────────────────────
    let bounded = !both_fixpoint;
    let mut per_goal = Vec::with_capacity(goals.len());
    let mut checker_status: Option<CheckerStatus> = None;
    let mut stop_the_line: Option<Disagreement> = mono.disagreement.clone();
    for g in goals {
        let source = src_cert.membership(&g.triple);
        let projection = prj_cert.membership(&g.triple);
        let mut certificate = None;
        let verdict = match (source, projection) {
            (Membership::NotDerivable, Membership::NotDerivable) => GoalVerdict::UngroundedInSource,
            (Membership::NotDerivable, _) => GoalVerdict::ProjectionOnly,
            (_, Membership::NotDerivable) => GoalVerdict::LostUnderProfileUnchecked,
            // Asserted FIRST: see CertificateIndex::membership.
            (_, Membership::Asserted) => GoalVerdict::PreservedAsserted,
            (_, Membership::Derived) => {
                let lines = prj_cert.slice_for(&g.triple).unwrap_or_default();
                let out = work.join("goals").join(&g.id).join(prj_cert.kind().derivations_file());
                prj_cert.write_slice(&lines, &out)?;
                let rules_path = prj_cert.rules_path();
                let status = run_checker(
                    prj_cert.kind(),
                    opts.checker.as_deref(),
                    &prj_cert.asserted_path(),
                    &out,
                    if prj_cert.kind() == CertKind::OoHorn { Some(&rules_path) } else { None },
                );
                let v = match &status {
                    CheckerStatus::Accepted(a) => {
                        certificate = Some(GoalCertificate {
                            dir: out.parent().unwrap_or(&out).display().to_string(),
                            format: if prj_cert.kind() == CertKind::OoHorn { "oo-horn/1" } else { "oo-cert/1" },
                            steps: lines.len(),
                            asserted: prj_cert.asserted_count(),
                            check_with: check_command(&prj_cert, &out),
                        });
                        // The token this goal's own checker run produced. It
                        // carries the theorem, so the warrant printed beside
                        // the verdict cannot drift from the run that earned it.
                        if prj_cert.kind() == CertKind::OoHorn {
                            GoalVerdict::PreservedUnderSuppliedRulesChecked(a.certified())
                        } else {
                            GoalVerdict::PreservedChecked(a.certified())
                        }
                    }
                    CheckerStatus::Rejected { stdout } => {
                        // NOT a downgrade. This module built the slice.
                        stop_the_line.get_or_insert(Disagreement {
                            severity: STOP_THE_LINE,
                            what: "certificate_rejected",
                            detail: format!(
                                "the checker rejected the sub-certificate for goal {} ({}): {}",
                                g.id,
                                g.as_written,
                                truncate(stdout.trim(), 400)
                            ),
                            means: CERTIFICATE_REJECTED_MEANS,
                        });
                        GoalVerdict::CertificateRejected
                    }
                    CheckerStatus::Unreadable { .. }
                    | CheckerStatus::Absent { .. }
                    | CheckerStatus::NotNeeded { .. } => GoalVerdict::PreservedUnchecked,
                };
                if checker_status.is_none() || matches!(status, CheckerStatus::Rejected { .. }) {
                    checker_status = Some(status);
                }
                v
            }
        };
        let outside = !seed_terms.is_empty()
            && ![&g.triple.0, &g.triple.1, &g.triple.2].iter().any(|t| seed_terms.contains(t.as_str()));
        per_goal.push(GoalReport {
            id: g.id.clone(),
            as_written: g.as_written.clone(),
            goal: format!("{} {} {}", g.triple.0, g.triple.1, g.triple.2),
            source,
            projection,
            verdict,
            warrant: verdict.warrant(),
            means: verdict.means(),
            certificate,
            bounded_by_iteration_cap: bounded && !verdict.is_preserved(),
            goal_outside_seed_neighbourhood: outside,
        });
    }

    let checker = checker_status.unwrap_or_else(|| {
        // No goal needed a checker run. Say what would have happened rather
        // than imply one was consulted.
        match find_checker(prj_cert.kind(), opts.checker.as_deref()) {
            Ok(p) => CheckerStatus::NotNeeded {
                what: format!(
                    "{} was found and no goal needed it: no preserved goal was DERIVED, so there \
                     was no sub-certificate to check",
                    p.display()
                ),
            },
            Err(why) => CheckerStatus::Absent { what: why, install: LAKE_INSTALL },
        }
    });
    if opts.require_checker && checker.is_absent() {
        anyhow::bail!(
            "require_checker was set and the checker is absent: {}. {LAKE_INSTALL}",
            match &checker {
                CheckerStatus::Absent { what, .. } => what.as_str(),
                _ => "",
            }
        );
    }

    // ── The demoted proxy ───────────────────────────────────────────────
    let coverage_proxy = coverage_proxy(graph, &opts.seed_iris, &projected_ttl);

    // ── Counts and the verdict on the run ───────────────────────────────
    let count = |f: fn(GoalVerdict) -> bool| per_goal.iter().filter(|g| f(g.verdict)).count();
    let preserved_checked = count(|v| {
        matches!(
            v,
            GoalVerdict::PreservedChecked(_) | GoalVerdict::PreservedUnderSuppliedRulesChecked(_)
        )
    });
    let preserved_asserted = count(|v| v == GoalVerdict::PreservedAsserted);
    let preserved_unchecked = count(|v| v == GoalVerdict::PreservedUnchecked);
    let lost = count(|v| v == GoalVerdict::LostUnderProfileUnchecked);
    let ungrounded = count(|v| v == GoalVerdict::UngroundedInSource);
    let projection_only = count(|v| v == GoalVerdict::ProjectionOnly);
    let certificate_rejected = count(|v| v == GoalVerdict::CertificateRejected);

    let exit_code = if stop_the_line.is_some() {
        2
    } else if lost + ungrounded + projection_only + certificate_rejected + refused.len() > 0 {
        1
    } else {
        0
    };
    let ok = exit_code == 0;


    let headline = format!(
        "{preserved_checked} of {} goal(s) preserved with a machine-checked certificate, \
         {preserved_asserted} preserved by assertion, {preserved_unchecked} preserved on the \
         engine's word alone, {lost} LOST by the projection, {ungrounded} UNGROUNDED IN THE \
         SOURCE, {} refused. Monotonicity gate: {}{}. {}",
        goals.len(),
        refused.len(),
        mono.status,
        if mono.reason.is_empty() { String::new() } else { format!(" ({})", mono.reason) },
        COVERAGE_LABEL,
    );

    Ok(PreservationReport {
        format: "oo-preserve/1",
        headline,
        profile: if opts.rules.is_some() { "supplied-horn-table".into() } else { opts.profile.clone() },
        rules: opts.rules.as_ref().map(|p| p.display().to_string()),
        rules_sha256: prj_run.rules_sha256.clone().or(src_run.rules_sha256.clone()),
        goals_total: goals.len(),
        preserved_checked,
        preserved_asserted,
        preserved_unchecked,
        lost,
        ungrounded_in_source: ungrounded,
        projection_only,
        certificate_rejected,
        refused: refused.len(),
        per_goal,
        refused_goals: refused.to_vec(),
        subset,
        monotonicity: mono,
        disagreement: stop_the_line,
        coverage_proxy,
        checker,
        source_hygiene: hygiene,
        source_fixpoint_reached: src_run.fixpoint_reached,
        projection_fixpoint_reached: prj_run.fixpoint_reached,
        source_iterations: src_run.iterations,
        projection_iterations: prj_run.iterations,
        iteration_cap: crate::runtime::reasoner_max_iterations(),
        work_dir: work.display().to_string(),
        limits: LIMITS,
        ok,
        exit_code,
    })
}

fn check_command(cert: &CertificateIndex, slice: &Path) -> String {
    match cert.kind() {
        CertKind::OoCert => format!(
            "cd lean && lake exe oo-cert {} {}",
            cert.asserted_path().display(),
            slice.display()
        ),
        CertKind::OoHorn => format!(
            "cd lean && lake exe oo-horn check {} {} {}",
            cert.rules_path().display(),
            cert.asserted_path().display(),
            slice.display()
        ),
    }
}

/// Build the demoted coverage block. Always carries [`COVERAGE_LABEL`], even
/// when the number itself could not be computed.
pub fn coverage_proxy(
    graph: &Arc<GraphStore>,
    seed_iris: &[String],
    projected_ttl: &str,
) -> CoverageProxy {
    if seed_iris.is_empty() {
        return CoverageProxy {
            report: None,
            not_computed_because: Some(
                "no seed IRIs were supplied. An aggregate over zero seeds is trivially 1.0 or \
                 trivially 0.0 depending on how it is written, and neither says anything"
                    .into(),
            ),
            is_a_warrant: false,
            label: COVERAGE_LABEL,
            truncated_seeds: Vec::new(),
            known_limits: PROXY_LIMITS,
        };
    }
    match check_projection_loss(graph, seed_iris, projected_ttl) {
        Ok(r) => {
            // A seed at exactly the LIMIT is the one whose ratio was computed
            // against a truncated source.
            let truncated: Vec<String> = r
                .per_seed
                .iter()
                .filter(|s| s.source_triples >= 1000)
                .map(|s| s.seed_iri.clone())
                .collect();
            let missing = if r.projection_parses {
                None
            } else {
                Some("the slice did not parse as Turtle, so no ratio was computed".to_string())
            };
            CoverageProxy {
                report: if r.projection_parses { Some(r) } else { None },
                not_computed_because: missing,
                is_a_warrant: false,
                label: COVERAGE_LABEL,
                truncated_seeds: truncated,
                known_limits: PROXY_LIMITS,
            }
        }
        Err(e) => CoverageProxy {
            report: None,
            not_computed_because: Some(format!("the proxy could not be computed: {e}")),
            is_a_warrant: false,
            label: COVERAGE_LABEL,
            truncated_seeds: Vec::new(),
            known_limits: PROXY_LIMITS,
        },
    }
}

/// Terms any seed's outbound neighbourhood mentions, plus the seeds. Empty when
/// no seeds were given, in which case no goal is ever reported outside it.
fn seed_neighbourhood_terms(graph: &Arc<GraphStore>, seeds: &[String]) -> HashSet<String> {
    let mut terms = HashSet::new();
    for s in seeds {
        terms.insert(format!("<{s}>"));
        let q = format!("SELECT DISTINCT ?p ?o WHERE {{ <{s}> ?p ?o }} LIMIT 1000");
        let Ok(js) = graph.sparql_select(&q) else { continue };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&js) else { continue };
        if let Some(rows) = v["results"].as_array() {
            for row in rows {
                for k in ["p", "o"] {
                    if let Some(t) = row[k].as_str() {
                        terms.insert(t.to_string());
                    }
                }
            }
        }
    }
    terms
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loaded(ttl: &str) -> Arc<GraphStore> {
        let g = Arc::new(GraphStore::new());
        g.load_turtle(ttl, None).expect("load");
        g
    }

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir()
            .join(format!("oo-preserve-unit-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    const SRC: &str = r#"
        @prefix : <http://ex.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        :A rdfs:subClassOf :B . :B rdfs:subClassOf :C . :a a :A .
    "#;

    #[test]
    fn a_turtle_goal_round_trips_through_the_store_parser() {
        let (goals, refused) = parse_goals_turtle(
            "@prefix : <http://ex.org/> .\n:a a :C .\n",
        )
        .unwrap();
        assert!(refused.is_empty(), "{refused:?}");
        assert_eq!(goals.len(), 1);
        assert_eq!(
            goals[0].triple,
            (
                "<http://ex.org/a>".into(),
                "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>".into(),
                "<http://ex.org/C>".into()
            )
        );
    }

    #[test]
    fn a_blank_node_goal_is_refused_by_name() {
        let (goals, refused) =
            parse_goals_turtle("@prefix : <http://ex.org/> .\n:a :p [ :q :r ] .\n").unwrap();
        assert!(goals.is_empty(), "{goals:?}");
        assert!(refused.iter().any(|r| r.reason == GoalRefusal::BlankNode), "{refused:?}");
    }

    #[test]
    fn a_negative_claim_is_refused_and_points_at_shacl() {
        let (goals, refused) = parse_goals_tsv(
            "NOT <http://ex.org/s>\t<http://ex.org/p>\t<http://ex.org/o>\n",
            0,
        )
        .unwrap();
        assert!(goals.is_empty());
        assert_eq!(refused[0].reason, GoalRefusal::NotAPositiveGroundTriple);
        assert!(refused[0].means.contains("onto_shacl"), "{:?}", refused[0]);
    }

    #[test]
    fn a_derivations_tsv_pipes_in_with_one_skipped_column() {
        // The shape `reason --certificate` writes: the rule name, then the
        // conclusion, then the premises. Only the conclusion is a goal, so the
        // extra triples on the line must be ignored rather than read as three
        // more goals.
        let line = "rdfs9\t<http://ex.org/a>\t<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>\t                    <http://ex.org/C>\t<http://ex.org/a>\t                    <http://www.w3.org/1999/02/22-rdf-syntax-ns#type>\t<http://ex.org/A>\n";
        let (goals, refused) = parse_goals_tsv(line, 1).unwrap();
        assert!(refused.is_empty(), "{refused:?}");
        assert_eq!(goals.len(), 1, "one line is one goal, not one per triple on it");
        assert_eq!(goals[0].triple.2, "<http://ex.org/C>");
        // And the caller's own bytes survive beside the canonical spelling.
        assert!(goals[0].as_written.contains("http://ex.org/a"));
    }

    #[test]
    fn a_short_tsv_line_is_refused_rather_than_padded() {
        let (goals, refused) = parse_goals_tsv("<http://ex.org/a>\t<http://ex.org/p>\n", 0).unwrap();
        assert!(goals.is_empty());
        assert_eq!(refused[0].reason, GoalRefusal::Unparseable);
    }

    #[test]
    fn goals_come_out_of_a_bgp_and_its_bindings_with_no_generator_trusted() {
        // The strongest goal source: the query the answer was rendered from,
        // with the answer's own bindings put back. Nothing here trusts a
        // language model with anything.
        let bgp = vec![
            ("?x".to_string(), "<http://ex.org/p>".to_string(), "<http://ex.org/o>".to_string()),
            ("?y".to_string(), "<http://ex.org/q>".to_string(), "?z".to_string()),
        ];
        let mut b = BTreeMap::new();
        b.insert("x".to_string(), "<http://ex.org/a>".to_string());
        b.insert("y".to_string(), "<http://ex.org/b>".to_string());
        let (goals, refused) = goals_from_bindings(&bgp, &b).unwrap();
        assert_eq!(goals.len(), 1, "only the fully bound pattern is askable: {goals:?}");
        assert_eq!(goals[0].triple.0, "<http://ex.org/a>");
        assert_eq!(
            refused.len(),
            1,
            "an unbound variable is refused BY NAME rather than dropped: {refused:?}"
        );
        assert_eq!(refused[0].reason, GoalRefusal::Unparseable);
        assert!(refused[0].as_written.contains("?z"), "{:?}", refused[0]);
    }

    #[test]
    fn an_empty_goal_set_is_an_error_not_a_green_report() {
        let g = loaded(SRC);
        let err = check_entailment_preservation(
            &g,
            Projection::Turtle(SRC),
            &[],
            &[],
            &Opts { work_dir: scratch("empty"), ..Default::default() },
        )
        .expect_err("zero goals must not produce a report");
        assert!(err.to_string().contains("no goals to ask"), "{err}");
    }

    #[test]
    fn the_differential_can_fire() {
        // The engine cannot be made unsound on demand, so the gate is shown
        // able to fail on the PURE function with a hand-built pair.
        let t = |s: &str| (s.to_string(), "<p>".to_string(), "<o>".to_string());
        let src: HashSet<Spelled> = [t("<a>")].into_iter().collect();
        let prj: HashSet<Spelled> = [t("<a>"), t("<ghost>")].into_iter().collect();
        let r = differential(
            &src,
            &prj,
            Guards { subset_verified: true, blank_nodes_unmatched: false, both_reached_fixpoint: true },
        );
        assert_eq!(r.status, "armed");
        assert_eq!(r.violation_count, 1);
        let d = r.disagreement.expect("a violation must produce a disagreement block");
        assert_eq!(d.severity, STOP_THE_LINE);
        assert_eq!(d.what, "monotonicity_violated");
    }

    #[test]
    fn every_guard_disarms_the_differential_with_its_own_reason() {
        let t = |s: &str| (s.to_string(), "<p>".to_string(), "<o>".to_string());
        let src: HashSet<Spelled> = HashSet::new();
        let prj: HashSet<Spelled> = [t("<ghost>")].into_iter().collect();
        for (g, why) in [
            (
                Guards { subset_verified: false, blank_nodes_unmatched: false, both_reached_fixpoint: true },
                "projection_is_not_a_subset",
            ),
            (
                Guards { subset_verified: true, blank_nodes_unmatched: true, both_reached_fixpoint: true },
                "blank_nodes_unmatched",
            ),
            (
                Guards { subset_verified: true, blank_nodes_unmatched: false, both_reached_fixpoint: false },
                "fixpoint_not_reached",
            ),
        ] {
            let r = differential(&src, &prj, g);
            assert_eq!(r.status, "disarmed", "{why}");
            assert_eq!(r.reason, why);
            assert!(r.disagreement.is_none(), "a disarmed gate must not accuse the engine");
        }
    }

    #[test]
    fn skolemising_survives_a_turtle_round_trip_and_canonicalising_would_not() {
        let g = loaded(
            r#"@prefix : <http://ex.org/> .
               @prefix owl: <http://www.w3.org/2002/07/owl#> .
               @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
               :C rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :p ; owl:someValuesFrom :D ] .
               :E rdfs:subClassOf :C ."#,
        );
        let (sk, map) = skolemise(&g).unwrap();
        assert!(!map.is_empty(), "the source had blank nodes to skolemise");
        let ttl = sk.serialize("turtle").unwrap();
        let reparsed = GraphStore::new();
        reparsed.load_turtle(&ttl, None).unwrap();
        let a: BTreeSet<Spelled> = sk.all_triples().unwrap().into_iter().collect();
        let b: BTreeSet<Spelled> = reparsed.all_triples().unwrap().into_iter().collect();
        assert_eq!(a, b, "a skolemised graph must survive a Turtle round trip unchanged");
    }
}
