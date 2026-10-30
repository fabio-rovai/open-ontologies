use std::sync::Arc;

use open_ontologies::extract_scaffold::{build_scaffold, validate_extraction};
use open_ontologies::graph::GraphStore;

fn scaffold(
    label: &str,
    comment: &str,
    property_label: &str,
) -> open_ontologies::extract_scaffold::ExtractionScaffold {
    let graph = Arc::new(GraphStore::new());
    let turtle = format!(
        r#"
@prefix ex: <https://example.org/> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
ex:Order rdfs:label {label}; rdfs:comment {comment} .
ex:quantity rdfs:domain ex:Order; rdfs:range xsd:integer; rdfs:label {property_label} .
"#
    );
    assert_eq!(graph.load_turtle(&turtle, None).unwrap(), 5);
    build_scaffold(&graph, "https://example.org/Order").unwrap()
}

#[test]
fn language_tagged_annotations_keep_lexical_field_names_and_typed_checks() {
    let plain = scaffold(r#""Order""#, r#""A customer order""#, r#""quantity""#);
    let tagged = scaffold(
        r#""Order"@en"#,
        r#""A customer order"@en"#,
        r#""quantity"@en"#,
    );
    assert_eq!(
        serde_json::to_value(&tagged).unwrap(),
        serde_json::to_value(&plain).unwrap()
    );
    assert_eq!(tagged.property_schema.len(), 1);
    let valid = validate_extraction(&tagged, r#"[{"quantity":"3"}]"#).unwrap();
    assert_eq!(valid.valid, 1);
    assert_eq!(valid.mean_conformance, 1.0);
    for key in ["quantity", "https://example.org/quantity"] {
        let extraction = serde_json::json!([{(key): "three"}]).to_string();
        let invalid = validate_extraction(&tagged, &extraction).unwrap();
        assert_eq!(invalid.valid, 0);
        assert!(
            invalid
                .issues
                .iter()
                .any(|issue| issue.kind == "type_mismatch")
        );
        assert!(
            !invalid
                .issues
                .iter()
                .any(|issue| issue.kind == "unknown_field")
        );
    }
}

#[test]
fn multilingual_labels_and_escaped_annotations_keep_their_text() {
    let actual = scaffold(
        r#""注文"@ja"#,
        r#""A \"customer\" order\n配送の説明"@en"#,
        r#""注文数量"@ja"#,
    );
    assert_eq!(actual.class_label.as_deref(), Some("注文"));
    assert_eq!(
        actual.class_comment.as_deref(),
        Some("A \"customer\" order\n配送の説明")
    );
    assert_eq!(
        actual.property_schema[0].property_label.as_deref(),
        Some("注文数量")
    );
    assert!(
        actual
            .prompt_template
            .contains("A \"customer\" order\n配送の説明")
    );
    let valid = validate_extraction(&actual, r#"[{"注文数量":3}]"#).unwrap();
    assert_eq!(valid.valid, 1);
    assert_eq!(valid.mean_conformance, 1.0);
    let invalid = validate_extraction(&actual, r#"[{"注文数量":"many"}]"#).unwrap();
    assert_eq!(invalid.valid, 0);
}

#[test]
fn escaped_property_labels_match_the_label_key_and_plain_controls_still_work() {
    let actual = scaffold(
        r#""Order""#,
        r#""A customer order""#,
        r#""quantity \"ordered\""@en"#,
    );
    assert_eq!(actual.class_label.as_deref(), Some("Order"));
    assert_eq!(actual.class_comment.as_deref(), Some("A customer order"));
    assert_eq!(
        actual.property_schema[0].property_label.as_deref(),
        Some("quantity \"ordered\"")
    );
    let valid = validate_extraction(&actual, r#"[{"quantity \"ordered\"":3}]"#).unwrap();
    assert_eq!(valid.valid, 1);
    assert_eq!(valid.mean_conformance, 1.0);
}
