mod common;

use serde_json::{Value, json};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

struct Mcp {
    child: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    stdout: tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    next_id: u64,
}

impl Mcp {
    async fn start(directory: &std::path::Path) -> Self {
        let (child, stdin, stdout) = {
            let _gate = common::exec_gate();
            let config = directory.join("config.toml");
            std::fs::write(
                &config,
                format!(
                    "[cache]\ndir = {}\n[embeddings]\nmodel_path = {}\ntokenizer_path = {}\n",
                    json!(directory.join("cache").to_string_lossy()),
                    json!(directory.join("absent.onnx").to_string_lossy()),
                    json!(directory.join("absent-tokenizer.json").to_string_lossy()),
                ),
            )
            .unwrap();
            let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
                .args(["--no-connect", "--data-dir"])
                .arg(directory)
                .args(["serve", "--config"])
                .arg(config)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .kill_on_drop(true)
                .spawn()
                .unwrap();
            let stdin = child.stdin.take().unwrap();
            let stdout = BufReader::new(child.stdout.take().unwrap()).lines();
            (child, stdin, stdout)
        };
        let mut session = Self {
            child,
            stdin,
            stdout,
            next_id: 1,
        };
        session.request("initialize", json!({"protocolVersion":"2025-03-26", "capabilities":{}, "clientInfo":{"name":"version-dataset-test", "version":"1"}})).await;
        session
            .write(json!({"jsonrpc":"2.0", "method":"notifications/initialized"}))
            .await;
        session
    }

    async fn write(&mut self, message: Value) {
        self.stdin
            .write_all(message.to_string().as_bytes())
            .await
            .unwrap();
        self.stdin.write_all(b"\n").await.unwrap();
        self.stdin.flush().await.unwrap();
    }

    async fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.write(json!({"jsonrpc":"2.0", "id":id, "method":method, "params":params}))
            .await;
        loop {
            let line =
                tokio::time::timeout(std::time::Duration::from_secs(20), self.stdout.next_line())
                    .await
                    .expect("MCP response timeout")
                    .unwrap()
                    .expect("MCP output closed");
            let response: Value = serde_json::from_str(&line).unwrap();
            if response["id"] == id {
                assert!(response.get("error").is_none(), "{response}");
                return response["result"].clone();
            }
        }
    }

    async fn call(&mut self, name: &str, arguments: Value) -> Value {
        let result = self
            .request("tools/call", json!({"name":name, "arguments":arguments}))
            .await;
        let value: Value =
            serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap();
        assert!(value.get("error").is_none(), "{value}");
        value
    }
}

use open_ontologies::graph::GraphStore;
use open_ontologies::ontology::OntologyService;
use open_ontologies::state::StateDb;
use std::sync::Arc;

const DATASET: &str = r#"
@prefix ex: <https://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
ex:Order a owl:Class .
ex:warehouse { ex:order-1 ex:status "pending" . }
ex:store { ex:order-1 ex:status "pending" . }
"#;
const DEFAULT: &str = "<https://example.org/order-1> <https://example.org/status> \"pending\" .";
const NAMED_QUERY: &str = "SELECT ?g ?s ?status WHERE { GRAPH ?g { ?s <https://example.org/status> ?status } } ORDER BY ?g";
const DEFAULT_QUERY: &str = "SELECT ?s ?status WHERE { ?s <https://example.org/status> ?status }";

fn snapshot_lines(graph: &GraphStore) -> std::collections::BTreeSet<String> {
    graph
        .serialize("nquads")
        .unwrap()
        .lines()
        .map(String::from)
        .collect()
}

#[test]
fn saved_versions_retain_default_named_and_inferred_graphs_without_collapsing_facts() {
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("state.db")).unwrap();
    for (name, source, format, count) in [
        (
            "dataset",
            DATASET.to_string(),
            oxigraph::io::RdfFormat::TriG,
            3,
        ),
        (
            "default",
            DEFAULT.to_string(),
            oxigraph::io::RdfFormat::NTriples,
            1,
        ),
        (
            "inferred",
            format!(
                "<https://example.org/order-1> <https://example.org/derivedStatus> \"pending\" <{}> .",
                open_ontologies::reason::INFERRED_GRAPH
            ),
            oxigraph::io::RdfFormat::NQuads,
            1,
        ),
    ] {
        let graph = Arc::new(GraphStore::new());
        assert_eq!(graph.load_content(&source, format).unwrap(), count);
        let expected = snapshot_lines(&graph);
        let saved: Value =
            serde_json::from_str(&OntologyService::save_version(&db, &graph, name).unwrap())
                .unwrap();
        assert_eq!(saved["triple_count"], count);
        let history: Value =
            serde_json::from_str(&OntologyService::list_versions(&db).unwrap()).unwrap();
        assert_eq!(history["versions"][0]["format"], "nquads");
        graph.clear().unwrap();
        graph
            .load_turtle(
                "<https://example.org/temporary> <https://example.org/status> \"changed\" .",
                None,
            )
            .unwrap();
        let restored: Value =
            serde_json::from_str(&OntologyService::rollback_version(&db, &graph, name).unwrap())
                .unwrap();
        assert_eq!(restored["triples_restored"], count);
        assert_eq!(graph.triple_count(), count);
        assert_eq!(snapshot_lines(&graph), expected);
    }
}

#[test]
fn legacy_ntriples_versions_remain_readable() {
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("state.db")).unwrap();
    db.conn().execute("INSERT INTO ontology_versions (label, triple_count, content, format) VALUES (?1, ?2, ?3, ?4)", rusqlite::params!["legacy", 1, DEFAULT, "ntriples"]).unwrap();
    let graph = Arc::new(GraphStore::new());
    graph
        .load_turtle(
            "<https://example.org/temporary> <https://example.org/status> \"changed\" .",
            None,
        )
        .unwrap();
    let restored: Value =
        serde_json::from_str(&OntologyService::rollback_version(&db, &graph, "legacy").unwrap())
            .unwrap();
    assert_eq!(restored["triples_restored"], 1);
    assert_eq!(graph.triple_count(), 1);
    let rows: Value = serde_json::from_str(&graph.sparql_select(DEFAULT_QUERY).unwrap()).unwrap();
    assert_eq!(rows["results"].as_array().unwrap().len(), 1);
    assert_eq!(rows["results"][0]["s"], "<https://example.org/order-1>");
}

#[tokio::test]
async fn public_mcp_version_and_rollback_preserve_dataset_query_scopes() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("orders.trig");
    std::fs::write(&path, DATASET).unwrap();
    let mut mcp = Mcp::start(tmp.path()).await;
    let loaded = mcp.call("onto_load", json!({"path":path})).await;
    assert_eq!(loaded["triples_loaded"], 3);
    let before = mcp.call("onto_query", json!({"query":NAMED_QUERY})).await;
    assert_eq!(before["results"].as_array().unwrap().len(), 2);
    let empty_default = mcp.call("onto_query", json!({"query":DEFAULT_QUERY})).await;
    assert!(empty_default["results"].as_array().unwrap().is_empty());
    let saved = mcp.call("onto_version", json!({"label":"dataset"})).await;
    assert_eq!(saved["triple_count"], 3);
    let restored = mcp.call("onto_rollback", json!({"label":"dataset"})).await;
    assert_eq!(restored["triples_restored"], 3);
    let after = mcp.call("onto_query", json!({"query":NAMED_QUERY})).await;
    assert_eq!(after, before);
    let after_default = mcp.call("onto_query", json!({"query":DEFAULT_QUERY})).await;
    assert_eq!(after_default, empty_default);
    let history = mcp.call("onto_history", json!({})).await;
    assert_eq!(history["versions"][0]["format"], "nquads");
    mcp.child.kill().await.unwrap();
}
