use open_ontologies::graph::GraphStore;
use open_ontologies::monitor::{Monitor, Watcher, WatcherAction};
use open_ontologies::state::StateDb;
use std::sync::Arc;
use tempfile::NamedTempFile;

#[test]
fn test_monitor_no_watchers_passes() {
    let tmp = NamedTempFile::new().unwrap();
    let db = StateDb::open(tmp.path()).unwrap();
    let graph = Arc::new(GraphStore::new());
    let monitor = Monitor::new(db, graph);

    let result = monitor.run_watchers();
    assert_eq!(result.status, "ok");
    assert!(result.alerts.is_empty());
}

#[test]
fn test_monitor_sparql_watcher_triggers() {
    let tmp = NamedTempFile::new().unwrap();
    let db = StateDb::open(tmp.path()).unwrap();
    let graph = Arc::new(GraphStore::new());

    // Load some data without labels
    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#, None).unwrap();

    let monitor = Monitor::new(db.clone(), graph);

    // Add a watcher that checks for classes without labels
    monitor.add_watcher(Watcher {
        id: "no_labels".into(),
        check_type: "sparql".into(),
        threshold: 0.0,
        severity: "error".into(),
        action: WatcherAction::Notify,
        query: Some("SELECT (COUNT(?c) AS ?count) WHERE { ?c a <http://www.w3.org/2002/07/owl#Class> . FILTER NOT EXISTS { ?c <http://www.w3.org/2000/01/rdf-schema#label> ?l } }".into()),
        message: Some("Classes without labels".into()),
    });

    let result = monitor.run_watchers();
    assert_eq!(result.status, "alert");
    assert_eq!(result.alerts.len(), 1);
    assert_eq!(result.alerts[0].watcher, "no_labels");
}

#[test]
fn test_monitor_block_flag() {
    let tmp = NamedTempFile::new().unwrap();
    let db = StateDb::open(tmp.path()).unwrap();
    let graph = Arc::new(GraphStore::new());

    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#, None).unwrap();

    let monitor = Monitor::new(db.clone(), graph);

    monitor.add_watcher(Watcher {
        id: "no_labels".into(),
        check_type: "sparql".into(),
        threshold: 0.0,
        severity: "error".into(),
        action: WatcherAction::BlockNextApply,
        query: Some("SELECT (COUNT(?c) AS ?count) WHERE { ?c a <http://www.w3.org/2002/07/owl#Class> . FILTER NOT EXISTS { ?c <http://www.w3.org/2000/01/rdf-schema#label> ?l } }".into()),
        message: Some("Classes without labels".into()),
    });

    let result = monitor.run_watchers();
    assert_eq!(result.status, "blocked");
    assert!(monitor.is_blocked());
}

#[test]
fn test_monitor_watcher_below_threshold_passes() {
    let tmp = NamedTempFile::new().unwrap();
    let db = StateDb::open(tmp.path()).unwrap();
    let graph = Arc::new(GraphStore::new());

    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#, None).unwrap();

    let monitor = Monitor::new(db.clone(), graph);

    monitor.add_watcher(Watcher {
        id: "class_count".into(),
        check_type: "sparql".into(),
        threshold: 10.0,  // threshold is 10, only 1 class loaded
        severity: "warning".into(),
        action: WatcherAction::Notify,
        query: Some("SELECT (COUNT(?c) AS ?count) WHERE { ?c a <http://www.w3.org/2002/07/owl#Class> }".into()),
        message: Some("Too many classes".into()),
    });

    let result = monitor.run_watchers();
    assert_eq!(result.status, "ok");
    assert!(result.alerts.is_empty());
    assert_eq!(result.passed.len(), 1);
}
