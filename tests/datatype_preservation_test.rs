//! Derived XSD datatypes must survive a load.
//!
//! RDF 1.1 makes the datatype IRI part of a literal's identity: `"0"^^xsd:integer`
//! and `"0"^^xsd:nonNegativeInteger` are different terms. A store that returns the
//! first when given the second has changed the data, and two things break at once.
//!
//!   1. Every `sh:datatype` constraint naming a derived integer type reports a
//!      violation against data that satisfies it. Nine such false violations in
//!      `jsonld-escaping-conformance` were the first sign, found by the
//!      differential run against pyshacl.
//!   2. Anything written back out carries the widened type, so a published file
//!      no longer says what its author said.
//!
//! These tests separate the two places the loss could happen — the parser or the
//! store — because the fix is in a different codebase depending on the answer.

use open_ontologies::graph::GraphStore;
use oxigraph::io::{RdfFormat, RdfParser};
use oxigraph::model::Term;
use oxigraph::sparql::{QueryResults, SparqlEvaluator};
use oxigraph::store::Store;
use std::io::Cursor;
use std::sync::Arc;

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

/// One literal per datatype the SHACL corpus actually uses.
const DOC: &str = r#"
@prefix ex: <http://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
ex:s
  ex:nonNegativeInteger "0"^^xsd:nonNegativeInteger ;
  ex:positiveInteger    "1"^^xsd:positiveInteger ;
  ex:negativeInteger    "-1"^^xsd:negativeInteger ;
  ex:nonPositiveInteger "0"^^xsd:nonPositiveInteger ;
  ex:long               "5"^^xsd:long ;
  ex:int                "5"^^xsd:int ;
  ex:short              "5"^^xsd:short ;
  ex:byte               "5"^^xsd:byte ;
  ex:unsignedLong       "5"^^xsd:unsignedLong ;
  ex:unsignedInt        "5"^^xsd:unsignedInt ;
  ex:unsignedShort      "5"^^xsd:unsignedShort ;
  ex:unsignedByte       "5"^^xsd:unsignedByte ;
  ex:integer            "5"^^xsd:integer ;
  ex:decimal            "5.5"^^xsd:decimal ;
  ex:token              "abc"^^xsd:token ;
  ex:anyURI             "http://x.example/"^^xsd:anyURI ;
  ex:date               "2026-01-01"^^xsd:date .
"#;

/// The predicate local name is the datatype local name, so a mismatch names itself.
fn expected_from_predicate(p: &str) -> String {
    p.rsplit('/').next().unwrap_or_default().to_string()
}

#[test]
fn the_parser_preserves_every_declared_datatype() {
    let parser = RdfParser::from_format(RdfFormat::Turtle);
    let mut wrong = Vec::new();
    for quad in parser.for_reader(Cursor::new(DOC.as_bytes())) {
        let quad = quad.expect("the document parses");
        let want = expected_from_predicate(quad.predicate.as_str());
        if let Term::Literal(lit) = &quad.object {
            let got = lit.datatype().as_str().trim_start_matches(XSD).to_string();
            if got != want {
                wrong.push(format!("{want} parsed as {got}"));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "the Turtle parser changed {} datatype(s): {wrong:?}",
        wrong.len()
    );
}

fn datatypes_after_insert(store: &Store) -> Vec<String> {
    let mut wrong = Vec::new();
    let results = SparqlEvaluator::new()
        .parse_query("SELECT ?p (DATATYPE(?v) AS ?dt) WHERE { ?s ?p ?v }")
        .unwrap()
        .on_store(store)
        .execute()
        .unwrap();
    if let QueryResults::Solutions(solutions) = results {
        for solution in solutions {
            let solution = solution.unwrap();
            let (Some(p), Some(dt)) = (solution.get("p"), solution.get("dt")) else {
                continue;
            };
            let want = expected_from_predicate(&p.to_string().replace(['<', '>'], ""));
            let got = dt
                .to_string()
                .replace(['<', '>'], "")
                .trim_start_matches(XSD)
                .to_string();
            if got != want {
                wrong.push(format!("{want} stored as {got}"));
            }
        }
    }
    wrong.sort();
    wrong
}

/// The exact set of datatypes oxigraph 0.5 does not preserve.
///
/// This is a characterisation test, not an aspiration: it pins current upstream
/// behaviour so the guard in `shacl.rs` can be trusted to cover exactly the right
/// IRIs and no others. It fails in both directions on purpose. If oxigraph starts
/// preserving these, it fails and the guard should be removed. If it starts losing
/// something new, it fails and the guard is missing a case.
const COLLAPSES_TO_INTEGER: [&str; 12] = [
    "byte", "short", "int", "long",
    "unsignedByte", "unsignedShort", "unsignedInt", "unsignedLong",
    "positiveInteger", "negativeInteger", "nonPositiveInteger", "nonNegativeInteger",
];

#[test]
fn the_store_loses_exactly_the_known_datatypes_and_no_others() {
    let store = Store::new().unwrap();
    let parser = RdfParser::from_format(RdfFormat::Turtle);
    for quad in parser.for_reader(Cursor::new(DOC.as_bytes())) {
        store.insert(quad.unwrap().as_ref()).unwrap();
    }
    let mut lost: Vec<String> = datatypes_after_insert(&store)
        .iter()
        .filter_map(|line| line.split(" stored as ").next().map(str::to_string))
        .collect();
    lost.sort();
    let mut want: Vec<String> = COLLAPSES_TO_INTEGER.iter().map(|s| s.to_string()).collect();
    want.sort();
    assert_eq!(
        lost, want,
        "the set of datatypes oxigraph drops has changed. If it shrank, remove the \
         matching arm from datatype_is_indistinguishable_in_store. If it grew, add one."
    );
}

/// The same set, through the path the tools actually take, so a regression is
/// attributed to the right layer rather than to whichever test runs first.
#[test]
fn graph_store_load_loses_exactly_the_same_set() {
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(DOC, None).unwrap();
    let json = graph
        .sparql_select("SELECT ?p (DATATYPE(?v) AS ?dt) WHERE { ?s ?p ?v }")
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let mut lost = Vec::new();
    for row in parsed["results"].as_array().cloned().unwrap_or_default() {
        let p = row["p"].as_str().unwrap_or_default().replace(['<', '>'], "");
        let dt = row["dt"].as_str().unwrap_or_default().replace(['<', '>'], "");
        let want = expected_from_predicate(&p);
        if dt.trim_start_matches(XSD) != want {
            lost.push(want);
        }
    }
    lost.sort();
    let mut want: Vec<String> = COLLAPSES_TO_INTEGER.iter().map(|s| s.to_string()).collect();
    want.sort();
    assert_eq!(lost, want, "the loss through load_turtle no longer matches the raw store");
}

/// The evaluator collapses the same set, independently of storage.
///
/// A literal typed in the query text is never stored, so this isolates
/// `spareval` from `oxigraph`'s encoder. It is why the SHACL guard cannot be
/// lifted by fixing storage alone: `DATATYPE()` would still answer wrongly.
///
/// `xsd:token` is checked alongside as a control. It is a derived type too, and
/// it survives, so the behaviour is specific to this set rather than a general
/// canonicalisation policy.
#[test]
fn the_evaluator_collapses_the_same_set_without_any_storage() {
    let graph = Arc::new(GraphStore::new());
    let json = graph
        .sparql_select(
            r#"SELECT
                 (DATATYPE("0"^^<http://www.w3.org/2001/XMLSchema#nonNegativeInteger>) AS ?derived)
                 (DATATYPE("abc"^^<http://www.w3.org/2001/XMLSchema#token>) AS ?control)
               WHERE { }"#,
        )
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let row = &parsed["results"][0];
    assert_eq!(
        row["derived"].as_str().unwrap_or_default().replace(['<', '>'], ""),
        format!("{XSD}integer"),
        "if this now answers nonNegativeInteger the evaluator is fixed; \
         check the storage layer too before lifting the SHACL guard"
    );
    assert_eq!(
        row["control"].as_str().unwrap_or_default().replace(['<', '>'], ""),
        format!("{XSD}token"),
        "the control must keep its datatype; if it does not, the defect is wider \
         than the recorded set"
    );
}
