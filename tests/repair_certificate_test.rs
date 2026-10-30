//! The optimality half of a weighted repair, refuted by `oo-lrat`.
//!
//! `repair` says a repair is optimal when its branch and bound closes. With
//! `certificate_dir` the same claim, "no set of soft axioms with scaled cost at
//! most w* - 1 hits every collected conflict", is written as CNF, picosat
//! refutes it, and `oo-lrat` checks the proof under `Lrat.unsat_of_check`. The
//! word `refuted` is minted only by that acceptance, so the tests here hold the
//! two directions apart: with the tools present the status is `refuted` and the
//! token is about the files on disk; with either tool absent it is
//! `undetermined`, with the reason, and never `refuted`.
//!
//! The first test needs picosat (Ubuntu ships the tracing build as
//! `picosat.trace`, so CI points `OO_SAT_SOLVER` at it) and `oo-lrat`, and
//! skips loudly without them; under `OO_REQUIRE_FIXTURES=1` the skip is a
//! failure. The rest need nothing.

mod common;

use open_ontologies::graph::GraphStore;
use open_ontologies::repair::{RepairOptions, WeightKind, repair};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

const W: &str = "https://example.org/weight#confidence";

const PREFIXES: &str = r#"
@prefix ex: <https://example.org/x/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix w: <https://example.org/weight#> .
"#;

/// Issue #346's example: the optimum drops `A disjointWith B` at 0.62.
const ISSUE: &str = r#"
ex:A rdfs:subClassOf ex:B .
ex:A owl:disjointWith ex:B .
ex:C rdfs:subClassOf ex:A .
ex:D rdfs:subClassOf ex:A .
[] a owl:Axiom ; owl:annotatedSource ex:A ; owl:annotatedProperty rdfs:subClassOf ;
   owl:annotatedTarget ex:B ; w:confidence "0.87"^^xsd:decimal .
[] a owl:Axiom ; owl:annotatedSource ex:A ; owl:annotatedProperty owl:disjointWith ;
   owl:annotatedTarget ex:B ; w:confidence "0.62"^^xsd:decimal .
[] a owl:Axiom ; owl:annotatedSource ex:C ; owl:annotatedProperty rdfs:subClassOf ;
   owl:annotatedTarget ex:A ; w:confidence "0.99"^^xsd:decimal .
[] a owl:Axiom ; owl:annotatedSource ex:D ; owl:annotatedProperty rdfs:subClassOf ;
   owl:annotatedTarget ex:A ; w:confidence "0.99"^^xsd:decimal .
"#;

/// Two conflicts sharing `X disjointWith Y`; under `probability` the optimum
/// drops the two 0.6 edges, and its scaled log-odds run to seven digits, so
/// the BDD for the cost bound is not a toy.
const DIVERGE: &str = r#"
ex:P rdfs:subClassOf ex:X , ex:Y .
ex:Q rdfs:subClassOf ex:X , ex:Y .
ex:X owl:disjointWith ex:Y .
[] a owl:Axiom ; owl:annotatedSource ex:P ; owl:annotatedProperty rdfs:subClassOf ;
   owl:annotatedTarget ex:X ; w:confidence 0.6 .
[] a owl:Axiom ; owl:annotatedSource ex:Q ; owl:annotatedProperty rdfs:subClassOf ;
   owl:annotatedTarget ex:X ; w:confidence 0.6 .
[] a owl:Axiom ; owl:annotatedSource ex:X ; owl:annotatedProperty owl:disjointWith ;
   owl:annotatedTarget ex:Y ; w:confidence 0.9 .
"#;

/// The same discovery `src/sat.rs` does, so the skip below is about the tools
/// that run and not about a guess at them.
fn tool(env: &str, name: &str) -> Option<PathBuf> {
    if let Ok(p) = std::env::var(env) {
        let p = PathBuf::from(p);
        return p.exists().then_some(p);
    }
    let built = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lean/.lake/build/bin").join(name);
    if built.exists() {
        return Some(built);
    }
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

fn store(body: &str) -> Arc<GraphStore> {
    let g = GraphStore::new();
    g.load_turtle(&format!("{PREFIXES}{body}"), None).expect("the fixture parses");
    Arc::new(g)
}

fn certified(body: &str, weights: WeightKind, default_weight: f64, dir: &Path) -> Value {
    let opts = RepairOptions {
        weight_property: Some(W.to_string()),
        default_weight,
        weights,
        certificate_dir: Some(dir.to_path_buf()),
        ..Default::default()
    };
    repair(&store(body), &opts).expect("the repair runs")
}

#[test]
fn optimality_is_refuted_by_oo_lrat_when_the_tools_are_present() {
    let solver = tool("OO_SAT_SOLVER", "picosat");
    let checker = tool("OO_LRAT", "oo-lrat");
    if common::skip_unless(
        solver.is_some() && checker.is_some(),
        "picosat (with trace support) and oo-lrat",
        "install picosat, or set OO_SAT_SOLVER to a tracing build such as Ubuntu's \
         /usr/bin/picosat.trace, and build oo-lrat with `cd lean && lake build` or set OO_LRAT",
    ) {
        return;
    }
    let checker = checker.unwrap();

    for (name, body, weights, default_weight, bound) in [
        ("issue", ISSUE, WeightKind::Cost, 1.0, 61i64),
        // 2 x round(ln(1.5) x 10^6) - 1.
        ("diverge", DIVERGE, WeightKind::Probability, 1.0, 2 * 405_465 - 1),
    ] {
        let d = tempfile::tempdir().unwrap();
        let v = certified(body, weights, default_weight, d.path());
        let c = &v["certificate"];
        assert_eq!(v["optimal"], true, "{name}: {v:#}");
        assert_eq!(c["status"], "refuted", "{name}: {c:#}");
        assert_eq!(c["theorem"], "Lrat.unsat_of_check", "{name}");
        assert_eq!(c["bound_refuted"], bound, "{name}: {c:#}");
        assert_eq!(c["bound_refuted"].as_i64().unwrap() + 1, v["total_cost_scaled"].as_i64().unwrap());
        assert_eq!(c["scaled_integer_costs"], true);
        assert!(c["not_machine_checked"].as_str().unwrap().contains("encoder"), "{c:#}");

        // The token is about the files on disk, and the checker, run again
        // here by the test rather than by the tool, accepts them.
        let cnf = d.path().join("problem.cnf");
        let lrat = d.path().join("problem.lrat");
        let digest = open_ontologies::verdict::subject_digest(&[cnf.as_path(), lrat.as_path()]).unwrap();
        assert_eq!(c["subject_sha256"], open_ontologies::verdict::hex32(&digest), "{name}");
        let out = Command::new(&checker).arg(&cnf).arg(&lrat).output().unwrap();
        let said = String::from_utf8_lossy(&out.stdout);
        assert_eq!(out.status.code(), Some(0), "{name}: {said}");
        assert!(said.contains("Lrat.unsat_of_check"), "{name}: {said}");
        let table = std::fs::read_to_string(d.path().join("variables.tsv")).unwrap();
        assert!(table.starts_with("variable\tcost_scaled\t"), "{table}");
    }
}

fn cli(dir: &Path, args: &[&str], envs: &[(&str, &str)]) -> (i32, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_open-ontologies"));
    cmd.arg("--data-dir").arg(dir.join("data")).args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("the binary runs");
    (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stdout).into_owned())
}

#[test]
fn without_a_solver_or_a_checker_the_status_is_undetermined_and_never_refuted() {
    let d = tempfile::tempdir().unwrap();
    let input = d.path().join("issue.ttl");
    std::fs::write(&input, format!("{PREFIXES}{ISSUE}")).unwrap();
    let missing = d.path().join("no-such-binary");
    let missing = missing.to_str().unwrap();
    let real_solver = tool("OO_SAT_SOLVER", "picosat");
    // (case, environment overrides, what the reason must name)
    type Case<'a> = (&'a str, Vec<(&'a str, String)>, &'a str);
    let mut cases: Vec<Case> =
        vec![("no solver", vec![("OO_SAT_SOLVER", missing.to_string())], "SAT solver")];
    // The checker's absence can only be seen when a solver ran first.
    if let Some(s) = &real_solver {
        cases.push((
            "no checker",
            vec![("OO_SAT_SOLVER", s.display().to_string()), ("OO_LRAT", missing.to_string())],
            "oo-lrat",
        ));
    }
    for (name, envs, reason) in cases {
        let cert = d.path().join(name.replace(' ', "-"));
        let envs: Vec<(&str, &str)> = envs.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let (code, text) = cli(
            d.path(),
            &[
                "repair",
                input.to_str().unwrap(),
                "--weight-property",
                W,
                "--certificate-dir",
                cert.to_str().unwrap(),
            ],
            &envs,
        );
        assert_eq!(code, 0, "{name}: a missing tool is not a failed repair: {text}");
        let v: Value = serde_json::from_str(text.trim()).unwrap();
        let c = &v["certificate"];
        assert_eq!(c["status"], "undetermined", "{name}: {c:#}");
        assert!(c["reason"].as_str().unwrap().contains(reason), "{name}: {c:#}");
        assert!(c.get("theorem").is_none(), "{name}: no theorem without a checker run: {c:#}");
        assert!(!text.contains("\"refuted\""), "{name}: the word was printed with no checker: {text}");
        // The search's own answer stands, and says what it rests on.
        assert_eq!(v["optimal"], true);
    }
}

#[test]
fn nothing_to_certify_is_said_rather_than_refuted() {
    let d = tempfile::tempdir().unwrap();
    // Already coherent: the empty repair is the only one.
    let v = certified("ex:A rdfs:subClassOf ex:B .", WeightKind::Cost, 1.0, d.path());
    assert_eq!(v["certificate"]["status"], "not_needed", "{v:#}");
    // A repair that costs nothing: p = 0.4 is dropped at no cost, and no
    // repair can cost less than zero.
    let free = r#"
ex:R rdfs:subClassOf ex:T , ex:U .
ex:T owl:disjointWith ex:U .
[] a owl:Axiom ; owl:annotatedSource ex:R ; owl:annotatedProperty rdfs:subClassOf ;
   owl:annotatedTarget ex:T ; w:confidence 0.4 .
"#;
    let v = certified(free, WeightKind::Probability, 1.0, d.path());
    assert_eq!(v["certificate"]["status"], "not_needed", "{v:#}");
    assert_eq!(v["total_cost_scaled"], 0);
    assert!(!d.path().join("problem.cnf").exists(), "nothing was written for nothing to prove");
}
