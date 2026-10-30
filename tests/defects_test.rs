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

// #341. `defect_count` counts query rows, and for `inherited_disjoint` a row is
// one (class, disjoint pair). A class under two disjoint pairs was two defects,
// so the headline count on a large ontology ran higher than the reasoner's list
// of unsatisfiable classes and nothing in the output said why. The listing was
// also capped at 50 a kind with no way to reach row 51.

/// The reproduction from the issue: one class, two disjoint pairs above it.
const DOUBLE: &str = r#"
    @prefix ex:   <https://example.org/x/> .
    @prefix owl:  <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    ex:A1 a owl:Class . ex:B1 a owl:Class . ex:A2 a owl:Class . ex:B2 a owl:Class .
    ex:A1 owl:disjointWith ex:B1 .
    ex:A2 owl:disjointWith ex:B2 .
    ex:X a owl:Class ; rdfs:subClassOf ex:A1, ex:B1, ex:A2, ex:B2 .
"#;

/// `n` classes, each under `A` and `B`, which are disjoint.
fn many_under_a_disjoint_pair(n: usize) -> String {
    let mut ttl = format!("{PREFIXES} ex:A owl:disjointWith ex:B .\n");
    for i in 0..n {
        ttl.push_str(&format!("ex:C{i} rdfs:subClassOf ex:A, ex:B .\n"));
    }
    ttl
}

#[test]
fn a_class_under_two_disjoint_pairs_is_two_findings_and_one_affected_class() {
    let report = check(DOUBLE);
    assert_eq!(
        report["defect_count"].as_u64(),
        Some(2),
        "defect_count counts findings, one per row: {report:#}"
    );
    assert_eq!(
        report["affected_classes"].as_u64(),
        Some(1),
        "both findings are about ex:X, and the reasoner reports ONE unsatisfiable class: \
         {report:#}"
    );
    assert_eq!(
        report["kind_counts"]["inherited_disjoint"],
        serde_json::json!({"findings": 2, "affected_classes": 1}),
        "{report:#}"
    );
}

#[test]
fn affected_properties_are_counted_apart_from_affected_classes() {
    let report = check(&format!(
        "{PREFIXES}
         ex:partOf a owl:TransitiveProperty, owl:FunctionalProperty .
         ex:Plant owl:disjointWith ex:Animal .
         ex:Coral rdfs:subClassOf ex:Plant, ex:Animal ."
    ));
    assert_eq!(report["affected_classes"].as_u64(), Some(1), "{report:#}");
    assert_eq!(report["affected_properties"].as_u64(), Some(1), "{report:#}");
    assert_eq!(
        report["kind_counts"]["transitive_and_functional"],
        serde_json::json!({"findings": 1, "affected_properties": 1}),
        "{report:#}"
    );
}

#[test]
fn every_kind_has_a_count_even_when_it_found_nothing() {
    let report = check(&format!("{PREFIXES} ex:Dog rdfs:subClassOf ex:Animal ."));
    let counts = report["kind_counts"].as_object().expect("kind_counts");
    assert_eq!(counts.len(), 8, "{report:#}");
    assert_eq!(report["affected_classes"].as_u64(), Some(0), "{report:#}");
}

fn check_limited(ttl: &str, limit: usize) -> serde_json::Value {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(ttl, None).unwrap();
    serde_json::from_str(&Defects::check_with_limit(&store, limit).unwrap()).unwrap()
}

#[test]
fn a_limit_of_zero_lists_every_row_and_truncates_nothing() {
    let report = check_limited(&many_under_a_disjoint_pair(80), 0);
    assert_eq!(report["defect_count"].as_u64(), Some(80), "{report:#}");
    assert_eq!(
        report["reported_count"].as_u64(),
        Some(80),
        "rows 51 to 80 must be reachable: {report:#}"
    );
    assert_eq!(report["truncated"], serde_json::json!({}), "{report:#}");
    assert_eq!(report["affected_classes"].as_u64(), Some(80), "{report:#}");
}

#[test]
fn a_limit_below_the_default_truncates_at_the_limit_and_says_so() {
    let report = check_limited(&many_under_a_disjoint_pair(80), 10);
    assert_eq!(report["reported_count"].as_u64(), Some(10), "{report:#}");
    assert_eq!(
        report["truncated"]["inherited_disjoint"].as_u64(),
        Some(80),
        "the full total is still reported: {report:#}"
    );
}

#[test]
fn a_limit_that_covers_every_row_truncates_nothing() {
    // `truncated` names a kind when some of its rows are NOT listed. Keyed on
    // the default instead of the limit in force, this would claim a cut that
    // did not happen.
    let report = check_limited(&many_under_a_disjoint_pair(60), 60);
    assert_eq!(report["reported_count"].as_u64(), Some(60), "{report:#}");
    assert_eq!(report["truncated"], serde_json::json!({}), "{report:#}");
}

#[test]
fn the_default_is_still_fifty_a_kind() {
    let report = check(&many_under_a_disjoint_pair(80));
    assert_eq!(report["reported_count"].as_u64(), Some(50), "{report:#}");
    assert_eq!(report["truncated"]["inherited_disjoint"].as_u64(), Some(80));
}

#[tokio::test]
async fn onto_defects_takes_the_same_limit() {
    use open_ontologies::inputs::OntoDefectsInput;
    use open_ontologies::server::OpenOntologiesServer;
    use open_ontologies::state::StateDb;
    use rmcp::handler::server::wrapper::Parameters;

    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("state.db")).unwrap();
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(&many_under_a_disjoint_pair(80), None).unwrap();
    let server = OpenOntologiesServer::new_with_graph(db, graph);

    let all: serde_json::Value =
        serde_json::from_str(&server.onto_defects(Parameters(OntoDefectsInput { limit: Some(0) })))
            .unwrap();
    assert_eq!(all["reported_count"].as_u64(), Some(80), "{all:#}");

    let default: serde_json::Value =
        serde_json::from_str(&server.onto_defects(Parameters(OntoDefectsInput { limit: None })))
            .unwrap();
    assert_eq!(default["reported_count"].as_u64(), Some(50), "{default:#}");
    assert_eq!(default["affected_classes"].as_u64(), Some(80), "{default:#}");
}
