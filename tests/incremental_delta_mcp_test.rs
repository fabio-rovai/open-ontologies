// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Strict delta parsing and durable withdrawal must coexist at the MCP boundary.

use open_ontologies::config::{CacheConfig, EmbeddingsConfig};
use open_ontologies::graph::GraphStore;
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;
use open_ontologies::toolfilter::ToolFilter;
use rmcp::ServiceExt;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const DELTA: &str =
    "<urn:alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <urn:Person> . # source fact\n";

async fn call(graph: &Arc<GraphStore>, arguments: Value) -> Value {
    let tmp = tempfile::tempdir().unwrap();
    let server = OpenOntologiesServer::new_with_registry_options(
        StateDb::open(&tmp.path().join("state.db")).unwrap(),
        graph.clone(),
        None,
        EmbeddingsConfig {
            model_path: Some(
                tmp.path()
                    .join("absent.onnx")
                    .to_string_lossy()
                    .into_owned(),
            ),
            tokenizer_path: Some(
                tmp.path()
                    .join("absent.json")
                    .to_string_lossy()
                    .into_owned(),
            ),
            ..Default::default()
        },
        CacheConfig {
            dir: tmp.path().join("cache").to_string_lossy().into_owned(),
            ..Default::default()
        },
        ToolFilter::default(),
    );
    let (mut client, pipe) = tokio::io::duplex(64 * 1024);
    for message in [
        json!({"jsonrpc":"2.0", "id":1, "method":"initialize", "params":{
            "protocolVersion":"2025-06-18", "capabilities":{},
            "clientInfo":{"name":"incremental-delta-test", "version":"1"}
        }}),
        json!({"jsonrpc":"2.0", "method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0", "id":2, "method":"tools/call", "params":{
            "name":"onto_reason_incremental", "arguments":arguments
        }}),
    ] {
        client
            .write_all(message.to_string().as_bytes())
            .await
            .unwrap();
        client.write_all(b"\n").await.unwrap();
    }
    client.flush().await.unwrap();
    let service = server.serve(pipe).await.unwrap();
    let mut reader = BufReader::new(client).lines();
    let result = loop {
        let line = tokio::time::timeout(std::time::Duration::from_secs(10), reader.next_line())
            .await
            .expect("MCP response timeout")
            .unwrap()
            .expect("MCP output closed");
        let response: Value = serde_json::from_str(&line).unwrap();
        if response["id"] == 2 {
            assert!(response.get("error").is_none(), "{response}");
            break serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap())
                .unwrap();
        }
    };
    drop(service);
    result
}

fn graph() -> Arc<GraphStore> {
    let graph = Arc::new(GraphStore::new());
    graph
        .load_ntriples(
            "<urn:Person> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <urn:Agent> .",
        )
        .unwrap();
    graph.load_ntriples(DELTA).unwrap();
    graph
}

fn has_agent(graph: &GraphStore) -> bool {
    graph.all_triples().unwrap().contains(&(
        "<urn:alice>".into(),
        "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>".into(),
        "<urn:Agent>".into(),
    ))
}

#[tokio::test]
async fn commented_deltas_preserve_the_tracking_default_and_opt_in() {
    for tracking in [false, true] {
        let graph = graph();
        let arguments = if tracking {
            json!({"delta":DELTA, "track_derivations":true})
        } else {
            json!({"delta":DELTA})
        };
        let result = call(&graph, arguments).await;
        assert!(result.get("error").is_none(), "{result}");
        assert!(
            has_agent(&graph),
            "the comment must not become part of the object"
        );
        assert_eq!(graph.ledger().unwrap().is_some(), tracking);
    }
}

#[tokio::test]
async fn malformed_and_mixed_deltas_leave_support_unchanged_before_valid_withdrawal() {
    let graph = graph();
    let result = call(&graph, json!({"delta":DELTA, "track_derivations":true})).await;
    assert!(result.get("error").is_none(), "{result}");
    let before = graph.snapshot("nquads").unwrap();
    for arguments in [
        json!({"delta":format!("{DELTA}not a triple"), "track_derivations":true}),
        json!({"removed":format!("{DELTA}not a triple")}),
        json!({"delta":DELTA, "removed":DELTA}),
        json!({"removed":DELTA, "materialize":false}),
    ] {
        let result = call(&graph, arguments).await;
        assert!(result["error"].is_string(), "{result}");
        assert_eq!(graph.snapshot("nquads").unwrap(), before);
        assert!(has_agent(&graph));
    }
    let result = call(&graph, json!({"removed":DELTA})).await;
    assert!(result.get("error").is_none(), "{result}");
    assert!(
        !has_agent(&graph),
        "withdrawal must retract the unsupported inference"
    );
    assert_eq!(graph.triple_count(), 1, "only the schema assertion remains");
}
