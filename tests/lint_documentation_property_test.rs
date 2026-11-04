// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! `missing_comment` means the class is undocumented, not that it lacks one
//! particular property.
//!
//! The rule read `rdfs:comment` alone. SKOS vocabularies document a class with
//! `skos:definition` and OBO Foundry ontologies (FoodOn among them) with
//! `obo:IAO_0000115`, so lint warned on every class of an ontology that was
//! fully documented in either convention, and the classes that really had no
//! documentation were lost in that noise (#354).

use open_ontologies::ontology::OntologyService;

const REPRO: &str = r#"
@prefix ex:      <http://example.org/demo#> .
@prefix owl:     <http://www.w3.org/2002/07/owl#> .
@prefix rdfs:    <http://www.w3.org/2000/01/rdf-schema#> .
@prefix skos:    <http://www.w3.org/2004/02/skos/core#> .
@prefix obo:     <http://purl.obolibrary.org/obo/> .
@prefix dcterms: <http://purl.org/dc/terms/> .
ex:A a owl:Class ; rdfs:label "A" ; rdfs:comment "Documented with rdfs:comment." .
ex:B a owl:Class ; rdfs:label "B" ; skos:definition "Documented with skos:definition." .
ex:C a owl:Class ; rdfs:label "C" ; obo:IAO_0000115 "Documented with the OBO definition property." .
ex:E a owl:Class ; rdfs:label "E" ; dcterms:description "Documented with dcterms:description." .
ex:D a owl:Class ; rdfs:label "D" .
"#;

fn missing_comment(ttl: &str) -> Vec<String> {
    let out: serde_json::Value =
        serde_json::from_str(&OntologyService::lint(ttl).unwrap()).unwrap();
    let mut v: Vec<String> = out["issues"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["type"] == "missing_comment")
        .map(|i| i["entity"].as_str().unwrap().to_string())
        .collect();
    v.sort();
    v
}

#[test]
fn a_class_documented_by_a_definition_property_is_not_missing_a_comment() {
    assert_eq!(
        missing_comment(REPRO),
        vec!["<http://example.org/demo#D>".to_string()],
        "only ex:D has no documentation of any kind"
    );
}

#[test]
fn the_message_names_every_property_that_would_have_counted() {
    let out: serde_json::Value =
        serde_json::from_str(&OntologyService::lint(REPRO).unwrap()).unwrap();
    let message = out["issues"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["type"] == "missing_comment")
        .and_then(|i| i["message"].as_str())
        .unwrap()
        .to_string();
    for property in ["rdfs:comment", "skos:definition", "IAO_0000115", "dcterms:description"] {
        assert!(message.contains(property), "{message}");
    }
}
