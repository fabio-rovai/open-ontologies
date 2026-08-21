use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

fn make_store_with_data() -> Arc<GraphStore> {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        ex:b1 a ex:Building ; rdfs:label "Tower Bridge" ; ex:height "65"^^xsd:integer .
        ex:b2 a ex:Building ; ex:height "96"^^xsd:integer .
    "#;
    store.load_turtle(ttl, None).unwrap();
    store
}

#[test]
fn test_shacl_mincount_violation() {
    let store = make_store_with_data();
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:BuildingShape a sh:NodeShape ;
            sh:targetClass ex:Building ;
            sh:property [
                sh:path rdfs:label ;
                sh:minCount 1 ;
                sh:message "Building must have a label" ;
            ] .
    "#;
    let result = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["conforms"], false);
    assert!(parsed["violation_count"].as_u64().unwrap() >= 1);
    // b2 has no label
    let violations = parsed["violations"].as_array().unwrap();
    assert!(violations.iter().any(|v| {
        v["focus_node"].as_str().unwrap().contains("b2")
    }));
}

#[test]
fn test_shacl_all_pass() {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:b1 a ex:Building ; rdfs:label "Tower Bridge" .
        ex:b2 a ex:Building ; rdfs:label "Big Ben" .
    "#;
    store.load_turtle(ttl, None).unwrap();
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:BuildingShape a sh:NodeShape ;
            sh:targetClass ex:Building ;
            sh:property [
                sh:path rdfs:label ;
                sh:minCount 1 ;
            ] .
    "#;
    let result = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["conforms"], true);
    assert_eq!(parsed["violation_count"], 0);
}

#[test]
fn test_shacl_maxcount_violation() {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:b1 a ex:Building ; rdfs:label "Tower Bridge" ; rdfs:label "Le pont de la Tour" .
    "#;
    store.load_turtle(ttl, None).unwrap();
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:BuildingShape a sh:NodeShape ;
            sh:targetClass ex:Building ;
            sh:property [
                sh:path rdfs:label ;
                sh:maxCount 1 ;
            ] .
    "#;
    let result = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["conforms"], false);
    assert!(parsed["violation_count"].as_u64().unwrap() >= 1);
}

#[test]
fn test_shacl_datatype_violation() {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        ex:b1 a ex:Building ; ex:height "sixty-five" .
    "#;
    store.load_turtle(ttl, None).unwrap();
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        ex:BuildingShape a sh:NodeShape ;
            sh:targetClass ex:Building ;
            sh:property [
                sh:path ex:height ;
                sh:datatype xsd:integer ;
            ] .
    "#;
    let result = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["conforms"], false);
    assert!(parsed["violation_count"].as_u64().unwrap() >= 1);
}

#[test]
fn test_shacl_inverse_path_mincount() {
    // A registrant must be pointed at by at least one fund via ^fundOf.
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:reg1 a ex:Registrant .
        ex:reg2 a ex:Registrant .
        ex:fund1 a ex:Fund ; ex:fundOf ex:reg1 .
    "#;
    store.load_turtle(ttl, None).unwrap();
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:RegistrantShape a sh:NodeShape ;
            sh:targetClass ex:Registrant ;
            sh:property [
                sh:path [ sh:inversePath ex:fundOf ] ;
                sh:minCount 1 ;
                sh:message "Registrant with no fund series attached" ;
            ] .
    "#;
    let result = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["conforms"], false);
    let violations = parsed["violations"].as_array().unwrap();
    // reg2 has no inbound fundOf; reg1 does.
    assert!(violations.iter().any(|v| v["focus_node"].as_str().unwrap().contains("reg2")));
    assert!(!violations.iter().any(|v| v["focus_node"].as_str().unwrap().contains("reg1")));
}

#[test]
fn test_shacl_pattern_violation() {
    // LEI-style syntax rule: 18 alphanumerics + 2 decimal check digits.
    // id1 conforms; id2 is 19 characters (a truncation) and must fire.
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:id1 a ex:LEI ; ex:value "969500AAAAAAAAAAAA75" .
        ex:id2 a ex:LEI ; ex:value "969500BBBBBBBBBBB12" .
    "#;
    store.load_turtle(ttl, None).unwrap();
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:LEIShape a sh:NodeShape ;
            sh:targetClass ex:LEI ;
            sh:property [
                sh:path ex:value ;
                sh:pattern "^[A-Z0-9]{18}[0-9]{2}$" ;
                sh:message "LEI must be 20 characters (ISO 17442)" ;
            ] .
    "#;
    let result = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["conforms"], false);
    let violations = parsed["violations"].as_array().unwrap();
    assert!(violations.iter().any(|v| v["focus_node"].as_str().unwrap().contains("id2")));
    assert!(!violations.iter().any(|v| v["focus_node"].as_str().unwrap().contains("id1")));
    assert!(violations.iter().any(|v| v["constraint"] == "pattern"));
}

#[test]
fn test_shacl_has_value_violation() {
    // Checksum policy: every LEI node must record ex:checksumValid true.
    // id1 records true; id2 records false; id3 records nothing.
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:id1 a ex:LEI ; ex:checksumValid true .
        ex:id2 a ex:LEI ; ex:checksumValid false .
        ex:id3 a ex:LEI .
    "#;
    store.load_turtle(ttl, None).unwrap();
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:LEIChecksumShape a sh:NodeShape ;
            sh:targetClass ex:LEI ;
            sh:property [
                sh:path ex:checksumValid ;
                sh:hasValue true ;
                sh:message "LEI check digits fail or are unrecorded" ;
            ] .
    "#;
    let result = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["conforms"], false);
    let violations = parsed["violations"].as_array().unwrap();
    assert!(violations.iter().any(|v| v["focus_node"].as_str().unwrap().contains("id2")));
    assert!(violations.iter().any(|v| v["focus_node"].as_str().unwrap().contains("id3")));
    assert!(!violations.iter().any(|v| v["focus_node"].as_str().unwrap().contains("id1")));
    assert!(violations.iter().any(|v| v["constraint"] == "hasValue"));
}

#[test]
fn test_shacl_unsupported_path_skipped_not_fatal() {
    // A sequence path is not executable; it must be reported as skipped
    // while the rest of the shapes still run.
    let store = make_store_with_data();
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:BuildingShape a sh:NodeShape ;
            sh:targetClass ex:Building ;
            sh:property [
                sh:path ( ex:inZone ex:zoneName ) ;
                sh:minCount 1 ;
            ] ;
            sh:property [
                sh:path rdfs:label ;
                sh:minCount 1 ;
            ] .
    "#;
    let result = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    // The label constraint still fires on b2.
    assert!(parsed["violations"].as_array().unwrap().iter().any(|v| {
        v["focus_node"].as_str().unwrap().contains("b2")
    }));
    assert!(parsed["skipped_constraints"].as_array().unwrap().len() == 1);
}

// Regression tests for four false-clean defects found while validating the
// Scotland land register ontology. In every case the validator previously
// returned `conforms: true` over data that violated the shape, or over a shapes
// graph it could not evaluate at all. A false clean is worse than an error,
// because the caller has no signal that anything was missed.

fn store_with(ttl: &str) -> Arc<GraphStore> {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(ttl, None).unwrap();
    store
}

const NOT_DATA: &str = r#"
    @prefix ex: <http://example.org/> .
    ex:a a ex:Thing ; ex:mech ex:bad .
"#;

#[test]
fn test_sh_not_is_not_silently_ignored() {
    // `sh:not` is not implemented. It must be reported as skipped, which
    // suppresses the verdict, rather than passing silently.
    let store = store_with(NOT_DATA);
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:mech ; sh:not [ sh:hasValue ex:bad ] ] .
    "#;
    let report = ShaclValidator::validate(&store, shapes).unwrap();
    let v: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert!(v["conforms"].is_null(), "must not claim a pass: {v}");
    assert!(
        v["skipped_constraints"].as_array().map_or(false, |a| !a.is_empty()),
        "sh:not must be recorded as skipped: {v}"
    );
}

#[test]
fn test_target_node_shape_does_not_report_a_pass() {
    // `sh:targetNode` selects no focus nodes here, so the shapes graph yields
    // nothing evaluable. Previously this reported conforms: true.
    let store = store_with("@prefix ex: <http://example.org/> . ex:b a ex:Thing .");
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetNode ex:b ;
            sh:property [ sh:path ex:p ; sh:minCount 1 ] .
    "#;
    let report = ShaclValidator::validate(&store, shapes).unwrap();
    let v: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert!(v["conforms"].is_null(), "must not claim a pass: {v}");
}

#[test]
fn test_implicit_class_target_is_evaluated() {
    // A node that is both sh:NodeShape and rdfs:Class targets its instances.
    // This is now implemented, so it must find the real violation.
    let store = store_with("@prefix ex: <http://example.org/> . ex:b a ex:Thing .");
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Thing a sh:NodeShape, rdfs:Class ;
            sh:property [ sh:path ex:p ; sh:minCount 1 ] .
    "#;
    let report = ShaclValidator::validate(&store, shapes).unwrap();
    let v: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(v["conforms"], serde_json::Value::Bool(false), "report: {v}");
    assert_eq!(v["violation_count"], 1, "report: {v}");
}

#[test]
fn test_target_class_still_works() {
    // Guard against the fixes above regressing the one target form that always
    // worked correctly.
    let store = store_with("@prefix ex: <http://example.org/> . ex:b a ex:Thing .");
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:p ; sh:minCount 1 ] .
    "#;
    let report = ShaclValidator::validate(&store, shapes).unwrap();
    let v: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(v["conforms"], serde_json::Value::Bool(false), "report: {v}");
    assert_eq!(v["violation_count"], 1, "report: {v}");
}
