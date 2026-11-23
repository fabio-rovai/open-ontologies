// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! The R1 wiring, seen from the reasoner's report.
//!
//! `tests/lean_refutation_producer_test.rs` proves the engine writes a
//! refutation `oo-refute` accepts, by calling the checker itself. This file
//! proves the ENGINE does the calling, and that its report keeps the three
//! outcomes apart:
//!
//! * an accepting checker mints `refutation_accepted_by_this_machines_checker`
//!   and sets `checked_by_lean: true`;
//! * a checker that exits 0 but names some other theorem does NOT mint it —
//!   acceptance is the theorem name, not the exit code;
//! * a checker that cannot be found reports
//!   `checker_absent_nothing_was_checked` and a false `checked_by_lean`, never
//!   a pass.
//!
//! `$OO_REFUTE` is the seam the last two use: it is an explicit instruction,
//! so discovery does not fall back. Every test here takes `env_lock()`, because
//! `std::env::set_var` is process-global and edition 2024 made it `unsafe` for
//! exactly that reason, and `common::exec_gate()` across the write and the
//! fork, the same way `tests/fol_prove_certified_test.rs` does.

mod common;

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::{InferenceTarget, Reasoner};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

const PREFIXES: &str = "@prefix : <http://ex.org/> .\n\
    @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n";

/// The same refutable graph the producer test uses. `leo` is in two disjoint
/// classes two `rdfs9` hops away from what is asserted, so the generated
/// refutation carries a derivation prefix and the checker has real work.
const INCONSISTENT: &str = ":Herbivore owl:disjointWith :Carnivore .\n\
    :Lion rdfs:subClassOf :BigCat . :BigCat rdfs:subClassOf :Carnivore .\n\
    :leo a :Lion ; a :Herbivore .\n";

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("oo-refute-checked-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn reason(ttl: &str, dir: &Path) -> serde_json::Value {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(ttl, None).unwrap();
    let out = Reasoner::run_full(
        &store,
        "owl-rl",
        false,
        InferenceTarget::DefaultGraph,
        Some(dir),
    )
    .unwrap();
    serde_json::from_str(&out).unwrap()
}

fn ontology(body: &str) -> String {
    format!("{PREFIXES}{body}")
}

/// `OO_REFUTE` is process-global state and this binary's three tests all move
/// it. A mutex per test binary is enough, because cargo runs each integration
/// test file in its own process.
fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Point `$OO_REFUTE` at `value` for the duration of `f`, then put back
/// whatever was there. The guard is held across the run so no sibling test can
/// observe the variable in a state it did not choose.
fn with_refute_env<T>(value: &Path, f: impl FnOnce() -> T) -> T {
    let _guard = env_lock();
    let had = std::env::var("OO_REFUTE").ok();
    // SAFETY: `env_lock()` serialises every mutation of OO_REFUTE in this
    // process, and no other thread here reads it.
    unsafe { std::env::set_var("OO_REFUTE", value) };
    let out = f();
    match had {
        Some(v) => unsafe { std::env::set_var("OO_REFUTE", v) },
        None => unsafe { std::env::remove_var("OO_REFUTE") },
    }
    out
}

fn fake_checker(name: &str, code: i32, theorem: &str) -> PathBuf {
    let path = scratch(name).join("fake-oo-refute.sh");
    let body = format!(
        "#!/bin/sh\n\
         cat <<'EOF'\n\
         {{\"ok\":true,\"format\":\"oo-refute/1\",\"asserted\":1,\"prefix\":1,\"rule\":\"cax-dw\",\
         \"verdict\":\"unsatisfiable_under_disjointness\",\"theorem\":\"{theorem}\",\
         \"means\":\"a fake checker's own line\"}}\n\
         EOF\n\
         exit {code}\n"
    );
    {
        let _gate = common::exec_gate();
        std::fs::write(&path, body).unwrap();
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

#[test]
fn an_accepting_checker_mints_the_certified_word() {
    let real = repo().join("lean/.lake/build/bin/oo-refute");
    if common::skip_unless(
        real.is_file(),
        &format!("the real oo-refute at {}", real.display()),
        "cd lean && lake build oo-refute",
    ) {
        return;
    }

    let dir = scratch("accepted");
    let r = with_refute_env(&real, || reason(&ontology(INCONSISTENT), &dir));

    assert_eq!(r["inconsistency"]["found"], true, "{r}");
    assert_eq!(r["inconsistency"]["refutation"]["written"], true, "{r}");
    assert_eq!(r["inconsistency"]["refutation"]["rule"], "cax-dw", "{r}");
    // THE claim of R1: the run itself ran the checker, and only then does the
    // certified word appear.
    assert_eq!(
        r["inconsistency"]["verdict"], "refutation_accepted_by_this_machines_checker",
        "an accepting checker must mint its own word: {r}"
    );
    assert_eq!(r["inconsistency"]["checked_by_lean"], true, "{r}");
    assert_eq!(
        r["inconsistency"]["refutation"]["verdict"], "refutation_accepted_by_this_machines_checker",
        "{r}"
    );
    let checker = &r["inconsistency"]["refutation"]["checker"];
    assert_eq!(checker["status"], "accepted", "{r}");
    assert_eq!(checker["exit"], 0, "{r}");
}

#[test]
fn a_checker_that_says_ok_with_the_wrong_theorem_is_not_accepted() {
    // The whole point of `accepted_naming`: exit status is a claim by the
    // binary, the theorem name is the claim's content. A checker that exits 0
    // and names `OOCert.certificate_sound` has accepted a DIFFERENT theorem,
    // so this run earns nothing and the engine says so.
    let fake = fake_checker("fake-wrong-theorem", 0, "OOCert.certificate_sound");
    let dir = scratch("wrong-theorem");
    let r = with_refute_env(&fake, || reason(&ontology(INCONSISTENT), &dir));

    assert_eq!(r["inconsistency"]["found"], true, "{r}");
    assert_eq!(r["inconsistency"]["refutation"]["written"], true, "{r}");
    assert_eq!(
        r["inconsistency"]["verdict"], "clash_found_by_this_engine",
        "exit 0 with the wrong theorem is not acceptance: {r}"
    );
    assert_eq!(r["inconsistency"]["checked_by_lean"], false, "{r}");
    // The checker DID answer, with something this crate cannot read. So the
    // file is no longer "not yet checked": the engine's own word stands, and
    // the unreadable bytes sit beside it under `checker.stdout`.
    assert_eq!(
        r["inconsistency"]["refutation"]["verdict"], "clash_found_by_this_engine",
        "{r}"
    );
    let checker = &r["inconsistency"]["refutation"]["checker"];
    assert_eq!(checker["status"], "unreadable", "{r}");
    // The code the binary ACTUALLY returned, not the code the status implies:
    // exit 0 with the wrong theorem is a different defect from exit 2.
    assert_eq!(checker["exit"], 0, "{r}");
    // The bytes are kept, because they are what a human needs to see why a
    // checker that said `ok` did not persuade the engine.
    assert!(
        checker["stdout"]
            .as_str()
            .unwrap_or_default()
            .contains("OOCert.certificate_sound"),
        "{r}"
    );
}

#[test]
fn a_missing_checker_reports_nothing_was_checked() {
    // The absence answer this project refuses to round to a pass. An explicit
    // OO_REFUTE that does not exist is an instruction, not a hint, so there is
    // no fallback to the built binary either.
    let fake = scratch("fake-absent").join("does-not-exist");
    let dir = scratch("absent");
    let r = with_refute_env(&fake, || reason(&ontology(INCONSISTENT), &dir));

    assert_eq!(r["inconsistency"]["found"], true, "{r}");
    assert_eq!(r["inconsistency"]["refutation"]["written"], true, "{r}");
    assert_eq!(
        r["inconsistency"]["verdict"], "clash_found_by_this_engine",
        "{r}"
    );
    assert_eq!(r["inconsistency"]["checked_by_lean"], false, "{r}");
    assert_eq!(
        r["inconsistency"]["refutation"]["verdict"], "refutation_written_not_yet_checked",
        "{r}"
    );
    let checker = &r["inconsistency"]["refutation"]["checker"];
    assert_eq!(
        checker["status"], "checker_absent_nothing_was_checked",
        "{r}"
    );
    assert!(checker["exit"].is_null(), "{r}");
    let what = checker["what"].as_str().unwrap_or_default();
    assert!(
        what.contains("lake build oo-refute"),
        "the absence must name the build that fixes it: {what}"
    );
    // Not a pass, and not silence: the engine's own finding still stands.
    assert_eq!(r["inconsistency"]["found"], true, "{r}");
}
