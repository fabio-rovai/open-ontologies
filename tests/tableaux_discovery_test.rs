//! Entity discovery must not depend on explicit typing declarations.
//!
//! Three defects shared one root cause and were found from the outside, by probing the
//! shipped binary with deliberately broken ontologies rather than by reading the code:
//!
//!   1. classes were discovered only through `a owl:Class`, so a class declared solely by
//!      rdfs:subClassOf / owl:equivalentClass / owl:disjointWith never entered the
//!      satisfiability sweep and an unsatisfiable class was reported satisfiable by
//!      omission;
//!   2. individuals were discovered only through `a owl:NamedIndividual`, which instance
//!      data in the wild almost never carries, so the ABox check received no individuals
//!      and a textbook inconsistency - one individual typed into two disjoint classes -
//!      sailed through;
//!   3. even when the ABox check DID prove an inconsistency, the headline `consistent`
//!      flag reported the TBox alone, publishing `true` next to the proof of `false`.
//!
//! The probes that exposed all three are preserved here as regressions.

use open_ontologies::graph::GraphStore;
use std::sync::Arc;

fn reasoned(ttl: &str) -> serde_json::Value {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(ttl, None).unwrap();
    let out = open_ontologies::tableaux::DlReasoner::run(&store, false).unwrap();
    serde_json::from_str(&out).unwrap()
}

fn unsat_classes(v: &serde_json::Value) -> Vec<String> {
    v["unsatisfiable_classes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn a_class_declared_only_by_subclassof_is_satisfiability_checked() {
    // Broken is never typed owl:Class; it exists only through its subclass axioms. It is
    // subsumed under two classes whose disjointness is inherited through a chain, so it is
    // unsatisfiable - and previously invisible.
    let v = reasoned(
        r#"
        @prefix : <http://ex.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        :Term a owl:Class .
        :Soc a owl:Class ; rdfs:subClassOf :Term .
        :Unresolved a owl:Class ; owl:disjointWith :Term .
        :Broken rdfs:subClassOf :Unresolved , :Soc .
        "#,
    );
    assert!(
        unsat_classes(&v).iter().any(|c| c.contains("Broken")),
        "untyped class missing from the sweep: {v}"
    );
}

#[test]
fn a_class_declared_only_by_disjointwith_is_discovered() {
    // Neither side of the disjointness is typed owl:Class. Both must still be named
    // classes, or the axiom checks nothing.
    let v = reasoned(
        r#"
        @prefix : <http://ex.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        :A owl:disjointWith :B .
        :C rdfs:subClassOf :A , :B .
        "#,
    );
    assert!(
        unsat_classes(&v).iter().any(|c| c.contains("C")),
        "disjointness between undeclared classes checked nothing: {v}"
    );
}

#[test]
fn an_instance_typed_by_a_plain_class_reaches_the_abox_check() {
    // x carries no owl:NamedIndividual typing, which is how instance data actually
    // arrives. It is typed into two disjoint classes: the textbook inconsistency.
    let v = reasoned(
        r#"
        @prefix : <http://ex.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        :A a owl:Class .
        :B a owl:Class ; owl:disjointWith :A .
        :x a :A , :B .
        "#,
    );
    assert!(
        v["abox"]["individuals_checked"].as_u64().unwrap() >= 1,
        "plain-typed individual never reached the ABox check: {v}"
    );
    assert_eq!(v["consistent"], false, "proven ABox clash not reported: {v}");
}

#[test]
fn a_proven_abox_clash_flips_the_headline_flag() {
    // The explicitly-typed variant: the ABox check already proved this inconsistent, and
    // the headline flag used to publish `true` next to that proof.
    let v = reasoned(
        r#"
        @prefix : <http://ex.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        :A a owl:Class .
        :B a owl:Class ; owl:disjointWith :A .
        :x a owl:NamedIndividual , :A , :B .
        "#,
    );
    assert_eq!(v["consistent"], false);
    assert_eq!(v["tbox_consistent"], true, "the TBox itself is fine: {v}");
    assert_eq!(v["abox"]["consistent"], false);
}

#[test]
fn schema_declarations_never_double_as_individuals() {
    // A pure schema: classes, a property, an ontology header. None of these subjects may
    // be swept into the ABox as individuals by the widened discovery.
    let v = reasoned(
        r#"
        @prefix : <http://ex.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        <http://ex.org/onto> a owl:Ontology .
        :A a owl:Class .
        :B a owl:Class ; rdfs:subClassOf :A .
        :p a owl:ObjectProperty .
        "#,
    );
    assert_eq!(v["abox"], serde_json::Value::Null, "schema swept into the ABox: {v}");
    assert_eq!(v["consistent"], true);
}

#[test]
fn consistent_instance_data_stays_consistent() {
    let v = reasoned(
        r#"
        @prefix : <http://ex.org/> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        :A a owl:Class .
        :B a owl:Class ; owl:disjointWith :A .
        :x a :A .
        :y a :B .
        "#,
    );
    assert_eq!(v["consistent"], true);
    assert_eq!(v["abox"]["individuals_checked"].as_u64().unwrap(), 2);
}
