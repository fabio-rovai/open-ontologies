// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! The owl-dl headline says `consistent: null` when the search that would
//! decide it did not finish.
//!
//! `consistent` was `tbox && abox.consistent`, and both default to consistent
//! when their search runs out. So an inconsistent knowledge base whose ABox
//! phase hit its budget printed a definite `"consistent": true`, with
//! `complete: false` and `abox.undecided: true` further down the same report
//! (#355). A consumer reading the field it would naturally read got a false
//! claim. Only a finished search may say `true`; a proof of a clash in either
//! phase still says `false`.
//!
//! Caps rather than clocks, because a cap stops a tableau at the same place on
//! every machine. The ABox case runs the binary with `[reasoner]
//! tableaux_max_nodes` in its own `config.toml`, the trigger reported on the
//! issue: the ABox tableau needs a node per individual and the TBox test needs
//! one, so the cap stops the first and not the second. The step cap cannot
//! separate them, since the clash below is on asserted types and found in two
//! steps. The TBox case moves the PROCESS-WIDE step cap in this binary, so every
//! in-process run takes `CAP`.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::Reasoner;
use open_ontologies::runtime;

static CAP: Mutex<()> = Mutex::new(());

const PREFIXES: &str = "@prefix : <http://example.org/t#> .\n\
    @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n";

/// The #355 repro at a size a test can afford: `n` individuals, each in its own
/// class with a disjoint partner, and one individual in both classes of a pair.
fn abox_clash(n: usize) -> String {
    let mut s = PREFIXES.to_string();
    for i in 0..n {
        s.push_str(&format!(
            ":C{i} a owl:Class . :D{i} a owl:Class . :D{i} owl:disjointWith :C{i} .\n\
             :x{i} a owl:NamedIndividual , :C{i} .\n"
        ));
    }
    s.push_str(&format!(":x{} a :D{} .\n", n - 1, n - 1));
    s
}

/// A consistent TBox whose consistency test has work to do: every individual
/// of `owl:Thing` must pick a side of two disjoint pairs.
const BRANCHING_TBOX: &str = ":A a owl:Class . :B a owl:Class . :A owl:disjointWith :B .\n\
    :P a owl:Class . :Q a owl:Class . :P owl:disjointWith :Q .\n\
    owl:Thing rdfs:subClassOf [ owl:unionOf ( :A :B ) ] , [ owl:unionOf ( :P :Q ) ] .\n";

/// A property chain can entail the edge that violates an all-values restriction.
/// `owl-dl` omits that axiom, so its finished search cannot prove consistency.
const PROPERTY_CHAIN_CLASH: &str = ":hasParent a owl:ObjectProperty .\n\
    :hasGrandparent a owl:ObjectProperty ; owl:propertyChainAxiom ( :hasParent :hasParent ) .\n\
    :NoGrandparent a owl:Class ; owl:equivalentClass [ a owl:Restriction ; owl:onProperty :hasGrandparent ; owl:allValuesFrom owl:Nothing ] .\n\
    :a a owl:NamedIndividual , :NoGrandparent ; :hasParent :b .\n\
    :b a owl:NamedIndividual ; :hasParent :c .\n\
    :c a owl:NamedIndividual .\n";

fn store(ttl: &str) -> Arc<GraphStore> {
    let g = Arc::new(GraphStore::new());
    g.load_turtle(ttl, None).expect("ontology must parse");
    g
}

/// Run owl-dl under a step cap (`None` leaves the default), putting the cap
/// back before anything can panic.
fn owl_dl(ttl: &str, cap: Option<usize>) -> serde_json::Value {
    let _serial = CAP.lock().unwrap_or_else(|e| e.into_inner());
    let previous = runtime::tableaux_max_steps().unwrap_or(0) as usize;
    if let Some(c) = cap {
        runtime::set_tableaux_max_steps(c);
    }
    let out = Reasoner::run(&store(ttl), "owl-dl", false);
    runtime::set_tableaux_max_steps(previous);
    serde_json::from_str(&out.expect("owl-dl run")).unwrap()
}

/// Run owl-dl in the binary over a data directory whose `config.toml` caps the
/// tableau at `max_nodes` nodes. `batch` loads and reasons in one process, so
/// the store is the one the file was loaded into.
fn owl_dl_with_node_cap(dir: &Path, ttl: &str, max_nodes: usize) -> serde_json::Value {
    use std::io::Write as _;
    let data = dir.join("data");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::write(
        data.join("config.toml"),
        format!("[reasoner]\ntableaux_max_nodes = {max_nodes}\n"),
    )
    .unwrap();
    let file = dir.join("abox_clash.ttl");
    std::fs::write(&file, ttl).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_open-ontologies"))
        .arg("--no-connect")
        .arg("--data-dir")
        .arg(&data)
        .args(["batch", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("binary should run");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(format!("load {}\nreason --profile owl-dl\n", file.display()).as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let last = stdout.lines().rfind(|l| !l.trim().is_empty()).unwrap_or_else(|| {
        panic!("batch printed nothing; stderr: {}", String::from_utf8_lossy(&out.stderr))
    });
    let envelope: serde_json::Value = serde_json::from_str(last).unwrap();
    assert_eq!(envelope["command"], "reason", "{envelope:#}");
    envelope["result"].clone()
}

#[test]
fn the_uncapped_run_proves_the_repro_inconsistent() {
    let v = owl_dl(&abox_clash(50), None);
    assert_eq!(v["consistent"], false, "{v:#}");
    assert_eq!(v["abox"]["consistent"], false, "{v:#}");
    assert_eq!(v["search_finished"], true, "{v:#}");
}

#[test]
fn an_unmodelled_property_chain_does_not_prove_the_kb_consistent() {
    let v = owl_dl(&format!("{PREFIXES}{PROPERTY_CHAIN_CLASH}"), None);
    assert_eq!(v["consistent"], serde_json::Value::Null, "{v:#}");
    assert_eq!(v["complete"], false, "{v:#}");
    assert_eq!(v["search_finished"], true, "{v:#}");
    assert_eq!(v["constructs_not_modelled"][0]["construct"], "owl:propertyChainAxiom", "{v:#}");
}

#[test]
fn an_inconsistent_kb_whose_abox_phase_ran_out_is_not_reported_consistent() {
    let d = tempfile::tempdir().unwrap();
    let v = owl_dl_with_node_cap(d.path(), &abox_clash(200), 50);

    assert_eq!(v["abox"]["undecided"], true, "the cap did not stop the ABox phase: {v:#}");
    assert_eq!(v["search_finished"], false, "{v:#}");
    assert!(
        v["budget_exhausted_in"].as_array().unwrap().iter().any(|p| p == "abox"),
        "{v:#}"
    );
    assert_eq!(
        v["consistent"],
        serde_json::Value::Null,
        "the KB is inconsistent and the search that would show it did not finish, so the \
         headline must be undetermined, not a definite answer: {v:#}"
    );
    assert_eq!(v["abox"]["consistent"], serde_json::Value::Null, "{v:#}");
    // The TBox phase finished, so its own answer stands.
    assert_eq!(v["tbox_consistent"], true, "{v:#}");
}

#[test]
fn a_tbox_consistency_test_that_ran_out_is_undetermined_and_named() {
    let decided = owl_dl(&format!("{PREFIXES}{BRANCHING_TBOX}"), None);
    assert_eq!(decided["consistent"], true, "{decided:#}");
    assert_eq!(decided["tbox_consistent"], true, "{decided:#}");

    let v = owl_dl(&format!("{PREFIXES}{BRANCHING_TBOX}"), Some(1));
    assert_eq!(v["tbox_consistent"], serde_json::Value::Null, "{v:#}");
    assert_eq!(v["consistent"], serde_json::Value::Null, "{v:#}");
    assert_eq!(v["search_finished"], false, "{v:#}");
    assert_eq!(v["complete"], false, "{v:#}");
    assert!(
        v["budget_exhausted_in"].as_array().unwrap().iter().any(|p| p == "consistency"),
        "the phase that ran out must be named: {v:#}"
    );
    let note = v["budget"]["note"].as_str().unwrap();
    assert!(note.contains("consistency"), "{note}");
    assert!(note.contains("step, node or depth cap"), "a step cap stopped it, not a clock: {note}");
}

#[test]
fn the_human_rendering_says_undetermined() {
    let d = tempfile::tempdir().unwrap();
    let v = owl_dl_with_node_cap(d.path(), &abox_clash(200), 50);
    let text = open_ontologies::output::render_human_for(Some("reason"), &v);
    assert!(text.contains("consistent: undetermined"), "{text}");
}
