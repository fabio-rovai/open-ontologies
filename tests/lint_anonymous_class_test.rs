//! Anonymous class expressions are not terms anyone can label, document or give a domain.
//!
//! `owl:Restriction`, `owl:unionOf` and `owl:intersectionOf` operands are all typed
//! `owl:Class`, so a lint query that selects every subject of `rdf:type owl:Class` reports
//! each one as missing a label and missing a comment. The counts that come out are not
//! wrong by a rounding error, they are wrong by however many class expressions the ontology
//! happens to use, and a well-modelled ontology uses more of them than a poor one. The rule
//! therefore penalised exactly the modelling it should have rewarded.
//!
//! This is the same defect we corrected publicly in our own audit of IATA ONE Record
//! (IATA-Cargo/ONE-Record#435), where 13 blank nodes inflated a published class count.
//! Found here by running this engine against an ontology of our own that defines its
//! classes properly.
//!
//! Pinned:
//!   1. blank-node class expressions are not reported as missing labels or comments;
//!   2. named classes that genuinely lack them still are;
//!   3. a class defined with `owl:equivalentClass` is not reported as an orphan.

use open_ontologies::ontology::OntologyService;

const WITH_ANONYMOUS_CLASSES: &str = r#"
@prefix : <http://ex.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

:Documented a owl:Class ; rdfs:label "Documented"@en ; rdfs:comment "Has both."@en .
:Bare a owl:Class .
:p a owl:ObjectProperty ; rdfs:domain :Documented ; rdfs:range :Documented .

# three anonymous class expressions, all typed owl:Class by OWL
:Documented rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :p ; owl:someValuesFrom :Documented ] .
:Documented rdfs:subClassOf [ a owl:Class ; owl:unionOf ( :Documented :Bare ) ] .
:Documented rdfs:subClassOf [ a owl:Class ; owl:intersectionOf ( :Documented :Bare ) ] .
"#;

fn issues(json: &str) -> Vec<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(json).unwrap()["issues"]
        .as_array()
        .unwrap()
        .clone()
}

#[test]
fn anonymous_class_expressions_are_not_linted() {
    let out = OntologyService::lint(WITH_ANONYMOUS_CLASSES).unwrap();
    let issues = issues(&out);

    let blank: Vec<_> = issues
        .iter()
        .filter(|i| i["entity"].as_str().unwrap_or("").starts_with("_:"))
        .collect();
    assert!(
        blank.is_empty(),
        "blank-node class expressions must not be linted, got {blank:?}"
    );
}

#[test]
fn named_classes_missing_metadata_are_still_reported() {
    let out = OntologyService::lint(WITH_ANONYMOUS_CLASSES).unwrap();
    let issues = issues(&out);

    let bare_label = issues.iter().any(|i| {
        i["type"] == "missing_label" && i["entity"].as_str().unwrap_or("").contains("Bare")
    });
    let bare_comment = issues.iter().any(|i| {
        i["type"] == "missing_comment" && i["entity"].as_str().unwrap_or("").contains("Bare")
    });
    assert!(bare_label, "a named class with no label must still be reported");
    assert!(bare_comment, "a named class with no comment must still be reported");

    let documented = issues
        .iter()
        .any(|i| i["entity"].as_str().unwrap_or("").contains("Documented"));
    assert!(!documented, "a fully documented named class must not be reported");
}

#[test]
fn a_class_defined_by_equivalence_is_not_an_orphan() {
    // owl:equivalentClass is how OWL defines a class in terms of others. Treating it as an
    // orphan because it carries no rdfs:subClassOf penalises the correct construction.
    let ttl = r#"
@prefix : <http://ex.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

:Base a owl:Class ; rdfs:label "Base"@en ; rdfs:comment "A base."@en .
:flag a owl:DatatypeProperty ; rdfs:domain :Base ; rdfs:range <http://www.w3.org/2001/XMLSchema#boolean> .
:Defined a owl:Class ; rdfs:label "Defined"@en ; rdfs:comment "Defined by equivalence."@en ;
    owl:equivalentClass [ a owl:Class ; owl:intersectionOf
        ( :Base [ a owl:Restriction ; owl:onProperty :flag ; owl:hasValue true ] ) ] .
"#;
    let dir = tempfile::tempdir().unwrap();
    let db = open_ontologies::state::StateDb::open(&dir.path().join("state.db")).unwrap();
    let store = std::sync::Arc::new(open_ontologies::graph::GraphStore::new());
    store.load_turtle(ttl, None).unwrap();
    let out = open_ontologies::enforce::Enforcer::new(db, store)
        .enforce("generic")
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    let orphans: Vec<_> = parsed["violations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["rule"] == "orphan_class")
        .filter(|v| v["entity"].as_str().unwrap_or("").contains("Defined"))
        .collect();
    assert!(
        orphans.is_empty(),
        "a class defined by owl:equivalentClass is not an orphan, got {orphans:?}"
    );
}
