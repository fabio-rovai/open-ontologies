//! Studio's Graph view queries, run against the engine (#440).
//!
//! `studio/src/components/GraphView.tsx` sends the queries in
//! `studio/src/lib/graph-queries.json` to `POST /api/query`, which answers with
//! `GraphStore::sparql_select`. Reading OWL-typed terms only, the view drew
//! nothing for a vocabulary written in plain RDFS: `rdfs:Class` was no class
//! and `rdf:Property` no property.

use open_ontologies::graph::GraphStore;
use std::collections::BTreeSet;
use std::path::Path;

const EX: &str = "http://example.org/";

/// The issue's fixture, plus a property typed both ways, which must keep its OWL kind.
const RDFS_VOCABULARY: &str = r#"
@prefix ex: <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:Vehicle a rdfs:Class .
ex:Car a rdfs:Class ; rdfs:subClassOf ex:Vehicle .
ex:Person a rdfs:Class .
ex:drives a rdf:Property ; rdfs:domain ex:Person ; rdfs:range ex:Car .
ex:alice a ex:Person ; ex:drives ex:car1 .
ex:car1 a ex:Car .

ex:name a rdf:Property , owl:DatatypeProperty ; rdfs:domain ex:Person ; rdfs:range xsd:string .
ex:loose a rdf:Property .
"#;

fn queries() -> serde_json::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("studio/src/lib/graph-queries.json");
    serde_json::from_str(&std::fs::read_to_string(&path).expect("graph-queries.json")).unwrap()
}

/// Run one Graph view query the way the studio builds it, and return its rows.
fn run(store: &GraphStore, name: &str, term: Option<&str>) -> Vec<serde_json::Value> {
    let q = queries();
    let mut body = q[name].as_str().expect(name).to_string();
    if let Some(t) = term {
        body = body.replace("$TERM", t);
    }
    let text = store
        .sparql_select(&format!("{}{}", q["prefixes"].as_str().unwrap(), body))
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    let out: serde_json::Value = serde_json::from_str(&text).unwrap();
    out["results"]
        .as_array()
        .unwrap_or_else(|| panic!("{name}: {text}"))
        .clone()
}

fn column(rows: &[serde_json::Value], var: &str) -> BTreeSet<String> {
    rows.iter()
        .filter_map(|r| r[var].as_str().map(str::to_string))
        .collect()
}

fn ex(local: &str) -> String {
    format!("<{EX}{local}>")
}

fn loaded() -> GraphStore {
    let store = GraphStore::new();
    store.load_turtle(RDFS_VOCABULARY, None).unwrap();
    store
}

#[test]
fn rdfs_classes_and_their_subclass_edges_are_read() {
    let store = loaded();
    let classes = column(&run(&store, "classes", None), "c");
    assert_eq!(
        classes,
        [ex("Car"), ex("Person"), ex("Vehicle")]
            .into_iter()
            .collect()
    );
    let sub = run(&store, "sub", None);
    let pairs: BTreeSet<(String, String)> = sub
        .iter()
        .map(|r| {
            (
                r["s"].as_str().unwrap().into(),
                r["o"].as_str().unwrap().into(),
            )
        })
        .collect();
    assert_eq!(pairs, [(ex("Car"), ex("Vehicle"))].into_iter().collect());
}

#[test]
fn an_rdf_property_with_an_iri_domain_and_range_is_a_property() {
    let store = loaded();
    let rows = run(&store, "props", None);
    let kinds: BTreeSet<(String, String)> = rows
        .iter()
        .map(|r| {
            (
                r["p"].as_str().unwrap().into(),
                r["t"].as_str().unwrap().into(),
            )
        })
        .collect();
    assert_eq!(
        kinds,
        [
            (
                ex("drives"),
                "<http://www.w3.org/1999/02/22-rdf-syntax-ns#Property>".to_string()
            ),
            // Typed both ways: the OWL type alone, so the view keeps it a data property.
            (
                ex("name"),
                "<http://www.w3.org/2002/07/owl#DatatypeProperty>".to_string()
            ),
        ]
        .into_iter()
        .collect(),
        "an rdf:Property with no domain or range (ex:loose) is left out"
    );
    let dr = run(&store, "domRange", None);
    let drives: BTreeSet<(String, String)> = dr
        .iter()
        .filter(|r| r["p"].as_str() == Some(ex("drives").as_str()))
        .map(|r| {
            (
                r["k"].as_str().unwrap().into(),
                r["c"].as_str().unwrap().into(),
            )
        })
        .collect();
    assert_eq!(
        drives,
        [
            ("\"d\"".to_string(), ex("Person")),
            ("\"r\"".to_string(), ex("Car")),
        ]
        .into_iter()
        .collect()
    );
}

#[test]
fn individuals_of_rdfs_classes_are_found_by_the_instance_search() {
    let store = loaded();
    assert_eq!(
        column(&run(&store, "instanceSearch", Some("car")), "n"),
        [ex("car1")].into_iter().collect()
    );
    assert_eq!(
        column(&run(&store, "instanceSearch", Some("alice")), "n"),
        [ex("alice")].into_iter().collect()
    );
}
