//! `oo-lrat` end to end, through the same discipline every other checker uses.
//!
//! The point of this checker is the one thing `src/fol_solve.rs` says it does
//! not have: "two solvers agreeing on `unsat` is two opinions and not a proof".
//! A checked proof is not an opinion. These tests run the binary on real files
//! and require the three exit codes to mean three different things.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

/// One scratch directory per CALL, not per process.
///
/// Keyed on the pid alone, the five tests below share a directory, and cargo
/// runs them in parallel: each overwrote the others' `p.cnf` and `p.lrat`
/// mid-run, so four of them checked a proof against a formula another test had
/// just written. Every one of them failed, which was lucky; the shape that is
/// not lucky is two tests agreeing for the wrong reason.
static NEXT: AtomicU64 = AtomicU64::new(0);

fn checker() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("OO_LRAT") {
        let p = PathBuf::from(p);
        return p.exists().then_some(p);
    }
    let built = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lean/.lake/build/bin/oo-lrat");
    built.exists().then_some(built)
}

struct Run {
    code: i32,
    out: String,
}

fn run(cnf: &str, lrat: &str) -> Option<Run> {
    let bin = checker()?;
    let dir = std::env::temp_dir().join(format!(
        "oo-lrat-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let c = dir.join("p.cnf");
    let l = dir.join("p.lrat");
    std::fs::write(&c, cnf).unwrap();
    std::fs::write(&l, lrat).unwrap();
    let o = Command::new(bin).arg(&c).arg(&l).output().expect("oo-lrat runs");
    Some(Run {
        code: o.status.code().unwrap_or(-1),
        out: String::from_utf8_lossy(&o.stdout).into_owned(),
    })
}

const UNSAT: &str = "p cnf 2 4\n1 2 0\n1 -2 0\n-1 2 0\n-1 -2 0\n";
const PROOF: &str = "5 1 0 2 1 0\n6 -1 0 4 3 0\n7 0 5 6 0\n";

#[test]
fn a_checked_proof_exits_zero_and_names_its_theorem() {
    let Some(r) = run(UNSAT, PROOF) else { return };
    assert_eq!(r.code, 0, "a checked proof must be exit 0: {}", r.out);
    assert!(
        r.out.contains(r#""theorem":"Lrat.unsat_of_check""#),
        "exit 0 must name the statement it discharged: {}",
        r.out
    );
    assert!(r.out.contains(r#""verdict":"unsatisfiable""#), "{}", r.out);
}

#[test]
fn a_proof_that_does_not_check_is_exit_one_and_names_no_theorem() {
    // Same formula, hints that do not propagate.
    let Some(r) = run(UNSAT, "5 1 0 3 4 0\n7 0 5 0\n") else { return };
    assert_eq!(r.code, 1, "a proof that fails to check must be exit 1: {}", r.out);
    assert!(
        !r.out.contains("theorem"),
        "a refusal must not name a theorem, or the word is free: {}",
        r.out
    );
}

#[test]
fn a_satisfiable_formula_cannot_be_refuted() {
    // `1 ∨ 2` has a model, so no proof of the empty clause can check.
    let Some(r) = run("p cnf 2 1\n1 2 0\n", "2 0 1 0\n") else { return };
    assert_eq!(r.code, 1, "{}", r.out);
    assert!(!r.out.contains("theorem"), "{}", r.out);
}

#[test]
fn an_unreadable_file_is_exit_two_and_not_a_verdict() {
    // A clause that never terminates is a truncated file, not an empty clause.
    let Some(r) = run("p cnf 1 1\n1\n", PROOF) else { return };
    assert_eq!(r.code, 2, "a parse failure must be exit 2, not a verdict: {}", r.out);
    assert!(!r.out.contains("verdict"), "{}", r.out);
}

#[test]
fn the_empty_clause_must_be_reached_and_a_prefix_is_not_a_proof() {
    let Some(r) = run(UNSAT, "5 1 0 2 1 0\n") else { return };
    assert_eq!(r.code, 1, "every line checking is not the same as proving: {}", r.out);
}

// ── The solver, wired ───────────────────────────────────────────────────────
//
// The tests above check proofs this file wrote. These check proofs a SOLVER
// wrote, which is the only kind that matters: a checker that only accepts its
// author's proofs has not been tested against anything.

use open_ontologies::sat::{refute, to_dimacs, SatOutcome};

fn solver_present() -> bool {
    std::env::var("OO_SAT_SOLVER").map(|p| std::path::Path::new(&p).exists()).unwrap_or(false)
        || std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .any(|d| d.join("picosat").is_file())
}

fn scratch(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "oo-sat-{}-{}-{}",
        std::process::id(),
        tag,
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

/// A deterministic pseudo-random 3-CNF. No `rand` dependency, and the same
/// seeds give the same formulas on every machine, which is what makes a
/// failure reproducible from the seed alone.
fn random_cnf(seed: u64) -> Vec<Vec<i32>> {
    let mut x = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
    let mut next = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    let vars = 3 + (next() % 7) as i32;
    let n = (vars as f64 * 5.5) as usize;
    (0..n)
        .map(|_| {
            (0..3)
                .map(|_| {
                    let v = 1 + (next() % vars as u64) as i32;
                    if next() % 2 == 0 { v } else { -v }
                })
                .collect()
        })
        .collect()
}

#[test]
fn a_solvers_own_refutations_check() {
    if checker().is_none() || !solver_present() {
        return;
    }
    // No `undetermined` counter: an undetermined outcome panics below, so a
    // count of them could never be read, and `-D warnings` says so.
    let (mut refuted, mut sat) = (0, 0);
    for seed in 1..60u64 {
        let clauses = random_cnf(seed);
        let dir = scratch("fuzz");
        match refute(&clauses, &dir).expect("the pipeline runs") {
            SatOutcome::Refuted(cert) => {
                assert_eq!(
                    cert.theorem(),
                    "Lrat.unsat_of_check",
                    "the token must carry the statement oo-lrat named"
                );
                refuted += 1;
            }
            SatOutcome::Satisfiable => sat += 1,
            SatOutcome::Undetermined(why) => {
                // A solver that says unsat and whose proof does not check is a
                // DISAGREEMENT, and the one thing that must never be silent.
                panic!("seed {seed}: {why}\n{}", to_dimacs(&clauses));
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
    assert!(
        refuted > 0 && sat > 0,
        "the corpus must contain both answers or it is measuring nothing: \
         refuted {refuted}, satisfiable {sat}"
    );
}

#[test]
fn a_satisfiable_formula_is_not_reported_as_refuted() {
    if checker().is_none() || !solver_present() {
        return;
    }
    let dir = scratch("sat");
    match refute(&[vec![1, 2], vec![-1, 2]], &dir).expect("runs") {
        SatOutcome::Satisfiable => {}
        other => panic!("a satisfiable formula came back as {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}
