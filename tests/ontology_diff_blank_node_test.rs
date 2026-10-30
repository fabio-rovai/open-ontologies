mod common;

use open_ontologies::ontology::OntologyService;

const ANONYMOUS: &str = r#"
@prefix ex: <https://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
ex:Order a owl:Class;
    rdfs:subClassOf [ a owl:Restriction; owl:onProperty ex:quantity;
                     owl:someValuesFrom <http://www.w3.org/2001/XMLSchema#integer> ] .
"#;
const EXPLICIT_REORDERED: &str = r#"
@prefix ex: <https://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
_:restriction owl:someValuesFrom <http://www.w3.org/2001/XMLSchema#integer>;
    owl:onProperty ex:quantity; a owl:Restriction .
ex:Order rdfs:subClassOf _:restriction; a owl:Class .
"#;

fn report(old: &str, new: &str) -> serde_json::Value {
    serde_json::from_str(&OntologyService::diff(old, new).unwrap()).unwrap()
}

fn assert_no_changes(value: &serde_json::Value) {
    assert_eq!(value["added"], 0);
    assert_eq!(value["removed"], 0);
    assert_eq!(value["added_triples"], serde_json::json!([]));
    assert_eq!(value["removed_triples"], serde_json::json!([]));
}

#[test]
fn anonymous_restrictions_and_equivalent_reordered_nodes_have_no_diff() {
    assert_no_changes(&report(ANONYMOUS, ANONYMOUS));
    assert_no_changes(&report(ANONYMOUS, EXPLICIT_REORDERED));
    assert_no_changes(&report(EXPLICIT_REORDERED, ANONYMOUS));
}

#[test]
fn anonymous_property_changes_remain_visible() {
    let changed = ANONYMOUS.replace("owl:onProperty ex:quantity", "owl:onProperty ex:price");
    let result = report(ANONYMOUS, &changed);
    assert_eq!(result["added"], 1);
    assert_eq!(result["removed"], 1);
    let added = result["added_triples"][0].as_str().unwrap();
    let removed = result["removed_triples"][0].as_str().unwrap();
    assert!(added.starts_with("_:c14n"));
    assert!(removed.starts_with("_:c14n"));
    assert!(added.ends_with("<https://example.org/price>"));
    assert!(removed.ends_with("<https://example.org/quantity>"));
}

#[test]
fn named_triple_changes_keep_existing_diff_counts_and_spelling() {
    let old =
        "<https://example.org/Order> <http://www.w3.org/2000/01/rdf-schema#label> \"Order\" .";
    let new = old.replace("\"Order\"", "\"Purchase order\"");
    assert_no_changes(&report(old, old));
    let result = report(old, &new);
    assert_eq!(result["added"], 1);
    assert_eq!(result["removed"], 1);
    assert_eq!(
        result["added_triples"],
        serde_json::json!([new.trim_end_matches(" .")])
    );
    assert_eq!(
        result["removed_triples"],
        serde_json::json!([old.trim_end_matches(" .")])
    );
}

#[test]
fn cli_diff_same_file_and_equivalent_anonymous_snapshot_report_no_changes() {
    let _gate = common::exec_gate();
    let tmp = tempfile::tempdir().unwrap();
    let old = tmp.path().join("orders.ttl");
    let equivalent = tmp.path().join("reordered.ttl");
    std::fs::write(&old, ANONYMOUS).unwrap();
    std::fs::write(&equivalent, EXPLICIT_REORDERED).unwrap();
    for new in [&old, &equivalent] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
            .args(["--no-connect", "--data-dir"])
            .arg(tmp.path())
            .arg("diff")
            .arg(&old)
            .arg(new)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_no_changes(&result);
    }
}
