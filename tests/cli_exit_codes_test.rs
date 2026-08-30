//! A command that failed must tell the shell it failed.
//!
//! `lint`, `vocab-check` and `diff` printed `{"error": ...}` and exited 0, so
//! `open-ontologies lint x.ttl || exit 1` passed on exactly the input it exists
//! to catch: a malformed ontology, a truncated download, a soft-404 body.
//!
//! `lint` was worse than the others. It swallowed a serialisation failure into
//! the string `# could not read as RDF: ...`, which is a valid Turtle document
//! containing zero triples, so lint parsed that happily and reported
//! `issue_count: 0` with exit 0 over a file it had never read. A clean bill of
//! health for an unreadable document is the one answer this command must not give.

use std::process::Command;

fn run(args: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
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
const BROKEN_XML: &str = "<?xml version=\"1.0\"?><rdf:RDF this is broken <<<\n";
const GOOD: &str = "@prefix : <http://ex.org/> .\n@prefix owl: <http://www.w3.org/2002/07/owl#> .\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n:C a owl:Class ; rdfs:label \"C\" ; rdfs:comment \"a class\" .\n";

#[test]
fn lint_fails_the_process_on_unparseable_turtle() {
    let d = tempfile::tempdir().unwrap();
    let f = write(d.path(), "g.ttl", GARBAGE);
    let (code, out) = run(&["lint", &f]);
    assert_eq!(code, 1, "lint must exit non-zero. output: {out}");
    assert!(out.contains("error"), "output: {out}");
}

/// The silent-clean-pass case. This one reported no error at all.
#[test]
fn lint_does_not_report_a_clean_bill_of_health_for_unreadable_rdfxml() {
    let d = tempfile::tempdir().unwrap();
    let f = write(d.path(), "broken.rdf", BROKEN_XML);
    let (code, out) = run(&["lint", &f]);
    assert_eq!(code, 1, "lint must exit non-zero. output: {out}");
    assert!(
        !out.contains("\"issue_count\":0"),
        "lint reported a clean result for a file it could not read: {out}"
    );
    assert!(out.contains("error"), "output: {out}");
}

#[test]
fn diff_fails_the_process_on_unparseable_input() {
    let d = tempfile::tempdir().unwrap();
    let f = write(d.path(), "g.ttl", GARBAGE);
    let (code, out) = run(&["diff", &f, &f]);
    assert_eq!(code, 1, "diff must exit non-zero. output: {out}");
}

#[test]
fn valid_input_still_succeeds() {
    let d = tempfile::tempdir().unwrap();
    let f = write(d.path(), "ok.ttl", GOOD);
    let (code, out) = run(&["lint", &f]);
    assert_eq!(code, 0, "a readable ontology must still exit 0: {out}");
    let (code, out) = run(&["diff", &f, &f]);
    assert_eq!(code, 0, "two readable ontologies must still exit 0: {out}");
}
