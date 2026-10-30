//! **Axioms the DL parser used to skip in silence.**
//!
//! The parser read binary `owl:disjointWith` and nothing else that states
//! disjointness, and it did not read `owl:equivalentProperty` at all. Nothing
//! reported the loss, so a run over an ontology that relied on them came back
//! `complete: true` with entailments missing. Measured on ORE 2015
//! `ore_ont_16461`, whose joints are disjoint through one 40-member
//! `DisjointClasses`: `Patient1 ⊑ Systemic_JIA_Patient`, which HermiT finds,
//! was missing. Each test here failed before the change.
//!
//! The last test is the other half: a construct the engine still approximates
//! must make the run say it is not complete.

use std::collections::BTreeSet;
use std::sync::Arc;

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::Reasoner;

fn run(ttl: &str) -> serde_json::Value {
    let store = Arc::new(GraphStore::new());
    let prefixes = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
                    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
                    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
                    @prefix ex: <http://example.org/> .\n";
    store.load_turtle(&format!("{prefixes}{ttl}"), None).unwrap();
    serde_json::from_str(&Reasoner::run(&store, "owl-dl", false).unwrap()).unwrap()
}

fn unsat(v: &serde_json::Value) -> BTreeSet<String> {
    v["unsatisfiable_classes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().trim_start_matches("<http://example.org/").trim_end_matches('>').to_string())
        .collect()
}

fn supers(v: &serde_json::Value, class: &str) -> BTreeSet<String> {
    let want = format!("<http://example.org/{class}>");
    v["classification"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["class"] == want.as_str())
        .flat_map(|r| r["superclasses"].as_array().unwrap().iter())
        .map(|s| s.as_str().unwrap().trim_start_matches("<http://example.org/").trim_end_matches('>').to_string())
        .collect()
}

#[test]
fn n_ary_disjoint_classes_are_pairwise_disjoint() {
    let v = run("ex:A a owl:Class . ex:B a owl:Class . ex:C a owl:Class .\n\
                 [ a owl:AllDisjointClasses ; owl:members ( ex:A ex:B ex:C ) ] .\n\
                 ex:AC a owl:Class ; rdfs:subClassOf ex:A , ex:C .\n");
    assert!(unsat(&v).contains("AC"), "{v}");
    assert_eq!(v["search_finished"], true);
}

#[test]
fn a_disjoint_union_is_a_union_of_disjoint_parts() {
    let v = run("ex:Animal a owl:Class ; owl:disjointUnionOf ( ex:Cat ex:Dog ) .\n\
                 ex:Cat a owl:Class . ex:Dog a owl:Class .\n\
                 ex:CatDog a owl:Class ; rdfs:subClassOf ex:Cat , ex:Dog .\n\
                 ex:Pet a owl:Class ; rdfs:subClassOf ex:Animal ; owl:disjointWith ex:Cat .\n");
    let u = unsat(&v);
    assert!(u.contains("CatDog"), "{v}");
    // An Animal that is not a Cat must be a Dog.
    assert!(supers(&v, "Pet").contains("Dog"), "{v}");
    assert!(supers(&v, "Cat").contains("Animal"), "{v}");
}

#[test]
fn an_equivalent_property_carries_the_others_restrictions() {
    let v = run("ex:p a owl:ObjectProperty ; owl:equivalentProperty ex:q .\n\
                 ex:q a owl:ObjectProperty .\n\
                 ex:Q a owl:Class ; owl:equivalentClass\n\
                   [ a owl:Restriction ; owl:onProperty ex:q ; owl:someValuesFrom owl:Thing ] .\n\
                 ex:P a owl:Class ; rdfs:subClassOf\n\
                   [ a owl:Restriction ; owl:onProperty ex:p ; owl:someValuesFrom owl:Thing ] .\n");
    assert!(supers(&v, "P").contains("Q"), "{v}");
}

#[test]
fn a_datatype_restriction_makes_the_run_say_it_is_not_complete() {
    let v = run("ex:age a owl:DatatypeProperty .\n\
                 ex:Adult a owl:Class ; rdfs:subClassOf\n\
                   [ a owl:Restriction ; owl:onProperty ex:age ; owl:someValuesFrom xsd:integer ] .\n");
    assert_eq!(v["search_finished"], true, "{v}");
    assert_eq!(v["complete"], false, "{v}");
    let listed: Vec<&str> = v["constructs_not_modelled"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["construct"].as_str().unwrap())
        .collect();
    assert!(listed.contains(&"datatype restriction"), "{listed:?}");
}
