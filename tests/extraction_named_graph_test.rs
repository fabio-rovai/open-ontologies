use std::sync::Arc;

use open_ontologies::extract_scaffold::{ExtractionScaffold, build_scaffold, validate_extraction};
use open_ontologies::graph::GraphStore;
use oxigraph::io::RdfFormat;

const CLASS: &str = "https://example.org/Order";
const ONTOLOGY: &str = r#"
@prefix ex: <https://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
ex:Order a owl:Class; rdfs:label "Order"; rdfs:comment "A customer order" .
ex:quantity a owl:DatatypeProperty; rdfs:domain ex:Order; rdfs:range xsd:integer; rdfs:label "quantity" .
"#;

fn graph(content: &str, format: RdfFormat) -> Arc<GraphStore> {
    let graph = Arc::new(GraphStore::new());
    assert_eq!(graph.load_content(content, format).unwrap(), 7);
    graph
}

fn assert_typed_schema(scaffold: &ExtractionScaffold) {
    assert_eq!(scaffold.class_label.as_deref(), Some("Order"));
    assert_eq!(scaffold.class_comment.as_deref(), Some("A customer order"));
    assert_eq!(scaffold.property_schema.len(), 1);
    assert_eq!(
        scaffold.property_schema[0].property_iri,
        "https://example.org/quantity"
    );
    assert_eq!(
        scaffold.property_schema[0].property_label.as_deref(),
        Some("quantity")
    );
    assert_eq!(
        scaffold.property_schema[0].range,
        "http://www.w3.org/2001/XMLSchema#integer"
    );
    let valid = validate_extraction(scaffold, r#"[{"quantity":"3"}]"#).unwrap();
    assert_eq!(valid.valid, 1);
    assert_eq!(valid.mean_conformance, 1.0);
    let invalid = validate_extraction(scaffold, r#"[{"quantity":"three"}]"#).unwrap();
    assert_eq!(invalid.valid, 0);
    assert!(
        invalid
            .issues
            .iter()
            .any(|issue| issue.kind == "type_mismatch")
    );
}

#[test]
fn turtle_trig_and_nquads_keep_the_same_nonempty_extraction_schema() {
    let turtle = graph(ONTOLOGY, RdfFormat::Turtle);
    let expected = build_scaffold(&turtle, CLASS).unwrap();
    assert_typed_schema(&expected);
    let (prefixes, body) = ONTOLOGY.split_once("ex:Order").unwrap();
    let trig = format!("{prefixes}<https://example.org/schema> {{ ex:Order{body} }}");
    let named = graph(&trig, RdfFormat::TriG);
    let nquads = named.serialize("nquads").unwrap();
    let copied = graph(&nquads, RdfFormat::NQuads);
    for dataset in [named, copied] {
        let actual = build_scaffold(&dataset, CLASS).unwrap();
        assert_typed_schema(&actual);
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(&expected).unwrap()
        );
        let user_query: serde_json::Value = serde_json::from_str(
            &dataset.sparql_select("SELECT ?p WHERE { ?p <http://www.w3.org/2000/01/rdf-schema#domain> <https://example.org/Order> }").unwrap(),
        ).unwrap();
        assert!(
            user_query["results"].as_array().unwrap().is_empty(),
            "user SELECT keeps its chosen dataset"
        );
    }
}

#[test]
fn class_metadata_and_property_declarations_can_be_in_different_graphs() {
    let trig = r#"
@prefix ex: <https://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
ex:metadata { ex:Order a owl:Class; rdfs:label "Order"; rdfs:comment "A customer order" . }
ex:domain { ex:quantity a owl:DatatypeProperty; rdfs:domain ex:Order . }
ex:range { ex:quantity rdfs:range xsd:integer; rdfs:label "quantity" . }
"#;
    let dataset = graph(trig, RdfFormat::TriG);
    assert_typed_schema(&build_scaffold(&dataset, CLASS).unwrap());
}
