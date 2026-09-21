//! `onto_fol_prove`'s certified path: CNF first, `oo-resolution` after the replay,
//! and the word `refutation_certified` only from a minted token.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

use open_ontologies::graph::GraphStore;
use open_ontologies::tstp::{prove_export, ProveOptions, Prover};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Cargo runs the tests in this file as parallel THREADS of one process, and
/// `$OO_RESOLUTION` is process-wide. One test sets it to a path that does not
/// exist, on purpose, to see the absent branch; the others read it to find the
/// checker. Being the sole WRITER does not make that safe when the readers are
/// its own siblings: the writer's window overlapped a reader twice on 21
/// September 2026, and the reader reported `checker_absent` and a verdict of
/// `refutation_fully_replayed` where `refutation_certified` was expected.
///
/// So every test that depends on how the checker is found takes this lock.
/// They then run one at a time, which costs a few seconds and removes a flake
/// that looks exactly like a real regression in the certified path.
///
/// `lock().unwrap_or_else(|e| e.into_inner())`: a test that panics while
/// holding the lock poisons it, and the next test would then fail for a
/// reason that has nothing to do with what it checks.
static CHECKER_ENV: Mutex<()> = Mutex::new(());

fn checker_env() -> MutexGuard<'static, ()> {
    CHECKER_ENV.lock().unwrap_or_else(|e| e.into_inner())
}

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
    let _env = checker_env();
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
    let _env = checker_env();
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
    let _env = checker_env();
    // Same RL goal, checker pointed at a path that does not exist. An explicit
    // OO_RESOLUTION is an instruction, not a hint, so there is no fallback: the
    // replay still runs, the certificate field says the checker was absent,
    // and the WORD is not printed. This test is the sole writer of OO_RESOLUTION in
    // its process, and it holds `checker_env()` so no sibling can read the
    // variable while it is pointed at nothing.
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
