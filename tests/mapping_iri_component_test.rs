//! Mapping IDs, generated predicates and lookup values must form valid IRIs.
use open_ontologies::graph::GraphStore;
use open_ontologies::mapping::{FieldMapping, MappingConfig};
use std::collections::HashMap;
const BASE: &str = "https://example.org/";

fn config() -> MappingConfig {
    MappingConfig {
        base_iri: BASE.into(),
        id_field: "id".into(),
        class: format!("{BASE}Record"),
        mappings: vec![],
    }
}

#[test]
fn quoted_identifiers_and_control_bytes_are_sanitized_without_touching_literal_values() {
    for (id, expected) in [
        ("\"row\"", "_row_"),
        ("tab\tid", "tab_id"),
        ("line\nbreak", "line_break"),
        ("return\rid", "return_id"),
        ("nul\0id", "nul_id"),
        ("delete\u{7f}id", "delete_id"),
    ] {
        let mapping = config();
        let row = HashMap::from([("id".into(), id.into())]);
        let triples = mapping.rows_to_ntriples(&[row]);
        assert!(
            triples.starts_with(&format!("<{BASE}{expected}>")),
            "{triples}"
        );
        assert_eq!(GraphStore::new().load_ntriples(&triples).unwrap(), 1);
    }
    let mapping = MappingConfig::from_headers(
        &["id".into(), "name".into()],
        BASE,
        &format!("{BASE}Record"),
    );
    let row = HashMap::from([
        ("id".into(), "\"row\"".into()),
        ("name".into(), "Name\nwith\ttabs".into()),
    ]);
    let triples = mapping.rows_to_ntriples(&[row]);
    assert_eq!(GraphStore::new().load_ntriples(&triples).unwrap(), 3);
    assert!(triples.contains(r#""\"row\""^^"#));
    assert!(triples.contains(r#"Name\nwith\ttabs"#));
}

#[test]
fn headers_and_lookups_use_the_same_valid_iri_component_rule() {
    let header = "title \"quoted\"\n";
    let mapping = MappingConfig::from_headers(
        &["id".into(), header.into()],
        BASE,
        &format!("{BASE}Record"),
    );
    let row = HashMap::from([
        ("id".into(), "one".into()),
        (header.into(), "A title".into()),
    ]);
    let triples = mapping.rows_to_ntriples(&[row]);
    assert!(triples.contains(&format!("<{BASE}ont#title__quoted__>")));
    assert_eq!(GraphStore::new().load_ntriples(&triples).unwrap(), 3);
    let mut mapping = config();
    mapping.mappings.push(FieldMapping {
        field: "related".into(),
        predicate: format!("{BASE}related"),
        datatype: None,
        class: None,
        lookup: true,
    });
    let row = HashMap::from([
        ("id".into(), "one".into()),
        ("related".into(), "\"other\"\t".into()),
    ]);
    let triples = mapping.rows_to_ntriples(&[row]);
    assert!(triples.contains(&format!("<{BASE}_other__>")));
    assert_eq!(GraphStore::new().load_ntriples(&triples).unwrap(), 2);
}

#[test]
fn safe_unicode_and_existing_underscore_substitutions_are_unchanged() {
    for (id, expected) in [
        ("Café", "Café"),
        ("a%20b", "a%20b"),
        ("safe_id", "safe_id"),
        ("hello world", "hello_world"),
        ("a<b>c", "a_b_c"),
        ("code?", "code_"),
    ] {
        let triples = config().rows_to_ntriples(&[HashMap::from([("id".into(), id.into())])]);
        assert!(
            triples.starts_with(&format!("<{BASE}{expected}>")),
            "{triples}"
        );
        assert_eq!(GraphStore::new().load_ntriples(&triples).unwrap(), 1);
    }
}

#[test]
fn actual_cli_ingests_valid_csv_with_a_quoted_identifier() {
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("input.csv");
    std::fs::write(&input, "id,name\n\"\"\"row\"\"\",Example\n").unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
        .arg("--no-connect")
        .arg("--data-dir")
        .arg(tmp.path().join("state"))
        .arg("ingest")
        .arg(input)
        .arg("--base-iri")
        .arg(BASE)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["rows"], 1);
    assert_eq!(result["triples_loaded"], 3);
}
