use open_ontologies::graph::GraphStore;

#[test]
fn test_create_graph_store() {
    let store = GraphStore::new();
    assert_eq!(store.triple_count(), 0);
}

#[test]
fn test_load_turtle() {
    let store = GraphStore::new();
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:Alice a ex:Person .
        ex:Alice ex:name "Alice" .
    "#;
    let result = store.load_turtle(ttl, None);
    assert!(result.is_ok());
    assert_eq!(store.triple_count(), 2);
}

#[test]
fn test_sparql_select() {
    let store = GraphStore::new();
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:Alice a ex:Person .
        ex:Bob a ex:Person .
    "#;
    store.load_turtle(ttl, None).unwrap();
    let result = store
        .sparql_select("SELECT ?s WHERE { ?s a <http://example.org/Person> }")
        .unwrap();
    assert!(result.contains("Alice"));
    assert!(result.contains("Bob"));
}

#[test]
fn test_validate_turtle_valid() {
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:Alice a ex:Person .
    "#;
    let result = GraphStore::validate_turtle(ttl);
    assert!(result.is_ok());
}

#[test]
fn test_validate_turtle_invalid() {
    let ttl = "this is not valid turtle @@@ garbage";
    let result = GraphStore::validate_turtle(ttl);
    assert!(result.is_err());
}

#[test]
fn test_convert_turtle_to_ntriples() {
    let store = GraphStore::new();
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:Alice a ex:Person .
    "#;
    store.load_turtle(ttl, None).unwrap();
    let nt = store.serialize("ntriples").unwrap();
    assert!(nt.contains("<http://example.org/Alice>"));
    assert!(nt.contains("<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"));
}

#[test]
fn test_load_from_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.ttl");
    std::fs::write(&path, r#"
        @prefix ex: <http://example.org/> .
        ex:Alice a ex:Person .
    "#).unwrap();

    let store = GraphStore::new();
    let result = store.load_file(path.to_str().unwrap());
    assert!(result.is_ok());
    assert_eq!(store.triple_count(), 1);
}

#[test]
fn test_get_stats() {
    let store = GraphStore::new();
    let ttl = r#"
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Person a owl:Class .
        ex:name a owl:DatatypeProperty .
        ex:Alice a ex:Person .
    "#;
    store.load_turtle(ttl, None).unwrap();
    let stats = store.get_stats().unwrap();
    assert!(stats.contains("classes"));
    assert!(stats.contains("triples"));
}
