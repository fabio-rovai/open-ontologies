use open_ontologies::ontology::OntologyService;
use open_ontologies::graph::GraphStore;
use open_ontologies::state::StateDb;
use std::sync::Arc;

fn test_db() -> StateDb {
    StateDb::open(std::path::Path::new(":memory:")).unwrap()
}

#[test]
fn test_save_and_list_versions() {
    let db = test_db();
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:Alice a ex:Person .
    "#;
    store.load_turtle(ttl, None).unwrap();

    let result = OntologyService::save_version(&db, &store, "v1.0");
    assert!(result.is_ok());

    let versions = OntologyService::list_versions(&db).unwrap();
    assert!(versions.contains("v1.0"));
}

#[test]
fn test_rollback_version() {
    let db = test_db();
    let store = Arc::new(GraphStore::new());

    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:Alice a ex:Person .
    "#;
    store.load_turtle(ttl, None).unwrap();
    OntologyService::save_version(&db, &store, "v1").unwrap();

    let ttl2 = r#"
        @prefix ex: <http://example.org/> .
        ex:Bob a ex:Person .
    "#;
    store.clear().unwrap();
    store.load_turtle(ttl2, None).unwrap();
    assert!(store.sparql_select("SELECT ?s WHERE { ?s a <http://example.org/Person> }").unwrap().contains("Bob"));

    let result = OntologyService::rollback_version(&db, &store, "v1");
    assert!(result.is_ok());
    let query_result = store.sparql_select("SELECT ?s WHERE { ?s a <http://example.org/Person> }").unwrap();
    assert!(query_result.contains("Alice"));
    assert!(!query_result.contains("Bob"));
}
