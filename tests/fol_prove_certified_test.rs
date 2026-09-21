//! `onto_fol_prove`'s certified path: CNF first, `oo-resolution` after the replay,
//! and the word `refutation_certified` only from a minted token.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use open_ontologies::graph::GraphStore;
use open_ontologies::tstp::{prove_export, ProveOptions, Prover};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn scratch() -> PathBuf {
    let d = std::env::temp_dir().join(format!("oo-prove-cert-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn tools() -> bool {
    let fores = std::env::var("OO_RESOLUTION").map(|p| PathBuf::from(p).exists()).unwrap_or(false)
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lean/.lake/build/bin/oo-resolution").exists();
    fores && Prover::Vampire.available()
}

fn graph_of(path: &str) -> std::sync::Arc<GraphStore> {
    let ttl = std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)).expect("fixture");
    let g = std::sync::Arc::new(GraphStore::new());
    g.load_turtle(&ttl, None).expect("turtle parses");
    g
}

fn run(path: &str, goal: &str) -> serde_json::Value {
    let dir = scratch();
    let goals = dir.join("goals.tsv");
    std::fs::write(&goals, goal).unwrap();
    let opts = ProveOptions { prover: Prover::Vampire, timeout_secs: 20 };
    let out = prove_export(&graph_of(path), &dir, &opts, Some(goals.as_path()), 0).expect("prove runs");
    serde_json::from_str(&out).expect("json")
}

const RL: &str = "benchmark/generated/ies-building-extension.ttl";
const RL_GOAL: &str = "<http://example.org/ontology/ies-building#Building>\t<http://www.w3.org/2000/01/rdf-schema#subClassOf>\t<http://ies.data.gov.uk/ontology/ies4#Entity>\n";

#[test]
fn an_rl_goal_is_certified_by_the_theorem_with_no_user_action() {
    if !tools() { return; }
    let j = run(RL, RL_GOAL);
    assert_eq!(j["problem_form"], "cnf", "an RL ontology must go to the prover as clauses: {j}");
    let g = &j["goals"][0]["report"];
    assert_eq!(g["verdict"], "refutation_certified", "{g}");
    assert_eq!(g["certificate"]["status"], "certified", "{g}");
    assert_eq!(g["certificate"]["theorem"], "Fo.unsat_of_check", "the theorem name comes from the token: {g}");
    assert!(g["checked_by"].as_str().unwrap().contains("Fo.unsat_of_check"), "{g}");
    assert!(!g["checked_by"].as_str().unwrap().contains("lean/ is not involved"), "{g}");
    assert_eq!(j["certified"], 1, "{j}");
    assert!(j["certified_means"].as_str().unwrap().starts_with("this many"), "{j}");
}

const DL: &str = "benchmark/generated/pizza-ai.ttl";
const DL_GOAL: &str = "<http://www.co-ode.org/ontologies/pizza/pizza.owl#American>\t<http://www.w3.org/2000/01/rdf-schema#subClassOf>\t<http://www.co-ode.org/ontologies/pizza/pizza.owl#Pizza>\n";

#[test]
fn a_dl_ontology_stays_an_opinion_and_the_report_names_the_axiom_that_cost_it() {
    if !Prover::Vampire.available() { return; }
    let j = run(DL, DL_GOAL);
    assert_eq!(j["problem_form"], "fof", "a superclass existential forces FOF: {j}");
    let g = &j["goals"][0]["report"];
    assert_ne!(g["verdict"], "refutation_certified", "{g}");
    assert_eq!(g["certificate"]["status"], "outside_clausal_fragment", "{g}");
    let why = g["certificate"]["why"].as_str().unwrap_or("");
    assert!(why.contains("Skolem FUNCTION") && why.contains("owl_"), "the refusal names the axiom: {why}");
    assert!(g["checked_by"].as_str().unwrap().contains("ORACLE OPINION"), "{g}");
    assert_eq!(j["certified"], 0, "{j}");
    assert!(j["certified_means"].as_str().unwrap().starts_with("NOTHING HERE IS CERTIFIED"), "{j}");
}

#[test]
fn the_certified_word_cannot_appear_without_the_checker() {
    // Same RL goal, checker pointed at a path that does not exist. An explicit
    // OO_RESOLUTION is an instruction, not a hint, so there is no fallback: the
    // replay still runs, the certificate field says the checker was absent,
    // and the WORD is not printed. This test is the sole writer of OO_RESOLUTION in
    // its process.
    if !Prover::Vampire.available() { return; }
    let had = std::env::var("OO_RESOLUTION").ok();
    unsafe { std::env::set_var("OO_RESOLUTION", "/nonexistent/oo-resolution") };
    let j = run(RL, RL_GOAL);
    match had { Some(v) => unsafe { std::env::set_var("OO_RESOLUTION", v) }, None => unsafe { std::env::remove_var("OO_RESOLUTION") } }
    let g = &j["goals"][0]["report"];
    assert_eq!(j["problem_form"], "cnf", "{j}");
    assert_ne!(g["verdict"], "refutation_certified", "no checker ran, so the word may not appear: {g}");
    assert_eq!(g["certificate"]["status"], "checker_absent", "{g}");
    assert!(g["checked_by"].as_str().unwrap().contains("ORACLE OPINION"), "{g}");
    assert_eq!(j["certified"], 0, "{j}");
}
