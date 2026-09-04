//! The `defects` command as a shell citizen.
//!
//! Written before the command exists, and the exit-code case is written first
//! on purpose. `lint`, `vocab-check` and `diff` each shipped printing
//! `{"error": ...}` and exiting 0, so `open-ontologies lint x.ttl || exit 1`
//! passed on exactly the input it existed to catch. A new checker that repeats
//! that is worse than no checker, because a CI gate built on it reports a pass
//! over a file it never read.

use std::process::Command;

/// Every invocation gets its own `--data-dir`, the discipline `cli_test.rs` and
/// `cli_exit_codes_test.rs` both keep: without it these tests open the
/// developer's real database and race each other.
fn run(dir: &std::path::Path, args: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
        .arg("--data-dir")
        .arg(dir.join("data"))
        .args(args)
        .output()
        .expect("binary should run");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code().unwrap_or(-1), text)
}

fn write(dir: &std::path::Path, name: &str, body: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p.to_str().unwrap().to_string()
}

const GARBAGE: &str = "this is not turtle at all <<<>>> @@@\n";

const CLEAN: &str = r#"
@prefix ex:   <http://ex.org/> .
@prefix owl:  <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
ex:Animal a owl:Class .
ex:Dog rdfs:subClassOf ex:Animal .
"#;

const DEFECTIVE: &str = r#"
@prefix ex:  <http://ex.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
ex:partOf a owl:TransitiveProperty, owl:FunctionalProperty .
"#;

#[test]
fn defects_fails_the_process_on_unparseable_input() {
    let d = tempfile::tempdir().unwrap();
    let f = write(d.path(), "g.ttl", GARBAGE);
    let (code, out) = run(d.path(), &["defects", &f]);
    assert_eq!(
        code, 1,
        "a checker that cannot read its input must say so to the shell: {out}"
    );
    assert!(out.contains("error"), "output: {out}");
}

#[test]
fn defects_fails_the_process_on_a_missing_file() {
    let d = tempfile::tempdir().unwrap();
    let missing = d.path().join("nope.ttl");
    let (code, out) = run(d.path(), &["defects", missing.to_str().unwrap()]);
    assert_eq!(code, 1, "output: {out}");
    assert!(
        out.contains("error"),
        "the JSON contract has to hold for the case it most needs to: {out}"
    );
}

#[test]
fn defects_reports_a_defect_it_finds() {
    let d = tempfile::tempdir().unwrap();
    let f = write(d.path(), "bad.ttl", DEFECTIVE);
    let (code, out) = run(d.path(), &["defects", &f]);
    assert_eq!(code, 0, "a readable file with a defect is a successful run: {out}");
    assert!(out.contains("transitive_and_functional"), "output: {out}");
}

#[test]
fn defects_on_a_clean_ontology_reports_none() {
    let d = tempfile::tempdir().unwrap();
    let f = write(d.path(), "good.ttl", CLEAN);
    let (code, out) = run(d.path(), &["defects", &f]);
    assert_eq!(code, 0, "output: {out}");
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect(&out);
    assert_eq!(parsed["defect_count"].as_u64(), Some(0), "{parsed:#}");
}
