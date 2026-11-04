// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! The reasoner must reach a fixpoint of its own rule set in one run.
//!
//! Found by an adversarial audit, 13 Sep 2026. Only the three data indices
//! (`rdf:type`, `rdfs:subClassOf`, `rdfs:subPropertyOf`) were rebuilt each
//! iteration. Every schema index (`rdfs:domain`, `rdfs:range`, the property
//! characteristics, `owl:inverseOf`, the equivalences, the restriction maps and
//! the RDF lists behind `owl:intersectionOf` / `owl:unionOf`) was filtered once
//! out of the pre-loop snapshot. A schema triple the reasoner itself derived
//! was therefore never used as a premise, and running `reason` twice derived
//! more than running it once.
//!
//! For query answers that is incompleteness. For certificates it is worse:
//! materialising turns run N's conclusions into run N+1's premises, so
//! `asserted.tsv` can list the reasoner's own output as an axiom with nothing
//! marking it as derived, and `certificate_sound` is conditional on the
//! assertions. One run reaching the fixpoint is what makes one certificate the
//! whole story.

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::Reasoner;
use std::sync::Arc;

const PREFIXES: &str = r#"
    @prefix : <http://ex.org/> .
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
"#;

fn run(ttl: &str, profile: &str) -> (Arc<GraphStore>, serde_json::Value) {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(&format!("{PREFIXES}{ttl}"), None).unwrap();
    let out = Reasoner::run(&store, profile, true).unwrap();
    let parsed = serde_json::from_str(&out).unwrap();
    (store, parsed)
}

fn holds(store: &Arc<GraphStore>, query: &str) -> bool {
    let json: serde_json::Value =
        serde_json::from_str(&store.sparql_select_union(query).unwrap()).unwrap();
    !json["results"].as_array().unwrap().is_empty()
}

/// `rdfs:domain` reached through a subproperty of `rdfs:domain`: rdfs7 derives
/// the domain axiom, and rdfs2 has to be able to use it in the same run.
const DERIVED_DOMAIN: &str = r#"
    :myDomain rdfs:subPropertyOf rdfs:domain .
    :p :myDomain :C .
    :x :p :y .
"#;

#[test]
fn a_derived_domain_axiom_is_used_in_the_same_run() {
    let (store, r) = run(DERIVED_DOMAIN, "rdfs");
    assert!(
        holds(&store, "SELECT ?s WHERE { <http://ex.org/x> a <http://ex.org/C> }"),
        "rdfs7 derives the domain axiom, so rdfs2 must fire on it in the same run: {r}"
    );
}

#[test]
fn a_second_run_derives_nothing_new() {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(&format!("{PREFIXES}{DERIVED_DOMAIN}"), None).unwrap();
    let first: serde_json::Value =
        serde_json::from_str(&Reasoner::run(&store, "rdfs", true).unwrap()).unwrap();
    let second: serde_json::Value =
        serde_json::from_str(&Reasoner::run(&store, "rdfs", true).unwrap()).unwrap();
    assert!(first["inferred_count"].as_u64().unwrap() > 0, "the first run must do work: {first}");
    assert_eq!(
        second["inferred_count"], 0,
        "the first run must reach the fixpoint, so the second has nothing to add: {second}"
    );
}

#[test]
fn a_derived_equivalence_feeds_the_restriction_rules() {
    // scm-eqc turns the equivalence into subClassOf, rdfs9 carries membership
    // across it, and cls-hv1 then fires on a restriction reached only that way.
    let (store, r) = run(
        r#"
        :G owl:equivalentClass :R .
        :R a owl:Restriction ; owl:onProperty :col ; owl:hasValue :red .
        :b a :G .
        "#,
        "owl-rl-ext",
    );
    assert!(
        holds(&store, "SELECT ?s WHERE { <http://ex.org/b> <http://ex.org/col> <http://ex.org/red> }"),
        "b is a G, G is the restriction, so b has the value: {r}"
    );
}

#[test]
fn cls_hv1_fires_on_an_individual_typed_with_the_restriction_directly() {
    // The engine used to emit a rule named `cls-hv1` that was the composite of
    // cax-sco and W3C cls-hv1: it demanded an explicit subClassOf hop, so this
    // ontology derived nothing at all.
    let (store, r) = run(
        r#"
        :R a owl:Restriction ; owl:onProperty :col ; owl:hasValue :red .
        :k a :R .
        "#,
        "owl-rl-ext",
    );
    assert!(
        holds(&store, "SELECT ?s WHERE { <http://ex.org/k> <http://ex.org/col> <http://ex.org/red> }"),
        "x rdf:type R with R = (col hasValue red) gives x col red: {r}"
    );
}

#[test]
fn the_subclass_route_to_cls_hv1_still_works() {
    // The case the composite rule did handle, which must not regress: it is now
    // rdfs9 followed by cls-hv1 rather than one rule.
    let (store, r) = run(
        r#"
        :R a owl:Restriction ; owl:onProperty :col ; owl:hasValue :red .
        :K rdfs:subClassOf :R .
        :k a :K .
        "#,
        "owl-rl-ext",
    );
    assert!(
        holds(&store, "SELECT ?s WHERE { <http://ex.org/k> <http://ex.org/col> <http://ex.org/red> }"),
        "{r}"
    );
}

#[test]
fn a_range_inference_onto_a_blank_node_is_not_dropped() {
    // The rdfs3 guard required the object to be an IRI when the invariant it
    // needs is "not a literal". Every range inference onto a blank node was
    // dropped, so the blank node never got typed and rdfs9 starved behind it.
    let (store, r) = run(
        r#"
        :addressOf rdfs:range :Address .
        :Address rdfs:subClassOf :Location .
        :person :addressOf [ :street "Main" ] .
        "#,
        "rdfs",
    );
    assert!(
        holds(&store, "SELECT ?b WHERE { ?b a <http://ex.org/Address> . ?b <http://ex.org/street> ?s }"),
        "the blank node must get its range type: {r}"
    );
    assert!(
        holds(&store, "SELECT ?b WHERE { ?b a <http://ex.org/Location> . ?b <http://ex.org/street> ?s }"),
        "and rdfs9 must then carry it up the hierarchy: {r}"
    );
}

#[test]
fn a_literal_object_still_gets_no_range_type() {
    // The other direction: a literal cannot be the subject of the conclusion,
    // so widening the guard must not start emitting one.
    let (store, _) = run(
        r#"
        :name rdfs:range :Name .
        :person :name "Ada" .
        "#,
        "rdfs",
    );
    assert!(
        !holds(&store, "SELECT ?s WHERE { ?s a <http://ex.org/Name> }"),
        "no subject can be a literal"
    );
}
