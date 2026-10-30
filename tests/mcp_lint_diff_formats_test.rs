//! The MCP `onto_lint` and `onto_diff` handlers must read a file the way
//! `onto_load` and CLI `lint` do.
//!
//! They used to pass the raw file to the Turtle parser, so an RDF/XML `.owl`
//! file failed with "Invalid IRI code point" and Turtle with relative IRIs
//! failed with "No scheme found in an absolute IRI", while CLI `lint` and
//! `onto_validate` accepted the same files.

use open_ontologies::graph::GraphStore;
use open_ontologies::inputs::{OntoDiffInput, OntoLintInput};
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;
use rmcp::handler::server::wrapper::Parameters;
use std::sync::Arc;

const TURTLE: &str = r#"
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix ex: <http://ex.org/> .
    ex:Animal a owl:Class ; rdfs:label "Animal" .
    ex:Dog a owl:Class ; rdfs:subClassOf ex:Animal .
"#;

/// The same two classes as `TURTLE`, serialised as RDF/XML.
const RDF_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#">
  <rdf:Description rdf:about="http://ex.org/Animal">
    <rdf:type rdf:resource="http://www.w3.org/2002/07/owl#Class"/>
    <rdfs:label>Animal</rdfs:label>
  </rdf:Description>
  <rdf:Description rdf:about="http://ex.org/Dog">
    <rdf:type rdf:resource="http://www.w3.org/2002/07/owl#Class"/>
    <rdfs:subClassOf rdf:resource="http://ex.org/Animal"/>
  </rdf:Description>
</rdf:RDF>
"#;

fn server() -> (tempfile::TempDir, OpenOntologiesServer) {
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("state.db")).unwrap();
    let server = OpenOntologiesServer::new_with_graph(db, Arc::new(GraphStore::new()));
    (tmp, server)
}

fn write(dir: &tempfile::TempDir, name: &str, content: &str) -> String {
    let path = dir.path().join(name);
    std::fs::write(&path, content).unwrap();
    path.to_str().unwrap().to_string()
}

async fn lint(server: &OpenOntologiesServer, path: &str) -> serde_json::Value {
    let out = server
        .onto_lint(Parameters(OntoLintInput {
            input: path.to_string(),
            inline: None,
        }))
        .await;
    serde_json::from_str(&out).unwrap()
}

#[tokio::test]
async fn onto_lint_reads_rdf_xml_like_turtle() {
    let (tmp, server) = server();
    let ttl = write(&tmp, "a.ttl", TURTLE);
    let owl = write(&tmp, "a.owl", RDF_XML);

    let from_ttl = lint(&server, &ttl).await;
    let from_owl = lint(&server, &owl).await;
    assert!(
        from_owl.get("error").is_none(),
        "onto_lint failed on RDF/XML: {from_owl}"
    );
    assert_eq!(
        from_owl["issue_count"], from_ttl["issue_count"],
        "{from_owl}"
    );
}

#[tokio::test]
async fn onto_lint_resolves_relative_iris_against_the_file() {
    let (tmp, server) = server();
    let rel = write(
        &tmp,
        "rel.ttl",
        "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<#Dog> a owl:Class .\n",
    );

    let out = lint(&server, &rel).await;
    assert!(
        out.get("error").is_none(),
        "onto_lint failed on relative IRIs: {out}"
    );
    assert!(out["issue_count"].as_u64().unwrap() > 0, "{out}");
}

#[tokio::test]
async fn onto_diff_reads_rdf_xml_like_turtle() {
    let (tmp, server) = server();
    let ttl = write(&tmp, "a.ttl", TURTLE);
    let owl = write(&tmp, "a.owl", RDF_XML);

    let out: serde_json::Value = serde_json::from_str(
        &server
            .onto_diff(Parameters(OntoDiffInput {
                old_path: owl,
                new_path: ttl,
            }))
            .await,
    )
    .unwrap();
    assert!(
        out.get("error").is_none(),
        "onto_diff failed on RDF/XML: {out}"
    );
    assert_eq!(out["added"], 0, "{out}");
    assert_eq!(out["removed"], 0, "{out}");
}

/// Two versions of a relative-IRI ontology: v2 adds one class.
const REL_V1: &str = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<#Dog> a owl:Class .\n<#Cat> a owl:Class .\n";
const REL_V2: &str = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<#Dog> a owl:Class .\n<#Cat> a owl:Class .\n<#Cow> a owl:Class .\n";

/// Each file used to get its own `@base`, so `<#Dog>` was two IRIs and every
/// triple came back as both added and removed (3 and 2 here).
#[tokio::test]
async fn onto_diff_resolves_both_sides_against_one_base() {
    let (tmp, server) = server();
    let v1 = write(&tmp, "v1.ttl", REL_V1);
    let v2 = write(&tmp, "v2.ttl", REL_V2);

    let out: serde_json::Value = serde_json::from_str(
        &server
            .onto_diff(Parameters(OntoDiffInput {
                old_path: v1,
                new_path: v2,
            }))
            .await,
    )
    .unwrap();
    assert!(out.get("error").is_none(), "{out}");
    assert_eq!(out["added"], 1, "{out}");
    assert_eq!(out["removed"], 0, "{out}");
}

fn cli_diff(old: &str, new: &str) -> (Option<i32>, serde_json::Value) {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
        .args(["diff", old, new])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("not JSON ({e}): {stdout}"));
    (out.status.code(), json)
}

#[test]
fn cli_diff_resolves_both_sides_against_one_base() {
    let tmp = tempfile::tempdir().unwrap();
    let v1 = write(&tmp, "v1.ttl", REL_V1);
    let v2 = write(&tmp, "v2.ttl", REL_V2);

    let (_, out) = cli_diff(&v1, &v2);
    assert!(out.get("error").is_none(), "{out}");
    assert_eq!(out["added"], 1, "{out}");
    assert_eq!(out["removed"], 0, "{out}");
}

#[test]
fn cli_diff_names_the_file_it_cannot_read() {
    let tmp = tempfile::tempdir().unwrap();
    let v1 = write(&tmp, "v1.ttl", REL_V1);
    let missing = tmp.path().join("missing.ttl");
    let missing = missing.to_str().unwrap();

    let (code, out) = cli_diff(&v1, missing);
    assert_eq!(code, Some(1), "{out}");
    let err = out["error"].as_str().unwrap_or_default();
    assert!(err.starts_with(&format!("Cannot read {missing}: ")), "{out}");
}
