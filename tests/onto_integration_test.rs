use open_ontologies::graph::GraphStore;
use open_ontologies::ontology::OntologyService;

#[test]
fn test_full_workflow_load_query_validate() {
    // 1. Load
    let store = GraphStore::new();
    let loaded = store.load_file("tests/data/sample.ttl");
    assert!(loaded.is_ok());
    let count = loaded.unwrap();
    assert!(count > 0);

    // 2. Query
    let result = store.sparql_select(
        "SELECT ?person WHERE { ?person a <http://example.org/test#Person> }"
    );
    assert!(result.is_ok());
    let json = result.unwrap();
    assert!(json.contains("Alice"));

    // 3. Stats
    let stats = store.get_stats().unwrap();
    assert!(stats.contains("\"classes\""));

    // 4. Validate the file
    let valid = OntologyService::validate_file("tests/data/sample.ttl");
    assert!(valid.is_ok());
    let report = valid.unwrap();
    assert!(report.contains("\"valid\":true"));

    // 5. Lint
    let content = std::fs::read_to_string("tests/data/sample.ttl").unwrap();
    let lint = OntologyService::lint(&content).unwrap();
    assert!(lint.contains("\"issue_count\""));

    // 6. Convert
    let nt = store.serialize("ntriples").unwrap();
    assert!(nt.contains("Alice"));
}

#[test]
fn test_diff_workflow() {
    let old = std::fs::read_to_string("tests/data/sample.ttl").unwrap();
    let new = old.replace("Alice", "AliceV2");
    let diff = OntologyService::diff(&old, &new).unwrap();
    assert!(diff.contains("added"));
    assert!(diff.contains("removed"));
}
