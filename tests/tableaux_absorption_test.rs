//! **Absorption keeps every entailment and stops the GCI blowup.**
//!
//! Before `Absorber`, an inclusion whose left-hand side was not a single named
//! class became a disjunction on every node of every tableau. The nominals
//! repro in `benchmark/reasoner/regressions/` (four `C ≡ ∃worksFor.{d}`) did
//! not finish in 20 s; ORE 2015 `ore_ont_8937` ran its subsumption phase out of
//! budget. These pin the answers HermiT gives, and the ABox cases pin that
//! moving `∃r.C ⊑ D` to `C ⊑ ∀r⁻.D` still reaches an `r`-predecessor that is an
//! individual rather than a tree parent, which is what the stored inverse edge
//! in `add_role_edge` is for.

use std::collections::BTreeSet;
use std::sync::Arc;

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::Reasoner;

const PREFIXES: &str = r#"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ex: <http://example.org/> .
"#;

fn run(ttl: &str) -> serde_json::Value {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(&format!("{PREFIXES}{ttl}"), None).unwrap();
    let out = Reasoner::run(&store, "owl-dl", false).unwrap();
    serde_json::from_str(&out).unwrap()
}

fn pairs(v: &serde_json::Value) -> BTreeSet<(String, String)> {
    let short = |s: &str| s.trim_start_matches("<http://example.org/").trim_end_matches('>').to_string();
    let mut out = BTreeSet::new();
    for row in v["classification"].as_array().unwrap() {
        for s in row["superclasses"].as_array().unwrap() {
            let s = s.as_str().unwrap();
            if !s.ends_with("#Thing>") {
                out.insert((short(row["class"].as_str().unwrap()), short(s)));
            }
        }
    }
    out
}

/// The regression file itself, with HermiT's eight answers (`FacultyOfN ⊑
/// Faculty`, `DepartmentN ⊑ Department`) and no others.
#[test]
fn the_four_nominal_repro_is_decided_and_matches_hermit() {
    let ttl = std::fs::read_to_string("benchmark/reasoner/regressions/nominals_blowup.ttl").unwrap();
    let store = Arc::new(GraphStore::new());
    store.load_turtle(&ttl, None).unwrap();
    let t = std::time::Instant::now();
    let v: serde_json::Value =
        serde_json::from_str(&Reasoner::run(&store, "owl-dl", false).unwrap()).unwrap();
    assert_eq!(v["search_finished"], true, "{v}");
    // `owl:hasValue` is approximated, so the run must not call itself complete
    // even though, here, its answers are exactly HermiT's.
    assert_eq!(v["complete"], false, "{v}");
    assert!(
        v["constructs_not_modelled"].as_array().unwrap().iter().any(|c| c["construct"] == "owl:hasValue"),
        "{v}"
    );
    assert!(t.elapsed().as_secs() < 10, "took {:?}", t.elapsed());
    let mut got: Vec<(String, String)> = Vec::new();
    for row in v["classification"].as_array().unwrap() {
        for s in row["superclasses"].as_array().unwrap() {
            let s = s.as_str().unwrap();
            if !s.ends_with("#Thing>") {
                got.push((row["class"].as_str().unwrap().to_string(), s.to_string()));
            }
        }
    }
    got.sort();
    let mut want: Vec<(String, String)> = Vec::new();
    for i in 1..=4 {
        want.push((format!("<http://lubm.example.org/Department{i}>"), "<http://lubm.example.org/Department>".into()));
        want.push((format!("<http://lubm.example.org/FacultyOf{i}>"), "<http://lubm.example.org/Faculty>".into()));
    }
    want.sort();
    assert_eq!(got, want);
}

/// `⊤ ⊑ C` makes `C` hold of everything, so every class is under it. This is
/// the shape of the ORE file whose answers were first misread as unsound.
#[test]
fn a_class_above_thing_is_a_superclass_of_every_class() {
    let v = run(r#"
        ex:A a owl:Class . ex:B a owl:Class .
        owl:Thing rdfs:subClassOf ex:Top1 .
        ex:Top1 a owl:Class .
    "#);
    let p = pairs(&v);
    assert!(p.contains(&("A".into(), "Top1".into())), "{p:?}");
    assert!(p.contains(&("B".into(), "Top1".into())), "{p:?}");
}

/// `∃r.⊤ ⊑ C` is a domain: whatever has an `r`-successor is a `C`.
#[test]
fn an_existential_over_thing_on_the_left_acts_as_a_domain() {
    let v = run(r#"
        ex:r a owl:ObjectProperty .
        ex:Employee a owl:Class .
        [ a owl:Restriction ; owl:onProperty ex:r ; owl:someValuesFrom owl:Thing ]
            rdfs:subClassOf ex:Employee .
        ex:Manager a owl:Class ; rdfs:subClassOf
            [ a owl:Restriction ; owl:onProperty ex:r ; owl:someValuesFrom ex:Dept ] .
        ex:Dept a owl:Class .
    "#);
    assert!(pairs(&v).contains(&("Manager".into(), "Employee".into())), "{v}");
}

/// A defined class reached through a conjunction and an existential: the
/// reverse half of the definition is a binary rule plus a `∀r⁻`.
#[test]
fn a_conjunctive_definition_is_recognised_through_its_absorbed_half() {
    let v = run(r#"
        ex:hasTopping a owl:ObjectProperty .
        ex:Pizza a owl:Class . ex:Cheese a owl:Class . ex:Mozzarella a owl:Class ;
            rdfs:subClassOf ex:Cheese .
        ex:CheesyPizza a owl:Class ; owl:equivalentClass [ owl:intersectionOf ( ex:Pizza
            [ a owl:Restriction ; owl:onProperty ex:hasTopping ; owl:someValuesFrom ex:Cheese ] ) ] .
        ex:Margherita a owl:Class ; rdfs:subClassOf ex:Pizza ,
            [ a owl:Restriction ; owl:onProperty ex:hasTopping ; owl:someValuesFrom ex:Mozzarella ] .
        ex:Base a owl:Class ; rdfs:subClassOf
            [ a owl:Restriction ; owl:onProperty ex:hasTopping ; owl:someValuesFrom ex:Mozzarella ] .
    "#);
    let p = pairs(&v);
    assert!(p.contains(&("Margherita".into(), "CheesyPizza".into())), "{p:?}");
    // Not a Pizza, so the binary rule must not fire.
    assert!(!p.contains(&("Base".into(), "CheesyPizza".into())), "{p:?}");
}

/// The ABox half. `Meaty ≡ ∃hasTopping.Meat` absorbs into `Meat ⊑
/// ∀hasTopping⁻.Meaty`, and `hasTopping` has no declared inverse, so the `∀`
/// reaches `a` only through an inverse edge on `b`, which the ABox builder
/// writes only for a role that HAS an inverse. Absorption mints one for every
/// role the TBox names; without that the contradiction (`a` is Vegetarian,
/// disjoint from Meaty) is missed and the ontology is called consistent.
#[test]
fn an_absorbed_existential_still_reaches_an_individual_predecessor() {
    let v = run(r#"
        ex:hasTopping a owl:ObjectProperty .
        ex:Meat a owl:Class . ex:Vegetarian a owl:Class .
        ex:Meaty a owl:Class ; owl:equivalentClass
            [ a owl:Restriction ; owl:onProperty ex:hasTopping ; owl:someValuesFrom ex:Meat ] ;
            owl:disjointWith ex:Vegetarian .
        ex:a a ex:Vegetarian ; ex:hasTopping ex:b .
        ex:b a ex:Meat .
    "#);
    assert_eq!(v["consistent"], false, "{v}");
}

/// Same, through a sub-role, in the tree tableau: the edge is on `hasCheese ⊑
/// hasTopping`, and the `∀hasTopping⁻` reaches the parent only through the
/// inverse edge `add_role_edge` stores. Removing that edge fails this test.
#[test]
fn an_absorbed_existential_reaches_a_predecessor_through_a_sub_role() {
    let v = run(r#"
        ex:hasTopping a owl:ObjectProperty .
        ex:hasCheese a owl:ObjectProperty ; rdfs:subPropertyOf ex:hasTopping .
        ex:Meat a owl:Class . ex:Vegetarian a owl:Class .
        ex:Meaty a owl:Class ; owl:equivalentClass
            [ a owl:Restriction ; owl:onProperty ex:hasTopping ; owl:someValuesFrom ex:Meat ] ;
            owl:disjointWith ex:Vegetarian .
        ex:VegPizza a owl:Class ; rdfs:subClassOf ex:Vegetarian ,
            [ a owl:Restriction ; owl:onProperty ex:hasCheese ; owl:someValuesFrom ex:Meat ] .
    "#);
    let unsat: Vec<&str> = v["unsatisfiable_classes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect();
    assert!(unsat.contains(&"<http://example.org/VegPizza>"), "{v}");
}
