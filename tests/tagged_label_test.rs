// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! Labels are read by their lexical value, not their N-Triples spelling (#406).
//!
//! `sparql_select` renders a literal in N-Triples form, so a tagged label
//! arrives as `"Dog"@en`. Stripping quotes from that leaves `Dog"@en`, which
//! matches nothing and compares badly with the same label untagged.

use arrow::array::{ArrayRef, StringArray};
use arrow::datatypes::{Field, Schema};
use arrow::record_batch::RecordBatch;
use open_ontologies::clinical::ClinicalCrosswalks;
use open_ontologies::drift::DriftDetector;
use open_ontologies::graph::GraphStore;
use open_ontologies::state::StateDb;
use parquet::arrow::ArrowWriter;
use std::sync::Arc;

#[test]
fn drift_compares_a_tagged_label_by_its_text() {
    let db = StateDb::open(&tempfile::tempdir().unwrap().keep().join("state.db")).unwrap();
    let v1 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Dog a owl:Class ; rdfs:label "Dog"@en .
    "#;
    let v2 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Hound a owl:Class ; rdfs:label "Dog" .
    "#;

    let out: serde_json::Value =
        serde_json::from_str(&DriftDetector::new(db).detect(v1, v2).unwrap()).unwrap();
    let rename = &out["likely_renames"][0];
    assert_eq!(rename["from"], "http://example.org/Dog", "{out}");
    assert_eq!(rename["to"], "http://example.org/Hound", "{out}");
    assert_eq!(rename["signals"]["label_similarity"], 1.0, "{out}");
}

#[test]
fn clinical_validation_matches_a_tagged_label_by_its_text() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("crosswalk.parquet");
    let columns: Vec<(&str, ArrayRef)> = [
        ("source_code", "I21"),
        ("source_system", "ICD10"),
        ("target_code", "22298006"),
        ("target_system", "SNOMED"),
        ("relation", "exactMatch"),
        ("source_label", "Myocardial infarction"),
        ("target_label", "Myocardial infarction"),
    ]
    .into_iter()
    .map(|(name, value)| (name, Arc::new(StringArray::from(vec![value])) as ArrayRef))
    .collect();
    let schema = Arc::new(Schema::new(
        columns
            .iter()
            .map(|(name, array)| Field::new(*name, array.data_type().clone(), true))
            .collect::<Vec<_>>(),
    ));
    let batch =
        RecordBatch::try_new(schema.clone(), columns.into_iter().map(|(_, a)| a).collect())
            .unwrap();
    let mut writer =
        ArrowWriter::try_new(std::fs::File::create(&path).unwrap(), schema, None).unwrap();
    writer.write(&batch).unwrap();
    writer.close().unwrap();
    let crosswalk = ClinicalCrosswalks::load(path.to_str().unwrap()).unwrap();

    let graph = GraphStore::new();
    graph
        .load_turtle(
            r#"@prefix owl: <http://www.w3.org/2002/07/owl#> .
               @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
               @prefix ex: <http://example.org/> .
               ex:MI a owl:Class ; rdfs:label "Myocardial infarction"@en ."#,
            None,
        )
        .unwrap();

    let out: serde_json::Value = serde_json::from_str(&crosswalk.validate_clinical(&graph)).unwrap();
    assert_eq!(out["unmatched"], serde_json::json!([]), "{out}");
    assert_eq!(out["validated"][0]["label"], "Myocardial infarction", "{out}");
}
