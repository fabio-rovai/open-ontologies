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
        session.request("initialize", json!({"protocolVersion":"2025-03-26", "capabilities":{}, "clientInfo":{"name":"alignment-evaluation-test", "version":"1"}})).await;
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

use open_ontologies::eval_alignment::{AlignmentEntry, evaluate};

fn entry(source: &str, target: &str, relation: &str) -> AlignmentEntry {
    AlignmentEntry {
        source: source.into(),
        target: target.into(),
        relation: relation.into(),
    }
}

fn cases() -> Vec<(
    Vec<AlignmentEntry>,
    Vec<AlignmentEntry>,
    f64,
    f64,
    f64,
    usize,
    usize,
    usize,
)> {
    let a = entry(
        "https://example.org/Order",
        "https://example.org/Purchase",
        "equivalent",
    );
    let b = entry(
        "https://example.org/Quantity",
        "https://example.org/Amount",
        "equivalent",
    );
    let c = entry(
        "https://example.org/Customer",
        "https://example.org/Buyer",
        "subsumed_by",
    );
    let different_relation = entry(&a.source, &a.target, "subsumed_by");
    vec![
        (
            vec![a.clone(), b.clone()],
            vec![a.clone(), b.clone()],
            1.0,
            1.0,
            1.0,
            2,
            0,
            0,
        ),
        (
            vec![a.clone(), a.clone(), b.clone()],
            vec![a.clone(), b.clone()],
            1.0,
            1.0,
            1.0,
            2,
            0,
            0,
        ),
        (
            vec![a.clone(), b.clone()],
            vec![a.clone(), a.clone(), b.clone()],
            1.0,
            1.0,
            1.0,
            2,
            0,
            0,
        ),
        (
            vec![a.clone(), a.clone(), b.clone()],
            vec![a.clone(), b.clone(), b.clone()],
            1.0,
            1.0,
            1.0,
            2,
            0,
            0,
        ),
        (
            vec![a.clone(), a.clone(), b],
            vec![a.clone(), a.clone(), c],
            0.5,
            0.5,
            0.5,
            1,
            1,
            1,
        ),
        (
            vec![a.clone(), a.clone()],
            vec![different_relation.clone(), different_relation],
            0.0,
            0.0,
            0.0,
            0,
            1,
            1,
        ),
        (vec![a.clone(), a.clone()], vec![], 0.0, 0.0, 0.0, 0, 0, 1),
        (vec![], vec![a.clone(), a], 0.0, 0.0, 0.0, 0, 1, 0),
        (vec![], vec![], 0.0, 0.0, 0.0, 0, 0, 0),
    ]
}

fn assert_report(result: &Value, expected: (f64, f64, f64, usize, usize, usize)) {
    let (precision, recall, f1, tp, fp, fn_) = expected;
    assert_eq!(result["precision"], precision);
    assert_eq!(result["recall"], recall);
    assert_eq!(result["f1"], f1);
    assert_eq!(result["true_positive"], tp);
    assert_eq!(result["false_positive"], fp);
    assert_eq!(result["false_negative"], fn_);
    assert_eq!(result["reference_size"], tp + fn_);
    assert_eq!(result["computed_size"], tp + fp);
}

#[test]
fn repeated_alignment_entries_do_not_change_set_scores_or_relation_identity() {
    for (reference, computed, precision, recall, f1, tp, fp, fn_) in cases() {
        let result = serde_json::to_value(evaluate(&reference, &computed)).unwrap();
        assert_report(&result, (precision, recall, f1, tp, fp, fn_));
    }
}

#[tokio::test]
async fn public_mcp_alignment_scores_and_counts_agree_with_the_same_sets() {
    let tmp = tempfile::tempdir().unwrap();
    let mut mcp = Mcp::start(tmp.path()).await;
    for (reference, computed, precision, recall, f1, tp, fp, fn_) in cases() {
        let result = mcp
            .call(
                "onto_eval_alignment",
                json!({
                    "reference_json": serde_json::to_string(&reference).unwrap(),
                    "computed_json": serde_json::to_string(&computed).unwrap(),
                }),
            )
            .await;
        assert_report(&result, (precision, recall, f1, tp, fp, fn_));
    }
    mcp.child.kill().await.unwrap();
}
