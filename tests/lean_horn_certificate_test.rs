// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! The generic Horn-rule certificate layer.
//!
//! The checker in `lean/OOCert/Rules.lean` has one arm per built-in rule, which
//! does not scale to rules a user writes, and rules a user writes are the whole
//! logic-programming family: RIF Core, Datalog, SWRL. `lean/OOCert/Horn.lean`
//! replaces the per-rule arms with a single generic one. A certificate cites a
//! rule by index into a supplied table, gives a binding, and the checker
//! verifies that the binding instantiates the rule's body into known triples
//! and its head into the claimed conclusion. `OOCert.horn_certificate_sound` is
//! proved once, for every rule table at once.
//!
//! # The distinction these tests exist to protect
//!
//! Two runs of the same checker prove different things.
//!
//! Over the BUILT-IN table, `Builtin.asHorn_sound` discharges every rule against
//! the semantics, so the result is absolute entailment: true in every model of
//! the asserted graph. `OOCert.entails_of_builtin_horn`.
//!
//! Over a table the user supplied, nothing discharges the rules. They are
//! assumptions the certificate carries, so the result is relative: true in every
//! model of the graph that ALSO satisfies those rules.
//! `OOCert.horn_certificate_sound`. A rule reading "every supplier is compliant"
//! makes certificates that check green for ever, and the certificate certifies
//! the inference, never the premises.
//!
//! Collapsing those into one verdict turns a verified checker into a device for
//! laundering an assumption into a fact. `a_user_rule_never_earns_the_absolute_verdict`
//! is the test that fails if that ever happens.

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
    repo().join("tests").join("fixtures").join("horn").join(name)
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

/// Build once per test binary. The proofs are part of the build, so a failure
/// here is a failure and never a skip.
fn checker() -> &'static Path {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT.get_or_init(|| {
        let out = Command::new("lake")
            .arg("build")
            .current_dir(lean_dir())
            .output()
            .expect("run lake build");
        assert!(
            out.status.success(),
            "lake build failed:\n{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let exe = lean_dir().join(".lake").join("build").join("bin").join("oo-horn");
        assert!(exe.exists(), "checker binary missing at {}", exe.display());
        exe
    })
}

/// Returns (exit code, stdout).
fn check(rules: &str, asserted: &str, cert: &str) -> (i32, String) {
    let out = Command::new(checker())
        .arg("check")
        .arg(fixture(rules))
        .arg(fixture(asserted))
        .arg(fixture(cert))
        .output()
        .expect("run oo-horn");
    (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stdout).to_string())
}

fn field<'a>(json: &'a str, key: &str) -> &'a str {
    let pat = format!("\"{key}\":\"");
    let start = json.find(&pat).unwrap_or_else(|| panic!("no {key} in {json}")) + pat.len();
    let rest = &json[start..];
    &rest[..rest.find('"').expect("unterminated")]
}

#[test]
fn the_built_in_table_earns_the_absolute_verdict() {
    if skip() {
        return;
    }
    let (code, out) = check("builtin_rules.tsv", "asserted.tsv", "good.tsv");
    assert_eq!(code, 0, "{out}");
    assert_eq!(field(&out, "verdict"), "entailed", "{out}");
    assert_eq!(field(&out, "theorem"), "OOCert.entails_of_builtin_horn", "{out}");
    assert_eq!(
        field(&out, "rules_digest"),
        field(&out, "builtin_rules_digest"),
        "the table that earns the absolute verdict must BE the built-in table: {out}"
    );
}

#[test]
fn a_user_rule_never_earns_the_absolute_verdict() {
    // The laundering guard. `user_rules.tsv` is the built-in table plus one
    // invented rule asserting that every supplier is compliant. The same
    // certificate still checks, because the inference is still valid, and the
    // verdict must say what it is relative to.
    if skip() {
        return;
    }
    let (code, out) = check("user_rules.tsv", "asserted.tsv", "good.tsv");
    assert_eq!(code, 0, "{out}");
    assert_ne!(
        field(&out, "verdict"),
        "entailed",
        "a run over user-supplied rules must NOT report absolute entailment: {out}"
    );
    assert_eq!(field(&out, "verdict"), "entailed_under_supplied_rules", "{out}");
    assert_eq!(field(&out, "theorem"), "OOCert.horn_certificate_sound", "{out}");
    assert_ne!(
        field(&out, "rules_digest"),
        field(&out, "builtin_rules_digest"),
        "the digest must show the table differed from the built-ins: {out}"
    );
    assert!(
        field(&out, "means").contains("assumed, not checked"),
        "the report must say the rules are assumed: {out}"
    );
}

#[test]
fn every_forgery_is_rejected() {
    if skip() {
        return;
    }
    // Each one is a different way to lie, and each must be caught on its own.
    let forgeries = [
        ("bad_conclusion.tsv", "the conclusion is not the rule's head under the binding"),
        ("bad_index.tsv", "the cited rule index is outside the table"),
        ("bad_premise.tsv", "a premise is neither asserted nor derived earlier"),
        ("bad_self.tsv", "the step cites its own conclusion as a premise"),
        ("bad_binding.tsv", "the binding does not instantiate the body"),
    ];
    for (name, why) in forgeries {
        let (code, out) = check("builtin_rules.tsv", "asserted.tsv", name);
        assert_eq!(code, 1, "{name} must be rejected because {why}: {out}");
    }
    // And a table too short to contain the cited rule.
    let (code, out) = check("short_rules.tsv", "asserted.tsv", "good.tsv");
    assert_eq!(code, 1, "a rule table missing the cited rule must be rejected: {out}");
}

#[test]
fn an_unreadable_file_is_exit_two_not_a_rejection() {
    if skip() {
        return;
    }
    let out = Command::new(checker())
        .arg("check")
        .arg("/nonexistent/rules.tsv")
        .arg("/nonexistent/asserted.tsv")
        .arg("/nonexistent/horn.tsv")
        .output()
        .expect("run oo-horn");
    assert_eq!(
        out.status.code(),
        Some(2),
        "exit 1 means a step was rejected; an unreadable file must not look like that"
    );
}

#[test]
fn the_built_in_rules_are_emitted_as_data() {
    if skip() {
        return;
    }
    let out = Command::new(checker()).arg("rules").output().expect("run oo-horn rules");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        27,
        "twenty-seven of the engine's rules are Horn rules. Four are not and stay as hardcoded \
         arms: cls-int1, cls-int2, cls-uni and cls-oo all read an RDF list off the graph, so the \
         LIST is a premise and the premise count is data rather than fixed by the rule. That is \
         the boundary of the Horn family, not a gap in it. If this number changed, say which rule \
         moved and why."
    );
    // The table printed here is the one the absolute verdict is pinned to, so it
    // must match the committed fixture byte for byte.
    let committed = std::fs::read_to_string(fixture("builtin_rules.tsv")).unwrap();
    assert_eq!(
        text.trim_end(),
        committed.trim_end(),
        "the committed rule table has drifted from the one the checker computes"
    );
}
