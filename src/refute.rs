// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Run `oo-refute` over a refutation this engine produced, and believe it only
//! when it names the theorem.
//!
//! The producer in `reason.rs` writes `refutation.tsv` in the format
//! `OOCert.RefuteParse.parseRefutation` reads. Writing a file in that format
//! proves nothing: only exit 0 from `oo-refute check` naming
//! [`OOCert.refutation_fast_sound`] does, and that evidence is minted here, by
//! [`crate::verdict::CheckerRun::accepted_naming`], like every other checker in
//! this crate.
//!
//! A missing checker is reported as [`RefuteCheck::CheckerAbsent`] and is never
//! a pass. Nothing in this module has a timeout: the checker either answers or
//! the process is killed, which is the same choice `src/sat.rs` and
//! `src/projection_entailment.rs` make for their checkers.

use crate::verdict::{Certified, CheckerBinary, CheckerRun};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The theorem `oo-refute check` prints when it accepts a refutation.
///
/// This is a name this crate is allowed to REQUIRE. It is never printed as a
/// verdict: the accepting run's token is the only thing that crosses.
pub const THEOREM: &str = "OOCert.refutation_fast_sound";

/// What running `oo-refute check` over the pair produced.
///
/// Four answers, never folded into each other. In particular
/// [`RefuteCheck::CheckerAbsent`] is not a pass and is not a rejection: nothing
/// was checked.
#[derive(Debug)]
pub enum RefuteCheck {
    /// The checker accepted, and named [`THEOREM`]. The token proves the run
    /// happened over the two files this call was handed.
    Accepted(Certified),
    /// Exit 1: the checker read the pair and refused it. `stdout` is its bytes
    /// verbatim, including the `reason` it printed; this module does not
    /// interpret it.
    Rejected { stdout: String },
    /// Exit 2, an exit code this module has no reading for, or exit 0 that
    /// named a different theorem. `stdout` is what the checker emitted
    /// (stdout and stderr together) so the defect can be seen; no verdict is
    /// claimed. `exit` is the code the binary actually returned, kept because
    /// "exit 0 and the wrong theorem" is a different defect from "exit 2".
    Unreadable { stdout: String, exit: i32 },
    /// Discovery found no `oo-refute`, or found one that could not be run.
    /// NOTHING WAS CHECKED.
    CheckerAbsent { what: String },
}

impl RefuteCheck {
    /// The word this crate may print about the run.
    pub fn status(&self) -> &'static str {
        match self {
            RefuteCheck::Accepted(_) => "accepted",
            RefuteCheck::Rejected { .. } => "rejected",
            RefuteCheck::Unreadable { .. } => "unreadable",
            RefuteCheck::CheckerAbsent { .. } => "checker_absent_nothing_was_checked",
        }
    }

    /// The exit code, when a checker ran at all.
    pub fn exit(&self) -> Option<i32> {
        match self {
            RefuteCheck::Accepted(_) => Some(0),
            RefuteCheck::Rejected { .. } => Some(1),
            RefuteCheck::Unreadable { exit, .. } => Some(*exit),
            RefuteCheck::CheckerAbsent { .. } => None,
        }
    }

    /// The checker's own bytes, when it ran and did not accept.
    pub fn stdout(&self) -> Option<&str> {
        match self {
            RefuteCheck::Rejected { stdout } | RefuteCheck::Unreadable { stdout, .. } => {
                Some(stdout)
            }
            _ => None,
        }
    }

    /// The accepting token, if there is one.
    pub fn certified(&self) -> Option<&Certified> {
        match self {
            RefuteCheck::Accepted(c) => Some(c),
            _ => None,
        }
    }

    /// Why no checker ran, when none did. `None` for every status that actually
    /// reached a binary: "the checker rejected this" is a different finding
    /// from "there was no checker".
    pub fn what(&self) -> Option<&str> {
        match self {
            RefuteCheck::CheckerAbsent { what } => Some(what),
            _ => None,
        }
    }
}

fn which_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

/// Find the `oo-refute` binary.
///
/// `explicit` is an instruction, not a hint: when a caller names a path that
/// does not exist this returns an error rather than falling back. The same is
/// true of `$OO_REFUTE`. Discovery order is otherwise the one the other
/// checker callers use: the env var, this checkout's `lake build` output, then
/// `PATH`.
pub fn find_checker(explicit: Option<&Path>) -> Result<CheckerBinary, String> {
    if let Some(p) = explicit {
        return if p.is_file() {
            Ok(CheckerBinary::found_at(p.to_path_buf()))
        } else {
            Err(format!(
                "the oo-refute checker named at {} does not exist. Build it with \
                 `cd lean && lake build oo-refute`",
                p.display()
            ))
        };
    }
    if let Ok(v) = std::env::var("OO_REFUTE") {
        let p = PathBuf::from(&v);
        return if p.is_file() {
            Ok(CheckerBinary::found_at(p))
        } else {
            Err(format!(
                "OO_REFUTE is set to {v}, which does not exist. Build it with \
                 `cd lean && lake build oo-refute`"
            ))
        };
    }
    let built = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lean/.lake/build/bin/oo-refute");
    if built.is_file() {
        return Ok(CheckerBinary::found_at(built));
    }
    if let Some(found) = which_on_path("oo-refute") {
        return Ok(CheckerBinary::found_at(found));
    }
    Err(format!(
        "no oo-refute checker found, so NOTHING WAS CHECKED. Build it with \
         `cd lean && lake build oo-refute` (looked for $OO_REFUTE, {}, and \
         oo-refute on PATH)",
        built.display()
    ))
}

/// Check `refutation` against `asserted`.
///
/// Both files are handed to the checker as inputs, and the accepting token's
/// subject digest covers both. The checker is run with
/// `oo-refute check ASSERTED REFUTATION`, which is its documented CLI.
pub fn check(asserted: &Path, refutation: &Path, explicit: Option<&Path>) -> RefuteCheck {
    let binary = match find_checker(explicit) {
        Ok(b) => b,
        Err(what) => return RefuteCheck::CheckerAbsent { what },
    };
    let mut cmd = Command::new(binary.path());
    cmd.arg("check").arg(asserted).arg(refutation);
    let run = match CheckerRun::spawn(&binary, cmd, &[asserted, refutation]) {
        Ok(run) => run,
        Err(e) => {
            return RefuteCheck::CheckerAbsent {
                what: format!(
                    "oo-refute was found at {} but could not be run ({e}), so NOTHING WAS CHECKED",
                    binary.path().display()
                ),
            };
        }
    };
    match run.exit() {
        0 => match run.accepted_naming(&[THEOREM]) {
            Some(certified) => RefuteCheck::Accepted(certified),
            // Exit 0 without the theorem is not an acceptance. It is also not
            // a rejection: the checker did not say the refutation was wrong,
            // it failed to say anything this crate can read.
            None => RefuteCheck::Unreadable {
                stdout: run.output(),
                exit: run.exit(),
            },
        },
        1 => RefuteCheck::Rejected {
            stdout: run.output(),
        },
        _ => RefuteCheck::Unreadable {
            stdout: run.output(),
            exit: run.exit(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_explicit_checker_that_is_not_there_is_an_error_that_names_the_build() {
        let err = find_checker(Some(Path::new("/nonexistent/oo-refute")))
            .expect_err("a named path that does not exist is an instruction");
        assert!(err.contains("cd lean && lake build oo-refute"), "{err}");
    }
}
