//! Does each step of a reasoning trace follow, and if not, which kind of not?
//!
//! `src/dlp.rs` asks which of an ontology's axioms the rule table can see.
//! This asks the same question one step at a time about a trace that CLAIMS
//! inference steps, and it returns the answer `dlp.rs` taught: a certificate is
//! a sound proof about the axioms the rules read and says nothing at all about
//! the ones no rule fires on. So a step that does not check has two very
//! different failures behind it and they are fixed in two different places.
//!
//! # The three families, and why the third is the point
//!
//! * `entailed`. The store entails the conclusion. Under which warrant is a
//!   separate field, because `OOCert.certificate_sound` and
//!   `OOCert.horn_certificate_sound` are different sentences, and the second
//!   gets its own family word rather than being folded into the first.
//! * `not_entailed`. Some rule in the table has a head that could conclude a
//!   triple of this shape, every axiom the step cites is inside the fragment,
//!   and this graph still does not support it. The bogus step.
//! * `outside_the_fragment`. Nothing this table does could ever produce this
//!   conclusion. Two separate reasons, kept apart exactly as `src/dlp.rs` keeps
//!   its two dimensions apart, because merging them destroys the only
//!   information a reader can act on: `outside_the_fragment` is a rewrite of
//!   the ontology and `no_rule_in_this_table_concludes_this_shape` is a patch
//!   to `src/reason.rs`.
//!
//! Collapsing the second and the third is how a labeller becomes useless. A
//! step that fails because the ontology says nothing and a step that fails
//! because the rule table is blind to the axiom that would license it are
//! opposite findings, and a dataset that labels both `unsound` teaches a model
//! that the fragment boundary is a reasoning error.
//!
//! The fragment question is asked FIRST, before the empty-head-list one. The
//! fragment is a fact about the LANGUAGE and would hold of a perfect OWL 2 RL
//! engine; an empty head list is a fact about the table this build ships. A
//! reader told to patch `src/reason.rs` for an axiom no Horn rule could ever
//! express has been sent to the wrong place.
//!
//! # Two answers per step, and neither is reported as the other
//!
//! A step can be LOCALLY sound, meaning the premises it cited entail the
//! conclusion it claimed, while the graph does not hold those premises at all.
//! That is a step reasoning from premises it invented. Both answers are
//! carried:
//!
//! * `local` is the one-line certificate over the step's OWN cited premises.
//!   `OOCert.certificate_sound` there reads: the conclusion is true in every
//!   model of the triples this step cited.
//! * `label` and `family` are about the STORE, decided over the run
//!   certificate's own files.
//! * `premises_in_store` and `premises_not_in_store` say whether the graph
//!   holds what the step read.
//!
//! The store question decides the headline, so family `entailed` can never be
//! claimed on the strength of premises the graph does not hold. The
//! invented-premise step gets its own label,
//! `locally_sound_but_premises_not_in_the_store`, inside the `not_entailed`
//! family, because that is the actionable finding and `not_entailed` alone
//! would throw away the fact that the step's own reasoning was valid.
//!
//! # The constraint this module inherits and cannot escape
//!
//! `docs/decisions/0003-a-rule-is-data-and-an-assumption-is-not-a-fact.md` and
//! `lean/HMain.lean` say the same thing: a certificate over a USER-SUPPLIED
//! rule table is reported as `entailed_under_supplied_rules`, never as
//! `entailed`, with a digest of the table that was in force. A labeller that
//! printed the absolute word for a step licensed by a rule the user wrote would
//! be assurance laundering with a per-step interface, which is worse than the
//! whole-run version because it produces training labels.
//!
//! So the word is not chosen here. It is a function of the theorem name the
//! CHECKER printed into its own stdout, read back through
//! [`crate::verdict::Certified::theorem`], and [`warrant_of_theorem`] is the
//! whole of that function. A step checked by `oo-horn` over a table that is not
//! the built-in one can reach `entailed_under_supplied_rules_checked` and
//! nothing else, and every report over a supplied table carries
//! `rules_tsv_sha256` and `conditional_on`.
//!
//! The one case where a supplied table earns the absolute word is the case
//! `oo-horn` itself announces: a table byte-identical to the built-in one makes
//! the checker print `OOCert.entails_of_builtin_horn`, which
//! `tests/reason_horn_emit_test.rs` already pins. Reading the word off the
//! checker is what makes that correct rather than a hole.
//!
//! # What a label does NOT say
//!
//! `not_entailed` is bounded by the rule table. The engine evaluates 29 of OWL
//! 2 RL's 78 rules (`reason::RULES_EVALUATED`, counted by
//! `tests/dlp_boundary_test.rs`), and RDF entailment is open-world, so
//! `not_entailed` means "not derivable under this table" and never "false".

use crate::graph::GraphStore;
use crate::projection_entailment::{
    CertKind, CertificateIndex, CheckerStatus, Membership, Spelled, run_checker,
};
use crate::reason::{
    AtomPat, BUILTIN_RULES, CHAINED_RULES, InferenceTarget, Kw, Pat, Reasoner, RulePattern, Slot,
};
use crate::verdict::Certified;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// `rdf:rest`, in the N-Triples spelling the store uses. The marker that a
/// step's premises carry the RDF list a chained rule reads: a one-element list
/// is `rdf:first` plus `rdf:rest rdf:nil`, so every list of every length has
/// one.
const RDF_REST: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#rest>";

// ─────────────────────────────────────────────────────────────────────────────
// The trace format
// ─────────────────────────────────────────────────────────────────────────────

/// One claimed inference step.
///
/// The format is `oo-trace/1` and it is one line per step:
///
/// ```text
/// rule TAB cs TAB cp TAB co TAB (ps TAB pp TAB po)*
/// ```
///
/// That is byte-identical to a line of `derivations.tsv` as `src/reason.rs`
/// writes it, and the reuse is the whole justification for the format. Three
/// things follow from it that a format invented here would not have given. The
/// output of `reason --certificate` is a valid trace, so the labeller has a
/// self-check with no fixture: every step of a trace the engine itself emitted
/// must land in an entailed family. The Lean parser `OOCert.Parse.parseSteps`
/// already reads it, so the one-step certificate this module writes needs no
/// new grammar in `lean/`. And `crate::boundary_core::push_triple_fields_bytes`
/// is already the one function that writes such a field, so nothing here
/// escapes a separator.
///
/// The rule field MAY be empty, which means no rule was claimed. An empty field
/// is unambiguous: `reason::parse_rules` refuses an empty rule name and no row
/// of `BUILTIN_RULES` has one.
///
/// Premises are triples and an IRI alone is REFUSED. A premise of a rule in
/// this engine is a triple, and accepting a bare IRI would mean guessing which
/// triple the trace meant. The refusal names the line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TraceStep {
    /// Empty when the trace claimed no rule.
    pub rule_claimed: String,
    pub conclusion: Spelled,
    pub premises: Vec<Spelled>,
}

/// Read `oo-trace/1`. Malformed input is an error naming the line, never a
/// guess, for the reason `reason::parse_rules` gives: a step half-read still
/// produces a label, and a label nobody wrote is worse than no label.
pub fn parse_trace(text: &str) -> anyhow::Result<Vec<TraceStep>> {
    let mut out = Vec::new();
    for (i, line) in text.split('\n').enumerate() {
        let n = i + 1;
        if line.is_empty() {
            continue;
        }
        if line.contains('\r') {
            anyhow::bail!(
                "trace line {n} contains a carriage return. The format is tab separated with LF \
                 endings, and a CR would become part of a term; convert the file rather than have \
                 it stripped silently"
            );
        }
        let f: Vec<&str> = line.split('\t').collect();
        // One rule field then 3(1+k) term fields, k >= 0. A conclusion with no
        // premise is legal: a trace may claim a step that reads nothing, and
        // saying so is more useful than refusing to read the line.
        if f.len() < 4 || !(f.len() - 1).is_multiple_of(3) {
            anyhow::bail!(
                "trace line {n} has {} tab-separated field(s). A step is one rule field then a \
                 conclusion and each premise as three fields, so the count after the rule must be \
                 a positive multiple of three. A premise given as a bare IRI is refused rather \
                 than guessed at: a premise of a rule in this engine is a triple",
                f.len()
            );
        }
        let terms: Vec<Spelled> = f[1..]
            .chunks(3)
            .map(|c| (c[0].to_string(), c[1].to_string(), c[2].to_string()))
            .collect();
        for t in &terms {
            for term in [&t.0, &t.1, &t.2] {
                if !crate::boundary_core::term_fits_the_format(term.as_bytes()) {
                    anyhow::bail!(
                        "trace line {n}: {term:?} is not a term in the certificate format. A term \
                         must be non-empty and carry no tab, newline or carriage return, and must \
                         be N-Triples: <iri>, _:blank, or a quoted literal. The store spells \
                         every term that way, so anything else could not match a triple it holds"
                    );
                }
            }
        }
        out.push(TraceStep {
            rule_claimed: f[0].to_string(),
            conclusion: terms[0].clone(),
            premises: terms[1..].to_vec(),
        });
    }
    if out.is_empty() {
        anyhow::bail!("the trace holds no steps, so there is nothing to label");
    }
    Ok(out)
}

// ─────────────────────────────────────────────────────────────────────────────
// The warrant, which is read off the checker and never chosen here
// ─────────────────────────────────────────────────────────────────────────────

/// Which of the two sentences a theorem name stands for.
///
/// Split out as a pure function over `&str` for one reason and it is a testing
/// reason: `Certified` has no public constructor, so a test cannot build one to
/// exercise the mapping. It can exercise this, and this is the only place the
/// word is decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Warrant {
    /// True in every model of the premises. `OOCert.certificate_sound`, and
    /// `OOCert.entails_of_builtin_horn`, which `lean/HMain.lean` prints when
    /// the table it was handed is byte-identical to the built-in one.
    Absolute,
    /// True in every model of the premises THAT ALSO SATISFIES the supplied
    /// rules. `OOCert.horn_certificate_sound`. Never shortened.
    UnderSuppliedRules,
}

/// `None` for any other name, and that is the refusal that matters: a renamed
/// or weakened theorem must not keep the label this module was written against.
/// `CheckerRun::accepted_naming` already refuses a theorem outside the list it
/// is given, so this is the second gate and not the only one.
pub fn warrant_of_theorem(theorem: &str) -> Option<Warrant> {
    match theorem {
        "OOCert.certificate_sound" | "OOCert.entails_of_builtin_horn" => Some(Warrant::Absolute),
        "OOCert.horn_certificate_sound" => Some(Warrant::UnderSuppliedRules),
        _ => None,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The labels
// ─────────────────────────────────────────────────────────────────────────────

/// The label of one step, about the STORE.
///
/// The certified variants carry a [`Certified`], which has no public
/// constructor, so none of them can be named on a path that did not run a Lean
/// checker and read a zero exit code.
///
/// `Deserialize` is deliberately not implemented, exactly as for
/// `projection_entailment::GoalVerdict`: reading `"entailed_checked"` out of
/// somebody else's JSON is not the same act as earning it.
///
/// ```compile_fail
/// use open_ontologies::trace_label::StepLabel;
/// use open_ontologies::verdict::Certified;
/// let l = StepLabel::EntailedChecked(Certified {
///     theorem: "OOCert.certificate_sound",
///     subject: [0u8; 32],
/// });
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepLabel {
    /// A Lean checker accepted a certificate slice of the RUN that concludes
    /// this triple, and the theorem it named is the absolute one.
    EntailedChecked(Certified),
    /// The same acceptance, and the theorem the checker named is the
    /// relativised one, because the run evaluated a SUPPLIED table. True in
    /// every model of the store that also satisfies those rules. Never
    /// shortened.
    EntailedUnderSuppliedRulesChecked(Certified),
    /// The conclusion is literally in the store. A lookup by exact-string
    /// membership in `asserted.tsv`, not a theorem, and it does not pretend to
    /// be one. Absolute whatever table was in force, because no rule was
    /// involved.
    AssertedNotDerived,
    /// The ENGINE derived it under the built-in table and no checker looked,
    /// because none was built.
    EntailedUnchecked,
    /// The ENGINE derived it under a SUPPLIED table and no checker looked. A
    /// separate variant from the one above and not a flag on it, so the family
    /// word cannot lose the qualification on a path where nothing was checked.
    EntailedUnderSuppliedRulesUnchecked,
    /// The store does not hold the conclusion, and a checker accepted the
    /// step's own premises as entailing it. The step's reasoning was valid and
    /// its premises were invented. Both halves are in the report:
    /// `local` carries the acceptance and `premises_not_in_store` names the
    /// triples the graph does not hold.
    LocallySoundButPremisesNotInTheStore(Certified),
    /// The closure does not hold it, some rule head could conclude a triple of
    /// this shape, and every axiom the step cites is inside the fragment.
    NotEntailedUnderThisTable,
    /// An axiom the step cites is one `src/dlp.rs` classifies `outside` or
    /// `partially_inside`. Dimension one: a fact about the LANGUAGE that would
    /// hold of a perfect OWL 2 RL engine, and only a rewrite of the ontology
    /// helps.
    OutsideTheFragment,
    /// No head in the table can unify with the conclusion, so no data could
    /// ever make this table derive it. Dimension two: a fact about THIS
    /// ENGINE's table, and a different table would see it.
    NoRuleInThisTableConcludesThisShape,
    /// The checker REJECTED a certificate slice of the run. A defect in the
    /// emitter or in this module, never a downgrade to an unchecked label.
    CertificateRejected,
}

impl StepLabel {
    pub fn word(&self) -> &'static str {
        match self {
            StepLabel::EntailedChecked(_) => "entailed_checked",
            StepLabel::EntailedUnderSuppliedRulesChecked(_) => {
                "entailed_under_supplied_rules_checked"
            }
            StepLabel::AssertedNotDerived => "asserted_not_derived",
            StepLabel::EntailedUnchecked => "entailed_unchecked",
            StepLabel::EntailedUnderSuppliedRulesUnchecked => {
                "entailed_under_supplied_rules_unchecked"
            }
            StepLabel::LocallySoundButPremisesNotInTheStore(_) => {
                "locally_sound_but_premises_not_in_the_store"
            }
            StepLabel::NotEntailedUnderThisTable => "not_entailed_under_this_table",
            StepLabel::OutsideTheFragment => "outside_the_fragment",
            StepLabel::NoRuleInThisTableConcludesThisShape => {
                "no_rule_in_this_table_concludes_this_shape"
            }
            StepLabel::CertificateRejected => "certificate_rejected",
        }
    }

    /// The headline answer, for a consumer that wants the grouping. The words
    /// come from the labels and the labels are never reconstructed from these.
    ///
    /// Three families, and the first has two spellings because the hard rule
    /// forbids one. A step whose warrant came from a SUPPLIED table reads
    /// `entailed_under_supplied_rules` here, never the bare word, so a consumer
    /// that filters on `family == "entailed"` cannot collect a conclusion that
    /// holds only in models satisfying rules nobody checked. Use
    /// [`is_entailed_family`](Self::is_entailed_family) for the three-way
    /// grouping rather than comparing strings.
    pub fn family(&self) -> &'static str {
        match self {
            StepLabel::EntailedChecked(_)
            | StepLabel::AssertedNotDerived
            | StepLabel::EntailedUnchecked => "entailed",
            StepLabel::EntailedUnderSuppliedRulesChecked(_)
            | StepLabel::EntailedUnderSuppliedRulesUnchecked => "entailed_under_supplied_rules",
            StepLabel::NotEntailedUnderThisTable
            | StepLabel::LocallySoundButPremisesNotInTheStore(_)
            | StepLabel::CertificateRejected => "not_entailed",
            StepLabel::OutsideTheFragment | StepLabel::NoRuleInThisTableConcludesThisShape => {
                "outside_the_fragment"
            }
        }
    }

    /// Whether this is the first family, under either of its two spellings.
    pub fn is_entailed_family(&self) -> bool {
        matches!(self.family(), "entailed" | "entailed_under_supplied_rules")
    }

    /// The theorem standing behind THIS label, which is a statement about the
    /// STORE. Read out of the evidence, so a label that ran no checker cannot
    /// name one.
    ///
    /// `LocallySoundButPremisesNotInTheStore` names none, even though it carries
    /// a `Certified`. The token it carries was earned over the step's own cited
    /// premises and says nothing about the store, so putting it in the field
    /// beside a store-level label would invite exactly the reading this label
    /// exists to prevent. It is reported under `local` instead, where the
    /// sentence it discharges is written down. The payload stays on the variant
    /// because it is what makes the word `locally_sound` unconstructible without
    /// a checker acceptance.
    pub fn theorem(&self) -> Option<&'static str> {
        match self {
            StepLabel::EntailedChecked(c) | StepLabel::EntailedUnderSuppliedRulesChecked(c) => {
                Some(c.theorem())
            }
            _ => None,
        }
    }

    /// The digest of the files the accepting run was handed, so a reader can
    /// ask whether the token is about the artefact in their hand. Absent
    /// wherever [`theorem`](Self::theorem) is, and for the same reason.
    pub fn subject_sha256(&self) -> Option<String> {
        match self {
            StepLabel::EntailedChecked(c) | StepLabel::EntailedUnderSuppliedRulesChecked(c) => {
                Some(c.subject_sha256())
            }
            _ => None,
        }
    }
}

impl Serialize for StepLabel {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.word())
    }
}

impl PartialEq<&str> for StepLabel {
    fn eq(&self, other: &&str) -> bool {
        self.word() == *other
    }
}

/// The answer to the LOCAL question: do the premises this step cited entail the
/// conclusion it claimed, in one application of the rule it named?
///
/// A different sentence from the label above, and never reported as it. The
/// certified variants carry a [`Certified`] for the same reason [`StepLabel`]'s
/// do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalVerdict {
    /// A checker accepted a one-line certificate whose asserted graph is this
    /// step's own cited premises. `OOCert.certificate_sound` there reads: the
    /// conclusion is true in every model of the triples this step cited.
    SoundChecked(Certified),
    /// The same over a SUPPLIED table: true in every model of the cited
    /// premises that also satisfies those rules.
    SoundUnderSuppliedRulesChecked(Certified),
    /// The checker rejected the one-line certificate. The cited rule, with the
    /// cited premises, does not produce the claimed conclusion.
    NotLicensedByTheCitedRule,
    /// Not asked. `rule_claim` says why: no rule was claimed, the name is in no
    /// table, a chained rule was cited without its list, or no binding
    /// instantiates the cited rule's body.
    NotAsked,
    /// The Lean checker was not built, so nobody looked. Never a pass.
    CheckerAbsent,
}

impl LocalVerdict {
    pub fn word(&self) -> &'static str {
        match self {
            LocalVerdict::SoundChecked(_) => "locally_sound_checked",
            LocalVerdict::SoundUnderSuppliedRulesChecked(_) => {
                "locally_sound_under_supplied_rules_checked"
            }
            LocalVerdict::NotLicensedByTheCitedRule => "not_licensed_by_the_cited_rule",
            LocalVerdict::NotAsked => "not_asked",
            LocalVerdict::CheckerAbsent => "checker_absent",
        }
    }
    pub fn theorem(&self) -> Option<&'static str> {
        match self {
            LocalVerdict::SoundChecked(c) | LocalVerdict::SoundUnderSuppliedRulesChecked(c) => {
                Some(c.theorem())
            }
            _ => None,
        }
    }
    pub fn subject_sha256(&self) -> Option<String> {
        match self {
            LocalVerdict::SoundChecked(c) | LocalVerdict::SoundUnderSuppliedRulesChecked(c) => {
                Some(c.subject_sha256())
            }
            _ => None,
        }
    }
    /// The token, where one was earned. Used to carry the acceptance into
    /// [`StepLabel::LocallySoundButPremisesNotInTheStore`] so that label names
    /// the theorem the checker actually printed.
    fn certified(&self) -> Option<Certified> {
        match self {
            LocalVerdict::SoundChecked(c) | LocalVerdict::SoundUnderSuppliedRulesChecked(c) => {
                Some(c.clone())
            }
            _ => None,
        }
    }
    pub fn is_sound(&self) -> bool {
        self.certified().is_some()
    }
}

impl Serialize for LocalVerdict {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.word())
    }
}

/// Whether the RULE the trace cited is the rule that licenses the step.
///
/// Orthogonal to [`StepLabel`] and reported beside it, never folded into it. A
/// step can have a true conclusion and a wrong citation, and for a dataset that
/// is the interesting row: it labels the JUSTIFICATION as well as the
/// conclusion. Folding a bad citation into `not_entailed` would be a false
/// statement about the ontology.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleClaim {
    /// The cited rule, with the cited premises, is what the checker accepted.
    Licensed,
    /// The trace claimed no rule. The step was labelled on its conclusion
    /// alone, by the store-level question.
    NoRuleClaimed,
    /// The name is in the table, and no instance of THAT rule with THOSE
    /// premises produced this conclusion. Whether the conclusion follows some
    /// other way is the label's business, not this field's.
    RuleInTableButDidNotFireHere,
    /// The name is in neither `reason::BUILTIN_RULES`, `reason::CHAINED_RULES`
    /// nor the supplied table.
    ///
    /// This is NOT a label. `OOCert.Parse.parseSteps` fails on an unknown rule
    /// name and `lean/Main.lean` exits 2 for that, which is "a file could not
    /// be parsed" and not "the step was rejected", so such a step is never
    /// handed to the checker at all: it is labelled on its conclusion by the
    /// store-level question, and the bad citation is reported here.
    RuleNotInTable,
    /// A chained rule (`cls-int1`, `cls-int2`, `cls-uni`, `cls-oo`) whose cited
    /// premises carry no `rdf:rest`, so the RDF list the rule reads is not in
    /// them. Those rules read their list off the ASSERTED graph, so a one-step
    /// certificate over incomplete premises would be rejected for a reason
    /// that is about the citation and not about the inference. Reported here
    /// rather than as a rejection.
    ChainedRulePremisesIncomplete,
    /// A supplied rule of that name exists and no binding instantiates its body
    /// with the cited premises, so there is no `horn.tsv` line to write.
    NoBindingInstantiatesTheCitedRule,
}

impl RuleClaim {
    pub fn word(self) -> &'static str {
        match self {
            RuleClaim::Licensed => "licensed",
            RuleClaim::NoRuleClaimed => "no_rule_claimed",
            RuleClaim::RuleInTableButDidNotFireHere => "rule_in_table_but_did_not_fire_here",
            RuleClaim::RuleNotInTable => "rule_not_in_table",
            RuleClaim::ChainedRulePremisesIncomplete => "chained_rule_premises_incomplete",
            RuleClaim::NoBindingInstantiatesTheCitedRule => {
                "no_binding_instantiates_the_cited_rule"
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Head-shape reachability: dimension two, decided from the table alone
// ─────────────────────────────────────────────────────────────────────────────

/// Every rule in the table whose HEAD can unify with this triple.
///
/// Empty means no data whatsoever could make this table derive a triple of this
/// shape, which is a stronger and more useful statement than "this graph does
/// not support it". It is decided from the table and never from the store.
///
/// # What this measured about the built-in table, and it is not what was expected
///
/// For the BUILT-IN table the list is never empty, and that is a fact about the
/// table rather than a weakness of this function. `rdfs7`'s head is
/// `[Var, Var, Var]`: from `s p o` and `p rdfs:subPropertyOf q` it concludes
/// `s q o`, so for ANY triple there is data that makes `rdfs7` conclude it.
/// `prp-trp`, `prp-symp`, `prp-inv1`, `prp-inv2` and `cls-hv1` have
/// all-variable heads too. So dimension two is vacuous under the built-in
/// table: every `not_derivable` conclusion there is either
/// `outside_the_fragment` or `not_entailed_under_this_table`, and
/// `no_rule_in_this_table_concludes_this_shape` is reachable only for a
/// SUPPLIED table whose heads fix a predicate.
///
/// `the_builtin_head_list_is_never_empty_because_rdfs7_concludes_anything`
/// pins that, and it is pinned rather than commented because the design note
/// this module was written from claimed the opposite: that a conclusion of
/// `owl:hasKey` would come back with an empty list. It does not, and a test
/// asserting it did would have been a gate that could not fail.
///
/// Variable repetition is honoured rather than assumed away. No head in
/// `BUILTIN_RULES` repeats a slot today, so a position-wise test would agree,
/// but a user can write `?x ?p ?x` as a head and a position-wise test would
/// accept a conclusion whose subject and object differ.
pub fn rules_whose_head_can_conclude(t: &Spelled, supplied: &[RulePattern]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if supplied.is_empty() {
        for r in BUILTIN_RULES {
            if head_slots_match(&r.head, t) && !out.iter().any(|n| n == r.name) {
                out.push(r.name.to_string());
            }
        }
        // The four list rules are not rows of BUILTIN_RULES because their
        // premise count is the length of an RDF list, which is data rather than
        // something the rule fixes. Every one of their heads concludes
        // an `rdf:type` triple. Read off the rule table in
        // `lean/OOCert/Rules.lean`, which `OOCert.checkStep` implements:
        // cls-int1 concludes `x rdf:type c`, cls-int2 `x rdf:type m`, cls-uni
        // `x rdf:type c`, cls-oo `m rdf:type c`. Four heads, one predicate.
        // `the_four_chained_rules_all_conclude_a_type_triple` pins this against
        // CHAINED_RULES so a fifth chained rule cannot be added without either
        // matching that shape or failing the suite.
        if t.1 == Kw::Type.iri() {
            out.extend(CHAINED_RULES.iter().map(|r| r.to_string()));
        }
    } else {
        for r in supplied {
            if pattern_head_matches(&r.head, t) && !out.iter().any(|n| n == &r.name) {
                out.push(r.name.clone());
            }
        }
    }
    out
}

fn head_slots_match(head: &[Slot; 3], t: &Spelled) -> bool {
    let terms = [&t.0, &t.1, &t.2];
    let mut bound: BTreeMap<usize, &str> = BTreeMap::new();
    for (slot, term) in head.iter().zip(terms) {
        match slot {
            Slot::Fixed(kw) => {
                if kw.iri() != term.as_str() {
                    return false;
                }
            }
            Slot::Var(i) => match bound.insert(*i, term) {
                Some(prev) if prev != term.as_str() => return false,
                _ => {}
            },
        }
    }
    true
}

fn pattern_head_matches(head: &AtomPat, t: &Spelled) -> bool {
    let mut bound: BTreeMap<&str, &str> = BTreeMap::new();
    for (pat, term) in [(&head.s, &t.0), (&head.p, &t.1), (&head.o, &t.2)] {
        match pat {
            Pat::Const(c) => {
                if c != term {
                    return false;
                }
            }
            Pat::Var(v) => match bound.insert(v.as_str(), term.as_str()) {
                Some(prev) if prev != term.as_str() => return false,
                _ => {}
            },
        }
    }
    true
}

// ─────────────────────────────────────────────────────────────────────────────
// The procedure
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Opts {
    /// The profile the store-level question is decided under. `owl-dl` is
    /// refused by `Reasoner::run_scoped` because the tableaux path records no
    /// rule applications, so there is nothing to label a step against.
    pub profile: String,
    /// A SUPPLIED Horn table. Its presence changes the label word for every
    /// checked step and puts `rules_tsv_sha256` in the report.
    pub rules: Option<PathBuf>,
    /// Where the run certificate, the per-step certificates and `labels.jsonl`
    /// land. Every intermediate file stays, so a label is reproducible by hand.
    pub work_dir: PathBuf,
    pub checker: Option<PathBuf>,
    /// Turn an absent checker into an error instead of an honest
    /// `entailed_unchecked`. The CI leg sets this.
    pub require_checker: bool,
}

#[derive(Serialize)]
pub struct LabelledStep {
    pub index: usize,
    pub rule_claimed: String,
    pub conclusion: Spelled,
    pub premises: Vec<Spelled>,
    /// The STORE-level answer.
    pub label: StepLabel,
    pub family: &'static str,
    pub theorem: Option<&'static str>,
    pub subject_sha256: Option<String>,
    /// Where the conclusion stands in the run certificate: `asserted`,
    /// `derived` or `not_derivable`.
    pub conclusion_in_store: &'static str,
    /// The LOCAL answer, over the step's own cited premises. Never folded into
    /// the label: a step can be locally sound and still conclude something the
    /// store does not entail, because it read premises the store does not hold.
    pub local: LocalAnswer,
    /// Does the graph hold every triple this step cited? False is the
    /// invented-premise finding, and it is why `local` and `label` are two
    /// fields rather than one.
    pub premises_in_store: bool,
    /// Which cited premises the run certificate holds in neither `asserted.tsv`
    /// nor its conclusions. Empty when `premises_in_store` is true.
    pub premises_not_in_store: Vec<Spelled>,
    pub rule_claim: RuleClaim,
    /// The rules in the table whose head could conclude a triple of this shape.
    /// Empty is what `no_rule_in_this_table_concludes_this_shape` rests on, and
    /// under the built-in table it is never empty; see
    /// [`rules_whose_head_can_conclude`].
    pub heads_that_could_conclude_it: Vec<String>,
    /// Present only when a premise is an axiom `src/dlp.rs` puts outside the
    /// fragment, and then it carries that classifier's own `outside_because`
    /// rows rather than a summary of them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fragment: Option<serde_json::Value>,
    /// The command that reproduces the local certificate. A third party with
    /// the Lean binary runs it and reads the same exit code. Absent when no
    /// local certificate was written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reproduce: Option<String>,
}

/// The local answer and its evidence, as one object so a consumer cannot read
/// the word without the theorem beside it.
#[derive(Serialize)]
pub struct LocalAnswer {
    pub verdict: LocalVerdict,
    pub theorem: Option<&'static str>,
    pub subject_sha256: Option<String>,
    pub means: &'static str,
}

impl LocalAnswer {
    fn of(v: LocalVerdict) -> LocalAnswer {
        LocalAnswer {
            theorem: v.theorem(),
            subject_sha256: v.subject_sha256(),
            means: match &v {
                LocalVerdict::SoundChecked(_) =>
                    "the conclusion is true in every model of the triples THIS STEP CITED. It \
                     says nothing about whether the store holds those triples; read \
                     premises_in_store for that",
                LocalVerdict::SoundUnderSuppliedRulesChecked(_) =>
                    "true in every model of the CITED PREMISES that also satisfies the supplied \
                     rule table. The rules are assumed and never checked. Never shorten this to \
                     the plain word",
                LocalVerdict::NotLicensedByTheCitedRule =>
                    "the checker rejected one application of the cited rule over the cited \
                     premises. Whether the conclusion follows some other way is the label's \
                     business",
                LocalVerdict::NotAsked =>
                    "not asked. rule_claim says why: no rule claimed, a name in no table, a \
                     chained rule cited without its list, or no binding instantiating its body",
                LocalVerdict::CheckerAbsent =>
                    "the Lean checker was not built, so nobody looked. This is not a pass",
            },
            verdict: v,
        }
    }
}

#[derive(Serialize)]
pub struct TraceLabelReport {
    pub format: &'static str,
    pub profile: String,
    pub steps: Vec<LabelledStep>,
    pub by_label: BTreeMap<&'static str, usize>,
    pub by_family: BTreeMap<&'static str, usize>,
    pub by_rule_claim: BTreeMap<&'static str, usize>,
    pub by_local_verdict: BTreeMap<&'static str, usize>,
    /// The digest of the supplied table as the run evaluated it, and the
    /// sentence that travels with it. Both absent when no table was supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules_tsv_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conditional_on: Option<String>,
    pub certificate_dir: String,
    pub dataset: String,
    pub families_mean: &'static str,
    pub bounded_by: &'static str,
    pub exit_code: i32,
}

/// Label every step of a trace.
///
/// # What a step costs, measured rather than assumed
///
/// Up to two checker processes per step: one for the local question and one for
/// the store-level question when the conclusion is derived. Measured on this
/// machine (macOS 25.6, Lean 4.33.1) over a two-premise one-derivation
/// certificate, 50 consecutive runs of `oo-cert`:
///
/// ```text
/// 2 premises,   1 derivation:  4.274 s wall / 50 runs = 85.5 ms per spawn
///                             0.09  s CPU  / 50 runs =  1.8 ms per spawn
/// 200 premises, 100 derivations: 1.832 s wall / 20 runs = 91.6 ms per spawn
/// ```
///
/// So 97.9 per cent of the wall time of a small check is not CPU spent in the
/// checker at all, and a certificate a hundred times bigger costs 7 per cent
/// more. The cost is per SPAWN and not per step. A 500-step trace where every
/// step is derived spawns up to 1000 processes and would take about 85 seconds
/// here.
///
/// Batching is deliberately NOT built, and the measurement above is only half
/// the reason. The other half is that the obvious batching is UNSOUND. Each
/// per-step certificate's `asserted.tsv` is that step's own cited premises, and
/// that is the whole point of the local question; concatenating a hundred of
/// them into one certificate would let step 40 borrow step 7's premises and
/// every one of them would check green. Doing it properly means a checker that
/// takes a list of (asserted, derivations) pairs, which is a change in `lean/`
/// and a new theorem, not a loop in Rust. So the number above is what a future
/// decision should be made against, `projection_entailment` already spawns per
/// goal against the same binaries and nobody has complained, and the trace
/// lengths this has been run on are 1 to 3 steps.
pub fn label_trace(
    graph: &Arc<GraphStore>,
    trace: &[TraceStep],
    opts: &Opts,
) -> anyhow::Result<TraceLabelReport> {
    std::fs::create_dir_all(&opts.work_dir)?;
    let run_dir = opts.work_dir.join("run");

    let supplied: Vec<RulePattern> = match &opts.rules {
        Some(p) => {
            let text = std::fs::read_to_string(p)
                .map_err(|e| anyhow::anyhow!("cannot read rule table {}: {e}", p.display()))?;
            crate::reason::parse_rules(&text)?
        }
        None => Vec::new(),
    };
    let kind = if supplied.is_empty() { CertKind::OoCert } else { CertKind::OoHorn };

    // ── The store-level question, decided over the certificate FILES ─────────
    //
    // Read back off disk rather than from an in-memory closure, for the reason
    // `src/projection_entailment.rs` gives at its head: the Lean checker only
    // ever sees those files, so deciding membership against a set the checker
    // never reads would let this module report an entailed family over a
    // certificate that does not contain the derivation.
    if supplied.is_empty() {
        Reasoner::run_full(
            graph,
            &opts.profile,
            false,
            InferenceTarget::DefaultGraph,
            Some(&run_dir),
        )?;
    } else {
        let p = opts.rules.as_ref().expect("a non-empty supplied table came from a path");
        Reasoner::run_horn(graph, p, &run_dir)?;
    }
    let index = CertificateIndex::read(&run_dir)?;

    // ── Dimension one, classified ONCE over the whole store ──────────────────
    //
    // `DlpBoundary::standing_of` re-reads and re-indexes the store on every
    // call, which is O(store). Called per step it would dominate the runtime on
    // a large ontology, so every premise of every step goes in one call and the
    // answers are sliced back out by offset.
    let mut flat: Vec<Spelled> = Vec::new();
    let mut spans: Vec<(usize, usize)> = Vec::with_capacity(trace.len());
    for st in trace {
        let start = flat.len();
        flat.extend(st.premises.iter().cloned());
        spans.push((start, flat.len()));
    }
    let classified = crate::dlp::DlpBoundary::standing_of(graph, &flat)?;

    let mut steps: Vec<LabelledStep> = Vec::with_capacity(trace.len());
    for (i, st) in trace.iter().enumerate() {
        let heads = rules_whose_head_can_conclude(&st.conclusion, &supplied);
        let (lo, hi) = spans[i];
        let fragment = fragment_finding(&classified[lo..hi]);
        let step_dir = opts.work_dir.join("steps").join(format!("{i:04}"));

        // Which cited premises the store does not hold. Decided over the run
        // certificate's closure, so a premise the engine DERIVED counts as
        // held: a step citing an inference is not citing an invention.
        let premises_not_in_store: Vec<Spelled> = st
            .premises
            .iter()
            .filter(|p| index.membership(p) == Membership::NotDerivable)
            .cloned()
            .collect();
        let premises_in_store = premises_not_in_store.is_empty();

        // ── 1. THE LOCAL QUESTION ───────────────────────────────────────────
        //
        // Only asked when a rule was claimed AND the checker can parse the
        // name: an unknown name makes `oo-cert` exit 2, which is "a file could
        // not be read" and not a rejection, and reporting exit 2 as a rejected
        // step would be the wrong alarm in the wrong direction.
        let mut rule_claim = if st.rule_claimed.is_empty() {
            RuleClaim::NoRuleClaimed
        } else if !known_rule(&st.rule_claimed, &supplied) {
            RuleClaim::RuleNotInTable
        } else if supplied.is_empty() && is_chained(&st.rule_claimed) && !cites_a_list(&st.premises)
        {
            RuleClaim::ChainedRulePremisesIncomplete
        } else if !supplied.is_empty()
            && horn_index_and_binding(&st.rule_claimed, st, &supplied).is_none()
        {
            RuleClaim::NoBindingInstantiatesTheCitedRule
        } else {
            RuleClaim::Licensed
        };
        let mut local = LocalVerdict::NotAsked;
        let mut reproduce = None;
        if rule_claim == RuleClaim::Licensed {
            write_one_step_certificate(&step_dir, kind, st, &supplied)?;
            reproduce = Some(reproduce_command(kind, &step_dir));
            let rules_path = (kind == CertKind::OoHorn).then(|| step_dir.join("rules.tsv"));
            local = match run_checker(
                kind,
                opts.checker.as_deref(),
                &step_dir.join("asserted.tsv"),
                &step_dir.join(kind.derivations_file()),
                rules_path.as_deref(),
            ) {
                CheckerStatus::Accepted(a) => {
                    // THE ONE PLACE A WORD IS CHOSEN, and it is chosen from the
                    // theorem the CHECKER printed. `warrant_of_theorem` is the
                    // whole of the rule that a supplied table cannot earn the
                    // absolute word.
                    let cert = a.certified();
                    match warrant_of_theorem(cert.theorem()) {
                        Some(Warrant::Absolute) => LocalVerdict::SoundChecked(cert),
                        Some(Warrant::UnderSuppliedRules) => {
                            LocalVerdict::SoundUnderSuppliedRulesChecked(cert)
                        }
                        // A zero exit naming a statement this module was not
                        // written against earns nothing. A renamed or weakened
                        // theorem must not keep this label.
                        None => LocalVerdict::NotLicensedByTheCitedRule,
                    }
                }
                CheckerStatus::Rejected { .. } => {
                    rule_claim = RuleClaim::RuleInTableButDidNotFireHere;
                    LocalVerdict::NotLicensedByTheCitedRule
                }
                CheckerStatus::Absent { what, install } => {
                    if opts.require_checker {
                        anyhow::bail!("{what}. {install}");
                    }
                    LocalVerdict::CheckerAbsent
                }
                CheckerStatus::NotNeeded { .. } => LocalVerdict::NotAsked,
                CheckerStatus::Unreadable { stdout } => {
                    anyhow::bail!(
                        "the checker could not read the one-step certificate this module wrote \
                         for step {i}, which is a defect here and never a result about the \
                         ontology: {stdout}"
                    );
                }
            };
        }

        // ── 2. THE STORE-LEVEL QUESTION, which decides the headline ─────────
        //
        // Asked for every step, whatever the local answer was. A step can have
        // a conclusion the store entails some other way, so saying
        // `not_entailed` because a citation was wrong would be false; and a
        // step can be locally sound over premises the store does not hold, so
        // reporting an entailed family on the strength of the local answer
        // would be worse.
        let membership = index.membership(&st.conclusion);
        let label = match membership {
            Membership::Asserted => StepLabel::AssertedNotDerived,
            Membership::Derived => {
                let lines = index.slice_for(&st.conclusion).unwrap_or_default();
                let out = step_dir.join("store").join(kind.derivations_file());
                index.write_slice(&lines, &out)?;
                let rp = (kind == CertKind::OoHorn).then(|| index.rules_path());
                match run_checker(
                    kind,
                    opts.checker.as_deref(),
                    &index.asserted_path(),
                    &out,
                    rp.as_deref(),
                ) {
                    CheckerStatus::Accepted(a) => {
                        let cert = a.certified();
                        match warrant_of_theorem(cert.theorem()) {
                            Some(Warrant::Absolute) => StepLabel::EntailedChecked(cert),
                            Some(Warrant::UnderSuppliedRules) => {
                                StepLabel::EntailedUnderSuppliedRulesChecked(cert)
                            }
                            None => StepLabel::CertificateRejected,
                        }
                    }
                    CheckerStatus::Rejected { .. } => StepLabel::CertificateRejected,
                    CheckerStatus::Absent { what, install } => {
                        if opts.require_checker {
                            anyhow::bail!("{what}. {install}");
                        }
                        unchecked_label(kind)
                    }
                    CheckerStatus::NotNeeded { .. } | CheckerStatus::Unreadable { .. } => {
                        unchecked_label(kind)
                    }
                }
            }
            // ── 3. THE THIRD ANSWER, and the reasons are kept apart ──────────
            Membership::NotDerivable => {
                match local.certified() {
                    // The invented-premise step, and it goes first because it
                    // explains the whole failure: the step's own reasoning was
                    // machine-checked sound and the graph does not hold what it
                    // read. Neither fragment answer would tell the reader that.
                    Some(c) if !premises_in_store => {
                        StepLabel::LocallySoundButPremisesNotInTheStore(c)
                    }
                    // The fragment is asked before the head list because it is
                    // the LANGUAGE fact: it would hold of a perfect OWL 2 RL
                    // engine, while an empty head list is a fact about the
                    // table this build ships and a different table would change
                    // it. `src/dlp.rs`'s module header draws exactly that
                    // order, and a reader who gets the table answer for an
                    // axiom no Horn rule could ever express would go patching
                    // `src/reason.rs` for nothing.
                    _ if fragment.is_some() => StepLabel::OutsideTheFragment,
                    _ if heads.is_empty() => StepLabel::NoRuleInThisTableConcludesThisShape,
                    _ => StepLabel::NotEntailedUnderThisTable,
                }
            }
        };

        steps.push(LabelledStep {
            index: i,
            rule_claimed: st.rule_claimed.clone(),
            conclusion: st.conclusion.clone(),
            premises: st.premises.clone(),
            theorem: label.theorem(),
            subject_sha256: label.subject_sha256(),
            family: label.family(),
            conclusion_in_store: membership_word(membership),
            local: LocalAnswer::of(local),
            premises_in_store,
            premises_not_in_store,
            rule_claim,
            heads_that_could_conclude_it: heads,
            fragment,
            reproduce,
            label,
        });
    }

    build_report(steps, opts, &supplied, &index)
}

fn unchecked_label(kind: CertKind) -> StepLabel {
    match kind {
        CertKind::OoCert => StepLabel::EntailedUnchecked,
        CertKind::OoHorn => StepLabel::EntailedUnderSuppliedRulesUnchecked,
    }
}

fn membership_word(m: Membership) -> &'static str {
    match m {
        Membership::Asserted => "asserted",
        Membership::Derived => "derived",
        Membership::NotDerivable => "not_derivable",
    }
}

/// `asserted.tsv` is the step's OWN premises and the derivation file is the one
/// step. So `OOCert.certificate_sound` reads: the conclusion is true in every
/// model of the triples this step cited. That is local soundness, and it is a
/// different sentence from "the store entails the conclusion", which is why
/// both are asked and neither is reported as the other.
fn write_one_step_certificate(
    dir: &Path,
    kind: CertKind,
    st: &TraceStep,
    supplied: &[RulePattern],
) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    let mut asserted: Vec<u8> = Vec::new();
    for p in &st.premises {
        crate::boundary_core::push_asserted_line_bytes(
            &mut asserted,
            p.0.as_bytes(),
            p.1.as_bytes(),
            p.2.as_bytes(),
        )
        .map_err(|pos| anyhow::anyhow!("a premise term does not fit the format at {pos:?}"))?;
    }
    std::fs::write(dir.join("asserted.tsv"), asserted)?;
    let mut line: Vec<u8> = Vec::new();
    match kind {
        CertKind::OoCert => line.extend_from_slice(st.rule_claimed.as_bytes()),
        CertKind::OoHorn => {
            // `oo-horn` cites a rule by INDEX into the table and carries the
            // binding in full, because `OOCert.checkHornStep` demands a binding
            // that instantiates the body. The binding is recovered by matching
            // the cited premises against the rule's body in order; a step whose
            // premises do not instantiate the body has no binding to write and
            // is `no_binding_instantiates_the_cited_rule` before this is
            // reached.
            let (ri, binds) = horn_index_and_binding(&st.rule_claimed, st, supplied)
                .ok_or_else(|| anyhow::anyhow!("no binding instantiates the cited rule's body"))?;
            std::fs::write(dir.join("rules.tsv"), crate::reason::rules_tsv(supplied))?;
            line.extend_from_slice(ri.to_string().as_bytes());
            line.push(b'\t');
            line.extend_from_slice(binds.len().to_string().as_bytes());
            for (v, term) in &binds {
                if !crate::boundary_core::field_fits_the_format(v.as_bytes()) {
                    anyhow::bail!(
                        "internal: the rule variable name {v:?} does not fit the certificate \
                         format, so no certificate was written"
                    );
                }
                line.push(b'\t');
                line.extend_from_slice(v.as_bytes());
                line.push(b'\t');
                line.extend_from_slice(term.as_bytes());
            }
        }
    }
    for t in std::iter::once(&st.conclusion).chain(st.premises.iter()) {
        crate::boundary_core::push_triple_fields_bytes(
            &mut line,
            t.0.as_bytes(),
            t.1.as_bytes(),
            t.2.as_bytes(),
        )
        .map_err(|pos| anyhow::anyhow!("a step term does not fit the format at {pos:?}"))?;
    }
    line.push(b'\n');
    std::fs::write(dir.join(kind.derivations_file()), line)?;
    Ok(())
}

/// The index of a supplied rule of this name whose body the cited premises
/// instantiate, and the binding, in the variable order
/// `Reasoner::run_horn_scoped` uses: first occurrence over body then head. The
/// order matters because a certificate this module writes and a certificate the
/// engine writes must be the same file for the same step.
fn horn_index_and_binding(
    name: &str,
    st: &TraceStep,
    supplied: &[RulePattern],
) -> Option<(usize, Vec<(String, String)>)> {
    for (ri, r) in supplied.iter().enumerate() {
        if r.name != name || r.body.len() != st.premises.len() {
            continue;
        }
        let mut bound: BTreeMap<&str, &str> = BTreeMap::new();
        let mut ok = true;
        for (atom, prem) in r.body.iter().zip(st.premises.iter()) {
            if !unify_atom(atom, prem, &mut bound) {
                ok = false;
                break;
            }
        }
        if !ok || !unify_atom(&r.head, &st.conclusion, &mut bound) {
            continue;
        }
        // First-occurrence order over body then head, which is exactly what
        // `run_horn_scoped` builds its `vars` vector from.
        let mut order: Vec<&str> = Vec::new();
        for a in r.body.iter().chain(std::iter::once(&r.head)) {
            for p in [&a.s, &a.p, &a.o] {
                if let Pat::Var(v) = p
                    && !order.contains(&v.as_str())
                {
                    order.push(v.as_str());
                }
            }
        }
        let binds: Vec<(String, String)> = order
            .iter()
            .filter_map(|v| bound.get(*v).map(|t| ((*v).to_string(), (*t).to_string())))
            .collect();
        if binds.len() != order.len() {
            // `parse_rules` refuses a head variable absent from the body, so
            // this cannot happen for a table it accepted. If it ever does, the
            // step gets no certificate rather than a half-written one.
            continue;
        }
        return Some((ri, binds));
    }
    None
}

fn unify_atom<'a>(a: &'a AtomPat, t: &'a Spelled, bound: &mut BTreeMap<&'a str, &'a str>) -> bool {
    for (pat, term) in [(&a.s, &t.0), (&a.p, &t.1), (&a.o, &t.2)] {
        match pat {
            Pat::Const(c) => {
                if c != term {
                    return false;
                }
            }
            Pat::Var(v) => match bound.insert(v.as_str(), term.as_str()) {
                Some(prev) if prev != term.as_str() => return false,
                _ => {}
            },
        }
    }
    true
}

/// Dimension one, asked of the axioms this step CITED rather than of the whole
/// ontology, because a step is labelled on its own premises. `None` when every
/// cited premise this classifier recognises is inside the fragment, which
/// includes the case where it recognises none of them.
fn fragment_finding(classified: &[Option<crate::dlp::Classified>]) -> Option<serde_json::Value> {
    let mut rows: Vec<serde_json::Value> = Vec::new();
    for c in classified.iter().flatten() {
        if c.standing != crate::dlp::Standing::Inside {
            rows.push(serde_json::json!({
                "axiom_type": c.kind,
                "subject": c.subject,
                "object": c.object,
                "standing": c.standing.word(),
                "outside_because": c.excluded,
                "horn_but_outside_owl2_rl": c.horn_but_outside_owl2_rl,
            }));
        }
    }
    (!rows.is_empty()).then(|| serde_json::json!({ "premises_outside_the_fragment": rows }))
}

fn known_rule(name: &str, supplied: &[RulePattern]) -> bool {
    if !supplied.is_empty() {
        return supplied.iter().any(|r| r.name == name);
    }
    BUILTIN_RULES.iter().any(|r| r.name == name) || CHAINED_RULES.contains(&name)
}

fn is_chained(name: &str) -> bool {
    CHAINED_RULES.contains(&name)
}

/// A chained rule reads an RDF list, so its premises must carry the chain.
fn cites_a_list(premises: &[Spelled]) -> bool {
    premises.iter().any(|p| p.1 == RDF_REST)
}

fn reproduce_command(kind: CertKind, dir: &Path) -> String {
    let d = dir.display();
    match kind {
        CertKind::OoCert => {
            format!("cd lean && lake exe oo-cert {d}/asserted.tsv {d}/derivations.tsv")
        }
        CertKind::OoHorn => {
            format!("cd lean && lake exe oo-horn check {d}/rules.tsv {d}/asserted.tsv {d}/horn.tsv")
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

const FAMILIES_MEAN: &str =
    "three families. `entailed` and `entailed_under_supplied_rules` are the same family under two \
     different warrants and the second is NEVER shortened to the first, because a conclusion that \
     holds only in models satisfying rules nobody checked is not entailed. `not_entailed` means \
     not derivable under this table. `outside_the_fragment` means no data could make this table \
     derive it, and its two labels are fixed in two different places: outside_the_fragment is a \
     rewrite of the ontology, no_rule_in_this_table_concludes_this_shape is a patch to \
     src/reason.rs";

const BOUNDED_BY: &str =
    "this engine evaluates 29 of OWL 2 RL's 78 rules and RDF entailment is open-world, so \
     not_entailed means NOT DERIVABLE UNDER THIS TABLE and never false";

fn build_report(
    steps: Vec<LabelledStep>,
    opts: &Opts,
    supplied: &[RulePattern],
    index: &CertificateIndex,
) -> anyhow::Result<TraceLabelReport> {
    let mut by_label: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut by_family: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut by_rule_claim: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut by_local_verdict: BTreeMap<&'static str, usize> = BTreeMap::new();
    for s in &steps {
        *by_label.entry(s.label.word()).or_default() += 1;
        *by_family.entry(s.family).or_default() += 1;
        *by_rule_claim.entry(s.rule_claim.word()).or_default() += 1;
        *by_local_verdict.entry(s.local.verdict.word()).or_default() += 1;
    }
    // 2 when a certificate this module or the emitter assembled was REJECTED,
    // because that is a defect here and not a finding about the trace. 1 when
    // any step is outside an entailed family, which is the ordinary finding. 0
    // when every step's conclusion is entailed by the store.
    let exit_code = if steps.iter().any(|s| s.label == "certificate_rejected") {
        2
    } else if steps.iter().all(|s| s.label.is_entailed_family()) {
        0
    } else {
        1
    };
    let (rules_tsv_sha256, conditional_on) = match &opts.rules {
        Some(p) => (
            Some(sha256_hex(crate::reason::rules_tsv(supplied).as_bytes())),
            Some(format!(
                "the rules in {}, which this run ASSUMED and never checked. A rule saying every \
                 supplier is compliant produces steps that check green for ever",
                p.display()
            )),
        ),
        None => (None, None),
    };
    let report = TraceLabelReport {
        format: "oo-trace-label/1",
        profile: opts.profile.clone(),
        steps,
        by_label,
        by_family,
        by_rule_claim,
        by_local_verdict,
        rules_tsv_sha256,
        conditional_on,
        certificate_dir: index.dir().display().to_string(),
        dataset: opts.work_dir.join("labels.jsonl").display().to_string(),
        families_mean: FAMILIES_MEAN,
        bounded_by: BOUNDED_BY,
        exit_code,
    };
    write_dataset(&report, opts, index)?;
    Ok(report)
}

/// One JSON object per line into `<work_dir>/labels.jsonl`, one line per step.
///
/// What makes it worth having, concretely and not as a claim: the label is not
/// a model's judgement and not this engine's opinion. It is the exit code of a
/// checker whose soundness is a machine-checked theorem, over files that ship
/// beside the record, reachable by the command in the `reproduce` field. A
/// consumer who does not trust us re-runs it. `subject_sha256` binds the token
/// to the exact bytes the accepting run was handed, computed by
/// `verdict::subject_digest` before the spawn, so a record cannot be moved onto
/// different files. And the negative labels are what a sound/unsound classifier
/// cannot get anywhere else: `outside_the_fragment` and
/// `no_rule_in_this_table_concludes_this_shape` are labelled negatives WITH A
/// REASON, which is the distinction no LLM judge produces and no benchmark of
/// entailed/not-entailed pairs contains.
fn write_dataset(
    report: &TraceLabelReport,
    opts: &Opts,
    index: &CertificateIndex,
) -> anyhow::Result<()> {
    let asserted_sha256 = std::fs::read_to_string(index.dir().join("asserted.sha256"))
        .ok()
        .map(|s| s.trim().to_string());
    let mut out = String::new();
    for s in &report.steps {
        let row = serde_json::json!({
            "format": report.format,
            "ontology_asserted_sha256": asserted_sha256,
            "profile": report.profile,
            "step": s.index,
            "rule_claimed": s.rule_claimed,
            "premises": s.premises,
            "conclusion": s.conclusion,
            "label": s.label.word(),
            "family": s.family,
            "theorem": s.theorem,
            "subject_sha256": s.subject_sha256,
            "conclusion_in_store": s.conclusion_in_store,
            "local_verdict": s.local.verdict.word(),
            "local_theorem": s.local.theorem,
            "local_subject_sha256": s.local.subject_sha256,
            "premises_in_store": s.premises_in_store,
            "premises_not_in_store": s.premises_not_in_store,
            "rule_claim": s.rule_claim.word(),
            "heads_that_could_conclude_it": s.heads_that_could_conclude_it,
            "fragment": s.fragment,
            "rules_tsv_sha256": report.rules_tsv_sha256,
            "conditional_on": report.conditional_on,
            "reproduce": s.reproduce,
            "bounded_by": report.bounded_by,
        });
        out.push_str(&serde_json::to_string(&row)?);
        out.push('\n');
    }
    std::fs::write(opts.work_dir.join("labels.jsonl"), out)?;
    Ok(())
}
