//! `onto_classify_el` against the tableau and against HermiT (#324).
//!
//! The EL saturation claims `complete: true` on input inside EL. That claim is
//! only worth what an independent engine says about it, so two are used:
//!
//!   - the `owl-dl` tableau, which since #313 classifies Pizza exactly as
//!     HermiT 1.4.3 does, on generated EL TBoxes that use every rule the
//!     saturation has (conjunction and existentials on both sides, domains,
//!     ranges, disjointness, a role hierarchy and a transitive role). Where the
//!     tableau calls itself complete the two tables must be equal, pair for pair.
//!   - HermiT's own classification of Pizza, recorded in
//!     `tests/fixtures/hermit/pizza_classification.tsv`. Pizza is outside EL
//!     (universals, unions, functional roles, nominals), so the saturation must
//!     say it is incomplete, and every pair it reports must be one HermiT
//!     entails.

use std::collections::BTreeSet;
use std::sync::Arc;

use open_ontologies::classify_el::classify;
use open_ontologies::graph::GraphStore;
use open_ontologies::reason::Reasoner;

const EX: &str = "http://example.org/";
const PIZZA: &str = "https://raw.githubusercontent.com/owlcs/pizza-ontology/refs/heads/master/pizza.owl#";

fn local(s: &str, ns: &str) -> String {
    s.trim_start_matches('<').trim_end_matches('>').trim_start_matches(ns).to_string()
}

/// The tableau's table: (pairs between satisfiable named classes, unsatisfiable classes, complete).
fn tableau(store: &Arc<GraphStore>, ns: &str) -> (BTreeSet<(String, String)>, BTreeSet<String>, bool) {
    let v: serde_json::Value = serde_json::from_str(&Reasoner::run(store, "owl-dl", false).unwrap()).unwrap();
    let unsat: BTreeSet<String> = v["unsatisfiable_classes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| local(s.as_str().unwrap(), ns))
        .collect();
    let mut pairs = BTreeSet::new();
    for row in v["classification"].as_array().unwrap() {
        let c = local(row["class"].as_str().unwrap(), ns);
        if unsat.contains(&c) {
            continue;
        }
        for s in row["superclasses"].as_array().unwrap() {
            let s = s.as_str().unwrap();
            if !s.ends_with("#Thing>") && local(s, ns) != c {
                pairs.insert((c.clone(), local(s, ns)));
            }
        }
    }
    (pairs, unsat, v["complete"] == true)
}

/// A small deterministic generator: no crate, no seed file, the same TBoxes on
/// every machine.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self, n: u64) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.0 >> 33) % n
    }
}

fn some(r: &str, filler: &str) -> String {
    format!("[ a owl:Restriction ; owl:onProperty ex:{r} ; owl:someValuesFrom {filler} ]")
}

fn generated_el_tbox(seed: u64) -> String {
    let mut g = Lcg(seed);
    let c = |g: &mut Lcg| format!("ex:C{}", g.next(7));
    let roles = ["r", "s"];
    let mut ttl = String::from("ex:r a owl:ObjectProperty . ex:s a owl:ObjectProperty .\n");
    for i in 0..7 {
        ttl.push_str(&format!("ex:C{i} a owl:Class .\n"));
    }
    if g.next(2) == 0 {
        ttl.push_str("ex:s rdfs:subPropertyOf ex:r .\n");
    }
    if g.next(2) == 0 {
        ttl.push_str("ex:r a owl:TransitiveProperty .\n");
    }
    for _ in 0..(6 + g.next(5)) {
        let role = roles[g.next(2) as usize];
        let (a, b, d) = (c(&mut g), c(&mut g), c(&mut g));
        let axiom = match g.next(9) {
            0 | 1 => format!("{a} rdfs:subClassOf {b} ."),
            2 => format!("{a} rdfs:subClassOf {} .", some(role, &b)),
            3 => format!("{} rdfs:subClassOf {b} .", some(role, &a)),
            4 => format!("[ owl:intersectionOf ( {a} {b} ) ] rdfs:subClassOf {d} ."),
            5 => format!("{a} owl:equivalentClass [ owl:intersectionOf ( {b} {} ) ] .", some(role, &d)),
            6 => format!("ex:{role} rdfs:domain {a} ."),
            7 => format!("ex:{role} rdfs:range {a} ."),
            _ => {
                if g.next(3) == 0 {
                    format!("{a} owl:disjointWith {b} .")
                } else {
                    format!("{a} rdfs:subClassOf {} .", some(role, &format!("[ owl:intersectionOf ( {b} {d} ) ]")))
                }
            }
        };
        ttl.push_str(&axiom);
        ttl.push('\n');
    }
    ttl
}

#[test]
fn el_saturation_equals_the_tableau_on_generated_el_tboxes() {
    let prefixes = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
                    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
                    @prefix ex: <http://example.org/> .\n";
    let mut compared = 0;
    let mut pairs_compared = 0;
    let mut disagreements: Vec<String> = Vec::new();
    for seed in 1..=120u64 {
        let ttl = generated_el_tbox(seed);
        let store = Arc::new(GraphStore::new());
        store.load_turtle(&format!("{prefixes}{ttl}"), None).unwrap();
        let el = classify(&store).unwrap();
        assert!(el.complete, "seed {seed} is EL by construction: {:?}\n{ttl}", el.incompleteness);

        let (want, want_unsat, tableau_complete) = tableau(&store, EX);
        if !tableau_complete {
            continue;
        }
        compared += 1;
        let got: BTreeSet<(String, String)> =
            el.subsumptions.iter().map(|(a, b)| (local(a, EX), local(b, EX))).collect();
        let got_unsat: BTreeSet<String> = el.unsatisfiable.iter().map(|s| local(s, EX)).collect();
        pairs_compared += want.len();
        if got != want || got_unsat != want_unsat {
            disagreements.push(format!(
                "seed {seed}: missed {:?}, extra {:?}, unsat el {:?} vs tableau {:?}\n{ttl}",
                want.difference(&got).collect::<Vec<_>>(),
                got.difference(&want).collect::<Vec<_>>(),
                got_unsat,
                want_unsat
            ));
        }
    }
    // Every generated TBox must reach the oracle, or this test proves nothing.
    assert_eq!(compared, 120, "the tableau must decide every generated TBox");
    assert!(pairs_compared > 200, "the corpus is too thin to say anything: {pairs_compared} pairs");
    assert!(disagreements.is_empty(), "{} disagreement(s):\n{}", disagreements.len(), disagreements.join("\n"));
}

#[test]
fn pizza_is_marked_incomplete_and_every_reported_pair_is_one_hermit_entails() {
    let store = Arc::new(GraphStore::new());
    store.load_file("benchmark/reference/pizza-reference.owl").unwrap();
    let el = classify(&store).unwrap();
    assert!(!el.complete);
    assert!(!el.incompleteness.is_empty());

    let fixture = std::fs::read_to_string("tests/fixtures/hermit/pizza_classification.tsv").unwrap();
    let mut hermit: BTreeSet<(String, String)> = BTreeSet::new();
    let mut hermit_unsat: BTreeSet<String> = BTreeSet::new();
    for line in fixture.lines().filter(|l| !l.starts_with('#') && !l.is_empty()) {
        let (a, b) = line.split_once('\t').unwrap();
        if a == "UNSAT" {
            hermit_unsat.insert(b.to_string());
        } else {
            hermit.insert((a.to_string(), b.to_string()));
        }
    }
    let got: BTreeSet<(String, String)> =
        el.subsumptions.iter().map(|(a, b)| (local(a, PIZZA), local(b, PIZZA))).collect();
    let got_unsat: BTreeSet<String> = el.unsatisfiable.iter().map(|s| local(s, PIZZA)).collect();
    // A pair under a class HermiT finds unsatisfiable is entailed too.
    let wrong: Vec<_> = got.iter().filter(|(a, _)| !hermit_unsat.contains(a)).filter(|p| !hermit.contains(*p)).collect();
    assert!(wrong.is_empty(), "pairs HermiT does not entail: {wrong:?}");
    assert!(got_unsat.is_subset(&hermit_unsat), "{got_unsat:?} vs {hermit_unsat:?}");
    eprintln!(
        "pizza: {} of HermiT's {} pairs, {} of {} unsatisfiable classes",
        got.intersection(&hermit).count(),
        hermit.len(),
        got_unsat.len(),
        hermit_unsat.len()
    );
}
