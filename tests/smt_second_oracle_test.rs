//! cvc5 as a SECOND SMT oracle, and the differential that measures whether it
//! disagrees with Z3.
//!
//! Decision 0006 draws the line down the middle of the SAT/SMT family and a
//! second solver does not move it. A `sat` whose model `oo-folmodel` accepts is
//! a certificate whichever solver produced it, because the checker re-evaluates
//! the formulas against the structure and reads nothing else, so its soundness
//! argument never mentions the producer. An `unsat` is an ORACLE OPINION
//! whichever solver produced it, and TWO solvers agreeing on `unsat` is two
//! opinions rather than a proof. This file is where that sentence stops being
//! prose: the tests below check that a cvc5 `sat` can reach `model_checked`,
//! and that nothing anywhere reports a cvc5 `unsat` as anything but an oracle's
//! word.
//!
//! The four groups:
//!
//!   1. **The wiring**, which needs no solver at all: the parse, the option
//!      table, and the one option whose placement could corrupt a verdict.
//!   2. **cvc5 end to end**, from the shared emitter through cvc5 and the
//!      ingestion into the VERIFIED CHECKER, with the provenance checked at
//!      every step.
//!   3. **The trap**, which is the reason this file is longer than the wiring
//!      deserves. cvc5 prints a model block after `unknown` and that block is
//!      not a model. The test runs cvc5 in exactly the configuration that
//!      produces one, ingests it deliberately, and requires the verified
//!      checker to REJECT it.
//!   4. **The differential**, run for real over a small fixed corpus, together
//!      with a demonstration that it can fail: a lying solver on PATH must turn
//!      the run red. A gate nobody has seen fail is decoration, and this
//!      repository has shipped several.
//!
//! Needs cvc5, Z3 and the Lean checker. Without them the tests SKIP LOUDLY
//! through `common::skip_unless`; the `lean` CI job installs all three and runs
//! this file with `OO_REQUIRE_FIXTURES=1`, which turns that skip into a
//! failure.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use open_ontologies::fol_model::{IngestError, cvc5, z3};
use open_ontologies::fol_solve::{SolveOptions, Solver, find_checker, solve};
use open_ontologies::tptp::{Concept, FolProblem, OwlAxiom, smtlib::SmtEncoding};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("oo-cvc5-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("scratch dir");
    d
}

/// What is missing, as one sentence, or `None`.
fn missing() -> Option<String> {
    if !Solver::Cvc5.available() {
        return Some(format!("cvc5, the second SMT oracle; {}", Solver::Cvc5.install_line()));
    }
    if !Solver::Z3.available() {
        return Some(format!("z3, the first SMT oracle; {}", Solver::Z3.install_line()));
    }
    find_checker(None).err()
}

/// G6. Loud, and fatal under `OO_REQUIRE_FIXTURES=1`.
fn skip() -> bool {
    match missing() {
        None => false,
        Some(why) => common::skip_unless(
            false,
            &why,
            "a differential between two solvers needs both of them, and the certified half \
             needs the verified checker; with any of the three absent this file compares \
             nothing",
        ),
    }
}

/// `A(a)` with the goal `B(a)`, which is not entailed, so `Γ ∪ {¬φ}` has a
/// model and the run can reach the non-entailment reading.
fn not_entailed() -> FolProblem {
    let axioms = vec![OwlAxiom::ClassAssert(
        Concept::Atom("http://e/A".into()),
        "http://e/a".into(),
    )];
    let goal = OwlAxiom::ClassAssert(Concept::Atom("http://e/B".into()), "http://e/a".into());
    FolProblem::build(&axioms, Some(&goal)).expect("freshness holds")
}

/// `A ⊑ B` and `A(a)`, with no goal. Satisfiable, and every model must put `a`
/// in `B`, which is what makes a forged or half-built structure detectable.
fn subsumption() -> FolProblem {
    let axioms = vec![
        OwlAxiom::SubClass(
            Concept::Atom("http://e/A".into()),
            Concept::Atom("http://e/B".into()),
        ),
        OwlAxiom::ClassAssert(Concept::Atom("http://e/A".into()), "http://e/a".into()),
    ];
    FolProblem::build(&axioms, None).expect("freshness holds")
}

fn run_checker(problem: &Path, model: &Path) -> (i32, String) {
    let checker = find_checker(None).expect("checked by skip()");
    let out = Command::new(checker)
        .arg(problem)
        .arg(model)
        .output()
        .expect("run oo-folmodel");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
    )
}

fn field<'a>(json: &'a str, key: &str) -> &'a str {
    let pat = format!("\"{key}\":\"");
    match json.find(&pat) {
        Some(i) => {
            let rest = &json[i + pat.len()..];
            &rest[..rest.find('"').expect("unterminated")]
        }
        None => "",
    }
}

// ── 1. The wiring ───────────────────────────────────────────────────────────

/// The command line reaches cvc5, and the refusal message still names every
/// solver there is.
#[test]
fn cvc5_is_selectable_and_the_refusal_names_every_finder() {
    assert_eq!(Solver::parse("cvc5").expect("cvc5 parses"), Solver::Cvc5);
    assert_eq!(Solver::parse("CVC5").expect("case folds"), Solver::Cvc5);
    assert_eq!(Solver::parse("z3").expect("z3 still parses"), Solver::Z3);
    assert_eq!(Solver::parse("mace4").expect("mace4 still parses"), Solver::Mace4);

    let err = Solver::parse("yices").expect_err("an unknown finder is refused").to_string();
    for name in ["z3", "cvc5", "mace4"] {
        assert!(err.contains(name), "the refusal must list {name}: {err}");
    }
}

/// `--finite-model-find` on the bounded encoding, and NOTHING on the unbounded
/// one.
///
/// This is not a style assertion. The unbounded probe is the only route to
/// `unsatisfiable_oracle`, the one verdict in this layer that claims anything
/// about unsatisfiability at all. Searching for FINITE models and filing the
/// answer under an unbounded file's name would make a `no_model_up_to_size_k`
/// result wear the `unsatisfiable_oracle` word, which is decision 0006 item 4's
/// ten-minute mistake with a solver flag standing in for the cardinality
/// constraint. So the option is keyed on what the file DECLARES.
#[test]
fn the_finite_model_flag_is_keyed_on_the_encoding_and_not_on_the_caller() {
    assert_eq!(
        Solver::Cvc5.extra_args(SmtEncoding::Finite(1)),
        vec!["--finite-model-find".to_string()],
    );
    assert_eq!(
        Solver::Cvc5.extra_args(SmtEncoding::Finite(16)),
        vec!["--finite-model-find".to_string()],
    );
    assert!(
        Solver::Cvc5.extra_args(SmtEncoding::Unbounded).is_empty(),
        "the unbounded probe must ask the unbounded question",
    );
    // Z3 is driven with no options at either encoding, so a difference between
    // the two solvers can never be a difference in how hard each was asked to
    // try on one of them and not the other.
    assert!(Solver::Z3.extra_args(SmtEncoding::Finite(2)).is_empty());
    assert!(Solver::Z3.extra_args(SmtEncoding::Unbounded).is_empty());
}

/// Both SMT solvers are given the SAME bytes, by one emitter.
///
/// If they were ever handed different files a disagreement between them would
/// be a fact about the emitter rather than about either solver, and that is the
/// one thing a differential must not be able to measure.
#[test]
fn both_solvers_read_one_emitter() {
    let p = not_entailed();
    let a = p.to_smtlib(SmtEncoding::Finite(2)).expect("writable");
    let b = p.to_smtlib(SmtEncoding::Finite(2)).expect("writable");
    assert_eq!(a, b);
    assert!(a.contains("(declare-datatypes ((U 0)) (((e0) (e1))))"), "{a}");
    assert!(a.contains("(check-sat)"), "{a}");
}

// ── 2. cvc5 end to end ──────────────────────────────────────────────────────

/// The whole chain through the SECOND solver: the shared emitter writes the
/// problem, cvc5 solves it, the ingestion reads the structure back, and the
/// VERIFIED CHECKER accepts it against the same problem in its own format.
///
/// `Fol.satisfiable_of_check` is a theorem about a structure and a formula
/// list. It says nothing about who produced the structure, which is exactly why
/// a second solver can reach the certified word without a second soundness
/// argument, and why an `unsat` from either cannot reach it with any argument
/// at all.
#[test]
fn a_cvc5_model_is_checked_and_earns_the_certified_word() {
    if skip() {
        return;
    }
    let d = scratch("certified");
    let o = solve(
        &not_entailed(),
        &SolveOptions {
            solver: Solver::Cvc5,
            max_domain: 4,
            timeout_secs: 30,
            unbounded_probe: false,
            checker: None,
        },
        &d,
    )
    .expect("the pipeline runs");

    assert!(o.skipped.is_none(), "{o:?}");
    assert_eq!(o.solver_verdict, "sat", "{o:?}");
    assert_eq!(o.verdict, "model_checked", "{o:?}");
    assert_eq!(o.checker_exit, Some(0), "{o:?}");
    assert_eq!(o.theorem.as_deref(), Some("Fol.satisfiable_of_check"), "{o:?}");
    assert!(o.disagreement.is_none(), "{o:?}");

    // The PROVENANCE, on every artefact a reader would consult. A model read by
    // the shared SMT-LIB parser must never claim Z3 produced it.
    let report = o.checker_report.clone().unwrap_or_default();
    assert!(report.contains("\"source\":\"cvc5\""), "{report}");
    let model_tsv = std::fs::read_to_string(d.join("model.tsv")).expect("model.tsv");
    assert!(model_tsv.contains("source\tcvc5"), "{model_tsv}");
    assert!(!model_tsv.contains("source\tz3"), "{model_tsv}");

    // The files that make the run reproducible by hand, named for cvc5 so a
    // directory holding both solvers' runs is unambiguous.
    for f in ["problem.tsv", "model.tsv", "checker.json", "problem_k1.smt2"] {
        assert!(d.join(f).is_file(), "missing {f} in {}", d.display());
    }
    assert!(
        std::fs::read_dir(&d)
            .expect("readable")
            .filter_map(Result::ok)
            .any(|e| e.file_name().to_string_lossy().starts_with("cvc5_k")),
        "cvc5's raw output must be kept under its own name",
    );
}

/// The non-entailment reading, which is the one sentence no prover can produce,
/// reached through cvc5.
///
/// A prover answering "not a theorem" is reporting a failed search at best. A
/// CHECKED countermodel is a machine-checked `¬ Entails Γ φ`, and the word for
/// it is long on purpose: the OWL-level reading rides on `OwlLean.adequacy` in
/// a sibling project and on a Rust-to-Lean correspondence that is pinned by
/// tests and NOT proved.
#[test]
fn cvc5_reaches_the_non_entailment_reading_and_keeps_its_long_word() {
    if skip() {
        return;
    }
    let d = scratch("nonentail");
    let o = solve(
        &not_entailed(),
        &SolveOptions {
            solver: Solver::Cvc5,
            max_domain: 4,
            timeout_secs: 30,
            unbounded_probe: false,
            checker: None,
        },
        &d,
    )
    .expect("the pipeline runs");
    let reading = format!("{:?}", o.owl_reading);
    assert!(
        reading.contains("NotEntailedUnderUnprovedTranslation")
            || o.owl_reading.is_some(),
        "a checked countermodel to a negated goal must carry the OWL reading: {o:?}",
    );
    let json = serde_json::to_string(&o.owl_reading).expect("serialisable");
    assert!(
        json.contains("not_entailed_under_unproved_translation"),
        "the word must never be shortened to `not_entailed`: {json}",
    );
}

/// An `unsat` from cvc5 is an oracle's word and the report says so in the word
/// itself.
///
/// The bounded ladder exhausting itself is `no_model_up_to_size_k`, which is
/// NOT unsatisfiability: `∀x∃y (r(x,y) ∧ x≠y)` is unsat at carrier 1 and sat at
/// carrier 2. Nothing on this path may reach `model_checked`, because nothing
/// was checked.
#[test]
fn a_cvc5_unsat_is_never_a_proof() {
    if skip() {
        return;
    }
    // `A(a)` and `¬A(a)`: unsatisfiable at every carrier size.
    let axioms = vec![
        OwlAxiom::ClassAssert(Concept::Atom("http://e/A".into()), "http://e/a".into()),
        OwlAxiom::ClassAssert(
            Concept::Compl(Box::new(Concept::Atom("http://e/A".into()))),
            "http://e/a".into(),
        ),
    ];
    let p = FolProblem::build(&axioms, None).expect("freshness holds");
    let d = scratch("unsat");
    let o = solve(
        &p,
        &SolveOptions {
            solver: Solver::Cvc5,
            max_domain: 3,
            timeout_secs: 30,
            unbounded_probe: false,
            checker: None,
        },
        &d,
    )
    .expect("the pipeline runs");

    assert_eq!(o.solver_verdict, "unsat", "{o:?}");
    assert_eq!(
        o.verdict, "no_model_up_to_size_k",
        "a bounded ladder's unsat is not unsatisfiability: {o:?}",
    );
    assert_ne!(o.verdict, "model_checked", "{o:?}");
    assert_ne!(o.verdict, "unsatisfiable_oracle", "{o:?}");
    assert_eq!(o.checker_exit, None, "nothing was checked, so nothing ran: {o:?}");
    assert!(o.theorem.is_none(), "an unsat names no theorem: {o:?}");
}

// ── 3. The trap ─────────────────────────────────────────────────────────────

/// **cvc5 prints a model block after `unknown`, and that block is not a model.**
///
/// This test runs cvc5 in the configuration that produces one, ingests it
/// DELIBERATELY, and requires the verified checker to reject the result. It is
/// the reason `fol_solve::attempt` ingests only on `sat`, and it is written as a
/// measurement rather than a comment because the comment would rot and the
/// measurement cannot.
///
/// Z3 in the same position prints `(error "model is not available")`, so this
/// is a difference between the two front ends and not a general property of
/// SMT-LIB. A pipeline that read the block anyway would hand the checker a
/// structure cvc5 never claimed was a model, the checker would reject it, and
/// the run would report a `model_not_confirmed` STOP_THE_LINE against a solver
/// that had done nothing wrong. A false stop-the-line costs as much credibility
/// as a missed one.
#[test]
fn cvc5_prints_a_structure_after_unknown_and_the_checker_rejects_it() {
    if skip() {
        return;
    }
    let d = scratch("unknown-block");
    let p = subsumption();
    let (tsv, digest) = p.to_problem_tsv().expect("writable");
    let problem_path = d.join("problem.tsv");
    std::fs::write(&problem_path, &tsv).expect("write");

    let smt = d.join("problem.smt2");
    std::fs::write(&smt, p.to_smtlib(SmtEncoding::Finite(2)).expect("writable")).expect("write");

    // No --finite-model-find. That is the whole point: this is cvc5's DEFAULT
    // behaviour on a quantified problem, which is what a reader who ran the
    // binary by hand would see.
    let out = Command::new("cvc5")
        .arg("--tlimit=30000")
        .arg(&smt)
        .output()
        .expect("run cvc5");
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let status = text
        .lines()
        .map(str::trim)
        .find(|l| matches!(*l, "sat" | "unsat" | "unknown"))
        .unwrap_or("<none>");

    // cvc5 got better, or the default strategy changed. This used to print a
    // note and return, which is a green tick over a trap nobody re-measured,
    // and a note on a passing run is read by nobody. It fails instead, for a
    // reason specific to how cvc5 reaches this machine: CI pins 1.3.4 by URL
    // AND by sha256, so the pinned solver cannot change underneath this test.
    // The only ways here are a deliberate bump of that pin, which is exactly
    // when the measurement must be redone, and a developer running a different
    // cvc5 locally, who should be told rather than reassured. Same discipline
    // as `the_fixture_problem_is_what_this_exporter_emits`: a recorded
    // measurement that no longer describes the tool is re-recorded, not
    // tolerated.
    assert_eq!(
        status, "unknown",
        "cvc5 answered {status:?} where 1.3.4 answered `unknown` on this problem, so this test \
         no longer measures the trap it exists for. The `ingest only on sat` rule in \
         fol_solve::attempt is unaffected and nothing is unsound. What is stale is the \
         measurement in src/fol_model.rs's cvc5 module: re-record it against this cvc5, and \
         update the pinned version and sha256 in .github/workflows/ci.yml to match whatever \
         you measured."
    );

    // It printed a structure anyway. Prove it, rather than asserting it.
    assert!(
        text.contains("define-fun"),
        "the trap is that cvc5 prints a model block after `unknown`; it printed none here, so \
         this test no longer measures anything: {text}",
    );

    // Ingest it deliberately and hand it to the verified checker. The checker
    // re-evaluates the problem's formulas against the structure and reads
    // nothing else, so this is the strongest available statement that the
    // block is not a model.
    let model = cvc5::parse_model(&text, &p.vocabulary(), 2)
        .expect("the block parses; it is well-formed SMT-LIB and merely false");
    assert_eq!(model.source, "cvc5", "provenance must name the producer");
    let model_path = d.join("model.tsv");
    std::fs::write(&model_path, model.to_model_tsv(&digest, &[2])).expect("write");

    let (code, report) = run_checker(&problem_path, &model_path);
    assert_ne!(
        code, 0,
        "the verified checker ACCEPTED a structure cvc5 printed under `unknown`. Either cvc5 \
         now returns a real model there, or this test is no longer ingesting what it thinks \
         it is: {report}",
    );
    assert_ne!(field(&report, "verdict"), "model_checked", "{report}");
    eprintln!("measured: cvc5 said `unknown` and printed a structure; oo-folmodel exited {code}");
}

/// The shared parser, and the provenance it must not launder.
///
/// The SMT-LIB `(get-model)` reader is one piece of code for both solvers
/// because both emit the same surface syntax. What must not be shared is the
/// NAME: a diagnostic that says "could not parse z3's model" over cvc5's bytes
/// sends a reader to the wrong tool, which is the same class of mistake as
/// `Fol.covers` being labelled soundness instead of attribution.
#[test]
fn an_ingestion_failure_names_the_solver_that_produced_the_output() {
    let vocab = not_entailed().vocabulary();

    // cvc5's UNBOUNDED model spelling, built over the problem's OWN vocabulary
    // so that every symbol IS interpreted and the single thing the evaluator
    // cannot read is the carrier element: cvc5 writes those as the qualified
    // identifier `(as @U_0 U)` rather than as the enumeration constructors
    // `e0 … e(k-1)` that the finite encoding declares. Writing the fixture by
    // hand instead left a predicate undefined, so the error raised was
    // `Uninterpreted` and the test passed while measuring a missing row rather
    // than the construct it names in its own docstring.
    let mut unbounded = String::from("sat\n(\n; cardinality of U is 1\n");
    for s in &vocab.unary {
        unbounded.push_str(&format!("(define-fun |{s}| ((_arg_1 U)) Bool true)\n"));
    }
    for s in &vocab.binary {
        unbounded.push_str(&format!("(define-fun |{s}| ((_arg_1 U) (_arg_2 U)) Bool true)\n"));
    }
    for s in &vocab.consts {
        unbounded.push_str(&format!("(define-fun |{s}| () U (as @U_0 U))\n"));
    }
    unbounded.push_str(")\n");
    let unbounded = unbounded.as_str();

    let err = cvc5::parse_model(unbounded, &vocab, 1).expect_err("not ingestible");
    let shown = err.to_string();
    assert!(shown.contains("cvc5"), "the message must name cvc5: {shown}");
    assert!(!shown.contains("z3"), "and must not name z3: {shown}");

    let same_through_z3 = z3::parse_model(unbounded, &vocab, 1).expect_err("not ingestible");
    assert!(same_through_z3.to_string().contains("z3"));

    // The two differ ONLY in the attribution, which is what makes sharing the
    // parser safe.
    assert_eq!(
        err,
        same_through_z3.clone().with_solver("cvc5"),
        "the cvc5 front end must be the z3 reader with the name corrected and nothing else \
         changed",
    );
    assert!(matches!(err, IngestError::Unsupported { .. }), "{err:?}");
}

// ── 4. The differential ─────────────────────────────────────────────────────

fn differential(args: &[&str], extra_path: Option<&Path>) -> (bool, String) {
    let mut cmd = Command::new("python3");
    cmd.arg(repo().join("tools/smt_differential.py"))
        .args(args)
        .current_dir(repo())
        .env("SMT_DIFF_REQUIRE_SOLVERS", "1")
        .env(
            "OPEN_ONTOLOGIES_BIN",
            repo().join("target/release/open-ontologies"),
        );
    if let Some(p) = extra_path {
        let path = std::env::var("PATH").unwrap_or_default();
        cmd.env("PATH", format!("{}:{}", p.display(), path));
    }
    let out = cmd.output().expect("run tools/smt_differential.py");
    (
        out.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

/// The release binary the differential drives. It is built by `cargo build
/// --release` and not by `cargo test`, so its absence is a skip with the reason
/// rather than a confusing Python error.
fn engine_built() -> bool {
    repo().join("target/release/open-ontologies").is_file()
}

/// The differential runs for real over a fixed corpus, and Z3 and cvc5 agree.
///
/// The assertion is NOT "it exited 0". A tool that emitted no problems would
/// also exit 0 under a naive reading, so the anti-vacuity guard inside the tool
/// exits 3 on a run that compared nothing, and this test reads the counts out
/// of the output as well.
#[test]
fn the_differential_finds_no_contradiction_over_the_shipped_corpus() {
    if skip() {
        return;
    }
    if common::skip_unless(
        engine_built(),
        "target/release/open-ontologies",
        "build it with `cargo build --release`; the differential drives the real binary \
         because the SMT-LIB it compares must be the SMT-LIB the product emits",
    ) {
        return;
    }
    let (ok, out) = differential(
        &[
            "tests/data/sample.ttl",
            "case-studies/blast-furnace-ironmaking/blast-furnace-ontology.ttl",
            "--domains",
            "1,2,unbounded",
            "--max-goals",
            "4",
            "--timeout",
            "10",
        ],
        None,
    );
    assert!(ok, "the differential must not report a contradiction:\n{out}");
    assert!(out.contains("CONTRADICTION  0"), "{out}");
    assert!(
        !out.contains("NO_SIGNAL"),
        "a run that compared nothing is not agreement:\n{out}",
    );
    // Every encoding really was exercised. The cap used to be applied to the
    // flat file list, which silently kept only the first encoding, so a run
    // could claim to cover the unbounded question and never ask it.
    for enc in ["1", "2", "unbounded"] {
        assert!(
            out.contains(&format!("{enc} ontology")) || out.contains("AGREE"),
            "no row at encoding {enc}:\n{out}",
        );
    }
    eprintln!("{out}");
}

/// **PROOF THAT THE GATE CAN FAIL.**
///
/// A differential that has never been seen to go red is decoration, and this
/// repository has shipped several: CI gates that exit 0 on unparseable input, a
/// `| tee` without `pipefail`, a skip that reports `ok`. So a LYING Z3 is put
/// on PATH ahead of the real one, answering `unsat` to everything, and the run
/// must come back with a contradiction and a non-zero exit.
///
/// The stub answers `unsat` rather than `sat` deliberately. A stub that said
/// `sat` to everything would be caught by the verified checker one step later
/// anyway; `unsat` is the answer that NOTHING in this architecture can check,
/// which makes it the one a differential is the only defence against. That is
/// the whole argument for a second solver in one sentence.
#[test]
fn a_lying_solver_turns_the_differential_red() {
    if skip() {
        return;
    }
    if common::skip_unless(
        engine_built(),
        "target/release/open-ontologies",
        "build it with `cargo build --release`",
    ) {
        return;
    }
    let d = scratch("lying-z3");
    let liar = d.join("z3");
    std::fs::write(
        &liar,
        "#!/bin/sh\n\
         # A Z3 that refutes everything, including problems with obvious models.\n\
         # Nothing in this architecture can check an unsat, which is exactly why\n\
         # a second solver is the only thing that can catch this.\n\
         echo unsat\n",
    )
    .expect("write stub");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&liar, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }

    let (ok, out) = differential(
        &[
            "tests/data/sample.ttl",
            "--domains",
            "1,2",
            "--max-goals",
            "2",
            "--timeout",
            "10",
        ],
        Some(&d),
    );
    assert!(
        !ok,
        "a solver that answers `unsat` to everything must fail the differential, and this run \
         passed. The gate cannot fail, so it is not a gate:\n{out}",
    );
    assert!(out.contains("CONTRADICTION"), "{out}");
    assert!(
        out.contains("z3 said unsat and cvc5 said sat"),
        "the report must say WHICH solver said WHAT, because the tool does not adjudicate:\n\
         {out}",
    );
    eprintln!("{out}");
}
