//! G4. The verdict discipline, and the test that fails if it is ever relaxed.
//!
//! Decision 0006 item 4 fixes five fields and five words, and the reason the
//! whole layer exists is that they are never collapsed into each other. This
//! file is to `src/fol_solve.rs` what
//! `lean_horn_certificate_test.rs::a_user_rule_never_earns_the_absolute_verdict`
//! is to the Horn layer: the guard against laundering something unchecked into
//! something certified.
//!
//! The three mechanical rules, each with its own test and each SHOWN FAILING
//! on input built to break it:
//!
//!   1. **`model_checked` requires `checker_exit == 0`.** There is one branch
//!      in `src/fol_solve.rs` that writes that word and it sits immediately
//!      after reading a zero exit code off `oo-folmodel`.
//!      `an_unchecked_result_never_reports_the_certified_word` drives the
//!      pipeline with a checker that rejects, a checker that cannot read the
//!      file, and no checker at all, and none of the three may reach it.
//!   2. **`unsatisfiable_oracle` requires a run with NO cardinality
//!      constraint.** `a_bounded_unsat_is_not_unsatisfiability` uses the
//!      decision record's own example: `∀x∃y (r(x,y) ∧ x≠y)` is unsat at
//!      carrier 1 and sat at carrier 2, so a driver that mapped the first to
//!      `unsatisfiable_oracle` would report a falsehood from correct solver
//!      output.
//!   3. **`owl_reading` is non-null only when the CHECKER said
//!      `goal_negated_present` and the verdict is `model_checked`.** Read back
//!      out of the checker's own JSON, never from this side's intention.
//!
//! # The stub checkers are the point, not a shortcut
//!
//! Three of these tests hand the pipeline a checker that is not `oo-folmodel`:
//! a script that always rejects, one that always fails to read, and a path
//! that does not exist. That is the only way to exercise the branches a
//! correct solver and a correct checker never take together, and it is exactly
//! the shape of the mistake the rule exists to stop: a pipeline that fell back
//! to the certified word when the checker was unavailable would report
//! `model_checked` on every machine without Lean installed.
//!
//! # G6
//!
//! A missing solver or a missing checker SKIPS LOUDLY with the install line
//! and never passes silently. `common::skip_unless` makes that fatal under
//! `OO_REQUIRE_FIXTURES=1`, and `a_missing_solver_is_loud` drives the real CLI
//! with an empty `PATH` to show the marker on stderr.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use open_ontologies::fol_solve::{
    Outcome, SolveOptions, Solver, find_checker, solve, verdict_means,
};
use open_ontologies::tptp::{FolProblem, Form, P1, P2, Term};

// ── Problems, built small so the carrier they need is known ─────────────────

fn cls(a: &str, t: Term) -> Form {
    Form::App1(P1::Cls(a.to_string()), t)
}
fn op(r: &str, t: Term, u: Term) -> Form {
    Form::App2(P2::Op(r.to_string()), t, u)
}
fn v(n: u32) -> Term {
    Term::Var(n)
}
fn k(a: &str) -> Term {
    Term::Const(a.to_string())
}
fn neg(f: Form) -> Form {
    Form::Neg(Box::new(f))
}
fn and(f: Form, g: Form) -> Form {
    Form::And(Box::new(f), Box::new(g))
}

fn problem(axioms: Vec<Form>, conjecture: Option<Form>) -> FolProblem {
    FolProblem {
        background: vec![],
        ind_axioms: vec![],
        axioms: axioms
            .into_iter()
            .enumerate()
            .map(|(i, f)| (format!("a{i}"), f))
            .collect(),
        conjecture: conjecture.map(|f| ("goal".to_string(), f)),
        individuals: Default::default(),
    }
}

/// Satisfiable at carrier 1.
fn easy() -> FolProblem {
    problem(vec![cls("A", k("a"))], None)
}

/// Decision 0006 item 4's own example. `∀x ∃y (r(x,y) ∧ ¬(x = y))` has no
/// model of size 1, because the only element is forced equal to itself, and
/// has one of size 2.
fn needs_two() -> FolProblem {
    problem(
        vec![Form::All(
            0,
            Box::new(Form::Ex(
                1,
                Box::new(and(op("r", v(0), v(1)), neg(Form::Eq(v(0), v(1))))),
            )),
        )],
        None,
    )
}

/// Unsatisfiable at every cardinality.
fn contradiction() -> FolProblem {
    problem(vec![cls("A", k("a")), neg(cls("A", k("a")))], None)
}

/// `A(a)` does not entail `B(a)`, so a countermodel exists at carrier 1.
fn not_entailed() -> FolProblem {
    problem(vec![cls("A", k("a"))], Some(cls("B", k("a"))))
}

// ── Environment ─────────────────────────────────────────────────────────────

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("oo-fol-verdict-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("scratch dir");
    d
}

fn missing() -> Option<String> {
    if !Solver::Z3.available() {
        return Some(format!("z3, the SMT solver; {}", Solver::Z3.install_line()));
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
            "the certifying pipeline needs both a model finder and the verified checker, and \
             certifies nothing without them",
        ),
    }
}

fn opts(max_domain: u32) -> SolveOptions {
    SolveOptions {
        solver: Solver::Z3,
        max_domain,
        timeout_secs: 20,
        unbounded_probe: false,
        checker: None,
    }
}

/// Write an executable stub and return its path. The stubs stand in for
/// `oo-folmodel` on the branches a correct checker never reaches.
fn stub(dir: &Path, name: &str, body: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, body).expect("write stub");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }
    p
}

// ── Rule 1: the certified word requires a zero exit from the checker ────────

/// The positive control. Without it every test below would pass on a pipeline
/// that never certified anything.
#[test]
fn a_checked_model_earns_the_certified_word() {
    if skip() {
        return;
    }
    let d = scratch("certified");
    let o = solve(&easy(), &opts(4), &d).expect("the pipeline runs");
    assert_eq!(o.verdict, "model_checked", "{o:?}");
    assert_eq!(o.checker_exit, Some(0));
    assert_eq!(o.solver_verdict, "sat");
    assert_eq!(o.theorem.as_deref(), Some("Fol.satisfiable_of_check"));
    assert!(o.disagreement.is_none());
    assert!(o.skipped.is_none());
    assert!(
        o.checker_report
            .as_deref()
            .unwrap_or_default()
            .contains("\"verdict\":\"model_checked\""),
        "the checker's own report must say so too: {o:?}"
    );
    assert!(verdict_means("model_checked").contains("CERTIFIED"));
    // The files a reader would need to reproduce the run by hand are all there.
    for f in ["problem.tsv", "model.tsv", "checker.json"] {
        assert!(d.join(f).is_file(), "missing {f}");
    }
}

/// THE LAUNDERING GUARD. Three ways for the check not to have happened, and
/// none of them may report the certified word.
///
/// A pipeline that fell back to `model_checked` when the checker was
/// unavailable would print the certified word on every machine without Lean,
/// which is the exact shape of the mistake this repository exists to catch.
#[test]
fn an_unchecked_result_never_reports_the_certified_word() {
    if skip() {
        return;
    }
    let d = scratch("unchecked");

    // (a) A checker that REJECTS. The solver said sat, so the verdict is the
    // oracle's word and never `rejected` as though the ontology were at fault
    // (decision 0006 item 5), and the disagreement is STOP_THE_LINE.
    let rejecting = stub(
        &d,
        "reject.sh",
        "#!/bin/sh\necho '{\"verdict\":\"rejected\",\"theorem\":\"Fol.check_complete\"}'\nexit 1\n",
    );
    let mut o1 = opts(4);
    o1.checker = Some(rejecting);
    let r1 = solve(&easy(), &o1, &d.join("a")).expect("runs");
    assert_ne!(r1.verdict, "model_checked", "{r1:?}");
    assert_eq!(r1.verdict, "satisfiable_oracle");
    assert_ne!(r1.verdict, "rejected");
    assert_eq!(r1.checker_exit, Some(1));
    assert!(r1.theorem.is_none(), "no theorem may be named: {r1:?}");
    assert!(r1.owl_reading.is_none());
    let dis = r1.disagreement.as_ref().expect("a disagreement");
    assert_eq!(dis.severity, "STOP_THE_LINE");
    assert_eq!(dis.what, "model_not_confirmed");
    assert!(dis.means.contains("false clean"), "{}", dis.means);

    // (b) A checker that cannot READ the file. Exit 2 is "unreadable", which
    // is not a verdict in either direction, and it must not become one.
    let unreadable = stub(
        &d,
        "unreadable.sh",
        "#!/bin/sh\necho '{\"verdict\":\"unreadable\",\"reason\":\"stub\"}'\nexit 2\n",
    );
    let mut o2 = opts(4);
    o2.checker = Some(unreadable);
    let r2 = solve(&easy(), &o2, &d.join("b")).expect("runs");
    assert_ne!(r2.verdict, "model_checked", "{r2:?}");
    assert_eq!(r2.checker_exit, Some(2));
    assert!(r2.disagreement.is_some());

    // (c) NO checker at all. Loud, and still not certified.
    let mut o3 = opts(4);
    o3.checker = Some(PathBuf::from("/nonexistent/oo-folmodel"));
    let r3 = solve(&easy(), &o3, &d.join("c")).expect("runs");
    assert_ne!(r3.verdict, "model_checked", "{r3:?}");
    assert_eq!(r3.checker_exit, None);
    assert!(r3.skipped.is_some(), "a missing checker must be reported: {r3:?}");

    // And a checker that ACCEPTS does reach it, so the branch is live and the
    // three results above are about the exit code rather than about the stub.
    let accepting = stub(
        &d,
        "accept.sh",
        "#!/bin/sh\necho '{\"verdict\":\"model_checked\",\"goal_negated_present\":false}'\nexit 0\n",
    );
    let mut o4 = opts(4);
    o4.checker = Some(accepting);
    let r4 = solve(&easy(), &o4, &d.join("d")).expect("runs");
    assert_eq!(r4.verdict, "model_checked");
}

// ── Rule 2: a bounded unsat is not unsatisfiability ─────────────────────────

/// Decision 0006 item 4's own counterexample, run for real.
///
/// `∀x∃y (r(x,y) ∧ x≠y)` is unsat at carrier 1 and sat at carrier 2. A driver
/// that mapped the first to `unsatisfiable_oracle` would be reporting a
/// falsehood from correct solver output, and it would take ten minutes to
/// write. The same problem produces two different verdicts here and the word
/// `unsatisfiable_oracle` appears in neither.
#[test]
fn a_bounded_unsat_is_not_unsatisfiability() {
    if skip() {
        return;
    }
    let d = scratch("bounded");
    let at_one = solve(&needs_two(), &opts(1), &d.join("k1")).expect("runs");
    assert_eq!(at_one.solver_verdict, "unsat", "{at_one:?}");
    assert_eq!(at_one.verdict, "no_model_up_to_size_k");
    assert_ne!(at_one.verdict, "unsatisfiable_oracle");
    assert_eq!(at_one.encoding, "finite(1)");
    assert_eq!(at_one.unbounded, None, "no unbounded run happened");
    assert!(at_one.bounded_search.contains("exhausted"), "{at_one:?}");
    assert!(verdict_means(at_one.verdict.word()).contains("NOT"));

    let at_two = solve(&needs_two(), &opts(2), &d.join("k2")).expect("runs");
    assert_eq!(at_two.verdict, "model_checked", "{at_two:?}");
    assert_eq!(at_two.encoding, "finite(2)");
    assert_eq!(at_two.cardinality_search, vec![1, 2]);
}

/// `unsatisfiable_oracle` is reachable ONLY through the unbounded probe.
///
/// The same genuinely unsatisfiable problem gives `no_model_up_to_size_k` with
/// the probe off and `unsatisfiable_oracle` with it on, and the second is
/// still an ORACLE's word: nothing checked a refutation and nothing here can.
#[test]
fn only_an_unbounded_run_can_say_unsatisfiable() {
    if skip() {
        return;
    }
    let d = scratch("unsat");
    let bounded = solve(&contradiction(), &opts(3), &d.join("bounded")).expect("runs");
    assert_eq!(bounded.verdict, "no_model_up_to_size_k", "{bounded:?}");
    assert_eq!(bounded.unbounded, None);

    let mut probing = opts(3);
    probing.unbounded_probe = true;
    let probed = solve(&contradiction(), &probing, &d.join("probed")).expect("runs");
    assert_eq!(probed.verdict, "unsatisfiable_oracle", "{probed:?}");
    assert_eq!(probed.unbounded, Some("unsat"));
    assert_eq!(probed.encoding, "unbounded");
    assert_eq!(probed.checker_exit, None, "nothing was checked");
    assert!(probed.theorem.is_none(), "no theorem may be named: {probed:?}");
    assert!(verdict_means("unsatisfiable_oracle").contains("oracle opinion"));
    // The bounded ladder's own finding survives beside it rather than being
    // erased by the probe.
    assert!(probed.bounded_search.contains("exhausted"), "{probed:?}");
}

// ── Rule 3: the OWL reading comes off the checker's own report ──────────────

/// A checked countermodel to a negated goal carries the OWL-level word, and a
/// run with no goal does not, even when it is certified.
///
/// The word is long on purpose. The OWL sentence rides on `OwlLean.adequacy`
/// in a sibling project AND on the Rust-to-Lean correspondence that decision
/// 0005 item 2 states is pinned by tests and NOT proved, so it must never be
/// shortened to "not entailed".
#[test]
fn the_owl_reading_is_read_off_the_checkers_own_report() {
    if skip() {
        return;
    }
    let d = scratch("owl");
    let withgoal = solve(&not_entailed(), &opts(3), &d.join("goal")).expect("runs");
    assert_eq!(withgoal.verdict, "model_checked", "{withgoal:?}");
    // `OwlReading` has no public constructor either: the word is compared, and
    // the presence of the value is what says the checker reported a negated
    // goal on a run it accepted.
    assert_eq!(
        withgoal.owl_reading.map(|r| r.word()),
        Some("not_entailed_under_unproved_translation")
    );
    assert!(
        withgoal
            .checker_report
            .as_deref()
            .unwrap_or_default()
            .contains("\"goal_negated_present\":true"),
        "the CHECKER must be the one reporting the goal: {withgoal:?}"
    );

    let nogoal = solve(&easy(), &opts(3), &d.join("nogoal")).expect("runs");
    assert_eq!(nogoal.verdict, "model_checked");
    assert!(nogoal.owl_reading.is_none(), "{nogoal:?}");

    // A checker that reports a goal but does NOT accept gets no OWL reading
    // either: the rule is a conjunction, and this is the half a report that
    // read only its own intention would get wrong.
    let lying = stub(
        &d,
        "goal_but_reject.sh",
        "#!/bin/sh\necho '{\"verdict\":\"rejected\",\"goal_negated_present\":true}'\nexit 1\n",
    );
    let mut o = opts(3);
    o.checker = Some(lying);
    let r = solve(&not_entailed(), &o, &d.join("lying")).expect("runs");
    assert!(r.owl_reading.is_none(), "{r:?}");

    // THE HALF THAT MAKES THE RULE BITE. The problem HAS a conjecture and the
    // checker ACCEPTS, but the checker reports that the file it read carried
    // no negated goal. The checker is the authority on that, because it is the
    // one that parsed `problem.tsv`; a report computed from this side's
    // `conjecture.is_some()` would claim a non-entailment over a file whose
    // goal never reached the checker, which is a writer bug reported as a
    // result about the ontology.
    //
    // Without this case the rule is untestable: on every honest run the two
    // sources agree, so swapping one for the other changes nothing and the
    // gate would be decoration.
    let silent = stub(
        &d,
        "accept_no_goal.sh",
        "#!/bin/sh\necho '{\"verdict\":\"model_checked\",\"goal_negated_present\":false}'\nexit 0\n",
    );
    let mut o2 = opts(3);
    o2.checker = Some(silent);
    let r2 = solve(&not_entailed(), &o2, &d.join("silent")).expect("runs");
    assert_eq!(r2.verdict, "model_checked", "{r2:?}");
    assert!(
        r2.owl_reading.is_none(),
        "the CHECKER said the file carried no goal, so no OWL reading may be claimed \
         however much this side intended one: {r2:?}"
    );
}

// ── The five words, and the fields that are never collapsed ─────────────────

/// Every word a verdict can take has a sentence, and the sentences say what
/// rests on a theorem and what does not.
#[test]
fn every_verdict_word_says_what_it_rests_on() {
    assert!(verdict_means("model_checked").contains("Fol.satisfiable_of_check"));
    assert!(verdict_means("satisfiable_oracle").contains("nothing here checked it"));
    assert!(verdict_means("no_model_up_to_size_k").contains("finite model property"));
    assert!(verdict_means("unsatisfiable_oracle").contains("decision 0005"));
    assert!(verdict_means("unknown_oracle").contains("Not evidence"));
    // The four oracle words must not claim a theorem.
    for w in [
        "satisfiable_oracle",
        "no_model_up_to_size_k",
        "unsatisfiable_oracle",
        "unknown_oracle",
    ] {
        assert!(!verdict_means(w).contains("CERTIFIED"), "{w} claims certification");
    }
}

/// The five fields decision 0006 fixes are all present and independently
/// readable on a real run, so a report cannot quietly drop one.
#[test]
fn the_five_fields_are_all_reported() {
    if skip() {
        return;
    }
    let o = solve(&not_entailed(), &opts(3), &scratch("fields")).expect("runs");
    let j = open_ontologies::fol_solve::outcome_json(&o);
    for field in [
        "solver_verdict",
        "encoding",
        "checker_exit",
        "verdict",
        "owl_reading",
    ] {
        assert!(j.get(field).is_some(), "missing {field} in {j}");
    }
    assert_eq!(j["solver_verdict"], "sat");
    assert_eq!(j["encoding"], "finite(1)");
    assert_eq!(j["checker_exit"], 0);
    assert_eq!(j["verdict"], "model_checked");
    assert_eq!(j["owl_reading"], "not_entailed_under_unproved_translation");
    assert_eq!(j["problem_digest"], o.problem_digest);
    assert_eq!(j["disagreement"], serde_json::Value::Null);
}

// ── G6: loud skips ──────────────────────────────────────────────────────────

/// A missing checker is REPORTED, not worked around.
///
/// A pipeline that silently dropped to an oracle verdict because Lean was not
/// built would make every run look like a solver limitation, which is
/// mis-attribution of exactly the kind this repository keeps finding in other
/// people's work.
#[test]
fn a_missing_checker_is_loud() {
    let e = find_checker(Some(Path::new("/nonexistent/oo-folmodel"))).expect_err("must fail");
    assert!(e.contains("/nonexistent/oo-folmodel"), "{e}");
    // The default search names the build line rather than shrugging.
    if let Err(why) = find_checker(None) {
        assert!(why.contains("lake build"), "{why}");
        assert!(why.contains("OO_FOLMODEL"), "{why}");
    }
}

/// A missing SOLVER skips loudly with the install line, run for real through
/// the CLI with an empty `PATH` so that nothing on this machine can be found.
///
/// Driven as a subprocess rather than in-process because `PATH` is global to
/// a process and the rest of this binary's tests need the real one.
#[test]
fn a_missing_solver_is_loud() {
    let bin = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join("open-ontologies");
    if common::skip_unless(
        bin.is_file(),
        "the debug CLI binary",
        "run `cargo build` first; this test drives the real command",
    ) {
        return;
    }
    let d = scratch("nopath");
    let out = Command::new(&bin)
        .arg("--no-connect")
        .arg("--data-dir")
        .arg(d.join("store"))
        .arg("batch")
        .arg("-")
        .env("PATH", "")
        .env("OPEN_ONTOLOGIES_STORAGE_MODE", "persistent")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            use std::io::Write as _;
            let script = format!(
                "load {}\nfol-model --out {} --solver z3 --max-domain 2\n",
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/data/sample.ttl")
                    .display(),
                d.join("solve").display()
            );
            c.stdin.as_mut().expect("stdin").write_all(script.as_bytes())?;
            c.wait_with_output()
        })
        .expect("run the CLI");
    let err = String::from_utf8_lossy(&out.stderr);
    let so = String::from_utf8_lossy(&out.stdout);
    assert!(
        err.contains("SKIPPED_SOLVER") || err.contains("SKIPPED_CHECKER"),
        "a missing solver must print the marker on stderr.\nstderr: {err}\nstdout: {so}"
    );
    assert!(
        err.contains("brew install") || err.contains("lake build"),
        "and the install line: {err}"
    );
    assert!(
        !so.contains("\"verdict\":\"model_checked\""),
        "nothing may be certified with no solver on PATH: {so}"
    );
}

/// A LYING SOLVER, end to end, through the real command.
///
/// This is the case the whole layer exists for, and a stub checker cannot
/// stand in for it: a model finder that answers `sat` and hands back a
/// structure that is not a model. The fake `z3` here returns a complete,
/// well-formed, correctly typed interpretation of every symbol
/// `tests/data/sample.ttl` exports, over a carrier of one, in which `thing` is
/// empty. That fails `background_2` (`∃x thing(x)`) and every `ind_typing`
/// axiom, and no amount of reading the file would tell you so — only running
/// the verified checker does.
///
/// What must happen, and all four are asserted:
///
///   * the verdict is `satisfiable_oracle`, because the solver said `sat` and
///     that is a claim about the ontology this pipeline did not verify. It is
///     NOT `rejected`, which would blame the ontology, and NOT `model_checked`;
///   * a `disagreement` block with `severity: STOP_THE_LINE`;
///   * the run summary counts it;
///   * THE COMMAND EXITS NON-ZERO, in batch mode, which is the mode every tool
///     in `tools/` uses. A pipeline in which the check never has to pass is not
///     a pipeline with a check in it.
#[test]
fn a_lying_solver_stops_the_line_and_fails_the_command() {
    if skip() {
        return;
    }
    let bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/open-ontologies");
    if common::skip_unless(
        bin.is_file(),
        "the debug CLI binary",
        "run `cargo build` first; this test drives the real command",
    ) {
        return;
    }
    let d = scratch("lying");
    let fake = d.join("bin");
    std::fs::create_dir_all(&fake).expect("bin dir");
    let ns = "http://example.org/test#";
    let body = format!(
        "#!/bin/sh\n         echo sat\n         cat <<'EOF'\n         (\n         (define-fun |c:{ns}Organization| ((x!0 U)) Bool true)\n         (define-fun |c:{ns}Person| ((x!0 U)) Bool true)\n         (define-fun lit ((x!0 U)) Bool false)\n         (define-fun thing ((x!0 U)) Bool false)\n         (define-fun |op:{ns}relationship| ((x!0 U) (x!1 U)) Bool true)\n         (define-fun |op:{ns}worksFor| ((x!0 U) (x!1 U)) Bool true)\n         (define-fun |i:{ns}AcmeCorp| () U e0)\n         (define-fun |i:{ns}Alice| () U e0)\n         )\n         EOF\n"
    );
    stub(&fake, "z3", &body);

    let script = format!(
        "load {}\nfol-model --out {} --solver z3 --max-domain 1 --unbounded-probe false\n",
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data/sample.ttl")
            .display(),
        d.join("solve").display()
    );
    let out = Command::new(&bin)
        .arg("--no-connect")
        .arg("--data-dir")
        .arg(d.join("store"))
        .arg("batch")
        .arg("-")
        .env("PATH", format!("{}:{}", fake.display(), std::env::var("PATH").unwrap_or_default()))
        .env("OPEN_ONTOLOGIES_STORAGE_MODE", "persistent")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            use std::io::Write as _;
            c.stdin.as_mut().expect("stdin").write_all(script.as_bytes())?;
            c.wait_with_output()
        })
        .expect("run the CLI");
    let so = String::from_utf8_lossy(&out.stdout);
    let line = so
        .lines()
        .find(|l| l.contains("\"command\":\"fol-model\""))
        .unwrap_or_else(|| panic!("no fol-model result in {so}"));
    let v: serde_json::Value = serde_json::from_str(line).expect("json");
    let r = &v["result"];
    let o = &r["ontology"];
    assert_eq!(o["solver_verdict"], "sat", "{r}");
    assert_eq!(o["verdict"], "satisfiable_oracle", "{r}");
    assert_ne!(o["verdict"], "model_checked", "{r}");
    assert_ne!(o["verdict"], "rejected", "the ontology is not at fault: {r}");
    assert_eq!(o["checker_exit"], 1, "{r}");
    assert_eq!(o["theorem"], serde_json::Value::Null, "{r}");
    assert_eq!(o["disagreement"]["severity"], "STOP_THE_LINE", "{r}");
    assert_eq!(o["disagreement"]["what"], "model_not_confirmed", "{r}");
    assert_eq!(r["stop_the_line"], 1, "{r}");
    assert_eq!(
        out.status.code(),
        Some(1),
        "a stop-the-line must FAIL the command, in batch mode, which is the mode \
         tools/ uses. stdout: {so}"
    );
}

/// The two finders differ in what they can be asked, and the difference is
/// data rather than prose because the verdict rules read it.
#[test]
fn the_two_finders_declare_their_own_limits() {
    // Mace4's minimum carrier is 2: `mace4 -n 1` is a FATAL error, measured on
    // LADR 2009-11A (`parm start_size, value 1 out of range [2..2147483647]`).
    assert_eq!(Solver::Mace4.min_domain(), 2);
    assert_eq!(Solver::Z3.min_domain(), 1);
    // And Mace4 has no unbounded question at all, so it can never produce
    // `unsatisfiable_oracle`.
    assert!(!Solver::Mace4.can_probe_unbounded());
    assert!(Solver::Z3.can_probe_unbounded());
    assert!(Solver::parse("mace4").is_ok());
    assert!(Solver::parse("z3").is_ok());
    assert!(Solver::parse("eprover").is_err(), "E is a prover, not a model finder");
    for s in [Solver::Z3, Solver::Mace4] {
        assert!(s.install_line().contains("install"));
    }
}

/// Mace4 reaches the certified word too, and its reduct is visible.
///
/// Mace4 clausifies and Skolemises, so the structure it prints interprets
/// names the translation never emitted. Dropping them is taking the reduct and
/// needs no lemma, because the checker re-evaluates formulas that cannot
/// mention them. What it does need is to be VISIBLE.
#[test]
fn mace4_also_earns_the_certified_word_and_names_its_reduct() {
    if common::skip_unless(
        Solver::Mace4.available() && find_checker(None).is_ok(),
        "mace4 and the verified checker",
        Solver::Mace4.install_line(),
    ) {
        return;
    }
    let d = scratch("mace4");
    let mut o = opts(4);
    o.solver = Solver::Mace4;
    // A top-level existential, which Mace4 Skolemises into a constant of its
    // own choosing.
    let p = problem(
        vec![
            Form::Ex(0, Box::new(cls("A", v(0)))),
            cls("A", k("a")),
        ],
        None,
    );
    let r: Outcome = solve(&p, &o, &d).expect("runs");
    assert_eq!(r.verdict, "model_checked", "{r:?}");
    assert_eq!(r.checker_exit, Some(0));
    // The ladder starts at 2, not 1.
    assert_eq!(r.cardinality_search.first(), Some(&2));
    assert!(
        !r.dropped_symbols.is_empty(),
        "Mace4's Skolem constant must be reported as the reduct: {r:?}"
    );
    assert!(
        r.checker_report.as_deref().unwrap_or_default().contains("\"source\":\"mace4\""),
        "{r:?}"
    );
}
