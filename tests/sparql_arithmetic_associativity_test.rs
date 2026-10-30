//! SPARQL arithmetic associates to the left (#349).
//!
//! The SPARQL 1.1 grammar makes `AdditiveExpression` and
//! `MultiplicativeExpression` left-associative, so `8-3-2` is `(8-3)-2 = 3` and
//! `7/20*1000` is `(7/20)*1000 = 350`. Oxigraph 0.5.9 parsed both chains to the
//! right and answered `7` and `0.00035`, silently, for every query, SHACL-SPARQL
//! constraint, monitor watcher and rule that computes a ratio, a percentage or
//! a difference of more than two operands without parentheses. Upstream fixed
//! the parser in oxigraph/oxigraph#1974, released in 0.5.12; the pin moved onto
//! 0.5.13 to take it.
//!
//! The three expressions are checked twice: through `GraphStore::sparql_select`,
//! which every tool that runs a written query goes through, and through the
//! `open-ontologies query --no-connect` command line the issue reports.

use open_ontologies::graph::GraphStore;
use std::process::Command;

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

/// Each case is a projected variable, the expression, the right answer and its
/// datatype. The comment gives what a right-associating parser computes, so a
/// regression names itself.
const CASES: [(&str, &str, f64, &str); 3] = [
    // 8-(3-2) = 7
    ("a", "8-3-2", 3.0, "integer"),
    // 7/(20*1000) = 0.00035
    ("b", "7/20*1000", 350.0, "decimal"),
    // A percentage of a ratio, the shape a monitor threshold takes:
    // 6/(20*(0.8/(1.0*100))) = 37.5
    ("c", "6/20*0.8/1.0*100", 24.0, "decimal"),
];

fn query() -> String {
    let projection: Vec<String> = CASES
        .iter()
        .map(|(var, expr, _, _)| format!("({expr} AS ?{var})"))
        .collect();
    format!("SELECT {} WHERE {{}}", projection.join(" "))
}

/// Split `"350"^^<...#decimal>` into its number and its datatype local name.
fn number_and_datatype(term: &str) -> (f64, String) {
    let (lexical, datatype) = term
        .strip_prefix('"')
        .and_then(|rest| rest.split_once("\"^^<"))
        .unwrap_or_else(|| panic!("{term} is not a typed literal"));
    let datatype = datatype
        .trim_end_matches('>')
        .strip_prefix(XSD)
        .unwrap_or_else(|| panic!("{term} is not an XSD literal"));
    let number = lexical
        .parse::<f64>()
        .unwrap_or_else(|e| panic!("{lexical} is not a number: {e}"));
    (number, datatype.to_string())
}

fn assert_left_associative(json: &str, path: &str) {
    let parsed: serde_json::Value =
        serde_json::from_str(json).unwrap_or_else(|e| panic!("{path} gave no JSON ({e}): {json}"));
    let row = &parsed["results"][0];
    let mut wrong = Vec::new();
    for (var, expr, want, want_datatype) in CASES {
        let Some(term) = row[var].as_str() else {
            wrong.push(format!("{expr} gave no answer"));
            continue;
        };
        let (got, datatype) = number_and_datatype(term);
        if (got - want).abs() > 1e-9 || datatype != want_datatype {
            wrong.push(format!(
                "{expr} = {got}^^xsd:{datatype}, expected {want}^^xsd:{want_datatype}"
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "{path} does not associate arithmetic to the left: {wrong:?}\n{json}"
    );
}

#[test]
fn arithmetic_chains_associate_to_the_left_in_sparql_select() {
    let store = GraphStore::new();
    let json = store.sparql_select(&query()).expect("the query runs");
    assert_left_associative(&json, "GraphStore::sparql_select");
}

#[test]
fn arithmetic_chains_associate_to_the_left_on_the_command_line() {
    let dir = tempfile::tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
        .arg("--data-dir")
        .arg(dir.path())
        .args(["query", "--no-connect"])
        .arg(query())
        .output()
        .expect("the binary runs");
    assert!(
        out.status.success(),
        "query exited {}: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    assert_left_associative(
        &String::from_utf8_lossy(&out.stdout),
        "open-ontologies query --no-connect",
    );
}
