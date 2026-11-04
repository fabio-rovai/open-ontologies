// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Regression tests for issue #131.
//!
//! Violation objects carried `constraint`, `focus_node`, `message` and
//! `severity` and nothing that named the shape that produced them. A report
//! built as one shape per rule, which is how a findings table is written, could
//! not be attributed except by parsing free text out of `message`. The W3C
//! validation report vocabulary carries `sh:sourceShape` and
//! `sh:sourceConstraintComponent` on every result for exactly this reason.
//!
//! `source_shape` is the shape that CARRIES the constraint. For a constraint
//! under `sh:property` that is the property shape, not the node shape holding
//! it, and the difference is the whole point: two property shapes on one path
//! under one node shape are precisely the case the issue describes. The first
//! version of this fix named the node shape for all fifteen property-borne
//! constraint sites, which left that case as unattributable as it was before,
//! and every test here was one-rule-per-node-shape and could not see it.
//! `two_property_shapes_on_one_path_are_told_apart` is that missing test.
//!
//! The enclosing node shape is still reported, under `node_shape`.

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

const SH: &str = "http://www.w3.org/ns/shacl#";

fn store() -> Arc<GraphStore> {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:w a ex:Widget ; ex:size 99 ; ex:colour "red" ; ex:part ex:notAPart ; ex:tag "a" , "b" .
        ex:notAPart a ex:Other .
    "#;
    store.load_turtle(ttl, None).unwrap();
    store
}

fn report(store: &Arc<GraphStore>, shapes: &str) -> serde_json::Value {
    serde_json::from_str(&ShaclValidator::validate(store, shapes).unwrap()).unwrap()
}

/// One named property shape per rule, so `source_shape` is meaningful rather
/// than a blank label.
const SHAPES: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

    ex:WidgetShape a sh:NodeShape ; sh:targetClass ex:Widget ;
      sh:property ex:MinCountRule , ex:MaxCountRule , ex:DatatypeRule , ex:ClassRule ,
                  ex:PatternRule , ex:InRule , ex:NodeKindRule , ex:HasValueRule ,
                  ex:MaxInclusiveRule , ex:MinLengthRule .

    ex:MinCountRule     sh:path ex:missing ; sh:minCount 1 .
    ex:MaxCountRule     sh:path ex:tag     ; sh:maxCount 1 .
    ex:DatatypeRule     sh:path ex:size    ; sh:datatype xsd:string .
    ex:ClassRule        sh:path ex:part    ; sh:class ex:Part .
    ex:PatternRule      sh:path ex:colour  ; sh:pattern "^blue$" .
    ex:InRule           sh:path ex:colour  ; sh:in ( "blue" "green" ) .
    ex:NodeKindRule     sh:path ex:colour  ; sh:nodeKind sh:IRI .
    ex:HasValueRule     sh:path ex:colour  ; sh:hasValue "blue" .
    ex:MaxInclusiveRule sh:path ex:size    ; sh:maxInclusive 10 .
    ex:MinLengthRule    sh:path ex:colour  ; sh:minLength 10 .

    ex:SparqlRule a sh:NodeShape ; sh:targetClass ex:Widget ;
      sh:sparql [ sh:message "too big" ;
        sh:select "SELECT $this WHERE { $this <http://example.org/size> ?s . FILTER(?s > 10) }" ] .
"#;

fn violations_of(r: &serde_json::Value, shape: &str) -> Vec<serde_json::Value> {
    r["violations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["source_shape"] == shape)
        .cloned()
        .collect()
}

#[test]
fn every_violation_names_its_source_shape_and_component() {
    let r = report(&store(), SHAPES);
    let violations = r["violations"].as_array().unwrap();
    assert!(violations.len() >= 11, "expected one violation per rule at least: {r}");
    for v in violations {
        let shape = v["source_shape"].as_str().unwrap_or_else(|| panic!("no source_shape: {v}"));
        assert!(shape.starts_with("http://example.org/"), "source_shape is the shape IRI: {v}");
        assert!(
            v["node_shape"].as_str().unwrap_or("").starts_with("http://example.org/"),
            "the enclosing node shape is reported too: {v}"
        );
        let component = v["source_constraint_component"]
            .as_str()
            .unwrap_or_else(|| panic!("no source_constraint_component: {v}"));
        assert!(
            component.starts_with(SH) && component.ends_with("ConstraintComponent"),
            "component must be a sh: constraint component IRI: {v}"
        );
        // The keys the issue asked to keep are still there.
        for key in ["constraint", "focus_node", "message", "severity"] {
            assert!(v.get(key).is_some(), "existing key {key} must survive: {v}");
        }
        if let Some(path) = v.get("path") {
            assert_eq!(v["result_path"], *path, "result_path mirrors path: {v}");
        }
    }
}

#[test]
fn components_are_the_w3c_names() {
    let r = report(&store(), SHAPES);
    let expect = [
        ("http://example.org/MinCountRule", "MinCountConstraintComponent", Some("http://example.org/missing")),
        ("http://example.org/MaxCountRule", "MaxCountConstraintComponent", Some("http://example.org/tag")),
        ("http://example.org/DatatypeRule", "DatatypeConstraintComponent", Some("http://example.org/size")),
        ("http://example.org/ClassRule", "ClassConstraintComponent", Some("http://example.org/part")),
        ("http://example.org/PatternRule", "PatternConstraintComponent", Some("http://example.org/colour")),
        ("http://example.org/InRule", "InConstraintComponent", Some("http://example.org/colour")),
        ("http://example.org/NodeKindRule", "NodeKindConstraintComponent", Some("http://example.org/colour")),
        ("http://example.org/HasValueRule", "HasValueConstraintComponent", Some("http://example.org/colour")),
        ("http://example.org/MaxInclusiveRule", "MaxInclusiveConstraintComponent", Some("http://example.org/size")),
        ("http://example.org/MinLengthRule", "MinLengthConstraintComponent", Some("http://example.org/colour")),
        ("http://example.org/SparqlRule", "SPARQLConstraintComponent", None),
    ];
    for (shape, component, path) in expect {
        let vs = violations_of(&r, shape);
        assert!(!vs.is_empty(), "{shape} produced no violation: {r}");
        for v in &vs {
            assert_eq!(v["source_constraint_component"], format!("{SH}{component}"), "{v}");
            if let Some(p) = path {
                assert_eq!(v["result_path"], p, "{v}");
            }
        }
    }
}

#[test]
fn two_property_shapes_on_one_path_are_told_apart() {
    // The case #131 was filed about, and the one the first fix could not
    // handle: both rules sit under ONE node shape and constrain ONE path.
    // pyshacl 0.40.1 returns ex:RuleA and ex:RuleB here.
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:WidgetShape a sh:NodeShape ; sh:targetClass ex:Widget ;
          sh:property ex:RuleA , ex:RuleB .
        ex:RuleA sh:path ex:colour ; sh:pattern "^blue$"  ; sh:message "cell not evidenced" .
        ex:RuleB sh:path ex:colour ; sh:minLength 10      ; sh:message "cell not evidenced" .
    "#;
    let r = report(&store(), shapes);
    assert_eq!(r["violation_count"], 2, "{r}");
    let mut seen: Vec<String> = r["violations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["source_shape"].as_str().unwrap().to_string())
        .collect();
    seen.sort();
    assert_eq!(
        seen,
        vec!["http://example.org/RuleA", "http://example.org/RuleB"],
        "two rules on one path under one node shape must be distinguishable: {r}"
    );
    for v in r["violations"].as_array().unwrap() {
        assert_eq!(
            v["node_shape"], "http://example.org/WidgetShape",
            "the enclosing node shape is still reported: {v}"
        );
    }
}

#[test]
fn the_shape_is_attributed_even_when_messages_are_identical() {
    // Two node shapes carrying one sh:sparql each. Here the shape that carries
    // the constraint IS the node shape, so that is what source_shape names.
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:RuleA a sh:NodeShape ; sh:targetClass ex:Widget ;
          sh:sparql [ sh:message "cell not evidenced" ;
            sh:select "SELECT $this WHERE { FILTER NOT EXISTS { $this <http://example.org/a> ?x } }" ] .
        ex:RuleB a sh:NodeShape ; sh:targetClass ex:Widget ;
          sh:sparql [ sh:message "cell not evidenced" ;
            sh:select "SELECT $this WHERE { FILTER NOT EXISTS { $this <http://example.org/b> ?x } }" ] .
    "#;
    let r = report(&store(), shapes);
    assert_eq!(r["violation_count"], 2, "{r}");
    let mut shapes_seen: Vec<String> = r["violations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["source_shape"].as_str().unwrap().to_string())
        .collect();
    shapes_seen.sort();
    assert_eq!(shapes_seen, vec!["http://example.org/RuleA", "http://example.org/RuleB"]);
}

#[test]
fn a_blank_property_shape_reports_its_blank_label_not_the_node_shape() {
    // pyshacl reports the blank property shape here. A blank label is less
    // useful than an IRI and is still strictly more identity than collapsing
    // every sibling onto the node shape, which is what the first fix did.
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:WidgetShape a sh:NodeShape ; sh:targetClass ex:Widget ;
          sh:property [ sh:path ex:colour ; sh:pattern "^blue$" ] ;
          sh:property [ sh:path ex:colour ; sh:minLength 10 ] .
    "#;
    let r = report(&store(), shapes);
    assert_eq!(r["violation_count"], 2, "{r}");
    let seen: std::collections::HashSet<String> = r["violations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["source_shape"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(seen.len(), 2, "the two blank property shapes must not collapse: {r}");
    for s in &seen {
        assert!(s.starts_with("_:"), "a blank property shape reports a blank label: {s}");
    }
}
