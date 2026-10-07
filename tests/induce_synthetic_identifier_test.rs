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
        session.request("initialize", json!({"protocolVersion":"2025-03-26", "capabilities":{}, "clientInfo":{"name":"synthetic-identifier-test", "version":"1"}})).await;
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
use open_ontologies::induce::induce;
use open_ontologies::ingest::DataIngester;

const BASE: &str = "https://example.org/orders/";
const DATA: &str = "kind,__row,quantity\nretail,import-A,3\nretail,import-A,5\nretail,import-A,8\n";
const QUERY: &str = "SELECT ?s ?kind ?source ?quantity WHERE { ?s <https://example.org/orders/ont#kind> ?kind; <https://example.org/orders/ont#__row> ?source; <https://example.org/orders/ont#quantity> ?quantity } ORDER BY ?quantity";

fn assert_three_retained_rows(result: &Value) {
    let rows = result["results"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    let subjects: std::collections::HashSet<_> =
        rows.iter().map(|row| row["s"].as_str().unwrap()).collect();
    assert_eq!(subjects.len(), 3);
    for (index, row) in rows.iter().enumerate() {
        assert_eq!(row["s"], format!("<{BASE}{}>", index + 1));
        assert_eq!(row["kind"], "\"retail\"");
        assert_eq!(row["source"], "\"import-A\"");
        assert_eq!(
            row["quantity"],
            format!(
                "\"{}\"^^<http://www.w3.org/2001/XMLSchema#integer>",
                [3, 5, 8][index]
            )
        );
    }
}

#[test]
fn synthetic_ids_preserve_original_cells_and_repeat_export_deterministically() {
    let rows = DataIngester::parse_csv(DATA).unwrap();
    let original = rows.clone();
    let induced = induce(
        &rows,
        &["kind".into(), "__row".into(), "quantity".into()],
        "Orders",
        BASE,
    );
    assert!(induced.id_synthesised);
    assert_eq!(induced.id_column, "__row_");
    assert_eq!(induced.mapping.id_field, "__row_");
    assert!(
        induced
            .columns
            .iter()
            .any(|column| column.columns == ["__row"])
    );
    let export = induced.instance_ntriples(&rows);
    assert_eq!(export, induced.instance_ntriples(&rows));
    assert_eq!(rows, original);
    let graph = GraphStore::new();
    graph.load_ntriples(&export).unwrap();
    assert_eq!(graph.triple_count(), 12);
    let result = serde_json::from_str(&graph.sparql_select(QUERY).unwrap()).unwrap();
    assert_three_retained_rows(&result);
}

#[test]
fn synthetic_field_skips_all_existing_names_and_explicit_identifiers_stay_unchanged() {
    let rows = DataIngester::parse_csv(
        "kind,__row,__row_,quantity\nretail,import-A,import-B,3\nretail,import-A,import-B,5\n",
    )
    .unwrap();
    let induced = induce(
        &rows,
        &[
            "kind".into(),
            "__row".into(),
            "__row_".into(),
            "quantity".into(),
        ],
        "Orders",
        BASE,
    );
    assert!(induced.id_synthesised);
    assert_eq!(induced.id_column, "__row__");
    let export = induced.instance_ntriples(&rows);
    assert!(export.contains("<https://example.org/orders/ont#__row> \"import-A\""));
    assert!(export.contains("<https://example.org/orders/ont#__row_> \"import-B\""));
    let rows =
        DataIngester::parse_csv("id,kind,quantity\norder-1,retail,3\norder-2,retail,5\n").unwrap();
    let induced = induce(
        &rows,
        &["id".into(), "kind".into(), "quantity".into()],
        "Orders",
        BASE,
    );
    assert!(!induced.id_synthesised);
    assert_eq!(induced.id_column, "id");
    assert_eq!(
        induced.instance_ntriples(&rows),
        induced.mapping.rows_to_ntriples(&rows)
    );
    assert!(
        induced
            .instance_ntriples(&rows)
            .contains("<https://example.org/orders/order-1>")
    );
}

#[tokio::test]
async fn batch_and_public_mcp_induction_load_the_same_three_rows() {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path().join("orders.csv");
    let commands = tmp.path().join("commands.txt");
    let batch = {
        let _gate = common::exec_gate();
        std::fs::write(&data, DATA).unwrap();
        std::fs::write(
            &commands,
            format!(
                "induce {} --base-iri {BASE}\nquery {QUERY}\n",
                data.display()
            ),
        )
        .unwrap();
        std::process::Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
            .args(["--no-connect", "--data-dir"])
            .arg(tmp.path())
            .arg("batch")
            .arg(commands)
            .output()
            .unwrap()
    };
    assert!(
        batch.status.success(),
        "{}",
        String::from_utf8_lossy(&batch.stderr)
    );
    let values: Vec<Value> = String::from_utf8(batch.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(values[0]["result"]["id_synthesised"], true);
    assert_eq!(values[0]["result"]["id_column"], "__row_");
    assert_three_retained_rows(&values[1]["result"]);
    let server_dir = tmp.path().join("mcp");
    std::fs::create_dir(&server_dir).unwrap();
    let mut mcp = Mcp::start(&server_dir).await;
    let induced = mcp
        .call("onto_induce", json!({"path":data, "base_iri":BASE}))
        .await;
    assert_eq!(induced["id_synthesised"], true);
    assert_eq!(induced["id_column"], "__row_");
    let queried = mcp.call("onto_query", json!({"query":QUERY})).await;
    assert_three_retained_rows(&queried);
    assert_eq!(queried, values[1]["result"]);
    mcp.child.kill().await.unwrap();
}
