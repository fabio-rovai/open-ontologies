//! Two property shapes on one path, each carrying its own list-valued constraint.
//!
//! `sh:or`, `sh:in` and `sh:not` all nest a list or a shape under a property shape
//! that is almost always a blank node, and a blank-node label written into a SPARQL
//! query is a fresh variable rather than a reference. The original workaround was to
//! collect these per shape and key them by `sh:path`, which avoids naming the blank
//! node at all.
//!
//! It also merges every property shape that shares a path. Two `sh:not` blocks on
//! one path became one conjunction, `?val = bad1 && ?val = bad2`, which no value can
//! satisfy, so both rules reported nothing and the run came back clean. That is a
//! false clean, the one failure mode this validator must not have, and it is
//! reachable from three constraints at once.
//!
//! Keying by the property shape's own printed term fixes it, the same way the
//! node-shape complement already matches a shape by its printed term rather than
//! splicing it into query text.
//!
//! Every expectation here is pyshacl's answer for the same input.

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

fn report(store: &Arc<GraphStore>, shapes: &str) -> serde_json::Value {
    serde_json::from_str(&ShaclValidator::validate(store, shapes).unwrap()).unwrap()
}

fn store_with(ttl: &str) -> Arc<GraphStore> {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(ttl, None).unwrap();
    store
}

const TWO_BADS: &str = r#"
    @prefix ex: <http://example.org/> .
    ex:a a ex:Thing ; ex:p ex:bad1 .
    ex:b a ex:Thing ; ex:p ex:bad2 .
"#;

#[test]
fn two_not_constraints_on_one_path_are_both_evaluated() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:p ; sh:not [ sh:hasValue ex:bad1 ] ] ;
            sh:property [ sh:path ex:p ; sh:not [ sh:hasValue ex:bad2 ] ] .
    "#;
    let r = report(&store_with(TWO_BADS), shapes);
    assert_eq!(
        r["conforms"],
        serde_json::json!(false),
        "pyshacl reports two violations here; a clean run is the bug: {r}"
    );
    assert_eq!(r["violation_count"], serde_json::json!(2), "{r}");
    let nodes: Vec<String> = r["violations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["focus_node"].as_str().unwrap_or_default().to_string())
        .collect();
    assert!(nodes.contains(&"http://example.org/a".to_string()), "{r}");
    assert!(nodes.contains(&"http://example.org/b".to_string()), "{r}");
}

#[test]
fn two_or_constraints_on_one_path_are_both_evaluated() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:p ; sh:or ( [ sh:datatype xsd:string ] ) ] ;
            sh:property [ sh:path ex:p ; sh:or ( [ sh:datatype xsd:integer ] ) ] .
    "#;
    // The value is a string, so the second shape fails. Merged into one
    // disjunction it would pass on the first alternative and report nothing.
    let r = report(
        &store_with(r#"@prefix ex: <http://example.org/> . ex:a a ex:Thing ; ex:p "x" ."#),
        shapes,
    );
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], serde_json::json!(1), "{r}");
}

#[test]
fn two_in_constraints_on_one_path_are_both_evaluated() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:p ; sh:in ( "x" ) ] ;
            sh:property [ sh:path ex:p ; sh:in ( "y" ) ] .
    "#;
    let r = report(
        &store_with(r#"@prefix ex: <http://example.org/> . ex:a a ex:Thing ; ex:p "x" ."#),
        shapes,
    );
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], serde_json::json!(1), "{r}");
}

/// The ordinary single-shape case must be untouched by the change of key.
#[test]
fn one_property_shape_per_path_still_behaves() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:p ; sh:not [ sh:hasValue ex:bad1 ] ] .
    "#;
    let r = report(&store_with(TWO_BADS), shapes);
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], serde_json::json!(1), "{r}");
    assert_eq!(r["violations"][0]["focus_node"], "http://example.org/a");
}

/// An unsupported nested form in one of two blocks must suppress the verdict
/// without silencing the block beside it.
#[test]
fn an_unsupported_block_does_not_silence_its_neighbour() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:p ; sh:not [ sh:hasValue ex:bad1 ] ] ;
            sh:property [ sh:path ex:p ; sh:not [ sh:closed true ] ] .
    "#;
    let r = report(&store_with(TWO_BADS), shapes);
    assert!(r["conforms"].is_null(), "one block is unevaluated: {r}");
    assert_eq!(
        r["violation_count"],
        serde_json::json!(1),
        "the evaluable block must still report ex:a: {r}"
    );
    assert_eq!(r["violations"][0]["focus_node"], "http://example.org/a");
}
