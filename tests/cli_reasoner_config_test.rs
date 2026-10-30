//! One-shot CLI commands honour `[reasoner]` in `<data-dir>/config.toml` (#339).
//!
//! `setup()` loaded `config.toml` and kept only its `[storage]` section, and
//! `runtime::init_from_config` was called by the three server modes alone. So
//! `open-ontologies reason --profile owl-dl` ran with the default budgets whatever
//! the file said, and its `budget` block reported those defaults as though they
//! were the configuration. The same file passed to `serve --config` was honoured.
//!
//! These tests read the `budget` block of a real `owl-dl` run, because that block
//! is what a reader is told is in force.

mod common;

use std::path::Path;
use std::process::Command;

const ONTOLOGY: &str = r#"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ex: <http://ex.org/> .
ex:Animal a owl:Class .
ex:Dog a owl:Class ; rdfs:subClassOf ex:Animal .
"#;

/// Every invocation shares one persistent store under `dir/data`, so the `load`
/// in one process is what the next process reasons over. `--no-connect` keeps a
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

/// A store whose config.toml carries `reasoner` as its `[reasoner]` section, with
/// the small ontology above already loaded.
fn loaded_store(reasoner: &str) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    let data = d.path().join("data");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::write(
        data.join("config.toml"),
        format!("[storage]\nmode = 'persistent'\n\n[reasoner]\n{reasoner}"),
    )
    .unwrap();
    let ttl = d.path().join("o.ttl");
    std::fs::write(&ttl, ONTOLOGY).unwrap();
    let (code, out) = run(d.path(), &["load", ttl.to_str().unwrap()]);
    assert_eq!(code, 0, "load must succeed: {out}");
    d
}

/// The `budget` block of an `owl-dl` run with `extra` appended to the command.
fn budget(dir: &Path, extra: &[&str]) -> serde_json::Value {
    let mut args = vec!["reason", "--profile", "owl-dl"];
    args.extend_from_slice(extra);
    let (code, out) = run(dir, &args);
    assert_eq!(code, 0, "reason must succeed: {out}");
    let line = out
        .lines()
        .find(|l| l.trim_start().starts_with('{'))
        .unwrap_or_else(|| panic!("no JSON in the output: {out}"));
    let v: serde_json::Value = serde_json::from_str(line).expect("reason prints JSON");
    v.get("budget")
        .cloned()
        .unwrap_or_else(|| panic!("owl-dl output has no budget block: {v}"))
}

#[test]
fn reason_reports_the_budgets_set_in_config_toml() {
    let d = loaded_store("tableaux_test_timeout_ms = 200\nclassify_timeout_ms = 60000\n");
    let b = budget(d.path(), &[]);
    assert_eq!(b["phase_timeout_ms"], 200, "config phase budget ignored: {b}");
    assert_eq!(b["classify_timeout_ms"], 60000, "config ceiling ignored: {b}");
    // 5 phases x 200 ms = 1000 ms, under the 60 000 ms ceiling.
    assert_eq!(b["phase_worst_case_ms"], 1000, "{b}");
    // The settings predict the phase budget binds; this tiny run hits neither
    // clock, and `binding_bound` reports what actually stopped it (#340).
    assert_eq!(b["configured_bound"], "phase", "{b}");
    assert_eq!(b["binding_bound"], "none", "{b}");
}

#[test]
fn reason_flags_override_config_toml() {
    let d = loaded_store("tableaux_test_timeout_ms = 200\nclassify_timeout_ms = 60000\n");

    let b = budget(
        d.path(),
        &["--phase-timeout-ms", "300", "--classify-timeout-ms", "700"],
    );
    assert_eq!(b["phase_timeout_ms"], 300, "flag must beat config: {b}");
    assert_eq!(b["classify_timeout_ms"], 700, "flag must beat config: {b}");
    // 5 x 300 = 1500 > 700, so the ceiling is now the bound expected to fire.
    assert_eq!(b["configured_bound"], "global", "{b}");

    // One flag alone leaves the other at its configured value, not the default.
    let b = budget(d.path(), &["--phase-timeout-ms", "400"]);
    assert_eq!(b["phase_timeout_ms"], 400, "{b}");
    assert_eq!(b["classify_timeout_ms"], 60000, "unflagged ceiling must stay configured: {b}");

    // 0 means no limit, as it does in config.toml.
    let b = budget(d.path(), &["--phase-timeout-ms", "0"]);
    assert!(b["phase_timeout_ms"].is_null(), "0 must mean no phase limit: {b}");
    assert_eq!(b["classify_timeout_ms"], 60000, "{b}");
}

#[test]
fn reason_without_config_or_flags_reports_the_defaults() {
    let d = loaded_store("");
    let b = budget(d.path(), &[]);
    assert_eq!(b["phase_timeout_ms"], 10_000, "{b}");
    assert_eq!(b["classify_timeout_ms"], 180_000, "{b}");
}
