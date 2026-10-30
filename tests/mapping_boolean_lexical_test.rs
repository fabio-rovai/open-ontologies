mod common;

use open_ontologies::graph::GraphStore;
use open_ontologies::induce::{Dt, induce};
use open_ontologies::ingest::DataIngester;
use open_ontologies::mapping::{FieldMapping, MappingConfig};

const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const DATA: &str = "id,active,original\norder-1,True,True\norder-2,False,False\norder-3,TRUE,TRUE\norder-4,FALSE,FALSE\n";
const QUERY: &str = "SELECT ?active (?active = true AS ?is_true) (?active = false AS ?is_false) WHERE { ?s <https://example.org/active> ?active }";

fn mapping() -> MappingConfig {
    MappingConfig {
        base_iri: "https://example.org/orders/".into(),
        id_field: "id".into(),
        class: "https://example.org/Order".into(),
        mappings: vec![
            FieldMapping {
                field: "active".into(),
                predicate: "https://example.org/active".into(),
                datatype: Some(XSD_BOOLEAN.into()),
                class: None,
                lookup: false,
            },
            FieldMapping {
                field: "original".into(),
                predicate: "https://example.org/original".into(),
                datatype: Some(XSD_STRING.into()),
                class: None,
                lookup: false,
            },
        ],
    }
}

fn assert_boolean_comparisons(result: &serde_json::Value, expected_rows: usize) {
    let rows = result["results"].as_array().unwrap();
    assert_eq!(rows.len(), expected_rows);
    for row in rows {
        let active = row["active"].as_str().unwrap();
        let truth = row["is_true"].as_str().unwrap();
        let falsity = row["is_false"].as_str().unwrap();
        assert!(
            active == format!("\"true\"^^<{XSD_BOOLEAN}>")
                || active == format!("\"false\"^^<{XSD_BOOLEAN}>")
        );
        assert_ne!(truth, falsity);
        assert_eq!(truth, active);
    }
}

#[test]
fn admitted_boolean_words_load_as_boolean_values_while_strings_keep_their_case() {
    let rows = DataIngester::parse_csv(DATA).unwrap();
    let triples = mapping().rows_to_ntriples(&rows);
    for word in ["True", "False", "TRUE", "FALSE"] {
        assert!(triples.contains(&format!(
            "<https://example.org/original> \"{word}\"^^<{XSD_STRING}>"
        )));
        assert!(!triples.contains(&format!("\"{word}\"^^<{XSD_BOOLEAN}>")));
    }
    let graph = GraphStore::new();
    graph.load_ntriples(&triples).unwrap();
    let query: serde_json::Value =
        serde_json::from_str(&graph.sparql_select(QUERY).unwrap()).unwrap();
    assert_boolean_comparisons(&query, 4);
}

#[test]
fn lowercase_booleans_and_existing_numeric_inference_policy_are_preserved() {
    let rows = DataIngester::parse_csv("id,active\norder-1,true\norder-2,false\n").unwrap();
    let triples = mapping().rows_to_ntriples(&rows);
    assert!(triples.contains(&format!("\"true\"^^<{XSD_BOOLEAN}>")));
    assert!(triples.contains(&format!("\"false\"^^<{XSD_BOOLEAN}>")));
    let rows = DataIngester::parse_csv("id,active\norder-1,1\norder-2,0\n").unwrap();
    let inferred = induce(
        &rows,
        &["id".into(), "active".into()],
        "Orders",
        "https://example.org/orders/",
    );
    assert_eq!(inferred.columns[0].datatype, Dt::Integer);
    let triples = mapping().rows_to_ntriples(&rows);
    assert!(!triples.contains(&format!("^^<{XSD_BOOLEAN}>")));
    assert!(triples.contains("<https://example.org/active> \"1\" ."));
    assert!(triples.contains("<https://example.org/active> \"0\" ."));
}

#[test]
fn cli_explicit_boolean_mapping_keeps_typed_sparql_comparisons() {
    let _gate = common::exec_gate();
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path().join("orders.csv");
    let config = tmp.path().join("mapping.json");
    let script = tmp.path().join("commands.txt");
    std::fs::write(&data, DATA).unwrap();
    std::fs::write(&config, serde_json::to_string(&mapping()).unwrap()).unwrap();
    std::fs::write(
        &script,
        format!(
            "ingest {} --mapping {}\nquery {QUERY}\n",
            data.display(),
            config.display()
        ),
    )
    .unwrap();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
        .args(["--no-connect", "--data-dir"])
        .arg(tmp.path())
        .arg("batch")
        .arg(script)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let output = String::from_utf8(result.stdout).unwrap();
    let results: Vec<serde_json::Value> = output
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(results[0]["result"]["rows"], 4);
    assert_boolean_comparisons(&results[1]["result"], 4);
}

#[test]
fn admitted_padded_json_booleans_compare_while_string_whitespace_is_preserved() {
    let words = [
        " True ",
        " FALSE ",
        " true ",
        " false ",
        "\tTRUE\n",
        "\tFalse\r\n",
    ];
    let input: Vec<_> = words.iter().enumerate().map(|(i, word)| {
        serde_json::json!({"id": format!("order-{i}"), "active": word, "original": word})
    }).collect();
    let rows = DataIngester::parse_json(&serde_json::to_string(&input).unwrap()).unwrap();
    let triples = mapping().rows_to_ntriples(&rows);
    let graph = GraphStore::new();
    graph.load_ntriples(&triples).unwrap();
    let query: serde_json::Value =
        serde_json::from_str(&graph.sparql_select(QUERY).unwrap()).unwrap();
    assert_boolean_comparisons(&query, words.len());
    let originals: serde_json::Value = serde_json::from_str(
        &graph
            .sparql_select("SELECT ?original WHERE { ?s <https://example.org/original> ?original }")
            .unwrap(),
    )
    .unwrap();
    let values: std::collections::HashSet<_> = originals["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["original"].as_str().unwrap())
        .collect();
    assert_eq!(
        values,
        std::collections::HashSet::from([
            "\" True \"",
            "\" FALSE \"",
            "\" true \"",
            "\" false \"",
            "\"\\tTRUE\\n\"",
            "\"\\tFalse\\r\\n\""
        ])
    );
}
