//! **A label signature already expanded somewhere is not expanded again.**
//!
//! Blocking used to require the blocker to be an ANCESTOR. An EL ontology whose
//! classes each have a few existentials into a long chain of distinct classes
//! (ORE 2015 `ore_ont_11497`, an anatomy ontology whose developmental stages are
//! each `preceded_by` the last) never repeats a label along one path before the
//! chain ends, so every path was expanded separately and the tree grew
//! exponentially with the chain's depth: 21 s, a class left undetermined, and a
//! different answer on each run. Anywhere blocking lets any older node with the
//! same pairwise signature block.
//!
//! The fixture is that shape in miniature: a chain `S0 … S{N}` where every
//! stage has three existentials into the next one. Each stage's subtree is
//! identical below the second level, so with anywhere blocking the expansion is
//! linear in `N` and fits a step budget the ancestor-only search runs out of.

use std::sync::Arc;

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::Reasoner;
use open_ontologies::runtime;

const N: usize = 18;

#[test]
fn a_branching_chain_is_decided_inside_a_linear_step_budget() {
    let mut ttl = String::from(
        "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
         @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
         @prefix ex: <http://example.org/> .\n\
         ex:a a owl:ObjectProperty . ex:b a owl:ObjectProperty . ex:c a owl:ObjectProperty .\n",
    );
    for i in 0..N {
        let next = i + 1;
        ttl.push_str(&format!(
            "ex:S{i} a owl:Class ; rdfs:subClassOf \
             [ a owl:Restriction ; owl:onProperty ex:a ; owl:someValuesFrom ex:S{next} ] , \
             [ a owl:Restriction ; owl:onProperty ex:b ; owl:someValuesFrom ex:S{next} ] , \
             [ a owl:Restriction ; owl:onProperty ex:c ; owl:someValuesFrom ex:S{next} ] .\n"
        ));
    }
    ttl.push_str(&format!("ex:S{N} a owl:Class .\n"));
    let store = Arc::new(GraphStore::new());
    store.load_turtle(&ttl, None).unwrap();

    let previous = runtime::tableaux_max_steps().unwrap_or(0) as usize;
    runtime::set_tableaux_max_steps(50_000);
    let out = Reasoner::run(&store, "owl-dl", false).unwrap();
    runtime::set_tableaux_max_steps(previous);

    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(
        v["complete"], true,
        "3^{N} paths under ancestor blocking; anywhere blocking expands each stage once: \
         undetermined={} budget_exhausted_in={}",
        v["undetermined_classes"], v["budget_exhausted_in"]
    );
}
