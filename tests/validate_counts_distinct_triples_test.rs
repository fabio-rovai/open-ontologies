//! `validate` must not report parse events as triples.
//!
//! An RDF graph is a set, so a statement repeated in the source contributes one triple
//! and not two. Counting parser events and labelling the result `triples` overstates any
//! generated document that repeats a statement, which real serialisers do constantly:
//! emitting a shared node's rdf:type once per record inflated one 16.7 MB file by 6.7
//! per cent against rdflib.

use open_ontologies::graph::GraphStore;

const DUPLICATED: &str = r#"
    @prefix ex: <http://example.org/> .
    ex:a ex:p ex:b .
    ex:a ex:p ex:b .
    ex:a ex:p ex:c .
"#;

#[test]
fn statements_and_triples_are_reported_separately() {
    let counts = GraphStore::validate_turtle(DUPLICATED).expect("validate");
    assert_eq!(counts.statements, 3, "three statements were parsed");
    assert_eq!(counts.triples, 2, "the graph holds two distinct triples");
}

#[test]
fn a_document_with_no_duplicates_reports_equal_counts() {
    let counts = GraphStore::validate_turtle(
        "@prefix ex: <http://example.org/> .\nex:a ex:p ex:b .\nex:a ex:p ex:c .\n",
    )
    .expect("validate");
    assert_eq!(counts.statements, counts.triples);
    assert_eq!(counts.triples, 2);
}

#[test]
fn load_reports_triples_added_not_statements_parsed() {
    let graph = GraphStore::new();
    let added = graph.load_turtle(DUPLICATED, None).expect("load");
    assert_eq!(added, 2, "the store deduplicates, so only two triples were added");
    assert_eq!(graph.triple_count(), 2);

    // Loading the same content again adds nothing at all.
    let again = graph.load_turtle(DUPLICATED, None).expect("reload");
    assert_eq!(again, 0, "re-loading identical content adds no triples");
    assert_eq!(graph.triple_count(), 2);
}
