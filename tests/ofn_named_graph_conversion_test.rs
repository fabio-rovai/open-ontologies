// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

use open_ontologies::ofn_writer::convert_file;

#[test]
fn conversion_refuses_named_graphs_instead_of_flattening_them() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ontology.trig");
    std::fs::write(
        &path,
        r#"@prefix : <http://example.org/> .
           :source { :Person a <http://www.w3.org/2002/07/owl#Class> . }
        "#,
    )
    .unwrap();

    let error = convert_file(path.to_str().unwrap()).unwrap_err();
    assert!(error.to_string().contains("cannot preserve named graphs"), "{error}");
}

#[test]
fn conversion_accepts_a_default_graph_in_a_trig_document() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ontology.trig");
    std::fs::write(
        &path,
        r#"@prefix : <http://example.org/> .
           @prefix owl: <http://www.w3.org/2002/07/owl#> .
           :Person a owl:Class .
        "#,
    )
    .unwrap();

    let output = convert_file(path.to_str().unwrap()).unwrap();
    assert!(output.text.contains("Declaration(Class(:Person))"), "{}", output.text);
    assert!(output.not_exported.is_empty(), "{:#?}", output.not_exported);
}
