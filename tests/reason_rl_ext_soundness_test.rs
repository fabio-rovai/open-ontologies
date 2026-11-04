// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Soundness of the `owl-rl-ext` forward-chaining rules.
//!
//! Found 13 Sep 2026 while designing the Lean certificate layer: `cls-svf1`
//! derived two things no OWL semantics licenses.
//!
//!   1. From `C rdfs:subClassOf [ owl:onProperty p ; owl:someValuesFrom D ]`,
//!      `x p y` and `y a D`, it inferred `x a C`. That is the CONVERSE of the
//!      axiom: being in `∃p.D` says nothing about being in `C`. Only an
//!      `owl:equivalentClass` licenses the step, and it is licensed there by
//!      the two `rdfs:subClassOf` triples the equivalence expands to.
//!   2. From `z p D`, where `D` is the filler class IRI itself, it inferred
//!      `z a ∃p.D`. A class used in object position is a resource, not an
//!      instance of itself.
//!
//! Every rule the reasoner applies has to correspond to a rule the Lean checker
//! can prove sound. These two had no such rule, which is how they were found.

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::Reasoner;
use std::sync::Arc;

const PREFIXES: &str = r#"
    @prefix : <http://ex.org/> .
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
"#;

fn run(ttl: &str) -> Arc<GraphStore> {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(&format!("{PREFIXES}{ttl}"), None).unwrap();
    Reasoner::run(&store, "owl-rl-ext", true).unwrap();
    store
}

fn holds(store: &Arc<GraphStore>, s: &str, o: &str) -> bool {
    let q = format!(
        "SELECT ?s WHERE {{ <http://ex.org/{s}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://ex.org/{o}> }}"
    );
    let json: serde_json::Value =
        serde_json::from_str(&store.sparql_select_union(&q).unwrap()).unwrap();
    !json["results"].as_array().unwrap().is_empty()
}

fn restriction_members(store: &Arc<GraphStore>) -> Vec<String> {
    let q = "SELECT ?x WHERE { ?x a ?r . ?r <http://www.w3.org/2002/07/owl#onProperty> ?p }";
    let json: serde_json::Value =
        serde_json::from_str(&store.sparql_select_union(q).unwrap()).unwrap();
    let mut v: Vec<String> = json["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["x"].as_str().unwrap().to_string())
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn a_subclass_of_a_restriction_is_not_inferred_from_the_restriction() {
    let store = run(r#"
        :C rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :p ; owl:someValuesFrom :D ] .
        :x :p :y .
        :y a :D .
    "#);
    assert!(
        !holds(&store, "x", "C"),
        "x ∈ ∃p.D and C ⊑ ∃p.D do not give x ∈ C (converse of the axiom)"
    );
    // The sound half of the rule still fires: x is in the restriction class.
    assert_eq!(restriction_members(&store), vec!["<http://ex.org/x>".to_string()]);
}

#[test]
fn a_class_iri_in_object_position_is_not_an_instance_of_itself() {
    let store = run(r#"
        :E owl:equivalentClass [ a owl:Restriction ; owl:onProperty :p ; owl:someValuesFrom :D ] .
        :z :p :D .
    "#);
    assert!(!holds(&store, "z", "E"), "z p D says nothing about z ∈ ∃p.D");
    assert!(restriction_members(&store).is_empty(), "nobody is in ∃p.D here");
}

#[test]
fn an_equivalent_class_of_a_restriction_is_still_inferred() {
    // Positive control: the case the unsound rule happened to get right must
    // keep working through the sound route (equivalentClass expands to two
    // subClassOf triples, and rdfs9 carries membership across one of them).
    let store = run(r#"
        :E owl:equivalentClass [ a owl:Restriction ; owl:onProperty :p ; owl:someValuesFrom :D ] .
        :x :p :y .
        :y a :D .
    "#);
    assert!(holds(&store, "x", "E"), "x ∈ ∃p.D and E ≡ ∃p.D give x ∈ E");
}

#[test]
fn has_value_rules_are_unchanged() {
    let store = run(r#"
        :F rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :q ; owl:hasValue :v ] .
        :a a :F .
        :G owl:equivalentClass [ a owl:Restriction ; owl:onProperty :q ; owl:hasValue :v ] .
        :b :q :v .
    "#);
    let q = "SELECT ?s WHERE { <http://ex.org/a> <http://ex.org/q> <http://ex.org/v> }";
    let json: serde_json::Value =
        serde_json::from_str(&store.sparql_select_union(q).unwrap()).unwrap();
    assert!(!json["results"].as_array().unwrap().is_empty(), "F ⊑ ∃q.{{v}} and a ∈ F give a q v");
    assert!(holds(&store, "b", "G"), "b q v and G ≡ ∃q.{{v}} give b ∈ G");
}
