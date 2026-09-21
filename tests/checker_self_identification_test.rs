//! Every checker says WHICH BINARY is running, and a reader can check it.
//!
//! Issue #204. Every checker printed the name of the theorem its acceptance
//! discharges and nothing that identifies the build that ran. A verdict is a
//! JSON object containing a theorem name, and any process can print that
//! object. The Rust side is careful never to fabricate the string, but a
//! reader had no way to tell an `oo-shacl` built from a commit whose proofs
//! were checked from one built where `Shacl` was not a default target and did
//! not compile, which the comment at the top of `lean/lakefile.toml` records
//! happening on 14 September 2026.
//!
//! A git revision baked in at build time would be a string the binary cannot
//! substantiate: a checker that SAYS it is `e483d75` is exactly as trustworthy
//! as one that says it discharged a theorem. So each checker reports the
//! digest of the file it is running from, read through `IO.appPath` when it
//! runs, and a reader matches that against the `SHASUMS.txt` the release
//! publishes. Since #241 the release publishes the checkers as assets, so the
//! number has something to be compared with.
//!
//! # What these tests are worth, and what they are not
//!
//! `every_checker_reports_the_digest_of_its_own_file` is the one that matters:
//! it runs each binary and compares what the binary SAYS about itself against
//! a digest this side computes from the same file with the `sha2` crate. It is
//! also the real test of the SHA-256 in `lean/SelfId/Sha256.lean`, which is
//! unverified and says so: three published vectors are checked at compile time
//! by `#guard`, and this checks a three-megabyte input against an independent
//! implementation.
//!
//! None of this makes a checker honest. A hostile binary can print any block
//! it likes and nothing here would notice. Residual hole 1 in `src/verdict.rs`
//! is narrowed and not closed, and the module header there says so.

mod common;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn bin_dir() -> PathBuf {
    repo().join("lean").join(".lake").join("build").join("bin")
}

/// The executables the lakefile declares, read from it rather than typed.
///
/// A tenth checker added to `lakefile.toml` without a `--version` turns this
/// file red on the day it lands, which a list of nine names could not do.
fn checkers_in_the_lakefile() -> BTreeSet<String> {
    let src = std::fs::read_to_string(repo().join("lean").join("lakefile.toml"))
        .expect("read lean/lakefile.toml");
    let mut out = BTreeSet::new();
    let mut in_exe = false;
    for line in src.lines() {
        let t = line.trim();
        if t == "[[lean_exe]]" {
            in_exe = true;
            continue;
        }
        if t.starts_with('[') {
            in_exe = false;
        }
        if in_exe && t.starts_with("name = \"") {
            let name = t.trim_start_matches("name = \"").trim_end_matches('"');
            out.insert(name.to_string());
            in_exe = false;
        }
    }
    assert!(
        out.len() >= 8,
        "the lakefile scan found only {out:?}, so its spelling has changed and this file \
         is measuring nothing"
    );
    out
}

fn skip() -> bool {
    // Any one of them standing in for "the Lean build has been run here".
    common::skip_unless(
        bin_dir().join("oo-cert").exists(),
        "the Lean checkers",
        "run `lake build` in lean/",
    )
}

fn version_block(exe: &str) -> serde_json::Value {
    let out = {
        let _gate = common::exec_gate();
        Command::new(bin_dir().join(exe)).arg("--version").output()
    }
    .unwrap_or_else(|e| panic!("run {exe} --version: {e}"));
    assert_eq!(out.status.code(), Some(0), "{exe} --version must exit 0");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let v: serde_json::Value = serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("{exe} --version printed no JSON ({e}): {text}"));
    v["checker"].clone()
}

fn sha256_of(path: &PathBuf) -> String {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).expect("read the binary");
    let mut h = Sha256::new();
    h.update(&bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn every_checker_the_lakefile_declares_answers_version() {
    if skip() {
        return;
    }
    for exe in checkers_in_the_lakefile() {
        let p = bin_dir().join(&exe);
        if !p.exists() {
            continue; // a target this build did not produce; the digest test covers the rest
        }
        let b = version_block(&exe);
        assert_eq!(b["name"], exe.as_str(), "the block must name the binary: {b}");
        assert!(
            b["toolchain"].as_str().is_some_and(|t| !t.is_empty()),
            "and the Lean toolchain that built it: {b}"
        );
    }
}

/// The test this file exists for.
#[test]
fn every_checker_reports_the_digest_of_its_own_file() {
    if skip() {
        return;
    }
    let mut checked = 0usize;
    for exe in checkers_in_the_lakefile() {
        let p = bin_dir().join(&exe);
        if !p.exists() {
            continue;
        }
        let said = version_block(&exe)["self_sha256"].as_str().unwrap_or_default().to_string();
        let real = sha256_of(&p);
        assert_eq!(
            said, real,
            "{exe} reports a digest of itself that does not match the file. Either the \
             SHA-256 in lean/SelfId/Sha256.lean is wrong, in which case no reported digest \
             will ever match a SHASUMS.txt line, or the binary is not reading its own path"
        );
        checked += 1;
    }
    assert!(checked >= 5, "only {checked} checkers were built, so this proved little");
    println!("{checked} checkers report their own digest, each matching an independent sha2");
}

/// A `--version` block and a verdict block must be the same claim.
#[test]
fn the_verdict_carries_the_same_block_as_version() {
    if skip() {
        return;
    }
    let d = std::env::temp_dir().join(format!("oo-selfid-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let a = d.join("asserted.tsv");
    let der = d.join("derivations.tsv");
    std::fs::write(&a, "<a>\t<b>\t<c>\n").unwrap();
    std::fs::write(&der, "").unwrap();

    let out = {
        let _gate = common::exec_gate();
        Command::new(bin_dir().join("oo-cert")).arg(&a).arg(&der).output()
    }
    .expect("run oo-cert");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let v: serde_json::Value = serde_json::from_str(&text).expect("oo-cert prints JSON");
    assert_eq!(v["theorem"], "OOCert.certificate_sound", "{text}");
    assert_eq!(
        v["checker"], version_block("oo-cert"),
        "a verdict and a --version must describe the same binary in the same words"
    );
}

/// The Rust side READS the block and never invents one.
#[test]
fn the_rust_side_reads_the_block_and_invents_nothing() {
    if skip() {
        return;
    }
    let src = std::fs::read_to_string(repo().join("src").join("verdict.rs")).expect("read it");
    assert!(
        !src.contains("self_sha256\":"),
        "src/verdict.rs must not build a checker block of its own. It reads the one the \
         checker printed; a second spelling here could describe a binary that never ran"
    );

    use open_ontologies::verdict::{CheckerBinary, CheckerRun};
    let d = std::env::temp_dir().join(format!("oo-selfid-read-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let a = d.join("asserted.tsv");
    let der = d.join("derivations.tsv");
    std::fs::write(&a, "<a>\t<b>\t<c>\n").unwrap();
    std::fs::write(&der, "").unwrap();

    let bin = bin_dir().join("oo-cert");
    let mut cmd = Command::new(&bin);
    cmd.arg(&a).arg(&der);
    let run = {
        let _gate = common::exec_gate();
        CheckerRun::spawn(&CheckerBinary::found_at(bin.clone()), cmd)
    }
    .expect("runs");
    let block = run.checker_block().expect("an oo-cert since #204 prints one");
    assert_eq!(block["self_sha256"].as_str().unwrap(), sha256_of(&bin));
}

/// And a process that is not a checker yields no block, rather than a blank one.
#[test]
fn something_that_prints_no_block_yields_none() {
    use open_ontologies::verdict::{CheckerBinary, CheckerRun};
    let d = std::env::temp_dir().join(format!("oo-selfid-none-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let script = d.join(if cfg!(windows) { "fake.cmd" } else { "fake.sh" });
    let body = if cfg!(windows) {
        "@echo off\r\necho {\"ok\":true,\"theorem\":\"OOCert.certificate_sound\"}\r\nexit /b 0\r\n"
            .to_string()
    } else {
        "#!/bin/sh\necho '{\"ok\":true,\"theorem\":\"OOCert.certificate_sound\"}'\nexit 0\n"
            .to_string()
    };
    {
        // Written with no fork in flight. This test wrote a script and executed
        // it while four siblings were forking `--version`, and CI answered
        // `ExecutableFileBusy`. See `common::exec_gate`.
        let _gate = common::exec_gate();
        std::fs::write(&script, body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    let cmd = Command::new(&script);
    let run = {
        let _gate = common::exec_gate();
        CheckerRun::spawn(&CheckerBinary::found_at(script), cmd)
    }
    .expect("runs");
    assert_eq!(run.named_theorem().as_deref(), Some("OOCert.certificate_sound"));
    assert!(
        run.checker_block().is_none(),
        "a process that printed no checker block must yield None. Filling it in would let a \
         report describe a binary that never said anything about itself"
    );
}
