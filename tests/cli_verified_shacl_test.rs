//! The verified SHACL evaluator, from the command line.
//!
//! Issue #203 is titled "the verified evaluator is unreachable from
//! `onto_shacl` AND THE CLI". PR #210 closed the first half: `onto_shacl`
//! grew `verified: true` and the MCP tool can now reach `oo-shacl`, whose
//! agreement with the Recommendation is the machine-checked theorem
//! `Shacl.validate_spec`. The command line could not. A user without an MCP
//! client could not obtain this repository's strongest SHACL answer at all,
//! which is the same defect the issue opened about, surviving in the half of
//! the interface that most people reach for first.
//!
//! `--verified` closes it, on both CLI paths: the single-command one in
//! `main.rs` and the batch executor in `batch.rs`, which are different code
//! and were separately capable of not having it.
//!
//! # Two questions, not one setting
//!
//! The flag does not make the same answer better. The verified evaluator
//! covers SHACL Core and refuses `sh:sparql` and user-defined components
//! outright; the default path runs those and skips others. Neither is a
//! superset, so the reports must be tellable apart, and they are: the verified
//! report carries a `verified` key the default path never emits.
//! `the_default_path_never_says_verified` is that test, and it needs no
//! checker, so it runs everywhere.
//!
//! # The refusal is the interesting half
//!
//! A temporal scope cannot be honoured by an evaluator that reads one
//! N-Triples dump of the whole store. Silently dropping it would answer a
//! different question from the one that was asked, which is precisely the
//! failure mode the unverified path's `skipped_constraints` list exists to
//! prevent. So it is refused, with the reason, and
//! `a_scope_with_verified_is_refused_and_not_dropped` runs without a checker
//! too, because argument handling should not be testable only on machines with
//! a Lean toolchain.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

fn run(dir: &Path, args: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
        .arg("--data-dir")
        .arg(dir.join("data"))
        .args(args)
        .output()
        .expect("the binary runs");
    (
        out.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("oo-cli-vshacl-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn write(dir: &Path, name: &str, body: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p.to_str().unwrap().to_string()
}

const SHAPES: &str = r#"
@prefix sh:  <http://www.w3.org/ns/shacl#> .
@prefix ex:  <http://ex.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:name ; sh:datatype xsd:string ; sh:minCount 1 ] .
"#;

/// `alice` satisfies the shape and `bob` does not: he is a Person with no
/// name, and the shape says `sh:minCount 1`. Both are here so a verdict can be
/// wrong in either direction — a run that blames nobody and a run that blames
/// everybody both fail, where a fixture with only a violation would let the
/// second through.
const DATA: &str = r#"
@prefix ex: <http://ex.org/> .
ex:alice a ex:Person ; ex:name "Alice" .
ex:bob   a ex:Person .
"#;

/// Where the verified evaluator would be found, by the rule `shacl_verified`
/// uses: an explicit `$OO_SHACL` is an instruction, then the build directory.
fn checker_present() -> bool {
    if let Ok(p) = std::env::var("OO_SHACL") {
        return Path::new(&p).exists();
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("lean/.lake/build/bin/oo-shacl")
        .exists()
}

fn skip() -> bool {
    common::skip_unless(
        checker_present(),
        "the verified SHACL evaluator (oo-shacl)",
        "run `lake build oo-shacl` in lean/, or point $OO_SHACL at a built one",
    )
}

#[test]
fn the_default_path_never_says_verified() {
    let d = scratch("default");
    let shapes = write(&d, "shapes.ttl", SHAPES);
    let data = write(&d, "data.ttl", DATA);
    let _ = run(&d, &["load", &data]);
    let (_code, out) = run(&d, &["shacl", &shapes]);
    assert!(
        !out.contains("\"verified\""),
        "the unverified path must not emit a key that says a theorem was involved: {out}"
    );
}

#[test]
fn a_scope_with_verified_is_refused_and_not_dropped() {
    // No checker needed: this is argument handling, and it must be testable on
    // a machine with no Lean toolchain.
    let d = scratch("scope");
    let shapes = write(&d, "shapes.ttl", SHAPES);
    for scope in [
        vec!["--valid-at", "2026-01-01T00:00:00Z"],
        vec!["--as-of", "2026-01-01T00:00:00Z"],
        vec!["--all-versions"],
    ] {
        let mut args = vec!["shacl", shapes.as_str(), "--verified"];
        args.extend_from_slice(&scope);
        let (_code, out) = run(&d, &args);
        assert!(
            out.contains("cannot be combined"),
            "a scope the verified evaluator cannot honour must be REFUSED, not dropped. \
             Args {scope:?} produced: {out}"
        );
        assert!(
            !out.contains("\"conforms\":true"),
            "and the refusal must not also carry a conformance answer: {out}"
        );
    }
}

/// Reachability, and only reachability.
///
/// The single-command path cannot carry data between invocations in the
/// default storage mode, so the store this validates is EMPTY and the verdict
/// is `conforms: true` over nothing. That is worth saying out loud rather than
/// leaving as a test that looks substantive and is not: what it proves is that
/// the flag reaches `oo-shacl` and that the theorem in the report is the
/// checker's own. The verdict with data in it is the batch test below.
#[test]
fn the_verified_flag_reaches_the_lean_evaluator() {
    if skip() {
        return;
    }
    let d = scratch("reach");
    let shapes = write(&d, "shapes.ttl", SHAPES);
    let (_code, out) = run(&d, &["shacl", &shapes, "--verified"]);
    assert!(
        out.contains("\"verified\""),
        "the verified path must say so in its report: {out}"
    );
    // The theorem is the checker's own word, echoed. If it were ever a Rust
    // literal the whole layer would be decoration, which is why this is
    // asserted and not merely documented.
    assert!(
        out.contains("Shacl.validate_spec"),
        "an accepted run names the theorem it discharged: {out}"
    );
}

/// The batch executor is different code from the single-command path and was
/// separately capable of not having the flag. It is also the only CLI route
/// that can load and then validate in one process, so it is where the verdict
/// has data under it.
#[test]
fn the_batch_path_finds_the_violation_the_shapes_describe() {
    if skip() {
        return;
    }
    let d = scratch("batch");
    let shapes = write(&d, "shapes.ttl", SHAPES);
    let data = write(&d, "data.ttl", DATA);
    let plan = write(
        &d,
        "plan.json",
        &format!(
            r#"[{{"command":"load","args":[{data:?}]}},
                {{"command":"shacl","args":[{shapes:?}, "--verified"]}}]"#
        ),
    );
    let (_code, out) = run(&d, &["batch", &plan]);
    assert!(out.contains("\"verified\""), "batch must reach the same evaluator: {out}");
    // `ex:bob` is a Person with no name, and the shape says minCount 1. A run
    // that reported `conforms: true` here would be reachable and wrong, which
    // is the failure the previous test cannot see.
    assert!(
        out.contains("\"conforms\": false") || out.contains("\"conforms\":false"),
        "the verified evaluator must find the missing name: {out}"
    );
    assert!(
        out.contains("http://ex.org/bob"),
        "and it must name the node it blames, not merely say something failed: {out}"
    );
    assert!(
        !out.contains("http://ex.org/alice"),
        "alice has a name and must not be blamed: {out}"
    );
}

/// The differential the issue said would fall out of this for free.
///
/// The two evaluators are not comparable in general: neither covers what the
/// other does. On a graph inside SHACL Core they must not DISAGREE, and this
/// is the first gate on `onto_shacl` that does not rest on SPARQL being right.
#[test]
fn the_two_evaluators_agree_on_a_graph_inside_shacl_core() {
    if skip() {
        return;
    }
    let d = scratch("differential");
    let shapes = write(&d, "shapes.ttl", SHAPES);
    let data = write(&d, "data.ttl", DATA);
    let plan = write(
        &d,
        "plan.json",
        &format!(
            r#"[{{"command":"load","args":[{data:?}]}},
                {{"command":"shacl","args":[{shapes:?}]}}]"#
        ),
    );
    let (_code, unverified) = run(&d, &["batch", &plan]);
    assert!(
        unverified.contains("http://ex.org/bob"),
        "the SPARQL-compiling path must blame bob too, or the two evaluators disagree \
         on a graph both of them fully cover: {unverified}"
    );
    assert!(
        !unverified.contains("\"verified\""),
        "and it must still not claim a theorem: {unverified}"
    );
}
