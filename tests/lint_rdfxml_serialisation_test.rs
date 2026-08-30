//! `lint` could not read RDF/XML, and `batch` could not reach `vocab-check`.
//!
//! `validate` has always sniffed the serialisation before parsing. `lint` did not: it read
//! the file as a string and handed the bytes straight to a Turtle parser, so every RDF/XML
//! document died on line 1 with a parse error raised by the wrong parser, and nothing in the
//! message said the format was the problem. OBO Foundry publishes RDF/XML, so this was not an
//! edge case. In a census of the 250 most-declared vocabulary URLs in the public metabolomics
//! record, 64 returned a parseable ontology and `lint` could read 2 of them. The engine was
//! unusable on the ecosystem it is meant to check, including OBI, ENVO, CHMO, BTO and MS.
//!
//! The second defect was reachability rather than parsing. `exec_vocab_check` existed and was
//! correct, but the dispatch arm matched `vocab_check` with an underscore while the CLI spells
//! the subcommand with a hyphen, so every documented invocation was rejected as an unknown
//! batch command. That mattered more than an ordinary name mismatch: the check requires a
//! loaded ontology, `load` does not persist under the default in-memory storage mode, and
//! `batch` is the only way to load and then operate inside one process. The command was
//! therefore unreachable in the only mode it was designed for.
//!
//! Both were found while using this engine as the third verification path in an ontology
//! census, and reported in that project's issue list before being fixed here.

use open_ontologies::graph::GraphStore;
use std::process::Command;

const RDFXML: &str = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:owl="http://www.w3.org/2002/07/owl#"
         xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#">
  <owl:Ontology rdf:about="http://ex.org/o"/>
  <owl:Class rdf:about="http://ex.org/o#Parent"/>
  <owl:Class rdf:about="http://ex.org/o#Child">
    <rdfs:subClassOf rdf:resource="http://ex.org/o#Parent"/>
  </owl:Class>
</rdf:RDF>
"#;

const TURTLE: &str = r#"@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
<http://ex.org/o#Parent> a owl:Class .
<http://ex.org/o#Child> a owl:Class ; rdfs:subClassOf <http://ex.org/o#Parent> .
"#;

fn write(dir: &std::path::Path, name: &str, body: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p.to_str().unwrap().to_string()
}

/// The regression itself: RDF/XML in, Turtle out, no parse error.
#[test]
fn content_as_turtle_accepts_rdfxml() {
    let out = GraphStore::content_as_turtle("o.owl", RDFXML.to_string())
        .expect("RDF/XML must not be handed to the Turtle parser");
    assert!(
        out.contains("http://ex.org/o#Child"),
        "converted Turtle lost the subject: {out}"
    );
    assert!(
        !out.contains("<?xml"),
        "content was passed through unconverted: {out}"
    );
}

/// The conversion must preserve meaning, not merely avoid erroring.
#[test]
fn rdfxml_conversion_preserves_the_subclass_axiom() {
    let out = GraphStore::content_as_turtle("o.owl", RDFXML.to_string()).unwrap();
    assert!(
        out.contains("subClassOf"),
        "rdfs:subClassOf did not survive the round trip: {out}"
    );
    assert!(
        out.contains("http://ex.org/o#Parent"),
        "the superclass did not survive the round trip: {out}"
    );
}

/// Turtle must still be returned untouched, so the fix cannot cost a needless reserialisation.
#[test]
fn turtle_is_passed_through_unchanged() {
    let out = GraphStore::content_as_turtle("o.ttl", TURTLE.to_string()).unwrap();
    assert_eq!(out, TURTLE, "Turtle should be returned verbatim");
}

/// End to end, through the binary, because that is how the defect was actually met.
#[test]
fn lint_reports_issues_on_rdfxml_rather_than_a_parse_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), "o.owl", RDFXML);

    let out = Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
        .args(["lint", &path])
        .output()
        .expect("binary should run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        out.status.success(),
        "lint failed on RDF/XML.\nstdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.contains("issue_count"),
        "expected a lint report, got: {stdout}{stderr}"
    );
    assert!(
        !stdout.contains("expected") && !stderr.contains("expected"),
        "a Turtle parse error leaked through: {stdout}{stderr}"
    );
    assert!(
        stdout.contains("ex.org/o#Child") || stdout.contains("ex.org/o#Parent"),
        "lint read the file but saw none of its classes: {stdout}"
    );
}

/// `batch` must accept the hyphenated spelling the CLI documents, and the underscore it
/// historically matched, so neither existing scripts nor documented ones break.
#[test]
fn batch_accepts_vocab_check_under_both_spellings() {
    let dir = tempfile::tempdir().unwrap();
    let onto = write(dir.path(), "o.ttl", TURTLE);
    let data = write(dir.path(), "d.ttl", TURTLE);

    for spelling in ["vocab-check", "vocab_check"] {
        let script = format!("load {onto}\n{spelling} {data}\n");
        let mut child = Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
            .args(["batch", "-"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        {
            use std::io::Write;
            child
                .stdin
                .as_mut()
                .unwrap()
                .write_all(script.as_bytes())
                .unwrap();
        }
        let out = child.wait_with_output().unwrap();
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            !combined.contains("unknown"),
            "batch rejected `{spelling}` as unknown: {combined}"
        );
    }
}
