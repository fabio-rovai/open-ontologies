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
//! These tests separate the three places the loss could happen (the parser,
//! the store, the SPARQL evaluator) because the fix is in a different place
//! depending on the answer.
//!
//! Since #335 the store keeps every datatype IRI and lexical form as written:
//! the pinned oxigraph value-encodes a literal only when decoding gives it back
//! (oxigraph/oxigraph#2010). That fixed point 2. Point 1 stayed open until #343,
//! because the evaluator, `spareval`, still read the twelve integer-derived
//! types as `xsd:integer` and `xsd:dateTimeStamp` as `xsd:dateTime` before
//! `DATATYPE()` saw them, and `DATATYPE()` is what the SHACL engine tests with.
//! The pin now carries the DATATYPE half of oxigraph/oxigraph#2010 and the STR
//! fix it builds on (oxigraph 1392252f), so `DATATYPE()` reads the literal as
//! written and point 1 is fixed too.

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
  ex:date               "2026-01-01"^^xsd:date ;
  ex:dateTimeStamp      "2026-01-01T00:00:00Z"^^xsd:dateTimeStamp .
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

/// The datatypes `DATATYPE()` reports wrongly over `store`, and how many
/// literals it was asked about.
fn datatypes_after_insert(store: &Store) -> (Vec<String>, usize) {
    let mut wrong = Vec::new();
    let mut checked = 0;
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
            checked += 1;
            let want = expected_from_predicate(&p.to_string().replace(['<', '>'], ""));
            let got = dt
                .to_string()
                .replace(['<', '>'], "")
                .trim_start_matches(XSD)
                .to_string();
            if got != want {
                wrong.push(format!("{want} reported as {got}"));
            }
        }
    }
    wrong.sort();
    (wrong, checked)
}

/// The datatypes `DATATYPE()` misreported on the 0.5.9 evaluator: the twelve
/// collapsed to `xsd:integer` and `dateTimeStamp` to `xsd:dateTime`. The SHACL
/// engine refused every constraint naming one of them until #343. They are
/// named here so that the tests below can show each is now reported, and
/// cannot pass by `DOC` losing one of them.
const FORMERLY_COLLAPSED: [&str; 13] = [
    "byte", "short", "int", "long",
    "unsignedByte", "unsignedShort", "unsignedInt", "unsignedLong",
    "positiveInteger", "negativeInteger", "nonPositiveInteger", "nonNegativeInteger",
    "dateTimeStamp",
];

/// Every literal in `DOC` and every formerly collapsed datatype among them, so
/// that an empty list of wrong answers is not empty for want of rows.
fn assert_doc_covers_the_formerly_collapsed_set(rows_checked: usize) {
    assert_eq!(rows_checked, 18, "DOC has 18 literals and each must be checked");
    for local in FORMERLY_COLLAPSED {
        assert!(
            DOC.contains(&format!("\"^^xsd:{local} ")),
            "DOC no longer carries xsd:{local}"
        );
    }
}

/// #335: the store gives every literal back with the datatype IRI and the
/// lexical form it was given. Read as TERMS, through the store API and through
/// a projected SPARQL variable, neither of which passes the literal through an
/// expression. Under Oxigraph 0.5.9 the thirteen above came back retyped, and
/// this test failed on exactly them.
#[test]
fn the_store_keeps_every_declared_datatype() {
    let store = Store::new().unwrap();
    let parser = RdfParser::from_format(RdfFormat::Turtle);
    let mut parsed = Vec::new();
    for quad in parser.for_reader(Cursor::new(DOC.as_bytes())) {
        let quad = quad.unwrap();
        parsed.push(quad.object.to_string());
        store.insert(quad.as_ref()).unwrap();
    }
    let mut back: Vec<String> = store.iter().map(|q| q.unwrap().object.to_string()).collect();
    parsed.sort();
    back.sort();
    assert_eq!(back, parsed, "the store respelled or retyped a literal");

    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(DOC, None).unwrap();
    let json = graph.sparql_select("SELECT ?v WHERE { ?s ?p ?v }").unwrap();
    let rows: serde_json::Value = serde_json::from_str(&json).unwrap();
    let mut projected: Vec<String> = rows["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["v"].as_str().unwrap().to_string())
        .collect();
    projected.sort();
    assert_eq!(projected, parsed, "a projected variable respelled or retyped a literal");
}

/// #343: `DATATYPE()` over stored literals reports every declared datatype.
/// On the 0.5.9 evaluator this listed exactly `FORMERLY_COLLAPSED`.
#[test]
fn datatype_over_stored_literals_reports_every_declared_datatype() {
    let store = Store::new().unwrap();
    let parser = RdfParser::from_format(RdfFormat::Turtle);
    for quad in parser.for_reader(Cursor::new(DOC.as_bytes())) {
        store.insert(quad.unwrap().as_ref()).unwrap();
    }
    let (wrong, checked) = datatypes_after_insert(&store);
    assert_doc_covers_the_formerly_collapsed_set(checked);
    assert!(wrong.is_empty(), "DATATYPE() misreports {} datatype(s): {wrong:?}", wrong.len());
}

/// The same, through the path the tools actually take, so a regression is
/// attributed to the right layer rather than to whichever test runs first.
#[test]
fn datatype_after_graph_store_load_reports_every_declared_datatype() {
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(DOC, None).unwrap();
    let json = graph
        .sparql_select("SELECT ?p (DATATYPE(?v) AS ?dt) WHERE { ?s ?p ?v }")
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let rows = parsed["results"].as_array().cloned().unwrap_or_default();
    let mut wrong = Vec::new();
    for row in &rows {
        let p = row["p"].as_str().unwrap_or_default().replace(['<', '>'], "");
        let dt = row["dt"].as_str().unwrap_or_default().replace(['<', '>'], "");
        let want = expected_from_predicate(&p);
        if dt.trim_start_matches(XSD) != want {
            wrong.push(format!("{want} reported as {dt}"));
        }
    }
    assert_doc_covers_the_formerly_collapsed_set(rows.len());
    assert!(wrong.is_empty(), "DATATYPE() through load_turtle misreports: {wrong:?}");
}

/// The evaluator keeps the datatype independently of storage.
///
/// A literal typed in the query text is never stored, so this isolates
/// `spareval` from `oxigraph`'s encoder. On the 0.5.9 evaluator `derived` was
/// `xsd:integer`, and that is why the SHACL guard outlived the #335 storage
/// fix. `xsd:token` is the control: a derived type the evaluator never
/// collapsed.
#[test]
fn the_evaluator_keeps_the_datatype_without_any_storage() {
    let graph = Arc::new(GraphStore::new());
    let json = graph
        .sparql_select(
            r#"SELECT
                 (DATATYPE("0"^^<http://www.w3.org/2001/XMLSchema#nonNegativeInteger>) AS ?derived)
                 (DATATYPE("2026-01-01T00:00:00Z"^^<http://www.w3.org/2001/XMLSchema#dateTimeStamp>) AS ?stamp)
                 (DATATYPE("abc"^^<http://www.w3.org/2001/XMLSchema#token>) AS ?control)
               WHERE { }"#,
        )
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let row = &parsed["results"][0];
    let datatype = |var: &str| row[var].as_str().unwrap_or_default().replace(['<', '>'], "");
    assert_eq!(datatype("derived"), format!("{XSD}nonNegativeInteger"), "{json}");
    assert_eq!(datatype("stamp"), format!("{XSD}dateTimeStamp"), "{json}");
    assert_eq!(datatype("control"), format!("{XSD}token"), "{json}");
}

/// A COMPUTED argument still takes the value path, by design: `?v + 0` is a
/// new number, and its datatype is the one arithmetic gives it. This is the
/// boundary of the fix, pinned so that nobody reads the tests above as "the
/// evaluator never retypes".
#[test]
fn a_computed_argument_is_still_typed_by_its_value() {
    let graph = Arc::new(GraphStore::new());
    let json = graph
        .sparql_select(
            r#"SELECT (DATATYPE("5"^^<http://www.w3.org/2001/XMLSchema#int> + 0) AS ?sum) WHERE { }"#,
        )
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        parsed["results"][0]["sum"].as_str().unwrap_or_default().replace(['<', '>'], ""),
        format!("{XSD}integer"),
        "{json}"
    );
}
