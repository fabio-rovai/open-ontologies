// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Integration tests for the IES4 enforce rule pack (#24).
//!
//! The pack adds three checks beyond the existing `boro` pack:
//!   1. `ies4_particular_class_overlap` — class is subclass of both
//!      `ies:Particular` and `ies:ClassOfEntity` (type-vs-token clash).
//!   2. `ies4_state_without_subject` — `ies:State` subclass has no
//!      `ies:isStateOf` restriction or instance-level usage.
//!   3. `ies4_event_without_participant` — `ies:Event` subclass has no
//!      participant pattern.

use open_ontologies::enforce::Enforcer;
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

fn run_ies4(graph: Arc<GraphStore>) -> serde_json::Value {
    let (db, _) = setup();
    let enforcer = Enforcer::new(db, graph);
    let result = enforcer.enforce("ies4").expect("enforce");
    serde_json::from_str(&result).expect("json")
}

#[test]
fn ies4_compliant_minimal_ontology_passes_all_rules() {
    // An ontology that satisfies all three IES4 rules:
    //   - no Particular/ClassOfEntity overlap
    //   - PersonState has owl:Restriction onProperty ies:isStateOf
    //   - WorkEvent has owl:Restriction onProperty ies:isParticipantIn
    let (_db, graph) = setup();
    let ttl = r#"
        @prefix owl:  <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ies:  <http://ies.data.gov.uk/ontology/ies4#> .
        @prefix ex:   <http://example.org/> .

        ies:Particular     a owl:Class .
        ies:ClassOfEntity  a owl:Class .
        ies:Entity         a owl:Class .
        ies:State          a owl:Class .
        ies:Event          a owl:Class .
        ies:isStateOf      a owl:ObjectProperty .
        ies:isParticipantIn a owl:ObjectProperty .

        ex:Person a owl:Class ; rdfs:subClassOf ies:Particular .

        ex:PersonState a owl:Class ;
            rdfs:subClassOf ies:State ;
            rdfs:subClassOf [ a owl:Restriction ;
                              owl:onProperty ies:isStateOf ;
                              owl:someValuesFrom ex:Person ] .

        ex:WorkEvent a owl:Class ;
            rdfs:subClassOf ies:Event ;
            rdfs:subClassOf [ a owl:Restriction ;
                              owl:onProperty ies:isParticipantIn ;
                              owl:someValuesFrom ex:Person ] .
    "#;
    graph.load_turtle(ttl, None).expect("load");
    let report = run_ies4(graph);
    assert_eq!(report["total_rules"].as_u64().unwrap(), 3);
    assert_eq!(report["passed_rules"].as_u64().unwrap(), 3);
    assert!(
        report["violations"].as_array().unwrap().is_empty(),
        "compliant ontology should have zero violations; got: {}",
        report["violations"]
    );
}

#[test]
fn ies4_flags_particular_class_overlap() {
    let (_db, graph) = setup();
    let ttl = r#"
        @prefix owl:  <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ies:  <http://ies.data.gov.uk/ontology/ies4#> .
        @prefix ex:   <http://example.org/> .

        ies:Particular    a owl:Class .
        ies:ClassOfEntity a owl:Class .
        ies:State         a owl:Class .
        ies:Event         a owl:Class .

        # Violation: class is subclass of both Particular and ClassOfEntity.
        ex:ConfusedClass a owl:Class ;
            rdfs:subClassOf ies:Particular ;
            rdfs:subClassOf ies:ClassOfEntity .
    "#;
    graph.load_turtle(ttl, None).expect("load");
    let report = run_ies4(graph);
    let violations = report["violations"].as_array().unwrap();
    assert!(
        violations.iter().any(|v| {
            v["rule"].as_str() == Some("ies4_particular_class_overlap")
                && v["entity"].as_str().unwrap_or("").contains("ConfusedClass")
        }),
        "expected ies4_particular_class_overlap violation; got: {:?}",
        violations
    );
    assert_eq!(
        violations
            .iter()
            .find(|v| v["rule"] == "ies4_particular_class_overlap")
            .map(|v| v["severity"].as_str().unwrap()),
        Some("error"),
        "particular/class overlap is a 4D principle violation — severity must be error"
    );
}

#[test]
fn ies4_flags_state_without_subject() {
    let (_db, graph) = setup();
    let ttl = r#"
        @prefix owl:  <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ies:  <http://ies.data.gov.uk/ontology/ies4#> .
        @prefix ex:   <http://example.org/> .

        ies:State a owl:Class .
        ies:Event a owl:Class .
        ies:Particular    a owl:Class .
        ies:ClassOfEntity a owl:Class .

        # Violation: State subclass with no ies:isStateOf restriction
        # or instance-level usage.
        ex:OrphanState a owl:Class ;
            rdfs:subClassOf ies:State .
    "#;
    graph.load_turtle(ttl, None).expect("load");
    let report = run_ies4(graph);
    let violations = report["violations"].as_array().unwrap();
    assert!(
        violations.iter().any(|v| {
            v["rule"].as_str() == Some("ies4_state_without_subject")
                && v["entity"].as_str().unwrap_or("").contains("OrphanState")
        }),
        "expected ies4_state_without_subject violation; got: {:?}",
        violations
    );
}

#[test]
fn ies4_flags_event_without_participant() {
    let (_db, graph) = setup();
    let ttl = r#"
        @prefix owl:  <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ies:  <http://ies.data.gov.uk/ontology/ies4#> .
        @prefix ex:   <http://example.org/> .

        ies:Event a owl:Class .
        ies:State a owl:Class .
        ies:Particular    a owl:Class .
        ies:ClassOfEntity a owl:Class .

        # Violation: Event subclass with no participant pattern.
        ex:OrphanEvent a owl:Class ;
            rdfs:subClassOf ies:Event .
    "#;
    graph.load_turtle(ttl, None).expect("load");
    let report = run_ies4(graph);
    let violations = report["violations"].as_array().unwrap();
    assert!(
        violations.iter().any(|v| {
            v["rule"].as_str() == Some("ies4_event_without_participant")
                && v["entity"].as_str().unwrap_or("").contains("OrphanEvent")
        }),
        "expected ies4_event_without_participant violation; got: {:?}",
        violations
    );
}

#[test]
fn ies4_event_with_instance_level_participant_passes() {
    // An Event subclass that has no restriction BUT has at least one instance
    // participating via ies:isParticipantIn should pass — the rule accepts
    // either restriction-level or instance-level evidence of participants.
    let (_db, graph) = setup();
    let ttl = r#"
        @prefix owl:  <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ies:  <http://ies.data.gov.uk/ontology/ies4#> .
        @prefix ex:   <http://example.org/> .

        ies:Event a owl:Class .
        ies:State a owl:Class .
        ies:Particular    a owl:Class .
        ies:ClassOfEntity a owl:Class .
        ies:isParticipantIn a owl:ObjectProperty .

        ex:Meeting a owl:Class ; rdfs:subClassOf ies:Event .

        # Instance-level participation — should satisfy the rule.
        ex:meeting42 a ex:Meeting ;
            ies:isParticipantIn ex:alice .
    "#;
    graph.load_turtle(ttl, None).expect("load");
    let report = run_ies4(graph);
    let event_violations: Vec<_> = report["violations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["rule"] == "ies4_event_without_participant")
        .collect();
    assert!(
        event_violations.is_empty(),
        "Meeting has instance-level participation; should not be flagged. Got: {:?}",
        event_violations
    );
}
