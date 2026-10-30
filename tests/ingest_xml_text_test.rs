use open_ontologies::ingest::DataIngester;

#[test]
fn cdata_and_fragmented_text_preserve_the_same_field_value() {
    for name in [
        "Tower Bridge",
        "<![CDATA[Tower Bridge]]>",
        "Tower<![CDATA[ Bridge]]>",
        "Tower<!-- provenance --> Bridge",
        "<![CDATA[Tower]]><!-- provenance --><![CDATA[ Bridge]]>",
        "  Tower<!-- provenance --> Bridge  ",
    ] {
        let xml = format!("<records><record><id>b1</id><name>{name}</name></record></records>");
        let rows = DataIngester::parse_xml(&xml).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["id"], "b1");
        assert_eq!(rows[0]["name"], "Tower Bridge", "source: {name}");
    }
}

#[test]
fn cdata_is_literal_and_text_entities_are_decoded() {
    let rows = DataIngester::parse_xml(
        "<records><record><name>A &amp; B<![CDATA[ <C> & D]]></name></record></records>",
    )
    .unwrap();
    assert_eq!(rows[0]["name"], "A & B <C> & D");
}

#[test]
fn an_undefined_entity_is_an_error_not_a_successful_empty_value() {
    let error = DataIngester::parse_xml(
        "<records><record><id>b1</id><name>Tower &unknown;</name></record></records>",
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Failed to decode XML field text")
    );
}

#[test]
fn fields_records_and_duplicate_field_overwrite_stay_separate() {
    let rows = DataIngester::parse_xml(
        "<records><record><id>b1</id><name>old</name><name>new</name></record>\
         <record><id>b2</id><name>Second</name></record></records>",
    )
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["name"], "new");
    assert_eq!(rows[1]["name"], "Second");
}

#[test]
fn cli_batch_ingest_and_query_preserve_cdata_field() {
    let dir = tempfile::tempdir().unwrap();
    let xml = dir.path().join("records.xml");
    std::fs::write(
        &xml,
        "<records><record><id>b1</id><name><![CDATA[Tower Bridge]]></name></record></records>",
    )
    .unwrap();
    let input = serde_json::json!([
        {"command": "ingest", "args": [xml.to_str().unwrap(), "--base-iri", "https://example.org/"]},
        {"command": "query", "args": "SELECT ?v WHERE { ?s <https://example.org/ont#name> ?v }"}
    ]);
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
        .args(["--no-connect", "--data-dir"])
        .arg(dir.path().join("state"))
        .arg("batch")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.to_string().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let results: Vec<serde_json::Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(results[0]["result"]["triples_loaded"], 3);
    assert_eq!(results[1]["result"]["results"][0]["v"], "\"Tower Bridge\"");
}

#[test]
fn unicode_whitespace_is_preserved_as_literal_field_content() {
    let rows = DataIngester::parse_xml(
        "<records><record><name>\u{a0}Tower Bridge\u{a0}</name></record></records>",
    )
    .unwrap();
    assert_eq!(rows[0]["name"], "\u{a0}Tower Bridge\u{a0}");
}

#[test]
fn empty_fields_keep_the_existing_absence_and_overwrite_contract() {
    let rows = DataIngester::parse_xml(
        "<records><record><id>b1</id><empty></empty><spaces> \t\r\n </spaces>\
         <name>Tower Bridge</name><name> \t </name></record></records>",
    )
    .unwrap();
    assert_eq!(rows[0]["name"], "Tower Bridge");
    assert!(!rows[0].contains_key("empty"));
    assert!(!rows[0].contains_key("spaces"));
}

#[test]
fn entity_encoded_boundary_spaces_keep_their_lexical_value() {
    let rows = DataIngester::parse_xml(
        "<records><record><name>&#x20;Tower Bridge&#x20;</name></record></records>",
    )
    .unwrap();
    assert_eq!(rows[0]["name"], " Tower Bridge ");
}

#[test]
fn comment_boundaries_do_not_turn_blank_text_into_a_field() {
    let rows = DataIngester::parse_xml(
        "<records><record><name> \t<!-- gap --> \r<!-- gap --> \n</name></record></records>",
    )
    .unwrap();
    assert!(!rows[0].contains_key("name"));
}
