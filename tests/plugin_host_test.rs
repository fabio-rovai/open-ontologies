//! WASM plugin host integration tests (ABI v1).
//!
//! Plugins here are assembled from WAT at test time so the suite needs no
//! wasm32 toolchain — the same ABI a real (Rust-compiled) plugin speaks.
#![cfg(feature = "plugins")]

use open_ontologies::plugins;
use std::path::PathBuf;

const MANIFEST: &str = r#"{"name":"echo","version":"0.1.0","tools":[{"name":"echo","description":"Returns its input document verbatim"}]}"#;

/// A well-behaved ABI v1 plugin: `oo_describe` serves the manifest from a data
/// segment at offset 0; `oo_alloc` hands out a fixed input buffer; `oo_call`
/// echoes the input region back (valid, since the host always sends JSON).
fn echo_plugin_wat() -> String {
    let escaped = MANIFEST.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        r#"(module
  (memory (export "memory") 2)
  (data (i32.const 0) "{escaped}")
  (func (export "oo_abi_version") (result i32) (i32.const 1))
  (func (export "oo_alloc") (param i32) (result i32) (i32.const 4096))
  (func (export "oo_describe") (result i64) (i64.const {len}))
  (func (export "oo_call") (param $ptr i32) (param $len i32) (result i64)
    (i64.or
      (i64.shl (i64.extend_i32_u (local.get $ptr)) (i64.const 32))
      (i64.extend_i32_u (local.get $len)))))"#,
        len = MANIFEST.len(),
    )
}

fn write_wasm(dir: &tempfile::TempDir, name: &str, wat: &str) -> PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, wat::parse_str(wat).expect("WAT should assemble")).unwrap();
    path
}

#[test]
fn describe_reads_manifest_over_abi_v1() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_wasm(&dir, "echo.wasm", &echo_plugin_wat());
    let manifest = plugins::describe(&path).expect("describe should succeed");
    assert_eq!(manifest.name, "echo");
    assert_eq!(manifest.version.as_deref(), Some("0.1.0"));
    assert_eq!(manifest.tools.len(), 1);
    assert_eq!(manifest.tools[0].name, "echo");
}

#[test]
fn call_round_trips_payload_through_guest_memory() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_wasm(&dir, "echo.wasm", &echo_plugin_wat());
    let payload = serde_json::json!({
        "tool": "echo",
        "input": {"answer": 42},
        "bindings": [{"class": "<http://example.org/A>", "label": "A"}],
    });
    let result = plugins::call(&path, &payload).expect("call should succeed");
    assert_eq!(result, payload, "echo plugin must return its input verbatim");
}

#[test]
fn non_plugin_wasm_is_rejected_with_missing_export() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_wasm(&dir, "empty.wasm", "(module)");
    let err = plugins::describe(&path).unwrap_err();
    assert!(err.contains("oo_abi_version"), "unexpected error: {err}");
}

#[test]
fn abi_version_mismatch_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let wat = r#"(module
      (memory (export "memory") 1)
      (func (export "oo_abi_version") (result i32) (i32.const 99)))"#;
    let path = write_wasm(&dir, "future.wasm", wat);
    let err = plugins::describe(&path).unwrap_err();
    assert!(err.contains("ABI"), "unexpected error: {err}");
}

#[test]
fn runaway_plugin_is_stopped_by_fuel_not_by_hanging() {
    // An infinite loop in oo_call must trap on fuel exhaustion. If fuel
    // metering regresses this test hangs instead of failing — that IS the
    // signal (the sandbox guarantee is gone).
    let dir = tempfile::tempdir().unwrap();
    let escaped = MANIFEST.replace('\\', "\\\\").replace('"', "\\\"");
    let wat = format!(
        r#"(module
  (memory (export "memory") 2)
  (data (i32.const 0) "{escaped}")
  (func (export "oo_abi_version") (result i32) (i32.const 1))
  (func (export "oo_alloc") (param i32) (result i32) (i32.const 4096))
  (func (export "oo_describe") (result i64) (i64.const {len}))
  (func (export "oo_call") (param i32) (param i32) (result i64)
    (loop $spin (br $spin))
    (i64.const 0)))"#,
        len = MANIFEST.len(),
    );
    let path = write_wasm(&dir, "spin.wasm", &wat);
    let err = plugins::call(&path, &serde_json::json!({"tool": "x", "input": null})).unwrap_err();
    assert!(err.contains("oo_call failed"), "unexpected error: {err}");
}

#[test]
fn oversized_return_is_capped() {
    // A plugin claiming a 4GiB-ish result length must be refused, not allocated.
    let escaped = MANIFEST.replace('\\', "\\\\").replace('"', "\\\"");
    let wat = format!(
        r#"(module
  (memory (export "memory") 2)
  (data (i32.const 0) "{escaped}")
  (func (export "oo_abi_version") (result i32) (i32.const 1))
  (func (export "oo_alloc") (param i32) (result i32) (i32.const 4096))
  (func (export "oo_describe") (result i64) (i64.const {len}))
  (func (export "oo_call") (param i32) (param i32) (result i64)
    (i64.const 0xFFFFFFFF)))"#,
        len = MANIFEST.len(),
    );
    let dir = tempfile::tempdir().unwrap();
    let path = write_wasm(&dir, "huge.wasm", &wat);
    let err = plugins::call(&path, &serde_json::json!({"tool": "x", "input": null})).unwrap_err();
    assert!(err.contains("cap"), "unexpected error: {err}");
}
