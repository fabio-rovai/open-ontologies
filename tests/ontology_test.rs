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
