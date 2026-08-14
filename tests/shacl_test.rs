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
