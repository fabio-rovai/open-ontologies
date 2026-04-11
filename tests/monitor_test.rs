use open_ontologies::graph::GraphStore;
use open_ontologies::monitor::{Monitor, Watcher, WatcherAction};
use open_ontologies::ontology::OntologyService;
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
        webhook_url: None,
        webhook_headers: None,
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
        webhook_url: None,
        webhook_headers: None,
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
        webhook_url: None,
        webhook_headers: None,
    });

    let result = monitor.run_watchers();
    assert_eq!(result.status, "ok");
    assert!(result.alerts.is_empty());
    assert_eq!(result.passed.len(), 1);
}

#[test]
fn test_monitor_auto_rollback_restores_version() {
    let tmp = NamedTempFile::new().unwrap();
    let db = StateDb::open(tmp.path()).unwrap();
    let graph = Arc::new(GraphStore::new());

    // Load a clean ontology and save a version
    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class ; rdfs:label "Dog" .
    "#, None).unwrap();
    OntologyService::save_version(&db, &graph, "clean").unwrap();
    let clean_count = graph.triple_count();

    // Now corrupt it — add unlabelled classes
    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Broken1 a owl:Class .
        ex:Broken2 a owl:Class .
        ex:Broken3 a owl:Class .
    "#, None).unwrap();
    assert!(graph.triple_count() > clean_count);

    // Set up auto_rollback watcher: triggers if >0 unlabelled classes
    let monitor = Monitor::new(db.clone(), graph.clone());
    monitor.add_watcher(Watcher {
        id: "unlabelled_rollback".into(),
        check_type: "sparql".into(),
        threshold: 0.0,
        severity: "error".into(),
        action: WatcherAction::AutoRollback,
        query: Some("SELECT (COUNT(?c) AS ?count) WHERE { ?c a <http://www.w3.org/2002/07/owl#Class> . FILTER NOT EXISTS { ?c <http://www.w3.org/2000/01/rdf-schema#label> ?l } }".into()),
        message: Some("Unlabelled classes detected".into()),
        webhook_url: None,
        webhook_headers: None,
    });

    let result = monitor.run_watchers();
    assert_eq!(result.status, "auto_rolled_back");
    assert_eq!(result.alerts.len(), 1);
    // Graph should be restored to the clean version
    assert_eq!(graph.triple_count(), clean_count);
}

#[test]
fn test_monitor_auto_rollback_no_version_still_alerts() {
    let tmp = NamedTempFile::new().unwrap();
    let db = StateDb::open(tmp.path()).unwrap();
    let graph = Arc::new(GraphStore::new());

    // Load data without saving any version first
    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#, None).unwrap();

    let monitor = Monitor::new(db.clone(), graph.clone());
    monitor.add_watcher(Watcher {
        id: "rollback_no_version".into(),
        check_type: "sparql".into(),
        threshold: 0.0,
        severity: "error".into(),
        action: WatcherAction::AutoRollback,
        query: Some("SELECT (COUNT(?c) AS ?count) WHERE { ?c a <http://www.w3.org/2002/07/owl#Class> }".into()),
        message: Some("Should not crash when no versions exist".into()),
        webhook_url: None,
        webhook_headers: None,
    });

    // Should still produce an alert even though rollback can't happen (no saved versions)
    let result = monitor.run_watchers();
    assert_eq!(result.alerts.len(), 1);
    assert_eq!(result.alerts[0].watcher, "rollback_no_version");
}
