use open_ontologies::ingest::DataIngester;

macro_rules! unfinished_document {
    ($name:ident, $source:expr) => {
        #[test]
        fn $name() {
            let result = DataIngester::parse_xml($source);
            assert!(result.is_err(), "unfinished source accepted: {result:?}");
            assert!(result.unwrap_err().to_string().contains("XML parse error"));
        }
    };
}

unfinished_document!(unclosed_root, "<records>");
unfinished_document!(unclosed_record, "<records><record>");
unfinished_document!(unclosed_field, "<records><record><name>Tower");
unfinished_document!(
    closed_field_in_unclosed_record,
    "<records><record><name>Tower</name>"
);
unfinished_document!(
    completed_record_in_unclosed_root,
    "<records><record><name>Tower</name></record>"
);
unfinished_document!(
    completed_prefix_does_not_hide_truncated_final_record,
    "<records><record><id>1</id></record><record><id>2</id>"
);

#[test]
fn completed_documents_still_preserve_all_rows_and_text() {
    for source in [
        "",
        " \t\n",
        "<records/>",
        "<records><record/></records>",
        "<records><record><name/></record></records>",
        "<records></records>",
        "<?xml version=\"1.0\"?><records></records>",
    ] {
        let rows = DataIngester::parse_xml(source).unwrap();
        assert!(rows.iter().all(|row| row.is_empty()), "{source}: {rows:?}");
    }
    let rows = DataIngester::parse_xml("<records><record><id>1</id><name>Tower<![CDATA[ Bridge]]></name></record><record><id>2</id><name>A &amp; B</name></record></records>").unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["name"], "Tower Bridge");
    assert_eq!(rows[1]["name"], "A & B");
    for source in [
        "</records>",
        "<records><record></records>",
        "<records><record><name>&unknown;</name></record></records>",
    ] {
        assert!(DataIngester::parse_xml(source).is_err());
    }
}

#[test]
fn file_dispatch_rejects_completed_prefix_of_truncated_xml() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("records.xml");
    std::fs::write(
        &file,
        "<records><record><id>1</id></record><record><id>2</id>",
    )
    .unwrap();
    assert!(DataIngester::parse_file(file.to_str().unwrap()).is_err());
    assert!(DataIngester::parse_file_with_format(file.to_str().unwrap(), Some("xml")).is_err());
}

#[test]
fn cli_ingest_exits_unsuccessfully_instead_of_reporting_partial_rows() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("records.xml");
    std::fs::write(
        &file,
        "<records><record><id>1</id></record><record><id>2</id>",
    )
    .unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
        .args(["--no-connect", "--data-dir"])
        .arg(dir.path().join("state"))
        .arg("ingest")
        .arg(&file)
        .env_remove("OPEN_ONTOLOGIES_STORAGE_MODE")
        .env_remove("OPEN_ONTOLOGIES_TOKEN")
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "partial ingestion succeeded: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("XML parse error"));
}
