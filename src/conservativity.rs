//! Does adding these axioms change anything I could already say?
//!
//! [`crate::closure_diff`] already computes `closure(G) \ closure(P)` for a
//! `P ⊆ G`, with the rule and the premises behind every row and the
//! monotonicity gate running for free. Conservativity is that computation
//! pointed the other way: take `G` to be the EXTENDED ontology and `P` to be
//! the one that was there before, and the difference is exactly the set of
//! consequences the extension introduced. Restrict it to the old signature and
//! the finding is "this change alters what the ontology says about terms that
//! already existed".
//!
//! Nothing here recomputes a closure, a certificate or a verdict word:
//! [`crate::closure_diff::SourceClosure`] does all of it, so there is still
//! exactly ONE place in this crate where `checked` is produced.
//!
//! # What this is NOT, said once here and again in the payload
//!
//! Deductive conservativity in a description logic is a different problem from
//! the one this file solves, and it is a hard one: ExpTime-complete for `EL`,
//! 2ExpTime-complete for `ALC` (Ghilardi, Lutz and Wolter, KR 2006, "Did I
//! Damage My Ontology?"), and UNDECIDABLE for `ALCQIO` (Lutz, Walther and
//! Wolter, IJCAI 2007). Model conservativity is stronger again and is
//! undecidable already for `EL` (Lutz and Wolter, JSC 2010).
//!
//! What is computed here is conservativity WITH RESPECT TO THE RULE TABLE the
//! engine evaluates, which is a Horn approximation of OWL and derives only
//! positive ground triples. It is sound in one direction only and the report
//! says so in a field, not a doc comment: a new consequence found IS a real
//! change to what the ontology derives, while finding none does NOT establish
//! that the extension is conservative in any description logic. The field is
//! called `conservative_under_rule_table` and there is deliberately no
//! `conservative` anywhere in the payload.

use crate::closure_diff::{CertificateVerdict, DiffOptions, NtTriple, SourceClosure, Warrant};
use crate::graph::GraphStore;
use crate::projection_entailment as pe;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::Arc;

/// Skolem prefix for the BASE graph.
///
/// The base is skolemised before the extension is merged in, so that the merged
/// store has blank nodes from the extension only and the base is a literal
/// subset of it. A distinct prefix is what stops `_:b0` in the base and `_:b0`
/// in the extension mapping onto the same IRI, which would silently identify
/// two different existentials. `/` cannot occur in an N-Triples blank node
/// label, so no default-prefix skolem IRI can collide with one of these.
const BASE_SKOLEM_PREFIX: &str = "https://open-ontologies.org/.well-known/genid/base/";

/// Namespaces whose IRIs are logical vocabulary rather than names of the
/// ontology's own things. A conclusion is "over the old signature" when every
/// IRI in it is either one of these or a name the base already used: otherwise
/// `ex:Old rdfs:subClassOf ex:Older` would count as outside the signature
/// because `rdfs:subClassOf` is not a class the base declared.
const LOGICAL_NAMESPACES: &[&str] = &[
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
    "http://www.w3.org/2000/01/rdf-schema#",
    "http://www.w3.org/2002/07/owl#",
    "http://www.w3.org/2001/XMLSchema#",
];

pub const WHAT_THIS_IS_NOT: &str =
    "this is conservativity WITH RESPECT TO THE RULE TABLE named in rule_table, and nothing else. \
     It is computed as closure(base ∪ extension) minus closure(base) under that table, restricted \
     to triples every name of which the base already used. It is NOT deductive conservativity in a \
     description logic (ExpTime-complete for EL, 2ExpTime-complete for ALC, and UNDECIDABLE for \
     ALCQIO) and it is NOT model conservativity (undecidable already for EL). The asymmetry is the \
     point: a new consequence reported here IS a real change to what this engine derives over the \
     old names, while finding none establishes only that this rule table derives nothing new, and \
     never that the extension is conservative in any logic.";

pub const FINDING_NOT_AN_ERROR: &str =
    "a non-conservative extension is a FINDING, not a failure. Adding axioms that change what the \
     ontology says about existing terms is often exactly the intended change; the point of \
     reporting it is that it should be intended rather than discovered later. Each row names the \
     rule that produced it and the premises the base did not have, so the decision can be made on \
     the derivation rather than on the verdict word.";

/// Five words, never collapsed into a boolean and never into each other.
pub const CONSERVATIVE: &str = "conservative_under_rule_table";
pub const NOT_CONSERVATIVE: &str = "not_conservative_under_rule_table";
pub const UNDECIDED_TRUNCATED: &str = "undecided_scan_truncated";
pub const UNDECIDED_NOT_AN_EXTENSION: &str = "undecided_not_an_extension";
/// The engine derived something for the base that it did not derive for the
/// EXTENDED graph. The base is a subset by construction and the rule table is
/// monotone, so that cannot happen unless the engine is wrong, and no answer
/// computed from those two closures means anything. It is never folded into
/// `not_conservative`, because a soundness bug and a finding about the ontology
/// go to opposite teams.
pub const UNDECIDED_ENGINE_UNSOUND: &str = "undecided_engine_soundness_violation";

// ───────────────────────────────────────────────────────────────────────────
// Options
// ───────────────────────────────────────────────────────────────────────────

/// How to read the turtle the caller supplied.
///
/// Guessing it is not an option. A delta that happens to restate one triple of
/// the base looks exactly like a replacement that dropped everything else, and
/// the two answers are opposite, so the caller says which it meant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionMode {
    /// The turtle holds the axioms being ADDED. The result is an extension by
    /// construction and nothing can be dropped.
    Delta,
    /// The turtle is the WHOLE proposed graph, as `onto_plan` receives it. Any
    /// asserted triple of the base it does not contain is a removal, and a
    /// change that removes is not an extension.
    Replacement,
}

impl ExtensionMode {
    pub fn parse(s: &str) -> anyhow::Result<ExtensionMode> {
        match s {
            "delta" | "added" => Ok(ExtensionMode::Delta),
            "replacement" | "full" => Ok(ExtensionMode::Replacement),
            other => anyhow::bail!(
                "unknown extension mode {other:?}: pass \"delta\" (the turtle holds only what is \
                 being added) or \"replacement\" (the turtle is the whole proposed graph)"
            ),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            ExtensionMode::Delta => "delta",
            ExtensionMode::Replacement => "replacement",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConservativityOptions {
    pub mode: ExtensionMode,
    /// `rdfs`, `owl-rl` or `owl-rl-ext`. `owl-dl` is refused upstream, because
    /// the tableaux path emits no rule trace.
    pub profile: String,
    /// Where both certificates land. A run is reproducible by hand from what it
    /// leaves behind.
    pub out: std::path::PathBuf,
    /// How many new consequences are examined for signature membership. The
    /// verdict is `undecided_scan_truncated` when the difference is larger,
    /// because a verdict computed from a prefix of the difference is not a
    /// verdict.
    pub scan_rows: usize,
    /// Rows rendered. Totals are always exact.
    pub max_rows: usize,
}

impl Default for ConservativityOptions {
    fn default() -> Self {
        ConservativityOptions {
            mode: ExtensionMode::Delta,
            profile: "owl-rl".into(),
            out: std::path::PathBuf::from("conservativity"),
            scan_rows: 200_000,
            max_rows: 100,
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Findings
// ───────────────────────────────────────────────────────────────────────────

/// A conclusion the extended ontology reaches and the base does not.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewConsequence {
    pub triple: NtTriple,
    /// True when every name in the triple is one the base already used, so this
    /// row changes what the ontology says about terms that already existed.
    /// That is the conservativity finding; the rest are the ordinary effect of
    /// adding new terms.
    pub over_old_signature: bool,
    pub warrant: Warrant,
    pub warrant_word: &'static str,
    /// Named only where a machine-checked theorem stands behind the row.
    pub theorem: Option<&'static str>,
    /// The rule that first derived it in the extended ontology. `None` when the
    /// extension ASSERTED it outright rather than deriving it.
    pub rule: Option<String>,
    /// The premises of that derivation the base's own closure does not hold:
    /// what the extension had to add for this conclusion to appear. The
    /// actionable half.
    pub premises_the_base_lacked: Vec<NtTriple>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConservativityReport {
    pub format: &'static str,
    /// One sentence the reader cannot miss, first after `format`.
    pub headline: String,
    /// `conservative_under_rule_table`, `not_conservative_under_rule_table`,
    /// `undecided_scan_truncated`, `undecided_not_an_extension` or
    /// `undecided_engine_soundness_violation`.
    pub conservativity_verdict: &'static str,
    /// `null` when the verdict is one of the undecided words. Never a bare
    /// `conservative`, and never `false` standing in for "we could not tell".
    pub conservative_under_rule_table: Option<bool>,
    /// What the verdict is about. Not a decoration: the verdict means nothing
    /// without it.
    pub rule_table: String,
    pub what_this_is_not: &'static str,
    pub finding_not_an_error: &'static str,
    /// How the supplied turtle was read: `delta` or `replacement`.
    pub mode: &'static str,

    /// True when the proposed graph contains every asserted triple of the base,
    /// so applying it is a pure addition. False means the change also REMOVES,
    /// and removal is not what conservativity is about: the verdict is then
    /// undecided and the removals are counted.
    pub is_an_extension: bool,
    pub base_triples_the_proposal_drops: usize,
    pub dropped_examples: Vec<String>,
    /// Base triples whose blank nodes could not be matched against the
    /// proposal, so `base_triples_the_proposal_drops` was decided over the
    /// ground ones only. Counted, never folded into the denominator.
    pub not_compared_blank_node_bearing: usize,

    pub base_signature_size: usize,
    /// Names the extension introduces that the base never used.
    pub signature_added: Vec<String>,
    pub signature_added_total: usize,

    pub new_consequences_over_old_signature: Vec<NewConsequence>,
    pub new_consequences_over_old_signature_total: usize,
    /// Every new consequence, including those that mention a new name. Bigger
    /// by construction and NOT the headline: a new class brings new conclusions
    /// about itself, which is what adding a class is.
    pub new_consequences_total: usize,
    /// New consequences the scan did not examine, because the difference was
    /// larger than `scan_rows`. Non-zero forces the undecided verdict.
    pub not_examined: usize,

    /// The engine's own soundness gate, run for free by the closure diff: a
    /// conclusion the BASE reaches that the EXTENDED ontology does not. The
    /// rule table is monotone and the base is a subset, so this must be empty,
    /// and a non-empty list is an engine bug rather than a finding about the
    /// ontology.
    pub engine_soundness_violations: Vec<String>,
    /// Non-null when the monotonicity gate did NOT run, with the reason, so an
    /// empty violation list can never mean "we did not look".
    pub engine_soundness_gate_skipped: Option<String>,

    pub base_certificate: CertificateVerdict,
    pub extended_certificate: CertificateVerdict,
    pub seconds: f64,
    /// 0 conservative under the rule table, 1 a finding, 2 undecided.
    pub exit_code: i32,
}

// ───────────────────────────────────────────────────────────────────────────
// The check
// ───────────────────────────────────────────────────────────────────────────

fn iri_of(term: &str) -> Option<&str> {
    term.strip_prefix('<').and_then(|t| t.strip_suffix('>'))
}

fn is_logical(iri: &str) -> bool {
    LOGICAL_NAMESPACES.iter().any(|ns| iri.starts_with(ns))
}

fn names_of<'a>(triples: impl Iterator<Item = &'a (String, String, String)>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (s, p, o) in triples {
        for t in [s, p, o] {
            if let Some(i) = iri_of(t)
                && !is_logical(i)
            {
                out.insert(i.to_string());
            }
        }
    }
    out
}

fn nt_of(triples: &[(String, String, String)]) -> String {
    triples.iter().map(|(s, p, o)| format!("{s} {p} {o} .\n")).collect()
}

/// Does adding `extension_ttl` to `base` change any consequence over the names
/// the base already used?
///
/// `extension_ttl` is read as the graph to MERGE IN, so the extended ontology
/// is `base ∪ extension` and the subset precondition the monotonicity gate
/// needs holds by construction. When the caller's turtle is a full REPLACEMENT
/// rather than a delta — which is what `onto_plan` receives — the triples it
/// drops are reported in `base_triples_the_proposal_drops` and the verdict
/// becomes `undecided_not_an_extension`, because a change that removes is not
/// an extension and this question is not the one to ask about it.
pub fn conservativity_check(
    base: &Arc<GraphStore>,
    extension_ttl: &str,
    opts: &ConservativityOptions,
) -> anyhow::Result<ConservativityReport> {
    let started = std::time::Instant::now();

    // ── Ground both sides so every comparison is exact ───────────────────
    let (base_sk, _) = pe::skolemise_with_prefix(base, BASE_SKOLEM_PREFIX)?;
    let base_triples = base_sk.all_triples()?;
    let base_nt = nt_of(&base_triples);

    let extended = Arc::new(GraphStore::new());
    extended.load_ntriples(&base_nt)?;
    extended.load_turtle(extension_ttl, None).map_err(|e| {
        anyhow::anyhow!("the proposed extension is not readable as Turtle: {e}")
    })?;

    // ── Is it an extension at all? Decided over the ORIGINAL base ────────
    //
    // Only in `Replacement` mode is the question meaningful. A delta says
    // nothing about the triples it does not mention, so counting them as
    // removals would report every delta as a removal of the entire ontology.
    let proposed = Arc::new(GraphStore::new());
    proposed.load_turtle(extension_ttl, None)?;
    let original = base.all_triples()?;
    let bn = |t: &(String, String, String)| {
        pe::is_blank(&t.0) || pe::is_blank(&t.1) || pe::is_blank(&t.2)
    };
    let (dropped, not_compared_blank_node_bearing) = match opts.mode {
        ExtensionMode::Delta => (Vec::new(), 0),
        ExtensionMode::Replacement => {
            let proposed_set: BTreeSet<(String, String, String)> =
                proposed.all_triples()?.into_iter().collect();
            let unmatched = original.iter().filter(|t| bn(t)).count()
                + proposed_set.iter().filter(|t| bn(t)).count();
            let gone: Vec<String> = original
                .iter()
                .filter(|t| !bn(t) && !proposed_set.contains(*t))
                .map(|(s, p, o)| format!("{s} {p} {o}"))
                .collect();
            (gone, unmatched)
        }
    };
    let is_an_extension = dropped.is_empty();
    let mut dropped_examples: Vec<String> = dropped.iter().take(10).cloned().collect();
    dropped_examples.sort();

    // ── One closure diff, in the extension direction ─────────────────────
    let diff_opts = DiffOptions {
        profile: opts.profile.clone(),
        out: opts.out.clone(),
        checker: None,
        // The base is already ground and the extension's blank nodes are
        // skolemised by the diff itself, under the DEFAULT prefix, which cannot
        // collide with BASE_SKOLEM_PREFIX.
        skolemise_source: true,
        max_rows: opts.scan_rows,
        seed_iris: Vec::new(),
    };
    let ext_closure = SourceClosure::build(&extended, &diff_opts)?;
    let report = ext_closure.diff(&base_nt, &diff_opts)?;

    // ── Partition the difference by the OLD signature ────────────────────
    let base_names = names_of(base_triples.iter());
    let extension_names = names_of(proposed.all_triples()?.iter());
    let signature_added: Vec<String> =
        extension_names.difference(&base_names).cloned().collect();

    let over_old = |t: &NtTriple| -> bool {
        [&t.0, &t.1, &t.2].into_iter().all(|term| match iri_of(term) {
            Some(i) => is_logical(i) || base_names.contains(i),
            // A literal is a value, not a name. A blank node has no name that
            // survives a graph boundary and is never waved through.
            None => term.starts_with('"'),
        })
    };

    let mut rows: Vec<NewConsequence> = Vec::new();
    let mut over_old_total = 0usize;
    for lost in &report.entailments_lost {
        let is_over = over_old(&lost.triple);
        if !is_over {
            continue;
        }
        over_old_total += 1;
        if rows.len() < opts.max_rows {
            rows.push(NewConsequence {
                triple: lost.triple.clone(),
                over_old_signature: true,
                warrant: lost.warrant,
                warrant_word: lost.warrant_word,
                theorem: lost.theorem,
                rule: lost.rule.clone(),
                premises_the_base_lacked: lost.blocking_premises.clone(),
            });
        }
    }
    let not_examined = report.lost_total.saturating_sub(report.entailments_lost.len());

    let engine_soundness_violations: Vec<String> = report
        .monotonicity_violations
        .iter()
        .map(|v| format!("{} {} {}", v.triple.0, v.triple.1, v.triple.2))
        .collect();

    // ── The verdict, never a bare boolean, and never collapsed ───────────
    //
    // The engine check comes FIRST. A monotonicity violation means the two
    // closures this answer was computed from cannot both be right, so every
    // count below it is meaningless, and reporting "not conservative" would
    // send a soundness bug to the ontology's author.
    let (verdict, flag, exit_code) = if !engine_soundness_violations.is_empty() {
        (UNDECIDED_ENGINE_UNSOUND, None, 2)
    } else if !is_an_extension {
        (UNDECIDED_NOT_AN_EXTENSION, None, 2)
    } else if not_examined > 0 {
        (UNDECIDED_TRUNCATED, None, 2)
    } else if over_old_total > 0 {
        (NOT_CONSERVATIVE, Some(false), 1)
    } else {
        (CONSERVATIVE, Some(true), 0)
    };

    let headline = format!(
        "{verdict}: {over_old_total} new consequence(s) over the {} name(s) the base already used, \
         out of {} new consequence(s) in total, under the {} rule table. Extended certificate: \
         {}{}. {}",
        base_names.len(),
        report.lost_total,
        opts.profile,
        report.source_certificate.verdict,
        match report.source_certificate.theorem {
            Some(t) => format!(" ({t})"),
            None => String::new(),
        },
        WHAT_THIS_IS_NOT,
    );

    Ok(ConservativityReport {
        format: "oo-conservativity/1",
        headline,
        conservativity_verdict: verdict,
        conservative_under_rule_table: flag,
        rule_table: opts.profile.clone(),
        what_this_is_not: WHAT_THIS_IS_NOT,
        finding_not_an_error: FINDING_NOT_AN_ERROR,
        mode: opts.mode.name(),
        is_an_extension,
        base_triples_the_proposal_drops: dropped.len(),
        dropped_examples,
        not_compared_blank_node_bearing,
        base_signature_size: base_names.len(),
        signature_added_total: signature_added.len(),
        signature_added: signature_added.into_iter().take(opts.max_rows).collect(),
        new_consequences_over_old_signature: rows,
        new_consequences_over_old_signature_total: over_old_total,
        new_consequences_total: report.lost_total,
        not_examined,
        engine_soundness_violations,
        engine_soundness_gate_skipped: report.monotonicity_gate_skipped.clone(),
        base_certificate: report.projection_certificate.clone(),
        extended_certificate: report.source_certificate.clone(),
        seconds: started.elapsed().as_secs_f64(),
        exit_code,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_logical_namespace_is_not_a_name() {
        assert!(is_logical("http://www.w3.org/2000/01/rdf-schema#subClassOf"));
        assert!(is_logical("http://www.w3.org/2002/07/owl#Thing"));
        assert!(!is_logical("http://ex.org/Cat"));
    }

    #[test]
    fn the_base_skolem_prefix_cannot_collide_with_the_default_one() {
        // The default prefix maps `_:label` to `<PREFIX + label>` and an
        // N-Triples blank node label cannot contain `/`, so no default-prefix
        // skolem IRI can ever spell a base-prefix one.
        assert!(BASE_SKOLEM_PREFIX.starts_with(pe::SKOLEM_PREFIX));
        let tail = &BASE_SKOLEM_PREFIX[pe::SKOLEM_PREFIX.len()..];
        assert!(tail.contains('/'), "the discriminator must be unspellable as a label: {tail:?}");
    }

    #[test]
    fn five_verdict_words_and_no_bare_conservative() {
        let words = [
            CONSERVATIVE,
            NOT_CONSERVATIVE,
            UNDECIDED_TRUNCATED,
            UNDECIDED_NOT_AN_EXTENSION,
            UNDECIDED_ENGINE_UNSOUND,
        ];
        for w in words {
            assert_ne!(w, "conservative", "a bare word is the one spelling that may not appear");
        }
        let distinct: BTreeSet<&str> = words.into_iter().collect();
        assert_eq!(distinct.len(), words.len(), "two verdicts must never share a spelling");
    }
}
