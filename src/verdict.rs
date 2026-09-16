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
//! 2. **Which artefact was accepted.** A [`Certified`] is `Copy`. Code that
//!    ran the checker over goal A could attach the token to goal B. Callers
//!    mint one token per run, inside the arm that owns that run's output, and
//!    the token carries the theorem name so at least the WARRANT cannot drift
//!    from the run that earned it. Binding the token to an artefact digest is
//!    the obvious next tightening and is not done here.
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
    code: Option<i32>,
    exit: i32,
    stdout: String,
    stderr: String,
}

impl CheckerRun {
    /// Run `cmd` and capture what it said. THE ONLY CONSTRUCTOR.
    ///
    /// `cmd` is the caller's, because the argument vector differs per checker
    /// (`oo-cert A D`, `oo-horn check R A D`, `oo-folmodel P M`), but the exit
    /// code is read here and nowhere else. An error from the spawn itself is
    /// returned as an error and never as a run: a checker that could not be
    /// started did not accept anything.
    pub fn spawn(binary: &CheckerBinary, mut cmd: Command) -> std::io::Result<CheckerRun> {
        let out = cmd.output()?;
        Ok(CheckerRun {
            binary: binary.path().to_path_buf(),
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

    /// The one function in this crate that returns a [`Certified`].
    ///
    /// `Some` exactly when the checker exited zero. `theorem` is the Lean
    /// statement that binary's acceptance discharges, and it travels inside
    /// the token so that no report can name a theorem without an acceptance.
    pub fn accepted(&self, theorem: &'static str) -> Option<Certified> {
        (self.exit == 0).then_some(Certified { theorem })
    }
}

/// Evidence that a verified checker accepted something, and the name of the
/// theorem its acceptance discharges.
///
/// The field is private to this module. A value of this type exists only
/// because [`CheckerRun::accepted`] read a zero exit code, so a variant that
/// carries one is unreachable on any path that did not run a checker.
///
/// ```compile_fail
/// // A struct literal: the field is private.
/// let c = open_ontologies::verdict::Certified { theorem: "OOCert.certificate_sound" };
/// ```
///
/// ```compile_fail
/// // And there is no constructor function to reach for either.
/// let c = open_ontologies::verdict::Certified::new("OOCert.certificate_sound");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Certified {
    theorem: &'static str,
}

impl Certified {
    /// The machine-checked statement behind the word.
    pub fn theorem(self) -> &'static str {
        self.theorem
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
/// let v = FolVerdict::ModelChecked(Certified { theorem: "Fol.satisfiable_of_check" });
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
    pub fn word(self) -> &'static str {
        match self {
            FolVerdict::ModelChecked(_) => "model_checked",
            FolVerdict::SatisfiableOracle => "satisfiable_oracle",
            FolVerdict::NoModelUpToSizeK => "no_model_up_to_size_k",
            FolVerdict::UnsatisfiableOracle => "unsatisfiable_oracle",
            FolVerdict::UnknownOracle => "unknown_oracle",
        }
    }
    /// The theorem, named only where one stands behind the word.
    pub fn theorem(self) -> Option<&'static str> {
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OwlReading(Certified);

impl OwlReading {
    /// The word, never shortened to "not entailed": it rides on
    /// `OwlLean.adequacy` in a sibling project AND on a Rust-to-Lean
    /// correspondence that decision 0005 item 2 says is pinned by tests and
    /// NOT proved.
    pub fn word(self) -> &'static str {
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
/// let v = ClosureVerdict::Checked(Certified { theorem: "OOCert.certificate_sound" });
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
    pub fn word(self) -> &'static str {
        match self {
            ClosureVerdict::Checked(_) => "checked",
            ClosureVerdict::Rejected => "rejected",
            ClosureVerdict::EngineOpinion => "engine_opinion",
        }
    }
    pub fn theorem(self) -> Option<&'static str> {
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
        ours.push(OwlReading::word(OwlReading(Certified { theorem: "x" })));
        ours.push(ClosureVerdict::Rejected.word());
        ours.push(ClosureVerdict::EngineOpinion.word());
        ours.push(ClosureVerdict::Checked(Certified { theorem: "x" }).word());
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
        #[cfg(unix)]
        {
            let mut c = Command::new("/bin/sh");
            c.arg("-c").arg(format!("exit {code}"));
            (CheckerBinary::found_at(PathBuf::from("/bin/sh")), c)
        }
        #[cfg(windows)]
        {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(format!("exit {code}"));
            (CheckerBinary::found_at(PathBuf::from("cmd")), c)
        }
    }

    /// A non-zero exit mints nothing, and there is no other route to a token.
    #[test]
    fn a_non_zero_exit_yields_no_certificate() {
        let (bin, c) = shell_exiting(1);
        let run = CheckerRun::spawn(&bin, c).expect("the system shell runs");
        assert_eq!(run.exit(), 1);
        assert!(run.accepted("OOCert.certificate_sound").is_none());
    }

    #[test]
    fn a_zero_exit_mints_the_theorem_it_was_given() {
        let (bin, c) = shell_exiting(0);
        let run = CheckerRun::spawn(&bin, c).expect("the system shell runs");
        let cert = run.accepted("OOCert.certificate_sound").expect("exit 0");
        assert_eq!(cert.theorem(), "OOCert.certificate_sound");
        assert_eq!(FolVerdict::ModelChecked(cert).word(), "model_checked");
        assert_eq!(FolVerdict::ModelChecked(cert).theorem(), Some("OOCert.certificate_sound"));
    }

    /// A checker that could not be started is an error, not a run, so it can
    /// never be asked whether it accepted anything.
    #[test]
    fn a_checker_that_cannot_start_is_not_a_run() {
        let bin = CheckerBinary::found_at(PathBuf::from("/nonexistent/oo-cert"));
        let c = Command::new("/nonexistent/oo-cert");
        assert!(CheckerRun::spawn(&bin, c).is_err());
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
        let cert = Certified { theorem: "Fol.satisfiable_of_check" };
        assert_eq!(serde_json::to_string(&FolVerdict::ModelChecked(cert)).unwrap(), "\"model_checked\"");
        assert_eq!(serde_json::to_string(&ClosureVerdict::Checked(cert)).unwrap(), "\"checked\"");
        assert_eq!(
            serde_json::to_string(&OwlReading(cert)).unwrap(),
            "\"not_entailed_under_unproved_translation\""
        );
    }
}
