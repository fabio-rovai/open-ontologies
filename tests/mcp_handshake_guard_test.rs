//! Issue #260: a request arriving before `initialize` must not end the process.
//!
//! The report was a fresh 2.0 install whose client sent
//! `{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{}}` as its
//! first line. rmcp's handshake loop (rmcp-1.4.0/src/service/server.rs:170-203)
//! answers a pre-initialize `ping` and rejects everything else, so `serve`
//! returned `ExpectedInitializeRequest`, `main` propagated it, and the client's
//! `initialize` hit EOF. Measured on rmcp 1.4.0 and on 1.8.0: the same two
//! lines reject it in both, so this is not a version bump away.
//!
//! These tests drive the real handler over a `tokio::io::duplex` pipe, so no
//! MCP client is involved. Every line is written before `serve` is called,
//! which is what the reporter's process did in effect: the pipe holds the
//! bytes while the server spends its startup loading the embedding model.

use open_ontologies::config::{CacheConfig, EmbeddingsConfig};
use open_ontologies::graph::GraphStore;
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;
use open_ontologies::toolfilter::ToolFilter;
use rmcp::ServiceExt;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const INITIALIZE: &str = r#"{"jsonrpc":"2.0","id":2,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"issue-260","version":"0"}}}"#;
/// The reporter's line, verbatim. "server/discover" is not an MCP method and
/// appears nowhere in this repository, so it is the client's own probe.
const DISCOVER: &str = r#"{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{}}"#;

/// Feed `lines` to a guarded server over a pipe and collect `want` replies.
///
/// Every path is inside the tempdir, including the cache dir and the embedding
/// model paths: `CacheConfig::default()` points at `~/.open-ontologies/cache`
/// (src/config.rs:281) and an unset `model_path` resolves under
/// `~/.open-ontologies/models`, and a test must touch neither.
async fn handshake_replies(lines: &[&str], want: usize) -> Result<Vec<serde_json::Value>, String> {
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("state.db")).unwrap();
    let embeddings = EmbeddingsConfig {
        model_path: Some(tmp.path().join("absent.onnx").to_string_lossy().into_owned()),
        tokenizer_path: Some(
            tmp.path()
                .join("absent-tokenizer.json")
                .to_string_lossy()
                .into_owned(),
        ),
        ..Default::default()
    };
    let cache = CacheConfig {
        dir: tmp.path().join("cache").to_string_lossy().into_owned(),
        ..Default::default()
    };
    let server = OpenOntologiesServer::new_with_registry_options(
        db,
        Arc::new(GraphStore::new()),
        None,
        embeddings,
        cache,
        ToolFilter::default(),
    );

    let (mut client, pipe) = tokio::io::duplex(64 * 1024);
    for line in lines {
        client.write_all(line.as_bytes()).await.unwrap();
        client.write_all(b"\n").await.unwrap();
    }
    client.flush().await.unwrap();

    let service = server
        .serve(open_ontologies::mcp_handshake::guard(pipe))
        .await
        .map_err(|e| e.to_string())?;

    let mut reader = BufReader::new(client).lines();
    let mut out = Vec::new();
    while out.len() < want {
        let line = tokio::time::timeout(std::time::Duration::from_secs(10), reader.next_line())
            .await
            .map_err(|_| format!("timed out holding {} of {want} replies", out.len()))?
            .map_err(|e| e.to_string())?;
        match line {
            Some(l) => out.push(serde_json::from_str(&l).map_err(|e| format!("{e}: {l}"))?),
            None => {
                return Err(format!(
                    "pipe closed holding {} of {want} replies",
                    out.len()
                ));
            }
        }
    }
    // Held until the last reply is read. Dropping the running service cancels
    // rmcp's service loop, and a reply not yet written is then lost: that cost
    // the second reply in the rmcp 1.4.0 reproduction before the read moved
    // ahead of the drop.
    drop(service);
    Ok(out)
}

/// FAILS without the guard: the handshake ends with
/// `ServerInitializeError::ExpectedInitializeRequest` and no reply is written.
#[tokio::test]
async fn unknown_request_before_initialize_does_not_end_the_handshake() {
    let out = handshake_replies(&[DISCOVER, INITIALIZE], 2)
        .await
        .expect("the handshake must survive a request sent before initialize");
    assert_eq!(out[0]["id"], 1, "the probe is answered first: {:?}", out[0]);
    assert_eq!(
        out[0]["error"]["code"], -32601,
        "an unknown method gets the code rmcp gives it after the handshake: {:?}",
        out[0]
    );
    assert_eq!(out[1]["id"], 2, "then the initialize reply: {:?}", out[1]);
    assert!(
        out[1]["result"]["protocolVersion"].is_string(),
        "initialize must still be answered, not swallowed: {:?}",
        out[1]
    );
}

/// FAILS without the guard for the same reason: rmcp's pre-initialize loop
/// turns away every request that is not `ping`, a known MCP method included.
#[tokio::test]
async fn known_request_before_initialize_is_answered_invalid_request() {
    const TOOLS_LIST: &str = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;
    let out = handshake_replies(&[TOOLS_LIST, INITIALIZE], 2)
        .await
        .expect("the handshake must survive tools/list sent before initialize");
    assert_eq!(
        out[0]["error"]["code"], -32600,
        "a known method out of order is an ordering fault, not a missing method: {:?}",
        out[0]
    );
    assert!(out[1]["result"]["protocolVersion"].is_string());
}

/// Passes before and after the change, so it is a regression guard and not
/// evidence of the bug. It measures that the wrapper does not intercept live
/// traffic: this -32601 comes from rmcp's own `on_custom_request`.
#[tokio::test]
async fn guard_is_transparent_after_initialize() {
    const INITIALIZED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    let out = handshake_replies(&[INITIALIZE, INITIALIZED, DISCOVER], 2)
        .await
        .expect("handshake");
    assert_eq!(out[0]["id"], 2);
    assert!(out[0]["result"]["protocolVersion"].is_string());
    assert_eq!(out[1]["id"], 1);
    assert_eq!(out[1]["error"]["code"], -32601);
}
