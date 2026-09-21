//! **Issue #162.** The README's runnable example is run.
//!
//! "Try the checker itself" prints three `oo-horn check` invocations and the
//! JSON each returned. Every line of it was produced by running the commands,
//! and nothing re-ran them afterwards, so the section could drift from the
//! checker without anything noticing. A reader who pastes a command and gets a
//! different answer than the page promises has been told something false by
//! the file that exists to be trusted.
//!
//! The commands and the expected output are READ OUT OF THE README rather than
//! written here. Hardcoding them would gate this file against itself: the
//! README could change and the test would keep passing against its own copy.
//! What is asserted is that the fixtures the README names, fed to the checker
//! the README names, produce the fields the README prints, with the exit codes
//! it claims.
//!
//! The third invocation is the one worth having. Same inference, a rule the
//! user supplied, and the verdict word MUST change from `entailed` to
//! `entailed_under_supplied_rules`. That difference is the line between a
//! verification layer and a device for laundering an assumption into a fact,
//! and the README says a test fails if it ever stops changing. This is that
//! test.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn readme() -> String {
    std::fs::read_to_string(repo().join("README.md")).expect("read README.md")
}

/// The fenced block that runs the checker.
///
/// Found by what it CONTAINS, not by the heading above it. Anchoring on the
/// title meant that renaming the section broke this test, which is the one test
/// whose job is to notice the example drifting from the checker; a heading is
/// prose and prose is allowed to change. What may not change without failing
/// here is that the README still shows somebody running `oo-horn check`.
fn example_block() -> String {
    let r = readme();
    let mut at = 0usize;
    while let Some(i) = r[at..].find("```bash") {
        let open = at + i + 7;
        let Some(len) = r[open..].find("```") else { break };
        let block = &r[open..open + len];
        if block.contains("lake exe oo-horn check") {
            return block.to_string();
        }
        at = open + len + 3;
    }
    panic!(
        "no fenced bash block in README.md runs `lake exe oo-horn check`. The runnable \
         example is the one part of the page a reader can paste, and this test exists so \
         that it cannot drift from the checker unnoticed."
    );
}

/// Each `oo-horn check` line in the block, as its three fixture arguments.
fn invocations() -> Vec<[String; 3]> {
    let block = example_block();
    let mut out = Vec::new();
    for line in block.lines() {
        let line = line.trim();
        let Some(args) = line.strip_prefix("$ lake exe oo-horn check ") else { continue };
        let parts: Vec<String> = args
            .split_whitespace()
            .map(|a| a.replace("$F", "tests/fixtures/horn"))
            .collect();
        assert_eq!(parts.len(), 3, "an oo-horn check line should name three files: {line}");
        out.push([parts[0].clone(), parts[1].clone(), parts[2].clone()]);
    }
    assert_eq!(
        out.len(),
        3,
        "the README section should carry three invocations; found {}. If the section was \
         rewritten, this test needs to follow it rather than be deleted",
        out.len()
    );
    out
}

fn lean_dir() -> PathBuf {
    repo().join("lean")
}

fn lake_available() -> bool {
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

fn checker() -> &'static Path {
    static BUILT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    BUILT.get_or_init(|| {
        let out = Command::new("lake")
            .arg("build")
            .current_dir(lean_dir())
            .output()
            .expect("run lake build");
        assert!(out.status.success(), "lake build failed:\n{}", String::from_utf8_lossy(&out.stderr));
        let exe = lean_dir().join(".lake").join("build").join("bin").join("oo-horn");
        assert!(exe.exists(), "oo-horn missing at {}", exe.display());
        exe
    })
}

/// Run one invocation exactly as the README spells it. Returns (success, output).
fn run(args: &[String; 3]) -> (bool, String) {
    let out = Command::new(checker())
        .arg("check")
        .args(args.iter().map(|a| repo().join(a)))
        .output()
        .expect("run oo-horn");
    (
        out.status.success(),
        format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)),
    )
}

#[test]
fn the_readme_example_runs_and_says_what_the_readme_says() {
    if skip() {
        return;
    }
    let inv = invocations();
    let block = example_block();

    // 1. Built-in rules over an honest certificate: accepted, absolute verdict.
    let (ok, out) = run(&inv[0]);
    assert!(ok, "the first command should exit 0:\n{out}");
    for fragment in ["\"ok\":true", "\"verdict\":\"entailed\"", "OOCert.entails_of_builtin_horn"] {
        assert!(out.contains(fragment), "the README shows {fragment}, the checker printed:\n{out}");
        assert!(block.contains(fragment), "the README stopped showing {fragment}");
    }

    // 2. A forged conclusion: refused, non-zero.
    let (ok, out) = run(&inv[1]);
    assert!(!ok, "the forged certificate must exit non-zero:\n{out}");
    assert!(out.contains("\"ok\":false"), "expected a refusal:\n{out}");
    assert!(block.contains("\"ok\":false"), "the README stopped showing the refusal");

    // 3. The same inference under a rule the user supplied. The verdict word
    //    MUST be the weaker one, and the theorem MUST be the conditional one.
    let (ok, out) = run(&inv[2]);
    assert!(ok, "the third command should exit 0:\n{out}");
    assert!(
        out.contains("\"verdict\":\"entailed_under_supplied_rules\""),
        "a certificate over rules the user supplied must NOT earn the absolute verdict. This is \
         the difference between a verification layer and a device for laundering an assumption \
         into a fact:\n{out}"
    );
    assert!(
        !out.contains("\"verdict\":\"entailed\""),
        "the third run reported the absolute verdict:\n{out}"
    );
    assert!(out.contains("OOCert.horn_certificate_sound"), "expected the conditional theorem:\n{out}");
}

/// The fixtures the README names exist. Without this the test above would fail
/// with a checker error rather than saying the page points at nothing.
#[test]
fn every_file_the_readme_example_names_exists() {
    for inv in invocations() {
        for arg in inv {
            assert!(
                repo().join(&arg).is_file(),
                "the README example names {arg}, which is not in the repository"
            );
        }
    }
}
