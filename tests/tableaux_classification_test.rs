//! **Classification answers HermiT's question with HermiT's answer.**
//!
//! Classification used to test every ordered pair of classes, and on Pizza
//! (99 classes) that was 19,394 tableau tests run under one 10 s phase budget.
//! It returned no subsumption at all and 143 undetermined classes. It now runs
//! one satisfiability test per class and reads the class's subsumers off that
//! test's model (see `ClassModel` in `src/tableaux.rs`): a named class absent
//! from the model's root label is a proven non-subsumer, and when the model was
//! built without a choice every class present in it is a proven subsumer. Only
//! the classes present in a model that DID make a choice are tested pairwise.
//!
//! The first test is the whole of Pizza against HermiT 1.4.3's classification
//! of the same file, recorded in `tests/fixtures/hermit/pizza_classification.tsv`
//! (500 subsumptions with equivalences, 2 unsatisfiable classes). The others
//! pin the two halves of the model-reading rule on fixtures small enough to
//! read, and the disjointness shapes the absorption of `owl:disjointWith`
//! decides.

use std::collections::BTreeSet;
use std::sync::Arc;

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::Reasoner;

const PIZZA: &str = "https://raw.githubusercontent.com/owlcs/pizza-ontology/refs/heads/master/pizza.owl#";

fn classify_file(path: &str) -> serde_json::Value {
    let store = Arc::new(GraphStore::new());
    store.load_file(path).expect("fixture must load");
    serde_json::from_str(&Reasoner::run(&store, "owl-dl", false).unwrap()).unwrap()
}

fn classify_ttl(ttl: &str) -> serde_json::Value {
    let store = Arc::new(GraphStore::new());
    let prefixes = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
                    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
                    @prefix ex: <http://example.org/> .\n";
    store.load_turtle(&format!("{prefixes}{ttl}"), None).unwrap();
    serde_json::from_str(&Reasoner::run(&store, "owl-dl", false).unwrap()).unwrap()
}

/// Named-class subsumptions in the output, `owl:Thing` dropped, local names.
fn pairs(v: &serde_json::Value, ns: &str) -> BTreeSet<(String, String)> {
    let local = |s: &str| s.trim_start_matches('<').trim_end_matches('>').trim_start_matches(ns).to_string();
    let mut out = BTreeSet::new();
    for row in v["classification"].as_array().unwrap() {
        let c = row["class"].as_str().unwrap();
        for s in row["superclasses"].as_array().unwrap() {
            let s = s.as_str().unwrap();
            if !s.ends_with("#Thing>") && s != c {
                out.insert((local(c), local(s)));
            }
        }
    }
    out
}

#[test]
fn pizza_classifies_exactly_as_hermit_does() {
    let v = classify_file("benchmark/reference/pizza-reference.owl");
    assert_eq!(v["search_finished"], true, "Pizza must be decided: {}", v["budget_exhausted_in"]);
    // Pizza has six `owl:hasValue` and one `owl:oneOf`, which this engine
    // approximates, so the run must not call itself complete even though its
    // answer here is exactly HermiT's.
    assert_eq!(v["complete"], false);

    let fixture = std::fs::read_to_string("tests/fixtures/hermit/pizza_classification.tsv").unwrap();
    let mut want: BTreeSet<(String, String)> = BTreeSet::new();
    let mut want_unsat: BTreeSet<String> = BTreeSet::new();
    for line in fixture.lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
        let (a, b) = line.split_once('\t').unwrap();
        if a == "UNSAT" {
            want_unsat.insert(b.to_string());
        } else {
            want.insert((a.to_string(), b.to_string()));
        }
    }
    assert_eq!(want.len(), 500);

    let got_unsat: BTreeSet<String> = v["unsatisfiable_classes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().trim_start_matches('<').trim_end_matches('>').trim_start_matches(PIZZA).to_string())
        .collect();
    assert_eq!(got_unsat, want_unsat);

    // An unsatisfiable class is under everything; HermiT lists it separately
    // and so does the fixture.
    let got: BTreeSet<(String, String)> =
        pairs(&v, PIZZA).into_iter().filter(|(a, _)| !got_unsat.contains(a)).collect();
    let missed: Vec<_> = want.difference(&got).collect();
    let wrong: Vec<_> = got.difference(&want).collect();
    assert!(wrong.is_empty(), "subsumptions HermiT does not entail: {wrong:?}");
    assert!(missed.is_empty(), "subsumptions HermiT entails that were not found: {missed:?}");
}

/// A model built WITH a choice is one model, not every model. `A ⊑ B ⊔ C`
/// puts `B` (or `C`) in the root label of the model found, and neither is a
/// subsumer of `A`. Reading the label as proof here would report one.
#[test]
fn a_class_chosen_in_one_model_is_not_reported_as_a_subsumer() {
    let v = classify_ttl(
        "ex:A a owl:Class ; rdfs:subClassOf [ owl:unionOf ( ex:B ex:C ) ] .\n\
         ex:B a owl:Class . ex:C a owl:Class .\n",
    );
    let p = pairs(&v, "http://example.org/");
    assert!(!p.contains(&("A".into(), "B".into())), "{p:?}");
    assert!(!p.contains(&("A".into(), "C".into())), "{p:?}");
}

/// A model built WITHOUT a choice is every model, so its root label is the
/// subsumer set and no pair needs a test of its own.
#[test]
fn a_deterministic_model_settles_its_subsumers_without_pair_tests() {
    let v = classify_ttl(
        "ex:r a owl:ObjectProperty .\n\
         ex:A a owl:Class ; rdfs:subClassOf ex:B ,\n\
             [ a owl:Restriction ; owl:onProperty ex:r ; owl:someValuesFrom ex:C ] .\n\
         ex:B a owl:Class . ex:C a owl:Class .\n\
         ex:D a owl:Class ; owl:equivalentClass\n\
             [ a owl:Restriction ; owl:onProperty ex:r ; owl:someValuesFrom ex:C ] .\n",
    );
    let p = pairs(&v, "http://example.org/");
    assert!(p.contains(&("A".into(), "D".into())), "A ⊑ D is entailed: {p:?}");
    let agent = &v["agents"]["subsumption_agent"];
    assert_eq!(agent["pairs_tested"], 0, "{agent}");
    assert!(agent["read_from_models"].as_u64().unwrap() >= 1, "{agent}");
}

/// Disjoint with `owl:Thing` is empty. The disjointness check looked for both
/// labels on one node, and `⊤` is never stored as a label, so this never fired.
/// ORE 2015 `ore_ont_1325` has exactly this axiom and HermiT reports 13
/// unsatisfiable classes from it.
#[test]
fn a_class_disjoint_with_thing_is_unsatisfiable() {
    let v = classify_ttl(
        "ex:V a owl:Class ; owl:disjointWith owl:Thing .\n\
         ex:W a owl:Class ; rdfs:subClassOf ex:V .\n\
         ex:X a owl:Class .\n",
    );
    let unsat: BTreeSet<&str> =
        v["unsatisfiable_classes"].as_array().unwrap().iter().map(|s| s.as_str().unwrap()).collect();
    assert!(unsat.contains("<http://example.org/V>"), "{v}");
    assert!(unsat.contains("<http://example.org/W>"), "{v}");
    assert!(!unsat.contains("<http://example.org/X>"), "{v}");
}
