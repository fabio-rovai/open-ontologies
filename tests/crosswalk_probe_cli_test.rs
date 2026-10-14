//! The contradiction probe, reached the way a user reaches it.
//!
//! `crosswalk::contradiction_probe` carries a crosswalk's claims into the
//! target and reports a clash when the target's own axioms deny them. It had
//! no caller outside its unit tests, so the README's sentence about clash
//! reporting described something no command could do. It is now the
//! `crosswalk-probe` subcommand and the batch verb of the same name, beside
//! `crosswalk-certify`, and these tests drive both through the binary.

mod common;

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

const SOURCE: &str = "@prefix s: <http://src.example/> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl:  <http://www.w3.org/2002/07/owl#> .
s:Employee a owl:Class ; rdfs:subClassOf s:Person .
s:Person   a owl:Class ; rdfs:subClassOf s:Agent .
s:Agent    a owl:Class .
";

/// Employees and machines are disjoint here, and there is no individual at
/// all: a taxonomy-shaped target, which is what a crosswalk usually connects.
const TARGET: &str = "@prefix t: <http://tgt.example/> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix owl:  <http://www.w3.org/2002/07/owl#> .
t:Employee a owl:Class ; rdfs:subClassOf t:Agent ; owl:disjointWith t:Machine .
t:Machine  a owl:Class .
t:Person   a owl:Class ; rdfs:subClassOf t:Agent .
t:Agent    a owl:Class .
";

/// One wrong row: s:Employee onto t:Machine. The source puts Employee under
/// Person, Person maps onto t:Employee, so the mapping carries `t:Machine ⊑
/// t:Employee` into a target that says the two are disjoint.
const WRONG: &str = "subject_id\tpredicate_id\tobject_id
http://src.example/Employee\tskos:exactMatch\thttp://tgt.example/Machine
http://src.example/Person\tskos:exactMatch\thttp://tgt.example/Employee
http://src.example/Agent\tskos:exactMatch\thttp://tgt.example/Agent
";

const RIGHT: &str = "subject_id\tpredicate_id\tobject_id
http://src.example/Employee\tskos:exactMatch\thttp://tgt.example/Employee
http://src.example/Person\tskos:exactMatch\thttp://tgt.example/Person
http://src.example/Agent\tskos:exactMatch\thttp://tgt.example/Agent
";

fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
    let p = dir.join(name);
    let _gate = common::exec_gate();
    std::fs::write(&p, body).unwrap();
    p
}

fn run(dir: &Path, args: &[&Path]) -> (Option<i32>, Value) {
    let out = {
        let _gate = common::exec_gate();
        Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
            .arg("--data-dir")
            .arg(dir.join("data"))
            .args(args)
            .output()
            .unwrap()
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().find(|l| l.starts_with('{')).unwrap_or("null");
    (
        out.status.code(),
        serde_json::from_str(line).unwrap_or(Value::Null),
    )
}

#[test]
fn the_subcommand_reports_a_clash_the_mapping_carries_in() {
    let d = tempfile::tempdir().unwrap();
    let (s, t, m) = (
        write(d.path(), "s.ttl", SOURCE),
        write(d.path(), "t.ttl", TARGET),
        write(d.path(), "wrong.tsv", WRONG),
    );
    let (code, v) = run(d.path(), &[Path::new("crosswalk-probe"), &s, &t, &m]);
    assert_eq!(v["contradicted"], true, "{v}");
    assert_eq!(
        v["verdict"], "the_target_denies_what_the_mapping_carried",
        "{v}"
    );
    assert_eq!(
        v["target_alone"]["clash_count"], 0,
        "the target alone is consistent: {v}"
    );
    assert!(
        v["by_rule"]["cax-dw"].as_u64().unwrap_or(0) > 0,
        "the disjointness is the clash: {v}"
    );
    assert!(
        v["carried_claims"]
            .to_string()
            .contains("tgt.example/Machine"),
        "and it says what it carried across: {v}"
    );
    assert_eq!(code, Some(1), "a contradiction must fail the command: {v}");
}

#[test]
fn the_subcommand_reports_no_clash_for_an_honest_mapping() {
    let d = tempfile::tempdir().unwrap();
    let (s, t, m) = (
        write(d.path(), "s.ttl", SOURCE),
        write(d.path(), "t.ttl", TARGET),
        write(d.path(), "right.tsv", RIGHT),
    );
    let (code, v) = run(d.path(), &[Path::new("crosswalk-probe"), &s, &t, &m]);
    assert_eq!(v["contradicted"], false, "{v}");
    assert_eq!(
        v["verdict"], "no_contradiction_among_the_rules_this_engine_looks_for",
        "{v}"
    );
    assert!(
        v["carried"].as_u64().unwrap_or(0) > 0,
        "something was carried and checked: {v}"
    );
    assert!(
        v["means"]
            .as_str()
            .unwrap_or("")
            .contains("NOT proof of compatibility"),
        "a clean probe names its limit: {v}"
    );
    assert_eq!(code, Some(0), "{v}");
}

#[test]
fn the_batch_verb_runs_the_same_probe() {
    let d = tempfile::tempdir().unwrap();
    let (s, t, m) = (
        write(d.path(), "s.ttl", SOURCE),
        write(d.path(), "t.ttl", TARGET),
        write(d.path(), "wrong.tsv", WRONG),
    );
    let plan = serde_json::json!([{
        "command": "crosswalk-probe",
        "args": [s.to_str().unwrap(), t.to_str().unwrap(), m.to_str().unwrap()],
    }]);
    let p = write(d.path(), "plan.json", &plan.to_string());
    let (code, v) = run(
        d.path(),
        &[Path::new("--no-connect"), Path::new("batch"), &p],
    );
    assert_eq!(v["command"], "crosswalk-probe", "{v}");
    assert_eq!(v["result"]["contradicted"], true, "{v}");
    assert_eq!(
        code,
        Some(1),
        "the batch carries the probe's exit code: {v}"
    );
}

#[test]
fn a_missing_argument_is_an_error_not_a_clean_probe() {
    let d = tempfile::tempdir().unwrap();
    let p = write(
        d.path(),
        "plan.json",
        r#"[{"command":"crosswalk-probe","args":["only-one.ttl"]}]"#,
    );
    let (code, v) = run(
        d.path(),
        &[Path::new("--no-connect"), Path::new("batch"), &p],
    );
    assert!(
        v["result"]["error"]
            .as_str()
            .unwrap_or("")
            .contains("SOURCE TARGET MAPPINGS"),
        "{v}"
    );
    assert_eq!(code, Some(1));
}
