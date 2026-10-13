//! **A restriction the DL parser cannot read must not become `⊤`.**
//!
//! `parse_restriction` returned `RawConcept::Top` for any restriction it did not
//! recognise: `owl:hasSelf`, or a restriction with no `owl:onProperty`. On the
//! right of an inclusion that is a harmless weakening. On the left it is not:
//! an `owl:equivalentClass` puts both sides on the left once, so
//! `Narcissist ≡ ∃loves.Self` became `⊤ ⊑ Narcissist`, and every class in the
//! ontology was reported a subclass of `Narcissist`. That is an unsound answer,
//! the one direction this reasoner promises never to err in.
//!
//! The repair reads an unrecognised restriction as an opaque atom named by its
//! own blank node. Replacing a sub-expression by a fresh name everywhere it
//! occurs is a sound approximation in both polarities: every model of the
//! ontology extends to a model of the rewritten one by giving the name the
//! expression's extension, so anything the rewritten ontology entails over the
//! original names, the original entails too.
//!
//! Anonymous inverse roles are the other half. `owl:onProperty [ owl:inverseOf
//! p ]` was read through the generic `owl:inverseOf` collector, which keeps ONE
//! inverse per role, so with two such blank nodes for the same `p` only the last
//! was linked to `p` and a `∀` through the other was inert.

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::Reasoner;
use std::sync::Arc;

const PREFIXES: &str = r#"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ex: <http://example.org/> .
"#;

fn supers(ttl: &str, class: &str) -> Vec<String> {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(&format!("{PREFIXES}{ttl}"), None).unwrap();
    let out = Reasoner::run(&store, "owl-dl", false).unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    // `complete` is false wherever a construct is not modelled (`owl:hasSelf`
    // here), so the search having finished is what this fixture needs.
    assert_eq!(v["search_finished"], true, "fixture must be decided: {v}");
    let want = format!("<http://example.org/{class}>");
    v["classification"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["class"] == want.as_str())
        .map(|row| {
            row["superclasses"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap().trim_start_matches("<http://example.org/").trim_end_matches('>').to_string())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn has_self_on_the_left_of_an_inclusion_does_not_make_everything_an_instance() {
    let s = supers(
        r#"
        ex:loves a owl:ObjectProperty .
        ex:Person a owl:Class .
        ex:Narcissist a owl:Class ; owl:equivalentClass
            [ a owl:Restriction ; owl:onProperty ex:loves ; owl:hasSelf true ] .
        "#,
        "Person",
    );
    assert!(
        !s.contains(&"Narcissist".to_string()),
        "Person ⊑ Narcissist is not entailed; the hasSelf restriction was read as ⊤: {s:?}"
    );
}

#[test]
fn a_restriction_without_on_property_does_not_become_top() {
    let s = supers(
        r#"
        ex:Person a owl:Class .
        ex:Odd a owl:Class ; owl:equivalentClass
            [ a owl:Restriction ; owl:someValuesFrom ex:Person ] .
        "#,
        "Person",
    );
    assert!(!s.contains(&"Odd".to_string()), "Person ⊑ Odd is not entailed: {s:?}");
}

#[test]
fn every_anonymous_inverse_of_a_property_is_its_inverse() {
    // Two separate blank nodes both say `inverse(hasChild)`. Each ∀ through one
    // of them must reach the parent of a child.
    let ttl = r#"
        ex:hasChild a owl:ObjectProperty .
        ex:Parent a owl:Class ; rdfs:subClassOf
            [ a owl:Restriction ; owl:onProperty ex:hasChild ; owl:someValuesFrom ex:Child ] .
        ex:Child a owl:Class ; rdfs:subClassOf
            [ a owl:Restriction ; owl:onProperty [ owl:inverseOf ex:hasChild ] ; owl:allValuesFrom ex:Adult ] ,
            [ a owl:Restriction ; owl:onProperty [ owl:inverseOf ex:hasChild ] ; owl:allValuesFrom ex:Guardian ] .
        ex:Adult a owl:Class .
        ex:Guardian a owl:Class .
    "#;
    let s = supers(ttl, "Parent");
    assert!(s.contains(&"Adult".to_string()), "Parent ⊑ Adult is entailed: {s:?}");
    assert!(s.contains(&"Guardian".to_string()), "Parent ⊑ Guardian is entailed: {s:?}");
}
