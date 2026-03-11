use open_ontologies::graph::GraphStore;
use std::sync::Arc;

#[test]
fn test_crosswalk_lookup() {
    use open_ontologies::clinical::ClinicalCrosswalks;
    let cw = ClinicalCrosswalks::load("data/crosswalks.parquet");
    if cw.is_err() {
        eprintln!("Skipping: crosswalks.parquet not found. Run scripts/build_crosswalks.py first.");
        return;
    }
    let cw = cw.unwrap();

    let results = cw.lookup("I10", "ICD10");
    assert!(!results.is_empty());
}

#[test]
fn test_crosswalk_search_by_label() {
    use open_ontologies::clinical::ClinicalCrosswalks;
    let cw = ClinicalCrosswalks::load("data/crosswalks.parquet");
    if cw.is_err() { return; }
    let cw = cw.unwrap();

    let results = cw.search_label("hypertension");
    assert!(!results.is_empty());
}

#[test]
fn test_validate_clinical_terms() {
    use open_ontologies::clinical::ClinicalCrosswalks;
    let cw = ClinicalCrosswalks::load("data/crosswalks.parquet");
    if cw.is_err() { return; }
    let cw = cw.unwrap();

    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Hypertension a owl:Class ; rdfs:label "Hypertension" .
        ex:FakeDisease a owl:Class ; rdfs:label "Xylophagous Syndrome" .
    "#, None).unwrap();

    let result = cw.validate_clinical(&graph);
    assert!(result.contains("validated") || result.contains("unmatched"));
}

#[test]
fn test_enrich_adds_skos_mapping() {
    use open_ontologies::clinical::ClinicalCrosswalks;
    let cw = ClinicalCrosswalks::load("data/crosswalks.parquet");
    if cw.is_err() { return; }
    let cw = cw.unwrap();

    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Hypertension a owl:Class .
    "#, None).unwrap();

    let result = cw.enrich(&graph, "http://example.org/Hypertension", "I10", "ICD10");
    assert!(result.contains("ok") || result.contains("enriched"));
}

#[test]
fn test_clinical_label_matching() {
    use open_ontologies::drift::jaro_winkler;
    let score = jaro_winkler("Hypertension", "HyperTensionSyndrome");
    assert!(score > 0.5);
    let score = jaro_winkler("Hypertension", "Essential hypertension");
    assert!(score > 0.4);
}
