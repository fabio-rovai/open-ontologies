//! **The verified SHACL evaluator, reachable from a tool.**
//!
//! `Shacl/` holds a mechanised SHACL Core evaluator whose soundness is a
//! machine-checked theorem, `Shacl.validate_spec`, and `oo-shacl` is the binary
//! that runs it. Until this module existed nothing in `src/` mentioned either.
//! A grep for `oo-shacl`, `ShaclMain` or `validate_spec` across the whole crate
//! returned one file, `tests/shacl_core_verified_test.rs`, and that is a test.
//!
//! So the strongest claim in the repository was unreachable from its own
//! interface. `onto_shacl` and the CLI compiled every constraint to SPARQL and
//! handed it to Oxigraph, unverified, with a `skipped_constraints` list; the
//! Lean result was a lab measurement with a binary beside it rather than
//! something a user could obtain. This closes that, behind `verified: true`.
//!
//! # Why a second path rather than a replacement
//!
//! The Rust path answers more. `Shacl/` covers SHACL Core and refuses
//! `sh:sparql` and user-defined components outright, which is 22 of the 120
//! W3C tests; the Rust evaluator runs those and skips others. Neither is a
//! superset, so replacing one with the other would lose answers people depend
//! on. `verified: true` says which question you are asking.
//!
//! # The three exit codes, and why the middle one is the point
//!
//! `oo-shacl` exits 0 with a verdict, 2 when a file could not be read, and 3
//! UNDETERMINED when the shapes graph uses something the development does not
//! implement or the evaluator declined to judge. Three is the code that
//! matters: it is the difference between "everything conforms" and "I did not
//! check everything", and a caller that collapsed them would be doing exactly
//! what the unverified path's `skipped_constraints` list exists to prevent.
//! There is no `conforms: true` on this path that is not covered by the
//! theorem.
//!
//! # The word is earned
//!
//! `ShaclVerified::Checked` carries a [`Certified`], which has no public
//! constructor: the only one in the crate comes out of
//! [`CheckerRun::accepted`] after a process exited zero. The same discipline
//! `src/fol_solve.rs` uses, and for the same reason.

use crate::graph::GraphStore;
use crate::verdict::{CheckerBinary, CheckerRun, Certified};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::process::Command;
use std::sync::Arc;

/// Serial number for the scratch directory, so concurrent calls cannot share
/// one. See `validate_verified`.
static NEXT_RUN: AtomicU64 = AtomicU64::new(0);

/// Where `oo-shacl` is, or a message saying how to get one.
///
/// `$OO_SHACL` first, so an operator can point at an installed binary, then the
/// repository's own build output. Mirrors `find_checker` in `src/fol_solve.rs`
/// and `src/closure_diff.rs`; the paths differ, the shape does not.
pub fn find_checker() -> Result<CheckerBinary, String> {
    resolve_checker(std::env::var("OO_SHACL").ok())
}

/// The resolution itself, with the environment passed in rather than read.
///
/// Split out so it is testable without `set_var`. A test that pointed
/// `OO_SHACL` at a nonexistent path to exercise the not-found branch changed
/// the variable for every OTHER test in the same process, because the
/// environment is process-global and the harness runs tests in parallel: two
/// unrelated tests in this file went red and one of them was the reachability
/// claim itself. A pure function has no such reach.
pub fn resolve_checker(env: Option<String>) -> Result<CheckerBinary, String> {
    if let Some(v) = env {
        let p = PathBuf::from(&v);
        if p.exists() {
            return Ok(CheckerBinary::found_at(p));
        }
        return Err(format!("OO_SHACL is set to {v}, which does not exist"));
    }
    let local = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("lean/.lake/build/bin/oo-shacl");
    if local.exists() {
        return Ok(CheckerBinary::found_at(local));
    }
    Err(format!(
        "the verified SHACL evaluator was not found. Build it with `cd lean && lake build \
         oo-shacl` (it is in defaultTargets, so a bare `lake build` is enough), or set \
         OO_SHACL. Looked in {}",
        local.display()
    ))
}

/// What the verified evaluator said.
#[derive(Debug, Clone)]
pub enum ShaclVerified {
    /// Exit 0. A verdict covered by `Shacl.validate_spec`. The `bool` is
    /// `conforms`; the token is the evidence a checker accepted the run.
    Checked(bool, Certified),
    /// Exit 3. The evaluator declined to judge, and said why. NOT a conformance
    /// verdict in either direction, and the reason it is a separate variant is
    /// that a caller cannot accidentally read it as one.
    Undetermined(String),
}

/// Serialise the store and the shapes and run the verified evaluator over them.
///
/// The two files are N-Triples because that is what `oo-shacl` reads, and the
/// shapes are round-tripped through a scratch store so a caller can pass the
/// Turtle every other tool here takes.
pub fn validate_verified(
    graph: &Arc<GraphStore>,
    shapes_ttl: &str,
) -> anyhow::Result<serde_json::Value> {
    let checker = match find_checker() {
        Ok(c) => c,
        Err(why) => {
            return Ok(serde_json::json!({
                "verified": false,
                "error": why,
                "note": "no verdict. A missing checker is not a conformance answer.",
            }))
        }
    };

    let data_nt = graph.snapshot("ntriples")?;
    // The shapes arrive as Turtle and `oo-shacl` reads N-Triples, so they go
    // through a scratch store. A shapes file that will not parse is an error
    // here rather than an empty shapes graph, because an empty shapes graph
    // conforms vacuously and that is the one answer nobody wants by accident.
    let scratch = GraphStore::new();
    scratch.load_turtle(shapes_ttl, None)?;
    let shapes_nt = scratch.snapshot("ntriples")?;

    // A directory per CALL, not per process.
    //
    // This was keyed on the pid alone, which is the same for every call in a
    // process, so two concurrent validations wrote each other's `data.nt` and
    // read it back. Two tests in this suite came back with byte-identical
    // reports over different graphs, one of them claiming `conforms: true` for
    // a graph that violates its shape, with a theorem name attached. The
    // server is the concurrent case that matters: `onto_shacl` is a tool
    // several sessions can call at once, and a verdict computed over another
    // caller's data is the worst thing this layer could produce.
    let serial = NEXT_RUN.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("oo-shacl-{}-{serial}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let data_path = dir.join("data.nt");
    let shapes_path = dir.join("shapes.nt");
    std::fs::write(&data_path, &data_nt)?;
    std::fs::write(&shapes_path, &shapes_nt)?;

    let mut cmd = Command::new(checker.path());
    cmd.arg("validate").arg(&data_path).arg(&shapes_path);
    let run = CheckerRun::spawn(&checker, cmd)?;
    let _ = std::fs::remove_dir_all(&dir);

    // The checker's own JSON, echoed rather than restated. Its verdict words
    // are its to print; this module quotes bytes it read from that stdout.
    let report: serde_json::Value =
        serde_json::from_str(run.stdout()).unwrap_or(serde_json::Value::Null);

    match run.exit() {
        0 => {
            let conforms = report
                .get("conforms")
                .and_then(|v| v.as_bool())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "oo-shacl exited 0 without a boolean `conforms`; refusing to guess \
                         one. stdout was: {}",
                        run.stdout()
                    )
                })?;
            let token = run.accepted("Shacl.validate_spec").ok_or_else(|| {
                anyhow::anyhow!("exit 0 did not yield a certificate token, which cannot happen")
            })?;
            let v = ShaclVerified::Checked(conforms, token);
            Ok(describe(&v, &report, run.binary().display().to_string()))
        }
        3 => {
            let why = report
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("the evaluator declined to judge and named no reason")
                .to_string();
            Ok(describe(
                &ShaclVerified::Undetermined(why),
                &report,
                run.binary().display().to_string(),
            ))
        }
        2 => Ok(serde_json::json!({
            "verified": false,
            "conforms": serde_json::Value::Null,
            "error": "the verified evaluator could not read the data or the shapes",
            "detail": run.output(),
            "note": "exit 2 is a parse failure, not a conformance answer",
        })),
        other => Ok(serde_json::json!({
            "verified": false,
            "conforms": serde_json::Value::Null,
            "error": format!("the verified evaluator exited {other}, which its CLI does not define"),
            "detail": run.output(),
        })),
    }
}

fn describe(v: &ShaclVerified, report: &serde_json::Value, binary: String) -> serde_json::Value {
    match v {
        ShaclVerified::Checked(conforms, token) => serde_json::json!({
            "verified": true,
            "conforms": conforms,
            "theorem": token.theorem(),
            "checker": binary,
            "report": report,
            "covers": "SHACL Core under Shacl/Spec.lean. sh:sparql and user-defined \
                       constraint components are REFUSED by this evaluator rather than \
                       skipped, so a verdict here is about every constraint it read.",
            "does_not_cover": "that Shacl/Spec.lean is the W3C Recommendation; it is a \
                               reading of it, measured against the Working Group's suite. \
                               Nor that the shapes compiler or the N-Triples parser is \
                               correct: both are outside the theorem and refuse rather \
                               than guess.",
        }),
        ShaclVerified::Undetermined(why) => serde_json::json!({
            "verified": true,
            "conforms": serde_json::Value::Null,
            "undetermined": why,
            "checker": binary,
            "report": report,
            "note": "the evaluator declined to judge. This is NOT conformance: the \
                     difference between 'everything conforms' and 'I did not check \
                     everything' is the whole reason this path exists.",
        }),
    }
}
