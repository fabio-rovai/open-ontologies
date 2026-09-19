//! **Issue #170.** Every `skipped_constraints` entry names the shape it came from.
//!
//! The field is recorded per constraint-evaluation SITE, not per shape, so one
//! shape can appear in `violations` and in `skipped_constraints` from the same
//! run. Read without knowing that, a report looks self-contradictory: the
//! validator says it skipped a constraint and then emits results for the same
//! shape. That reading cost a consumer real time.
//!
//! The granularity is now stated in the `onto_shacl` description. The sentence
//! makes a promise about the SHAPE of each entry, and this is the promise:
//! an entry without a shape leaves a reader unable to line the two lists up,
//! which is exactly the confusion the sentence exists to remove.
//!
//! A reader distinguishing "this constraint did not run" from "it ran and
//! selected nothing" needs both lists keyed the same way. Getting that
//! backwards is the unsafe direction: a naive reading marks a shape as
//! unchecked when it partly ran.

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

/// `MINUS` is forbidden in a pre-bound query by SHACL 5.2.1, so the validator
/// must refuse the constraint rather than substitute into it. That refusal is
/// a skip, which is what gives this test something to inspect.
const SHAPES: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    ex:TestShape a sh:NodeShape ;
        sh:targetNode ex:thing ;
        sh:sparql [
            sh:select """
                SELECT $this
                WHERE { $this ?x ?any . MINUS { $this ?x "v" } }"""
        ] .
"#;

const DATA: &str = r#"
    @prefix ex: <http://example.org/> .
    ex:thing ex:p "v" .
"#;

#[test]
fn every_skipped_constraint_names_the_shape_it_was_written_on() {
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(DATA, None).expect("data");
    let json = ShaclValidator::validate(&graph, SHAPES).expect("validate");
    let r: serde_json::Value = serde_json::from_str(&json).expect("report is JSON");

    let skipped = r["skipped_constraints"]
        .as_array()
        .expect("skipped_constraints is an array");
    assert!(
        !skipped.is_empty(),
        "nothing was skipped, so this test inspected no entry and would pass over a report \
         that dropped the shape from every one of them: {r}"
    );

    for entry in skipped {
        let shape = entry["shape"].as_str();
        assert!(
            shape.is_some_and(|s| !s.is_empty()),
            "a skipped_constraints entry carries no shape, so a reader cannot line it up \
             against violations and cannot tell a constraint that did not run from one that \
             ran and selected nothing: {entry}"
        );
        assert!(
            entry["reason"].as_str().is_some_and(|s| !s.is_empty()),
            "a skip with no reason is indistinguishable from a silent drop: {entry}"
        );
    }
}

/// And the verdict is withheld rather than reported as conforming. A skipped
/// constraint that left `conforms: true` behind would be the false clean this
/// whole field exists to prevent.
#[test]
fn a_skipped_constraint_withholds_the_verdict() {
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(DATA, None).expect("data");
    let json = ShaclValidator::validate(&graph, SHAPES).expect("validate");
    let r: serde_json::Value = serde_json::from_str(&json).expect("report is JSON");
    assert!(
        r["conforms"].is_null(),
        "a run that skipped a constraint must not claim conformance: {r}"
    );
}
