// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! The command line, end to end, including the exit codes.
//!
//! Wiring that nothing exercises is how `push` came to be proxied to a batch
//! runner with no arm for it: the one proxy-able command that could not run
//! while a daemon was up. These tests run the real binary.
//!
//! The store is in-memory per process, so `batch` is the only way to load a
//! source and then ask about a slice of it in one run, exactly as for
//! `reason --certificate`.

mod common;

use std::path::Path;
use std::process::Command;

fn lean_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lean")
}

fn lake_available() -> bool {
    // `.current_dir(lean_dir())`: elan resolves the toolchain from the working
    // directory, and the crate root has no `lean-toolchain` in its ancestry.
    Command::new("lake")
        .arg("--version")
        .current_dir(lean_dir())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn skip() -> bool {
    common::skip_unless(
        lake_available(),
        "lake (the Lean 4 build tool)",
        "install elan from https://github.com/leanprover/elan; lean/lean-toolchain pins the version",
    )
}

/// Every invocation gets its own `--data-dir`, the discipline `cli_test.rs`
/// keeps: without it these open the developer's real store, run migrations
/// against it, and race every other process that has it open.
fn batch(dir: &Path, script: &str) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
        .arg("--no-connect")
        .arg("--data-dir")
        .arg(dir.join("data"))
        .arg("batch")
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            use std::io::Write;
            c.stdin.as_mut().unwrap().write_all(script.as_bytes())?;
            c.wait_with_output()
        })
        .expect("run the binary");
    (
        out.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

const SRC: &str = "@prefix : <http://ex.org/> .\n\
                   @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
                   :A rdfs:subClassOf :B .\n:B rdfs:subClassOf :C .\n:a a :A .\n";

fn write(dir: &Path, name: &str, body: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p.to_str().unwrap().to_string()
}

#[test]
fn preserve_reports_a_preserved_claim_and_a_lost_one() {
    if skip() {
        return;
    }
    let d = tempfile::tempdir().unwrap();
    let src = write(d.path(), "src.ttl", SRC);
    let whole = write(d.path(), "whole.ttl", SRC);
    // The slice that drops the link the goal needs.
    let partial = write(
        d.path(),
        "partial.ttl",
        "@prefix : <http://ex.org/> .\n\
         @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
         :A rdfs:subClassOf :B .\n:a a :A .\n",
    );
    let goals = write(
        d.path(),
        "goals.ttl",
        "@prefix : <http://ex.org/> .\n:a a :C .\n",
    );

    let (code, out) = batch(
        d.path(),
        &format!(
            "load {src}\npreserve --projection {whole} --goals {goals} --out {}/w --profile owl-rl-ext\n",
            d.path().display()
        ),
    );
    assert_eq!(code, 0, "a preserved claim must exit 0: {out}");
    assert!(out.contains("preserved_checked"), "{out}");
    assert!(
        out.contains("OOCert.certificate_sound"),
        "the theorem must be named on a checked verdict: {out}"
    );

    let (code, out) = batch(
        d.path(),
        &format!(
            "load {src}\npreserve --projection {partial} --goals {goals} --out {}/p --profile owl-rl-ext\n",
            d.path().display()
        ),
    );
    assert_eq!(code, 1, "a lost claim must fail the process: {out}");
    assert!(out.contains("lost_under_profile_unchecked"), "{out}");
    assert!(
        out.contains("is neither necessary nor sufficient"),
        "the coverage label travels with every report: {out}"
    );
}

#[test]
fn preserve_refuses_both_projection_forms_at_once_and_neither() {
    if skip() {
        return;
    }
    let d = tempfile::tempdir().unwrap();
    let src = write(d.path(), "src.ttl", SRC);
    let goals = write(d.path(), "goals.ttl", "@prefix : <http://ex.org/> .\n:a a :C .\n");
    let (code, out) = batch(
        d.path(),
        &format!(
            "load {src}\npreserve --goals {goals} --out {}/n --profile owl-rl-ext\n",
            d.path().display()
        ),
    );
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("exactly one of --projection"), "{out}");
}

#[test]
fn closure_diff_runs_and_names_what_was_lost() {
    if skip() {
        return;
    }
    let d = tempfile::tempdir().unwrap();
    let src = write(d.path(), "src.ttl", SRC);
    // `:X rdfs:subClassOf :C` puts `:C` in the slice's VOCABULARY without
    // putting `:B rdfs:subClassOf :C` in its closure, so `:a rdf:type :C` is a
    // conclusion the answer could have rested on and did not survive. Without
    // that line the loss is real but outside the slice's own vocabulary, which
    // is the partition the headline exists to make and is correctly not a
    // failure.
    let slice = write(
        d.path(),
        "slice.ttl",
        "@prefix : <http://ex.org/> .\n\
         @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
         :A rdfs:subClassOf :B .\n:a a :A .\n:X rdfs:subClassOf :C .\n",
    );
    let (code, out) = batch(
        d.path(),
        &format!(
            "load {src}\nclosure-diff --projection {slice} --out {}/d --profile owl-rl-ext\n",
            d.path().display()
        ),
    );
    assert_eq!(code, 1, "a conclusion lost in the slice's own vocabulary exits 1: {out}");
    assert!(out.contains("oo-closure-diff/1"), "{out}");
    assert!(out.contains("lost_in_projection_vocabulary"), "{out}");
    assert!(
        out.contains("gaming direction"),
        "the headline's own gaming direction is in the payload: {out}"
    );
    assert!(
        out.contains("\"verdict\":\"checked\""),
        "the source certificate must be checked when lake is present: {out}"
    );
    assert!(
        out.contains("\"warrant\":\"asserted_in_source\""),
        "a lookup must be reported as a lookup, never folded into the checked count: {out}"
    );
}
