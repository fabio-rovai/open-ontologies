//! `oo-fores` end to end.
//!
//! Decision 0005 names the missing piece: certifying a superposition proof
//! "needs a verified first-order calculus with unification that does not exist
//! in core Lean". This is that calculus, minus the unification, which turns out
//! not to be needed: a checker does not have to FIND a most general unifier, it
//! has to confirm that a substitution it was handed makes two literals
//! complementary, and that is an equality on terms.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn checker() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("OO_FORES") {
        let p = PathBuf::from(p);
        return p.exists().then_some(p);
    }
    let built = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lean/.lake/build/bin/oo-fores");
    built.exists().then_some(built)
}

struct Run {
    code: i32,
    out: String,
}

fn run(cert: &str) -> Option<Run> {
    let bin = checker()?;
    // One directory per call. These run in parallel and a shared path makes
    // two tests read each other's certificate.
    let dir = std::env::temp_dir().join(format!(
        "oo-fores-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("r.cert");
    std::fs::write(&f, cert).unwrap();
    let o = Command::new(bin).arg(&f).output().expect("oo-fores runs");
    let _ = std::fs::remove_dir_all(&dir);
    Some(Run {
        code: o.status.code().unwrap_or(-1),
        out: String::from_utf8_lossy(&o.stdout).into_owned(),
    })
}

/// `¬P(x) ∨ Q(x)`, `P(a)`, `¬Q(a)`. The smallest refutation that needs a
/// substitution: resolving the first two instantiates `x` to `a`.
const CLAUSES: &str = "c\t1\t-p0[v0] +p1[v0]\nc\t2\t+p0[f0]\nc\t3\t-p1[f0]\n";
const STEPS: &str = "r\t4\t+p1[f0]\t1\t-p0[v0]\t0=f0\t+p1[v0]\t2\t+p0[f0]\t\t\n\
                     r\t5\t\t4\t+p1[f0]\t\t\t3\t-p1[f0]\t\t\n";

#[test]
fn a_checked_refutation_exits_zero_and_names_its_theorem() {
    let Some(r) = run(&format!("{CLAUSES}{STEPS}")) else { return };
    assert_eq!(r.code, 0, "{}", r.out);
    assert!(r.out.contains(r#""theorem":"Fo.unsat_of_check""#), "{}", r.out);
}

#[test]
fn without_the_substitution_the_literals_do_not_resolve() {
    // `¬P(x)` and `P(a)` are complementary only after `x ↦ a`. Drop it and the
    // checker compares `x` with `a` and refuses.
    let bad = "r\t4\t+p1[f0]\t1\t-p0[v0]\t\t+p1[v0]\t2\t+p0[f0]\t\t\n";
    let Some(r) = run(&format!("{CLAUSES}{bad}")) else { return };
    assert_eq!(r.code, 1, "{}", r.out);
    assert!(!r.out.contains("theorem"), "a refusal must name no theorem: {}", r.out);
}

#[test]
fn a_remainder_that_drops_a_literal_is_refused() {
    // Clause 1 is `¬P(x) ∨ Q(x)`. Resolving away `¬P(x)` leaves `Q(x)`, and a
    // step claiming the empty remainder is claiming more than it proved.
    let bad = "r\t4\t\t1\t-p0[v0]\t0=f0\t\t2\t+p0[f0]\t\t\n";
    let Some(r) = run(&format!("{CLAUSES}{bad}")) else { return };
    assert_eq!(r.code, 1, "{}", r.out);
}

#[test]
fn a_prefix_of_a_refutation_is_not_a_refutation() {
    let one = "r\t4\t+p1[f0]\t1\t-p0[v0]\t0=f0\t+p1[v0]\t2\t+p0[f0]\t\t\n";
    let Some(r) = run(&format!("{CLAUSES}{one}")) else { return };
    assert_eq!(r.code, 1, "every step checking is not the same as proving: {}", r.out);
}

#[test]
fn a_satisfiable_clause_set_cannot_be_refuted() {
    // `¬P(x) ∨ Q(x)` and `P(a)` alone have a model, so no step reaches the
    // empty clause.
    let sat = "c\t1\t-p0[v0] +p1[v0]\nc\t2\t+p0[f0]\n";
    let step = "r\t3\t\t1\t-p0[v0]\t0=f0\t\t2\t+p0[f0]\t\t\n";
    let Some(r) = run(&format!("{sat}{step}")) else { return };
    assert_eq!(r.code, 1, "{}", r.out);
}

#[test]
fn an_unparsable_certificate_is_exit_two_and_not_a_verdict() {
    let Some(r) = run("c\t1\tthis is not a literal\n") else { return };
    assert_eq!(r.code, 2, "a parse failure must be exit 2, not a verdict: {}", r.out);
    assert!(!r.out.contains("verdict"), "{}", r.out);
}
