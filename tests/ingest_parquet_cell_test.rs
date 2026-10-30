use arrow::array::{Date32Array, Decimal128Array, Int64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use open_ontologies::ingest::DataIngester;
use parquet::arrow::ArrowWriter;
use std::sync::Arc;

#[test]
fn parquet_cells_keep_their_own_date_decimal_and_null_values() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("dates.parquet");
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("date", DataType::Date32, true),
        Field::new("amount", DataType::Decimal128(6, 2), true),
        Field::new("count", DataType::Int64, false),
    ]));
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(vec!["one", "two", "three"])),
            Arc::new(Date32Array::from(vec![Some(20727), Some(20728), None])),
            Arc::new(
                Decimal128Array::from(vec![Some(1234), Some(5678), None])
                    .with_precision_and_scale(6, 2)
                    .unwrap(),
            ),
            Arc::new(Int64Array::from(vec![10, 20, 30])),
        ],
    )
    .unwrap();
    let mut writer =
        ArrowWriter::try_new(std::fs::File::create(&file).unwrap(), schema, None).unwrap();
    writer.write(&batch).unwrap();
    writer.close().unwrap();
    let rows = DataIngester::parse_file(file.to_str().unwrap()).unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0]["date"], "2026-10-01");
    assert_eq!(rows[1]["date"], "2026-10-02");
    assert_eq!(rows[0]["amount"], "12.34");
    assert_eq!(rows[1]["amount"], "56.78");
    assert_eq!(rows[2]["date"], "");
    assert_eq!(rows[2]["amount"], "");
    assert_eq!(rows[0]["id"], "one");
    assert_eq!(rows[1]["count"], "20");
}

#[test]
fn primitive_and_null_parquet_cells_keep_existing_lexical_values() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("primitives.parquet");
    let schema = Arc::new(Schema::new(vec![
        Field::new("name", DataType::Utf8, true),
        Field::new("count", DataType::Int64, true),
    ]));
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(StringArray::from(vec![Some("one"), None])),
            Arc::new(Int64Array::from(vec![Some(10), None])),
        ],
    )
    .unwrap();
    let mut writer =
        ArrowWriter::try_new(std::fs::File::create(&file).unwrap(), schema, None).unwrap();
    writer.write(&batch).unwrap();
    writer.close().unwrap();
    let rows = DataIngester::parse_file(file.to_str().unwrap()).unwrap();
    assert_eq!(rows[0]["name"], "one");
    assert_eq!(rows[0]["count"], "10");
    assert_eq!(rows[1]["name"], "");
    assert_eq!(rows[1]["count"], "");
}
