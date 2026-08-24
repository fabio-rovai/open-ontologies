//! `load` in one-shot CLI mode must say when nothing was kept.
//!
//! With the default in-memory storage backend every CLI invocation builds a
//! fresh store, so `open-ontologies load data.ttl` populates a graph that is
//! dropped when the process exits. It reported `{"ok": true, "triples_loaded": N}`
//! and the very next `stats` reported zero, which reads as data loss rather than
//! as the documented consequence of the storage mode.
//!
//! Same rule as the SHACL reports: a success-shaped answer must not stand in for
//! something that did not persist.

use std::process::Command;

fn oo(dir: &tempfile::TempDir) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_open-ontologies"));
    cmd.arg("--data-dir").arg(dir.path());
    cmd
}

fn write_ttl(dir: &tempfile::TempDir) -> std::path::PathBuf {
    let path = dir.path().join("data.ttl");
    std::fs::write(
        &path,
        "@prefix ex: <http://example.org/> .\nex:alice a ex:Person .\n",
    )
    .unwrap();
    path
}

#[test]
fn load_in_memory_mode_warns_that_the_store_is_not_persisted() {
    let dir = tempfile::tempdir().unwrap();
    let ttl = write_ttl(&dir);

    let out = oo(&dir)
        .env("OPEN_ONTOLOGIES_STORAGE_MODE", "memory")
        .arg("load")
        .arg(&ttl)
        .output()
        .unwrap();
    let report: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("load emits JSON");

    assert_eq!(report["ok"], serde_json::json!(true));
    assert_eq!(report["triples_loaded"], serde_json::json!(1));
    let warning = report["warning"]
        .as_str()
        .expect("in-memory load must carry a warning");
    assert!(
        warning.contains("persist"),
        "warning should say the load did not persist, got: {warning}"
    );
}

#[test]
fn load_in_persistent_mode_carries_no_warning() {
    let dir = tempfile::tempdir().unwrap();
    let ttl = write_ttl(&dir);

    let out = oo(&dir)
        .env("OPEN_ONTOLOGIES_STORAGE_MODE", "persistent")
        .arg("load")
        .arg(&ttl)
        .output()
        .unwrap();
    let report: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("load emits JSON");

    assert_eq!(report["triples_loaded"], serde_json::json!(1));
    assert!(
        report["warning"].is_null(),
        "a persisted load has nothing to warn about"
    );
}
