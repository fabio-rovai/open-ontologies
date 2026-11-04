// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! `oo-folmodel` end to end: the shipped fixtures, and the engine's own output.
//!
//! `lean/Fol/` carries the theorem. `Fol.satisfiable_of_check` says an
//! accepted structure really is a model, so the problem it was checked against
//! really is satisfiable, and `Fol.not_entails_of_check` turns a checked model
//! of `¬φ :: Γ` into a machine-checked NON-ENTAILMENT — the one sentence no
//! theorem prover can produce, because a prover answering "not a theorem" is
//! reporting a failed search at best.
//!
//! Until this file existed the Lean side had never been run against anything
//! the Rust side emitted. Decision 0006 recorded that gap in its own "Still
//! not done": *nothing has been emitted by the engine and checked end to end*.
//! These tests close it, in three groups:
//!
//!   1. **The shipped fixtures**, with the exit codes measured when they were
//!      written. Ten pairs, four of them G3 forgeries at the FILE level.
//!   2. **The engine's own output**, from a real ontology fixture through the
//!      exporter, the SMT-LIB printer, Z3, the ingestion and back into the
//!      checker.
//!   3. **Forging the engine's output**, one bit at a time, so that the
//!      acceptance in group 2 is shown to be a decision rather than a habit.
//!
//! The checker needs a Lean toolchain (`lake`) and the `oo-folmodel` target,
//! which `lean/lakefile.toml` carries in `defaultTargets` so a bare
//! `lake build` compiles it. Without them these tests SKIP LOUDLY through
//! `common::skip_unless`; the CI job that installs Lean runs them with
//! `OO_REQUIRE_FIXTURES=1`, which turns that skip into a failure.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use open_ontologies::fol_model::z3;
use open_ontologies::fol_solve::{Solver, find_checker};
use open_ontologies::tptp::{Concept, FolProblem, OwlAxiom, smtlib::SmtEncoding};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn lean_dir() -> PathBuf {
    std::env::var("OO_LEAN_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| repo().join("lean"))
}

fn fixture(name: &str) -> PathBuf {
    repo().join("tests/fixtures/folmodel").join(name)
}

fn lake_available() -> bool {
    // `.current_dir(lean_dir())` is not cosmetic: elan resolves the toolchain
    // from the working directory's `lean-toolchain`, and the crate root has
    // none in its ancestry. The same probe as `tests/dl_model_certificate_test.rs`.
    Command::new("lake")
        .arg("--version")
        .current_dir(lean_dir())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Build once per test binary. The PROOFS are part of the build, so a failure
/// here is a failure and never a skip.
fn build() -> &'static Result<PathBuf, String> {
    static BUILT: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    BUILT.get_or_init(|| {
        if !lake_available() {
            return Err("lake (the Lean 4 build tool); install elan from \
                        https://github.com/leanprover/elan, and lean/lean-toolchain pins the \
                        version"
                .to_string());
        }
        let out = Command::new("lake")
            .arg("build")
            .arg("oo-folmodel")
            .current_dir(lean_dir())
            .output()
            .expect("run lake build oo-folmodel");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        if !out.status.success() {
            if text.contains("unknown target") || text.contains("no such target") {
                return Err(format!(
                    "the `oo-folmodel` target in lean/lakefile.toml, which must also be in \
                     defaultTargets so the CI job's bare `lake build` compiles these proofs. \
                     lake said: {text}"
                ));
            }
            panic!("lake build oo-folmodel failed:\n{text}");
        }
        let exe = lean_dir().join(".lake/build/bin/oo-folmodel");
        if !exe.exists() {
            panic!("checker binary missing at {}", exe.display());
        }
        Ok(exe)
    })
}

fn skip() -> bool {
    match build() {
        Ok(_) => false,
        Err(why) => common::skip_unless(
            false,
            why,
            "the model-certificate tests check nothing without the verified checker",
        ),
    }
}

fn checker() -> &'static Path {
    build().as_ref().expect("checked by skip()").as_path()
}

/// Returns (exit code, stdout).
fn check(problem: &Path, model: &Path) -> (i32, String) {
    let out = Command::new(checker())
        .arg(problem)
        .arg(model)
        .output()
        .expect("run oo-folmodel");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
    )
}

fn field<'a>(json: &'a str, key: &str) -> &'a str {
    let pat = format!("\"{key}\":\"");
    match json.find(&pat) {
        Some(i) => {
            let rest = &json[i + pat.len()..];
            &rest[..rest.find('"').expect("unterminated")]
        }
        None => "",
    }
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("oo-leanfol-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("scratch");
    d
}

// ── 1. The shipped fixtures ─────────────────────────────────────────────────

/// The worked example of `lean/Fol/Syntax.lean`, accepted, naming both
/// theorems because the problem carries a `goal_negated` line.
#[test]
fn the_worked_example_is_accepted() {
    if skip() {
        return;
    }
    let (code, out) = check(&fixture("problem.tsv"), &fixture("model.tsv"));
    assert_eq!(code, 0, "{out}");
    assert_eq!(field(&out, "verdict"), "model_checked", "{out}");
    assert_eq!(field(&out, "theorem"), "Fol.satisfiable_of_check", "{out}");
    assert_eq!(
        field(&out, "non_entailment_theorem"),
        "Fol.not_entails_of_check",
        "the non-entailment is the sentence this layer exists for: {out}"
    );
    assert_eq!(field(&out, "problem_digest"), "4403d8aaa0c422f7", "{out}");
    assert!(out.contains("\"goal_negated_present\":true"), "{out}");
    assert!(out.contains("\"closed\":true"), "{out}");
}

/// G3, at the file level. Every way of lying, each on its own fixture, each
/// with the exit code measured when the fixture was written.
///
/// The three exit codes are not interchangeable. `1` is a REJECTION, which by
/// `Fol.check_complete` is a statement about the structure and not about the
/// checker giving up. `2` is UNREADABLE, which is not a verdict in either
/// direction. `0` is the certificate.
#[test]
fn every_forgery_is_rejected() {
    if skip() {
        return;
    }
    // Rejections: the structure, or the file's claim about which problem it is
    // for, is wrong.
    let rejected: &[(&str, &str, &str)] = &[
        (
            "model_no_edge.tsv",
            "",
            "the worksFor edge the subClassOf axiom requires was deleted",
        ),
        (
            "model_undeclared.tsv",
            "undeclared_symbol",
            "the model does not interpret a symbol the problem uses",
        ),
        (
            "model_wrong_digest.tsv",
            "problem_digest_mismatch",
            "the model claims to be for a different problem",
        ),
    ];
    for (name, reason, why) in rejected {
        let (code, out) = check(&fixture("problem.tsv"), &fixture(name));
        assert_eq!(code, 1, "{name} must be REJECTED because {why}: {out}");
        assert_eq!(field(&out, "verdict"), "rejected", "{out}");
        if !reason.is_empty() {
            assert_eq!(field(&out, "reason"), *reason, "{out}");
        }
    }

    // Unreadable: the file does not describe a structure at all.
    let unreadable: &[(&str, &str)] = &[
        ("model_empty_domain.tsv", "a first-order structure cannot have an empty carrier"),
        ("model_bad_arity.tsv", "a binary row for a symbol declared unary"),
        ("model_index_out_of_range.tsv", "an index outside the declared carrier"),
        ("model_missing_row.tsv", "a declared binary symbol with a missing row"),
    ];
    for (name, why) in unreadable {
        let (code, out) = check(&fixture("problem.tsv"), &fixture(name));
        assert_eq!(code, 2, "{name} must be UNREADABLE because {why}: {out}");
        assert_eq!(field(&out, "verdict"), "unreadable", "{out}");
    }
}

/// The coverage gate says ATTRIBUTION in machine-readable form, so a report
/// cannot quietly promote it into a soundness failure.
///
/// Soundness holds without it: `FinModel`'s three fields are total, an
/// undeclared symbol is still interpreted, and `check` validates the resulting
/// structure from scratch. What the gate buys is that the structure certified
/// is the structure the solver described rather than the solver's completed
/// with defaults.
#[test]
fn the_coverage_gate_is_labelled_attribution() {
    if skip() {
        return;
    }
    let (code, out) = check(&fixture("problem.tsv"), &fixture("model_undeclared.tsv"));
    assert_eq!(code, 1, "{out}");
    assert_eq!(field(&out, "gate"), "attribution", "{out}");
    assert!(!field(&out, "symbol").is_empty(), "it must name the symbol: {out}");
    assert!(!field(&out, "arity").is_empty(), "and its arity: {out}");
}

/// G3.5: a perfectly good structure checked against a problem it was not built
/// for. The digest is what catches it, and the rejection says so.
#[test]
fn a_model_of_another_problem_is_rejected() {
    if skip() {
        return;
    }
    let (code, out) = check(&fixture("problem_other.tsv"), &fixture("model.tsv"));
    assert_eq!(code, 1, "{out}");
    assert_eq!(field(&out, "reason"), "problem_digest_mismatch", "{out}");
    assert_eq!(field(&out, "found"), "4403d8aaa0c422f7", "{out}");
    assert_ne!(field(&out, "expected"), field(&out, "found"), "{out}");
}

/// An EMPTY problem is exit 2 and never a certificate.
///
/// Every structure satisfies the empty formula list, so a certificate over one
/// would report `model_checked` and say nothing. An exporter bug that dropped
/// every formula would otherwise produce a clean green run, which is the exact
/// shape decision 0005 item 7 records happening inside this repository already.
#[test]
fn an_empty_problem_is_unreadable_and_never_certified() {
    if skip() {
        return;
    }
    let (code, out) = check(&fixture("problem_empty.tsv"), &fixture("model.tsv"));
    assert_eq!(code, 2, "{out}");
    assert_ne!(field(&out, "verdict"), "model_checked", "{out}");
    assert_eq!(field(&out, "verdict"), "unreadable", "{out}");
}

/// A file that cannot be opened is exit 2, not a rejection. "I could not read
/// the file" and "this is not a model" are different answers.
#[test]
fn an_unreadable_file_is_exit_two_not_a_rejection() {
    if skip() {
        return;
    }
    let (code, _) = check(Path::new("/nonexistent/p.tsv"), Path::new("/nonexistent/m.tsv"));
    assert_eq!(code, 2);
}

// ── 2. The engine's own output ──────────────────────────────────────────────

/// The whole chain, for real: the exporter builds the problem, the SMT-LIB
/// printer writes it, Z3 solves it, the ingestion reads the structure back,
/// and the VERIFIED CHECKER accepts it against the problem the exporter also
/// wrote in its own format.
///
/// This is the end-to-end run decision 0006 recorded as missing. Everything
/// before it had been measured against files written by hand.
/// Did the solver say `sat`, on ANY line?
///
/// Not `text.starts_with("sat")`, which is what this file used to ask and what
/// broke it on CI. Z3 on Ubuntu is several minor versions behind the one this
/// was written against, and it answers `unsupported` to one of the preamble
/// commands before going on to solve the problem correctly and print the model.
/// So the output begins `unsupported\nsat\n(…)` there and `sat\n(…)` here.
///
/// The engine was already right: `fol_solve::read_sat` scans LINES for a bare
/// status, precisely so a leading `unsupported` or `(error …)` cannot be
/// mistaken for a verdict. The test was stricter than the code it tests, which
/// is its own kind of defect: it failed on a platform where the product works.
fn said_sat(text: &str) -> bool {
    text.lines().any(|l| l.trim() == "sat")
}

#[test]
fn the_engines_own_export_is_solved_and_checked() {
    if skip() {
        return;
    }
    if common::skip_unless(Solver::Z3.available(), "z3", Solver::Z3.install_line()) {
        return;
    }
    let d = scratch("engine");

    // `A(a)` does not entail `B(a)`, so a countermodel exists and the run
    // should reach the non-entailment theorem.
    let axioms = vec![OwlAxiom::ClassAssert(
        Concept::Atom("http://e/A".into()),
        "http://e/a".into(),
    )];
    let goal = OwlAxiom::ClassAssert(Concept::Atom("http://e/B".into()), "http://e/a".into());
    let p = FolProblem::build(&axioms, Some(&goal)).expect("freshness holds");

    let (tsv, digest) = p.to_problem_tsv().expect("writable");
    let problem_path = d.join("problem.tsv");
    std::fs::write(&problem_path, &tsv).expect("write");

    let smt = p.to_smtlib(SmtEncoding::Finite(2)).expect("writable");
    let smt_path = d.join("problem.smt2");
    std::fs::write(&smt_path, smt).expect("write");

    let out = Command::new("z3").arg(&smt_path).output().expect("run z3");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(said_sat(&text), "z3 must find a countermodel: {text}");

    let model = z3::parse_model(&text, &p.vocabulary(), 2).expect("a readable model");
    let model_path = d.join("model.tsv");
    std::fs::write(&model_path, model.to_model_tsv(&digest, &[1, 2])).expect("write");

    let (code, report) = check(&problem_path, &model_path);
    assert_eq!(code, 0, "the checker must accept the engine's own model: {report}");
    assert_eq!(field(&report, "verdict"), "model_checked", "{report}");
    assert_eq!(
        field(&report, "non_entailment_theorem"),
        "Fol.not_entails_of_check",
        "{report}"
    );
    // The digest the Rust computed is the digest the Lean recomputed, which is
    // the whole point of specifying FNV-1a rather than using a library.
    assert_eq!(field(&report, "problem_digest"), digest, "{report}");
    // Every formula the exporter wrote reached the checker.
    assert!(
        report.contains(&format!("\"formulas\":{}", tsv.lines().count())),
        "{report}"
    );
    assert!(report.contains("\"source\":\"z3\""), "{report}");
}

// ── 3. Forging the engine's output ──────────────────────────────────────────

/// The acceptance above is a DECISION, not a habit: one bit of the model the
/// engine wrote, flipped, and the same checker says no.
///
/// Three edits, three different failures. A flipped truth value is a
/// rejection; a deleted declaration is unreadable; a digest of one character
/// different is a rejection naming the mismatch.
#[test]
fn one_edit_to_the_engines_model_is_caught() {
    if skip() {
        return;
    }
    if common::skip_unless(Solver::Z3.available(), "z3", Solver::Z3.install_line()) {
        return;
    }
    let d = scratch("forge");
    // `∀x (A(x) → B(x))` and `A(a)`, so any model must put `a` in B.
    let axioms = vec![
        OwlAxiom::SubClass(
            Concept::Atom("http://e/A".into()),
            Concept::Atom("http://e/B".into()),
        ),
        OwlAxiom::ClassAssert(Concept::Atom("http://e/A".into()), "http://e/a".into()),
    ];
    let p = FolProblem::build(&axioms, None).expect("freshness holds");
    let (tsv, digest) = p.to_problem_tsv().expect("writable");
    let problem_path = d.join("problem.tsv");
    std::fs::write(&problem_path, &tsv).expect("write");
    std::fs::write(
        d.join("p.smt2"),
        p.to_smtlib(SmtEncoding::Finite(2)).expect("writable"),
    )
    .expect("write");
    let text = String::from_utf8_lossy(
        &Command::new("z3").arg(d.join("p.smt2")).output().expect("z3").stdout,
    )
    .to_string();
    assert!(said_sat(&text), "{text}");
    let model = z3::parse_model(&text, &p.vocabulary(), 2).expect("readable");
    let good = model.to_model_tsv(&digest, &[2]);
    std::fs::write(d.join("model.tsv"), &good).expect("write");
    let (code, out) = check(&problem_path, &d.join("model.tsv"));
    assert_eq!(code, 0, "the unforged model must be accepted: {out}");

    // (a) Empty the row for `c:http://e/B`, which the subsumption needs.
    let b_row = good
        .lines()
        .find(|l| l.starts_with("p1\tc:http://e/B\t"))
        .expect("a row for B")
        .to_string();
    let bits = b_row.rsplit('\t').next().expect("bits");
    let forged = good.replace(&b_row, &format!("p1\tc:http://e/B\t{}", "0".repeat(bits.len())));
    assert_ne!(forged, good, "the forgery must change something");
    std::fs::write(d.join("forged_row.tsv"), forged).expect("write");
    let (code, out) = check(&problem_path, &d.join("forged_row.tsv"));
    assert_eq!(code, 1, "a flipped truth value must be rejected: {out}");
    assert_eq!(field(&out, "verdict"), "rejected", "{out}");
    assert_eq!(field(&out, "theorem"), "Fol.check_complete_closed", "{out}");
    assert!(!field(&out, "label").is_empty(), "it must name the formula: {out}");

    // (b) Delete a declaration, leaving its row behind.
    let forged2: String = good
        .lines()
        .filter(|l| *l != "decl1\tthing")
        .map(|l| format!("{l}\n"))
        .collect();
    std::fs::write(d.join("forged_decl.tsv"), forged2).expect("write");
    let (code, out) = check(&problem_path, &d.join("forged_decl.tsv"));
    assert_eq!(code, 2, "a row for an undeclared symbol is unreadable: {out}");

    // (c) One hex digit of the digest.
    let mut ch: Vec<char> = digest.chars().collect();
    ch[15] = if ch[15] == '0' { '1' } else { '0' };
    let forged3 = good.replace(
        &format!("problem\t{digest}"),
        &format!("problem\t{}", ch.iter().collect::<String>()),
    );
    std::fs::write(d.join("forged_digest.tsv"), forged3).expect("write");
    let (code, out) = check(&problem_path, &d.join("forged_digest.tsv"));
    assert_eq!(code, 1, "{out}");
    assert_eq!(field(&out, "reason"), "problem_digest_mismatch", "{out}");
    assert_eq!(field(&out, "expected"), digest, "{out}");
}

/// The checker is where `find_checker` finds it, and it is in `defaultTargets`
/// so a bare `lake build` produces it.
///
/// That second half has bitten this repository once already and the lakefile
/// comment records it: a proof outside the default set is not built by the CI
/// job's bare `lake build`, and a proof that does not compile under a green
/// gate is the exact failure this project exists to catch.
#[test]
fn the_checker_is_in_the_default_build() {
    if skip() {
        return;
    }
    let found = find_checker(None).expect("the pipeline must find the checker it just built");
    assert!(found.is_file());
    let lakefile =
        std::fs::read_to_string(lean_dir().join("lakefile.toml")).expect("read lakefile");
    let defaults = lakefile
        .lines()
        .find(|l| l.trim_start().starts_with("defaultTargets"))
        .expect("a defaultTargets line");
    assert!(defaults.contains("\"oo-folmodel\""), "{defaults}");
    assert!(defaults.contains("\"Fol\""), "{defaults}");
}
