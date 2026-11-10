// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Exercise the public MCP wire format across SDK upgrades, including older
//! clients and both HTTP session modes. Calling handler methods directly would
//! miss changes in routing, protocol negotiation and response serialization.

use open_ontologies::config::{CacheConfig, EmbeddingsConfig};
use open_ontologies::graph::GraphStore;
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;
use open_ontologies::toolfilter::ToolFilter;
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

struct ServerTask(tokio::task::JoinHandle<std::io::Result<()>>);

impl Drop for ServerTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}

async fn rpc_response(response: reqwest::Response, id: u64) -> Value {
    let status = response.status();
    let body = response.text().await.unwrap();
    assert!(status.is_success(), "HTTP {status}: {body}");
    // Streamable HTTP permits JSON or an SSE stream for a request reply.
    let reply: Value = if let Ok(value) = serde_json::from_str(&body) {
        value
    } else {
        body.lines()
            .filter_map(|line| line.strip_prefix("data:"))
            .filter_map(|data| serde_json::from_str::<Value>(data.trim()).ok())
            .find(|value| value["id"] == id)
            .unwrap_or_else(|| panic!("no reply for request {id}: {body}"))
    };
    assert_eq!(reply["jsonrpc"], "2.0");
    assert_eq!(reply["id"], id);
    assert!(reply.get("error").is_none(), "{reply}");
    reply["result"].clone()
}

async fn roundtrip(stateful: bool, protocol: &str) {
    let dir = tempfile::tempdir().unwrap();
    let server = OpenOntologiesServer::new_with_registry_options(
        StateDb::open(&dir.path().join("state.db")).unwrap(),
        Arc::new(GraphStore::new()),
        None,
        EmbeddingsConfig {
            model_path: Some(
                dir.path()
                    .join("absent.onnx")
                    .to_string_lossy()
                    .into_owned(),
            ),
            tokenizer_path: Some(
                dir.path()
                    .join("absent.json")
                    .to_string_lossy()
                    .into_owned(),
            ),
            ..Default::default()
        },
        CacheConfig {
            dir: dir.path().join("cache").to_string_lossy().into_owned(),
            ..Default::default()
        },
        ToolFilter::default(),
    );
    #[allow(clippy::field_reassign_with_default)]
    let config = {
        let mut config = StreamableHttpServerConfig::default();
        config.legacy_session_mode = stateful;
        config
    };
    let service: StreamableHttpService<_, LocalSessionManager> = StreamableHttpService::new(
        move || Ok(server.clone()),
        LocalSessionManager::default().into(),
        config,
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    let router = axum::Router::new().nest_service("/mcp", service);
    let _task = ServerTask(tokio::spawn(
        async move { axum::serve(listener, router).await },
    ));
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();
    let request = || {
        client
            .post(&url)
            .header("Accept", "application/json, text/event-stream")
    };
    let response = request()
        .json(&json!({
            "jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {
                "protocolVersion": protocol, "capabilities": {},
                "clientInfo": {"name": "http-compatibility", "version": "1"}
            }
        }))
        .send()
        .await
        .unwrap();
    let session = response.headers().get("mcp-session-id").cloned();
    assert_eq!(session.is_some(), stateful);
    let initialized = rpc_response(response, 1).await;
    assert_eq!(initialized["protocolVersion"], protocol);
    assert!(initialized["capabilities"]["tools"].is_object());
    assert!(initialized["capabilities"]["prompts"].is_object());

    let request = || {
        let request = request().header("MCP-Protocol-Version", protocol);
        match &session {
            Some(session) => request.header("Mcp-Session-Id", session),
            None => request,
        }
    };
    let notified = request()
        .json(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
        .send()
        .await
        .unwrap();
    assert_eq!(notified.status(), reqwest::StatusCode::ACCEPTED);
    for (id, method, params, field, expected) in [
        (2, "tools/list", json!({}), "tools", "onto_status"),
        (3, "prompts/list", json!({}), "prompts", "explore_ontology"),
    ] {
        let response = request()
            .json(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))
            .send()
            .await
            .unwrap();
        let result = rpc_response(response, id).await;
        assert!(
            result[field]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["name"] == expected)
        );
    }
    let response = request()
        .json(&json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call",
            "params": {"name": "onto_status", "arguments": {}}}))
        .send()
        .await
        .unwrap();
    let result = rpc_response(response, 4).await;
    assert_ne!(result["isError"], true, "{result}");
    let status: Value =
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
    assert!(status["tools"].as_u64().unwrap() > 0);

    let response = request()
        .json(&json!({"jsonrpc": "2.0", "id": 5, "method": "prompts/get",
            "params": {"name": "explore_ontology"}}))
        .send()
        .await
        .unwrap();
    let result = rpc_response(response, 5).await;
    assert_eq!(result["messages"][0]["role"], "user");
    assert!(
        result["messages"][0]["content"]["text"]
            .as_str()
            .unwrap()
            .contains("onto_stats")
    );
}

#[tokio::test]
async fn stateful_http_preserves_tools_and_prompts_for_existing_clients() {
    for protocol in ["2025-03-26", "2025-06-18", "2025-11-25"] {
        roundtrip(true, protocol).await;
    }
}

#[tokio::test]
async fn stateless_http_preserves_tools_and_prompts_for_existing_clients() {
    for protocol in ["2025-03-26", "2025-06-18", "2025-11-25"] {
        roundtrip(false, protocol).await;
    }
}
