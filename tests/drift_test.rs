use open_ontologies::drift::DriftDetector;
use open_ontologies::state::StateDb;
use tempfile::NamedTempFile;

fn setup() -> StateDb {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    StateDb::open(&path).unwrap()
}

#[test]
fn test_drift_no_changes() {
    let db = setup();

    let v1 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
        ex:Cat a owl:Class .
    "#;

    let detector = DriftDetector::new(db);
    let result = detector.detect(v1, v1).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert!(parsed["likely_renames"].as_array().unwrap().is_empty());
    assert!(parsed["added"].as_array().unwrap().is_empty());
    assert!(parsed["removed"].as_array().unwrap().is_empty());
    assert!(parsed["drift_velocity"].as_f64().unwrap() < 0.01);
}

#[test]
fn test_drift_detects_addition_and_removal() {
    let db = setup();

    let v1 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
        ex:Cat a owl:Class .
    "#;

    let v2 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
        ex:Bird a owl:Class .
    "#;

    let detector = DriftDetector::new(db);
    let result = detector.detect(v1, v2).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert!(parsed["removed"].as_array().unwrap().iter().any(|v| v.as_str().unwrap().contains("Cat")));
    assert!(parsed["added"].as_array().unwrap().iter().any(|v| v.as_str().unwrap().contains("Bird")));
}

#[test]
fn test_drift_detects_likely_rename_by_domain_range() {
    let db = setup();

    let v1 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:authoredBy a owl:ObjectProperty ;
            rdfs:domain ex:Paper ;
            rdfs:range ex:Person .
    "#;

    let v2 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:writtenBy a owl:ObjectProperty ;
            rdfs:domain ex:Paper ;
            rdfs:range ex:Person .
    "#;

    let detector = DriftDetector::new(db);
    let result = detector.detect(v1, v2).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let renames = parsed["likely_renames"].as_array().unwrap();
    assert!(!renames.is_empty());
    assert!(renames[0]["from"].as_str().unwrap().contains("authoredBy"));
    assert!(renames[0]["to"].as_str().unwrap().contains("writtenBy"));
    assert!(renames[0]["confidence"].as_f64().unwrap() > 0.5);
}

#[test]
fn test_drift_label_similarity() {
    let db = setup();

    let v1 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:DomesticCat a owl:Class ; rdfs:label "Domestic Cat" .
    "#;

    let v2 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:HouseCat a owl:Class ; rdfs:label "House Cat" .
    "#;

    let detector = DriftDetector::new(db);
    let result = detector.detect(v1, v2).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let renames = parsed["likely_renames"].as_array().unwrap();
    assert!(!renames.is_empty());
    assert!(renames[0]["signals"]["label_similarity"].as_f64().unwrap() > 0.3);
}

#[test]
fn test_drift_velocity() {
    let db = setup();

    let v1 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:A a owl:Class .
        ex:B a owl:Class .
        ex:C a owl:Class .
        ex:D a owl:Class .
    "#;

    // Replace 3 of 4 classes = high drift
    let v2 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:A a owl:Class .
        ex:X a owl:Class .
        ex:Y a owl:Class .
        ex:Z a owl:Class .
    "#;

    let detector = DriftDetector::new(db);
    let result = detector.detect(v1, v2).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert!(parsed["drift_velocity"].as_f64().unwrap() > 0.5);
}

#[test]
fn test_drift_feedback_improves_confidence() {
    let db = setup();
    let detector = DriftDetector::new(db);

    detector.record_feedback("ex:a", "ex:b", "rename", 0.8, "rename",
        true, 0.9, false, true);
    detector.record_feedback("ex:c", "ex:d", "rename", 0.6, "different_concept",
        false, 0.3, false, false);

    let weights = detector.get_learned_weights();
    assert_eq!(weights.len(), 4);
}

#[test]
fn test_jaro_winkler_similarity() {
    use open_ontologies::drift::jaro_winkler;
    assert!(jaro_winkler("authoredBy", "authoredBy") > 0.99);
    assert!(jaro_winkler("authoredBy", "writtenBy") > 0.4);
    assert!(jaro_winkler("Dog", "Cat") < 0.6);
    assert!(jaro_winkler("DomesticCat", "HouseCat") > 0.5);
}
