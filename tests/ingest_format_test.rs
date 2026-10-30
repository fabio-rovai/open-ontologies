use open_ontologies::ingest::DataIngester;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

#[test]
fn cli_explicit_json_format_overrides_non_json_extensions() {
    let dir = tempfile::tempdir().unwrap();
    for extension in ["data", "csv"] {
        let file = dir.path().join(format!("records.{extension}"));
        std::fs::write(&file, r#"[{"id":"b1","name":"Tower Bridge"}]"#).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
            .args(["--no-connect", "--data-dir"])
            .arg(dir.path().join(format!("state-{extension}")))
            .arg("ingest")
            .arg(&file)
            .args(["--format", "json"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(result["rows"], 1, "{result}");
        assert_eq!(result["triples_loaded"], 3, "{result}");
    }
}

#[test]
fn omitted_format_keeps_extension_detection_and_csv_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let json_file = dir.path().join("records.json");
    std::fs::write(&json_file, r#"[{"id":"b1","name":"Tower Bridge"}]"#).unwrap();
    let rows = DataIngester::parse_file(json_file.to_str().unwrap()).unwrap();
    assert_eq!(rows[0]["name"], "Tower Bridge");
    let csv_file = dir.path().join("records.data");
    std::fs::write(&csv_file, "id,name\nb1,Tower Bridge\n").unwrap();
    let rows = DataIngester::parse_file(csv_file.to_str().unwrap()).unwrap();
    assert_eq!(rows[0]["name"], "Tower Bridge");
}

#[test]
fn explicit_formats_share_the_existing_parsers_and_reject_unknown_formats() {
    let dir = tempfile::tempdir().unwrap();
    for (format, content) in [
        ("json", r#"[{"id":"b1","name":"Tower Bridge"}]"#),
        ("ndjson", "{\"id\":\"b1\",\"name\":\"Tower Bridge\"}\n"),
        ("csv", "id,name\nb1,Tower Bridge\n"),
        ("yaml", "- id: b1\n  name: Tower Bridge\n"),
        (
            "xml",
            "<records><record><id>b1</id><name>Tower Bridge</name></record></records>",
        ),
    ] {
        let file = dir.path().join(format!("{format}.data"));
        std::fs::write(&file, content).unwrap();
        let rows =
            DataIngester::parse_file_with_format(file.to_str().unwrap(), Some(format)).unwrap();
        assert_eq!(rows[0]["id"], "b1");
        assert_eq!(rows[0]["name"], "Tower Bridge");
    }
    let missing = dir.path().join("missing.data");
    let error = DataIngester::parse_file_with_format(missing.to_str().unwrap(), Some("unknown"))
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Unsupported data format: unknown")
    );
}

struct McpChild {
    child: Child,
    input: ChildStdin,
    lines: Receiver<String>,
    reader: Option<std::thread::JoinHandle<()>>,
}

impl McpChild {
    fn start(state: &std::path::Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
            .args(["--no-connect", "--data-dir"])
            .arg(state)
            .arg("serve")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (send, lines) = channel();
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(line) => {
                        if send.send(line).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            child,
            input,
            lines,
            reader: Some(reader),
        }
    }

    fn send(&mut self, message: Value) {
        writeln!(self.input, "{message}").unwrap();
        self.input.flush().unwrap();
    }

    fn request(&mut self, id: u64, method: &str, params: Value) -> Value {
        self.send(json!({"jsonrpc":"2.0", "id":id, "method":method, "params":params}));
        loop {
            let line = self
                .lines
                .recv_timeout(Duration::from_secs(10))
                .expect("MCP response timed out or server exited");
            let response: Value = serde_json::from_str(&line).unwrap();
            if response["id"] == id {
                assert!(response.get("error").is_none(), "{response}");
                return response["result"].clone();
            }
        }
    }

    fn tool(&mut self, id: u64, name: &str, arguments: Value) -> Value {
        let result = self.request(
            id,
            "tools/call",
            json!({"name":name, "arguments":arguments}),
        );
        serde_json::from_str(result["content"][0]["text"].as_str().unwrap()).unwrap()
    }
}

impl Drop for McpChild {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

#[test]
fn mcp_ingest_and_map_honor_their_documented_format_field() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("records.data");
    std::fs::write(&file, r#"[{"id":"b1","name":"Tower Bridge"}]"#).unwrap();
    let mut mcp = McpChild::start(&dir.path().join("state"));
    mcp.request(
        1,
        "initialize",
        json!({
            "protocolVersion":"2025-03-26", "capabilities":{},
            "clientInfo":{"name":"ingest-format-test", "version":"1"}
        }),
    );
    mcp.send(json!({"jsonrpc":"2.0", "method":"notifications/initialized"}));
    let ingested = mcp.tool(
        2,
        "onto_ingest",
        json!({
            "path":file, "format":"json", "base_iri":"https://example.org/"
        }),
    );
    assert_eq!(ingested["rows_processed"], 1, "{ingested}");
    assert_eq!(ingested["triples_loaded"], 3, "{ingested}");
    let mapping = mcp.tool(3, "onto_map", json!({"data_path":file, "format":"json"}));
    assert_eq!(mapping["data_fields"], json!(["id", "name"]));
    let invalid = mcp.tool(4, "onto_ingest", json!({"path":file, "format":"unknown"}));
    assert!(
        invalid["error"]
            .as_str()
            .unwrap()
            .contains("Unsupported data format: unknown")
    );
}
