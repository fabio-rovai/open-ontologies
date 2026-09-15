//! `fol-prove` through the real binary, because an exit code that does not
//! bite is the same defect as a check that does not run.
//!
//! Decision 0006 records this exact failure happening in this repository: the
//! certifying pipeline exited non-zero on a stop-the-line only on the direct
//! CLI path, while `batch`, the mode every tool in `tools/` uses, decided the
//! exit code from the presence of an `"error"` key and a stop-the-line is not
//! an error. So in the one place the gate had to bite, it did not. These tests
//! drive the process and read the exit status rather than calling the library,
//! for that reason and no other.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture(name: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tstp").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("oo-tstp-cli-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("scratch dir");
    d
}

/// Run `fol-prove --problem … --proof …` and return (exit code, stdout).
fn check_pair(dir: &Path, problem: &str, proof: &str) -> (i32, String) {
    let p = dir.join("problem.p");
    let d = dir.join("proof.tstp");
    std::fs::write(&p, problem).unwrap();
    std::fs::write(&d, proof).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
        .args(["--no-connect", "fol-prove", "--problem"])
        .arg(&p)
        .arg("--proof")
        .arg(&d)
        .output()
        .expect("the binary runs");
    (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stdout).to_string())
}

/// The positive control. Without it every test below would pass on a command
/// that exited 1 on everything.
#[test]
fn a_genuine_pair_exits_zero_and_reports_what_it_replayed() {
    let d = scratch("good");
    let (code, out) = check_pair(&d, &fixture("problem.p"), &fixture("vampire-5.1.0.tstp"));
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("\"verdict\":\"refutation_partially_replayed\""), "{out}");
    assert!(out.contains("\"leaves_match_problem\":true"), "{out}");
    // The report names the file it is about, so a log line is self-contained.
    assert!(out.contains("\"problem\":"), "{out}");
    assert!(out.contains("\"proof\":"), "{out}");
}

/// A mutated leaf must fail the process, not merely be mentioned in a field
/// nobody reads. This is the gate `tools/fol_differential.py` now rests on.
#[test]
fn a_rejected_derivation_exits_one() {
    let d = scratch("bad");
    let proof = fixture("vampire-5.1.0.tstp").replace(
        "'c:http://example.org/Greek'(X0) => 'c:http://example.org/Person'(X0)",
        "'c:http://example.org/Greek'(X0) => 'c:http://example.org/Mortal'(X0)",
    );
    let (code, out) = check_pair(&d, &fixture("problem.p"), &proof);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(code, 1, "a rejected derivation must fail a pipeline.\n{out}");
    assert!(out.contains("\"verdict\":\"derivation_rejected\""), "{out}");
    assert!(out.contains("\"stop_the_line\":1"), "{out}");
}

/// A prover that answered without printing a proof leaves a word and nothing
/// to check. That is not a rejection and must not exit 1: it would turn every
/// timeout into a build failure.
#[test]
fn an_output_with_no_derivation_exits_zero_and_says_why() {
    let d = scratch("noproof");
    let (code, out) = check_pair(&d, &fixture("problem.p"), "% SZS status Theorem for x\n");
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("\"verdict\":\"derivation_unparsed\""), "{out}");
    assert!(out.contains("--proof-object"), "the fix is named in the output: {out}");
}

/// The subcommand refuses to do half the job. Without `--out` and without a
/// recorded pair there is nothing to run and nothing to check, and saying so
/// is better than running the prover over an empty store.
#[test]
fn fol_prove_with_neither_a_directory_nor_a_pair_is_refused() {
    let out = Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
        .args(["--no-connect", "fol-prove"])
        .output()
        .expect("the binary runs");
    assert_ne!(out.status.code(), Some(0));
    let text = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(text.contains("--out") || text.contains("problem"), "{text}");
}

/// A missing prover is reported loudly and is never worked around. G6: the
/// absence of a second opinion is not agreement with one.
#[test]
fn a_missing_prover_is_loud() {
    let d = scratch("missing");
    // An empty PATH makes every prover missing, which is the only portable way
    // to drive this branch on a machine that has one installed.
    let out = Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
        .args(["--no-connect", "--data-dir"])
        .arg(d.join("store"))
        .args(["fol-prove", "--out"])
        .arg(d.join("run"))
        .args(["--prover", "eprover"])
        .env("PATH", "")
        .output()
        .expect("the binary runs");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let _ = std::fs::remove_dir_all(&d);
    assert!(text.contains("\"skipped\""), "{text}");
    assert!(text.contains("not on PATH"), "{text}");
    assert!(
        text.contains("ABSENCE of a second opinion"),
        "a skip must not read as agreement: {text}"
    );
}
