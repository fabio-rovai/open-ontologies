// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Issue #260: a request arriving before `initialize` must not end the process.
//!
//! The original report concerned a `server/discover` probe on rmcp 1.4.
//! rmcp 3.9 recognizes discovery, but this server keeps session-based protocol
//! revisions. Exercise legacy replies, discovery validation and initialization
//! fallback over the real transport.
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
/// The reporter's line, verbatim. On rmcp 1.4 "server/discover" was no MCP
/// method. On rmcp 3.9 it is the 2026-07-28 protocol's probe, which must carry
/// a protocol version and client capabilities in `_meta`; this line carries
/// neither.
const DISCOVER: &str = r#"{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{}}"#;
/// A well-formed 2026-07-28 probe, as a client that speaks that revision sends
/// it before anything else.
const DISCOVER_2026: &str = r#"{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}}}"#;
/// rmcp's code for a request whose `_meta` lacks the 2026-07-28 fields.
const INVALID_PARAMS: i64 = -32602;
const UNKNOWN: &str = r#"{"jsonrpc":"2.0","id":1,"method":"unknown/probe","params":{}}"#;

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
        model_path: Some(
            tmp.path()
                .join("absent.onnx")
                .to_string_lossy()
                .into_owned(),
        ),
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

/// Unknown legacy probes keep the guard's method-not-found reply.
#[tokio::test]
async fn unknown_request_before_initialize_does_not_end_the_handshake() {
    let out = handshake_replies(&[UNKNOWN, INITIALIZE], 2)
        .await
        .expect("the handshake must survive a request sent before initialize");
    assert_eq!(out[0]["id"], 1, "the probe is answered first: {:?}", out[0]);
    assert_eq!(
        out[0]["error"]["code"], -32601,
        "an unknown probe keeps its method-not-found reply: {:?}",
        out[0]
    );
    assert_eq!(out[1]["id"], 2, "then the initialize reply: {:?}", out[1]);
    assert!(
        out[1]["result"]["protocolVersion"].is_string(),
        "initialize must still be answered, not swallowed: {:?}",
        out[1]
    );
}

/// Known legacy requests sent early keep the guard's invalid-request reply.
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
/// traffic: the unknown-method error comes from the SDK's live handler.
#[tokio::test]
async fn guard_is_transparent_after_initialize() {
    const INITIALIZED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    let out = handshake_replies(&[INITIALIZE, INITIALIZED, UNKNOWN], 2)
        .await
        .expect("handshake");
    assert_eq!(out[0]["id"], 2);
    assert!(out[0]["result"]["protocolVersion"].is_string());
    assert_eq!(out[1]["id"], 1);
    assert_eq!(out[1]["error"]["code"], -32601);
}

/// A 2026-07-28 client probes first and is told which revisions this server
/// speaks. `supported_protocol_versions` stops at 2025-11-25, because that
/// revision is the last with sessions and the server keeps per-session state
/// (src/server.rs). The client then falls back to `initialize`, which must
/// still be answered, and negotiates a session revision.
#[tokio::test]
async fn a_2026_07_28_probe_is_told_the_session_revisions_and_initialize_still_works() {
    let out = handshake_replies(&[DISCOVER_2026, INITIALIZE], 2)
        .await
        .expect("the handshake must survive a 2026-07-28 probe");
    assert_eq!(out[0]["id"], 1, "the probe is answered first: {:?}", out[0]);
    assert_eq!(
        out[0]["error"]["code"], -32022,
        "2026-07-28 is refused as an unsupported version: {:?}",
        out[0]
    );
    let supported: Vec<&str> = out[0]["error"]["data"]["supported"]
        .as_array()
        .expect("the refusal lists the supported revisions")
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(
        supported,
        ["2024-11-05", "2025-03-26", "2025-06-18", "2025-11-25"],
        "every revision with an initialize handshake, and not 2026-07-28"
    );
    assert_eq!(out[1]["id"], 2, "then the initialize reply: {:?}", out[1]);
    assert_eq!(out[1]["result"]["protocolVersion"], "2025-06-18");
}

#[tokio::test]
async fn discovery_without_inline_metadata_does_not_end_the_handshake() {
    let out = handshake_replies(&[DISCOVER, INITIALIZE], 2).await.unwrap();
    assert_eq!(out[0]["id"], 1);
    assert_eq!(out[0]["error"]["code"], INVALID_PARAMS);
    assert!(out[1]["result"]["protocolVersion"].is_string());
}

#[tokio::test]
async fn inline_metadata_is_validated_by_the_sdk_before_dispatch() {
    let discover = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "server/discover",
        "params": {"_meta": {
            "io.modelcontextprotocol/protocolVersion": "2026-07-28"
        }}
    })
    .to_string();
    let out = handshake_replies(&[&discover, INITIALIZE], 2)
        .await
        .unwrap();
    assert_eq!(out[0]["id"], 1);
    assert_eq!(
        out[0]["error"]["code"], INVALID_PARAMS,
        "client capabilities are missing: {:?}",
        out[0]
    );
    assert!(out[0].get("result").is_none());
    assert!(out[1]["result"]["protocolVersion"].is_string());
}

#[tokio::test]
async fn discovery_does_not_bypass_initialization_for_legacy_tools() {
    const TOOLS_LIST: &str = r#"{"jsonrpc":"2.0","id":3,"method":"tools/list","params":{}}"#;
    let out = handshake_replies(&[DISCOVER_2026, TOOLS_LIST, INITIALIZE], 3)
        .await
        .unwrap();
    assert_eq!(out[0]["id"], 1);
    assert_eq!(out[0]["error"]["code"], -32022);
    assert_eq!(out[1]["id"], 3);
    assert_eq!(out[1]["error"]["code"], -32600);
    assert!(out[1].get("result").is_none());
    assert_eq!(out[2]["id"], 2);
    assert_eq!(out[2]["result"]["protocolVersion"], "2025-06-18");
}

#[tokio::test]
async fn discovery_metadata_is_validated_after_initialize() {
    const INITIALIZED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    let out = handshake_replies(&[INITIALIZE, INITIALIZED, DISCOVER], 2)
        .await
        .unwrap();
    assert_eq!(out[0]["id"], 2);
    assert_eq!(out[1]["id"], 1);
    assert_eq!(out[1]["error"]["code"], INVALID_PARAMS);
}

#[tokio::test]
async fn initialize_preserves_subsequent_notifications() {
    use rmcp::model::ClientJsonRpcMessage;
    use rmcp::transport::Transport;

    let (mut client, pipe) = tokio::io::duplex(4096);
    for line in [
        INITIALIZE,
        r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":1}}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"ping"}"#,
    ] {
        client.write_all(line.as_bytes()).await.unwrap();
        client.write_all(b"\n").await.unwrap();
    }
    let mut guarded = open_ontologies::mcp_handshake::guard(pipe);
    assert!(matches!(
        guarded.receive().await,
        Some(ClientJsonRpcMessage::Request(_))
    ));
    let next = tokio::time::timeout(std::time::Duration::from_secs(2), guarded.receive())
        .await
        .expect("the notification must reach the SDK after initialize");
    assert!(
        matches!(next, Some(ClientJsonRpcMessage::Notification(_))),
        "{next:?}"
    );
}
