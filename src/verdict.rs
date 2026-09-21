//! Certified verdicts that cannot be spelled without the evidence.
//!
//! Every certified word this engine prints — `model_checked`, the OWL-level
//! `not_entailed_under_unproved_translation`, `preserved_checked`,
//! `preserved_under_supplied_rules_checked`, `checked`, and the theorem names
//! `OOCert.certificate_sound` / `OOCert.horn_certificate_sound` /
//! `Fol.satisfiable_of_check` — is now a variant that CARRIES a [`Certified`],
//! and [`Certified`] has no public constructor. The only value of that type in
//! the whole crate comes out of [`CheckerRun::accepted`], and the only way to
//! obtain a [`CheckerRun`] is [`CheckerRun::spawn`], which runs the checker
//! itself and reads its exit status.
//!
//! # What this replaces
//!
//! Until this module existed the discipline was carried by comments and by
//! tests. Three files each had a line saying "the ONE place this word is
//! produced", and a suite that failed if the word appeared with no exit code
//! behind it. Those tests were right and they still run. But a test only
//! covers the paths someone pointed it at: a new report type that formatted
//! `"model_checked"` in a new module would pass every one of them until
//! somebody noticed and wrote a guard.
//!
//! Now it is a privacy error. `FolVerdict::ModelChecked(..)` cannot be named
//! outside this module without a `Certified`, `Certified` cannot be built
//! outside this module at all, and this module hands one out only after
//! `std::process::Command::output` returned and `ExitStatus::code()` was zero.
//! The compile-fail doctests below are the executable form of that claim; they
//! run under `cargo test` like any other test.
//!
//! # What it does NOT claim
//!
//! Three residual holes, stated rather than papered over.
//!
//! 1. **Which binary ran.** [`CheckerBinary::found_at`] is public and takes
//!    any path, so the wrapper documents where a path is supposed to come from
//!    and does not enforce it; and the discovery routines that call it honour
//!    `$OO_CERT` / `$OO_HORN` / `$OO_FOLMODEL`, so an operator who points one
//!    of those at a script that exits zero gets a certified word for free. The
//!    type system cannot tell a proved-sound Lean binary from a shell builtin.
//!    What it does guarantee is that SOMETHING was executed and exited zero,
//!    which is strictly more than a string literal guarantees, and it is the
//!    property the tests were checking by hand.
//!
//!    Narrowed, not closed, by #204. Every checker now prints a `checker`
//!    block naming itself, the Lean toolchain that built it, and the SHA-256
//!    of the file it is running from, and
//!    [`CheckerRun::checker_block`] reads it. A reader matches that digest
//!    against the `SHASUMS.txt` a release publishes. What that buys is a way
//!    for an HONEST build to say which build it is; a hostile binary can print
//!    the same block with any numbers in it, and nothing here would notice.
//! 2. **Which artefact was accepted.** Mostly closed, and what remains is
//!    named rather than rounded off. A [`Certified`] now carries the digest of
//!    the inputs the accepting run was handed, computed by
//!    [`CheckerRun::spawn`] from those files before it spawned, and
//!    [`Certified::is_about`] lets a holder of an artefact ask whether the
//!    token is about the thing in their hand. The token is no longer `Copy`,
//!    so a value earned over goal A cannot silently reappear on goal B.
//!
//!    What is left: it is still `Clone`, so duplication is possible, and the
//!    difference is only that `.clone()` is visible where an implicit copy was
//!    not. And the digest is over the files the CALLER named as the run's
//!    inputs; a caller that names the wrong files gets a token bound to the
//!    wrong thing. That is a lie a reader can see in the argument list next to
//!    the command, which is where it should be, and it is not a lie the type
//!    system prevents.
//! 3. **Deserialisation.** None of these verdict types implements
//!    `Deserialize`, deliberately. Reading `"model_checked"` out of somebody
//!    else's JSON is not the same act as earning it, and a `Deserialize` impl
//!    would be a public constructor for every certified variant. Reports are
//!    read back as `serde_json::Value`.
//!
//! # The words that belong to a Lean checker
//!
//! [`CHECKER_OWNED_WORDS`] lists the verdicts the Lean binaries print for
//! themselves. No `word()` in this module returns one, and
//! [`no_engine_word_is_a_checker_word`] asserts that over every vocabulary
//! defined here. Rust states its own words; it quotes the checker's only by
//! echoing bytes it read from the checker's stdout.

use std::path::{Path, PathBuf};
use std::process::Command;

// ───────────────────────────────────────────────────────────────────────────
// The evidence
// ───────────────────────────────────────────────────────────────────────────

/// A path this process resolved to a checker binary.
///
/// The field is private, so a caller cannot conjure one from a `PathBuf`: it
/// has to come through [`CheckerBinary::found_at`], which is what every
/// `find_checker` in the crate calls once it has a path that exists. This is a
/// thin wrapper and it earns its place by making [`CheckerRun::spawn`]'s
/// signature say "this ran a checker" rather than "this ran a command".
#[derive(Clone, Debug)]
pub struct CheckerBinary(PathBuf);

impl CheckerBinary {
    /// Wrap a path a discovery routine has already resolved.
    pub fn found_at(path: PathBuf) -> CheckerBinary {
        CheckerBinary(path)
    }
    pub fn path(&self) -> &Path {
        &self.0
    }
}

/// A checker that RAN, with the exit status this process read off it.
///
/// Every field is private and there is exactly one constructor, and that
/// constructor spawns a process. No module can hand this type a number.
#[derive(Clone, Debug)]
pub struct CheckerRun {
    binary: PathBuf,
    subject: [u8; 32],
    code: Option<i32>,
    exit: i32,
    stdout: String,
    stderr: String,
}

/// The digest of a set of files, in the framing [`CheckerRun::spawn`] uses.
///
/// Public because the comparison is the point: a holder of an artefact asks
/// whether a [`Certified`] is about THAT artefact, and both sides of that
/// question have to be computed the same way. This is the one function that
/// computes it, so the two sides cannot drift.
///
/// Each file contributes its base name, its length and its bytes, each
/// length-prefixed, so no two different sets of inputs can frame to the same
/// bytes by running together at the seam. The base name is included because
/// `oo-cert A D` handed `derivations.tsv` as the asserted file and
/// `asserted.tsv` as the derivations file is a different run from the right
/// one, and the digest should say so. The full path is NOT included: it
/// differs between machines, and a digest that changes when a directory moves
/// is a digest nobody can compare.
///
/// An unreadable input is an error and never a digest. A run whose inputs
/// could not be read cannot mint a token bound to them.
pub fn subject_digest(inputs: &[&Path]) -> std::io::Result<[u8; 32]> {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(b"oo-checker-subject/1\n");
    h.update((inputs.len() as u64).to_le_bytes());
    for p in inputs {
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        h.update((name.len() as u64).to_le_bytes());
        h.update(name.as_bytes());
        let bytes = std::fs::read(p)?;
        h.update((bytes.len() as u64).to_le_bytes());
        h.update(&bytes);
    }
    Ok(h.finalize().into())
}

/// A digest as the lower-case hex a report prints.
pub fn hex32(d: &[u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

impl CheckerRun {
    /// Run `cmd` and capture what it said. THE ONLY CONSTRUCTOR.
    ///
    /// `cmd` is the caller's, because the argument vector differs per checker
    /// (`oo-cert A D`, `oo-horn check R A D`, `oo-folmodel P M`), but the exit
    /// code is read here and nowhere else. An error from the spawn itself is
    /// returned as an error and never as a run: a checker that could not be
    /// started did not accept anything.
    pub fn spawn(
        binary: &CheckerBinary,
        mut cmd: Command,
        inputs: &[&Path],
    ) -> std::io::Result<CheckerRun> {
        // BEFORE the spawn, so the digest is of what the checker is about to
        // read rather than of whatever is on disk once it has finished. A
        // checker that writes into its own input directory would otherwise be
        // hashed after the fact.
        if inputs.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "a checker run must name the files it is about. A token bound to no inputs                  is bound to nothing, and every such token would be interchangeable with                  every other, which is the drift this binding exists to stop",
            ));
        }
        let subject = subject_digest(inputs)?;
        let out = cmd.output()?;
        Ok(CheckerRun {
            binary: binary.path().to_path_buf(),
            subject,
            // Kept as the `Option` the OS gave, because `None` (killed by a
            // signal) and `Some(n)` are different facts and one report prints
            // the difference.
            code: out.status.code(),
            // `-1` for a signal death, which is not an acceptance under any
            // reading and must not collide with 0.
            exit: out.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }

    pub fn binary(&self) -> &Path {
        &self.binary
    }
    /// The exit code, reported verbatim so a caller can distinguish `lean/`'s
    /// reserved 1 (rejected) from 2 (unreadable). A process killed by a signal
    /// reads as `-1`, which no exit status can be.
    pub fn exit(&self) -> i32 {
        self.exit
    }
    /// The raw `ExitStatus::code()`, for the one diagnostic that prints
    /// `None` and `Some(n)` differently.
    pub fn code(&self) -> Option<i32> {
        self.code
    }
    /// The checker's own stdout. Its JSON report, where it writes one.
    pub fn stdout(&self) -> &str {
        &self.stdout
    }
    /// stdout then stderr, the spelling `run_checker` has always reported.
    ///
    /// These are the only bytes in a report that may carry a CHECKER's verdict
    /// word, because they came from the checker.
    pub fn output(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }

    /// The theorem this run named in its own stdout, if it named one.
    ///
    /// Every `lean/` entry point that mints a verdict prints a `theorem` field
    /// saying which statement its acceptance discharges. Reading it here is
    /// what lets a report quote the checker rather than describe it.
    pub fn named_theorem(&self) -> Option<String> {
        let v: serde_json::Value = serde_json::from_str(&self.stdout).ok()?;
        Some(v.get("theorem")?.as_str()?.to_string())
    }

    /// The `checker` block this run printed about ITSELF, if it printed one.
    ///
    /// Read out of the checker's stdout and never constructed here, for the
    /// reason [`named_theorem`](CheckerRun::named_theorem) is read rather than
    /// asserted: a Rust literal describing which binary ran would be exactly
    /// the string a reader cannot check.
    ///
    /// `None` when the run printed no such block, which is the honest answer
    /// for a checker older than #204 and for anything that is not a checker at
    /// all. A report that shows nothing where this is `None` is telling the
    /// truth; a report that filled it in would not be.
    ///
    /// What it does NOT establish: that the process was the binary it names.
    /// A hostile executable can print any block it likes, and residual hole 1
    /// in this module's header is narrowed by this rather than closed. What an
    /// HONEST build gets is a way to say which build it is, matchable by
    /// someone who does not trust it against the `SHASUMS.txt` a release
    /// publishes.
    pub fn checker_block(&self) -> Option<serde_json::Value> {
        let v: serde_json::Value = serde_json::from_str(&self.stdout).ok()?;
        v.get("checker").cloned()
    }

    /// The one function in this crate that returns a [`Certified`].
    ///
    /// `Some` exactly when the checker exited zero AND its own stdout named one
    /// of `allowed` as the theorem it discharged. The returned token carries
    /// the name the CHECKER printed, not the caller's guess at it, so no report
    /// can name a theorem without an acceptance that named the same one.
    ///
    /// The list is a list because one checker legitimately proves one of two
    /// statements depending on its input: `lean/HMain.lean` earns the absolute
    /// `OOCert.entails_of_builtin_horn` when the rule table is exactly the
    /// built-ins and the relativised `OOCert.horn_certificate_sound` for any
    /// other table. Passing a single literal made the Rust side report the
    /// relativised name for both, so a caller could not tell an entailment from
    /// an entailment-under-supplied-rules by reading the field that exists to
    /// tell them.
    ///
    /// A checker that exits zero while naming a statement NOT on the list mints
    /// nothing. That is the case worth refusing: a renamed or weakened theorem
    /// would otherwise keep the label this call site was written against.
    pub fn accepted_naming(&self, allowed: &[&'static str]) -> Option<Certified> {
        if self.exit != 0 {
            return None;
        }
        let named = self.named_theorem()?;
        allowed
            .iter()
            .copied()
            .find(|t| *t == named.as_str())
            .map(|theorem| Certified { theorem, subject: self.subject })
    }
}

/// Evidence that a verified checker accepted something, and the name of the
/// theorem its acceptance discharges.
///
/// The field is private to this module. A value of this type exists only
/// because [`CheckerRun::accepted_naming`] read a zero exit code and a matching
/// theorem name, so a variant that
/// carries one is unreachable on any path that did not run a checker.
///
/// ```compile_fail
/// // A struct literal: the fields are private.
/// let c = open_ontologies::verdict::Certified {
///     theorem: "OOCert.certificate_sound",
///     subject: [0u8; 32],
/// };
/// ```
///
/// ```compile_fail
/// // And there is no constructor function to reach for either.
/// let c = open_ontologies::verdict::Certified::new("OOCert.certificate_sound");
/// ```
///
/// ```compile_fail
/// // NOT `Copy`, so a token earned over goal A cannot silently reappear on
/// // goal B. Restoring the derive would make this compile, and a
/// // `compile_fail` doctest that compiles is a failure.
/// fn assert_copy<T: Copy>() {}
/// assert_copy::<open_ontologies::verdict::Certified>();
/// ```
/// # Which artefact, and what `Clone` still allows
///
/// The token carries the digest of the inputs the accepting run was handed,
/// computed by [`CheckerRun::spawn`] before it spawned. So a token proves
/// acceptance OF SOMETHING NAMED rather than acceptance in the abstract, and
/// [`Certified::is_about`] lets a holder of an artefact ask whether this token
/// is about the artefact in their hand.
///
/// It is deliberately NOT `Copy`. While it was, a token earned over goal A
/// could be attached to goal B by any use of the value, silently, with nothing
/// in the source to see. It is still `Clone`, so a determined caller can still
/// duplicate one — but `.clone()` is a visible act that a reader and a grep
/// can both find, which an implicit copy was not. That is the whole of the
/// difference and it is not more than that.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Certified {
    theorem: &'static str,
    subject: [u8; 32],
}

impl Certified {
    /// The machine-checked statement behind the word.
    pub fn theorem(&self) -> &'static str {
        self.theorem
    }

    /// The digest of what the accepting run was handed, as hex.
    pub fn subject_sha256(&self) -> String {
        hex32(&self.subject)
    }

    /// Is this token about the artefact the caller is holding?
    ///
    /// The caller computes their side with [`subject_digest`] over the same
    /// files in the same order. A `false` here means the token was earned over
    /// something else, which is exactly the drift this type could not detect
    /// before.
    pub fn is_about(&self, digest: &[u8; 32]) -> bool {
        &self.subject == digest
    }
}

// ───────────────────────────────────────────────────────────────────────────
// The first-order model pipeline (decision 0006)
// ───────────────────────────────────────────────────────────────────────────

/// The five words `onto_fol_model` may print, and no sixth.
///
/// `ModelChecked` is the only certified one. It carries the evidence, so the
/// line that used to be a one-word assignment is now a compile error unless a
/// checker ran:
///
/// ```compile_fail
/// use open_ontologies::verdict::{Certified, FolVerdict};
/// // The laundering line, as it would have to be written today.
/// let v = FolVerdict::ModelChecked(Certified { theorem: "Fol.satisfiable_of_check", subject: [0u8; 32] });
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FolVerdict {
    /// CERTIFIED. `oo-folmodel` accepted the structure. Requires exit 0.
    ModelChecked(Certified),
    /// A solver said `sat` and nothing checked it.
    SatisfiableOracle,
    /// A BOUNDED search was exhausted. Not unsatisfiability.
    NoModelUpToSizeK,
    /// A solver said `unsat` on the UNBOUNDED encoding. An opinion for ever.
    UnsatisfiableOracle,
    /// Timeout or give-up.
    UnknownOracle,
}

impl FolVerdict {
    /// The wire word. Byte-identical to the string literals this replaced.
    pub fn word(&self) -> &'static str {
        match self {
            FolVerdict::ModelChecked(_) => "model_checked",
            FolVerdict::SatisfiableOracle => "satisfiable_oracle",
            FolVerdict::NoModelUpToSizeK => "no_model_up_to_size_k",
            FolVerdict::UnsatisfiableOracle => "unsatisfiable_oracle",
            FolVerdict::UnknownOracle => "unknown_oracle",
        }
    }
    /// The theorem, named only where one stands behind the word.
    pub fn theorem(&self) -> Option<&'static str> {
        match self {
            FolVerdict::ModelChecked(c) => Some(c.theorem()),
            _ => None,
        }
    }
    pub fn is_certified(self) -> bool {
        self.theorem().is_some()
    }
    /// Every word, for a suite that wants to enumerate the vocabulary without
    /// being able to build the certified variant.
    pub const WORDS: [&'static str; 5] = [
        "model_checked",
        "satisfiable_oracle",
        "no_model_up_to_size_k",
        "unsatisfiable_oracle",
        "unknown_oracle",
    ];
}

/// The OWL-level reading, which is a SECOND certified word on the same
/// boundary and was a second place to launder one.
///
/// Decision 0006 item 4: non-null only when the CHECKER reported
/// `goal_negated_present` AND the verdict is `model_checked`. Both conditions
/// are now arguments that cannot be faked — the report is read off
/// [`CheckerRun::output`], and the `Certified` is the verdict's own evidence.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OwlReading(Certified);

impl OwlReading {
    /// The word, never shortened to "not entailed": it rides on
    /// `OwlLean.adequacy` in a sibling project AND on a Rust-to-Lean
    /// correspondence that decision 0005 item 2 says is pinned by tests and
    /// NOT proved.
    pub fn word(&self) -> &'static str {
        "not_entailed_under_unproved_translation"
    }
}

impl CheckerRun {
    /// Mint the OWL-level reading, if this run earned it.
    ///
    /// Takes the verdict's own [`Certified`] so the "and the verdict is
    /// `model_checked`" half of the rule is carried by the signature rather
    /// than by a comment, and reads `goal_negated_present` back out of the
    /// checker's report rather than out of this side's intention.
    pub fn owl_reading(&self, certified: Certified) -> Option<OwlReading> {
        self.stdout
            .contains("\"goal_negated_present\":true")
            .then_some(OwlReading(certified))
    }
}

// ───────────────────────────────────────────────────────────────────────────
// The closure-certificate pipeline (decision 0002)
// ───────────────────────────────────────────────────────────────────────────

/// Why a run is entitled to the conclusions in its closure. NEVER COLLAPSED.
///
/// ```compile_fail
/// use open_ontologies::verdict::{Certified, ClosureVerdict};
/// let v = ClosureVerdict::Checked(Certified { theorem: "OOCert.certificate_sound", subject: [0u8; 32] });
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClosureVerdict {
    /// CERTIFIED. `oo-cert` or `oo-horn` accepted the whole certificate.
    Checked(Certified),
    /// The checker rejected it. A defect report, never a weaker result.
    Rejected,
    /// The checker was absent, unreadable-in, or not needed. The engine's own
    /// opinion about its own output.
    EngineOpinion,
}

impl ClosureVerdict {
    pub fn word(&self) -> &'static str {
        match self {
            ClosureVerdict::Checked(_) => "checked",
            ClosureVerdict::Rejected => "rejected",
            ClosureVerdict::EngineOpinion => "engine_opinion",
        }
    }
    pub fn theorem(&self) -> Option<&'static str> {
        match self {
            ClosureVerdict::Checked(c) => Some(c.theorem()),
            _ => None,
        }
    }
    pub fn is_checked(self) -> bool {
        matches!(self, ClosureVerdict::Checked(_))
    }
}

// ───────────────────────────────────────────────────────────────────────────
// The engine's own words
// ───────────────────────────────────────────────────────────────────────────

/// What THIS ENGINE may say about a contradiction it found in its own closure.
///
/// There is deliberately no certified variant. Nothing in the Rust tree runs
/// `oo-refute`, so there is no code path that could mint the evidence one
/// would need, and adding an unreachable variant would put the checker's word
/// into the binary for the first time. When a Rust caller for `oo-refute` is
/// written, it mints a [`Certified`] through [`CheckerRun::accepted`] like
/// every other checker and a variant is added here — not before.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EngineRefutation {
    /// This engine found a clash in the closure it computed and NOTHING has
    /// checked that. Deliberately not the checker's word.
    ClashFoundByThisEngine,
    /// A `refutation.tsv` was written in the format `oo-refute` reads. That it
    /// was WRITTEN is all this says: exit 0 from `oo-refute check` is the
    /// result that means anything.
    RefutationWrittenNotYetChecked,
}

impl EngineRefutation {
    pub fn word(self) -> &'static str {
        match self {
            EngineRefutation::ClashFoundByThisEngine => "clash_found_by_this_engine",
            EngineRefutation::RefutationWrittenNotYetChecked => {
                "refutation_written_not_yet_checked"
            }
        }
    }
}

/// The verdicts the LEAN binaries print for themselves.
///
/// Rust may put one of these in a report only by echoing
/// [`CheckerRun::output`] verbatim, or by naming a word a rule table is
/// ELIGIBLE FOR under a key that says so — `src/rulesyntax.rs` writes
/// `verdict_this_table_is_eligible_for`, which is a statement about what
/// `oo-horn` could conclude and never a verdict this side is pronouncing.
/// Nothing in this module returns one of these from a `word()`, and
/// [`no_engine_word_is_a_checker_word`] fails if that ever changes.
pub const CHECKER_OWNED_WORDS: &[&str] = &[
    // `oo-cert` / `oo-horn`, lean/Main.lean.
    "entailed",
    "entailed_under_supplied_rules",
    // `oo-refute`, lean/OOCert/Refute.lean.
    "unsatisfiable_under_disjointness",
    // `oo-resolution`, lean/FoMain.lean, and `oo-lrat`, lean/LratMain.lean. Both
    // print this for an accepted refutation. The Rust side says
    // `refutation_certified` and `refuted`, never this.
    "unsatisfiable",
];

/// The word a SUPPLIED Horn table can earn from `oo-horn`, for the one place
/// that is allowed to name it: an eligibility statement, never a verdict.
///
/// Held here so the string exists once in the crate and a reader who greps for
/// it lands on this paragraph.
pub const ELIGIBLE_UNDER_SUPPLIED_RULES: &str = "entailed_under_supplied_rules";

// ───────────────────────────────────────────────────────────────────────────
// Serialisation: the wire word, and nothing but the wire word
// ───────────────────────────────────────────────────────────────────────────

macro_rules! serialize_as_word {
    ($t:ty) => {
        impl serde::Serialize for $t {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(self.word())
            }
        }
        impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.word())
            }
        }
        // So the suite can keep asserting against the wire word without
        // being able to build the certified variant to compare with.
        impl PartialEq<str> for $t {
            fn eq(&self, other: &str) -> bool {
                self.word() == other
            }
        }
        impl PartialEq<&str> for $t {
            fn eq(&self, other: &&str) -> bool {
                self.word() == *other
            }
        }
    };
}

serialize_as_word!(FolVerdict);
serialize_as_word!(OwlReading);
serialize_as_word!(ClosureVerdict);
serialize_as_word!(EngineRefutation);

#[cfg(test)]
mod tests {
    use super::*;

    /// The structural half of "the Rust never reprints a Lean verdict".
    ///
    /// Every verdict string the crate prints is now a `word()` of one of these
    /// vocabularies. None of them may be a word a Lean binary prints for
    /// itself, or an engine opinion and a machine-checked result share a
    /// string and a consumer cannot tell them apart.
    #[test]
    fn no_engine_word_is_a_checker_word() {
        let mut ours: Vec<&str> = Vec::new();
        ours.extend(FolVerdict::WORDS);
        ours.push(OwlReading(Certified { theorem: "x", subject: [0u8; 32] }).word());
        ours.push(ClosureVerdict::Rejected.word());
        ours.push(ClosureVerdict::EngineOpinion.word());
        ours.push(ClosureVerdict::Checked(Certified { theorem: "x", subject: [0u8; 32] }).word());
        ours.push(EngineRefutation::ClashFoundByThisEngine.word());
        ours.push(EngineRefutation::RefutationWrittenNotYetChecked.word());
        for w in ours {
            assert!(
                !CHECKER_OWNED_WORDS.contains(&w),
                "the engine states {w:?}, which is a word a Lean checker prints for itself"
            );
        }
    }

    /// A shell invoked with an inline command, rather than a script this test
    /// writes and then executes. Writing one costs both portability and
    /// reliability: a `.sh` is not executable on Windows, and on Linux a thread
    /// that forks while another holds a write fd open makes the child inherit
    /// it, so the exec fails with ETXTBSY no matter how unique the filename is.
    /// Both were observed in CI. `/bin/sh` and `cmd` are already there.
    fn shell_exiting(code: i32) -> (CheckerBinary, Command) {
        shell_saying(code, Some("OOCert.certificate_sound"))
    }

    /// A shell that prints a checker's JSON line and then exits.
    ///
    /// The fake used to exit in silence, which no real checker does, and which
    /// meant the mint could not be tested against the thing it now reads: the
    /// theorem the binary NAMED. `None` is the checker that prints nothing.
    fn shell_saying(code: i32, theorem: Option<&str>) -> (CheckerBinary, Command) {
        #[cfg(unix)]
        {
            let say = match theorem {
                Some(t) => format!("echo '{{\"ok\":true,\"theorem\":\"{t}\"}}'; "),
                None => String::new(),
            };
            let mut c = Command::new("/bin/sh");
            c.arg("-c").arg(format!("{say}exit {code}"));
            (CheckerBinary::found_at(PathBuf::from("/bin/sh")), c)
        }
        #[cfg(windows)]
        {
            // `arg` quotes for a C-runtime parser, turning every `"` into `\"`.
            // cmd.exe has no such parser: it echoed the backslashes and the
            // line was not JSON, so `named_theorem` saw nothing and all three
            // minting tests failed on windows-latest. `raw_arg` hands cmd the
            // line verbatim. The `&` follows the brace with no space so the
            // echoed line carries no trailing blank either.
            use std::os::windows::process::CommandExt;
            let say = match theorem {
                Some(t) => format!("echo {{\"ok\":true,\"theorem\":\"{t}\"}}& "),
                None => String::new(),
            };
            let mut c = Command::new("cmd");
            c.raw_arg(format!("/C {say}exit {code}"));
            (CheckerBinary::found_at(PathBuf::from("cmd")), c)
        }
    }

    /// A file for the shell runs to be ABOUT. They test exit codes and theorem
    /// names, not content, but a run must name its inputs, so they name this.
    fn a_file(tag: &str, body: &str) -> PathBuf {
        let p = std::env::temp_dir()
            .join(format!("oo-verdict-{tag}-{}.txt", std::process::id()));
        std::fs::write(&p, body).expect("write the scratch input");
        p
    }

    /// A non-zero exit mints nothing, and there is no other route to a token.
    #[test]
    fn a_non_zero_exit_yields_no_certificate() {
        let (bin, c) = shell_exiting(1);
        let run = CheckerRun::spawn(&bin, c, &[&a_file("shell", "input")]).expect("the system shell runs");
        assert_eq!(run.exit(), 1);
        assert!(run.accepted_naming(&["OOCert.certificate_sound"]).is_none());
    }

    #[test]
    fn a_zero_exit_mints_the_theorem_the_checker_named() {
        let (bin, c) = shell_exiting(0);
        let run = CheckerRun::spawn(&bin, c, &[&a_file("shell", "input")]).expect("the system shell runs");
        let cert = run.accepted_naming(&["OOCert.certificate_sound"]).expect("exit 0");
        assert_eq!(cert.theorem(), "OOCert.certificate_sound");
        assert_eq!(FolVerdict::ModelChecked(cert.clone()).word(), "model_checked");
        assert_eq!(FolVerdict::ModelChecked(cert).theorem(), Some("OOCert.certificate_sound"));
    }

    /// The case the old signature could not see. A checker that exits zero
    /// while discharging a DIFFERENT statement used to mint a token labelled
    /// with whatever the Rust call site had typed, which is the one thing the
    /// field is supposed to rule out.
    #[test]
    fn a_zero_exit_naming_another_theorem_mints_nothing() {
        let (bin, c) = shell_saying(0, Some("OOCert.something_weaker"));
        let run = CheckerRun::spawn(&bin, c, &[&a_file("shell", "input")]).expect("the system shell runs");
        assert_eq!(run.exit(), 0);
        assert_eq!(run.named_theorem().as_deref(), Some("OOCert.something_weaker"));
        assert!(run.accepted_naming(&["OOCert.certificate_sound"]).is_none());
    }

    /// And a checker that names nothing at all mints nothing, rather than
    /// having a name supplied for it.
    #[test]
    fn a_zero_exit_naming_no_theorem_mints_nothing() {
        let (bin, c) = shell_saying(0, None);
        let run = CheckerRun::spawn(&bin, c, &[&a_file("shell", "input")]).expect("the system shell runs");
        assert_eq!(run.exit(), 0);
        assert_eq!(run.named_theorem(), None);
        assert!(run.accepted_naming(&["OOCert.certificate_sound"]).is_none());
    }

    /// Two statements, and the token carries the one that was printed.
    #[test]
    fn a_checker_with_two_theorems_reports_the_one_it_proved() {
        let allowed = ["OOCert.horn_certificate_sound", "OOCert.entails_of_builtin_horn"];
        for named in allowed {
            let (bin, c) = shell_saying(0, Some(named));
            let run = CheckerRun::spawn(&bin, c, &[&a_file("shell", "input")]).expect("the system shell runs");
            let cert = run.accepted_naming(&allowed).expect("exit 0 naming an allowed theorem");
            assert_eq!(
                cert.theorem(),
                named,
                "the token must carry what the checker printed, not the first entry in the list"
            );
        }
    }

    /// A checker that could not be started is an error, not a run, so it can
    /// never be asked whether it accepted anything.
    #[test]
    fn a_checker_that_cannot_start_is_not_a_run() {
        let bin = CheckerBinary::found_at(PathBuf::from("/nonexistent/oo-cert"));
        let c = Command::new("/nonexistent/oo-cert");
        assert!(CheckerRun::spawn(&bin, c, &[&a_file("shell", "input")]).is_err());
    }

    /// The wire words, pinned as literals. A rename that changed one of these
    /// would change an MCP response and a CLI JSON field.
    #[test]
    fn the_wire_words_are_unchanged() {
        assert_eq!(serde_json::to_string(&FolVerdict::SatisfiableOracle).unwrap(), "\"satisfiable_oracle\"");
        assert_eq!(serde_json::to_string(&FolVerdict::NoModelUpToSizeK).unwrap(), "\"no_model_up_to_size_k\"");
        assert_eq!(serde_json::to_string(&FolVerdict::UnsatisfiableOracle).unwrap(), "\"unsatisfiable_oracle\"");
        assert_eq!(serde_json::to_string(&FolVerdict::UnknownOracle).unwrap(), "\"unknown_oracle\"");
        assert_eq!(serde_json::to_string(&ClosureVerdict::Rejected).unwrap(), "\"rejected\"");
        assert_eq!(serde_json::to_string(&ClosureVerdict::EngineOpinion).unwrap(), "\"engine_opinion\"");
        assert_eq!(
            serde_json::to_string(&EngineRefutation::ClashFoundByThisEngine).unwrap(),
            "\"clash_found_by_this_engine\""
        );
        let cert = Certified { theorem: "Fol.satisfiable_of_check", subject: [0u8; 32] };
        // `.clone()` three times where a `Copy` token needed none. That is the
        // change this issue asked for, visible at its first call site.
        assert_eq!(
            serde_json::to_string(&FolVerdict::ModelChecked(cert.clone())).unwrap(),
            "\"model_checked\""
        );
        assert_eq!(
            serde_json::to_string(&ClosureVerdict::Checked(cert.clone())).unwrap(),
            "\"checked\""
        );
        assert_eq!(
            serde_json::to_string(&OwlReading(cert)).unwrap(),
            "\"not_entailed_under_unproved_translation\""
        );
    }
}
