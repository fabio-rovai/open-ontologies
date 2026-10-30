//! A gate command must tell the shell what its verdict was (#323).
//!
//! `cli_exit_codes_test.rs` covers commands whose failure is an `error` field.
//! The commands here fail with a VERDICT instead: `shacl` with
//! `conforms: false`, `unpack` with `certificate_refused`, `enforce` with an
//! error-severity violation, `monitor` with a blocking watcher. All of them
//! exited 0, so `open-ontologies shacl shapes.ttl || exit 1` passed on exactly
//! the data it exists to catch. The codes are 0 pass, 1 fail, 2 not determined.

mod common;

use open_ontologies::graph::GraphStore;
use open_ontologies::pack::{PackRequest, Packer};
use open_ontologies::reason::{InferenceTarget, Reasoner};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

/// Every invocation shares one persistent store under `dir`, so a `load` in
/// one process is what the next process validates. `--no-connect` keeps a
/// developer's running daemon out of it.
fn run(dir: &Path, args: &[&str]) -> (i32, String) {
    let out = {
        let _gate = common::exec_gate();
        Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
            .env("OPEN_ONTOLOGIES_STORAGE_MODE", "persistent")
            .arg("--data-dir")
            .arg(dir.join("data"))
            .arg("--no-connect")
            .args(args)
            .output()
            .expect("binary should run")
    };
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code().unwrap_or(-1), text)
}

fn store_dir() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(d.path().join("data")).unwrap();
    std::fs::write(
        d.path().join("data").join("config.toml"),
        "[storage]\nmode = 'persistent'\n",
    )
    .unwrap();
    d
}

fn write(dir: &Path, name: &str, body: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p.to_str().unwrap().to_string()
}

const DATA: &str = r#"
@prefix ex: <http://ex.org/> .
ex:alice a ex:Person ; ex:name "Alice" .
ex:bob a ex:Person .
"#;

const NAME_REQUIRED: &str = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://ex.org/> .
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] .
"#;

/// Targets only people with a name, so every focus node conforms.
const NAME_REQUIRED_OF_ALICE: &str = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://ex.org/> .
ex:AliceShape a sh:NodeShape ;
    sh:targetNode ex:alice ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] .
"#;

/// A length bound that is not an integer is skipped by name, so the engine
/// cannot say the data conforms and reports `conforms: null`.
const UNEVALUABLE: &str = r#"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://ex.org/> .
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:name ; sh:minLength "three" ] .
"#;

fn loaded_store() -> tempfile::TempDir {
    let d = store_dir();
    let data = write(d.path(), "data.ttl", DATA);
    let (code, out) = run(d.path(), &["load", &data]);
    assert_eq!(code, 0, "load failed: {out}");
    d
}

#[test]
fn shacl_exits_1_when_the_data_does_not_conform() {
    let d = loaded_store();
    let shapes = write(d.path(), "s.ttl", NAME_REQUIRED);
    let (code, out) = run(d.path(), &["shacl", &shapes]);
    assert!(out.contains("\"conforms\":false"), "precondition: {out}");
    assert_eq!(code, 1, "non-conforming data must fail the process: {out}");
}

#[test]
fn shacl_exits_0_when_the_data_conforms() {
    let d = loaded_store();
    let shapes = write(d.path(), "s.ttl", NAME_REQUIRED_OF_ALICE);
    let (code, out) = run(d.path(), &["shacl", &shapes]);
    assert!(out.contains("\"conforms\":true"), "precondition: {out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn shacl_exits_2_when_conformance_was_not_determined() {
    let d = loaded_store();
    let shapes = write(d.path(), "s.ttl", UNEVALUABLE);
    let (code, out) = run(d.path(), &["shacl", &shapes]);
    assert!(out.contains("\"conforms\":null"), "precondition: {out}");
    assert_eq!(code, 2, "an undetermined verdict is neither pass nor fail: {out}");
}

#[test]
fn shacl_verified_exits_1_when_the_data_does_not_conform() {
    let d = loaded_store();
    let shapes = write(d.path(), "s.ttl", NAME_REQUIRED);
    let (code, out) = run(d.path(), &["shacl", &shapes, "--verified"]);
    // Without a Lean checker the verified path answers null, which is 2; with
    // one it answers false, which is 1. Either way it is not a pass.
    assert_ne!(code, 0, "{out}");
    if out.contains("\"conforms\":false") {
        assert_eq!(code, 1, "{out}");
    }
}

/// `ies4_particular_class_overlap` is an error-severity rule.
const IES4_CLASH: &str = r#"
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ies: <http://ies.data.gov.uk/ontology/ies4#> .
@prefix ex: <http://ex.org/> .
ex:Thing rdfs:subClassOf ies:Particular , ies:ClassOfEntity .
"#;

#[test]
fn enforce_exits_1_on_an_error_severity_violation_and_0_without_one() {
    let d = store_dir();
    let f = write(d.path(), "ies.ttl", IES4_CLASH);
    let (code, out) = run(d.path(), &["load", &f]);
    assert_eq!(code, 0, "load failed: {out}");
    let (code, out) = run(d.path(), &["enforce", "ies4"]);
    assert!(out.contains("ies4_particular_class_overlap"), "precondition: {out}");
    assert_eq!(code, 1, "an error-severity violation must fail the process: {out}");

    let clean = store_dir();
    let (code, out) = run(clean.path(), &["enforce", "ies4"]);
    assert_eq!(code, 0, "an empty store has no violations: {out}");
}

#[test]
fn monitor_exits_0_with_no_blocking_watcher() {
    let d = store_dir();
    let (code, out) = run(d.path(), &["monitor"]);
    assert!(out.contains("\"status\":\"ok\""), "precondition: {out}");
    assert_eq!(code, 0, "{out}");
}

fn fake_checker(dir: &Path, code: i32) -> PathBuf {
    let ext = if cfg!(windows) { "cmd" } else { "sh" };
    let p = dir.join(format!("exit{code}.{ext}"));
    let say = r#"{"ok":true,"theorem":"OOCert.certificate_sound","checker":{"name":"fake","sha256":"0"}}"#;
    let body = if cfg!(windows) {
        format!("@echo off\r\necho {say}\r\nexit /b {code}\r\n")
    } else {
        format!("#!/bin/sh\necho '{say}'\nexit {code}\n")
    };
    let _gate = common::exec_gate();
    std::fs::write(&p, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    p
}

fn certified_pack(dir: &Path) -> String {
    let cert_dir = dir.join("cert");
    std::fs::create_dir_all(&cert_dir).unwrap();
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            "@prefix : <http://ex.org/> . \
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> . \
             :A rdfs:subClassOf :B . :B rdfs:subClassOf :C . :a a :A .",
            None,
        )
        .unwrap();
    Reasoner::run_full(&store, "owl-rl-ext", true, InferenceTarget::DefaultGraph, Some(&cert_dir))
        .unwrap();
    let path = dir.join("carried.oopack");
    Packer::new(store)
        .pack(&PackRequest {
            path: path.to_str().unwrap(),
            name: "carried",
            version: "1.0.0",
            evidence: None,
            certificate_dir: Some(&cert_dir),
            profile_claimed: Some("owl-rl-ext"),
        })
        .expect("pack with a certificate");
    path.to_str().unwrap().to_string()
}

#[test]
fn unpack_exit_code_follows_the_certificate_verdict() {
    let d = store_dir();
    let pack = certified_pack(d.path());
    let accept = fake_checker(d.path(), 0);
    let refuse = fake_checker(d.path(), 1);

    let (code, out) = run(d.path(), &["unpack", &pack, "--checker", accept.to_str().unwrap()]);
    assert!(out.contains("certificate_accepted"), "precondition: {out}");
    assert_eq!(code, 0, "{out}");

    let (code, out) = run(d.path(), &["unpack", &pack, "--checker", refuse.to_str().unwrap()]);
    assert!(out.contains("certificate_refused"), "precondition: {out}");
    assert_eq!(code, 1, "a refused certificate must fail the process: {out}");

    let (code, out) = run(d.path(), &["unpack", &pack, "--no-check"]);
    assert!(out.contains("checker_absent_nothing_was_checked"), "precondition: {out}");
    assert_eq!(code, 2, "nothing was checked, which is not a pass: {out}");
}
