//! Native SELECT rows must reach the human renderer on local and proxy paths.

use open_ontologies::{graph::GraphStore, output};
use serde_json::{Value, json};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

const DATA: &str = r#"
    @prefix ex: <https://example.org/> .
    ex:alice ex:label "Alice"@en ; ex:count 7 .
    ex:bob ex:label "Bob"@fr .
"#;
const QUERY: &str = "SELECT ?s ?label ?count ?missing WHERE { ?s <https://example.org/label> ?label . OPTIONAL { ?s <https://example.org/count> ?count } OPTIONAL { ?s <https://example.org/missing> ?missing } } ORDER BY ?s";

fn assert_native_rows(value: &Value) {
    assert_eq!(
        value["variables"],
        json!(["s", "label", "count", "missing"])
    );
    let rows = value["results"]
        .as_array()
        .expect("native flat SELECT rows");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["s"], "<https://example.org/alice>");
    assert_eq!(rows[0]["label"], "\"Alice\"@en");
    assert_eq!(
        rows[0]["count"],
        "\"7\"^^<http://www.w3.org/2001/XMLSchema#integer>"
    );
    assert!(rows[0].get("missing").is_none());
    assert!(rows[1].get("count").is_none());
}

fn assert_native_table(rendered: &str) {
    assert!(
        !rendered.trim_start().starts_with('{'),
        "SELECT fell back to JSON: {rendered}"
    );
    let lines: Vec<_> = rendered.lines().collect();
    assert_eq!(lines.len(), 4, "header, separator and two rows: {rendered}");
    assert_eq!(
        lines[0].split_whitespace().collect::<Vec<_>>(),
        ["s", "label", "count", "missing"]
    );
    assert!(lines[1].chars().all(|c| c == '-' || c == ' '));
    assert!(lines[2].contains("<https://example.org/alice>"));
    assert!(lines[2].contains("\"Alice\"@en"));
    assert!(lines[2].contains("\"7\"^^<http://www.w3.org/2001/XMLSchema#integer>"));
    assert!(lines[3].contains("<https://example.org/bob>"));
    assert!(lines[3].contains("\"Bob\"@fr"));
    assert!(
        !lines[3].contains("integer"),
        "unbound cells must stay empty"
    );
}

#[test]
fn real_store_select_renders_flat_rows_on_both_entrypoints() {
    let store = GraphStore::new();
    store.load_turtle(DATA, None).unwrap();
    let result: Value = serde_json::from_str(&store.sparql_select(QUERY).unwrap()).unwrap();
    assert_native_rows(&result);
    for command in [None, Some("query")] {
        let rendered = output::render_human_for(command, &result);
        eprintln!("STORE {command:?}: {rendered}");
        assert_native_table(&rendered);
    }
}

#[test]
fn empty_store_select_renders_no_results() {
    let store = GraphStore::new();
    let result: Value = serde_json::from_str(&store.sparql_select(QUERY).unwrap()).unwrap();
    assert_eq!(result["results"], json!([]));
    for command in [None, Some("query")] {
        assert_eq!(output::render_human_for(command, &result), "No results.");
    }
}

#[test]
fn standard_bindings_and_non_select_fallbacks_keep_their_rendering() {
    let standard = json!({"variables": ["s", "label"], "results": {"bindings": [
        {"s": {"type": "uri", "value": "urn:item"}, "label": {"value": "Item", "xml:lang": "en"}},
        {"s": {"type": "uri", "value": "urn:other"}}
    ]}});
    let expected = "s          label\n---------  -----\nurn:item   Item \nurn:other";
    for command in [None, Some("query")] {
        assert_eq!(output::render_human_for(command, &standard), expected);
        assert_eq!(
            output::render_human_for(
                command,
                &json!({"variables":["s"],"results":{"bindings":[]}})
            ),
            "No results."
        );
        for value in [
            json!({"result": true}),
            json!({"results": [{"s": "ordinary result"}]}),
        ] {
            assert_eq!(
                output::render_human_for(command, &value),
                serde_json::to_string_pretty(&value).unwrap()
            );
        }
        assert_eq!(
            output::render_human_for(
                command,
                &json!({"error":"invalid SPARQL","variables":["s"],"results":[]})
            ),
            "Error: invalid SPARQL"
        );
    }
    // Preserve the existing CONSTRUCT rendering, even though it shares the
    // stats discriminator. This patch only changes SELECT recognition.
    let construct = json!({"triples":[{"subject":"urn:s","predicate":"urn:p","object":"urn:o"}]});
    assert_eq!(
        output::render_human(&construct),
        "Triples:     0\nClasses:     0\nProperties:  0\nIndividuals: 0"
    );
}

fn oo(dir: &tempfile::TempDir) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_open-ontologies"));
    cmd.arg("--data-dir").arg(dir.path());
    cmd.env_remove("OPEN_ONTOLOGIES_TOKEN")
        .env_remove("OPEN_ONTOLOGIES_STORAGE_MODE")
        .env_remove("GOVERNANCE_WEBHOOK")
        .env("NO_PROXY", "127.0.0.1,localhost")
        .env("no_proxy", "127.0.0.1,localhost")
        .env("RUST_LOG", "warn");
    cmd
}

fn run(mut cmd: Command) -> Output {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            let out = child.wait_with_output().unwrap();
            panic!("CLI timed out: {}", String::from_utf8_lossy(&out.stderr));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

fn query(dir: &tempfile::TempDir, flags: &[&str], sparql: &str) -> String {
    let mut cmd = oo(dir);
    cmd.args(flags).args(["query", sparql]);
    String::from_utf8(run(cmd).stdout).unwrap()
}

fn check_cli_queries(dir: &tempfile::TempDir, path: &str, local: bool) {
    let local_flags: &[&str] = if local { &["--no-connect"] } else { &[] };
    let mut human_flags = local_flags.to_vec();
    human_flags.push("--human");
    let human = query(dir, &human_flags, QUERY);
    let empty = query(dir, &human_flags, "SELECT ?s WHERE { ?s <urn:absent> ?o }");
    eprintln!("CLI {path} SELECT: {human}");
    eprintln!("CLI {path} EMPTY: {empty}");

    // Execute JSON controls before the human assertions, including precedence.
    for extra in [
        &[][..],
        &["--human", "--json"][..],
        &["--human", "--pretty"][..],
    ] {
        let mut flags = local_flags.to_vec();
        flags.extend_from_slice(extra);
        let value: Value = serde_json::from_str(&query(dir, &flags, QUERY)).unwrap();
        assert_native_rows(&value);
    }
    let ask = query(
        dir,
        &human_flags,
        "ASK { ?s <https://example.org/label> ?label }",
    );
    assert_eq!(
        serde_json::from_str::<Value>(&ask).unwrap(),
        json!({"result":true})
    );
    assert_native_table(&human);
    assert_eq!(empty.trim(), "No results.");
}

#[test]
fn built_cli_local_select_uses_human_table_and_preserves_json_precedence() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("config.toml"),
        "[storage]\nmode = 'persistent'\n",
    )
    .unwrap();
    let ttl = dir.path().join("data.ttl");
    std::fs::write(&ttl, DATA).unwrap();
    let mut load = oo(&dir);
    load.env("OPEN_ONTOLOGIES_STORAGE_MODE", "persistent")
        .arg("--no-connect")
        .arg("load")
        .arg(&ttl);
    let loaded: Value = serde_json::from_slice(&run(load).stdout).unwrap();
    assert_eq!(loaded["triples_loaded"], 3);
    check_cli_queries(&dir, "local", true);
}

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn built_cli_proxy_select_uses_real_http_store_and_human_dispatch() {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config.toml");
    std::fs::write(&config, "[storage]\nmode = 'memory'\n").unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    let mut cmd = oo(&dir);
    cmd.env("OPEN_ONTOLOGIES_STORAGE_MODE", "memory")
        .arg("serve-http")
        .arg("--config")
        .arg(&config)
        .args(["--host", "127.0.0.1", "--port", &addr.port().to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut server = Server(cmd.spawn().unwrap());
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(
            server.0.try_wait().unwrap().is_none(),
            "HTTP server exited before binding"
        );
        if std::net::TcpStream::connect(addr).is_ok() {
            break;
        }
        assert!(Instant::now() < deadline, "HTTP server did not bind");
        std::thread::sleep(Duration::from_millis(20));
    }
    // Advertise this owned, real serve-http child through the public daemon
    // record. It remains attached to the test so every exit reaps it.
    let info = open_ontologies::daemon::DaemonInfo {
        pid: server.0.id(),
        url: format!("http://{addr}"),
        token: None,
    };
    std::fs::write(
        dir.path().join("daemon.json"),
        serde_json::to_string(&info).unwrap(),
    )
    .unwrap();
    let ttl = dir.path().join("data.ttl");
    std::fs::write(&ttl, DATA).unwrap();
    let mut load = oo(&dir);
    load.env("OPEN_ONTOLOGIES_STORAGE_MODE", "memory")
        .arg("load")
        .arg(&ttl);
    let loaded: Value = serde_json::from_slice(&run(load).stdout).unwrap();
    assert_eq!(loaded["triples_loaded"], 3);
    // A local in-memory query must be empty: rows below can only come from
    // the actual proxy/server store, not a silent local fallback.
    let local: Value = serde_json::from_str(&query(&dir, &["--no-connect"], QUERY)).unwrap();
    assert_eq!(local["results"], json!([]));
    check_cli_queries(&dir, "proxy", false);
}
