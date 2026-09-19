//! A prover's derivation, translated into `lean/Fo`'s certificate format and
//! checked by `oo-fores`, whose acceptance discharges `Fo.unsat_of_check`.
//!
//! These tests record a MEASURED boundary rather than a hoped-for one. Pure
//! CNF without equality translates and checks. Clausification and equality
//! reasoning do not, and the steps that do not are named and counted rather
//! than skipped.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use open_ontologies::tstp::to_fo_certificate;

static NEXT: AtomicU64 = AtomicU64::new(0);

fn scratch() -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "oo-tstp-fo-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn on_path(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

fn fores() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("OO_FORES") {
        let p = PathBuf::from(p);
        return p.exists().then_some(p);
    }
    let built = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("lean/.lake/build/bin/oo-fores");
    built.exists().then_some(built)
}

/// Run Vampire on a problem and return its derivation.
fn vampire(problem: &str) -> Option<String> {
    let bin = on_path("vampire")?;
    let d = scratch();
    let p = d.join("p.p");
    std::fs::write(&p, problem).unwrap();
    let o = Command::new(bin).arg("--proof").arg("tptp").arg(&p).output().ok()?;
    let _ = std::fs::remove_dir_all(&d);
    Some(String::from_utf8_lossy(&o.stdout).into_owned())
}

/// Hand the certificate to `oo-fores` and return its exit code and output.
fn check(cert: &str) -> Option<(i32, String)> {
    let bin = fores()?;
    let d = scratch();
    let f = d.join("r.cert");
    std::fs::write(&f, cert).unwrap();
    let o = Command::new(bin).arg(&f).output().ok()?;
    let _ = std::fs::remove_dir_all(&d);
    Some((
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).into_owned(),
    ))
}

const MODUS_PONENS: &str = "cnf(c1, axiom, ( ~p(X) | q(X) )).\n\
                            cnf(c2, axiom, ( p(a) )).\n\
                            cnf(c3, negated_conjecture, ( ~q(a) )).\n";

/// Four links, so the proof is a chain rather than a single step.
const CHAIN: &str = "cnf(b1, axiom, ( ~p(X) | q(X) )).\n\
                     cnf(b2, axiom, ( ~q(X) | r(X) )).\n\
                     cnf(b3, axiom, ( ~r(X) | s(X) )).\n\
                     cnf(b4, axiom, ( p(a) )).\n\
                     cnf(b5, negated_conjecture, ( ~s(a) )).\n";

#[test]
fn a_vampire_refutation_of_pure_cnf_is_checked_by_the_lean_theorem() {
    let Some(proof) = vampire(MODUS_PONENS) else { return };
    let cert = to_fo_certificate(&proof).expect("the derivation parses");
    assert!(
        cert.untranslated.is_empty(),
        "pure CNF should translate whole, left out: {:?}",
        cert.untranslated
    );
    assert!(cert.reaches_false, "the certificate must reach the empty clause");
    let Some((code, out)) = check(&cert.text) else { return };
    assert_eq!(code, 0, "oo-fores refused a real Vampire proof:\n{}\n{out}", cert.text);
    assert!(out.contains(r#""theorem":"Fo.unsat_of_check""#), "{out}");
}

#[test]
fn a_longer_chain_translates_every_step() {
    let Some(proof) = vampire(CHAIN) else { return };
    let cert = to_fo_certificate(&proof).expect("parses");
    assert!(cert.translated >= 3, "expected a chain, got {} steps", cert.translated);
    assert!(cert.untranslated.is_empty(), "{:?}", cert.untranslated);
    let Some((code, _)) = check(&cert.text) else { return };
    assert_eq!(code, 0);
}

#[test]
fn clausification_is_named_and_counted_rather_than_skipped() {
    // A FOF problem makes the prover clausify first, and clausification is not
    // resolution. The certificate must NOT reach the empty clause, and every
    // step it could not take must be named.
    let fof = "fof(a1, axiom, ![X] : (p(X) => q(X))).\n\
               fof(a2, axiom, p(a)).\n\
               fof(g, conjecture, q(a)).\n";
    let Some(proof) = vampire(fof) else { return };
    let cert = to_fo_certificate(&proof).expect("parses");
    assert!(
        !cert.reaches_false,
        "clausification is outside this calculus and must not be claimed"
    );
    assert!(!cert.untranslated.is_empty(), "the steps left out must be named");
    assert!(
        cert.untranslated.iter().any(|(_, r)| r.contains("cnf_transformation")),
        "clausification should be named as the reason: {:?}",
        cert.untranslated
    );
    // And the checker refuses what the translation could not complete.
    let Some((code, out)) = check(&cert.text) else { return };
    assert_eq!(code, 1, "{out}");
    assert!(!out.contains("theorem"), "a refusal names no theorem: {out}");
}

#[test]
fn equality_reasoning_is_outside_the_calculus_and_says_so() {
    let eq = "cnf(e1, axiom, ( f(X) = g(X) )).\n\
              cnf(e2, axiom, ( p(f(a)) )).\n\
              cnf(e3, negated_conjecture, ( ~p(g(a)) )).\n";
    let Some(proof) = vampire(eq) else { return };
    let cert = to_fo_certificate(&proof).expect("parses");
    assert!(
        !cert.reaches_false,
        "paramodulation is not resolution and must not be translated as if it were"
    );
    assert!(!cert.untranslated.is_empty(), "{:?}", cert.untranslated);
}

#[test]
fn the_rule_name_is_a_hint_and_the_witness_is_the_check() {
    // E calls every inference `spm`, including ones that are ordinary binary
    // resolution. A translator keyed on rule NAMES would translate none of
    // them. This one offers every two-parent step to the resolvent witness, so
    // what matters is whether the step IS resolution, not what it is called.
    let Some(bin) = on_path("eprover") else { return };
    let d = scratch();
    let p = d.join("p.p");
    std::fs::write(&p, CHAIN).unwrap();
    let o = Command::new(bin).arg("--proof-object").arg("--auto").arg(&p).output().unwrap();
    let _ = std::fs::remove_dir_all(&d);
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    let Some(start) = text.find("SZS output start") else { return };
    let proof = &text[start..];
    let cert = to_fo_certificate(proof).expect("parses");
    assert!(
        cert.translated > 0,
        "E's `spm` steps over non-equality literals are resolution and must translate"
    );
}
