//! Realizing individuals into classes DEFINED by owl:equivalentClass.
//!
//! `check_abox` reads concept labels off one completed tableau. That is sound for detecting
//! inconsistency but it is not realization: a concept satisfied in whichever model the
//! expansion happened to find is not thereby entailed, and the branch that would add a
//! defined class is usually not the branch recorded. The observable effect was that defining
//! a class by owl:equivalentClass and letting the reasoner find its members did not work at
//! all, for any ontology. Three separate causes had to be fixed:
//!
//!   1. individual nodes carried no atom for themselves, so the nominal approximation behind
//!      owl:hasValue could never be satisfied;
//!   2. the class subsumption closure was built from the ROLE hierarchy, so a definition
//!      naming a superclass never matched an individual typed by a subclass;
//!   3. datatype-property assertions were discarded entirely, so a definition selecting
//!      individuals by a boolean flag matched nothing.
//!
//! Found by running this engine against an ontology of our own whose defect classes are all
//! defined by equivalence, and cross-checked against an OWL RL reasoner, which derives the
//! same members.

use open_ontologies::graph::GraphStore;
use std::sync::Arc;

const ONTOLOGY: &str = r#"
@prefix : <http://ex.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

:Term a owl:Class .
:PropertyTerm a owl:Class ; rdfs:subClassOf :Term .
:Status a owl:Class .
:Dead a owl:NamedIndividual, :Status .

:status a owl:ObjectProperty .
:usedBy a owl:ObjectProperty .
:closed a owl:DatatypeProperty .

# defined by a superclass conjunct plus two existentials
:Zombie a owl:Class ; owl:equivalentClass [ a owl:Class ; owl:intersectionOf (
    :Term
    [ a owl:Restriction ; owl:onProperty :status ; owl:hasValue :Dead ]
    [ a owl:Restriction ; owl:onProperty :usedBy ; owl:someValuesFrom :Term ] ) ] .

# defined by a boolean flag on a datatype property
:Closed a owl:Class ; owl:equivalentClass [ a owl:Class ; owl:intersectionOf (
    :Term [ a owl:Restriction ; owl:onProperty :closed ; owl:hasValue true ] ) ] .

:host a owl:NamedIndividual, :PropertyTerm .

# dead and still used: a zombie
:zombie a owl:NamedIndividual, :PropertyTerm ; :status :Dead ; :usedBy :host .
# dead but detached: must NOT be a zombie
:retired a owl:NamedIndividual, :PropertyTerm ; :status :Dead .
# used but alive: must NOT be a zombie
:live a owl:NamedIndividual, :PropertyTerm ; :usedBy :host .
# boolean flag set, and not set
:shut a owl:NamedIndividual, :PropertyTerm ; :closed true .
:open a owl:NamedIndividual, :PropertyTerm ; :closed false .
"#;

fn realized(ttl: &str) -> serde_json::Value {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(ttl, None).unwrap();
    let out = open_ontologies::tableaux::DlReasoner::run(&store, false).unwrap();
    serde_json::from_str(&out).unwrap()
}

fn types_of(v: &serde_json::Value, individual: &str) -> Vec<String> {
    v["abox"]["inferred"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter(|r| r["individual"].as_str().unwrap_or("").trim_end_matches('>').ends_with(individual))
                .flat_map(|r| r["inferred_types"].as_array().cloned().unwrap_or_default())
                .filter_map(|t| t.as_str().map(|s| s.rsplit('/').next().unwrap_or(s).trim_end_matches('>').to_string()))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn an_individual_is_realized_into_a_class_defined_by_equivalence() {
    let v = realized(ONTOLOGY);
    assert!(
        types_of(&v, "/zombie").contains(&"Zombie".to_string()),
        "an individual meeting every conjunct must be realized, got {:?}",
        types_of(&v, "/zombie")
    );
}

#[test]
fn an_individual_missing_a_conjunct_is_not_realized() {
    let v = realized(ONTOLOGY);
    for who in ["/retired", "/live"] {
        assert!(
            !types_of(&v, who).contains(&"Zombie".to_string()),
            "{who} satisfies only part of the definition and must not be realized, got {:?}",
            types_of(&v, who)
        );
    }
}

#[test]
fn a_definition_over_a_datatype_flag_is_realized() {
    let v = realized(ONTOLOGY);
    assert!(
        types_of(&v, "/shut").contains(&"Closed".to_string()),
        "owl:hasValue on a datatype property must select members, got {:?}",
        types_of(&v, "/shut")
    );
    assert!(
        !types_of(&v, "/open").contains(&"Closed".to_string()),
        "the opposite flag value must not match, got {:?}",
        types_of(&v, "/open")
    );
}

#[test]
fn a_subclass_typed_individual_satisfies_a_superclass_conjunct() {
    // :zombie is typed :PropertyTerm and the definition names :Term. This is the case the
    // role-hierarchy mix-up broke: the class closure was empty, so nothing ever matched.
    let v = realized(ONTOLOGY);
    assert!(
        types_of(&v, "/zombie").contains(&"Zombie".to_string()),
        "asserted subclass must satisfy a superclass conjunct"
    );
}
