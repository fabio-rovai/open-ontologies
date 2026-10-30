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

/// 80 classes each under a disjoint pair, so 80 rows of `inherited_disjoint`.
fn eighty() -> String {
    let mut ttl = String::from(
        "@prefix ex: <http://ex.org/> .\n\
         @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
         @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
         ex:A owl:disjointWith ex:B .\n",
    );
    for i in 0..80 {
        ttl.push_str(&format!("ex:C{i} rdfs:subClassOf ex:A, ex:B .\n"));
    }
    ttl
}

/// #341: the 50-a-kind listing could not be lifted, so rows 51 to 80 were
/// counted under `truncated` and could not be obtained by any means.
#[test]
fn defects_limit_zero_lists_every_row() {
    let d = tempfile::tempdir().unwrap();
    let f = write(d.path(), "eighty.ttl", &eighty());
    let (code, out) = run(d.path(), &["defects", "--limit", "0", &f]);
    assert_eq!(code, 0, "output: {out}");
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect(&out);
    assert_eq!(parsed["reported_count"].as_u64(), Some(80), "{parsed:#}");
    assert_eq!(parsed["truncated"], serde_json::json!({}), "{parsed:#}");
}

#[test]
fn defects_limit_n_lists_n_and_reports_the_total() {
    let d = tempfile::tempdir().unwrap();
    let f = write(d.path(), "eighty.ttl", &eighty());
    let (code, out) = run(d.path(), &["defects", "--limit", "10", &f]);
    assert_eq!(code, 0, "output: {out}");
    let parsed: serde_json::Value = serde_json::from_str(out.trim()).expect(&out);
    assert_eq!(parsed["reported_count"].as_u64(), Some(10), "{parsed:#}");
    assert_eq!(parsed["truncated"]["inherited_disjoint"].as_u64(), Some(80), "{parsed:#}");
    assert_eq!(parsed["affected_classes"].as_u64(), Some(80), "{parsed:#}");
}

/// `defects x.ttl` checks the FILE. It used to load the file into the main
/// store, so under persistent storage it judged the file together with whatever
/// the store held and left the file's triples behind in it, while the same
/// command proxied to a daemon checked the file alone.
#[test]
fn defects_checks_the_file_alone_and_leaves_a_persistent_store_untouched() {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("data")).unwrap();
    std::fs::write(
        d.path().join("data").join("config.toml"),
        "[storage]\nmode = 'persistent'\n",
    )
    .unwrap();
    let persistent = |args: &[&str]| -> (i32, String) {
        let out = Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
            .env("OPEN_ONTOLOGIES_STORAGE_MODE", "persistent")
            .arg("--data-dir")
            .arg(d.path().join("data"))
            .arg("--no-connect")
            .args(args)
            .output()
            .expect("binary should run");
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    };
    let clean = write(d.path(), "good.ttl", CLEAN);
    let bad = write(d.path(), "bad.ttl", DEFECTIVE);
    let (code, out) = persistent(&["load", &clean]);
    assert_eq!(code, 0, "{out}");
    let triples = |out: &str| -> u64 {
        let v: serde_json::Value = serde_json::from_str(out.trim()).expect(out);
        v["triples"].as_u64().expect(out)
    };
    let before = triples(&persistent(&["stats"]).1);

    let (code, out) = persistent(&["defects", &bad]);
    assert_eq!(code, 0, "{out}");
    let report: serde_json::Value = serde_json::from_str(out.trim()).expect(&out);
    assert_eq!(report["defect_count"].as_u64(), Some(1), "{report:#}");

    let after = triples(&persistent(&["stats"]).1);
    assert_eq!(before, after, "checking a file must not write it into the store");
}
