//! JSON-LD is a first-class RDF serialisation and the underlying `oxrdfio`
//! already supports it, but the engine's format handling did not: `.jsonld`
//! fell through extension detection to Turtle, so a valid JSON-LD document was
//! handed to the Turtle parser and failed with a misleading syntax error.

use open_ontologies::graph::GraphStore;

const JSONLD_DOC: &str = r#"{
  "@context": { "ex": "http://example.org/" },
  "@id": "ex:Alice",
  "@type": "ex:Person",
  "ex:name": "Alice"
}"#;

#[test]
fn validate_file_reads_a_jsonld_document() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.jsonld");
    std::fs::write(&path, JSONLD_DOC).unwrap();

    let counts = GraphStore::validate_file(path.to_str().unwrap())
        .expect("a .jsonld file must validate as JSON-LD, not as Turtle");
    assert_eq!(counts.triples, 2);
    assert_eq!(counts.statements, 2, "this document repeats no statement");
}

#[test]
fn load_file_reads_a_jsonld_document() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.jsonld");
    std::fs::write(&path, JSONLD_DOC).unwrap();

    let store = GraphStore::new();
    let loaded = store
        .load_file(path.to_str().unwrap())
        .expect("a .jsonld file must load as JSON-LD, not as Turtle");
    assert_eq!(loaded, 2);
    assert_eq!(store.triple_count(), 2);

    let results = store
        .sparql_select("SELECT ?s WHERE { ?s a <http://example.org/Person> }")
        .unwrap();
    assert!(results.contains("Alice"));
}

#[test]
fn serialize_emits_jsonld() {
    let store = GraphStore::new();
    store
        .load_turtle(
            r#"@prefix ex: <http://example.org/> .
               ex:Alice a ex:Person ."#,
            None,
        )
        .unwrap();

    let out = store
        .serialize("jsonld")
        .expect("jsonld must be an accepted serialisation format");
    let head = out.trim_start();
    assert!(head.starts_with('[') || head.starts_with('{'));
    assert!(out.contains("http://example.org/Alice"));
}

#[test]
fn jsonld_round_trips_through_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("round.jsonld");

    let source = GraphStore::new();
    source
        .load_turtle(
            r#"@prefix ex: <http://example.org/> .
               ex:Alice a ex:Person ;
                        ex:name "Alice" ."#,
            None,
        )
        .unwrap();
    source.save_file(path.to_str().unwrap(), "jsonld").unwrap();

    let reloaded = GraphStore::new();
    reloaded.load_file(path.to_str().unwrap()).unwrap();
    assert_eq!(reloaded.triple_count(), source.triple_count());
}

#[test]
fn json_ld_spelling_is_accepted_as_a_format_name() {
    let store = GraphStore::new();
    store
        .load_turtle(
            r#"@prefix ex: <http://example.org/> .
               ex:Alice a ex:Person ."#,
            None,
        )
        .unwrap();

    // "json-ld" is the spelling used by the W3C media type registration and by
    // most other tooling, so accepting only "jsonld" turns a correct format
    // name into an error.
    assert!(store.serialize("json-ld").is_ok());
}

#[test]
fn a_jsonld_body_in_a_misnamed_file_is_still_read_as_jsonld() {
    // Extension detection is a hint, not proof: `.owl` and `.rdf` are routinely
    // used for whatever serialisation the publisher happened to emit. The
    // engine already sniffs the body to rescue Turtle-in-.owl; JSON-LD needs
    // the same treatment or a misnamed file fails with a parser error.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.owl");
    std::fs::write(&path, JSONLD_DOC).unwrap();

    let counts = GraphStore::validate_file(path.to_str().unwrap())
        .expect("a JSON-LD body must be detected from its content");
    assert_eq!(counts.triples, 2);
}
