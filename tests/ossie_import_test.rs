//! Apache Ossie ontology import, checked against the reference document.
//!
//! `tests/fixtures/ossie_enterprise_ontology.yaml` is an Ossie ontology document
//! exercising every construct in the 0.2.0.dev0 spec. The expected counts here
//! are cross-checked against an independent Python implementation of the same
//! mapping, so a divergence in either direction shows up as a test failure
//! rather than as a quietly different graph.

use open_ontologies::graph::GraphStore;
use open_ontologies::ossie::{parse_document, to_owl_shacl};
use std::collections::BTreeMap;

const BASE: &str = "https://example.org/onto#";

fn fixture() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ossie_enterprise_ontology.yaml"
    ))
    .expect("fixture is present")
}

fn convert() -> open_ontologies::ossie::OssieConversion {
    let document = parse_document(&fixture()).expect("fixture parses");
    to_owl_shacl(&document, Some(BASE), true).expect("fixture converts")
}

#[test]
fn structure_counts_match_the_source_document() {
    let result = convert();
    assert_eq!(result.concepts, 14);
    assert_eq!(result.entity_types, 6);
    assert_eq!(result.value_types, 8);
    assert_eq!(result.relationships, 15);
}

#[test]
fn every_unenforceable_construct_is_reported() {
    let result = convert();
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for issue in &result.issues {
        *counts.entry(issue.kind).or_default() += 1;
    }

    assert_eq!(counts.get("DERIVATION_NOT_EXPRESSIBLE"), Some(&4));
    assert_eq!(counts.get("INVERSE_FUNCTIONAL_DATA_PROPERTY"), Some(&3));
    assert_eq!(counts.get("REQUIRES_NOT_EXPRESSIBLE"), Some(&3));
    assert_eq!(counts.get("NARY_RELATIONSHIP_REIFIED"), Some(&2));
    assert_eq!(counts.get("NARY_MULTIPLICITY_SHACL_ONLY"), Some(&2));
    assert_eq!(counts.get("UNARY_RELATIONSHIP_AS_CLASS"), Some(&2));
    assert_eq!(result.issues.len(), 16);
}

#[test]
fn no_identifier_in_the_document_survives_into_owl() {
    // Every OneToOne in the fixture is onto a ValueType, which is
    // InverseFunctionalDataProperty and therefore outside OWL 2 DL. This is the
    // structural point: fact-based models identify entity types by value types,
    // so an Ossie-to-OWL path that does not also emit SHACL loses every
    // identifier in the model.
    let result = convert();
    let one_to_one = result
        .issues
        .iter()
        .filter(|i| i.kind == "INVERSE_FUNCTIONAL_DATA_PROPERTY")
        .count();
    assert_eq!(one_to_one, 3);
    assert!(!result.turtle.contains("owl:InverseFunctionalProperty"));
}

#[test]
fn shacl_carries_what_owl_cannot() {
    let result = convert();
    // 3 OneToOne uniqueness constraints + 2 n-ary tuple dependencies.
    assert_eq!(result.sparql_constraints, 5);
    assert!(result.turtle.contains("?otherLast != ?last"));
    assert!(result.turtle.contains("FILTER (?other != $this)"));
}

#[test]
fn the_compiled_graph_loads_into_the_store() {
    let result = convert();
    let store = GraphStore::new();
    let triples = store
        .load_turtle(&result.turtle, None)
        .expect("compiled Ossie ontology must load");
    assert!(triples > 300, "expected a full graph, got {triples}");
}

#[test]
fn compiled_ontology_is_queryable() {
    let result = convert();
    let store = GraphStore::new();
    store.load_turtle(&result.turtle, None).expect("loads");

    // The whole point of the compile: an Ossie ontology becomes answerable by
    // SPARQL, which it is not in its native form.
    let answer = store
        .sparql_select(
            "PREFIX owl: <http://www.w3.org/2002/07/owl#> \
             SELECT (COUNT(DISTINCT ?c) AS ?n) WHERE { ?c a owl:Class }",
        )
        .expect("query runs");
    // 6 concepts + 2 unary classes + 2 link classes.
    assert!(answer.contains("10"), "unexpected class count: {answer}");
}

#[test]
fn shacl_is_optional_but_lossy() {
    let document = parse_document(&fixture()).expect("parses");
    let owl_only = to_owl_shacl(&document, Some(BASE), false).expect("converts");
    assert_eq!(owl_only.sparql_constraints, 0);
    assert!(!owl_only.turtle.contains("sh:NodeShape"));
    // The issues are still reported: the constraints are lost, not hidden.
    assert_eq!(owl_only.issues.len(), 16);
}
