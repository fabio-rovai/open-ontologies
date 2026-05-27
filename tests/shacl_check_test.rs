//! Integration tests for `ShaclValidator::check_shapes` (#18).
//!
//! Verifies the structural dry-run primitive: the function must catch missing
//! `sh:targetClass`, `sh:path`, `sh:class`, and unrecognised `sh:datatype` IRIs
//! against the loaded ontology, but accept well-formed shapes referencing
//! existing terms.

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

const ONTOLOGY: &str = r#"
    @prefix owl:  <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix ex:   <http://example.org/> .

    ex:Person   a owl:Class .
    ex:Address  a owl:Class .
    ex:hasName  a owl:DatatypeProperty ;
                rdfs:domain ex:Person ;
                rdfs:range  <http://www.w3.org/2001/XMLSchema#string> .
    ex:livesAt  a owl:ObjectProperty ;
                rdfs:domain ex:Person ;
                rdfs:range  ex:Address .
"#;

fn loaded() -> Arc<GraphStore> {
    let g = Arc::new(GraphStore::new());
    g.load_turtle(ONTOLOGY, None).expect("load ontology");
    g
}

#[test]
fn well_formed_shapes_pass() {
    let graph = loaded();

    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .

        ex:PersonShape a sh:NodeShape ;
            sh:targetClass ex:Person ;
            sh:property [
                sh:path ex:hasName ;
                sh:datatype <http://www.w3.org/2001/XMLSchema#string> ;
            ] ;
            sh:property [
                sh:path ex:livesAt ;
                sh:class ex:Address ;
            ] .
    "#;

    let report_str = ShaclValidator::check_shapes(&graph, shapes).expect("check_shapes");
    let report: serde_json::Value = serde_json::from_str(&report_str).expect("json");

    assert!(
        report["ok"].as_bool().unwrap(),
        "well-formed shapes should produce ok=true; got:\n{}",
        report_str
    );
    assert!(report["parses"].as_bool().unwrap());
    assert_eq!(report["issue_count"].as_u64().unwrap(), 0);
    assert_eq!(report["shape_count"].as_u64().unwrap(), 1);
}

#[test]
fn missing_target_class_is_flagged() {
    let graph = loaded();

    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .

        ex:GhostShape a sh:NodeShape ;
            sh:targetClass ex:DoesNotExist ;
            sh:property [
                sh:path ex:hasName ;
            ] .
    "#;

    let report_str = ShaclValidator::check_shapes(&graph, shapes).expect("check_shapes");
    let report: serde_json::Value = serde_json::from_str(&report_str).expect("json");

    assert!(!report["ok"].as_bool().unwrap());
    assert!(report["issue_count"].as_u64().unwrap() >= 1);

    let issues = report["issues"].as_array().unwrap();
    assert!(
        issues.iter().any(|i| {
            i["kind"].as_str() == Some("missing_target_class")
                && i["value"].as_str().unwrap_or("").contains("DoesNotExist")
        }),
        "expected a missing_target_class issue for ex:DoesNotExist; got: {:?}",
        issues
    );
}

#[test]
fn missing_path_property_is_flagged() {
    let graph = loaded();

    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .

        ex:PersonShape a sh:NodeShape ;
            sh:targetClass ex:Person ;
            sh:property [
                sh:path ex:undeclaredProperty ;
            ] .
    "#;

    let report_str = ShaclValidator::check_shapes(&graph, shapes).expect("check_shapes");
    let report: serde_json::Value = serde_json::from_str(&report_str).expect("json");

    assert!(!report["ok"].as_bool().unwrap());
    let issues = report["issues"].as_array().unwrap();
    assert!(
        issues.iter().any(|i| {
            i["kind"].as_str() == Some("missing_path")
                && i["value"].as_str().unwrap_or("").contains("undeclaredProperty")
        }),
        "expected a missing_path issue; got: {:?}",
        issues
    );
}

#[test]
fn missing_class_constraint_is_flagged() {
    let graph = loaded();

    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .

        ex:PersonShape a sh:NodeShape ;
            sh:targetClass ex:Person ;
            sh:property [
                sh:path ex:livesAt ;
                sh:class ex:NoSuchClass ;
            ] .
    "#;

    let report_str = ShaclValidator::check_shapes(&graph, shapes).expect("check_shapes");
    let report: serde_json::Value = serde_json::from_str(&report_str).expect("json");

    assert!(!report["ok"].as_bool().unwrap());
    let issues = report["issues"].as_array().unwrap();
    assert!(
        issues.iter().any(|i| {
            i["kind"].as_str() == Some("missing_class_constraint")
                && i["value"].as_str().unwrap_or("").contains("NoSuchClass")
        }),
        "expected a missing_class_constraint issue; got: {:?}",
        issues
    );
}

#[test]
fn unrecognised_datatype_is_flagged() {
    let graph = loaded();

    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .

        ex:PersonShape a sh:NodeShape ;
            sh:targetClass ex:Person ;
            sh:property [
                sh:path ex:hasName ;
                sh:datatype ex:CustomNotXsd ;
            ] .
    "#;

    let report_str = ShaclValidator::check_shapes(&graph, shapes).expect("check_shapes");
    let report: serde_json::Value = serde_json::from_str(&report_str).expect("json");

    let issues = report["issues"].as_array().unwrap();
    assert!(
        issues.iter().any(|i| i["kind"].as_str() == Some("unrecognised_datatype")),
        "expected an unrecognised_datatype issue; got: {:?}",
        issues
    );
}

#[test]
fn invalid_turtle_returns_parse_error_not_panic() {
    let graph = loaded();

    let bad_shapes = "this is not turtle: << >> /// invalid";
    let report_str = ShaclValidator::check_shapes(&graph, bad_shapes).expect("check_shapes");
    let report: serde_json::Value = serde_json::from_str(&report_str).expect("json");

    assert!(!report["parses"].as_bool().unwrap());
    assert!(!report["ok"].as_bool().unwrap());
    assert!(report["parse_error"].is_string());
}

#[test]
fn well_formed_shapes_carry_per_shape_diagnostic_detail() {
    // Beyond the top-level `ok` flag, callers (LLMs) need per-shape detail
    // to write targeted fixes. Verify that.
    let graph = loaded();

    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .

        ex:PersonShape a sh:NodeShape ;
            sh:targetClass ex:Person ;
            sh:property [
                sh:path ex:hasName ;
                sh:datatype <http://www.w3.org/2001/XMLSchema#string> ;
            ] .
    "#;

    let report_str = ShaclValidator::check_shapes(&graph, shapes).expect("check_shapes");
    let report: serde_json::Value = serde_json::from_str(&report_str).expect("json");

    let shapes_arr = report["shapes"].as_array().unwrap();
    assert_eq!(shapes_arr.len(), 1);
    let s = &shapes_arr[0];
    assert!(s["target_class"].as_str().unwrap().contains("Person"));
    assert!(s["target_class_exists"].as_bool().unwrap());
    let pc = s["property_constraints"].as_array().unwrap();
    assert_eq!(pc.len(), 1);
    assert!(pc[0]["path_exists"].as_bool().unwrap());
    assert!(pc[0]["datatype_recognised"].as_bool().unwrap());
}
