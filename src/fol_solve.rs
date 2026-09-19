//! The certifying pipeline: export, solve, ingest, CHECK, and report a verdict
//! that says what it rests on.
//!
//! ```text
//!   FolProblem ──┬─ problem.tsv ─────────────────────────────┐
//!                │                                           │
//!                ├─ problem.smt2  ── z3   ─┐                 │
//!                └─ problem.in    ── mace4 ┴─ model.tsv ──── oo-folmodel
//!                                                                │
//!                                                        Fol.satisfiable_of_check
//! ```
//!
//! One `FolProblem`, three files, ONE question. The solver's file and
//! `problem.tsv` are both folds over `FolProblem::checker_entries`, so the
//! thing the solver is asked and the thing the checker checks cannot drift
//! apart by construction rather than by care.
//!
//! # The verdict discipline, which is the reason this exists
//!
//! Decision 0006 item 4 fixes five fields and five verdict words, and this
//! module is where they are computed. The rules are mechanical, and
//! `tests/fol_solver_verdict_test.rs` fails if any of them is ever relaxed.
//! Since the verdict-by-construction change they are also UNREPRESENTABLE
//! otherwise: the verdict is a [`crate::verdict::FolVerdict`], its certified
//! variant carries a [`crate::verdict::Certified`], and that type has no
//! public constructor. The rules restated, with what now enforces each:
//!
//! 1. `model_checked` requires `checker_exit == 0`. It is not a string in this
//!    file any more. `FolVerdict::ModelChecked` takes evidence that only
//!    `CheckerRun::accepted` returns, and only on a zero exit code from a
//!    process this crate spawned. A new code path cannot write that word.
//! 2. `unsatisfiable_oracle` requires a run whose emitted problem carried NO
//!    cardinality constraint. It is written only in the arm that reads the
//!    unbounded probe's answer. A bounded run's `unsat` is
//!    `no_model_up_to_size_k`, which is not unsatisfiability: the formula
//!    `∀x∃y (r(x,y) ∧ x≠y)` is unsat at carrier 1 and sat at carrier 2.
//! 3. `owl_reading` is non-null only when the CHECKER reported
//!    `goal_negated_present` and the verdict is `model_checked`. It is read
//!    back out of the checker's own JSON rather than taken from this side's
//!    intention, and it carries its own long word,
//!    `not_entailed_under_unproved_translation`, which must never be shortened
//!    to "not entailed": the OWL-level reading rides on `OwlLean.adequacy` in
//!    a sibling project AND on the Rust-to-Lean correspondence that decision
//!    0005 item 2 states is pinned by tests and NOT proved.
//!
//! # A rejected model is a stop-the-line event
//!
//! If a solver answers `sat` and the verified checker then REJECTS the
//! structure, two things that are supposed to agree do not. Either the solver
//! is wrong, or the ingestion misread it, or the SMT-LIB/LADR printer does not
//! say what `problem.tsv` says. None of those is a fact about the ontology, so
//! the verdict stays `satisfiable_oracle` exactly as decision 0006 item 5
//! requires and is NEVER `rejected` as though the ontology were at fault.
//!
//! But it is not a quiet fallback either. It is reported in its own
//! [`Disagreement`] block with `severity: STOP_THE_LINE`, it is counted in the
//! run summary, and it makes the command exit non-zero — the same treatment
//! `tools/shacl_differential.py` gives a FALSE_CLEAN, and for the same reason:
//! a disagreement between a checker and the thing it is checking is the one
//! result that must fail a pipeline rather than be filed under "other".
//!
//! # What a green run does NOT say
//!
//! Nothing about unsatisfiability unless the word `unsatisfiable_oracle` is
//! printed, and that word is an ORACLE's, never a certificate's. The absence
//! of a finite model implies nothing: SHIQ lacks the finite model property, so
//! a satisfiable ontology can have only infinite models and will never receive
//! a certificate here. That is a limitation of the method and it is the honest
//! counterweight to the asymmetry the layer is built on.

use std::collections::BTreeMap;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::fol_model::{FiniteModel, IngestError, cvc5, mace4, z3};
use crate::tptp::{FolProblem, ladr, smtlib::SmtEncoding};
use crate::verdict::{CheckerBinary, CheckerRun, FolVerdict, OwlReading};

/// The Lean statement `oo-folmodel`'s acceptance discharges. It travels inside
/// the evidence token, so a report cannot name it without an accepted run.
const FOL_THEOREM: &str = "Fol.satisfiable_of_check";

/// Which model finder to drive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Solver {
    /// Z3 over SMT-LIB 2. Carrier declared as an enumeration datatype, so a
    /// `sat` comes with a structure of known size; also one of the two that
    /// can be asked the unbounded question.
    Z3,
    /// cvc5 over the SAME SMT-LIB 2 this crate already emits, as a SECOND
    /// oracle rather than a better one.
    ///
    /// It exists so that a `sat` or an `unsat` has somewhere to be contradicted
    /// from. Decision 0006's line does not move an inch for it: a `sat` whose
    /// model `oo-folmodel` accepts is a certificate whichever solver produced
    /// it, and an `unsat` is an oracle opinion whichever solver produced it,
    /// and two solvers agreeing on `unsat` is two opinions and not a proof.
    ///
    /// Measured on cvc5 1.3.4, and the reason
    /// [`Solver::extra_args`] exists: with default options cvc5 answers
    /// `unknown` on the engine's quantified problems, because its default
    /// quantifier strategy is E-matching and E-matching is incomplete. See
    /// [`crate::fol_model::cvc5`] for the model-block trap, which is worse.
    Cvc5,
    /// Mace4 over LADR. A dedicated finite model finder, from the
    /// unmaintained Prover9 distribution whose REFUTATION half is uncheckable
    /// and whose MODEL half is exactly what this layer certifies.
    Mace4,
}

impl Solver {
    pub fn parse(s: &str) -> anyhow::Result<Solver> {
        match s.to_ascii_lowercase().as_str() {
            "z3" | "smt" | "smtlib" => Ok(Solver::Z3),
            "cvc5" | "cvc" => Ok(Solver::Cvc5),
            "mace4" | "ladr" => Ok(Solver::Mace4),
            other => anyhow::bail!(
                "unknown model finder {other:?}; expected `z3`, `cvc5` or `mace4`"
            ),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Solver::Z3 => "z3",
            Solver::Cvc5 => "cvc5",
            Solver::Mace4 => "mace4",
        }
    }
    pub fn binary(self) -> &'static str {
        match self {
            Solver::Z3 => "z3",
            Solver::Cvc5 => "cvc5",
            Solver::Mace4 => "mace4",
        }
    }
    /// The line to print when the binary is missing. G6: a missing solver
    /// SKIPS LOUDLY with the install line and never passes silently.
    pub fn install_line(self) -> &'static str {
        match self {
            Solver::Z3 => "install Z3: `brew install z3` (macOS) or \
                           https://github.com/Z3Prover/z3/releases",
            // Pinned, and the version is in the sentence rather than left to
            // `latest`. The disagreement counts this repository publishes are a
            // property of a VERSION PAIR, so a reader who installs whatever is
            // current is not reproducing the measurement.
            Solver::Cvc5 => "install cvc5 1.3.4 from \
                             https://github.com/cvc5/cvc5/releases/tag/cvc5-1.3.4 \
                             (cvc5-macOS-arm64-static.zip or cvc5-Linux-x86_64-static.zip) \
                             and put the binary on PATH",
            Solver::Mace4 => "install Mace4, which ships in the LADR/Prover9 distribution: \
                              `brew install prover9` (macOS) or \
                              https://www.cs.unm.edu/~mccune/prover9/",
        }
    }
    /// Solver options beyond the problem file and the time limit.
    ///
    /// Only cvc5 has any, and the one it has is keyed on the ENCODING rather
    /// than on the caller's wishes, which is the whole care in this function.
    ///
    /// `--finite-model-find` on the FINITE encoding adds no assumption:
    /// `(declare-datatypes ((U 0)) ((e0) … ))` already fixes a finite carrier
    /// of known size, so "look for a finite model" is precisely the question
    /// the file asks, and without the flag cvc5 answers `unknown` on problems
    /// it can decide. On the UNBOUNDED encoding the flag would change the
    /// question, and an answer to a bounded question reported under an
    /// unbounded file's name is the `no_model_up_to_size_k` mistake of decision
    /// 0006 item 4 wearing a solver flag instead of a cardinality constraint.
    /// The unbounded probe is the ONLY route to `unsatisfiable_oracle`, so
    /// getting this wrong would corrupt the one verdict that must not be
    /// cheapened.
    pub fn extra_args(self, enc: SmtEncoding) -> Vec<String> {
        match (self, enc) {
            (Solver::Cvc5, SmtEncoding::Finite(_)) => vec!["--finite-model-find".to_string()],
            _ => Vec::new(),
        }
    }
    /// The smallest carrier the finder can be asked about.
    ///
    /// Mace4's is 2, not 1: `mace4 -n 1` is a FATAL error, measured on LADR
    /// 2009-11A (`assign_parm: parm start_size, value 1 out of range
    /// [2..2147483647]`). A one-element model is therefore reachable through
    /// Z3 and not through Mace4, and a ladder that started at 1 for both would
    /// turn that into a fatal run rather than a skipped size.
    pub fn min_domain(self) -> u32 {
        match self {
            Solver::Z3 | Solver::Cvc5 => 1,
            Solver::Mace4 => 2,
        }
    }
    /// Whether the finder can be asked about an unbounded carrier at all.
    /// Mace4 cannot: it is a finite model finder and every run of it is
    /// bounded, so it can never produce `unsatisfiable_oracle`.
    ///
    /// cvc5 can, and the probe runs it with NO extra options for the reason
    /// [`Solver::extra_args`] gives. Its `unknown` there is common and honest;
    /// an `unknown` is not evidence in either direction and the verdict stays
    /// whatever the bounded ladder established.
    pub fn can_probe_unbounded(self) -> bool {
        matches!(self, Solver::Z3 | Solver::Cvc5)
    }
    pub fn available(self) -> bool {
        which(self.binary()).is_some()
    }
}

fn which(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|d| d.join(bin))
        .find(|p| p.is_file())
}

/// How to run the pipeline.
#[derive(Clone, Debug)]
pub struct SolveOptions {
    pub solver: Solver,
    /// The largest carrier the ladder tries. Default 16, from the measured
    /// cost of the compiled checker: about 9M evaluation points per second and
    /// cubic in the carrier at quantifier depth 3, so 2000 formulas at carrier
    /// 16 is 8.2M points and 0.92s while carrier 32 is 65.5M and 6.8s.
    pub max_domain: u32,
    /// Seconds per solver invocation.
    pub timeout_secs: u32,
    /// After a bounded ladder finds nothing, ask the UNBOUNDED question too.
    /// This is the only route to `unsatisfiable_oracle` and it is off for
    /// Mace4, which has no unbounded question.
    pub unbounded_probe: bool,
    /// Path to `oo-folmodel`. `None` looks in `lean/.lake/build/bin/` and on
    /// `$PATH`, and reports loudly if it finds neither.
    pub checker: Option<PathBuf>,
}

impl Default for SolveOptions {
    fn default() -> Self {
        SolveOptions {
            solver: Solver::Z3,
            max_domain: 16,
            timeout_secs: 30,
            unbounded_probe: true,
            checker: None,
        }
    }
}

/// What a single solver invocation said.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolverSays {
    Sat,
    Unsat,
    Unknown,
}

impl SolverSays {
    pub fn name(self) -> &'static str {
        match self {
            SolverSays::Sat => "sat",
            SolverSays::Unsat => "unsat",
            SolverSays::Unknown => "unknown",
        }
    }
}

/// A disagreement between two things that were supposed to agree. Not a
/// verdict about the ontology, and not a footnote either.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Disagreement {
    pub severity: &'static str,
    pub what: &'static str,
    pub detail: String,
    pub means: &'static str,
}

/// The five fields decision 0006 fixes, plus what is needed to audit them.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Outcome {
    /// What the oracle said, on the run that decided the verdict.
    pub solver_verdict: &'static str,
    /// What it was asked. `unbounded` or `finite(k)`.
    pub encoding: String,
    /// `null` means the checker was never run.
    pub checker_exit: Option<i32>,
    /// The one word. See the module docstring for the three mechanical rules.
    /// A type, not a string: the certified variant carries the evidence.
    pub verdict: FolVerdict,
    /// Non-null only when the CHECKER reported a negated goal AND the verdict
    /// is `model_checked`. Both halves are in the type: minting one takes the
    /// checker's own report and the verdict's own `Certified`.
    pub owl_reading: Option<OwlReading>,

    /// The theorem the certified verdict names, when there is one.
    pub theorem: Option<String>,
    /// The sizes the ladder tried, in order.
    pub cardinality_search: Vec<u32>,
    /// What the bounded ladder established, separately from the verdict, so
    /// that an unbounded probe's answer cannot erase it.
    pub bounded_search: String,
    /// The unbounded probe's answer, when it ran. This is the ONLY field the
    /// `unsatisfiable_oracle` verdict is allowed to be computed from.
    pub unbounded: Option<&'static str>,
    /// `oo-folmodel`'s own JSON, verbatim, when it ran.
    pub checker_report: Option<String>,
    /// Symbols the solver interpreted that the translation never emitted, so
    /// the checked structure is a reduct of the solver's.
    pub dropped_symbols: Vec<String>,
    pub problem_digest: String,
    pub formulas: usize,
    /// Set when a checker and the thing it checks disagreed. Stop the line.
    pub disagreement: Option<Disagreement>,
    /// Set when the run could not happen at all. G6: loud, never silent.
    pub skipped: Option<String>,
    pub seconds: f64,
}

impl Outcome {
    /// A run that did not happen, with the reason. `reason` is the whole
    /// sentence: the caller appends the install line where there is one,
    /// because a missing CHECKER has nothing to do with the solver's.
    fn skip(reason: String) -> Outcome {
        Outcome {
            solver_verdict: "unknown",
            encoding: "none".into(),
            checker_exit: None,
            verdict: FolVerdict::UnknownOracle,
            owl_reading: None,
            theorem: None,
            cardinality_search: Vec::new(),
            bounded_search: "not run".into(),
            unbounded: None,
            checker_report: None,
            dropped_symbols: Vec::new(),
            problem_digest: String::new(),
            formulas: 0,
            disagreement: None,
            skipped: Some(reason),
            seconds: 0.0,
        }
    }
}

/// Find `oo-folmodel`.
///
/// Its absence is reported rather than worked around. A pipeline that quietly
/// dropped to an oracle verdict because the checker was not built would make
/// every run look like a solver limitation, which is the precise shape of
/// mis-attribution this repository exists to catch.
pub fn find_checker(explicit: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(p) = explicit {
        return if p.is_file() {
            Ok(p.to_path_buf())
        } else {
            Err(format!("no oo-folmodel at {}", p.display()))
        };
    }
    if let Ok(v) = std::env::var("OO_FOLMODEL") {
        let p = PathBuf::from(v);
        if p.is_file() {
            return Ok(p);
        }
    }
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("lean/.lake/build/bin/oo-folmodel");
    if here.is_file() {
        return Ok(here);
    }
    if let Some(p) = which("oo-folmodel") {
        return Ok(p);
    }
    Err(format!(
        "oo-folmodel, the verified checker. Build it with `cd lean && lake build` (it is in \
         defaultTargets, so a bare `lake build` is enough), or set OO_FOLMODEL. Looked in {} \
         and on PATH",
        here.display()
    ))
}

/// Run a command with a wall-clock limit, capturing stdout+stderr to a file.
///
/// Output goes to a FILE and not a pipe: a solver that fills a pipe while
/// nobody reads it deadlocks, and Mace4 is chatty.
fn run_limited(
    mut cmd: Command,
    out_path: &Path,
    limit: Duration,
) -> std::io::Result<(Option<i32>, String, bool)> {
    let f = std::fs::File::create(out_path)?;
    let f2 = f.try_clone()?;
    cmd.stdout(Stdio::from(f)).stderr(Stdio::from(f2)).stdin(Stdio::null());
    let mut child = cmd.spawn()?;
    let start = Instant::now();
    let mut timed_out = false;
    let code = loop {
        match child.try_wait()? {
            Some(status) => break status.code(),
            None => {
                if start.elapsed() > limit {
                    let _ = child.kill();
                    let _ = child.wait();
                    timed_out = true;
                    break None;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    };
    let mut text = String::new();
    std::fs::File::open(out_path)?.read_to_string(&mut text).ok();
    Ok((code, text, timed_out))
}

fn read_sat(text: &str) -> SolverSays {
    // The FIRST standalone status line. Z3 prints `sat` / `unsat` / `unknown`
    // on its own line before the model, and `(error …)` before that if the
    // file was bad. Matching a bare substring would find the `unsat` inside
    // `unsatisfiable` in a comment, which is why this is line-oriented.
    for line in text.lines() {
        match line.trim() {
            "sat" => return SolverSays::Sat,
            "unsat" => return SolverSays::Unsat,
            "unknown" => return SolverSays::Unknown,
            _ => {}
        }
    }
    SolverSays::Unknown
}

/// Mace4's exit codes, measured on LADR 2009-11A on this machine:
/// 0 a model was found (`exit (max_models)`), 1 a FATAL error (including
/// `-n 1`, whose carrier is below its minimum of 2), 2 the search was
/// exhausted with no model (`exit (exhausted)`). Anything else is a resource
/// limit and is `unknown`, never `unsat`.
fn read_mace4(code: Option<i32>, text: &str, timed_out: bool) -> (SolverSays, Option<String>) {
    if timed_out {
        return (SolverSays::Unknown, None);
    }
    match code {
        Some(0) => (SolverSays::Sat, None),
        Some(2) => (SolverSays::Unsat, None),
        Some(1) => (
            SolverSays::Unknown,
            Some(format!(
                "mace4 exited 1 (fatal). It said: {}",
                text.lines()
                    .filter(|l| l.contains("Fatal") || l.contains("error"))
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(" / ")
            )),
        ),
        other => (
            SolverSays::Unknown,
            Some(format!("mace4 exited {other:?}, which is a resource limit, not a verdict")),
        ),
    }
}

/// One bounded attempt at one carrier size.
struct Attempt {
    says: SolverSays,
    model: Option<Result<FiniteModel, IngestError>>,
    note: Option<String>,
}

#[allow(clippy::too_many_arguments)]
fn attempt(
    problem: &FolProblem,
    opts: &SolveOptions,
    dir: &Path,
    k: u32,
    table: Option<&ladr::SymbolTable>,
) -> anyhow::Result<Attempt> {
    let limit = Duration::from_secs(opts.timeout_secs.max(1) as u64);
    match opts.solver {
        // ONE file for both SMT solvers, written by ONE emitter. The file name
        // carries no solver in it for that reason: if the two were ever handed
        // different bytes, a disagreement between them would be a fact about
        // this function rather than about either solver, and that is the one
        // thing a differential must not be able to measure.
        Solver::Z3 | Solver::Cvc5 => {
            let enc = SmtEncoding::Finite(k);
            let file = dir.join(format!("problem_k{k}.smt2"));
            std::fs::write(&file, problem.to_smtlib(enc)?)?;
            let mut cmd = Command::new(opts.solver.binary());
            match opts.solver {
                Solver::Cvc5 => {
                    cmd.arg(format!("--tlimit={}", opts.timeout_secs.max(1) as u64 * 1000));
                }
                _ => {
                    cmd.arg(format!("-T:{}", opts.timeout_secs.max(1)));
                }
            }
            cmd.args(opts.solver.extra_args(enc)).arg(&file);
            let (_, text, timed_out) = run_limited(
                cmd,
                &dir.join(format!("{}_k{k}.out", opts.solver.name())),
                limit,
            )?;
            let says = if timed_out { SolverSays::Unknown } else { read_sat(&text) };
            // Ingest ONLY on `sat`, and for cvc5 that is load-bearing rather
            // than tidy: it prints a `(get-model)` block after `unknown` too,
            // and that block can falsify an asserted axiom. See
            // `crate::fol_model::cvc5` for the measurement.
            let model = match says {
                SolverSays::Sat => Some(match opts.solver {
                    Solver::Cvc5 => cvc5::parse_model(&text, &problem.vocabulary(), k as usize),
                    _ => z3::parse_model(&text, &problem.vocabulary(), k as usize),
                }),
                _ => None,
            };
            Ok(Attempt { says, model, note: None })
        }
        Solver::Mace4 => {
            let tab = table.expect("mace4 needs a symbol table");
            let file = dir.join("problem.in");
            if !file.exists() {
                std::fs::write(&file, ladr::problem(problem, tab, opts.timeout_secs.max(1))?)?;
            }
            let mut cmd = Command::new("mace4");
            cmd.arg("-n").arg(k.to_string())
                .arg("-N").arg(k.to_string())
                .arg("-t").arg(opts.timeout_secs.max(1).to_string())
                .arg("-f").arg(&file);
            let (code, text, timed_out) =
                run_limited(cmd, &dir.join(format!("mace4_k{k}.out")), limit)?;
            let (says, note) = read_mace4(code, &text, timed_out);
            let model = match says {
                SolverSays::Sat => Some(mace4::parse_model(&text, tab)),
                _ => None,
            };
            Ok(Attempt { says, model, note })
        }
    }
}

/// Export, solve, ingest, check, and report.
///
/// `dir` receives every intermediate file, so a run is reproducible by hand
/// from what it left behind: the problem in the checker's format, the problem
/// in the solver's, the solver's raw output, the model in the checker's
/// format, and the checker's own JSON.
pub fn solve(problem: &FolProblem, opts: &SolveOptions, dir: &Path) -> anyhow::Result<Outcome> {
    let started = Instant::now();
    std::fs::create_dir_all(dir)?;

    if !opts.solver.available() {
        // G6. Loud, with the install line, and never a silent pass. The
        // distinctive marker is the same one tests/common/mod.rs uses.
        let msg = format!(
            "missing solver: {} ({})",
            opts.solver.binary(),
            opts.solver.install_line()
        );
        eprintln!("SKIPPED_SOLVER: {msg}");
        return Ok(Outcome::skip(msg));
    }
    let checker = match find_checker(opts.checker.as_deref()) {
        Ok(p) => p,
        Err(why) => {
            eprintln!("SKIPPED_CHECKER: {why}");
            return Ok(Outcome::skip(format!("missing checker: {why}")));
        }
    };

    let (problem_tsv, digest) = problem.to_problem_tsv()?;
    std::fs::write(dir.join("problem.tsv"), &problem_tsv)?;
    let entries = problem.checker_entries();
    let table = match opts.solver {
        Solver::Mace4 => {
            let t = ladr::SymbolTable::build(problem)?;
            std::fs::write(dir.join("symbols.tsv"), ladr::table_tsv(&t))?;
            Some(t)
        }
        // Neither SMT solver needs a symbol table: SMT-LIB has `|…|` quoting,
        // so an IRI is written as itself and read back as itself. The table
        // exists for LADR, which has no quoting construct at all.
        Solver::Z3 | Solver::Cvc5 => None,
    };

    let mut out = Outcome {
        solver_verdict: "unknown",
        encoding: "none".into(),
        checker_exit: None,
        verdict: FolVerdict::UnknownOracle,
        owl_reading: None,
        theorem: None,
        cardinality_search: Vec::new(),
        bounded_search: String::new(),
        unbounded: None,
        checker_report: None,
        dropped_symbols: Vec::new(),
        problem_digest: digest.clone(),
        formulas: entries.len(),
        disagreement: None,
        skipped: None,
        seconds: 0.0,
    };

    // ── The bounded ladder ──────────────────────────────────────────────
    let lo = opts.solver.min_domain();
    let hi = opts.max_domain.max(lo);
    let mut last_unknown: Option<String> = None;
    for k in lo..=hi {
        out.cardinality_search.push(k);
        let a = attempt(problem, opts, dir, k, table.as_ref())?;
        match a.says {
            SolverSays::Unsat => continue,
            SolverSays::Unknown => {
                last_unknown = a.note.or_else(|| {
                    Some(format!("{} gave no verdict at carrier {k}", opts.solver.name()))
                });
                break;
            }
            SolverSays::Sat => {
                out.solver_verdict = "sat";
                out.encoding = SmtEncoding::Finite(k).name();
                out.bounded_search = format!("a model was reported at carrier {k}");
                let model = match a.model.expect("sat carries a model attempt") {
                    Ok(m) => m,
                    Err(e) => {
                        // The solver said sat and we could not read what it
                        // built. Not a fact about the ontology, and not quiet.
                        out.verdict = FolVerdict::SatisfiableOracle;
                        out.disagreement = Some(Disagreement {
                            severity: "STOP_THE_LINE",
                            what: "model_not_ingested",
                            detail: e.to_string(),
                            means: "the solver answered sat and this pipeline could not turn \
                                    what it printed into a structure, so NOTHING was checked. \
                                    Either the ingestion is wrong or the solver printed \
                                    something it has not printed before. This is a defect in \
                                    the pipeline until shown otherwise, and it is not a \
                                    statement about the ontology",
                        });
                        out.seconds = started.elapsed().as_secs_f64();
                        return Ok(out);
                    }
                };
                out.dropped_symbols = model.dropped.clone();
                if let Err(e) = model.covers(&problem.vocabulary(), opts.solver.name()) {
                    out.verdict = FolVerdict::SatisfiableOracle;
                    out.disagreement = Some(Disagreement {
                        severity: "STOP_THE_LINE",
                        what: "model_incomplete",
                        detail: e.to_string(),
                        means: "the solver answered sat and left a symbol of the problem \
                                uninterpreted. The checker's own coverage gate would report \
                                this as an ATTRIBUTION failure; it is caught here so the \
                                message can name the solver. Nothing was certified",
                    });
                    out.seconds = started.elapsed().as_secs_f64();
                    return Ok(out);
                }
                let model_tsv = model.to_model_tsv(&digest, &out.cardinality_search);
                let model_path = dir.join("model.tsv");
                std::fs::write(&model_path, &model_tsv)?;

                // ── The only place the certified word can be reached ──
                //
                // It is no longer "the only place" by inspection. `CheckerRun`
                // is the only constructor of the evidence `FolVerdict::
                // ModelChecked` requires, so this is the only place it CAN be
                // reached, and a second one would have to run a checker too.
                let mut cmd = Command::new(&checker);
                cmd.arg(dir.join("problem.tsv")).arg(&model_path);
                let run = CheckerRun::spawn(&CheckerBinary::found_at(checker.clone()), cmd)?;
                let report = run.stdout().trim().to_string();
                std::fs::write(dir.join("checker.json"), &report)?;
                let exit = run.exit();
                out.checker_exit = Some(exit);
                out.checker_report = Some(report.clone());
                if let Some(cert) = run.accepted(FOL_THEOREM) {
                    out.verdict = FolVerdict::ModelChecked(cert);
                    out.theorem = Some(cert.theorem().to_string());
                    // Read the goal flag back off the CHECKER's report, not
                    // off this side's intention. The `cert` argument is the
                    // "and the verdict is model_checked" half of the rule,
                    // carried by the signature instead of by this comment.
                    out.owl_reading = run.owl_reading(cert);
                } else {
                    out.verdict = FolVerdict::SatisfiableOracle;
                    out.disagreement = Some(Disagreement {
                        severity: "STOP_THE_LINE",
                        what: "model_not_confirmed",
                        detail: format!(
                            "oo-folmodel exited {exit} on the structure {} reported. It said: \
                             {report}",
                            opts.solver.name()
                        ),
                        means: "a solver answered sat and the VERIFIED checker rejected the \
                                structure it produced. One of the solver, the ingestion and \
                                the two printers is wrong, and this is not a statement about \
                                the ontology: the verdict stays satisfiable_oracle and is \
                                never `rejected`. Treat it the way shacl_differential.py \
                                treats a false clean",
                    });
                }
                out.seconds = started.elapsed().as_secs_f64();
                return Ok(out);
            }
        }
    }

    // ── The ladder found nothing ────────────────────────────────────────
    match &last_unknown {
        Some(why) => {
            out.solver_verdict = "unknown";
            out.encoding = SmtEncoding::Finite(*out.cardinality_search.last().unwrap_or(&hi)).name();
            out.bounded_search = format!("stopped: {why}");
            out.verdict = FolVerdict::UnknownOracle;
        }
        None => {
            out.solver_verdict = "unsat";
            out.encoding = SmtEncoding::Finite(hi).name();
            out.bounded_search = format!("exhausted: no model of size {lo}..{hi}");
            // NOT unsatisfiability. Decision 0006 item 4.
            out.verdict = FolVerdict::NoModelUpToSizeK;
        }
    }

    // ── The unbounded probe, the only route to unsatisfiable_oracle ─────
    if opts.unbounded_probe && opts.solver.can_probe_unbounded() && last_unknown.is_none() {
        let file = dir.join("problem_unbounded.smt2");
        std::fs::write(&file, problem.to_smtlib(SmtEncoding::Unbounded)?)?;
        let mut cmd = Command::new(opts.solver.binary());
        match opts.solver {
            Solver::Cvc5 => {
                cmd.arg(format!("--tlimit={}", opts.timeout_secs.max(1) as u64 * 1000));
            }
            _ => {
                cmd.arg(format!("-T:{}", opts.timeout_secs.max(1)));
            }
        }
        // No `extra_args` here by construction rather than by omission:
        // `Solver::extra_args` returns nothing for the unbounded encoding, and
        // the reason is in its docstring. Passing --finite-model-find on this
        // file would ask a bounded question and record the answer under the
        // unbounded encoding's name, which is the one substitution that could
        // turn `no_model_up_to_size_k` into `unsatisfiable_oracle` by accident.
        cmd.args(opts.solver.extra_args(SmtEncoding::Unbounded)).arg(&file);
        let (_, text, timed_out) = run_limited(
            cmd,
            &dir.join(format!("{}_unbounded.out", opts.solver.name())),
            Duration::from_secs(opts.timeout_secs.max(1) as u64),
        )?;
        let says = if timed_out { SolverSays::Unknown } else { read_sat(&text) };
        out.unbounded = Some(says.name());
        match says {
            SolverSays::Unsat => {
                out.solver_verdict = "unsat";
                out.encoding = SmtEncoding::Unbounded.name();
                out.verdict = FolVerdict::UnsatisfiableOracle;
            }
            SolverSays::Sat => {
                out.solver_verdict = "sat";
                out.encoding = SmtEncoding::Unbounded.name();
                // Satisfiable, and with no finite model small enough to
                // certify. Exactly the SHIQ finite-model-property case.
                out.verdict = FolVerdict::SatisfiableOracle;
            }
            SolverSays::Unknown => {}
        }
    }

    out.seconds = started.elapsed().as_secs_f64();
    Ok(out)
}

/// The JSON one run reports, with the vocabulary written out beside the
/// verdict so that a reader who has never opened decision 0006 still cannot
/// mistake an oracle's word for a certificate's.
pub fn outcome_json(o: &Outcome) -> serde_json::Value {
    serde_json::json!({
        "solver_verdict": o.solver_verdict,
        "encoding": o.encoding,
        "checker_exit": o.checker_exit,
        "verdict": o.verdict,
        "owl_reading": o.owl_reading,
        "theorem": o.theorem,
        "formulas": o.formulas,
        "problem_digest": o.problem_digest,
        "cardinality_search": o.cardinality_search,
        "bounded_search": o.bounded_search,
        "unbounded_probe": o.unbounded,
        "dropped_symbols": o.dropped_symbols,
        "checker_report": o.checker_report,
        "disagreement": o.disagreement,
        "skipped": o.skipped,
        "seconds": (o.seconds * 1000.0).round() / 1000.0,
        "verdict_means": verdict_means(o.verdict.word()),
    })
}

/// One sentence per word, so a report is readable without the decision record
/// open beside it.
pub fn verdict_means(v: &str) -> &'static str {
    match v {
        "model_checked" => "CERTIFIED. A finite structure was checked against the problem by \
                            lean/Fol/, whose soundness is machine-checked: \
                            Fol.satisfiable_of_check turns an accepted structure into \
                            Satisfiable. This says nothing about unsatisfiability in any \
                            direction",
        "satisfiable_oracle" => "a solver said satisfiable and no structure was certified. \
                                 Trust the solver or do not; nothing here checked it",
        "no_model_up_to_size_k" => "a bounded search was exhausted. This is NOT \
                                    unsatisfiability: a theory with no model of size k can \
                                    have one of size k+1, and SHIQ has no finite model \
                                    property at all, so a satisfiable ontology can have only \
                                    infinite models",
        "unsatisfiable_oracle" => "a solver said unsatisfiable on the UNBOUNDED encoding. It \
                                   can never be more than an oracle opinion: checking a \
                                   refutation needs a verified first-order calculus with \
                                   unification, which does not exist in core Lean \
                                   (decision 0005)",
        "unknown_oracle" => "the solver timed out, gave up, or is incomplete on this problem. \
                             Not evidence in either direction",
        _ => "unrecognised verdict",
    }
}

/// Run the pipeline over the loaded ontology and, optionally, one problem per
/// goal. Returns the report JSON.
pub fn solve_export(
    graph: &std::sync::Arc<crate::graph::GraphStore>,
    dir: &Path,
    opts: &SolveOptions,
    goals: Option<&Path>,
    goals_skip_columns: usize,
) -> anyhow::Result<String> {
    let triples = graph.all_triples()?;
    let read = crate::tptp::read_graph(triples);
    std::fs::create_dir_all(dir)?;

    let base = FolProblem::build(&read.axioms, None)?;
    let ontology = solve(&base, opts, &dir.join("ontology"))?;

    let mut goal_reports = Vec::new();
    // A goal the fragment cannot express is NAMED, not skipped. `tptp::export`
    // already reports its `not_asked` list for the same reason: a run that
    // silently asked fewer questions than the file contained would report a
    // clean sweep over a subset nobody chose, which is the shape decision 0005
    // item 7 records happening in this repository already.
    let mut not_asked: Vec<serde_json::Value> = Vec::new();
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    *counts.entry(ontology.verdict.word()).or_default() += 1;
    let mut stop_the_line = usize::from(ontology.disagreement.is_some());

    if let Some(path) = goals {
        let text = std::fs::read_to_string(path)?;
        for (i, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() < goals_skip_columns + 3 {
                not_asked.push(serde_json::json!({
                    "line": line,
                    "why": format!(
                        "fewer than {} tab-separated columns",
                        goals_skip_columns + 3
                    ),
                }));
                continue;
            }
            let (s, p, o) = (
                cols[goals_skip_columns],
                cols[goals_skip_columns + 1],
                cols[goals_skip_columns + 2],
            );
            let ax = match crate::tptp::triple_as_axiom(&read, s, p, o) {
                Ok(ax) => ax,
                Err(why) => {
                    not_asked.push(serde_json::json!({
                        "triple": [s, p, o],
                        "why": why,
                    }));
                    continue;
                }
            };
            let gp = FolProblem::build(&read.axioms, Some(&ax))?;
            let g = solve(&gp, opts, &dir.join(format!("goal_{i:05}")))?;
            *counts.entry(g.verdict.word()).or_default() += 1;
            if g.disagreement.is_some() {
                stop_the_line += 1;
            }
            goal_reports.push(serde_json::json!({
                "triple": [s, p, o],
                "outcome": outcome_json(&g),
            }));
        }
    }

    Ok(serde_json::json!({
        "solver": opts.solver.name(),
        "max_domain": opts.max_domain,
        "timeout_secs": opts.timeout_secs,
        "dir": dir.display().to_string(),
        "ontology": outcome_json(&ontology),
        "goals": goal_reports,
        "goals_not_asked": not_asked.len(),
        "not_asked": not_asked,
        "not_asked_means": "no axiom form in OwlLean/Syntax.lean corresponds to the triple, so \
                            the question could not be put at all. NOT evidence about the \
                            ontology in either direction, and listed rather than skipped so a \
                            run cannot report a clean sweep over a subset nobody chose",
        "verdict_counts": counts,
        "verdict_counts_include_the_ontology_itself": true,
        "stop_the_line": stop_the_line,
        "stop_the_line_means": "a solver answered sat and the VERIFIED checker rejected, or \
                                could not be given, the structure it produced. The command \
                                exits non-zero on any of these, the way \
                                tools/shacl_differential.py exits non-zero on a FALSE_CLEAN. \
                                It is a bug in the solver, the ingestion or the printers, and \
                                it is NOT a statement about the ontology",
        "not_certified": "only the verdict `model_checked` rests on a machine-checked theorem. \
                          `unsatisfiable_oracle` is a solver's opinion and can never be more, \
                          because checking a refutation needs a verified first-order calculus \
                          with unification that does not exist in core Lean. \
                          `no_model_up_to_size_k` is a statement about a bounded search and \
                          not about satisfiability",
        "owl_reading_means": "the OWL-level sentence rides on OwlLean.adequacy in the sibling \
                              owl-lean project AND on the Rust-to-Lean translation \
                              correspondence, which decision 0005 item 2 states is PINNED BY \
                              TESTS AND NOT PROVED. That is why it carries its own word and \
                              must never be shortened to `not entailed`",
    })
    .to_string())
}
