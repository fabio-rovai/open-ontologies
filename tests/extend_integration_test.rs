use open_ontologies::graph::GraphStore;
use open_ontologies::ingest::DataIngester;
use open_ontologies::mapping::{MappingConfig, FieldMapping};
use open_ontologies::shacl::ShaclValidator;
use open_ontologies::reason::Reasoner;
use std::sync::Arc;

#[test]
fn test_full_pipeline_csv_to_validated_rdf() {
    // 1. Load an ontology
    let store = Arc::new(GraphStore::new());
    let ontology = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .

        ex:Building a owl:Class ;
            rdfs:label "Building" .
        ex:Landmark a owl:Class ;
            rdfs:subClassOf ex:Building ;
            rdfs:label "Landmark" .
        ex:hasName a owl:DatatypeProperty ;
            rdfs:domain ex:Building ;
            rdfs:label "has name" .
    "#;
    store.load_turtle(ontology, None).unwrap();
    let onto_triples = store.triple_count();
    assert!(onto_triples > 0);

    // 2. Parse CSV data
    let csv = "id,name,type\nb1,Tower Bridge,Landmark\nb2,Big Ben,Landmark\n";
    let rows = DataIngester::parse_csv(csv).unwrap();
    assert_eq!(rows.len(), 2);

    // 3. Create explicit mapping
    let mapping = MappingConfig {
        base_iri: "http://example.org/".to_string(),
        id_field: "id".to_string(),
        class: "http://example.org/Landmark".to_string(),
        mappings: vec![
            FieldMapping {
                field: "id".to_string(),
                predicate: "http://example.org/id".to_string(),
                datatype: Some("http://www.w3.org/2001/XMLSchema#string".to_string()),
                class: None,
                lookup: false,
            },
            FieldMapping {
                field: "name".to_string(),
                predicate: "http://www.w3.org/2000/01/rdf-schema#label".to_string(),
                datatype: Some("http://www.w3.org/2001/XMLSchema#string".to_string()),
                class: None,
                lookup: false,
            },
        ],
    };

    // 4. Ingest — convert rows to N-Triples and load
    let ntriples = mapping.rows_to_ntriples(&rows);
    let loaded = store.load_ntriples(&ntriples).unwrap();
    assert!(loaded > 0);

    // 5. Validate with SHACL — Landmarks must have labels
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

        ex:LandmarkShape a sh:NodeShape ;
            sh:targetClass ex:Landmark ;
            sh:property [
                sh:path rdfs:label ;
                sh:minCount 1 ;
                sh:message "Landmark must have a label" ;
            ] .
    "#;
    let report = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(parsed["conforms"], true, "SHACL should pass — both landmarks have labels");

    // 6. Run RDFS reasoning — Landmarks should be inferred as Buildings
    let reason_report = Reasoner::run(&store, "rdfs", true).unwrap();
    let reason_parsed: serde_json::Value = serde_json::from_str(&reason_report).unwrap();
    let inferred = reason_parsed["inferred_count"].as_u64().unwrap();
    assert!(inferred >= 2, "Should infer at least 2 triples (b1 and b2 are Buildings)");

    // 7. Verify inference worked via SPARQL
    let check = store.sparql_select(
        "ASK { <http://example.org/b1> a <http://example.org/Building> }"
    ).unwrap();
    assert!(check.contains("true"), "b1 should be inferred as a Building via subClassOf");

    let check2 = store.sparql_select(
        "ASK { <http://example.org/b2> a <http://example.org/Building> }"
    ).unwrap();
    assert!(check2.contains("true"), "b2 should be inferred as a Building via subClassOf");
}
