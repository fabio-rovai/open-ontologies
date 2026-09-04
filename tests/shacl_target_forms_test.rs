//! The three explicit target forms: `sh:targetNode`, `sh:targetSubjectsOf`,
//! `sh:targetObjectsOf`.
//!
//! Until now the validator selected focus nodes one way only, by class, and
//! recorded the other three forms as skipped. Skipping was the honest answer
//! while they were unimplemented — a shape whose only target is `sh:targetNode`
//! selected nothing, and reporting `conforms: true` over nothing is the failure
//! mode this validator must not have. But honest and absent is still absent:
//! a shapes graph written against the specification gets no verdict at all.
//!
//! These pin the three forms against the answers pyshacl gives for the same
//! data and shapes.

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

fn report(store: &Arc<GraphStore>, shapes: &str) -> serde_json::Value {
    serde_json::from_str(&ShaclValidator::validate(store, shapes).unwrap()).unwrap()
}

fn store() -> Arc<GraphStore> {
    let store = Arc::new(GraphStore::new());
    store
        .load_turtle(
            r#"
            @prefix ex: <http://example.org/> .
            ex:a a ex:Thing ; ex:mech ex:bad ; ex:p "present" .
            ex:b a ex:Thing .
            ex:c ex:knows ex:d .
        "#,
            None,
        )
        .unwrap();
    store
}

const TARGET_NODE: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    ex:S a sh:NodeShape ; sh:targetNode ex:b ;
        sh:property [ sh:path ex:p ; sh:minCount 1 ; sh:message "needs p" ] .
"#;

const TARGET_NODE_SATISFIED: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    ex:S a sh:NodeShape ; sh:targetNode ex:a ;
        sh:property [ sh:path ex:p ; sh:minCount 1 ; sh:message "needs p" ] .
"#;

const TARGET_SUBJECTS_OF: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    ex:S a sh:NodeShape ; sh:targetSubjectsOf ex:knows ;
        sh:property [ sh:path ex:p ; sh:minCount 1 ; sh:message "needs p" ] .
"#;

const TARGET_OBJECTS_OF: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    ex:S a sh:NodeShape ; sh:targetObjectsOf ex:knows ;
        sh:property [ sh:path ex:p ; sh:minCount 1 ; sh:message "needs p" ] .
"#;

#[test]
fn target_node_selects_the_named_node_and_reports_its_violation() {
    let r = report(&store(), TARGET_NODE);
    assert_eq!(r["focus_nodes"], serde_json::json!(1));
    assert_eq!(
        r["conforms"],
        serde_json::json!(false),
        "ex:b has no ex:p; pyshacl reports one violation here"
    );
    assert_eq!(r["violation_count"], serde_json::json!(1));
    assert_eq!(r["violations"][0]["focus_node"], "http://example.org/b");
}

#[test]
fn target_node_that_satisfies_the_constraint_conforms() {
    let r = report(&store(), TARGET_NODE_SATISFIED);
    assert_eq!(r["focus_nodes"], serde_json::json!(1));
    assert_eq!(r["conforms"], serde_json::json!(true));
    assert_eq!(r["violation_count"], serde_json::json!(0));
}

#[test]
fn target_subjects_of_selects_the_subject() {
    let r = report(&store(), TARGET_SUBJECTS_OF);
    assert_eq!(r["focus_nodes"], serde_json::json!(1));
    assert_eq!(r["conforms"], serde_json::json!(false));
    assert_eq!(r["violations"][0]["focus_node"], "http://example.org/c");
}

#[test]
fn target_objects_of_selects_the_object() {
    let r = report(&store(), TARGET_OBJECTS_OF);
    assert_eq!(r["focus_nodes"], serde_json::json!(1));
    assert_eq!(r["conforms"], serde_json::json!(false));
    assert_eq!(r["violations"][0]["focus_node"], "http://example.org/d");
}

/// An implemented target form must stop appearing in `skipped_constraints`.
/// A stale skip is as misleading as a missing one: it suppresses the verdict
/// on a run in which nothing was in fact missed.
#[test]
fn implemented_target_forms_are_no_longer_recorded_as_skipped() {
    for shapes in [TARGET_NODE, TARGET_SUBJECTS_OF, TARGET_OBJECTS_OF] {
        let r = report(&store(), shapes);
        let skipped = r["skipped_constraints"].as_array().cloned().unwrap_or_default();
        let stale: Vec<_> = skipped
            .iter()
            .filter(|s| {
                let c = s["constraint"].as_str().unwrap_or_default();
                c.contains("targetNode") || c.contains("targetSubjectsOf") || c.contains("targetObjectsOf")
            })
            .collect();
        assert!(stale.is_empty(), "target form still recorded as skipped: {stale:?}");
    }
}

/// `sh:targetNode` is an explicit target, not a query: it selects the named node
/// whether or not that node appears in the data. A node absent from the data is
/// therefore still a focus node, and a `sh:minCount 1` over it fails.
///
/// Checked against pyshacl, which reports `conforms=False` with one MinCount
/// violation on `ex:nosuchnode` for exactly this input. Treating an absent node
/// as an empty target instead would be a false clean of our own making.
#[test]
fn target_node_absent_from_the_data_is_still_a_focus_node() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetNode ex:nosuchnode ;
            sh:property [ sh:path ex:p ; sh:minCount 1 ] .
    "#;
    let r = report(&store(), shapes);
    assert_eq!(r["focus_nodes"], serde_json::json!(1));
    assert_eq!(r["conforms"], serde_json::json!(false));
    assert_eq!(r["violation_count"], serde_json::json!(1));
    assert_eq!(
        r["violations"][0]["focus_node"],
        "http://example.org/nosuchnode"
    );
}
