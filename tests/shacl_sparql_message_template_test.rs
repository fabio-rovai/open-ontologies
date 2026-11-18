// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! SHACL 5.3.2 message templates are expanded for each solution (#411).

use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::{ShaclReport, ShaclValidator};
use open_ontologies::shacl_report::ReportFormat;
use open_ontologies::temporal::ScopeRequest;
use oxigraph::io::{RdfFormat, RdfParser};
use oxigraph::model::{Literal, Term};
use serde_json::Value;
use std::io::Cursor;
use std::sync::Arc;

fn report(data: &str, message: &str, select: &str) -> ShaclReport {
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(data, None).unwrap();
    let shapes = format!(
        r#"
        @prefix ex: <http://example.org/> .
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Item ;
            sh:sparql [ {message} sh:select """{select}""" ] .
        "#
    );
    ShaclValidator::report_scoped(&graph, &shapes, &ScopeRequest::Unscoped).unwrap()
}

fn messages(report: &ShaclReport) -> Vec<String> {
    let json: Value = serde_json::from_str(&report.to_json_string()).unwrap();
    assert_eq!(json["conforms"], false, "{json}");
    assert!(json["skipped_constraints"].is_null(), "{json}");
    let mut messages: Vec<_> = json["violations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["message"].as_str().unwrap().to_string())
        .collect();
    messages.sort();
    messages
}

const DATA: &str = r#"
    @prefix ex: <http://example.org/> .
    ex:a a ex:Item ; ex:v 5 .
"#;

const SELECT_VALUE: &str =
    "SELECT $this ?value WHERE { $this <http://example.org/v> ?value . FILTER(?value > 3) }";

fn assert_rdf_message(report: &ShaclReport, expected: Literal) {
    for (format, parser_format) in [
        (ReportFormat::Turtle, RdfFormat::Turtle),
        (ReportFormat::NTriples, RdfFormat::NTriples),
    ] {
        let rendered = report.render(format).unwrap();
        let found: Vec<_> = RdfParser::from_format(parser_format)
            .for_reader(Cursor::new(rendered.as_bytes()))
            .map(Result::unwrap)
            .filter(|q| q.predicate.as_str() == "http://www.w3.org/ns/shacl#resultMessage")
            .map(|q| q.object)
            .collect();
        assert_eq!(found, vec![Term::Literal(expected.clone())], "{rendered}");
    }
}

#[test]
fn the_issue_example_expands_value_and_focus_without_changing_the_result_value() {
    let r = report(
        DATA,
        r#"sh:message "value too big: {?value} (focus {$this})" ;"#,
        SELECT_VALUE,
    );
    assert_eq!(
        messages(&r),
        ["value too big: 5 (focus <http://example.org/a>)"]
    );
    let json: Value = serde_json::from_str(&r.to_json_string()).unwrap();
    assert_eq!(json["violation_count"], 1);
    assert_eq!(
        json["violations"][0]["value"],
        "\"5\"^^<http://www.w3.org/2001/XMLSchema#integer>"
    );
    assert_rdf_message(
        &r,
        Literal::new_simple_literal("value too big: 5 (focus <http://example.org/a>)"),
    );
}

#[test]
fn each_solution_expands_both_variable_markers_and_repeated_names() {
    let data = r#"
        @prefix ex: <http://example.org/> .
        ex:a a ex:Item ; ex:v 5, 9 .
        ex:b a ex:Item ; ex:v 7 .
    "#;
    let r = report(
        data,
        r#"sh:message "{?this}: {?value}/{$value}/{?value}; {$label}" ;"#,
        r#"SELECT $this ?value ("large" AS ?label) WHERE { $this <http://example.org/v> ?value }"#,
    );
    assert_eq!(
        messages(&r),
        [
            "<http://example.org/a>: 5/5/5; large",
            "<http://example.org/a>: 9/9/9; large",
            "<http://example.org/b>: 7/7/7; large",
        ]
    );
}

#[test]
fn unbound_variables_and_ordinary_braces_remain_literal_text() {
    let r = report(
        DATA,
        r#"sh:message "{?missing} {$missing} {value} {?} { ?value} {?value!} {ordinary {?value}} {?value" ;"#,
        "SELECT $this ?value ?missing WHERE { $this <http://example.org/v> ?value }",
    );
    assert_eq!(
        messages(&r),
        ["{?missing} {$missing} {value} {?} { ?value} {?value!} {ordinary 5} {?value"]
    );
}

#[test]
fn inserted_literal_values_are_not_expanded_again() {
    let data = r#"
        @prefix ex: <http://example.org/> .
        ex:a a ex:Item ; ex:v "{$this} and {?other}" .
    "#;
    let r = report(
        data,
        r#"sh:message "{?value} / {$other}" ;"#,
        r#"SELECT $this ?value ("done" AS ?other) WHERE { $this <http://example.org/v> ?value }"#,
    );
    assert_eq!(messages(&r), ["{$this} and {?other} / done"]);
}

#[test]
fn blank_node_focus_and_iri_values_keep_their_rdf_form() {
    let data = r#"
        @prefix ex: <http://example.org/> .
        _:item a ex:Item ; ex:v ex:value .
    "#;
    let r = report(
        data,
        r#"sh:message "{?this}: {$value}" ;"#,
        "SELECT $this ?value WHERE { $this <http://example.org/v> ?value }",
    );
    let json: Value = serde_json::from_str(&r.to_json_string()).unwrap();
    let focus = json["violations"][0]["focus_node"].as_str().unwrap();
    assert!(focus.starts_with("_:"), "{json}");
    let expected = format!("{focus}: <http://example.org/value>");
    assert_eq!(messages(&r), [expected.as_str()]);
    assert_rdf_message(&r, Literal::new_simple_literal(expected));
}

#[test]
fn language_and_escaped_literal_contents_survive_rdf_report_serialization() {
    let data = r#"
        @prefix ex: <http://example.org/> .
        ex:a a ex:Item ; ex:v "line\n\"quoted\"\\path"@en .
    "#;
    let r = report(
        data,
        r#"sh:message "Valeur: {?value} ({$this})"@fr ;"#,
        "SELECT $this ?value WHERE { $this <http://example.org/v> ?value }",
    );
    let expected = "Valeur: line\n\"quoted\"\\path (<http://example.org/a>)";
    assert_eq!(messages(&r), [expected]);
    assert_rdf_message(
        &r,
        Literal::new_language_tagged_literal(expected, "fr").unwrap(),
    );
}

#[test]
fn a_bound_message_keeps_precedence_and_is_not_a_template() {
    let r = report(
        DATA,
        r#"sh:message "fallback {?value}" ;"#,
        r#"SELECT $this ?value ("override {?value}"@en AS ?message) WHERE {
            $this <http://example.org/v> ?value
        }"#,
    );
    assert_eq!(messages(&r), ["override {?value}"]);
    assert_rdf_message(
        &r,
        Literal::new_language_tagged_literal("override {?value}", "en").unwrap(),
    );
}

#[test]
fn default_and_static_messages_keep_their_existing_behavior() {
    for (message, expected) in [
        ("", "SPARQL constraint violated"),
        (r#"sh:message "" ;"#, "SPARQL constraint violated"),
        (r#"sh:message "too big" ;"#, "too big"),
    ] {
        let r = report(DATA, message, SELECT_VALUE);
        assert_eq!(messages(&r), [expected]);
        assert_rdf_message(&r, Literal::new_simple_literal(expected));
    }
}
