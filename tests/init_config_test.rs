// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! `init` must not hide a malformed existing config and fall back to the local
//! embedding provider, which can trigger a large, unintended model download.

#![cfg(feature = "embeddings")]

use std::fs;
use std::process::Command;

use tempfile::tempdir;

#[test]
fn init_reports_malformed_existing_config_without_starting_model_download() {
    let temp = tempdir().unwrap();
    let data_dir = temp.path().join("data");
    fs::create_dir_all(&data_dir).unwrap();
    fs::write(data_dir.join("config.toml"), "[embeddings\nprovider = \"openai\"\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
        .args(["--no-connect", "init", "--data-dir"])
        .arg(&data_dir)
        .output()
        .unwrap();

    assert!(!output.status.success(), "init unexpectedly accepted malformed config");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("failed to parse config file"), "unexpected stderr: {stderr}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("initialized successfully"), "init reported success: {stdout}");
    assert!(!data_dir.join("models").exists(), "init created a local model directory");
}
