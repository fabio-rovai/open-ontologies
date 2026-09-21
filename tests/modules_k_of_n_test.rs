//! Modules that reason alone, and facts that earn "true everywhere".
//!
//! The premise is not assumed, it is DEMONSTRATED by
//! `the_merged_graph_is_inconsistent_where_the_modules_are_not`. Birds fly.
//! Penguins are birds and do not fly. In one graph that is a contradiction and
//! the engine finds it. In two modules it is two consistent contexts that
//! disagree, each still ordinary monotonic OWL 2 RL, each still carrying a
//! certificate `oo-cert` accepts.
//!
//! That is the whole argument for this module, so a test has to make it rather
//! than a docstring asserting it.

use open_ontologies::modules::{distributed, promoted_ntriples, Module};

const P: &str = "@prefix : <http://ex.org/> .\n\
                 @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
                 @prefix owl:  <http://www.w3.org/2002/07/owl#> .\n";

/// Birds fly, and tweety is a bird.
const BIRDS: &str = ":Bird rdfs:subClassOf :Flier .\n:tweety a :Bird .\n";

/// Penguins are birds and are not fliers, and pingu is one. On its own this is
/// perfectly consistent: nothing in HERE says birds fly.
const PENGUINS: &str =
    ":Penguin rdfs:subClassOf :Bird .\n:Penguin owl:disjointWith :Flier .\n:pingu a :Penguin .\n";

/// A third context that happens to agree with the first about birds.
const FIELD_GUIDE: &str = ":Bird rdfs:subClassOf :Flier .\n:robin a :Bird .\n";

fn m(name: &str, body: &str) -> Module {
    Module { name: name.to_string(), ttl: format!("{P}{body}") }
}

fn three() -> Vec<Module> {
    vec![m("birds", BIRDS), m("penguins", PENGUINS), m("field_guide", FIELD_GUIDE)]
}

#[test]
fn the_merged_graph_is_inconsistent_where_the_modules_are_not() {
    use std::sync::Arc;
    use open_ontologies::graph::GraphStore;
    use open_ontologies::reason::{InferenceTarget, Reasoner};

    // One graph holding both contexts.
    let merged = Arc::new(GraphStore::new());
    merged.load_turtle(&format!("{P}{BIRDS}{PENGUINS}"), None).expect("parses");
    let out =
        Reasoner::run_full(&merged, "owl-rl", false, InferenceTarget::DefaultGraph, None).unwrap();
    let r: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(
        r["inconsistency"]["found"], true,
        "the premise of this whole module is that one graph cannot hold both contexts. If \
         this is ever false the argument is gone: {r}"
    );

    // The same axioms, kept apart, are two consistent contexts.
    let js = distributed(&three(), 2).expect("runs");
    assert_eq!(js["modules"].as_array().unwrap().len(), 3);
    // Nothing above blew up, which is the point: each module reasoned alone
    // and neither had to be made consistent with the other.
    assert!(js["promoted"].as_u64().unwrap() > 0, "{js}");
}

#[test]
fn no_module_inherits_another_modules_axioms() {
    let js = distributed(&three(), 2).expect("runs");
    let all = js.to_string();
    // `pingu a :Flier` follows ONLY if the penguin module is handed the bird
    // module's rule. It must appear nowhere.
    assert!(
        !all.contains("pingu> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://ex.org/Flier>"),
        "a module inherited another's axioms, which is exactly what this design refuses"
    );
}

#[test]
fn a_fact_two_of_three_entail_is_promoted_at_two_and_not_at_three() {
    let want = "<http://ex.org/Bird> <http://www.w3.org/2000/01/rdf-schema#subClassOf> \
                <http://ex.org/Flier> .";
    let at2 = distributed(&three(), 2).expect("runs");
    let promoted2 = promoted_ntriples(&at2);
    assert!(promoted2.contains(want), "two modules entail it, so k=2 promotes it");

    let at3 = distributed(&three(), 3).expect("runs");
    let promoted3 = promoted_ntriples(&at3);
    assert!(
        !promoted3.contains(want),
        "the penguin module does not entail it, so k=3 must NOT promote it. A threshold that \
         promoted regardless would make the count decorative"
    );
}

#[test]
fn a_fact_only_one_module_entails_is_contested_and_named() {
    let js = distributed(&three(), 2).expect("runs");
    let contested = js["contested_facts"].as_array().cloned().unwrap_or_default();
    let pingu = contested.iter().find(|r| {
        r["triple"].as_str().unwrap_or_default().contains("/pingu")
    });
    let row = pingu.expect("pingu's own facts are entailed in one module only");
    assert_eq!(row["agreed"], 1);
    assert_eq!(
        row["modules"].as_array().unwrap()[0], "penguins",
        "a contested fact must name WHO holds it, or the disagreement is not actionable"
    );
}

#[test]
fn a_threshold_that_cannot_be_met_is_refused_rather_than_answered() {
    assert!(distributed(&three(), 0).is_err(), "k=0 promotes everything and means nothing");
    assert!(distributed(&three(), 4).is_err(), "k above n can never be met");
    assert!(distributed(&[], 1).is_err(), "no modules, nothing to count");
}

#[test]
fn the_report_says_agreement_is_a_count_and_not_a_proof() {
    let js = distributed(&three(), 2).expect("runs");
    let means = js["means"].as_str().unwrap_or_default();
    assert!(means.contains("COUNT and not a proof"), "{means}");
    assert!(
        means.contains("copied the same mistaken axiom"),
        "the failure mode of counting agreement has to be stated, not implied: {means}"
    );
    assert_eq!(js["verdict"], "agreed_by_k_of_n_modules");
    for forbidden in ["certificate_sound", "machine-checked", "proved"] {
        assert!(!js.to_string().contains(forbidden), "claims {forbidden}, which nothing proved");
    }
}
