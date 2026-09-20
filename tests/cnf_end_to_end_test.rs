//! Decision 0005's chain, closed for the clausal fragment, on a real ontology.
//!
//! `ies-building-extension.ttl` is exported as CNF with a goal that is two
//! subclass steps deep, Vampire refutes it, the refutation is translated into
//! `lean/Fo`'s format, and `oo-fores` accepts it, discharging
//! `Fo.unsat_of_check`. Every link is a real tool on real output; the test
//! skips cleanly when a tool is absent and asserts hard when it is present.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use open_ontologies::graph::GraphStore;
use open_ontologies::tptp::{export, Syntax};
use open_ontologies::tstp::to_fo_certificate;

static NEXT: AtomicU64 = AtomicU64::new(0);

fn scratch() -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "oo-cnf-e2e-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn on_path(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?).map(|d| d.join(name)).find(|p| p.is_file())
}

fn fores() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("OO_FORES") {
        let p = PathBuf::from(p);
        return p.exists().then_some(p);
    }
    let built = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lean/.lake/build/bin/oo-fores");
    built.exists().then_some(built)
}

const ONTOLOGY: &str = "benchmark/generated/ies-building-extension.ttl";

/// `Building ⊑ BuiltEntity ⊑ ies:Entity`, so the goal is derived, not asserted.
const GOAL: &str = "<http://example.org/ontology/ies-building#Building>\t\
                    <http://www.w3.org/2000/01/rdf-schema#subClassOf>\t\
                    <http://ies.data.gov.uk/ontology/ies4#Entity>\n";

fn export_with_goal(syntax: Syntax) -> (PathBuf, PathBuf) {
    let ttl = std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(ONTOLOGY))
        .expect("the benchmark ontology is in the tree");
    let graph = std::sync::Arc::new(GraphStore::new());
    graph.load_turtle(&ttl, None).expect("turtle parses");
    let dir = scratch();
    let goals = dir.join("goal.tsv");
    std::fs::write(&goals, GOAL).unwrap();
    export(&graph, &dir, syntax, Some(goals.as_path()), 0).expect("export runs");
    let goal_file = dir.join("goals").join("goal_00000.p");
    assert!(goal_file.exists(), "the goal problem must be written for {}", syntax.name());
    (dir, goal_file)
}

fn szs(vampire: &Path, problem: &Path) -> (String, String) {
    let o = Command::new(vampire).arg("--proof").arg("tptp").arg(problem).output().unwrap();
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    let status = text
        .lines()
        .find_map(|l| l.strip_prefix("% SZS status ").map(|r| r.split_whitespace().next().unwrap_or("").to_string()))
        .unwrap_or_default();
    (status, text)
}

#[test]
fn the_cnf_export_of_a_real_ontology_is_clauses_only() {
    let (_dir, goal) = export_with_goal(Syntax::Cnf);
    let text = std::fs::read_to_string(&goal).unwrap();
    assert!(text.lines().any(|l| l.starts_with("cnf(")), "{}", &text[..text.len().min(400)]);
    assert!(!text.lines().any(|l| l.starts_with("fof(")), "no fof records in a cnf export");
    assert!(!text.contains('?'), "no existential survives; the two closed ones become constants");
    assert!(text.contains("i:$domain_witness"), "background_2's witness constant is named");
    assert!(text.contains("i:$sk_goal_"), "the negated goal's witness constant is named");
    assert!(text.contains("negated_conjecture"), "the goal is negated once and says so");
}

#[test]
fn fof_and_cnf_of_the_same_goal_get_the_same_verdict() {
    // `Theorem` is what Vampire says of a FOF problem with a conjecture and
    // `Unsatisfiable` of a CNF problem whose negated conjecture is asserted.
    // They are one verdict in two dialects, and the two encodings must agree.
    let Some(v) = on_path("vampire") else { return };
    let (_d1, fof) = export_with_goal(Syntax::Tptp);
    let (_d2, cnf) = export_with_goal(Syntax::Cnf);
    let (sf, _) = szs(&v, &fof);
    let (sc, _) = szs(&v, &cnf);
    assert_eq!(sf, "Theorem", "FOF encoding");
    assert_eq!(sc, "Unsatisfiable", "CNF encoding");
}

#[test]
fn vampires_refutation_of_the_cnf_export_is_checked_by_the_lean_theorem() {
    let (Some(v), Some(f)) = (on_path("vampire"), fores()) else { return };
    let (dir, cnf) = export_with_goal(Syntax::Cnf);
    let (status, proof) = szs(&v, &cnf);
    assert_eq!(status, "Unsatisfiable");

    let cert = to_fo_certificate(&proof).expect("the derivation parses");
    assert!(
        cert.untranslated.is_empty(),
        "a CNF problem's refutation is resolution end to end; left out: {:?}",
        cert.untranslated
    );
    assert!(cert.reaches_false, "the certificate must reach the empty clause");

    let cert_path = dir.join("r.cert");
    std::fs::write(&cert_path, &cert.text).unwrap();
    let o = Command::new(f).arg(&cert_path).output().unwrap();
    let out = String::from_utf8_lossy(&o.stdout);
    assert_eq!(o.status.code(), Some(0), "oo-fores refused a real refutation:\n{out}\n{}", cert.text);
    assert!(out.contains(r#""theorem":"Fo.unsat_of_check""#), "{out}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_fof_refutation_of_the_same_goal_does_not_translate_whole() {
    // The control. Same ontology, same goal, FOF encoding: the prover
    // clausifies and Skolemises first, and none of that is resolution, so the
    // certificate must NOT reach the empty clause. This is the measurement
    // that justified building the CNF path at all.
    let Some(v) = on_path("vampire") else { return };
    let (_dir, fof) = export_with_goal(Syntax::Tptp);
    let (_, proof) = szs(&v, &fof);
    let cert = to_fo_certificate(&proof).expect("parses");
    assert!(!cert.reaches_false, "FOF input must leave clausification untranslated");
    assert!(
        cert.untranslated.iter().any(|(_, r)| r.contains("cnf_transformation") || r.contains("skolemi")),
        "the untranslated steps must be named: {:?}",
        cert.untranslated
    );
}
