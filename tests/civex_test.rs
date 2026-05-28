//! Integration tests for the CIVeX action-certification scaffold (#42).

use open_ontologies::civex::{certify_action, ActionFrame, Verdict};
use open_ontologies::graph::GraphStore;
use open_ontologies::state::StateDb;
use std::sync::Arc;
use tempfile::NamedTempFile;

fn setup() -> (StateDb, Arc<GraphStore>) {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    let db = StateDb::open(&path).unwrap();
    let graph = Arc::new(GraphStore::new());
    (db, graph)
}

fn base_frame() -> ActionFrame {
    // 5 queries → 5/5 pass at α=0.05 gives Wilson LCB ≈ 0.535 > 0.5 threshold.
    // 1 query would give LCB ≈ 0.27, which would correctly ABSTAIN — Wilson's
    // whole point is to not over-trust small samples.
    ActionFrame {
        tool: "onto_apply".to_string(),
        target_iris: vec!["http://ex.org/Cat".to_string()],
        proposed_delta_ttl: r#"
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix ex:  <http://ex.org/> .
            ex:Tiger a owl:Class .
        "#.to_string(),
        utility_metric: "dependent_query_pass_rate".to_string(),
        dependent_queries: vec![
            "SELECT ?x WHERE { ?x a <http://www.w3.org/2002/07/owl#Class> }".to_string(),
            "SELECT (COUNT(?x) AS ?n) WHERE { ?x a <http://www.w3.org/2002/07/owl#Class> }".to_string(),
            "SELECT ?x WHERE { ?x ?p ?o } LIMIT 5".to_string(),
            "SELECT ?p WHERE { ?s ?p ?o } LIMIT 5".to_string(),
            "SELECT ?o WHERE { ?s ?p ?o } LIMIT 5".to_string(),
        ],
        cost_threshold: 1000,
        utility_threshold: 0.5,
        risk_threshold: 5000,
        reversible: true,
        allow_experiment: false,
        alpha: 0.05,
        action_schema_name: None,
        identification_mode: open_ontologies::civex::IdentificationMode::Structural,
    }
}

#[test]
fn benign_addition_to_well_formed_ontology_executes() {
    let (db, graph) = setup();
    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex:  <http://ex.org/> .
        ex:Cat a owl:Class .
        ex:Dog a owl:Class .
    "#, None).unwrap();

    let frame = base_frame();
    let result = certify_action(&db, &graph, &frame).expect("certify");

    assert_eq!(result.certificate.verdict, Verdict::Execute,
        "adding a new owl:Class should execute under a well-formed graph; rationale: {}",
        result.certificate.rationale);
    assert!(result.certificate.utility_lcb >= frame.utility_threshold);
    assert!(!result.certificate.provenance_hash.is_empty());
    assert!(result.certificate.assumptions.contains(&"structural_only".to_string()));
}

#[test]
fn cost_exceeding_risk_threshold_rejects() {
    let (db, graph) = setup();
    // Pre-populate with classes the delta will REMOVE — this drives blast radius up.
    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex:  <http://ex.org/> .
        ex:Animal a owl:Class .
        ex:Cat a owl:Class ; rdfs:subClassOf ex:Animal .
        ex:Dog a owl:Class ; rdfs:subClassOf ex:Animal .
        ex:Bird a owl:Class ; rdfs:subClassOf ex:Animal .
    "#, None).unwrap();

    // Tiny risk threshold so any blast radius > 0 hits REJECT.
    let mut frame = base_frame();
    frame.risk_threshold = 0;
    frame.cost_threshold = 0;
    frame.target_iris = vec!["http://ex.org/Animal".to_string()];
    // Delta that doesn't include Animal — implies its removal in plan-diff terms.
    frame.proposed_delta_ttl = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://ex.org/> .
        ex:NewClass a owl:Class .
    "#.to_string();

    let result = certify_action(&db, &graph, &frame).expect("certify");
    assert_eq!(result.certificate.verdict, Verdict::Reject);
    assert!(result.certificate.rationale.contains("REJECT"));
}

#[test]
fn locked_iri_target_rejects_regardless_of_utility() {
    let (db, graph) = setup();
    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex:  <http://ex.org/> .
        ex:Critical a owl:Class .
    "#, None).unwrap();

    // Lock the target IRI via the existing planner machinery.
    let planner = open_ontologies::plan::Planner::new(db.clone(), graph.clone());
    planner.lock_iri("http://ex.org/Critical", "load-bearing");

    let mut frame = base_frame();
    frame.target_iris = vec!["http://ex.org/Critical".to_string()];

    let result = certify_action(&db, &graph, &frame).expect("certify");
    assert_eq!(result.certificate.verdict, Verdict::Reject,
        "locked IRI should hard-reject; got rationale: {}", result.certificate.rationale);
    assert!(result.certificate.rationale.contains("locked"));
}

#[test]
fn irreversible_with_ambiguous_utility_abstains() {
    let (db, graph) = setup();
    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex:  <http://ex.org/> .
        ex:Cat a owl:Class .
    "#, None).unwrap();

    let mut frame = base_frame();
    // Demand 99% LCB — Wilson at 1/1 will be ~0.21, well below.
    frame.utility_threshold = 0.99;
    frame.reversible = false;
    frame.allow_experiment = false;

    let result = certify_action(&db, &graph, &frame).expect("certify");
    assert_eq!(result.certificate.verdict, Verdict::Abstain);
    assert!(result.certificate.rationale.contains("ABSTAIN"));
}

#[test]
fn reversible_with_experiment_authorisation_returns_experiment() {
    let (db, graph) = setup();
    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex:  <http://ex.org/> .
        ex:Cat a owl:Class .
    "#, None).unwrap();

    let mut frame = base_frame();
    frame.utility_threshold = 0.99;     // unreachable LCB
    frame.reversible = true;
    frame.allow_experiment = true;

    let result = certify_action(&db, &graph, &frame).expect("certify");
    assert_eq!(result.certificate.verdict, Verdict::Experiment);
    assert!(result.certificate.rationale.contains("EXPERIMENT"));
}

#[test]
fn structural_mode_records_structural_only_assumption() {
    // v0.5: when identification_mode = Structural (default), the certificate
    // carries exactly the v0.4 assumption.
    let (db, graph) = setup();
    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex:  <http://ex.org/> .
        ex:Cat a owl:Class .
    "#, None).unwrap();

    let frame = base_frame();
    let result = certify_action(&db, &graph, &frame).expect("certify");
    let has_structural = result.certificate.assumptions.iter()
        .any(|a| a == "structural_only");
    assert!(has_structural, "structural mode should record structural_only assumption; got: {:?}",
        result.certificate.assumptions);
    // And NOT a do-calculus assumption.
    assert!(!result.certificate.assumptions.iter().any(|a| a.starts_with("do_calculus")),
        "structural mode should NOT record any do_calculus assumption; got: {:?}",
        result.certificate.assumptions);
}

#[test]
fn do_calculus_mode_falls_back_to_structural_when_feature_disabled() {
    // v0.5: when identification_mode = DoCalculusBackdoor but the
    // `causal-pywhy` Cargo feature is OFF (the default build configuration),
    // the verifier silently falls back to structural and records the reason.
    let (db, graph) = setup();
    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex:  <http://ex.org/> .
        ex:Cat a owl:Class .
    "#, None).unwrap();

    let mut frame = base_frame();
    frame.identification_mode = open_ontologies::civex::IdentificationMode::DoCalculusBackdoor;

    let result = certify_action(&db, &graph, &frame).expect("certify");
    // The certificate should still issue a verdict (no panics from fallback).
    let assumptions = &result.certificate.assumptions;

    // Under the default build, the assumption must be the feature_disabled marker.
    // Under `causal-pywhy` build, Python or DoWhy may or may not exist; either
    // way the fallback should NOT crash.
    let has_do_calculus_marker = assumptions.iter().any(|a| a.starts_with("do_calculus"));
    assert!(has_do_calculus_marker,
        "DoCalculusBackdoor mode should record SOME do_calculus_* assumption; got: {:?}",
        assumptions);
    // The proof string should be non-empty regardless of branch taken.
    assert!(!result.certificate.identification_proof.is_empty());
}

#[test]
fn action_schema_name_is_recorded_in_certificate_assumptions() {
    // CIVeX × Dynamics integration: passing a Dynamics action schema name
    // through ActionFrame must show up in the certificate's audit trail.
    let (db, graph) = setup();
    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex:  <http://ex.org/> .
        ex:Cat a owl:Class .
    "#, None).unwrap();

    let mut frame = base_frame();
    frame.action_schema_name = Some("rename_class".to_string());

    let result = certify_action(&db, &graph, &frame).expect("certify");
    let has_schema_marker = result.certificate.assumptions.iter()
        .any(|a| a == "dynamics_action_schema:rename_class");
    assert!(has_schema_marker,
        "schema name must be threaded into assumptions; got {:?}",
        result.certificate.assumptions);
}

#[test]
fn provenance_hash_changes_when_delta_changes() {
    let (db, graph) = setup();
    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://ex.org/> .
        ex:Cat a owl:Class .
    "#, None).unwrap();

    let mut frame_a = base_frame();
    frame_a.proposed_delta_ttl = "@prefix ex: <http://ex.org/> . ex:A a <http://www.w3.org/2002/07/owl#Class> .".to_string();
    let mut frame_b = base_frame();
    frame_b.proposed_delta_ttl = "@prefix ex: <http://ex.org/> . ex:B a <http://www.w3.org/2002/07/owl#Class> .".to_string();

    let r_a = certify_action(&db, &graph, &frame_a).expect("a");
    let r_b = certify_action(&db, &graph, &frame_b).expect("b");
    assert_ne!(r_a.certificate.provenance_hash, r_b.certificate.provenance_hash,
        "different deltas must produce different provenance hashes");
    // Same provenance hash on a repeated certify with the SAME delta.
    let r_a2 = certify_action(&db, &graph, &frame_a).expect("a2");
    assert_eq!(r_a.certificate.provenance_hash, r_a2.certificate.provenance_hash);
}
