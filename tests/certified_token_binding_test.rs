//! A certified token says WHICH artefact was accepted, not merely that
//! something was.
//!
//! Issue #164. `Certified` proved *an* acceptance: it carried the theorem name
//! the checker printed, so the warrant could not drift from the run that
//! earned it, but nothing tied it to the thing the run was about. It was also
//! `Copy`, so a token earned over goal A could reappear on goal B through any
//! use of the value, invisibly. The issue called the existing safety what it
//! was: discipline, not a type.
//!
//! Two changes, and the second is what makes the first structural.
//!
//! The token now carries the digest of the inputs the accepting run was
//! handed, computed by `CheckerRun::spawn` from those files **before** it
//! spawned, so a checker that writes into its own input directory is hashed as
//! it was read rather than as it was left. `Certified::is_about` lets a holder
//! of an artefact ask whether the token is about the thing in their hand.
//!
//! And the token is no longer `Copy`. It is still `Clone`, which this file
//! does not pretend otherwise about: the difference is that `.clone()` is a
//! visible act a reader and a grep can both find, where an implicit copy was
//! neither.
//!
//! A shell script is not `oo-cert`, and `src/verdict.rs` states plainly that
//! the type system cannot tell the two apart — that is residual hole 1 and
//! this issue does not close it. What a script CAN do is exit zero while
//! naming a theorem, which is the whole of what the mint reads, so it is
//! enough to test what the mint does with the digest.

use open_ontologies::verdict::{subject_digest, CheckerBinary, CheckerRun};
use std::path::{Path, PathBuf};
use std::process::Command;

const THEOREM: &str = "OOCert.certificate_sound";

/// ONE DIRECTORY PER TEST, and the reason is worth writing down.
///
/// These tests first shared a single directory keyed on the process id, and
/// two of them wrote a file called `asserted.tsv` into it. Cargo runs tests in
/// parallel threads of one process, so one test's write landed between
/// another's run and its recomputation, and the digests disagreed. It passed
/// on my machine and failed on CI, which is the signature of exactly this.
/// The script the fake checker runs lives here too: writing over a file
/// another thread is executing is `ETXTBSY` on Linux.
fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("oo-token-binding-{}-{tag}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn file(tag: &str, name: &str, body: &str) -> PathBuf {
    let p = dir(tag).join(name);
    std::fs::write(&p, body).unwrap();
    p
}

/// A script that exits zero and prints a theorem, which is all the mint reads.
fn fake_checker(tag: &str) -> PathBuf {
    let ext = if cfg!(windows) { "cmd" } else { "sh" };
    let p = dir(tag).join(format!("accepts.{ext}"));
    let say = format!(r#"{{"ok":true,"theorem":"{THEOREM}"}}"#);
    let body = if cfg!(windows) {
        format!("@echo off\r\necho {say}\r\nexit /b 0\r\n")
    } else {
        format!("#!/bin/sh\necho '{say}'\nexit 0\n")
    };
    std::fs::write(&p, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    p
}

fn run_over(tag: &str, inputs: &[&Path]) -> std::io::Result<CheckerRun> {
    let bin = fake_checker(tag);
    let mut cmd = Command::new(&bin);
    for i in inputs {
        cmd.arg(i);
    }
    CheckerRun::spawn(&CheckerBinary::found_at(bin), cmd, inputs)
}

#[test]
fn a_token_is_about_the_artefact_its_run_was_handed() {
    let a = file("about", "asserted.tsv", "s\tp\to\n");
    let run = run_over("about", &[&a]).expect("the script runs");
    let token = run.accepted_naming(&[THEOREM]).expect("exit 0 naming the theorem");

    let mine = subject_digest(&[&a]).expect("the caller hashes the same file");
    assert!(
        token.is_about(&mine),
        "a holder of the artefact the checker was handed must be able to recognise the token"
    );
    assert_eq!(token.subject_sha256().len(), 64, "a sha256 in hex");
}

#[test]
fn a_token_earned_over_one_artefact_is_not_about_another() {
    let a = file("other", "goal-a.tsv", "a\tp\to\n");
    let b = file("other", "goal-b.tsv", "b\tp\to\n");
    let token = run_over("other", &[&a]).unwrap().accepted_naming(&[THEOREM]).expect("minted");

    let other = subject_digest(&[&b]).unwrap();
    assert!(
        !token.is_about(&other),
        "THE DEFECT THIS CLOSES. A token earned over one artefact answered nothing about \
         another, so a caller could attach it to a second goal and no check objected"
    );
}

#[test]
fn the_digest_is_of_the_bytes_and_not_of_the_path() {
    // The same content under two directories is the same subject: a digest that
    // changed when a directory moved would be a digest nobody could compare.
    let one = dir("bytes").join("one");
    let two = dir("bytes").join("two");
    std::fs::create_dir_all(&one).unwrap();
    std::fs::create_dir_all(&two).unwrap();
    let p1 = one.join("asserted.tsv");
    let p2 = two.join("asserted.tsv");
    std::fs::write(&p1, "s\tp\to\n").unwrap();
    std::fs::write(&p2, "s\tp\to\n").unwrap();
    assert_eq!(subject_digest(&[&p1]).unwrap(), subject_digest(&[&p2]).unwrap());

    // And a different byte is a different subject.
    std::fs::write(&p2, "s\tp\tOTHER\n").unwrap();
    assert_ne!(subject_digest(&[&p1]).unwrap(), subject_digest(&[&p2]).unwrap());
}

#[test]
fn the_digest_knows_which_file_played_which_part() {
    // `oo-cert A D` handed the derivations as the asserted file is a different
    // run from the right one, and the digest has to say so. The base name is in
    // the framing for exactly this.
    let a = file("parts", "asserted.tsv", "same bytes\n");
    let d = file("parts", "derivations.tsv", "same bytes\n");
    assert_ne!(
        subject_digest(&[&a, &d]).unwrap(),
        subject_digest(&[&d, &a]).unwrap(),
        "swapping which file is the asserted one must change the subject"
    );
}

#[test]
fn a_run_that_names_no_inputs_mints_nothing_because_it_cannot_start() {
    let e = run_over("empty", &[]).expect_err("a run about nothing is refused");
    assert_eq!(e.kind(), std::io::ErrorKind::InvalidInput);
    assert!(
        e.to_string().contains("must name the files it is about"),
        "the refusal must say why: {e}"
    );
}

#[test]
fn an_unreadable_input_is_an_error_and_never_a_digest() {
    let missing = dir("missing").join("was-never-written.tsv");
    assert!(subject_digest(&[&missing]).is_err());
    assert!(
        run_over("missing", &[&missing]).is_err(),
        "a run whose inputs could not be read must not mint a token bound to them"
    );
}

/// The `Copy` removal, asserted where a reader will look for it.
///
/// A trait bound is the only way to state this from outside the crate, and a
/// `compile_fail` doctest is the only way to assert that a bound does NOT
/// hold. This test exists so the claim has a home in the test suite; the
/// enforcing half is the doctest on `Certified` itself.
#[test]
fn the_token_is_clone_but_not_copy() {
    fn assert_clone<T: Clone>() {}
    assert_clone::<open_ontologies::verdict::Certified>();
    // `assert_copy::<Certified>()` is a compile error, asserted by the
    // `compile_fail` doctest in src/verdict.rs. Restoring `Copy` would make
    // that doctest compile, and a `compile_fail` doctest that compiles fails.
}
