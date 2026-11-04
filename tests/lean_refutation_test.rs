// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! `oo-refute/1`: certifying that a graph has NO model.
//!
//! Seventeen rules in the OWL 2 RL profile conclude `false` rather than a
//! triple; `lean/OOCert/Refute.lean` lists all seventeen with the W3C tables
//! they come from. The derivation certificate cannot express that, because
//! `Step.conclusion` is a `Triple` and `OOCert.certificate_sound` concludes
//! `Entails`. There is no triple to conclude. `Refute.lean` adds the second
//! format, with `cax-dw` as its one rule, and `OOCert.refutation_sound` proved
//! about it. Of the sixteen it does not implement, fifteen are expressible in
//! that layer and absent; `dt-not-type` is not expressible there at all,
//! because `OOCert.Semantics` has no datatype value space, which is the same
//! ground on which `the_rules_that_were_left_out_stay_out` in
//! `reason_rl_coverage_test.rs` keeps the whole `dt-*` family out of the
//! engine.
//!
//! # The two things this file exists to protect
//!
//! **A refutation must be able to fail.** Six forgeries are checked, one per
//! way to lie: an invented disjointness axiom, a premise whose predicate is not
//! `owl:disjointWith`, two class memberships about different individuals, a
//! forged derivation prefix, a missing prefix so that a premise was never
//! derived, and the right three premises in the wrong order. A seventh case is
//! a graph that is genuinely consistent, where the refutation must be rejected
//! because there is nothing to find. `OOCert.feed_is_not_refuted` proves that
//! rejection is correct rather than a limitation.
//!
//! **The verdict must not outrun what was proved.** `Unsat G` means no
//! interpretation satisfies `Model I G` AND `RefuteConditions I`. It is
//! unsatisfiability relative to reading `owl:disjointWith` as disjointness, not
//! absolute unsatisfiability, and the report says so.
//!
//! # The trap, which is why `guard` exists
//!
//! Over a refuted graph every triple is entailed under the disjointness-aware
//! reading, so a derivation certificate carries no information. But `oo-cert`
//! still reports `"ok":true` for it, and what it reports is TRUE: its verdict
//! quantifies over `OOCert.Model I G` alone, a class that is never empty
//! because `Conditions` has no negative condition and cannot notice a clash.
//! `the_old_checker_still_says_yes_to_the_refuted_graph` runs both binaries on
//! the same two files and pins the contradiction between their verdicts, so the
//! contrast is computed here rather than asserted in a docstring.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn lean_dir() -> PathBuf {
    repo().join("lean")
}

fn fixture(name: &str) -> PathBuf {
    repo().join("tests").join("fixtures").join("refute").join(name)
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

/// Build once per test binary.
///
/// `oo-refute` is NOT in the lakefile's `defaultTargets`, so a bare `lake build`
/// does not build it and does not typecheck `OOCert/Refute.lean` or
/// `OOCert/RefuteWitness.lean` either. Naming the target here is therefore the
/// only thing that puts those proofs, and the `#guard_msgs` axiom tripwires in
/// them, in front of a compiler. If this line is ever changed to a bare
/// `lake build`, the refutation layer stops being checked and nothing else
/// reports it.
fn built() -> &'static PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT.get_or_init(|| {
        for target in ["oo-refute", "oo-cert"] {
            let out = Command::new("lake")
                .arg("build")
                .arg(target)
                .current_dir(lean_dir())
                .output()
                .expect("run lake build");
            assert!(
                out.status.success(),
                "lake build {target} failed:\n{}\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
        }
        lean_dir().join(".lake").join("build").join("bin")
    })
}

fn bin(name: &str) -> PathBuf {
    let exe = built().join(name);
    assert!(exe.exists(), "binary missing at {}", exe.display());
    exe
}

fn refute_bin() -> PathBuf {
    bin("oo-refute")
}

/// Returns (exit code, stdout, stderr).
fn run(exe: &Path, args: &[&Path]) -> (i32, String, String) {
    let mut cmd = Command::new(exe);
    for a in args {
        cmd.arg(a);
    }
    let out = cmd.output().expect("run the checker");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

fn check(asserted: &str, refutation: &str) -> (i32, String, String) {
    let (g, r) = (fixture(asserted), fixture(refutation));
    run(&refute_bin(), &[Path::new("check"), &g, &r])
}

fn guard(asserted: &str, derivations: &str, refutation: &str) -> (i32, String, String) {
    let (g, d, r) = (fixture(asserted), fixture(derivations), fixture(refutation));
    run(&refute_bin(), &[Path::new("guard"), &g, &d, &r])
}

fn field<'a>(json: &'a str, key: &str) -> &'a str {
    let pat = format!("\"{key}\":\"");
    let start = json.find(&pat).unwrap_or_else(|| panic!("no {key} in {json}")) + pat.len();
    let rest = &json[start..];
    &rest[..rest.find('"').expect("unterminated")]
}

#[test]
fn a_real_refutation_is_accepted() {
    if skip() {
        return;
    }
    let (code, out, _) = check("asserted.tsv", "good.tsv");
    assert_eq!(code, 0, "{out}");
    assert_eq!(field(&out, "verdict"), "unsatisfiable_under_disjointness", "{out}");
    assert_eq!(field(&out, "theorem"), "OOCert.refutation_fast_sound", "{out}");
    assert_eq!(field(&out, "rule"), "cax-dw", "{out}");
    // The clash is not visible in the asserted triples. `leo rdf:type Carnivore`
    // has to be derived by rdfs9 first, which is why a refutation carries a
    // derivation prefix at all.
    assert!(out.contains("\"prefix\":1"), "the refutation must use its derivation prefix: {out}");
}

#[test]
fn the_verdict_never_claims_absolute_unsatisfiability() {
    // `Unsat G` is relative to `RefuteConditions`. A report that dropped the
    // qualifier would be claiming more than `OOCert.refutation_sound` proves,
    // and `OOCert.saturated_is_a_model` shows the unqualified claim is false:
    // every graph, this one included, still has a model of `Model I G` alone.
    if skip() {
        return;
    }
    let (_, out, _) = check("asserted.tsv", "good.tsv");
    let means = field(&out, "means");
    assert!(
        means.contains("RELATIVE TO"),
        "the verdict must say what it is conditional on: {out}"
    );
    assert!(
        means.contains("owl:disjointWith"),
        "and name the reading it is conditional on: {out}"
    );
}

#[test]
fn every_forgery_is_rejected() {
    if skip() {
        return;
    }
    // Each is a different way to lie, and each must be caught on its own.
    let forgeries = [
        ("bad_unasserted_axiom.tsv", "the disjointness axiom it cites is not in the graph"),
        ("bad_wrong_predicate.tsv", "the first premise is a subClassOf triple, not a disjointness axiom"),
        ("bad_two_individuals.tsv", "the two class memberships are about different individuals"),
        ("bad_prefix.tsv", "the rdfs9 step in the prefix does not conclude what it claims"),
        ("bad_missing_prefix.tsv", "a premise was neither asserted nor derived by an earlier step"),
        ("bad_order.tsv", "the premises are in the wrong order"),
    ];
    for (name, why) in forgeries {
        let (code, out, err) = check("asserted.tsv", name);
        assert_eq!(code, 1, "{name} must be rejected because {why}: {out}{err}");
        assert!(out.contains("\"ok\":false"), "{name}: {out}");
        // A rejection must not be reported as a consistency result.
        assert!(
            field(&out, "means").contains("not a proof of consistency"),
            "{name}: a rejected refutation says nothing about consistency and the report must \
             say so: {out}"
        );
    }
}

#[test]
fn a_consistent_graph_is_not_refuted() {
    // The same refutation, over the same schema, minus the one assertion that
    // makes the clash. `OOCert.feed_is_not_refuted` proves this graph really
    // does have a model satisfying both `Model` and `RefuteConditions`, so the
    // rejection below is correct and not a gap in the checker.
    if skip() {
        return;
    }
    let (code, out, _) = check("consistent_asserted.tsv", "consistent_attempt.tsv");
    assert_eq!(code, 1, "a consistent graph must not be refutable: {out}");
}

#[test]
fn a_malformed_refutation_is_exit_two_not_a_rejection() {
    // Exit 1 means "the checker rejected a step". A file it could not read is a
    // different answer and must not be reported as the first one, or a harness
    // written to the documented contract reads an unparseable file as a checked
    // rejection.
    if skip() {
        return;
    }
    let malformed = [
        ("bad_no_version.tsv", "a derivations.tsv is not a refutation"),
        ("bad_no_refute_line.tsv", "a refutation with no contradiction step proves nothing"),
        ("bad_refute_not_last.tsv", "the contradiction step must be last"),
        ("bad_unknown_rule.tsv", "the rule it cites is not implemented here"),
    ];
    for (name, why) in malformed {
        let (code, _, err) = check("asserted.tsv", name);
        assert_eq!(code, 2, "{name} must be exit 2 because {why}: {err}");
    }
    // The unimplemented rule is named rather than reported as bad syntax: the
    // honest answer is that this checker has no semantic condition for it.
    let (_, _, err) = check("asserted.tsv", "bad_unknown_rule.tsv");
    assert!(err.contains("cax-adc"), "the refused rule must be named: {err}");
    assert!(
        err.contains("cax-dw"),
        "and the report must say which rules ARE implemented: {err}"
    );
}

#[test]
fn an_unreadable_file_is_exit_two() {
    if skip() {
        return;
    }
    let (code, _, _) = run(
        &refute_bin(),
        &[
            Path::new("check"),
            Path::new("/nonexistent/a.tsv"),
            Path::new("/nonexistent/b.tsv"),
        ],
    );
    assert_eq!(
        code, 2,
        "exit 1 means a step was rejected; an unreadable file must not look like that"
    );
}

#[test]
fn the_old_checker_still_says_yes_to_the_refuted_graph() {
    // THE TRAP, computed rather than described. `derivations.tsv` is a valid
    // derivation certificate over a graph that `oo-refute` refutes. `oo-cert`
    // accepts it and is not wrong: its verdict is `OOCert.Entails`, which
    // quantifies over `OOCert.Model I G`, and that class is never empty
    // (`OOCert.saturated_is_a_model`). The guarantee a consumer wants is the
    // disjointness-aware one, and over this graph that one is empty
    // (`OOCert.a_certificate_adds_nothing_when_the_graph_is_refuted`).
    if skip() {
        return;
    }
    let (g, d) = (fixture("asserted.tsv"), fixture("derivations.tsv"));
    let (cert_code, cert_out, _) = run(&bin("oo-cert"), &[&g, &d]);
    assert_eq!(
        cert_code, 0,
        "the derivation certificate must still be accepted by oo-cert, which is the whole \
         problem: {cert_out}"
    );
    assert!(cert_out.contains("\"ok\":true"), "{cert_out}");

    let (ref_code, ref_out, _) = check("asserted.tsv", "good.tsv");
    assert_eq!(ref_code, 0, "and the same graph must be refutable: {ref_out}");
}

#[test]
fn the_guard_refuses_a_certificate_over_a_graph_it_can_refute() {
    if skip() {
        return;
    }
    let (code, out, _) = guard("asserted.tsv", "derivations.tsv", "good.tsv");
    assert_eq!(code, 1, "the guard must refuse: {out}");
    assert!(
        out.contains("\"certificate_ok\":true"),
        "the certificate itself checks, which is exactly why the refusal has to come from \
         somewhere else: {out}"
    );
    assert!(out.contains("\"refuted\":true"), "{out}");
    assert_eq!(field(&out, "verdict"), "certificate_refused_graph_is_unsatisfiable", "{out}");
    assert_eq!(
        field(&out, "theorem"),
        "OOCert.a_certificate_adds_nothing_when_the_graph_is_refuted",
        "{out}"
    );
}

#[test]
fn the_guard_passes_when_nothing_is_refuted() {
    // And it must not refuse everything, or it is a gate that always fires,
    // which is as useless as one that never does.
    if skip() {
        return;
    }
    let (code, out, _) =
        guard("consistent_asserted.tsv", "consistent_derivations.tsv", "consistent_attempt.tsv");
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("\"refuted\":false"), "{out}");
    assert_eq!(field(&out, "theorem"), "OOCert.certificate_sound", "{out}");
    // A pass is not a consistency proof. Only one of the seventeen OWL 2 RL
    // rules that conclude `false` is implemented, so "no clash found" is the
    // honest wording and the report must use it.
    assert!(
        field(&out, "means").contains("not a proof of consistency"),
        "the passing verdict must not be read as consistency: {out}"
    );
}
