//! The whole certified vocabulary, in one place, checked from outside the crate.
//!
//! `src/verdict.rs` makes a certified verdict unconstructible without evidence:
//! `Certified` has a private field, `CheckerRun::accepted` is its only
//! producer, and the only constructor of a `CheckerRun` spawns a process and
//! reads its exit status. The `compile_fail` doctests in that module are the
//! gate on the CONSTRUCTION side.
//!
//! This file is the gate on the other two sides, and it runs as an ordinary
//! integration test, from outside the crate, where privacy actually applies.
//!
//! 1. **The wire words are pinned as literals.** A rename inside the crate that
//!    changed one of them would change an MCP tool response, a CLI JSON field
//!    and every downstream consumer, silently, because the strings are produced
//!    by `word()` now and not typed at each site. This file types them.
//! 2. **No word the engine states is a word a Lean checker states.** An engine
//!    opinion and a machine-checked result must never share a string, or a
//!    consumer cannot tell them apart. `tests/lean_refutation_producer_test.rs
//!    ::the_engine_never_states_the_checkers_verdict` checks that over a
//!    RESPONSE; this checks it over the vocabulary the responses are drawn
//!    from, so a word added tomorrow is covered before anyone writes a run that
//!    prints it.
//!
//! Where a certified word has to be named, this file EARNS the token the way
//! the engine does: it runs a process that exits zero through the crate's own
//! `run_checker`. There is no other way to get one, which is the point.

mod common;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use open_ontologies::closure_diff::Warrant;
use open_ontologies::projection_entailment::{self as pe, CertKind, CheckerStatus, GoalVerdict};
use open_ontologies::verdict::{
    CHECKER_OWNED_WORDS, CheckerBinary, CheckerRun, Certified, ClosureVerdict, EngineRefutation,
    FolVerdict,
};

/// A shell script that exits with `code`, made executable.
///
/// Not `/bin/true`: that is `/usr/bin/true` on macOS and absent from `/bin`
/// entirely, which this suite discovered by failing. A script written here
/// runs the same everywhere and covers the non-zero codes too.
/// A real executable on disk, for the call sites that hand `run_checker` a
/// PATH rather than a command.
///
/// Every script is written ONCE, on first use, for every exit code the suite
/// needs. That matters: `cargo test` runs these in parallel, and on Linux a
/// thread that forks while another holds a write fd open makes the child
/// inherit it, so a later exec of that file fails with ETXTBSY however unique
/// its name is. CI failed that way twice. Writing before the parallel phase
/// begins means there is no open write fd left to inherit.
fn script_exiting(code: i32) -> PathBuf {
    script_saying(code, "OOCert.certificate_sound")
}

/// A fake checker that prints the theorem it is told to and then exits.
///
/// Keyed by BOTH, because which theorem a checker names is now part of what
/// the mint reads: `oo-horn` legitimately names one of two depending on its
/// rule table, and a checker naming a third mints nothing.
fn script_saying(code: i32, theorem: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("oo-verdict-vocabulary-scripts");
    std::fs::create_dir_all(&dir).unwrap();
    let ext = if cfg!(windows) { "cmd" } else { "sh" };
    let slug: String = theorem.chars().filter(|c| c.is_alphanumeric()).collect();
    // A FRESH path per call, not one keyed on (code, theorem).
    //
    // Cargo runs these tests as parallel threads of one process, and two of
    // them ask for the same (0, "OOCert.certificate_sound") script. Keyed on
    // the pair they got the same FILE, so one thread rewrote a script while
    // the other was executing it. On Linux that is ETXTBSY: the spawn fails,
    // `run_checker` reports the checker as absent, and
    // `only_a_zero_exit_produces_an_acceptance` fails an assertion about exit
    // codes for a reason that has nothing to do with exit codes. It passed on
    // macOS every time, which is why it read as a CI defect.
    //
    // A counter costs one file per call and removes the class rather than the
    // instance: no future pair of callers can collide either.
    static NEXT_SCRIPT: AtomicU64 = AtomicU64::new(0);
    let serial = NEXT_SCRIPT.fetch_add(1, Ordering::Relaxed);
    let p = dir.join(format!("exit{code}_{slug}_{serial}.{ext}"));
    // The fake PRINTS its theorem, because a real checker does and the mint now
    // reads it. A script that exits zero in silence named nothing and correctly
    // mints nothing; accepting with one would be testing the old contract.
    let say = format!(r#"{{"ok":true,"theorem":"{theorem}"}}"#);
    let body = if cfg!(windows) {
        format!("@echo off\r\necho {say}\r\nexit /b {code}\r\n")
    } else {
        format!("#!/bin/sh\necho '{say}'\nexit {code}\n")
    };
    {
        // The write and the chmod happen with no fork in flight; see EXEC_GATE.
        let _gate = common::exec_gate();
        std::fs::write(&p, body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    p
}


/// A `Certified`, obtained the only way anything can obtain one: by running
/// something that exits zero.
///
/// A shell script is not `oo-cert`, and `src/verdict.rs` says plainly that the
/// type system cannot tell the two apart. What it CAN say is that a process
/// ran and exited zero, and that is exactly what this borrows in order to name
/// the certified variants below.
fn earned(theorem: &'static str) -> Certified {
    // The fake has to PRINT the theorem, because that is now what the mint
    // reads. A shell that exits zero in silence is a checker that named
    // nothing, and it correctly mints nothing.
    let (bin, cmd) = shell_naming(theorem);
    let run = {
        let _gate = common::exec_gate();
        CheckerRun::spawn(&bin, cmd)
    }
    .expect("the system shell must be runnable");
    assert_eq!(run.exit(), 0);
    run.accepted_naming(&[theorem]).expect("exit 0 naming it mints the token")
}

fn shell_naming(theorem: &str) -> (CheckerBinary, std::process::Command) {
    #[cfg(unix)]
    {
        let mut c = std::process::Command::new("/bin/sh");
        c.arg("-c")
            .arg(format!("echo '{{\"ok\":true,\"theorem\":\"{theorem}\"}}'"));
        (CheckerBinary::found_at(PathBuf::from("/bin/sh")), c)
    }
    #[cfg(windows)]
    {
        // `raw_arg`, not `arg`: `arg` escapes the quotes for a C-runtime
        // parser and cmd.exe has none, so it echoed the backslashes and the
        // line was not JSON. Same defect, same fix as `shell_saying` in
        // `src/verdict.rs`.
        use std::os::windows::process::CommandExt;
        let mut c = std::process::Command::new("cmd");
        c.raw_arg(format!("/C echo {{\"ok\":true,\"theorem\":\"{theorem}\"}}"));
        (CheckerBinary::found_at(PathBuf::from("cmd")), c)
    }
}

/// Every word this crate may print as a verdict, with the exact bytes it must
/// put on the wire. Typed here on purpose: this file is the pin.
fn every_word() -> Vec<(&'static str, String)> {
    let cert = earned("OOCert.certificate_sound");
    vec![
        // decision 0006, `onto_fol_model`
        ("FolVerdict::ModelChecked", FolVerdict::ModelChecked(cert).word().to_string()),
        ("FolVerdict::SatisfiableOracle", FolVerdict::SatisfiableOracle.word().to_string()),
        ("FolVerdict::NoModelUpToSizeK", FolVerdict::NoModelUpToSizeK.word().to_string()),
        ("FolVerdict::UnsatisfiableOracle", FolVerdict::UnsatisfiableOracle.word().to_string()),
        ("FolVerdict::UnknownOracle", FolVerdict::UnknownOracle.word().to_string()),
        // decision 0002 / 0007, the closure certificate
        ("ClosureVerdict::Checked", ClosureVerdict::Checked(cert).word().to_string()),
        ("ClosureVerdict::Rejected", ClosureVerdict::Rejected.word().to_string()),
        ("ClosureVerdict::EngineOpinion", ClosureVerdict::EngineOpinion.word().to_string()),
        ("Warrant::Checked", Warrant::Checked(cert).name().to_string()),
        ("Warrant::AssertedInSource", Warrant::AssertedInSource.name().to_string()),
        ("Warrant::EngineOpinion", Warrant::EngineOpinion.name().to_string()),
        // decision 0007, per goal
        ("GoalVerdict::PreservedChecked", GoalVerdict::PreservedChecked(cert).word().to_string()),
        (
            "GoalVerdict::PreservedUnderSuppliedRulesChecked",
            GoalVerdict::PreservedUnderSuppliedRulesChecked(cert).word().to_string(),
        ),
        ("GoalVerdict::PreservedAsserted", GoalVerdict::PreservedAsserted.word().to_string()),
        ("GoalVerdict::PreservedUnchecked", GoalVerdict::PreservedUnchecked.word().to_string()),
        (
            "GoalVerdict::LostUnderProfileUnchecked",
            GoalVerdict::LostUnderProfileUnchecked.word().to_string(),
        ),
        ("GoalVerdict::UngroundedInSource", GoalVerdict::UngroundedInSource.word().to_string()),
        ("GoalVerdict::ProjectionOnly", GoalVerdict::ProjectionOnly.word().to_string()),
        ("GoalVerdict::CertificateRejected", GoalVerdict::CertificateRejected.word().to_string()),
        ("GoalVerdict::Refused", GoalVerdict::Refused.word().to_string()),
        // the engine's own words about a contradiction it found itself
        (
            "EngineRefutation::ClashFoundByThisEngine",
            EngineRefutation::ClashFoundByThisEngine.word().to_string(),
        ),
        (
            "EngineRefutation::RefutationWrittenNotYetChecked",
            EngineRefutation::RefutationWrittenNotYetChecked.word().to_string(),
        ),
    ]
}

/// The wire format, pinned. These bytes are in MCP tool responses, in CLI JSON
/// and in fixtures, so a rename is a breaking change to every consumer and must
/// be a deliberate one.
#[test]
fn the_wire_words_are_exactly_these() {
    let want: Vec<(&str, &str)> = vec![
        ("FolVerdict::ModelChecked", "model_checked"),
        ("FolVerdict::SatisfiableOracle", "satisfiable_oracle"),
        ("FolVerdict::NoModelUpToSizeK", "no_model_up_to_size_k"),
        ("FolVerdict::UnsatisfiableOracle", "unsatisfiable_oracle"),
        ("FolVerdict::UnknownOracle", "unknown_oracle"),
        ("ClosureVerdict::Checked", "checked"),
        ("ClosureVerdict::Rejected", "rejected"),
        ("ClosureVerdict::EngineOpinion", "engine_opinion"),
        ("Warrant::Checked", "checked"),
        ("Warrant::AssertedInSource", "asserted_in_source"),
        ("Warrant::EngineOpinion", "engine_opinion"),
        ("GoalVerdict::PreservedChecked", "preserved_checked"),
        (
            "GoalVerdict::PreservedUnderSuppliedRulesChecked",
            "preserved_under_supplied_rules_checked",
        ),
        ("GoalVerdict::PreservedAsserted", "preserved_asserted"),
        ("GoalVerdict::PreservedUnchecked", "preserved_unchecked"),
        ("GoalVerdict::LostUnderProfileUnchecked", "lost_under_profile_unchecked"),
        ("GoalVerdict::UngroundedInSource", "ungrounded_in_source"),
        ("GoalVerdict::ProjectionOnly", "projection_only"),
        ("GoalVerdict::CertificateRejected", "certificate_rejected"),
        ("GoalVerdict::Refused", "refused"),
        ("EngineRefutation::ClashFoundByThisEngine", "clash_found_by_this_engine"),
        ("EngineRefutation::RefutationWrittenNotYetChecked", "refutation_written_not_yet_checked"),
    ];
    let got = every_word();
    assert_eq!(got.len(), want.len(), "a verdict was added or removed without pinning its word");
    for ((gname, gword), (wname, wword)) in got.iter().zip(want.iter()) {
        assert_eq!(gname, wname, "the vocabularies are listed in different orders");
        assert_eq!(
            gword, wword,
            "{gname} now writes {gword:?} and used to write {wword:?}. That string is in MCP \
             responses and CLI JSON. If the rename is deliberate, change it here and say so in \
             the CHANGELOG; if it is not, this is the bug."
        );
    }
}

/// `serde` writes the same bare string the derives used to write, which is what
/// keeps the wire format unchanged now that the certified variants carry a
/// payload serde would otherwise have wrapped in an object.
#[test]
fn serde_writes_the_bare_word_and_never_an_object() {
    let cert = earned("OOCert.certificate_sound");
    let cases: Vec<(String, &str)> = vec![
        (serde_json::to_string(&FolVerdict::ModelChecked(cert)).unwrap(), "\"model_checked\""),
        (
            serde_json::to_string(&FolVerdict::SatisfiableOracle).unwrap(),
            "\"satisfiable_oracle\"",
        ),
        (serde_json::to_string(&ClosureVerdict::Checked(cert)).unwrap(), "\"checked\""),
        (serde_json::to_string(&Warrant::Checked(cert)).unwrap(), "\"checked\""),
        (serde_json::to_string(&Warrant::AssertedInSource).unwrap(), "\"asserted_in_source\""),
        (
            serde_json::to_string(&GoalVerdict::PreservedChecked(cert)).unwrap(),
            "\"preserved_checked\"",
        ),
        (
            serde_json::to_string(&GoalVerdict::PreservedUnderSuppliedRulesChecked(cert)).unwrap(),
            "\"preserved_under_supplied_rules_checked\"",
        ),
        (
            serde_json::to_string(&EngineRefutation::ClashFoundByThisEngine).unwrap(),
            "\"clash_found_by_this_engine\"",
        ),
    ];
    for (got, want) in cases {
        assert_eq!(got, want, "a verdict serialised as something other than its bare word");
    }
}

/// An engine opinion and a machine-checked result must never share a string.
///
/// The response-level version of this lives in
/// `tests/lean_refutation_producer_test.rs::the_engine_never_states_the_checkers_verdict`.
/// This is the vocabulary-level version, so a word added to any of these enums
/// is covered before a run exists that could print it.
#[test]
fn no_word_the_engine_states_is_a_word_a_lean_checker_states() {
    for (name, word) in every_word() {
        assert!(
            !CHECKER_OWNED_WORDS.contains(&word.as_str()),
            "{name} states {word:?}, which is a verdict a Lean binary prints for itself. \
             Rust may put one of those in a report only by echoing the checker's own bytes."
        );
    }
}

/// The theorem name is as unspeakable as the verdict, because it travels inside
/// the evidence rather than beside it.
///
/// `tests/closure_diff_test.rs::an_unchecked_run_never_prints_the_checked_word`
/// asserts that `OOCert.certificate_sound` appears nowhere in an unchecked
/// report. It can now only appear where a `Certified` does.
#[test]
fn a_theorem_is_named_only_where_the_evidence_is() {
    let cert = earned("OOCert.certificate_sound");
    assert_eq!(GoalVerdict::PreservedChecked(cert).warrant(), "OOCert.certificate_sound");
    assert_eq!(Warrant::Checked(cert).theorem(), Some("OOCert.certificate_sound"));
    assert_eq!(ClosureVerdict::Checked(cert).theorem(), Some("OOCert.certificate_sound"));

    for v in [
        GoalVerdict::PreservedAsserted,
        GoalVerdict::PreservedUnchecked,
        GoalVerdict::LostUnderProfileUnchecked,
        GoalVerdict::UngroundedInSource,
        GoalVerdict::ProjectionOnly,
        GoalVerdict::CertificateRejected,
        GoalVerdict::Refused,
    ] {
        assert_eq!(v.warrant(), "none", "{v} named a theorem with no evidence behind it");
        assert!(!v.is_checked());
    }
    for w in [Warrant::AssertedInSource, Warrant::EngineOpinion] {
        assert_eq!(w.theorem(), None);
        assert!(!w.is_checked());
    }
    for v in [ClosureVerdict::Rejected, ClosureVerdict::EngineOpinion] {
        assert_eq!(v.theorem(), None);
        assert!(!v.is_checked());
    }
    for v in [
        FolVerdict::SatisfiableOracle,
        FolVerdict::NoModelUpToSizeK,
        FolVerdict::UnsatisfiableOracle,
        FolVerdict::UnknownOracle,
    ] {
        assert_eq!(v.theorem(), None);
        assert!(!v.is_certified());
    }
}

/// The token comes from a zero exit code and from nothing else, checked through
/// the crate's own `run_checker` rather than through the primitive.
///
/// `sh -c "exit 1"` is `lean/Main.lean`'s REJECTED, `exit 2` is its UNREADABLE,
/// and neither is an acceptance. A run that could not start is `Absent`.
#[test]
fn only_a_zero_exit_produces_an_acceptance() {
    // Per PROCESS, not per machine. A fixed name under the system temp
    // directory is shared by every concurrent run on the box, and these two
    // files are truncated on entry, so a second run could empty them under the
    // first one's feet.
    let dir = std::env::temp_dir().join(format!("oo-verdict-vocabulary-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let a = dir.join("asserted.tsv");
    let d = dir.join("derivations.tsv");
    std::fs::write(&a, "").unwrap();
    std::fs::write(&d, "").unwrap();

    let run = |code: i32| {
        let script = script_exiting(code);
        let _gate = common::exec_gate();
        pe::run_checker(CertKind::OoCert, Some(script.as_path()), &a, &d, None)
    };

    // `assert!(matches!(...))` prints nothing but the line number, and this
    // test went red twice on CI and green everywhere else. A failure has to
    // say WHICH status it got, or the next person reads a line number and
    // guesses, which is what happened.
    let got = |st: &CheckerStatus| -> String {
        match st {
            CheckerStatus::Accepted(_) => "accepted".to_string(),
            CheckerStatus::Rejected { stdout } => format!("rejected, stdout {stdout:?}"),
            CheckerStatus::Unreadable { stdout } => format!("unreadable, stdout {stdout:?}"),
            CheckerStatus::Absent { what, .. } => format!("ABSENT: {what}"),
            CheckerStatus::NotNeeded { what } => format!("not needed: {what}"),
        }
    };
    let r0 = run(0);
    assert!(matches!(r0, CheckerStatus::Accepted(_)), "exit 0 must accept, got {}", got(&r0));
    let r1 = run(1);
    assert!(matches!(r1, CheckerStatus::Rejected { .. }), "exit 1 must reject, got {}", got(&r1));
    let r2 = run(2);
    assert!(matches!(r2, CheckerStatus::Unreadable { .. }), "exit 2 unreadable, got {}", got(&r2));
    let r3 = run(3);
    assert!(matches!(r3, CheckerStatus::Unreadable { .. }), "exit 3 unreadable, got {}", got(&r3));

    let absent = {
        let _gate = common::exec_gate();
        pe::run_checker(
            CertKind::OoCert,
            Some(Path::new("/nonexistent/oo-cert")),
            &a,
            &d,
            None,
        )
    };
    assert!(absent.is_absent(), "a checker that cannot be found accepted nothing");

    // And the acceptance carries the theorem the KIND names, not one the
    // caller chose afterwards.
    // `oo-horn` names ONE OF TWO, and the token carries whichever it named.
    // `lean/HMain.lean` prints the absolute `OOCert.entails_of_builtin_horn`
    // when the rule table is exactly the built-ins and the relativised
    // `OOCert.horn_certificate_sound` for any other table. The Rust side used
    // to hardcode the relativised name for both, so the field that exists to
    // tell an entailment from an entailment-under-supplied-rules told neither.
    for named in ["OOCert.horn_certificate_sound", "OOCert.entails_of_builtin_horn"] {
        let script = script_saying(0, named);
        let status = {
            let _gate = common::exec_gate();
            pe::run_checker(CertKind::OoHorn, Some(script.as_path()), &a, &d, Some(a.as_path()))
        };
        match status {
            CheckerStatus::Accepted(acc) => {
                assert_eq!(acc.theorem(), named, "the token must carry what oo-horn printed");
                assert_eq!(acc.certified().theorem(), named);
            }
            other => panic!("exit 0 naming {named} must be an acceptance: {other:?}"),
        }
    }

    // And a checker that exits zero while naming a statement this call site
    // was not written against mints nothing. That is the case the old
    // signature could not see: it would have minted a token labelled with
    // whatever the Rust literal said.
    let wrong = script_saying(0, "OOCert.something_else_entirely");
    assert!(
        !matches!(
            {
                let _gate = common::exec_gate();
                pe::run_checker(CertKind::OoHorn, Some(wrong.as_path()), &a, &d, Some(a.as_path()))
            },
            CheckerStatus::Accepted(_)
        ),
        "a checker naming an unexpected theorem must not mint an acceptance"
    );
}
