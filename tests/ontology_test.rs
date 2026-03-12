use open_ontologies::ontology::OntologyService;

#[test]
fn test_validate_valid_file() {
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:Alice a ex:Person .
    "#;
    let result = OntologyService::validate_string(ttl);
    assert!(result.is_ok());
    let report = result.unwrap();
    assert!(report.contains("\"valid\":true"));
}

#[test]
fn test_validate_invalid_file() {
    let result = OntologyService::validate_string("@@@ not turtle");
    assert!(result.is_ok()); // returns report, not error
    let report = result.unwrap();
    assert!(report.contains("\"valid\":false"));
}

#[test]
fn test_convert_format() {
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:Alice a ex:Person .
    "#;
    let result = OntologyService::convert(ttl, "turtle", "ntriples");
    assert!(result.is_ok());
    let nt = result.unwrap();
    assert!(nt.contains("<http://example.org/Alice>"));
}

#[test]
fn test_diff_ontologies() {
    let old = r#"
        @prefix ex: <http://example.org/> .
        ex:Alice a ex:Person .
        ex:Bob a ex:Person .
    "#;
    let new = r#"
        @prefix ex: <http://example.org/> .
        ex:Alice a ex:Person .
        ex:Charlie a ex:Person .
    "#;
    let result = OntologyService::diff(old, new);
    assert!(result.is_ok());
    let diff = result.unwrap();
    assert!(diff.contains("Bob")); // removed
    assert!(diff.contains("Charlie")); // added
}

#[test]
fn test_lint_ontology() {
    let ttl = r#"
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Person a owl:Class .
        ex:Animal a owl:Class ;
            rdfs:label "Animal" ;
            rdfs:comment "An animal" .
    "#;
    let result = OntologyService::lint(ttl);
    assert!(result.is_ok());
    let report = result.unwrap();
    // Person has no label/comment, should be flagged
    assert!(report.contains("Person"));
}

#[test]
fn test_lint_with_feedback_suppression() {
    use open_ontologies::ontology::OntologyService;
    use open_ontologies::state::StateDb;
    use open_ontologies::feedback::record_tool_feedback;

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    let db = StateDb::open(&path).unwrap();

    let ttl = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class .
    "#;

    // Without feedback, missing_label should appear
    let result = OntologyService::lint_with_feedback(ttl, Some(&db)).unwrap();
    let v: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(v["issue_count"].as_u64().unwrap() > 0);
    assert_eq!(v["suppressed_count"].as_u64().unwrap(), 0);

    // Find the entity string used in the lint output for Dog
    let issues = v["issues"].as_array().unwrap();
    let dog_issue = issues.iter().find(|i| {
        i["type"].as_str().unwrap_or("") == "missing_label" &&
        i["entity"].as_str().unwrap_or("").contains("example.org/Dog")
    });
    assert!(dog_issue.is_some(), "Should have missing_label for Dog");
    let entity_str = dog_issue.unwrap()["entity"].as_str().unwrap();

    // Dismiss 3 times using the exact entity string from lint output
    for _ in 0..3 {
        record_tool_feedback(&db, "lint", "missing_label", entity_str, false).unwrap();
    }

    // Now the issue should be suppressed
    let result = OntologyService::lint_with_feedback(ttl, Some(&db)).unwrap();
    let v: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(v["suppressed_count"].as_u64().unwrap() > 0);
}
