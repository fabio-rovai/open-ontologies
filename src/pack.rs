//! Knowledge packs: a verified graph, its evidence and its proof in one
//! portable file.
//!
//! TrustGraph calls the idea a knowledge core, and it is the right one: what
//! you promote between environments should be a versioned artifact, not a
//! pile of loose Turtle whose provenance and checks live somewhere else. The
//! shape here is deliberately boring so that anything can read it, and the
//! graph is stored as ordinary N-Triples rather than a proprietary blob.
//!
//! A pack carries:
//!
//!   - the graph itself (N-Triples, sorted, so two packs of the same graph
//!     are byte-identical and diffable);
//!   - a manifest: name, version, counts, creation time, tool version;
//!   - a checksum over the graph, so tampering or truncation is detectable;
//!   - the verification evidence recorded at pack time (lint, enforce), so
//!     the receiving environment can see what the graph passed rather than
//!     take it on trust;
//!   - and, when the sender has one, the derivation certificate for the run
//!     that produced the graph, plus a second checksum over the graph AND the
//!     certificate together.
//!
//! # Evidence and proof are different objects
//!
//! `evidence` is what the SENDER'S engine said. A receiver who reads it is
//! trusting a report. The certificate is not a report: it is the premises and
//! the derivation steps, and the receiver runs their own checker over them.
//! The engine that wrote the pack is then out of the trusted set for that one
//! claim, which is why the two live in separate fields rather than being
//! folded together.
//!
//! What a certificate settles and what it does not is stated in the RECEIVER'S
//! report rather than only here, in [`MEANS`], [`DOES_NOT_MEAN`] and
//! [`ABSENT_MEANS`]. The short form: it proves the materialised triples follow
//! from the asserted ones under the rules that ran. It proves nothing about
//! whether the asserted triples are true, and it says nothing about axioms no
//! rule in the fragment can read.
//!
//! Unpacking verifies both digests before loading a single triple, and a
//! certificate this machine's checker REFUSED stops the load unless the caller
//! asks for it anyway.

use crate::graph::{GraphStore, ReadScope};
use crate::projection_entailment::{self as pe, CertKind, CheckerStatus, LAKE_INSTALL};
use crate::verdict::Certified;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The only file names a pack may carry inside a certificate, in BOTH
/// directions.
///
/// At pack time a certificate directory can hold whatever an earlier run left
/// in it, and slurping the directory would put unrelated bytes under the
/// content digest. At unpack time the names come from the SENDER, and a pack
/// naming `../../.ssh/authorized_keys` would otherwise be written wherever the
/// receiver's scratch directory resolves that to. Every name here is one
/// `Reasoner::run_full` writes or one `Reasoner::run_horn_scoped` writes.
const CERTIFICATE_FILES: &[&str] = &[
    "asserted.tsv",
    "asserted.sha256",
    "derivations.tsv",
    "horn.tsv",
    "rules.tsv",
    "scope.tsv",
    "refutation.tsv",
];

/// A certificate directory, carried inside a pack.
///
/// The files travel as text rather than as base64 because they are TSV and a
/// pack is meant to be diffable, which is the same reason the graph is sorted
/// N-Triples. Bytes are preserved exactly: nothing here normalises a line
/// ending, because the checker reads these bytes and a rewritten `\r\n` would
/// be a different file from the one the reasoner wrote and hashed.
///
/// ONE certificate, and the field is singular in the format rather than a list.
/// A real promotion may involve several runs, an RDFS pass and then a supplied
/// Horn table, and making this a list would be cheap now and expensive later.
/// It is singular because a second certificate raises a question nobody can
/// answer honestly yet: what the COMBINED verdict is when one is accepted and
/// the other refused. Until there is an answer to that, a pack that needs two
/// proofs is two packs.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PackedCertificate {
    /// `oo-cert/1` or `oo-horn/1`, echoed from the reasoner's report. ECHOED
    /// AND NEVER ACTED ON: [`kind_of`] decides which checker reads this from
    /// the files that are present, so a sender cannot point the receiver at
    /// the wrong checker by editing a string.
    pub format: String,
    /// File name to contents. A `BTreeMap` because [`content_digest`] frames
    /// the entries in iteration order, and two orders would be two digests for
    /// one certificate.
    pub files: BTreeMap<String, String>,
    /// Lines in `asserted.tsv` and in the derivations file. Under the content
    /// digest like everything else, so a reader can size the certificate before
    /// writing a byte of it to disk and still be reading a number nobody
    /// edited.
    pub asserted: usize,
    pub derivations: usize,
    /// The profile or rule file the SENDER says produced this. Nothing in the
    /// certificate proves it and no checker reads it: `oo-cert` checks each
    /// step against the rule the step itself names, and `oo-horn` against
    /// `rules.tsv`. It is recorded because it is useful and it is labelled a
    /// sender's claim everywhere it is printed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_claimed_by_sender: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub created_at: String,
    pub tool_version: String,
    pub triples: usize,
    /// SHA-256 of the graph text and NOTHING ELSE. Unchanged, including for a
    /// pack that carries a certificate, so a reader who checked packs before
    /// this field existed is still checking the same thing.
    pub sha256: String,
    /// What the graph passed at pack time. Absent means not checked, which
    /// is itself information the receiver should act on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<serde_json::Value>,
    /// The derivation certificate for the run that produced this graph.
    ///
    /// `evidence` above is what the sender's engine SAID. This is what a
    /// receiver's own checker can re-derive, which is a different kind of
    /// object, so it gets its own field rather than being folded into the
    /// evidence blob.
    ///
    /// `#[serde(default)]` here is explicitness and NOT the backward
    /// compatibility guarantee, which is worth writing down because it is easy
    /// to believe otherwise. serde's derive already reads a missing field of
    /// type `Option<T>` as `None`, so deleting the attribute changes nothing:
    /// that was mutated and the old-pack test stayed green. What actually keeps
    /// an old pack loading is the `(None, None)` arm of the digest match in
    /// [`Packer::unpack`], and
    /// `pack_certificate_test::an_old_pack_loads_and_reports_that_it_carries_no_certificate`
    /// is what turns red when that arm starts refusing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificate: Option<PackedCertificate>,
    /// SHA-256 over the graph AND the certificate, framed by
    /// [`content_digest`].
    ///
    /// THE BINDING, stated precisely. `content_sha256` is the digest of, in
    /// order: a domain-separating tag, the length and bytes of the graph text,
    /// a presence byte for the certificate and, when one is present, the
    /// length-prefixed `format`, the length-prefixed
    /// `profile_claimed_by_sender`, the `asserted` and `derivations` counts,
    /// the number of files, and then every (name, body) pair in `BTreeMap`
    /// order, each half length-prefixed. So it covers every field of
    /// [`PackedCertificate`] and every byte of every file in it.
    ///
    /// `sha256` cannot see a swapped certificate, and a swapped certificate is
    /// the whole attack: a certificate that is internally sound over SOMEBODY
    /// ELSE'S assertions checks green, and a receiver that verified only the
    /// graph checksum would accept a true proof about premises that are not in
    /// the pack. Absent on every pack written before this field existed, which
    /// is why a pack that carries a certificate and no `content_sha256` is
    /// refused rather than checked (see [`Packer::unpack`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_sha256: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Pack {
    pub manifest: Manifest,
    /// N-Triples, sorted line-wise.
    pub graph: String,
}

fn checksum(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// The digest that binds a certificate to the graph it travels with.
///
/// Every part is length-prefixed, so no two different (graph, certificate)
/// pairs can frame to the same bytes by running together at a seam. That is the
/// framing [`crate::verdict::subject_digest`] uses, for the same reason, and it
/// is written out explicitly rather than taken from `serde_json::to_string` so
/// that a serde_json release cannot change what a security digest covers.
///
/// Public because the comparison is the point, and because a test that plays a
/// competent attacker has to be able to recompute it.
pub fn content_digest(graph: &str, cert: Option<&PackedCertificate>) -> String {
    let mut h = Sha256::new();
    h.update(b"oo-pack-content/1\n");
    h.update((graph.len() as u64).to_le_bytes());
    h.update(graph.as_bytes());
    match cert {
        None => h.update(0u64.to_le_bytes()),
        Some(c) => {
            h.update(1u64.to_le_bytes());
            for s in [
                c.format.as_str(),
                c.profile_claimed_by_sender.as_deref().unwrap_or(""),
            ] {
                h.update((s.len() as u64).to_le_bytes());
                h.update(s.as_bytes());
            }
            h.update((c.asserted as u64).to_le_bytes());
            h.update((c.derivations as u64).to_le_bytes());
            h.update((c.files.len() as u64).to_le_bytes());
            for (name, body) in &c.files {
                h.update((name.len() as u64).to_le_bytes());
                h.update(name.as_bytes());
                h.update((body.len() as u64).to_le_bytes());
                h.update(body.as_bytes());
            }
        }
    }
    format!("{:x}", h.finalize())
}

/// What a receiver learned about the certificate in a pack. NEVER COLLAPSED.
///
/// The two accepted variants carry a [`Certified`], which has no public
/// constructor anywhere in the crate, so neither can be named on a path that
/// did not run a checker and read a zero exit code off it.
/// [`PackCertVerdict::CheckerAbsent`] and
/// [`PackCertVerdict::NoCertificateInPack`] are separate words on purpose:
/// "this build has no checker" and "this pack has no proof" are different
/// facts, and neither may render as anything that reads like a pass.
///
/// ```compile_fail
/// use open_ontologies::pack::PackCertVerdict;
/// use open_ontologies::verdict::Certified;
/// let v = PackCertVerdict::Accepted(Certified {
///     theorem: "OOCert.certificate_sound",
///     subject: [0u8; 32],
/// });
/// ```
#[derive(Clone, Debug)]
pub enum PackCertVerdict {
    /// CERTIFIED. `oo-cert` accepted the packed certificate under
    /// `OOCert.certificate_sound`.
    Accepted(Certified),
    /// CERTIFIED AND WEAKER. `oo-horn check` accepted a certificate over a rule
    /// table the SENDER supplied, so the conclusions hold in every model of the
    /// asserted triples that also satisfies that table. Never shortened to the
    /// word above.
    AcceptedUnderSuppliedRules(Certified),
    /// Exit 1. The checker named a step it would not accept. A defect in the
    /// pack, never a downgrade to an unchecked verdict.
    Refused,
    /// Exit 2, an unreadable file, an unknown file name, or files that do not
    /// form a certificate either checker reads. Not a verdict in either
    /// direction, exactly as `lean/Main.lean` reserves the two codes.
    Unreadable,
    /// The certificate checks and is about assertions this pack does not
    /// contain. A sound proof over premises nobody shipped.
    NotAboutThisPack,
    /// No checker on this machine. NOTHING WAS CHECKED.
    CheckerAbsent,
    /// The pack carries no certificate at all. Every pack written before this
    /// field existed lands here.
    NoCertificateInPack,
}

impl PackCertVerdict {
    pub fn word(&self) -> &'static str {
        match self {
            PackCertVerdict::Accepted(_) => "certificate_accepted",
            PackCertVerdict::AcceptedUnderSuppliedRules(_) => {
                "certificate_accepted_under_supplied_rules"
            }
            PackCertVerdict::Refused => "certificate_refused",
            PackCertVerdict::Unreadable => "certificate_unreadable",
            PackCertVerdict::NotAboutThisPack => "certificate_not_about_this_pack",
            PackCertVerdict::CheckerAbsent => "checker_absent_nothing_was_checked",
            PackCertVerdict::NoCertificateInPack => "no_certificate_in_pack",
        }
    }
    pub fn theorem(&self) -> Option<&'static str> {
        match self {
            PackCertVerdict::Accepted(c) | PackCertVerdict::AcceptedUnderSuppliedRules(c) => {
                Some(c.theorem())
            }
            _ => None,
        }
    }
    /// True only for the two variants a checker had to accept to produce.
    pub fn is_checked(&self) -> bool {
        self.theorem().is_some()
    }
    /// Whether this verdict stops the graph being loaded. A refused or
    /// unreadable certificate, and one that is about another graph, are defects
    /// in the pack: a graph whose own proof does not check is not a graph to
    /// promote. An absent checker is not a defect and does not block: refusing
    /// the load there would make a proof-carrying pack unusable on every
    /// machine without Lean, which is the opposite of the point.
    pub fn blocks_the_load(&self) -> bool {
        matches!(
            self,
            PackCertVerdict::Refused
                | PackCertVerdict::Unreadable
                | PackCertVerdict::NotAboutThisPack
        )
    }
    /// Every word, for a suite that enumerates the vocabulary without being
    /// able to build the certified variants.
    pub const WORDS: [&'static str; 7] = [
        "certificate_accepted",
        "certificate_accepted_under_supplied_rules",
        "certificate_refused",
        "certificate_unreadable",
        "certificate_not_about_this_pack",
        "checker_absent_nothing_was_checked",
        "no_certificate_in_pack",
    ];
}

// Written by hand: `serialize_as_word!` is private to src/verdict.rs.
impl serde::Serialize for PackCertVerdict {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.word())
    }
}

impl std::fmt::Display for PackCertVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.word())
    }
}

/// What an acceptance settles, in the report rather than in the documentation.
///
/// Held as a constant because the same words belong in `onto_unpack`, in the
/// CLI output and in the decision record, and a claim that lives in four copies
/// drifts in three of them.
pub const MEANS: &str = "A checker on THIS machine re-derived every conclusion in this pack's \
     certificate from the triples the certificate lists as asserted, and accepted all of them. \
     You did not have to trust the sender's engine to learn that. To repeat it by hand, unpack \
     again with certificate_out_dir set and run the command in check_with over the files it \
     keeps.";

/// What an acceptance does NOT settle. Printed beside every accepted verdict.
pub const DOES_NOT_MEAN: &str = "The asserted triples themselves are NOT proved. A certificate \
     proves that the materialised triples follow from the asserted ones under the rules that ran, \
     and nothing more. It does not check that the asserted triples are true, that they describe \
     the world, that they came from the system the sender says they came from, or that the sender \
     left anything out. It is also silent about axioms outside the fragment: a profile run \
     evaluates part of OWL 2 RL, a supplied table is whatever rules.tsv holds, and an axiom no \
     rule fires on contributed nothing and is invisible here. Run onto_dlp_boundary over the \
     loaded graph to see which of its axioms the rule table can read. A sound proof over the \
     wrong premises is still the wrong answer.";

/// What an absent checker settles, which is nothing.
///
/// The wording follows `web/try/api/index.py`, which says the release carries
/// no checker rather than implying a check happened. "We did not check" must
/// never render as "it passed".
pub const ABSENT_MEANS: &str = "No checker ran here, so NOTHING WAS CHECKED. That is not a pass \
     and it is not a failure: no statement in this report rests on a checker having run.";

/// The limit of what running the checker buys, printed beside every checker
/// result.
///
/// The receiver chose the binary. `$OO_CERT` pointing at a script that exits
/// zero and prints the right theorem name earns the accepted word, exactly as
/// `src/verdict.rs` residual hole 1 says. That is now the RECEIVER'S own foot
/// rather than the sender's, which is an improvement and not a closure, and the
/// report says so rather than leaving it in a comment.
pub const CHECKER_LIMIT: &str = "The checker is a binary THIS machine chose. A path in $OO_CERT \
     that exits zero and prints the right theorem name earns the accepted word, so this verdict \
     is only as good as the binary behind it. checker_says_it_is below is the block the binary \
     printed about itself; match its sha256 against the SHASUMS.txt of the release you meant to \
     run. A hostile binary can print any block it likes and nothing here would notice. This is \
     the limit of what running a checker buys you, and it is the receiver's own risk rather than \
     the sender's, which is the improvement.";

/// Which checker a packed certificate is for, decided by the files it carries.
///
/// `run_full` writes `derivations.tsv` and `run_horn_scoped` writes `horn.tsv`
/// plus the `rules.tsv` the run evaluated, so the file names are the
/// discriminator. The `format` field is not consulted: reading it would let a
/// sender choose the receiver's checker.
fn kind_of(files: &BTreeMap<String, String>) -> anyhow::Result<CertKind> {
    anyhow::ensure!(
        files.contains_key("asserted.tsv"),
        "this certificate carries no asserted.tsv, so there is nothing for a checker to check \
         the derivations against"
    );
    match (
        files.contains_key("derivations.tsv"),
        files.contains_key("horn.tsv"),
    ) {
        (true, false) => Ok(CertKind::OoCert),
        (false, true) => {
            anyhow::ensure!(
                files.contains_key("rules.tsv"),
                "horn.tsv is checked against the rules.tsv the run evaluated and this certificate \
                 carries none, so there is no table for oo-horn to check it against"
            );
            Ok(CertKind::OoHorn)
        }
        (true, true) => anyhow::bail!(
            "this certificate carries both derivations.tsv and horn.tsv, so two runs wrote into \
             one directory. Which checker reads it would be decided here rather than by the \
             sender, and the two earn different verdicts, so it is refused"
        ),
        (false, false) => anyhow::bail!(
            "this certificate carries neither derivations.tsv nor horn.tsv, so it records no \
             derivation"
        ),
    }
}

/// Read a certificate directory into the form a pack carries.
pub fn read_certificate_dir(
    dir: &Path,
    profile_claimed: Option<&str>,
) -> anyhow::Result<PackedCertificate> {
    anyhow::ensure!(
        dir.is_dir(),
        "{} is not a directory, so there is no certificate to pack. Run onto_reason with \
         certificate_dir first",
        dir.display()
    );
    let mut files: BTreeMap<String, String> = BTreeMap::new();
    for name in CERTIFICATE_FILES {
        let p = dir.join(name);
        if !p.exists() {
            continue;
        }
        let bytes = std::fs::read(&p)?;
        // Not `from_utf8_lossy`. A pack is JSON, and a lossy conversion would
        // hand the receiver's checker different bytes from the ones the
        // reasoner wrote and hashed into asserted.sha256.
        let body = String::from_utf8(bytes).map_err(|e| {
            anyhow::anyhow!(
                "{} is not valid UTF-8 ({e}), so carrying it in a pack would change the bytes \
                 the checker reads",
                p.display()
            )
        })?;
        files.insert((*name).to_string(), body);
    }
    let kind = kind_of(&files)?;
    let asserted = files["asserted.tsv"]
        .lines()
        .filter(|l| !l.is_empty())
        .count();
    let derivations = files[kind.derivations_file()]
        .lines()
        .filter(|l| !l.is_empty())
        .count();
    Ok(PackedCertificate {
        format: match kind {
            CertKind::OoCert => "oo-cert/1".to_string(),
            CertKind::OoHorn => "oo-horn/1".to_string(),
        },
        files,
        asserted,
        derivations,
        profile_claimed_by_sender: profile_claimed.map(str::to_string),
    })
}

/// How much of a certificate the graph in front of us accounts for.
///
/// Two counts, and they answer two different questions. `in_certificate_only`
/// is the one that can refuse a pack: an assertion the certificate rests on
/// that is not in the graph means the proof is about premises nobody shipped,
/// and the checker cannot see that, because `oo-cert` verifies the steps
/// against the triples in `asserted.tsv` and has no way to ask where they came
/// from. `conclusions_not_in_pack` is reported and never refuses: a sender who
/// reasoned without materialising ships a graph that does not contain its own
/// conclusions, which is a legitimate thing to do and a thing the receiver
/// should be told.
struct Coverage {
    asserted_in_certificate: usize,
    in_certificate_only: Vec<String>,
    in_certificate_only_total: usize,
    conclusions: usize,
    conclusions_not_in_pack: Vec<String>,
    conclusions_not_in_pack_total: usize,
    unparsed_derivation_lines: usize,
}

/// The conclusions a certificate records, in the same spelling `asserted.tsv`
/// uses.
///
/// `derivations.tsv` is `rule` then 3(1+k) term fields, so the conclusion is
/// fields 1..4. `horn.tsv` is
/// `ruleIndex, bindCount, (var, term)*, cs, cp, co, premises*`, so the
/// conclusion starts at 2 + 2 * bindCount. A line whose shape does not parse is
/// COUNTED and never skipped in silence: a swallowed parse failure would let a
/// conclusion escape the coverage count, and the count would then be a number
/// that cannot go wrong.
fn conclusions_of(files: &BTreeMap<String, String>, kind: CertKind) -> (BTreeSet<String>, usize) {
    let mut out = BTreeSet::new();
    let mut unparsed = 0usize;
    for line in files[kind.derivations_file()].lines() {
        if line.is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        let at = match kind {
            CertKind::OoCert => 1usize,
            CertKind::OoHorn => match f.get(1).and_then(|k| k.parse::<usize>().ok()) {
                Some(k) => 2 + 2 * k,
                None => {
                    unparsed += 1;
                    continue;
                }
            },
        };
        match (f.get(at), f.get(at + 1), f.get(at + 2)) {
            (Some(s), Some(p), Some(o)) => {
                out.insert(format!("{s}\t{p}\t{o}"));
            }
            _ => unparsed += 1,
        }
    }
    (out, unparsed)
}

/// Compare a certificate against a graph, in the ONE spelling both sides use.
///
/// [`crate::reason::asserted_bytes`] is the function that turns a selection of
/// triples into `asserted.tsv` bytes, and the certificate's own `asserted.tsv`
/// was written by the same code, so the two sets are comparable as sets of
/// lines and no second spelling is introduced here.
fn coverage(
    graph: &Arc<GraphStore>,
    files: &BTreeMap<String, String>,
    kind: CertKind,
) -> anyhow::Result<Coverage> {
    let (triples, _) = graph.triples_in_scope(&ReadScope::AllGraphs)?;
    let bytes = crate::reason::asserted_bytes(&triples)?;
    let in_graph: BTreeSet<String> = String::from_utf8_lossy(&bytes)
        .lines()
        .map(str::to_string)
        .collect();
    let in_cert: BTreeSet<String> = files["asserted.tsv"]
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    let (concl, unparsed) = conclusions_of(files, kind);
    let missing_premises: Vec<String> = in_cert.difference(&in_graph).cloned().collect();
    let missing_conclusions: Vec<String> = concl.difference(&in_graph).cloned().collect();
    Ok(Coverage {
        asserted_in_certificate: in_cert.len(),
        in_certificate_only_total: missing_premises.len(),
        in_certificate_only: missing_premises.into_iter().take(8).collect(),
        conclusions: concl.len(),
        conclusions_not_in_pack_total: missing_conclusions.len(),
        conclusions_not_in_pack: missing_conclusions.into_iter().take(8).collect(),
        unparsed_derivation_lines: unparsed,
    })
}

/// A store holding exactly the triples a pack's graph text carries.
///
/// Asked of a store of its own rather than of the caller's, because the
/// question is about the graph IN THE PACK and asking it of the caller's store
/// would answer a different question over whatever was already loaded.
fn stage(graph_text: &str) -> anyhow::Result<Arc<GraphStore>> {
    let staged = Arc::new(GraphStore::new());
    staged.load_ntriples(graph_text)?;
    Ok(staged)
}

/// What to pack. A struct rather than six positional arguments, because
/// `pack("a", "b", "c", None, None, None)` is a line nobody can read.
pub struct PackRequest<'a> {
    pub path: &'a str,
    pub name: &'a str,
    pub version: &'a str,
    pub evidence: Option<serde_json::Value>,
    /// A directory a previous `reason --certificate DIR` wrote. `None` packs no
    /// certificate, which is what every pack written before this field existed
    /// carries.
    ///
    /// The certificate is taken from a run that HAPPENED rather than produced
    /// here. Packing would otherwise have to choose the profile and the
    /// materialisation target itself, and the certificate would then describe a
    /// run nobody asked for.
    pub certificate_dir: Option<&'a Path>,
    pub profile_claimed: Option<&'a str>,
}

/// What the receiver is allowed to decide.
#[derive(Default)]
pub struct UnpackOptions {
    pub verify_only: bool,
    /// Off skips the checker and reports `checker_absent_nothing_was_checked`
    /// with `skipped_by_caller: true`. There is no option that makes an
    /// unchecked run print a checked word.
    pub check_certificate: bool,
    /// An explicit checker path, passed through to
    /// [`crate::projection_entailment::find_checker`]. Handed in rather than
    /// read from `$OO_CERT` here so that a test can point at a fake without
    /// mutating the environment of a parallel test.
    pub checker: Option<PathBuf>,
    /// Keep the certificate files here instead of in a scratch directory that
    /// is removed, so a receiver can re-run the checker by hand.
    pub certificate_out_dir: Option<PathBuf>,
    /// Load the graph even when the certificate did not check. Off by default:
    /// a graph whose own proof is refused is a defect, not something to
    /// promote. This changes behaviour only for packs that carry a
    /// certificate, so no pack written before this existed is affected.
    pub load_even_if_refused: bool,
}

pub struct Packer {
    graph: Arc<GraphStore>,
}

impl Packer {
    pub fn new(graph: Arc<GraphStore>) -> Self {
        Self { graph }
    }

    /// Write the loaded graph, its evidence and, when one is given, the
    /// certificate for the run that produced it.
    pub fn pack(&self, req: &PackRequest<'_>) -> anyhow::Result<String> {
        let nt = self.graph.serialize("ntriples")?;

        // Sorted, so the same graph always produces the same bytes: packs
        // become diffable and the checksum becomes meaningful across
        // machines rather than reflecting store iteration order.
        let mut lines: Vec<&str> = nt.lines().filter(|l| !l.trim().is_empty()).collect();
        lines.sort_unstable();
        let graph = lines.join("\n") + "\n";

        let certificate = match req.certificate_dir {
            None => None,
            Some(dir) => {
                let cert = read_certificate_dir(dir, req.profile_claimed)?;
                // Refused at pack time, not warned about. A certificate whose
                // assertions are not all in the graph being packed produces an
                // artefact whose proof is about premises the pack does not
                // contain, and a receiver's checker would accept it. Catching
                // it here costs one set difference and no Lean toolchain, so
                // the sender learns it rather than the auditor. The receiver
                // refuses it too: the receiver must not depend on the sender
                // having checked.
                //
                // Asked of a store staged from THE PACKED TEXT rather than of
                // the live store, so the sender computes the identical set the
                // receiver will compute. Asking the live store a slightly
                // different question is how a pack-time pass and an unpack-time
                // refusal end up in the same artefact.
                let kind = kind_of(&cert.files)?;
                let staged = stage(&graph)?;
                let c = coverage(&staged, &cert.files, kind)?;
                anyhow::ensure!(
                    c.in_certificate_only.is_empty(),
                    "this certificate rests on {} assertion(s) that are not in the graph being \
                     packed, for example {:?}. A checker would accept it, because oo-cert \
                     verifies the steps against the triples inside the certificate and cannot \
                     ask where they came from. Reason and pack the same graph",
                    c.in_certificate_only_total,
                    c.in_certificate_only.first()
                );
                Some(cert)
            }
        };

        let manifest = Manifest {
            name: req.name.to_string(),
            version: req.version.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            triples: lines.len(),
            sha256: checksum(&graph),
            evidence: req.evidence.clone(),
            content_sha256: Some(content_digest(&graph, certificate.as_ref())),
            certificate,
        };

        let pack = Pack { manifest, graph };
        // The whole pack is built in memory, certificate included, and there is
        // no streaming path. A large store's certificate makes a very large
        // pack, so the byte counts below are reported rather than left for a
        // receiver to discover when the file will not fit.
        let json = serde_json::to_string_pretty(&pack)?;
        std::fs::write(req.path, &json)?;

        Ok(serde_json::json!({
            "ok": true,
            "path": req.path,
            "name": pack.manifest.name,
            "version": pack.manifest.version,
            "triples": pack.manifest.triples,
            "sha256": pack.manifest.sha256,
            "content_sha256": pack.manifest.content_sha256,
            "content_sha256_covers": "the graph text and every field and file of the certificate, \
                                      each length-prefixed. sha256 above covers the graph alone \
                                      and cannot see a swapped certificate",
            "has_evidence": pack.manifest.evidence.is_some(),
            "certificate": pack.manifest.certificate.as_ref().map(|c| serde_json::json!({
                "format": c.format,
                "files": c.files.keys().collect::<Vec<_>>(),
                "asserted": c.asserted,
                "derivations": c.derivations,
                "bytes": c.files.values().map(|v| v.len()).sum::<usize>(),
                "profile_claimed_by_sender": c.profile_claimed_by_sender,
                "checked_by_this_packer": false,
                "means": "the certificate travels with the graph and the content digest covers \
                          both. NOTHING HERE HAS CHECKED IT: the receiver runs its own checker, \
                          which is the point of shipping it",
            })),
            "graph_bytes": pack.graph.len(),
            "pack_json_bytes": json.len(),
            "size_means": "the pack is assembled in memory with no streaming path, so \
                           pack_json_bytes is also roughly the peak cost of writing it and of \
                           reading it back",
        })
        .to_string())
    }

    /// Load a pack, refusing it if a digest does not match, and re-check any
    /// certificate it carries with THIS machine's checker.
    pub fn unpack(&self, path: &str, opts: &UnpackOptions) -> anyhow::Result<String> {
        let raw = std::fs::read_to_string(path)?;
        let pack: Pack = serde_json::from_str(&raw)?;

        let actual = checksum(&pack.graph);
        if actual != pack.manifest.sha256 {
            return Ok(serde_json::json!({
                "error": "checksum mismatch: the pack was modified or truncated after it was written",
                "expected": pack.manifest.sha256,
                "actual": actual,
                "verified": false,
                "loaded": false,
            })
            .to_string());
        }

        // The digest above covers the graph and nothing else. A pack that
        // carries a certificate and no digest over both bodies is REFUSED
        // rather than checked: dropping `content_sha256` and pasting in a
        // certificate that is sound over another graph leaves `sha256`
        // matching, and there would be nothing left to notice with.
        match (&pack.manifest.certificate, &pack.manifest.content_sha256) {
            (Some(_), None) => {
                return Ok(serde_json::json!({
                    "error": "this pack carries a certificate and no content_sha256 binding it to \
                              the graph. The graph checksum cannot see a swapped certificate, so \
                              an unbound one is refused rather than checked",
                    "verified": false,
                    "loaded": false,
                    "certificate": {
                        "present": true,
                        "verdict": PackCertVerdict::Refused,
                        "means": ABSENT_MEANS,
                    },
                })
                .to_string());
            }
            (cert, Some(recorded)) => {
                let recomputed = content_digest(&pack.graph, cert.as_ref());
                if *recorded != recomputed {
                    return Ok(serde_json::json!({
                        "error": "content checksum mismatch: the graph and the certificate in this \
                                  pack are not the pair that was written. One of them was replaced \
                                  afterwards",
                        "expected": recorded,
                        "actual": recomputed,
                        "verified": false,
                        "loaded": false,
                    })
                    .to_string());
                }
            }
            // An old pack: no certificate and no content digest. It loads, and
            // the report says it carries no proof rather than failing.
            (None, None) => {}
        }

        let (verdict, report) = match &pack.manifest.certificate {
            None => (
                PackCertVerdict::NoCertificateInPack,
                serde_json::json!({
                    "present": false,
                    "verdict": PackCertVerdict::NoCertificateInPack,
                    "theorem": serde_json::Value::Null,
                    "means": "this pack carries no certificate, so there is nothing here to \
                              re-derive. Its evidence is what the sender's engine said about the \
                              graph and this receiver has re-run none of it",
                }),
            ),
            Some(cert) => self.check_packed_certificate(&pack, cert, opts)?,
        };

        if opts.verify_only {
            return Ok(serde_json::json!({
                "ok": true,
                "verified": true,
                "loaded": false,
                "manifest": pack.manifest,
                "certificate": report,
            })
            .to_string());
        }

        if verdict.blocks_the_load() && !opts.load_even_if_refused {
            return Ok(serde_json::json!({
                "ok": false,
                "verified": true,
                "loaded": false,
                "why_not_loaded": "the certificate in this pack did not check on this machine, so \
                                   nothing was loaded. A graph whose own proof is refused is a \
                                   defect, not something to promote. Pass load_even_if_refused to \
                                   load it anyway and inspect it",
                "manifest": pack.manifest,
                "certificate": report,
            })
            .to_string());
        }

        let loaded = self.graph.load_ntriples(&pack.graph)?;
        Ok(serde_json::json!({
            "ok": true,
            "verified": true,
            "loaded": true,
            "triples_loaded": loaded,
            "manifest": pack.manifest,
            "certificate": report,
        })
        .to_string())
    }

    fn check_packed_certificate(
        &self,
        pack: &Pack,
        cert: &PackedCertificate,
        opts: &UnpackOptions,
    ) -> anyhow::Result<(PackCertVerdict, serde_json::Value)> {
        let base = |v: &PackCertVerdict, extra: serde_json::Value| {
            let mut o = serde_json::json!({
                "present": true,
                "verdict": v.clone(),
                "theorem": v.theorem(),
                "checked_here": v.is_checked(),
                "format_claimed_by_sender": cert.format,
                "profile_claimed_by_sender": cert.profile_claimed_by_sender,
                "asserted": cert.asserted,
                "derivations": cert.derivations,
                "certificate_bytes": cert.files.values().map(|v| v.len()).sum::<usize>(),
            });
            if let (Some(a), Some(b)) = (o.as_object_mut(), extra.as_object()) {
                for (k, val) in b {
                    a.insert(k.clone(), val.clone());
                }
            }
            o
        };

        let kind = match kind_of(&cert.files) {
            Ok(k) => k,
            Err(e) => {
                let v = PackCertVerdict::Unreadable;
                let r = base(
                    &v,
                    serde_json::json!({"why": e.to_string(), "means": ABSENT_MEANS}),
                );
                return Ok((v, r));
            }
        };

        if !opts.check_certificate {
            let v = PackCertVerdict::CheckerAbsent;
            let r = base(
                &v,
                serde_json::json!({
                    "skipped_by_caller": true,
                    "means": ABSENT_MEANS,
                    "why": "the caller asked for no check, so nothing was checked. This is the \
                            same word an absent checker earns, because the two produce the same \
                            amount of evidence",
                }),
            );
            return Ok((v, r));
        }

        // The checker reads files, so the pack's strings go to disk. The name
        // joined is the WHITELIST's own literal and never the sender's string,
        // so a crafted name cannot escape this directory even if the equality
        // test below were wrong.
        let dir = match &opts.certificate_out_dir {
            Some(d) => d.clone(),
            None => scratch_dir(),
        };
        // A read-only filesystem with no writable temp directory is a reason to
        // report that nothing was checked, not a reason to fail an unpack that
        // used to succeed.
        if let Err(e) = std::fs::create_dir_all(&dir) {
            let v = PackCertVerdict::Unreadable;
            let r = base(
                &v,
                serde_json::json!({
                    "why": format!(
                        "the certificate could not be written to {} ({e}), so no checker could \
                         read it", dir.display()
                    ),
                    "means": ABSENT_MEANS,
                }),
            );
            return Ok((v, r));
        }
        for name in cert.files.keys() {
            let Some(known) = CERTIFICATE_FILES.iter().find(|k| *k == name) else {
                let v = PackCertVerdict::Unreadable;
                let r = base(
                    &v,
                    serde_json::json!({
                        "why": format!(
                            "this pack names a certificate file this build does not write: \
                             {name:?}. It was not written to disk"
                        ),
                        "allowed": CERTIFICATE_FILES,
                        "means": ABSENT_MEANS,
                    }),
                );
                return Ok((v, r));
            };
            if let Err(e) = std::fs::write(dir.join(*known), &cert.files[*known]) {
                let v = PackCertVerdict::Unreadable;
                let r = base(
                    &v,
                    serde_json::json!({
                        "why": format!("{known} could not be written to {} ({e})", dir.display()),
                        "means": ABSENT_MEANS,
                    }),
                );
                return Ok((v, r));
            }
        }

        let asserted = dir.join("asserted.tsv");
        let derivations = dir.join(kind.derivations_file());
        let rules = (kind == CertKind::OoHorn).then(|| dir.join("rules.tsv"));
        let status = pe::run_checker(
            kind,
            opts.checker.as_deref(),
            &asserted,
            &derivations,
            rules.as_deref(),
        );

        // The question is about the graph IN THE PACK, and it is asked BEFORE
        // anything is loaded, so a pack that is about to be refused never
        // touches the receiver's graph.
        let staged = stage(&pack.graph)?;
        let cover = coverage(&staged, &cert.files, kind)?;

        let check_with = match &rules {
            Some(r) => format!(
                "{} check {} {} {}",
                kind.binary(),
                r.display(),
                asserted.display(),
                derivations.display()
            ),
            None => format!(
                "{} {} {}",
                kind.binary(),
                asserted.display(),
                derivations.display()
            ),
        };
        // A scratch directory is removed once the checker has read it, so the
        // command above names files that will not be there. Saying which is
        // cheap; printing a command a reader cannot run and letting them find
        // out is not.
        let kept = opts.certificate_out_dir.is_some();
        let checker_block = serde_json::json!({
            "status": status.word(),
            "check_with": check_with,
            "certificate_dir": dir.display().to_string(),
            "certificate_dir_kept": kept,
            "check_with_means": if kept {
                "the certificate files are still in certificate_dir, so this command repeats the \
                 check exactly"
            } else {
                "the certificate went to a scratch directory that is removed once the checker has \
                 read it, so this command names files that no longer exist. Unpack again with \
                 certificate_out_dir set to keep them and run it"
            },
            "checker_says_it_is": checker_self_block(&status),
            "checker_limit": CHECKER_LIMIT,
            "coverage": {
                "asserted_in_certificate": cover.asserted_in_certificate,
                "in_certificate_only": cover.in_certificate_only_total,
                "sample_in_certificate_only": cover.in_certificate_only,
                "conclusions": cover.conclusions,
                "conclusions_not_in_pack": cover.conclusions_not_in_pack_total,
                "sample_conclusions_not_in_pack": cover.conclusions_not_in_pack,
                "unparsed_derivation_lines": cover.unparsed_derivation_lines,
                "means": "in_certificate_only counts assertions the proof rests on that this pack \
                          does not contain, and a checker cannot see them. conclusions_not_in_pack \
                          counts triples the certificate derives that this pack does not carry, \
                          which is the normal shape when the sender reasoned without materialising",
            },
            "refutation_in_pack": cert.files.contains_key("refutation.tsv"),
            "refutation_means": "a refutation.tsv is carried when the run found a contradiction. \
                                 NOTHING in this crate runs oo-refute, so it was not checked here \
                                 and no verdict in this report is about it",
        });

        // The order of these arms is the policy. A checker that said no, or
        // could not read the files, outranks everything: those are defects. A
        // certificate that checks and is about another graph is next. Only then
        // can an acceptance be reported, and only while carrying the token the
        // run earned.
        let (verdict, extra) = match status {
            CheckerStatus::Rejected { ref stdout } => (
                PackCertVerdict::Refused,
                serde_json::json!({
                    "checker_said": stdout,
                    "means": "the checker named a step it would not accept. STOP. This pack's own \
                              proof does not check, which is a defect in the pack",
                }),
            ),
            CheckerStatus::Unreadable { ref stdout } => (
                PackCertVerdict::Unreadable,
                serde_json::json!({"checker_said": stdout, "means": ABSENT_MEANS}),
            ),
            CheckerStatus::Absent { ref what, install } => (
                PackCertVerdict::CheckerAbsent,
                serde_json::json!({"why": what, "install": install, "means": ABSENT_MEANS}),
            ),
            CheckerStatus::NotNeeded { ref what } => (
                PackCertVerdict::CheckerAbsent,
                serde_json::json!({"why": what, "install": LAKE_INSTALL, "means": ABSENT_MEANS}),
            ),
            CheckerStatus::Accepted(ref a) if cover.in_certificate_only_total > 0 => (
                PackCertVerdict::NotAboutThisPack,
                serde_json::json!({
                    "checker_said": a.stdout(),
                    "means": "the checker accepted this certificate and it is not about this pack. \
                              Its derivations follow from the assertions listed INSIDE it, and \
                              those are not all in the graph you were sent, so the proof is over \
                              premises nobody shipped",
                }),
            ),
            CheckerStatus::Accepted(ref a) => {
                let cert_token = a.certified();
                let weaker = matches!(kind, CertKind::OoHorn);
                let v = match kind {
                    CertKind::OoCert => PackCertVerdict::Accepted(cert_token),
                    CertKind::OoHorn => PackCertVerdict::AcceptedUnderSuppliedRules(cert_token),
                };
                (
                    v,
                    serde_json::json!({
                        "checker_said": a.stdout(),
                        "means": MEANS,
                        "does_not_mean": DOES_NOT_MEAN,
                        "and_only_under_these_rules": weaker.then_some(
                            "this certificate is over a rule table the SENDER supplied, carried in \
                             rules.tsv. Its conclusions hold in every model of the asserted \
                             triples that also satisfies that table, and a table saying every \
                             supplier is compliant produces steps that check green for ever. Read \
                             rules.tsv before reading the verdict"),
                    }),
                )
            }
        };

        let mut r = base(&verdict, checker_block);
        if let (Some(a), Some(b)) = (r.as_object_mut(), extra.as_object()) {
            for (k, val) in b {
                a.insert(k.clone(), val.clone());
            }
        }
        // Rewritten from the verdict AFTER the merge, so no `extra` arm can put
        // a theorem name beside a verdict that did not earn one.
        r["theorem"] = match verdict.theorem() {
            Some(t) => serde_json::json!(t),
            None => serde_json::Value::Null,
        };
        r["checked_here"] = serde_json::json!(verdict.is_checked());
        if opts.certificate_out_dir.is_none() {
            // Best effort, and it is fine if it fails: a leftover scratch
            // directory is untidy and a missing one would break the
            // check_with command this report just printed.
            let _ = std::fs::remove_dir_all(&dir);
        }
        Ok((verdict, r))
    }
}

/// The `checker` block the binary printed ABOUT ITSELF, read out of its stdout.
///
/// Never constructed here. A Rust literal naming which binary ran would be
/// exactly the string a reader cannot check. `None` when the process printed no
/// such block, which is the honest answer for an old checker and for anything
/// that is not a checker at all.
fn checker_self_block(status: &CheckerStatus) -> Option<serde_json::Value> {
    let text = match status {
        CheckerStatus::Accepted(a) => a.stdout(),
        CheckerStatus::Rejected { stdout } | CheckerStatus::Unreadable { stdout } => stdout,
        CheckerStatus::Absent { .. } | CheckerStatus::NotNeeded { .. } => return None,
    };
    let line = text.lines().find(|l| l.starts_with('{'))?;
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    v.get("checker").cloned()
}

/// A directory nothing else is writing into.
///
/// The pid alone is not enough: `cargo test` runs these as threads of ONE
/// process, so two unpacks in one binary would collide on it and each would see
/// the other's files.
fn scratch_dir() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "oo-unpack-{}-{nanos}-{serial}",
        std::process::id()
    ))
}
