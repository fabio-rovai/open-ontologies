// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

use open_ontologies::ingest::DataIngester;
use std::collections::HashMap;

#[test]
fn test_parse_csv() {
    let csv_content = "id,name,category\nb1,Tower Bridge,Landmark\nb2,Big Ben,Landmark\n";
    let rows = DataIngester::parse_csv(csv_content).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["name"], "Tower Bridge");
    assert_eq!(rows[1]["id"], "b2");
}

#[test]
fn test_parse_csv_rejects_duplicate_headers_instead_of_dropping_a_column() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("duplicate-headers.csv");
    std::fs::write(&path, "id,amount,amount\nrow-1,10,25\n").unwrap();

    let error = DataIngester::parse_file(path.to_str().unwrap()).unwrap_err();
    assert!(
        error.to_string().contains("Duplicate CSV header \"amount\""),
        "{error}"
    );
}

#[test]
fn test_parse_csv_ignores_empty_columns_that_have_no_header() {
    // Spreadsheet exports often end every line with commas for columns that
    // were formatted but never filled. A blank header names nothing, so two
    // of them are not a duplicate, and an empty column under one is skipped.
    let rows = DataIngester::parse_csv("name,,\nAlice,,\nBob,,\n").unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], HashMap::from([("name".to_string(), "Alice".to_string())]));
    assert_eq!(rows[1], HashMap::from([("name".to_string(), "Bob".to_string())]));
}

#[test]
fn test_headers_in_order_leaves_out_columns_that_have_no_header() {
    // The column list feeds mapping generation, which turns every header into
    // a property. A blank header would become a property with an empty name.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("trailing-commas.csv");
    std::fs::write(&path, "name,,age,\nAlice,,30,\n").unwrap();
    let path = path.to_str().unwrap();

    let rows = DataIngester::parse_file(path).unwrap();
    assert_eq!(DataIngester::headers_in_order(path, &rows), vec!["name", "age"]);
}

#[test]
fn test_parse_csv_rejects_a_value_under_a_blank_header() {
    // A value with no column name has nowhere to go: refused by column
    // rather than stored under an empty key or dropped.
    let error = DataIngester::parse_csv("name,\nAlice,x\n").unwrap_err().to_string();
    assert!(error.contains("CSV column 2 has a value"), "{error}");
    assert!(error.contains("no header"), "{error}");
}

#[test]
fn test_parse_json_array() {
    let json = r#"[{"id":"b1","name":"Tower Bridge"},{"id":"b2","name":"Big Ben"}]"#;
    let rows = DataIngester::parse_json(json).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["name"], "Tower Bridge");
}

#[test]
fn parse_json_rejects_non_object_root_values() {
    let error = DataIngester::parse_json("42").unwrap_err();
    assert!(error.to_string().contains("object or an array of objects"));
}

#[test]
fn parse_json_rejects_non_object_array_items_instead_of_dropping_them() {
    let error = DataIngester::parse_json(r#"[{"id":"kept"}, 42]"#).unwrap_err();
    assert!(error.to_string().contains("index 1"));
}

#[test]
fn test_parse_ndjson() {
    let ndjson = "{\"id\":\"b1\",\"name\":\"Tower Bridge\"}\n{\"id\":\"b2\",\"name\":\"Big Ben\"}\n";
    let rows = DataIngester::parse_ndjson(ndjson).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1]["name"], "Big Ben");
}

#[test]
fn test_parse_yaml() {
    let yaml = "- id: b1\n  name: Tower Bridge\n- id: b2\n  name: Big Ben\n";
    let rows = DataIngester::parse_yaml(yaml).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["name"], "Tower Bridge");
}

#[test]
fn test_parse_xml_records() {
    let xml = r#"<records><record><id>b1</id><name>Tower Bridge</name></record><record><id>b2</id><name>Big Ben</name></record></records>"#;
    let rows = DataIngester::parse_xml(xml).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["id"], "b1");
}

#[test]
fn test_detect_format() {
    assert_eq!(DataIngester::detect_format("data.csv"), "csv");
    assert_eq!(DataIngester::detect_format("data.json"), "json");
    assert_eq!(DataIngester::detect_format("data.jsonl"), "ndjson");
    assert_eq!(DataIngester::detect_format("data.ndjson"), "ndjson");
    assert_eq!(DataIngester::detect_format("data.xml"), "xml");
    assert_eq!(DataIngester::detect_format("data.yaml"), "yaml");
    assert_eq!(DataIngester::detect_format("data.yml"), "yaml");
    assert_eq!(DataIngester::detect_format("data.xlsx"), "xlsx");
    assert_eq!(DataIngester::detect_format("data.parquet"), "parquet");
}

#[test]
fn test_extract_headers() {
    let csv_content = "id,name,category\nb1,Tower Bridge,Landmark\n";
    let rows = DataIngester::parse_csv(csv_content).unwrap();
    let headers = DataIngester::extract_headers(&rows);
    assert!(headers.contains(&"id".to_string()));
    assert!(headers.contains(&"name".to_string()));
    assert!(headers.contains(&"category".to_string()));
}
