//! Propositional refutation, with the proof checked rather than believed.
//!
//! `fol_solve.rs` states the problem this module answers: "two solvers agreeing
//! on `unsat` is two opinions and not a proof". A solver's `unsat` is an opinion
//! because the search behind it is large, fast and untrusted. The proof it emits
//! is neither large to check nor trusted to produce, so this runs the solver,
//! converts what it emits, and hands it to `oo-lrat`, whose acceptance
//! discharges `Lrat.unsat_of_check`.
//!
//! The word `refuted` is minted by [`crate::verdict::CheckerRun::accepted_naming`]
//! and nowhere else, so it cannot be printed without a checker run that named
//! that theorem.

use crate::verdict::{CheckerBinary, CheckerRun, Certified};
use std::path::{Path, PathBuf};
use std::process::Command;

/// What a refutation attempt produced.
#[derive(Debug)]
pub enum SatOutcome {
    /// The proof checked. The formula has no model, and the token says which
    /// statement says so.
    Refuted(Certified),
    /// The solver reported a model. Not a certificate: this module checks
    /// refutations, and a reported model is the solver's word.
    Satisfiable,
    /// No answer, and the reason. Never folded into either of the above.
    Undetermined(String),
}

/// A clause as DIMACS writes it: non-zero signed integers.
pub type Clause = Vec<i32>;

/// Render clauses as DIMACS CNF.
pub fn to_dimacs(clauses: &[Clause]) -> String {
    let vars = clauses
        .iter()
        .flat_map(|c| c.iter())
        .map(|l| l.unsigned_abs())
        .max()
        .unwrap_or(0);
    let mut s = format!("p cnf {} {}\n", vars, clauses.len());
    for c in clauses {
        for l in c {
            s.push_str(&l.to_string());
            s.push(' ');
        }
        s.push_str("0\n");
    }
    s
}

/// picosat's extended trace (`-T`) to LRAT.
///
/// The formats agree on derived lines: `id lit* 0 antecedent* 0`. A trace also
/// lists the ORIGINAL clauses, with an EMPTY antecedent list, and those come
/// from the CNF rather than from the proof, so they are dropped. A trace also
/// lists only the clauses in the unsat core and keeps their input identifiers,
/// which is why identifiers are not positions anywhere in this pipeline.
pub fn trace_to_lrat(trace: &str) -> String {
    let mut out = String::new();
    for line in trace.lines() {
        let toks: Vec<&str> = line.split_whitespace().collect();
        let Some(zero) = toks.iter().position(|t| *t == "0") else {
            continue;
        };
        // An original clause: nothing between the two terminators.
        if toks.get(zero + 1).is_none_or(|t| *t == "0") {
            continue;
        }
        out.push_str(&toks.join(" "));
        out.push('\n');
    }
    out
}

fn find(env: &str, name: &str) -> Option<PathBuf> {
    if let Ok(p) = std::env::var(env) {
        let p = PathBuf::from(p);
        return p.exists().then_some(p);
    }
    let built = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("lean/.lake/build/bin")
        .join(name);
    if built.exists() {
        return Some(built);
    }
    which_on_path(name)
}

fn which_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

/// Refute `clauses`, and check the refutation.
///
/// `dir` is where the intermediate files go; the caller owns its lifetime so a
/// failed run can be inspected.
pub fn refute(clauses: &[Clause], dir: &Path) -> anyhow::Result<SatOutcome> {
    let Some(solver) = find("OO_SAT_SOLVER", "picosat") else {
        return Ok(SatOutcome::Undetermined(
            "no SAT solver found. Set OO_SAT_SOLVER, or install picosat".into(),
        ));
    };
    let Some(checker) = find("OO_LRAT", "oo-lrat") else {
        return Ok(SatOutcome::Undetermined(
            "oo-lrat was not found, so a proof could not be checked. Build it \
             with `cd lean && lake build`, or set OO_LRAT"
                .into(),
        ));
    };

    std::fs::create_dir_all(dir)?;
    let cnf = dir.join("problem.cnf");
    let trace = dir.join("problem.trace");
    let lrat = dir.join("problem.lrat");
    std::fs::write(&cnf, to_dimacs(clauses))?;

    let run = Command::new(&solver).arg("-T").arg(&trace).arg(&cnf).output()?;
    match run.status.code() {
        // picosat: 20 unsatisfiable, 10 satisfiable.
        Some(10) => return Ok(SatOutcome::Satisfiable),
        Some(20) => {}
        other => {
            return Ok(SatOutcome::Undetermined(format!(
                "the solver exited {other:?}, which its CLI does not define as an answer"
            )))
        }
    }

    let trace_text = std::fs::read_to_string(&trace).unwrap_or_default();
    if trace_text.is_empty() {
        return Ok(SatOutcome::Undetermined(
            "the solver said unsatisfiable and emitted no trace, so there is \
             nothing to check. That is an opinion, and this function does not \
             report opinions"
                .into(),
        ));
    }
    std::fs::write(&lrat, trace_to_lrat(&trace_text))?;

    let mut cmd = Command::new(checker.clone());
    cmd.arg(&cnf).arg(&lrat);
    let checked = CheckerRun::spawn(&CheckerBinary::found_at(checker), cmd, &[&cnf, &lrat])?;
    match checked.accepted_naming(&["Lrat.unsat_of_check"]) {
        Some(cert) => Ok(SatOutcome::Refuted(cert)),
        None => Ok(SatOutcome::Undetermined(format!(
            "the solver said unsatisfiable and the proof did NOT check, which is \
             a disagreement between a solver and a checker and not a verdict. \
             exit {}: {}",
            checked.exit(),
            checked.output().trim()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimacs_counts_the_variables_it_saw() {
        let s = to_dimacs(&[vec![1, -3], vec![2]]);
        assert!(s.starts_with("p cnf 3 2\n"), "{s}");
        assert!(s.contains("1 -3 0\n"), "{s}");
    }

    #[test]
    fn the_original_clauses_of_a_trace_are_not_proof_lines() {
        // Two originals (empty antecedent lists) and one derived line.
        let t = "1 1 2 0 0\n2 -1 0 0\n5 2 0 1 2 0\n";
        assert_eq!(trace_to_lrat(t), "5 2 0 1 2 0\n");
    }

    #[test]
    fn a_blank_or_ragged_line_is_skipped_and_not_guessed_at() {
        assert_eq!(trace_to_lrat("\n   \nnot a line\n"), "");
    }
}
