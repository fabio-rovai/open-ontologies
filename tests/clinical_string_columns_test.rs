mod common;

use arrow::array::{ArrayRef, Int32Array, LargeStringArray, StringArray};
use arrow::datatypes::{Field, Schema};
use arrow::record_batch::RecordBatch;
use open_ontologies::clinical::ClinicalCrosswalks;
use parquet::arrow::ArrowWriter;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

const COLUMNS: [(&str, [&str; 2]); 7] = [
    ("source_code", ["SAMPLE1", "SAMPLE_OTHER"]),
    ("source_system", ["ICD10", "ICD10"]),
    ("target_code", ["SAMPLE2", "SAMPLE_OTHER_TARGET"]),
    ("target_system", ["SNOMED", "SNOMED"]),
    ("relation", ["exactMatch", "closeMatch"]),
    ("source_label", ["Sample source 商品", "Other source"]),
    ("target_label", ["Sample target", "Other target"]),
];

fn columns(large: [bool; 7]) -> Vec<(&'static str, ArrayRef)> {
    COLUMNS
        .iter()
        .zip(large)
        .map(|((name, values), large)| {
            let array: ArrayRef = if large {
                Arc::new(LargeStringArray::from(values.to_vec()))
            } else {
                Arc::new(StringArray::from(values.to_vec()))
            };
            (*name, array)
        })
        .collect()
}

fn write_crosswalk(path: &Path, columns: Vec<(&str, ArrayRef)>) {
    let _gate = common::exec_gate();
    let schema = Arc::new(Schema::new(
        columns
            .iter()
            .map(|(name, array)| Field::new(*name, array.data_type().clone(), true))
            .collect::<Vec<_>>(),
    ));
    let batch = RecordBatch::try_new(
        schema.clone(),
        columns.into_iter().map(|(_, array)| array).collect(),
    )
    .unwrap();
    let file = std::fs::File::create(path).unwrap();
    let mut writer = ArrowWriter::try_new(file, schema, None).unwrap();
    writer.write(&batch).unwrap();
    writer.close().unwrap();
}

fn assert_sample(crosswalk: &ClinicalCrosswalks) {
    let rows = crosswalk.lookup("SAMPLE1", "ICD10");
    assert_eq!(rows.len(), 1);
    let row = rows[0];
    assert_eq!(row.source_code, "SAMPLE1");
    assert_eq!(row.source_system, "ICD10");
    assert_eq!(row.target_code, "SAMPLE2");
    assert_eq!(row.target_system, "SNOMED");
    assert_eq!(row.relation, "exactMatch");
    assert_eq!(row.source_label, "Sample source 商品");
    assert_eq!(row.target_label, "Sample target");
    assert_eq!(crosswalk.lookup("SAMPLE_OTHER", "ICD10").len(), 1);
    assert!(crosswalk.lookup("SAMPLE1", "SNOMED").is_empty());
    let matches = crosswalk.search_label("Sample source 商品");
    assert_eq!(matches[0]["source_code"], "SAMPLE1");
    assert_eq!(matches[0]["similarity"], 1.0);
}

#[test]
fn crosswalk_loads_both_string_widths_and_mixed_columns() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("crosswalks.parquet");
    for large in [
        [false; 7],
        [true; 7],
        [true, false, true, false, true, false, true],
    ] {
        write_crosswalk(&path, columns(large));
        let crosswalk = ClinicalCrosswalks::load(path.to_str().unwrap()).unwrap();
        assert_sample(&crosswalk);
    }
}

#[test]
fn unsupported_or_missing_columns_and_null_labels_keep_existing_behavior() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("crosswalks.parquet");
    let mut missing = columns([false; 7]);
    missing.pop();
    write_crosswalk(&path, missing);
    assert!(
        ClinicalCrosswalks::load(path.to_str().unwrap())
            .unwrap()
            .lookup("SAMPLE1", "ICD10")
            .is_empty()
    );

    let mut unsupported = columns([true; 7]);
    unsupported[0].1 = Arc::new(Int32Array::from(vec![1, 2]));
    write_crosswalk(&path, unsupported);
    assert!(
        ClinicalCrosswalks::load(path.to_str().unwrap())
            .unwrap()
            .lookup("SAMPLE1", "ICD10")
            .is_empty()
    );

    for large in [false, true] {
        let mut nullable = columns([large; 7]);
        nullable[5].1 = if large {
            Arc::new(LargeStringArray::from(vec![None, Some("Other source")]))
        } else {
            Arc::new(StringArray::from(vec![None, Some("Other source")]))
        };
        write_crosswalk(&path, nullable);
        let crosswalk = ClinicalCrosswalks::load(path.to_str().unwrap()).unwrap();
        let rows = crosswalk.lookup("SAMPLE1", "ICD10");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].source_label, "");
    }
}

#[test]
fn crosswalk_cli_returns_identical_mappings_for_each_string_schema() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir(directory.path().join("data")).unwrap();
    let path = directory.path().join("data/crosswalks.parquet");
    let mut outputs = Vec::new();
    for large in [
        [false; 7],
        [true; 7],
        [true, false, true, false, true, false, true],
    ] {
        write_crosswalk(&path, columns(large));
        let output = {
            let _gate = common::exec_gate();
            Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
                .current_dir(directory.path())
                .args(["--no-connect", "--data-dir"])
                .arg(directory.path().join("state"))
                .args(["crosswalk", "SAMPLE1", "--system", "ICD10"])
                .output()
                .unwrap()
        };
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["mappings"].as_array().unwrap().len(), 1);
        assert_eq!(value["mappings"][0]["target_code"], "SAMPLE2");
        assert_eq!(value["mappings"][0]["source_label"], "Sample source 商品");
        outputs.push(value);
    }
    assert_eq!(outputs[0], outputs[1]);
    assert_eq!(outputs[0], outputs[2]);
}
