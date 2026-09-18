//! **Issue #168.** A focus node reachable by more than one type path was
//! selected once per path, so every core constraint checked it more than once
//! and reported each violation more than once.
//!
//! The mechanism is not a duplicate triple, which an RDF graph cannot hold. A
//! target of `sh:targetClass :Animal` compiles to the property path
//! `?focus rdf:type/rdfs:subClassOf* :Animal`, and a node typed as two classes
//! that are both subclasses of the target matches that path once for each, so
//! `?focus` binds twice. `violation_count` and `focus_nodes` are both inflated,
//! and those are exactly what a caller reads to decide whether a graph is clean
//! and how much was actually checked.
//!
//! Found by a differential oracle against pySHACL 0.40.1 on a 74-shape OWL 2 DL
//! corpus: this engine returned 570 results where pySHACL returned 548, and the
//! disagreement ran in one direction in every row.
//!
//! **Fixed in the query construction, not by collapsing results.** SHACL 5.3.2
//! emits one result per SPARQL solution and both implementations do; collapsing
//! identical results took a 39-shape corpus from 249 to 245 and broke an exact
//! agreement with pySHACL. The focus selection is now a `SELECT DISTINCT`
//! subquery, which is what the `sh:or` path at section 5 already did.

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use std::sync::Arc;

/// `:rex` is a `:Dog` and a `:Mammal`, and both are subclasses of `:Animal`, so
/// the target path reaches it twice.
const DATA: &str = r#"
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix ex:   <http://example.org/> .

    ex:Dog    rdfs:subClassOf ex:Animal .
    ex:Mammal rdfs:subClassOf ex:Animal .

    ex:rex a ex:Dog , ex:Mammal ;
           ex:addr ex:notAnAddress .

    ex:notAnAddress a ex:Building .   # NOT an ex:Animal, so not a focus node itself
"#;

const SHAPES: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    ex:AnimalShape a sh:NodeShape ;
        sh:targetClass ex:Animal ;
        sh:property [ sh:path ex:addr ; sh:class ex:Address ] .
"#;

#[test]
fn a_focus_node_reachable_by_two_type_paths_is_one_focus_node() {
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(DATA, None).expect("load");
    let json = ShaclValidator::validate(&graph, SHAPES).expect("validate");
    let r: serde_json::Value = serde_json::from_str(&json).expect("parse");

    assert_eq!(
        r["focus_nodes"],
        serde_json::json!(1),
        "one node reachable by two paths is still one focus node: {r}"
    );
}

#[test]
fn a_violation_is_reported_once_per_node_not_once_per_type_path() {
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(DATA, None).expect("load");
    let json = ShaclValidator::validate(&graph, SHAPES).expect("validate");
    let r: serde_json::Value = serde_json::from_str(&json).expect("parse");

    assert_eq!(r["conforms"], serde_json::json!(false), "report: {r}");
    assert_eq!(
        r["violation_count"],
        serde_json::json!(1),
        "ex:rex has one bad ex:addr, so one violation, not one per type path: {r}"
    );
}

/// The sibling case, so the fix cannot be read as special to `rdf:type`. Two
/// subclass steps reach the same node by two routes through the hierarchy.
#[test]
fn a_diamond_in_the_class_hierarchy_does_not_multiply_the_focus_set() {
    let graph = Arc::new(GraphStore::new());
    graph
        .load_turtle(
            r#"
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            @prefix ex:   <http://example.org/> .
            ex:Left  rdfs:subClassOf ex:Top .
            ex:Right rdfs:subClassOf ex:Top .
            ex:Both  rdfs:subClassOf ex:Left , ex:Right .
            ex:thing a ex:Both .
        "#,
            None,
        )
        .expect("load");
    let json = ShaclValidator::validate(
        &graph,
        r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:TopShape a sh:NodeShape ;
            sh:targetClass ex:Top ;
            sh:property [ sh:path ex:name ; sh:minCount 1 ] .
    "#,
    )
    .expect("validate");
    let r: serde_json::Value = serde_json::from_str(&json).expect("parse");

    assert_eq!(
        r["focus_nodes"],
        serde_json::json!(1),
        "ex:thing reaches ex:Top by two routes and is one node: {r}"
    );
    assert_eq!(
        r["violation_count"],
        serde_json::json!(1),
        "and is missing ex:name once: {r}"
    );
}
