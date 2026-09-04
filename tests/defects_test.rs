//! The ontology is checked before the data is.
//!
//! A self-contradicting ontology makes every fact-level conclusion suspect, so
//! the declarations are examined first, on their own, with no data present.
//!
//! This is a different question from satisfiability, which `onto_dl_check`
//! already answers. A property declared both transitive and functional is
//! perfectly satisfiable; it is still a trap, because the two declarations
//! together manufacture contradictions the moment instances arrive. The
//! tableaux reasoner is right to call it consistent and it is still worth
//! reporting.

use open_ontologies::defects::Defects;
use open_ontologies::graph::GraphStore;
use std::sync::Arc;

fn check(ttl: &str) -> serde_json::Value {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(ttl, None).unwrap();
    let json = Defects::check(&store).unwrap();
    serde_json::from_str(&json).unwrap()
}

fn kinds(report: &serde_json::Value) -> Vec<String> {
    report["defects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["kind"].as_str().unwrap().to_string())
        .collect()
}

const PREFIXES: &str = r#"
    @prefix ex:   <http://example.org/> .
    @prefix owl:  <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
"#;

#[test]
fn a_property_both_transitive_and_functional_is_reported() {
    let report = check(&format!(
        "{PREFIXES} ex:partOf a owl:TransitiveProperty, owl:FunctionalProperty ."
    ));
    assert!(
        kinds(&report).contains(&"transitive_and_functional".to_string()),
        "{report:#}"
    );
}

#[test]
fn a_property_both_symmetric_and_asymmetric_is_reported() {
    let report = check(&format!(
        "{PREFIXES} ex:sibling a owl:SymmetricProperty, owl:AsymmetricProperty ."
    ));
    assert!(
        kinds(&report).contains(&"symmetric_and_asymmetric".to_string()),
        "{report:#}"
    );
}

#[test]
fn a_subclass_cycle_is_reported() {
    let report = check(&format!(
        "{PREFIXES}
         ex:A rdfs:subClassOf ex:B .
         ex:B rdfs:subClassOf ex:C .
         ex:C rdfs:subClassOf ex:A ."
    ));
    assert!(kinds(&report).contains(&"subclass_cycle".to_string()), "{report:#}");
}

#[test]
fn a_sub_property_cycle_is_reported() {
    let report = check(&format!(
        "{PREFIXES}
         ex:p rdfs:subPropertyOf ex:q .
         ex:q rdfs:subPropertyOf ex:p ."
    ));
    assert!(
        kinds(&report).contains(&"sub_property_cycle".to_string()),
        "{report:#}"
    );
}

#[test]
fn a_class_disjoint_with_its_own_ancestor_is_reported() {
    let report = check(&format!(
        "{PREFIXES}
         ex:Puppy rdfs:subClassOf ex:Dog .
         ex:Dog   rdfs:subClassOf ex:Animal .
         ex:Puppy owl:disjointWith ex:Animal ."
    ));
    assert!(
        kinds(&report).contains(&"disjoint_with_ancestor".to_string()),
        "a class that is disjoint from something it is also a kind of can have \
         no instances: {report:#}"
    );
}

#[test]
fn a_class_under_two_disjoint_parents_is_reported() {
    let report = check(&format!(
        "{PREFIXES}
         ex:Plant  owl:disjointWith ex:Animal .
         ex:Coral  rdfs:subClassOf ex:Plant, ex:Animal ."
    ));
    assert!(
        kinds(&report).contains(&"inherited_disjoint".to_string()),
        "{report:#}"
    );
}

#[test]
fn an_inverse_pair_that_is_not_mutual_is_reported() {
    let report = check(&format!(
        "{PREFIXES} ex:parentOf owl:inverseOf ex:childOf ."
    ));
    assert!(
        kinds(&report).contains(&"inverse_not_mutual".to_string()),
        "one direction was declared and the other was not: {report:#}"
    );
}

#[test]
fn a_mutual_inverse_pair_is_not_reported() {
    let report = check(&format!(
        "{PREFIXES}
         ex:parentOf owl:inverseOf ex:childOf .
         ex:childOf  owl:inverseOf ex:parentOf ."
    ));
    assert!(
        !kinds(&report).contains(&"inverse_not_mutual".to_string()),
        "{report:#}"
    );
}

#[test]
fn a_property_declared_its_own_inverse_without_being_symmetric_is_reported() {
    let report = check(&format!("{PREFIXES} ex:marriedTo owl:inverseOf ex:marriedTo ."));
    assert!(kinds(&report).contains(&"self_inverse".to_string()), "{report:#}");
}

#[test]
fn a_clean_ontology_reports_nothing() {
    let report = check(&format!(
        "{PREFIXES}
         ex:Animal a owl:Class .
         ex:Dog rdfs:subClassOf ex:Animal .
         ex:Cat rdfs:subClassOf ex:Animal .
         ex:Dog owl:disjointWith ex:Cat .
         ex:parentOf a owl:ObjectProperty ; owl:inverseOf ex:childOf .
         ex:childOf  a owl:ObjectProperty ; owl:inverseOf ex:parentOf ."
    ));
    assert_eq!(report["defect_count"].as_u64(), Some(0), "{report:#}");
}

#[test]
fn the_report_names_every_kind_it_looked_for() {
    // A check that finds nothing and a check that never ran look identical
    // unless the report says what was examined.
    let report = check(&format!("{PREFIXES} ex:Animal a owl:Class ."));
    let checked: Vec<String> = report["kinds_checked"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k.as_str().unwrap().to_string())
        .collect();
    assert_eq!(checked.len(), 8, "eight kinds are implemented: {checked:?}");
    assert!(checked.contains(&"transitive_and_functional".to_string()));
    assert!(checked.contains(&"disjoint_with_ancestor".to_string()));
}

#[test]
fn a_kind_that_fires_in_bulk_is_capped_and_says_so() {
    // Anything reported item by item needs an upper bound. A shared vocabulary
    // with one bad declaration can produce thousands of identical rows, and a
    // reader facing a thousand identical cards decides nothing from them.
    let mut ttl = String::from(PREFIXES);
    for i in 0..(Defects::MAX_PER_KIND + 20) {
        ttl.push_str(&format!("ex:p{i} a owl:TransitiveProperty, owl:FunctionalProperty .\n"));
    }
    let report = check(&ttl);

    let reported = kinds(&report)
        .iter()
        .filter(|k| *k == "transitive_and_functional")
        .count();
    assert_eq!(
        reported,
        Defects::MAX_PER_KIND,
        "the listing must stop at the cap: {report:#}"
    );
    assert_eq!(
        report["truncated"]["transitive_and_functional"].as_u64(),
        Some((Defects::MAX_PER_KIND + 20) as u64),
        "the total must still be reported, or the cap hides the scale of the problem"
    );
}

// A sweep over the 153 readable ontologies in this repo returned 27 findings,
// 25 of them `inverse_not_mutual`. That kind is real and it is also harmless:
// `p owl:inverseOf q` entails the reverse, so nothing is broken, a reader is
// merely not told. Reporting it beside `disjoint_with_ancestor`, which makes a
// class uninstantiable, buries the finding that matters under the finding that
// does not. The kinds need to be ranked.

fn severity_of(report: &serde_json::Value, kind: &str) -> String {
    report["defects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["kind"] == kind)
        .unwrap_or_else(|| panic!("{kind} not in {report:#}"))["severity"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn a_class_that_can_have_no_instances_is_an_error() {
    let report = check(&format!(
        "{PREFIXES}
         ex:Puppy rdfs:subClassOf ex:Dog .
         ex:Dog   rdfs:subClassOf ex:Animal .
         ex:Puppy owl:disjointWith ex:Animal ."
    ));
    assert_eq!(severity_of(&report, "disjoint_with_ancestor"), "error");
}

#[test]
fn a_pair_that_will_manufacture_contradictions_is_a_warning() {
    let report = check(&format!(
        "{PREFIXES} ex:partOf a owl:TransitiveProperty, owl:FunctionalProperty ."
    ));
    assert_eq!(severity_of(&report, "transitive_and_functional"), "warning");
}

#[test]
fn a_declaration_that_costs_nothing_but_clarity_is_info() {
    let report = check(&format!("{PREFIXES} ex:parentOf owl:inverseOf ex:childOf ."));
    assert_eq!(
        severity_of(&report, "inverse_not_mutual"),
        "info",
        "the entailment holds either way, so this is hygiene and must not \
         outrank a class that can have no instances"
    );
}

#[test]
fn the_report_totals_each_severity_so_the_serious_ones_are_not_buried() {
    let report = check(&format!(
        "{PREFIXES}
         ex:Plant owl:disjointWith ex:Animal .
         ex:Coral rdfs:subClassOf ex:Plant, ex:Animal .
         ex:parentOf owl:inverseOf ex:childOf .
         ex:ownerOf  owl:inverseOf ex:ownedBy ."
    ));
    assert_eq!(report["severity_counts"]["error"].as_u64(), Some(1), "{report:#}");
    assert_eq!(report["severity_counts"]["info"].as_u64(), Some(2), "{report:#}");
}
