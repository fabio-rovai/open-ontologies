mod common;

use open_ontologies::graph::GraphStore;
use open_ontologies::ontology::OntologyService;

const RELATIVE_TTL: &str = "<#Order> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> .";
const RELATIVE_XML: &str = r##"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:owl="http://www.w3.org/2002/07/owl#">
<owl:Class rdf:about="#Order"/>
</rdf:RDF>"##;
const ABSOLUTE_TTL: &str = "<https://example.org/Order> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> .";

#[test]
fn validation_accepts_relative_iris_in_turtle_and_rdfxml() {
    let tmp = tempfile::tempdir().unwrap();
    for (name, content) in [("orders.ttl", RELATIVE_TTL), ("orders.rdf", RELATIVE_XML)] {
        let path = tmp.path().join(name);
        std::fs::write(&path, content).unwrap();
        let path = path.to_str().unwrap();
        let counts = GraphStore::validate_file(path).unwrap();
        assert_eq!(counts.triples, 1);
        assert_eq!(counts.statements, 1);
        let report: serde_json::Value =
            serde_json::from_str(&OntologyService::validate_file(path).unwrap()).unwrap();
        assert_eq!(report["valid"], true);
        assert_eq!(report["triple_count"], 1);
    }
}

#[test]
fn encoded_paths_resolve_the_entire_filename_before_the_fragment() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("Order Ontology % # café");
    std::fs::create_dir(&dir).unwrap();
    for (name, content) in [("orders.ttl", RELATIVE_TTL), ("orders.rdf", RELATIVE_XML)] {
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        let path_str = path.to_str().unwrap();
        let store = GraphStore::new();
        assert_eq!(store.load_file(path_str).unwrap(), 1);
        let serialized = store.serialize("ntriples").unwrap();
        assert!(serialized.contains("Order%20Ontology%20%25%20%23%20caf%C3%A9/"));
        assert!(
            serialized.contains(&format!("{name}#Order>")),
            "{serialized}"
        );
        assert_eq!(GraphStore::validate_file(path_str).unwrap().triples, 1);
        let normalized = GraphStore::read_as_turtle(path_str).unwrap();
        let parsed = GraphStore::new();
        parsed.load_turtle(&normalized, None).unwrap();
        assert_eq!(parsed.serialize("ntriples").unwrap(), serialized);
    }
}

#[test]
fn explicit_document_bases_override_the_local_file_base() {
    let tmp = tempfile::tempdir().unwrap();
    let turtle = format!("@base <https://example.org/base/> .\n{RELATIVE_TTL}");
    let xml = RELATIVE_XML.replace(
        "<rdf:RDF ",
        "<rdf:RDF xml:base=\"https://example.org/base/\" ",
    );
    for (name, content) in [("explicit.ttl", turtle), ("explicit.rdf", xml)] {
        let path = tmp.path().join(name);
        std::fs::write(&path, content).unwrap();
        let path = path.to_str().unwrap();
        let store = GraphStore::new();
        store.load_file(path).unwrap();
        assert!(
            store
                .serialize("ntriples")
                .unwrap()
                .starts_with("<https://example.org/base/#Order>")
        );
        assert_eq!(GraphStore::validate_file(path).unwrap().triples, 1);
        let normalized = GraphStore::read_as_turtle(path).unwrap();
        let normalized_store = GraphStore::new();
        normalized_store.load_turtle(&normalized, None).unwrap();
        assert_eq!(
            normalized_store.serialize("ntriples").unwrap(),
            store.serialize("ntriples").unwrap()
        );
    }
}

#[test]
fn absolute_iris_and_invalid_document_results_are_preserved() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("absolute.ttl");
    std::fs::write(&path, ABSOLUTE_TTL).unwrap();
    assert_eq!(
        GraphStore::validate_file(path.to_str().unwrap())
            .unwrap()
            .triples,
        1
    );
    let store = GraphStore::new();
    store.load_file(path.to_str().unwrap()).unwrap();
    assert!(
        store
            .serialize("ntriples")
            .unwrap()
            .starts_with("<https://example.org/Order>")
    );
    std::fs::write(&path, "this is not RDF").unwrap();
    assert!(GraphStore::validate_file(path.to_str().unwrap()).is_err());
    let report: serde_json::Value =
        serde_json::from_str(&OntologyService::validate_file(path.to_str().unwrap()).unwrap())
            .unwrap();
    assert_eq!(report["valid"], false);
    assert!(!report["errors"].as_array().unwrap().is_empty());
}

#[test]
fn cli_validation_and_conversion_accept_a_normal_folder_with_spaces() {
    let _gate = common::exec_gate();
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("Order Ontology");
    std::fs::create_dir(&dir).unwrap();
    for (name, content) in [("orders.ttl", RELATIVE_TTL), ("orders.rdf", RELATIVE_XML)] {
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        for command in ["validate", "convert", "lint"] {
            let mut process = std::process::Command::new(env!("CARGO_BIN_EXE_open-ontologies"));
            process.args(["--no-connect", "--data-dir"]).arg(tmp.path());
            process.arg(command).arg(&path);
            if command == "convert" {
                process.args(["--to", "ntriples"]);
            }
            let result = process.output().unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stdout)
            );
            let stdout = String::from_utf8(result.stdout).unwrap();
            if command == "validate" {
                let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
                assert_eq!(report["ok"], true);
                assert_eq!(report["triples"], 1);
            } else if command == "lint" {
                let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
                assert_eq!(report["issue_count"], 2);
            } else {
                assert!(stdout.contains("Order%20Ontology/"));
                assert!(stdout.contains(&format!("{name}#Order>")));
            }
        }
    }
}

#[test]
fn turtle_source_and_comments_are_preserved_and_inline_input_is_unchanged() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("orders.ttl");
    let content = format!("# A comment\n{RELATIVE_TTL}\n");
    std::fs::write(&path, &content).unwrap();
    let normalized = GraphStore::read_as_turtle(path.to_str().unwrap()).unwrap();
    assert!(normalized.starts_with("@base <file://"));
    assert_eq!(GraphStore::validate_turtle(&normalized).unwrap().triples, 1);
    assert!(normalized.ends_with(&content));
    assert_eq!(
        GraphStore::content_as_turtle("-", content.clone()).unwrap(),
        content
    );
    assert_eq!(
        GraphStore::content_as_turtle(
            tmp.path().join("missing.ttl").to_str().unwrap(),
            content.clone()
        )
        .unwrap(),
        content
    );
}

fn diagnostic_line(error: &str) -> u64 {
    error
        .split_once("line ")
        .and_then(|(_, suffix)| suffix.split_whitespace().next())
        .and_then(|line| line.parse().ok())
        .unwrap_or_else(|| panic!("no parser line in diagnostic: {error}"))
}

const MISSING_DOT_TTL: &str = concat!(
    "@prefix ex: <http://example.org/> .\n",
    "ex:a a ex:C .\n",
    "ex:b a ex:C\n",
    "ex:c a ex:C .\n",
);

#[test]
fn normalized_turtle_preserves_parse_error_lines_for_consumers() {
    use open_ontologies::modules::{Module, distributed};
    use open_ontologies::vocab_check::check_data_vocab;
    use std::sync::Arc;
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("diagnostics.ttl");
    let path_str = path.to_str().unwrap();
    let ontology = Arc::new(GraphStore::new());
    let commented = format!(
        "# Original leading comment\r\n@base <https://example.org/base/> .\r\n{}",
        MISSING_DOT_TTL.replace('\n', "\r\n")
    );
    let bom = format!("\u{feff}{ABSOLUTE_TTL}");
    for (name, content) in [
        ("first line", "this is not RDF"),
        ("missing dot", MISSING_DOT_TTL),
        ("comment and explicit base", commented.as_str()),
        ("BOM", bom.as_str()),
    ] {
        std::fs::write(&path, content).unwrap();
        let original_error = GraphStore::validate_file(path_str).unwrap_err().to_string();
        let expected_line = diagnostic_line(&original_error);
        if name == "missing dot" {
            assert_eq!(expected_line, 4, "{original_error}");
        }
        let normalized = GraphStore::read_as_turtle(path_str).unwrap();
        let modules = [Module {
            name: name.to_string(),
            ttl: normalized.clone(),
        }];
        let errors = [
            (
                "graph loader",
                GraphStore::new()
                    .load_turtle(&normalized, None)
                    .unwrap_err()
                    .to_string(),
            ),
            (
                "lint",
                OntologyService::lint(&normalized).unwrap_err().to_string(),
            ),
            ("modules", distributed(&modules, 1).unwrap_err().to_string()),
            (
                "vocabulary check",
                check_data_vocab(&ontology, &normalized, &[])
                    .unwrap_err()
                    .to_string(),
            ),
        ];
        for (consumer, error) in errors {
            assert_eq!(
                diagnostic_line(&error),
                expected_line,
                "{name}: {consumer}: {error}; original: {original_error}"
            );
        }
    }
}

#[test]
fn cli_checkers_report_the_original_turtle_error_line() {
    let _gate = common::exec_gate();
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("diagnostics.ttl");
    let path_str = path.to_str().unwrap();
    for content in ["this is not RDF", MISSING_DOT_TTL] {
        std::fs::write(&path, content).unwrap();
        let expected_line =
            diagnostic_line(&GraphStore::validate_file(path_str).unwrap_err().to_string());
        for command in ["validate", "lint", "defects", "vocab-check"] {
            let result = std::process::Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
                .args(["--no-connect", "--data-dir"])
                .arg(tmp.path())
                .arg(command)
                .arg(&path)
                .output()
                .unwrap();
            let stdout = String::from_utf8(result.stdout).unwrap();
            let stderr = String::from_utf8_lossy(&result.stderr);
            assert_eq!(
                result.status.code(),
                Some(1),
                "{command}: stdout={stdout}; stderr={stderr}"
            );
            let report: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|error| {
                panic!("{command}: {error}; stdout={stdout}; stderr={stderr}")
            });
            let error = report["error"]
                .as_str()
                .unwrap_or_else(|| panic!("{command}: {report}"));
            assert_eq!(diagnostic_line(error), expected_line, "{command}: {error}");
        }
    }
}
