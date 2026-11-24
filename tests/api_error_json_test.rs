// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! An error is valid JSON whatever its message contains (#420).
//!
//! The REST routes under `serve-http`'s `/api` and several CLI commands built
//! `{"error": ...}` by string formatting. A message carrying a double quote or
//! a backslash (a SPARQL or Turtle parse error quoting the offending
//! character, a format name the caller supplied, any Windows path) produced
//! invalid JSON. The REST routes then parsed their own output, failed, and
//! answered `null` with HTTP 200; the CLI printed the broken text and, for the
//! commands that set their exit code from the `error` key, exited 0.

mod common;

use common::HttpServer;
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::Command;
use std::time::Duration;

/// A SPARQL query whose parse error quotes `"` and `\` back to the caller.
const BAD_QUERY: &str = r#"SELECT * WHERE { ?s ?p "\q" }"#;

fn oo(dir: &tempfile::TempDir) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_open-ontologies"));
    cmd.arg("--data-dir").arg(dir.path());
    cmd.env_remove("OPEN_ONTOLOGIES_TOKEN")
        .env_remove("OPEN_ONTOLOGIES_STORAGE_MODE")
        .env_remove("GOVERNANCE_WEBHOOK")
        .env("NO_PROXY", "127.0.0.1,localhost")
        .env("no_proxy", "127.0.0.1,localhost")
        .env("RUST_LOG", "warn");
    cmd
}

fn start_server(dir: &tempfile::TempDir) -> (HttpServer, SocketAddr) {
    let config = dir.path().join("config.toml");
    std::fs::write(&config, "[storage]\nmode = 'memory'\n").unwrap();
    let mut cmd = oo(dir);
    cmd.env("OPEN_ONTOLOGIES_STORAGE_MODE", "memory")
        .arg("serve-http")
        .arg("--config")
        .arg(&config);
    let server = common::spawn_serve_http(cmd);
    let addr = server.addr;
    (server, addr)
}

/// One HTTP/1.1 POST over a plain socket, so the test needs no HTTP client.
fn post(addr: SocketAddr, path: &str, body: &Value) -> (u16, Value) {
    let body = body.to_string();
    let mut stream = TcpStream::connect(addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(60)))
        .unwrap();
    write!(
        stream,
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    let (head, payload) = response
        .split_once("\r\n\r\n")
        .expect("an HTTP response with a header block");
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("a status code");
    let value = serde_json::from_str(payload)
        .unwrap_or_else(|e| panic!("response body is not JSON ({e}): {payload}"));
    (status, value)
}

fn error_text(value: &Value) -> &str {
    value["error"]
        .as_str()
        .unwrap_or_else(|| panic!("expected an error object, got {value}"))
}

#[test]
fn api_query_parse_error_quoting_a_quote_and_a_backslash_is_an_error_object() {
    let dir = tempfile::tempdir().unwrap();
    let (_server, addr) = start_server(&dir);
    let (status, body) = post(addr, "/api/query", &json!({ "query": BAD_QUERY }));
    assert_eq!(status, 200);
    let message = error_text(&body);
    assert!(message.contains('"') && message.contains('\\'), "{message}");
}

#[test]
fn api_save_reports_a_caller_supplied_format_that_contains_a_quote_or_backslash() {
    let dir = tempfile::tempdir().unwrap();
    let (_server, addr) = start_server(&dir);
    let out = dir.path().join("out.ttl");
    for format in ["ya\"ml", "ya\\ml"] {
        let (_, body) = post(
            addr,
            "/api/save",
            &json!({ "path": out.to_str().unwrap(), "format": format }),
        );
        let message = error_text(&body);
        assert!(message.contains(format), "{message}");
    }
}

#[test]
fn api_save_success_names_a_path_that_needs_escaping() {
    let dir = tempfile::tempdir().unwrap();
    let (_server, addr) = start_server(&dir);
    // Windows forbids `"` in a file name, but every Windows path already
    // carries backslashes, which need escaping just the same.
    let name = if cfg!(windows) {
        "saved.ttl"
    } else {
        "sa\"v\\ed.ttl"
    };
    let out = dir.path().join(name);
    let out = out.to_str().unwrap();
    let (status, body) = post(
        addr,
        "/api/save",
        &json!({ "path": out, "format": "turtle" }),
    );
    assert_eq!(status, 200);
    assert_eq!(body, json!({ "ok": true, "path": out }));
    assert!(std::path::Path::new(out).exists());
}

#[test]
fn cli_lint_parse_error_is_valid_json_and_exits_nonzero() {
    let dir = tempfile::tempdir().unwrap();
    let ttl = dir.path().join("escape.ttl");
    std::fs::write(
        &ttl,
        "@prefix ex: <http://example.org/> .\nex:a ex:b \"bad \\q escape\" .\n",
    )
    .unwrap();
    let out = oo(&dir)
        .args(["--no-connect", "--json", "lint"])
        .arg(&ttl)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("lint output is not JSON ({e}): {stdout}"));
    assert!(error_text(&value).contains("\\q"), "{value}");
    assert_eq!(out.status.code(), Some(1), "{stdout}");
}

#[test]
fn cli_query_parse_error_is_valid_json() {
    let dir = tempfile::tempdir().unwrap();
    let out = oo(&dir)
        .args(["--no-connect", "--json", "query", BAD_QUERY])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let value: Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("query output is not JSON ({e}): {stdout}"));
    let message = error_text(&value);
    assert!(message.contains('"') && message.contains('\\'), "{message}");
}
