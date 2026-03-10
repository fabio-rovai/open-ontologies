use open_ontologies::ingest::DataIngester;

#[test]
fn test_parse_csv() {
    let csv_content = "id,name,category\nb1,Tower Bridge,Landmark\nb2,Big Ben,Landmark\n";
    let rows = DataIngester::parse_csv(csv_content).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["name"], "Tower Bridge");
    assert_eq!(rows[1]["id"], "b2");
}

#[test]
fn test_parse_json_array() {
    let json = r#"[{"id":"b1","name":"Tower Bridge"},{"id":"b2","name":"Big Ben"}]"#;
    let rows = DataIngester::parse_json(json).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["name"], "Tower Bridge");
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
