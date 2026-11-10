// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use open_ontologies::{graph::GraphStore, tptp};

const ONTOLOGY: &str = r#"
@prefix ex: <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
ex:Person a owl:Class .
ex:Alice a ex:Person .
ex:Bob a ex:Person .
"#;

const ALICE: &str = "<http://example.org/Alice>\t<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>\t<http://example.org/Person>";
const BOB: &str = "<http://example.org/Bob>\t<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>\t<http://example.org/Person>";

#[test]
fn reusing_an_export_directory_removes_stale_goal_files_only() {
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(ONTOLOGY, None).expect("fixture parses");
    let temp = tempfile::tempdir().expect("tempdir");
    let out = temp.path().join("export");
    let goals = temp.path().join("goals.tsv");
    let syntax = tptp::Syntax::parse("tptp", None, None, None).expect("TPTP");

    std::fs::write(&goals, format!("{ALICE}\n{BOB}\n")).expect("two goals");
    tptp::export(&graph, &out, syntax, Some(&goals), 0).expect("first export");
    let stale = out.join("goals/goal_00001.p");
    assert!(stale.exists(), "the second goal was written");
    let unrelated = out.join("goals/notes.txt");
    std::fs::write(&unrelated, "keep this user file").expect("unrelated file");

    std::fs::write(&goals, format!("{ALICE}\n")).expect("one goal");
    let report = tptp::export(&graph, &out, syntax, Some(&goals), 0)
        .expect("second export reuses output directory");
    let report: serde_json::Value = serde_json::from_str(&report).expect("JSON report");

    assert_eq!(report["goal_problems"], 1);
    assert!(
        !stale.exists(),
        "the prior run's second goal must not survive"
    );
    assert!(
        out.join("goals/goal_00000.p").exists(),
        "the current goal remains"
    );
    assert_eq!(
        std::fs::read_to_string(&unrelated).unwrap(),
        "keep this user file"
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(out.join("goals.json")).expect("current manifest"))
            .expect("manifest JSON");
    assert_eq!(manifest["goals"].as_array().unwrap().len(), 1);

    tptp::export(&graph, &out, syntax, None, 0).expect("export without goals");
    assert!(!out.join("goals/goal_00000.p").exists());
    assert!(!out.join("goals.json").exists());
    assert_eq!(
        std::fs::read_to_string(unrelated).unwrap(),
        "keep this user file"
    );
}
