//! **A clash that does not depend on a choice is not retried under its other
//! alternatives.**
//!
//! The tableau used chronological backtracking: when a branch clashed it
//! undid the most recent choice and tried the next alternative, whatever
//! caused the clash. On the Pizza ontology a wrong guess on the pizza itself
//! (a defined class's reverse half, `VegetarianPizza` tried first on
//! `AmericanHot`) only clashes once a topping is expanded, so every
//! combination of the toppings' own disjuncts was retried before the guess
//! that caused it. The three pizzas with the most toppings never finished.
//!
//! The fixture isolates that shape. `ex:P ⊑ G1 ⊔ ∃s.Fine`, and the atom `G1`
//! sorts before the existential, so the root tries the wrong guess `G1`
//! first. `G1` forces `∀r.¬W` on every `r`-successor, and one successor,
//! `Bad ⊑ W1 ⊔ W2` with both under `W`, clashes under either choice. The
//! other `K` successors each choose between two classes that are BOTH
//! consistent. When those choices are made before the bad successor's,
//! chronological backtracking explores all `2^K` of them before it returns to
//! `G1`; backjumping sees that the bad successor's clash depends on `G1` and
//! on nothing else, and returns there at once. `ex:P` is satisfiable through
//! the second disjunct either way, so the difference is only whether the
//! answer arrives inside the budget. The budget is in STEPS, which are
//! deterministic.

use std::sync::Arc;

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::Reasoner;
use open_ontologies::runtime;

const K: usize = 14;

fn fixture() -> String {
    let mut ttl = String::from(
        r#"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ex: <http://example.org/> .
ex:r a owl:ObjectProperty .
ex:s a owl:ObjectProperty .
ex:Fine a owl:Class .
ex:W a owl:Class .
ex:W1 a owl:Class ; rdfs:subClassOf ex:W .
ex:W2 a owl:Class ; rdfs:subClassOf ex:W .
ex:Bad a owl:Class ; rdfs:subClassOf [ owl:unionOf ( ex:W1 ex:W2 ) ] .
ex:G1 a owl:Class ; rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:r ;
    owl:allValuesFrom [ owl:complementOf ex:W ] ] .
"#,
    );
    let mut somes = String::new();
    for i in 0..K {
        ttl.push_str(&format!(
            "ex:C{i} a owl:Class ; rdfs:subClassOf [ owl:unionOf ( ex:X{i} ex:Y{i} ) ] .\n\
             ex:X{i} a owl:Class . ex:Y{i} a owl:Class .\n"
        ));
        somes.push_str(&format!(
            " , [ a owl:Restriction ; owl:onProperty ex:r ; owl:someValuesFrom ex:C{i} ]"
        ));
    }
    ttl.push_str(&format!(
        "ex:P a owl:Class ; rdfs:subClassOf \
         [ owl:unionOf ( ex:G1 [ a owl:Restriction ; owl:onProperty ex:s ; owl:someValuesFrom ex:Fine ] ) ] , \
         [ a owl:Restriction ; owl:onProperty ex:r ; owl:someValuesFrom ex:Bad ]{somes} .\n"
    ));
    ttl
}

#[test]
fn an_unrelated_clash_is_found_without_trying_every_harmless_choice() {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(&fixture(), None).unwrap();

    let previous = runtime::tableaux_max_steps().unwrap_or(0) as usize;
    // Ample for a search linear in K, far short of 2^14 combinations.
    runtime::set_tableaux_max_steps(20_000);
    let out = Reasoner::run(&store, "owl-dl", false).unwrap();
    runtime::set_tableaux_max_steps(previous);

    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let undetermined: Vec<&str> = v["undetermined_classes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect();
    assert!(
        !undetermined.contains(&"<http://example.org/P>"),
        "ex:P must be decided inside the step budget; budget_exhausted_in={}",
        v["budget_exhausted_in"]
    );
    let unsat = v["unsatisfiable_classes"].as_array().unwrap();
    assert!(unsat.is_empty(), "every class here is satisfiable: {unsat:?}");
    let p_supers: Vec<&str> = v["classification"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["class"] == "<http://example.org/P>")
        .flat_map(|row| row["superclasses"].as_array().unwrap().iter().map(|s| s.as_str().unwrap()))
        .collect();
    assert!(
        !p_supers.contains(&"<http://example.org/G1>"),
        "P ⊑ G1 does not hold; the second disjunct is a model without it: {p_supers:?}"
    );
}
