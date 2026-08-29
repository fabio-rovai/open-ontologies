use open_ontologies::graph::GraphStore;

#[test]
fn test_snapshot_and_restore() {
    let store = GraphStore::new();
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:Alice a ex:Person .
        ex:Bob a ex:Person .
    "#;
    store.load_turtle(ttl, None).unwrap();
    assert_eq!(store.triple_count(), 2);

    // Snapshot
    let snapshot = store.snapshot("ntriples").unwrap();
    assert!(!snapshot.is_empty());
    assert!(snapshot.contains("Alice"));

    // Clear and restore
    store.clear().unwrap();
    assert_eq!(store.triple_count(), 0);

    store.load_ntriples(&snapshot).unwrap();
    assert_eq!(store.triple_count(), 2);
}

#[tokio::test]
async fn test_fetch_url_invalid() {
    let result = GraphStore::fetch_url("http://localhost:99999/nonexistent").await;
    assert!(result.is_err());
}

#[test]
fn test_load_ntriples() {
    let store = GraphStore::new();
    let nt = r#"<http://example.org/Alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .
"#;
    let result = store.load_ntriples(nt);
    assert!(result.is_ok());
    assert_eq!(store.triple_count(), 1);
}

// Red-team #7: the onto_push graph-name argument is spliced into a remote SPARQL
// UPDATE. A malicious value must be rejected as a bad IRI BEFORE any request, so
// it can never break out of the GRAPH clause into a DROP ALL.
#[tokio::test]
async fn push_rejects_a_graph_name_that_would_inject_an_update() {
    use open_ontologies::graph::{GraphStore, SparqlAuth};
    let auth = SparqlAuth::from_parts(None, None, None);
    let evil = "x> {} }; DROP ALL ; INSERT DATA { GRAPH <x";
    let result = GraphStore::push_sparql_auth(
        "http://localhost:1/never-reached",
        "<http://ex/s> <http://ex/p> <http://ex/o> .",
        Some(evil),
        &auth,
    )
    .await;
    let err = result.expect_err("a malicious graph name must be rejected");
    assert!(
        err.to_string().contains("not a valid absolute IRI"),
        "must fail IRI validation before any network I/O, got: {err}"
    );
}
