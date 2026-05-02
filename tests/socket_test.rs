use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use open_ontologies::graph::GraphStore;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

const TEST_ONTOLOGY: &str = r#"
@prefix : <http://example.org/> .
@prefix schema: <http://schema.org/> .

:DoctorWho a schema:TVSeries ;
    schema:locationCreated :BBCTelevisionCentre ;
    schema:dateCreated "1963" ;
    schema:creator :SydneyNewman .

:BBCTelevisionCentre a schema:Place ;
    schema:address "London" .
"#;

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Helper: start the socket server in the background and return the path.
async fn start_server(graph: Arc<GraphStore>) -> String {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let sock_path = format!(
        "/tmp/oo-test-{}-{}.sock",
        std::process::id(),
        id
    );
    let _ = std::fs::remove_file(&sock_path);
    let path_clone = sock_path.clone();
    tokio::spawn(async move {
        open_ontologies::socket::serve(&path_clone, graph)
            .await
            .ok();
    });
    // Wait until the socket file actually exists
    for _ in 0..50 {
        if std::path::Path::new(&sock_path).exists() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    sock_path
}

/// Send a single request and read back the newline-delimited response.
async fn roundtrip(sock_path: &str, request: &str) -> serde_json::Value {
    let stream = UnixStream::connect(sock_path).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut line = format!("{}\n", request);
    writer.write_all(line.as_bytes()).await.unwrap();
    writer.shutdown().await.unwrap();

    let mut buf_reader = BufReader::new(reader);
    line.clear();
    buf_reader.read_line(&mut line).await.unwrap();
    serde_json::from_str(line.trim()).unwrap()
}

#[tokio::test]
async fn ground_existing_iri_triple() {
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(TEST_ONTOLOGY, None).unwrap();
    let sock = start_server(graph).await;

    let resp = roundtrip(
        &sock,
        r#"{"action":"ground","triples":[{"s":"http://example.org/DoctorWho","p":"http://schema.org/locationCreated","o":"http://example.org/BBCTelevisionCentre"}]}"#,
    )
    .await;

    let status = resp["results"][0]["status"].as_str().unwrap();
    assert_eq!(status, "grounded");
    assert!(resp["results"][0]["confidence"].as_u64().unwrap() > 0);

    std::fs::remove_file(&sock).ok();
}

#[tokio::test]
async fn ground_existing_literal_triple() {
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(TEST_ONTOLOGY, None).unwrap();
    let sock = start_server(graph).await;

    let resp = roundtrip(
        &sock,
        r#"{"action":"ground","triples":[{"s":"http://example.org/DoctorWho","p":"http://schema.org/dateCreated","o":"1963"}]}"#,
    )
    .await;

    let status = resp["results"][0]["status"].as_str().unwrap();
    assert_eq!(status, "grounded");

    std::fs::remove_file(&sock).ok();
}

#[tokio::test]
async fn ground_unknown_triple() {
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(TEST_ONTOLOGY, None).unwrap();
    let sock = start_server(graph).await;

    let resp = roundtrip(
        &sock,
        r#"{"action":"ground","triples":[{"s":"http://example.org/Unknown","p":"http://schema.org/name","o":"Foo"}]}"#,
    )
    .await;

    let status = resp["results"][0]["status"].as_str().unwrap();
    assert_eq!(status, "unknown");

    std::fs::remove_file(&sock).ok();
}

#[tokio::test]
async fn check_consistency_clean() {
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(TEST_ONTOLOGY, None).unwrap();
    let sock = start_server(graph).await;

    let resp = roundtrip(
        &sock,
        r#"{"action":"check_consistency","triples":[{"s":"http://example.org/DoctorWho","p":"http://schema.org/dateCreated","o":"1963"}]}"#,
    )
    .await;

    assert!(resp["consistent"].as_bool().unwrap());
    assert_eq!(resp["contradiction_count"].as_u64().unwrap(), 0);

    std::fs::remove_file(&sock).ok();
}

#[tokio::test]
async fn unknown_action_returns_error() {
    let graph = Arc::new(GraphStore::new());
    let sock = start_server(graph).await;

    let resp = roundtrip(
        &sock,
        r#"{"action":"bogus","triples":[]}"#,
    )
    .await;

    assert!(resp["error"].as_str().unwrap().contains("unknown action"));

    std::fs::remove_file(&sock).ok();
}

#[tokio::test]
async fn bad_json_returns_error() {
    let graph = Arc::new(GraphStore::new());
    let sock = start_server(graph).await;

    let resp = roundtrip(&sock, "not json at all").await;
    assert!(resp["error"].as_str().unwrap().contains("bad request"));

    std::fs::remove_file(&sock).ok();
}
