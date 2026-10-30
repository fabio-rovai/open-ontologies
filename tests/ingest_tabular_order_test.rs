use std::collections::HashMap;
use std::sync::Arc;

use arrow::array::{ArrayRef, Int64Array, RecordBatch, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use open_ontologies::induce::induce;
use open_ontologies::ingest::DataIngester;
use parquet::arrow::ArrowWriter;

fn assert_first_column_is_identifier(path: &str) {
    let rows = DataIngester::parse_file(path).unwrap();
    let headers = DataIngester::headers_in_order(path, &rows);
    assert_eq!(headers, ["record_id", "amount", "date"]);
    let inferred = induce(&rows, &headers, "Orders", "https://example.org/orders/");
    assert_eq!(inferred.id_column, "record_id");
    assert!(!inferred.id_synthesised);
    assert_eq!(inferred.mapping.id_field, "record_id");
}

#[test]
fn xlsx_first_column_remains_the_identifier_candidate() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/ingest/header-order.xlsx");
    assert_first_column_is_identifier(path.to_str().unwrap());
}

#[test]
fn parquet_schema_order_remains_the_identifier_candidate() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("orders.parquet");
    let schema = Arc::new(Schema::new(vec![
        Field::new("record_id", DataType::Utf8, false),
        Field::new("amount", DataType::Int64, false),
        Field::new("date", DataType::Utf8, false),
    ]));
    let columns: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(vec!["order-b", "order-a"])),
        Arc::new(Int64Array::from(vec![100, 200])),
        Arc::new(StringArray::from(vec!["2026-10-06", "2026-10-07"])),
    ];
    let batch = RecordBatch::try_new(schema.clone(), columns).unwrap();
    let mut writer =
        ArrowWriter::try_new(std::fs::File::create(&path).unwrap(), schema, None).unwrap();
    writer.write(&batch).unwrap();
    writer.close().unwrap();
    assert_first_column_is_identifier(path.to_str().unwrap());
}

#[test]
fn csv_order_and_sorted_fallback_and_extra_keys_are_preserved() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("orders.csv");
    std::fs::write(
        &path,
        "record_id,amount,date\norder-b,100,2026-10-06\norder-a,200,2026-10-07\n",
    )
    .unwrap();
    assert_first_column_is_identifier(path.to_str().unwrap());
    let mut rows = DataIngester::parse_file(path.to_str().unwrap()).unwrap();
    rows.push(HashMap::from([("extra".into(), "value".into())]));
    assert_eq!(
        DataIngester::headers_in_order(path.to_str().unwrap(), &rows),
        ["record_id", "amount", "date", "extra"]
    );
    assert_eq!(
        DataIngester::headers_in_order("unavailable.json", &rows),
        ["amount", "date", "extra", "record_id"]
    );
}
