//! A node shape's own constraints are EVALUATED whatever target form selected
//! the shape, and the ones this validator cannot evaluate are recorded.
//!
//! History, because the test is shaped by it. The node-shape complement once
//! built its set of known shapes from the `sh:targetClass` discovery query
//! alone, so a shape whose only target was `sh:targetNode`,
//! `sh:targetSubjectsOf` or `sh:targetObjectsOf` had an unimplemented
//! constraint dropped with no record and the verdict came back `true`. That was
//! closed by widening the complement. Then every node-level value constraint
//! was still only RECORDED, never run: `sh:class` directly on a node shape
//! produced `conforms: null` over data that plainly violated it, and 31 of the
//! 32 `core/node` cases in the W3C suite were UNDETERMINED. This file now pins
//! both halves: a value constraint on the node shape reports a violation for
//! every target form, and a construct that is not evaluated is still recorded
//! for every target form.

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

const DATA: &str = r#"
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix ex:  <http://example.org/> .

    ex:Person a owl:Class .
    ex:Address a owl:Class .

    ex:alice a ex:Person ; ex:addr ex:notAnAddress .
    ex:notAnAddress a ex:Person .
    ex:home a ex:Address .
"#;

/// The four target forms, each selecting `ex:alice` (or, for objectsOf, the
/// object `ex:notAnAddress`), each asserting the node-level constraint given.
fn shapes(target: &str, constraint: &str) -> String {
    format!(
        r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:Shape a sh:NodeShape ;
            {target} ;
            {constraint} .
        "#
    )
}

const TARGETS: [(&str, &str); 4] = [
    ("sh:targetClass", "sh:targetClass ex:Person"),
    ("sh:targetNode", "sh:targetNode ex:alice"),
    ("sh:targetSubjectsOf", "sh:targetSubjectsOf ex:addr"),
    ("sh:targetObjectsOf", "sh:targetObjectsOf ex:addr"),
];

fn report(shapes: &str) -> serde_json::Value {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(DATA, None).unwrap();
    let out = ShaclValidator::validate(&store, shapes).unwrap();
    serde_json::from_str(&out).unwrap()
}

/// `sh:class ex:Address` on the node shape: none of the selected nodes is an
/// Address, so every target form must report a ClassConstraintComponent
/// violation and a false verdict. Not null: the constraint ran.
#[test]
fn a_node_level_class_constraint_is_evaluated_for_every_target_form() {
    for (label, target) in TARGETS {
        let r = report(&shapes(target, "sh:class ex:Address"));
        assert_eq!(
            r["conforms"],
            false,
            "{label}: node-level sh:class was not evaluated (verdict {}): {r}",
            r["conforms"]
        );
        let hits: Vec<&serde_json::Value> = r["violations"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| {
                v["source_constraint_component"]
                    == "http://www.w3.org/ns/shacl#ClassConstraintComponent"
                    && v["source_shape"] == "http://example.org/Shape"
            })
            .collect();
        assert!(!hits.is_empty(), "{label}: no class violation attributed to the node shape: {r}");
        assert!(
            hits.iter().all(|v| v.get("result_path").is_none()),
            "{label}: a node-shape result carries a result_path; node shapes have none: {r}"
        );
        assert!(
            r["skipped_constraints"]
                .as_array()
                .map(|s| s.iter().all(|e| e["constraint"] != "sh:class"))
                .unwrap_or(true),
            "{label}: sh:class was evaluated AND recorded as skipped: {r}"
        );
    }
}

/// The gate can pass: the same constraint over a node that satisfies it is a
/// true verdict, not null and not false.
#[test]
fn a_satisfied_node_level_constraint_conforms() {
    let r = report(&shapes("sh:targetNode ex:home", "sh:class ex:Address"));
    assert_eq!(r["conforms"], true, "{r}");
    assert_eq!(r["violation_count"], 0, "{r}");
}

/// The other node-level value constraints, each over a node that violates it.
#[test]
fn the_other_node_level_value_constraints_are_evaluated() {
    for (constraint, component) in [
        ("sh:nodeKind sh:Literal", "NodeKindConstraintComponent"),
        ("sh:hasValue ex:home", "HasValueConstraintComponent"),
        ("sh:in ( ex:home ex:bob )", "InConstraintComponent"),
        ("sh:pattern \"^http://example.org/b\"", "PatternConstraintComponent"),
        ("sh:minLength 100", "MinLengthConstraintComponent"),
        ("sh:maxLength 3", "MaxLengthConstraintComponent"),
        // An IRI cannot be compared with a number; SHACL 4.5 makes that a violation.
        ("sh:minInclusive 1", "MinInclusiveConstraintComponent"),
    ] {
        let r = report(&shapes("sh:targetNode ex:alice", constraint));
        assert_eq!(r["conforms"], false, "{constraint}: {r}");
        assert!(
            r["violations"].as_array().unwrap().iter().any(|v| {
                v["source_constraint_component"] == format!("http://www.w3.org/ns/shacl#{component}")
            }),
            "{constraint}: no {component} result: {r}"
        );
    }
}

/// A node-level construct this validator does not evaluate must still reach
/// `skipped_constraints` and suppress the verdict, for every target form. The
/// data would pass it, so a fix that "works" by dropping the record shows up
/// as a true verdict here.
#[test]
fn an_unevaluated_node_level_construct_is_recorded_for_every_target_form() {
    for (label, target) in TARGETS {
        let r = report(&shapes(target, "sh:languageIn ( \"en\" )"));
        let skipped = r["skipped_constraints"].as_array().cloned().unwrap_or_default();
        assert!(
            skipped.iter().any(|e| e["constraint"] == "http://www.w3.org/ns/shacl#languageIn"),
            "{label}: node-level sh:languageIn was dropped with no skipped_constraints entry: {r}"
        );
        assert!(
            r["conforms"].is_null(),
            "{label}: a run with an unevaluated constraint reported {} instead of null",
            r["conforms"]
        );
    }
}
