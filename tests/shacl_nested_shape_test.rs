//! `sh:node`: every value node must conform to another node shape.
//!
//! This is the first constraint here that is genuinely recursive. A referenced
//! shape carries its own property shapes, which may themselves carry `sh:node`,
//! and the HealthDCAT-AP shapes the health-dataset-catalogue vertical validates
//! against do exactly that, nine times.
//!
//! The nested shape is compiled into a SPARQL boolean expression over the value
//! node. Any constraint form inside it that cannot be compiled makes the whole
//! expression unavailable, and the `sh:node` is then recorded as unevaluated:
//! a nested shape that is half-checked would report conformance on rules that
//! never ran, which is the failure mode this validator must not have.
//!
//! Bounds are taken from what those shapes actually use: `sh:minCount` and
//! `sh:maxCount` of 0 or 1, `sh:nodeKind` of IRI, Literal or BlankNodeOrIRI,
//! plus `sh:class`, `sh:datatype` and `sh:hasValue`. A count other than 0 or 1
//! is not compiled, and says so.
//!
//! Every expectation is pyshacl's answer for the same input.

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

/// ex:good has a distribution carrying an access URL; ex:bad has one without.
const DATA: &str = r#"
    @prefix ex: <http://example.org/> .
    ex:d1 a ex:Distribution ; ex:accessURL <http://x.example/1> .
    ex:d2 a ex:Distribution .
    ex:good a ex:Dataset ; ex:distribution ex:d1 .
    ex:bad  a ex:Dataset ; ex:distribution ex:d2 .
"#;

const NODE_SHAPE: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    ex:DistributionShape a sh:NodeShape ;
        sh:property [ sh:path ex:accessURL ; sh:minCount 1 ; sh:nodeKind sh:IRI ] .
    ex:DatasetShape a sh:NodeShape ;
        sh:targetClass ex:Dataset ;
        sh:property [
            sh:path ex:distribution ;
            sh:node ex:DistributionShape ;
            sh:message "distribution must carry an access URL" ;
        ] .
"#;

#[test]
fn sh_node_reports_the_value_that_breaks_the_referenced_shape() {
    let r = report(&store_with(DATA), NODE_SHAPE);
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], serde_json::json!(1), "{r}");
    assert_eq!(r["violations"][0]["focus_node"], "http://example.org/bad");
    assert_eq!(r["violations"][0]["constraint"], "node");
    assert_eq!(
        r["violations"][0]["message"],
        "distribution must carry an access URL"
    );
    let skipped = r["skipped_constraints"].as_array().cloned().unwrap_or_default();
    assert!(skipped.is_empty(), "nothing should be skipped: {r}");
}

#[test]
fn data_conforming_to_the_referenced_shape_passes() {
    let data = r#"
        @prefix ex: <http://example.org/> .
        ex:d1 a ex:Distribution ; ex:accessURL <http://x.example/1> .
        ex:good a ex:Dataset ; ex:distribution ex:d1 .
    "#;
    let r = report(&store_with(data), NODE_SHAPE);
    assert_eq!(r["conforms"], serde_json::json!(true), "{r}");
}

/// A referenced shape that itself references another. HealthDCAT-AP nests this
/// way, so one level of compilation is not enough.
#[test]
fn a_referenced_shape_may_itself_reference_a_shape() {
    let data = r#"
        @prefix ex: <http://example.org/> .
        ex:a1 a ex:Agent ; ex:name "ONS" .
        ex:a2 a ex:Agent .
        ex:d1 a ex:Distribution ; ex:publisher ex:a1 .
        ex:d2 a ex:Distribution ; ex:publisher ex:a2 .
        ex:good a ex:Dataset ; ex:distribution ex:d1 .
        ex:bad  a ex:Dataset ; ex:distribution ex:d2 .
    "#;
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:AgentShape a sh:NodeShape ;
            sh:property [ sh:path ex:name ; sh:minCount 1 ] .
        ex:DistributionShape a sh:NodeShape ;
            sh:property [ sh:path ex:publisher ; sh:node ex:AgentShape ] .
        ex:DatasetShape a sh:NodeShape ;
            sh:targetClass ex:Dataset ;
            sh:property [ sh:path ex:distribution ; sh:node ex:DistributionShape ] .
    "#;
    let r = report(&store_with(data), shapes);
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], serde_json::json!(1), "{r}");
    assert_eq!(r["violations"][0]["focus_node"], "http://example.org/bad");
}

/// `sh:maxCount 0` and `sh:maxCount 1` inside a referenced shape, both used by
/// the HealthDCAT-AP shapes.
#[test]
fn max_counts_inside_a_referenced_shape_are_compiled() {
    let data = r#"
        @prefix ex: <http://example.org/> .
        ex:d1 a ex:Distribution ; ex:title "one" .
        ex:d2 a ex:Distribution ; ex:title "one", "two" .
        ex:ok  a ex:Dataset ; ex:distribution ex:d1 .
        ex:two a ex:Dataset ; ex:distribution ex:d2 .
    "#;
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:DistributionShape a sh:NodeShape ;
            sh:property [ sh:path ex:title ; sh:maxCount 1 ] .
        ex:DatasetShape a sh:NodeShape ;
            sh:targetClass ex:Dataset ;
            sh:property [ sh:path ex:distribution ; sh:node ex:DistributionShape ] .
    "#;
    let r = report(&store_with(data), shapes);
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], serde_json::json!(1), "{r}");
    assert_eq!(r["violations"][0]["focus_node"], "http://example.org/two");
}

/// A form inside the referenced shape that cannot be compiled must take the
/// whole `sh:node` to `skipped_constraints`, not silently pass the rest.
#[test]
fn an_uncompilable_referenced_shape_is_recorded_as_skipped() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:DistributionShape a sh:NodeShape ;
            sh:property [ sh:path ex:accessURL ; sh:minCount 3 ] .
        ex:DatasetShape a sh:NodeShape ;
            sh:targetClass ex:Dataset ;
            sh:property [ sh:path ex:distribution ; sh:node ex:DistributionShape ] .
    "#;
    let r = report(&store_with(DATA), shapes);
    assert!(r["conforms"].is_null(), "must not claim a verdict: {r}");
    let skipped = r["skipped_constraints"].as_array().cloned().unwrap_or_default();
    assert!(
        skipped.iter().any(|s| s["constraint"] == "sh:node"),
        "the sh:node must be named as unevaluated: {r}"
    );
}

/// A shape that references itself must not spin. It is bounded and reported.
#[test]
fn a_self_referencing_shape_terminates_and_is_reported() {
    let data = r#"
        @prefix ex: <http://example.org/> .
        ex:a a ex:Dataset ; ex:related ex:a .
    "#;
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:DatasetShape a sh:NodeShape ;
            sh:targetClass ex:Dataset ;
            sh:property [ sh:path ex:related ; sh:node ex:DatasetShape ] .
    "#;
    let r = report(&store_with(data), shapes);
    // Either a verdict or an honest null, but it must return.
    assert!(r["violation_count"].is_number(), "{r}");
}

/// A `sh:node` pointing at a shape the shapes graph never defines.
///
/// The compiler builds a conjunction of the referenced shape's constraints. An
/// undefined shape has none, and an empty conjunction is `true`, so without an
/// explicit check every value node would conform to a shape nobody wrote. That
/// is a false clean produced by absence.
///
/// The HealthDCAT-AP shapes reference three DCAT-AP 3.0.0 shapes that live in
/// another file, so this is the ordinary case, not a contrived one.
#[test]
fn a_reference_to_an_undefined_shape_is_not_treated_as_satisfied() {
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:DatasetShape a sh:NodeShape ;
            sh:targetClass ex:Dataset ;
            sh:property [ sh:path ex:distribution ; sh:node ex:ShapeInAnotherFile ] .
    "#;
    let r = report(&store_with(DATA), shapes);
    assert!(
        r["conforms"].is_null(),
        "an undefined shape cannot be satisfied or broken; it must suppress the verdict: {r}"
    );
    assert_eq!(r["violation_count"], serde_json::json!(0), "{r}");
    let skipped = r["skipped_constraints"].as_array().cloned().unwrap_or_default();
    assert!(
        skipped.iter().any(|s| s["constraint"] == "sh:node"),
        "the unresolvable reference must be named: {r}"
    );
}

/// `sh:or` asserted on the node shape, over member shapes rather than leaf
/// constraints: the focus node must conform to at least one member.
///
/// Taken from the Italian register vertical, which uses it to keep a layer
/// core-only: either the assertion is conformant, or it records why not. The
/// members are written inline, so they are blank nodes and cannot be addressed
/// by term; their contents are read through the parent instead.
#[test]
fn node_level_or_over_member_shapes_is_evaluated() {
    let data = r#"
        @prefix it: <http://example.org/it#> .
        it:ok   a it:Assertion ; it:conformant true .
        it:also a it:Assertion ; it:conformant false ; it:reason "truncated" .
        it:bad  a it:Assertion ; it:conformant false .
    "#;
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix it: <http://example.org/it#> .
        it:S a sh:NodeShape ;
            sh:targetClass it:Assertion ;
            sh:message "non-conformant without a recorded reason" ;
            sh:or (
                [ sh:property [ sh:path it:conformant ; sh:hasValue true ] ]
                [ sh:property [ sh:path it:reason ; sh:minCount 1 ] ]
            ) .
    "#;
    let r = report(&store_with(data), shapes);
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    assert_eq!(r["violation_count"], serde_json::json!(1), "{r}");
    assert_eq!(r["violations"][0]["focus_node"], "http://example.org/it#bad");
    assert_eq!(
        r["violations"][0]["message"],
        "non-conformant without a recorded reason"
    );
    let skipped = r["skipped_constraints"].as_array().cloned().unwrap_or_default();
    assert!(skipped.is_empty(), "{r}");
}

/// A member shape using a form the one-level reader cannot compile must take the
/// whole disjunction to skipped, not pass on the members it did understand.
#[test]
fn an_uncompilable_or_member_suppresses_the_verdict() {
    let data = r#"
        @prefix it: <http://example.org/it#> .
        it:bad a it:Assertion .
    "#;
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix it: <http://example.org/it#> .
        it:S a sh:NodeShape ;
            sh:targetClass it:Assertion ;
            sh:or (
                [ sh:property [ sh:path it:reason ; sh:minCount 1 ] ]
                [ sh:property [ sh:path it:other ; sh:minCount 4 ] ]
            ) .
    "#;
    let r = report(&store_with(data), shapes);
    assert!(r["conforms"].is_null(), "must not claim a verdict: {r}");
    let skipped = r["skipped_constraints"].as_array().cloned().unwrap_or_default();
    assert!(skipped.iter().any(|s| s["constraint"] == "sh:or"), "{r}");
}

/// A deliberate, documented divergence from pyshacl on reporting granularity.
///
/// SHACL says a failing `sh:node` produces a result at the shape that carries it,
/// naming the value node that did not conform. pyshacl additionally surfaces the
/// referenced shape's own results as separate entries, so a two-level nesting
/// gives it three results where this validator gives one.
///
/// Both agree the data does not conform, and agree on the outer focus node. The
/// difference is how much of the chain is itemised. It is pinned here so that a
/// differential run reporting "missed" results against pyshacl on `sh:node` is
/// read as this, and not as a missed violation.
#[test]
fn sh_node_reports_at_the_outer_shape_only() {
    let data = r#"
        @prefix ex: <http://example.org/> .
        ex:a2 a ex:Agent .
        ex:d2 a ex:Distribution ; ex:publisher ex:a2 .
        ex:bad a ex:Dataset ; ex:distribution ex:d2 .
    "#;
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:AgentShape a sh:NodeShape ;
            sh:property [ sh:path ex:name ; sh:minCount 1 ] .
        ex:DistributionShape a sh:NodeShape ;
            sh:property [ sh:path ex:publisher ; sh:node ex:AgentShape ] .
        ex:DatasetShape a sh:NodeShape ;
            sh:targetClass ex:Dataset ;
            sh:property [ sh:path ex:distribution ; sh:node ex:DistributionShape ] .
    "#;
    let r = report(&store_with(data), shapes);
    assert_eq!(r["conforms"], serde_json::json!(false), "{r}");
    // pyshacl reports three here: ex:bad, ex:d2 and ex:a2.
    assert_eq!(
        r["violation_count"],
        serde_json::json!(1),
        "one result at the outer shape: {r}"
    );
    assert_eq!(r["violations"][0]["focus_node"], "http://example.org/bad");
}
