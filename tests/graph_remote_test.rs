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
