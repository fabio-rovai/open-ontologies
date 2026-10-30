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
        session.request("initialize", json!({"protocolVersion":"2025-03-26", "capabilities":{}, "clientInfo":{"name":"schema-discovery-test", "version":"1"}})).await;
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

const TTL: &str = r#"
@prefix ex: <https://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
ex:Order a owl:Class; rdfs:label "Order"; rdfs:comment "A customer order" .
ex:quantity a owl:DatatypeProperty; rdfs:domain ex:Order; rdfs:range xsd:integer; rdfs:label "quantity" .
"#;

#[tokio::test]
async fn mapping_and_scaffold_discover_the_same_nonempty_schema_in_all_graph_formats() {
    let graph = open_ontologies::graph::GraphStore::new();
    let (prefixes, body) = TTL.split_once("ex:Order").unwrap();
    let trig = format!("{prefixes}<https://example.org/schema> {{ ex:Order{body} }}");
    graph
        .load_content(&trig, oxigraph::io::RdfFormat::TriG)
        .unwrap();
    let nquads = graph.serialize("nquads").unwrap();
    let mut expected = None;
    for (extension, source) in [
        ("ttl", TTL),
        ("trig", trig.as_str()),
        ("nq", nquads.as_str()),
    ] {
        let tmp = tempfile::tempdir().unwrap();
        let ontology = tmp.path().join(format!("orders.{extension}"));
        let data = tmp.path().join("orders.csv");
        std::fs::write(&ontology, source).unwrap();
        std::fs::write(&data, "id,quantity\norder-1,3\n").unwrap();
        let mut mcp = Mcp::start(tmp.path()).await;
        let load = mcp.call("onto_load", json!({"path":ontology})).await;
        assert_eq!(load["triples_loaded"], 7);
        let mapping = mcp.call("onto_map", json!({"data_path":data})).await;
        assert_eq!(
            mapping["ontology_classes"],
            json!(["https://example.org/Order"])
        );
        assert_eq!(
            mapping["ontology_properties"],
            json!(["https://example.org/quantity"])
        );
        assert_eq!(mapping["mapping"]["class"], "https://example.org/Order");
        if let Some(ref expected) = expected {
            assert_eq!(&mapping, expected);
        } else {
            expected = Some(mapping);
        }
        let scaffold = mcp
            .call(
                "onto_extract_scaffold",
                json!({"class_iri":"https://example.org/Order"}),
            )
            .await;
        assert_eq!(scaffold["property_schema"].as_array().unwrap().len(), 1);
        let invalid = mcp.call("onto_extract_validate", json!({"scaffold_json":scaffold.to_string(), "extraction_json":"[{\"quantity\":\"three\"}]"})).await;
        assert_eq!(invalid["valid"], 0);
        let user = mcp
            .call(
                "onto_query",
                json!({"query":"SELECT ?c WHERE { ?c a <http://www.w3.org/2002/07/owl#Class> }"}),
            )
            .await;
        assert_eq!(
            user["results"].as_array().unwrap().len(),
            usize::from(extension == "ttl")
        );
        let explicit = json!({"base_iri":"https://example.org/rows/", "id_field":"id", "class":"https://example.org/ExplicitOrder", "mappings":[{"field":"quantity", "predicate":"https://example.org/quantity", "datatype":"http://www.w3.org/2001/XMLSchema#integer"}]});
        let ingest = mcp
            .call(
                "onto_ingest",
                json!({"path":data, "inline_mapping":true, "mapping":explicit.to_string()}),
            )
            .await;
        assert_eq!(ingest["rows_processed"], 1);
        let overridden = mcp
            .call(
                "onto_query",
                json!({"query":"SELECT ?c WHERE { <https://example.org/rows/order-1> a ?c }"}),
            )
            .await;
        assert_eq!(
            overridden["results"][0]["c"],
            "<https://example.org/ExplicitOrder>"
        );
        mcp.child.kill().await.unwrap();
    }
}
