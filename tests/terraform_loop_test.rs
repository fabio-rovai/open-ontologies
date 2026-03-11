use open_ontologies::graph::GraphStore;
use open_ontologies::plan::Planner;
use open_ontologies::drift::DriftDetector;
use open_ontologies::monitor::Monitor;
use open_ontologies::enforce::Enforcer;
use open_ontologies::lineage::LineageLog;
use open_ontologies::state::StateDb;
use std::sync::Arc;
use tempfile::NamedTempFile;

/// Full Terraform-style loop: plan → enforce → apply → monitor → drift
#[test]
fn test_full_terraform_loop() {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    let db = StateDb::open(&path).unwrap();
    let graph = Arc::new(GraphStore::new());

    // --- Initial state ---
    let v1 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Animal a owl:Class ; rdfs:label "Animal" .
        ex:Dog a owl:Class ; rdfs:subClassOf ex:Animal ; rdfs:label "Dog" .
    "#;
    graph.load_turtle(v1, None).unwrap();

    // --- Lineage ---
    let lineage = LineageLog::new(db.clone());
    let session = lineage.new_session();
    lineage.record(&session, "L", "load", "v1");

    // --- Plan changes ---
    let v2 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Animal a owl:Class ; rdfs:label "Animal" .
        ex:Dog a owl:Class ; rdfs:subClassOf ex:Animal ; rdfs:label "Dog" .
        ex:Cat a owl:Class ; rdfs:subClassOf ex:Animal ; rdfs:label "Cat" .
    "#;

    let planner = Planner::new(db.clone(), graph.clone());
    let plan = planner.plan(v2).unwrap();
    let plan_parsed: serde_json::Value = serde_json::from_str(&plan).unwrap();
    assert_eq!(plan_parsed["risk_score"].as_str().unwrap(), "low");
    lineage.record(&session, "P", "plan", "low_risk");

    // --- Enforce ---
    let enforcer = Enforcer::new(db.clone(), graph.clone());
    let _enforce_result = enforcer.enforce("generic").unwrap();
    lineage.record(&session, "E", "enforce", "generic");

    // --- Apply ---
    let apply_result = planner.apply("safe").unwrap();
    let apply_parsed: serde_json::Value = serde_json::from_str(&apply_result).unwrap();
    assert_eq!(apply_parsed["ok"].as_bool().unwrap(), true);
    lineage.record(&session, "A", "apply", "safe");

    // --- Monitor ---
    let monitor = Monitor::new(db.clone(), graph.clone());
    let mon_result = monitor.run_watchers();
    assert_eq!(mon_result.status, "ok");
    lineage.record(&session, "M", "monitor", "ok");

    // --- Drift check (v1 vs v2) ---
    let detector = DriftDetector::new(db.clone());
    let drift = detector.detect(v1, v2).unwrap();
    let drift_parsed: serde_json::Value = serde_json::from_str(&drift).unwrap();
    assert!(drift_parsed["drift_velocity"].as_f64().unwrap() < 0.5); // Low drift — just an addition
    lineage.record(&session, "D", "drift", "low");

    // --- Verify lineage ---
    let events = lineage.get_compact(&session);
    let lines: Vec<&str> = events.trim().lines().collect();
    assert_eq!(lines.len(), 6); // L, P, E, A, M, D
}
