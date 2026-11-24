// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! `cls-maxc1` is certifiable only for the zero the checker actually compares against.
//!
//! The detector is deliberately wider than the checker. `literal_is_zero` reads
//! the lexical form and nothing else, so `owl:maxCardinality 0` — which Turtle
//! types `xsd:integer` — is found, exactly as a canonical
//! `"0"^^xsd:nonNegativeInteger` is. `lean/OOCert/Refute.lean` compares terms
//! syntactically against the canonical spelling, so a refutation of the integer
//! spelling is rejected; writing one anyway would make `onto_pack` refuse a
//! genuinely contradictory graph.
//!
//! So certifiability is asked per witness, not per rule, and this file holds the
//! four ends of that:
//!
//! * the integer spelling is DETECTED and REPORTED with `certifiable: false`,
//!   and no refutation is written for it;
//! * the canonical spelling is certified end to end by the real checker;
//! * a graph carrying both that non-canonical clash and a `cax-dw` clash gets a
//!   refutation for `cax-dw`, which is the certifiable witness;
//! * a rejected run says `clash_found_by_this_engine` in the nested refutation
//!   verdict, because "not yet checked" is false once the checker has answered.

mod common;

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::{InferenceTarget, Reasoner};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

const PREFIXES: &str = "@prefix : <http://ex.org/> .\n\
    @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n";

/// `owl:maxCardinality 0` written as a Turtle integer, so the object is
/// `"0"^^xsd:integer`: a zero the detector sees and the checker does not accept.
const INTEGER_ZERO: &str = ":NoChildren a owl:Restriction ;\n\
    owl:onProperty :parentOf ;\n\
    owl:maxCardinality 0 .\n\
    :bob a :NoChildren ; :parentOf :carol .\n";

/// The same restriction plus a `cax-dw` contradiction. The `cax-dw` witness is
/// the one a checkable refutation exists for, so it must be the one written.
const INTEGER_ZERO_AND_CAX_DW: &str = ":NoChildren a owl:Restriction ;\n\
    owl:onProperty :parentOf ;\n\
    owl:maxCardinality 0 .\n\
    :bob a :NoChildren ; :parentOf :carol .\n\
    :Herbivore owl:disjointWith :Carnivore .\n\
    :Lion rdfs:subClassOf :BigCat . :BigCat rdfs:subClassOf :Carnivore .\n\
    :leo a :Lion ; a :Herbivore .\n";

/// A plain `cax-dw` contradiction, used to make the checker answer.
const INCONSISTENT: &str = ":Herbivore owl:disjointWith :Carnivore .\n\
    :Lion rdfs:subClassOf :BigCat . :BigCat rdfs:subClassOf :Carnivore .\n\
    :leo a :Lion ; a :Herbivore .\n";

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("oo-clash-spelling-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn ontology(body: &str) -> String {
    format!("{PREFIXES}{body}")
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

/// `OO_REFUTE` is process-global, and one test here points it at a rejecting
/// script. Every test takes this lock, so no sibling can observe the variable
/// in a state it did not choose, and a legacy redirect cannot leak into the
/// legs that expect the real checker.
fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Set `$OO_REFUTE` and put back whatever was there on drop. The caller holds
/// [`env_lock`]; the guard is deliberately not taken here, because a test that
/// already holds it cannot take it again.
struct RefuteEnv {
    had: Option<std::ffi::OsString>,
}

impl RefuteEnv {
    fn set(value: &Path) -> Self {
        let had = std::env::var_os("OO_REFUTE");
        // SAFETY: every test in this binary holds `env_lock()` for the whole
        // run, and no other thread here reads OO_REFUTE outside that guard.
        unsafe { std::env::set_var("OO_REFUTE", value) };
        Self { had }
    }
}

impl Drop for RefuteEnv {
    fn drop(&mut self) {
        match self.had.take() {
            Some(v) => unsafe { std::env::set_var("OO_REFUTE", v) },
            None => unsafe { std::env::remove_var("OO_REFUTE") },
        }
    }
}

/// A script that answers like a rejecting `oo-refute` and exits 1.
#[cfg(unix)]
fn refusing_checker() -> PathBuf {
    let path = scratch("fake-refuter").join("oo-refute.sh");
    let body = "#!/bin/sh\n\
                cat <<'EOF'\n\
                {\"ok\":false,\"format\":\"oo-refute/1\",\"reason\":\"the fake refuter refused \
                the file\",\"means\":\"not a proof of consistency\"}\n\
                EOF\n\
                exit 1\n";
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

fn listed<'a>(r: &'a serde_json::Value, rule: &str) -> &'a serde_json::Value {
    r["inconsistency"]["clashes"]
        .as_array()
        .expect("clashes is an array")
        .iter()
        .find(|c| c["rule"] == rule)
        .unwrap_or_else(|| panic!("no `{rule}` entry in {:?}", r["inconsistency"]["clashes"]))
}

fn string_values(v: &serde_json::Value, out: &mut Vec<String>) {
    match v {
        serde_json::Value::String(s) => out.push(s.clone()),
        serde_json::Value::Array(a) => a.iter().for_each(|x| string_values(x, out)),
        serde_json::Value::Object(o) => o.values().for_each(|x| string_values(x, out)),
        _ => {}
    }
}

/// The real checker, or a skip that becomes a failure under OO_REQUIRE_FIXTURES.
fn real_checker_or_skip() -> bool {
    let real = repo().join("lean/.lake/build/bin/oo-refute");
    common::skip_unless(
        real.is_file(),
        &format!("the real oo-refute at {}", real.display()),
        "cd lean && lake build oo-refute",
    )
}

#[test]
fn an_integer_typed_zero_is_detected_and_not_called_certifiable() {
    let _env = env_lock();
    let dir = scratch("integer-zero");
    let r = reason(&ontology(INTEGER_ZERO), &dir);

    assert_eq!(
        r["inconsistency"]["found"],
        serde_json::Value::Bool(true),
        "the restriction admits no value and :bob has one: it is a real clash. {r}"
    );
    assert_eq!(
        r["inconsistency"]["by_rule"]["cls-maxc1"],
        serde_json::json!(1),
        "the integer spelling must still be DETECTED. {r}"
    );
    let clash = listed(&r, "cls-maxc1");
    assert_eq!(
        clash["certifiable"],
        serde_json::Value::Bool(false),
        "the checker compares terms and requires \"0\"^^xsd:nonNegativeInteger, so this witness \
         is not certifiable even though the rule is. {clash}"
    );
    assert_eq!(
        r["inconsistency"]["verdict"],
        serde_json::json!("clash_found_by_this_engine"),
        "without an accepted refutation the engine states its own word. {r}"
    );
    assert_eq!(
        r["inconsistency"]["checked_by_lean"],
        serde_json::Value::Bool(false)
    );
    assert_eq!(
        r["inconsistency"]["refutation"]["written"],
        serde_json::Value::Bool(false),
        "no refutation can be written for a witness the checker would reject. {r}"
    );
    let why = r["inconsistency"]["refutation"]["why"]
        .as_str()
        .expect("a written:false report says why");
    assert!(
        why.contains("canonical") && why.contains("nonNegativeInteger"),
        "the reason has to name the spelling condition: {why}"
    );
    assert!(
        r["inconsistency"]["refutation"].get("checker").is_none(),
        "nothing ran, so there is no checker block: {r}"
    );
    assert!(
        !dir.join("refutation.tsv").exists(),
        "a refused witness must not leave a certificate-shaped file behind"
    );
}

#[test]
fn the_canonical_zero_is_certified_by_the_real_checker() {
    let _env = env_lock();
    if real_checker_or_skip() {
        return;
    }
    let dir = scratch("canonical-zero");
    let fixture = repo().join("tests/fixtures/clash-coverage/cls-maxc1.ttl");
    let ttl = std::fs::read_to_string(&fixture).expect("read the canonical fixture");
    let r = reason(&ttl, &dir);

    assert_eq!(
        r["inconsistency"]["by_rule"]["cls-maxc1"],
        serde_json::json!(1),
        "{r}"
    );
    assert_eq!(
        listed(&r, "cls-maxc1")["certifiable"],
        serde_json::Value::Bool(true)
    );
    assert_eq!(
        r["inconsistency"]["refutation"]["written"],
        serde_json::Value::Bool(true)
    );
    assert_eq!(
        r["inconsistency"]["refutation"]["rule"],
        serde_json::json!("cls-maxc1")
    );
    assert_eq!(
        r["inconsistency"]["verdict"],
        serde_json::json!("refutation_accepted_by_this_machines_checker"),
        "the canonical spelling is the witness the checker accepts. {r}"
    );
    assert_eq!(
        r["inconsistency"]["checked_by_lean"],
        serde_json::Value::Bool(true)
    );
    assert_eq!(
        r["inconsistency"]["refutation"]["verdict"],
        r["inconsistency"]["verdict"]
    );
    assert_eq!(
        r["inconsistency"]["refutation"]["checker"]["status"],
        serde_json::json!("accepted")
    );
}

#[test]
fn a_non_certifiable_witness_does_not_shadow_a_certifiable_one() {
    let _env = env_lock();
    if real_checker_or_skip() {
        return;
    }
    let dir = scratch("mixed");
    let r = reason(&ontology(INTEGER_ZERO_AND_CAX_DW), &dir);

    assert_eq!(
        r["inconsistency"]["by_rule"]["cls-maxc1"],
        serde_json::json!(1),
        "{r}"
    );
    assert_eq!(
        r["inconsistency"]["by_rule"]["cax-dw"],
        serde_json::json!(1),
        "{r}"
    );
    assert_eq!(
        listed(&r, "cls-maxc1")["certifiable"],
        serde_json::Value::Bool(false),
        "the flags are per instance, not per rule. {r}"
    );
    assert_eq!(
        listed(&r, "cax-dw")["certifiable"],
        serde_json::Value::Bool(true)
    );
    assert_eq!(
        r["inconsistency"]["refutation"]["rule"],
        serde_json::json!("cax-dw"),
        "the writer must pass over the integer-typed cls-maxc1 witness and pick the one that \
         can be checked. {r}"
    );
    assert_eq!(
        r["inconsistency"]["refutation"]["written"],
        serde_json::Value::Bool(true)
    );
    assert_eq!(
        r["inconsistency"]["verdict"],
        serde_json::json!("refutation_accepted_by_this_machines_checker")
    );
    assert_eq!(
        r["inconsistency"]["checked_by_lean"],
        serde_json::Value::Bool(true)
    );
    assert_eq!(
        r["inconsistency"]["refutation"]["checker"]["status"],
        serde_json::json!("accepted")
    );
}

#[cfg(unix)]
#[test]
fn a_rejected_run_says_the_engine_found_a_clash_not_not_yet_checked() {
    let _env = env_lock();
    let dir = scratch("rejected");
    let _refute = RefuteEnv::set(&refusing_checker());
    let r = reason(&ontology(INCONSISTENT), &dir);

    assert_eq!(
        r["inconsistency"]["refutation"]["written"],
        serde_json::Value::Bool(true),
        "the file was written; the checker then refused it. {r}"
    );
    assert_eq!(
        r["inconsistency"]["verdict"],
        serde_json::json!("clash_found_by_this_engine"),
        "a rejection does not mint the accepted word. {r}"
    );
    assert_eq!(
        r["inconsistency"]["checked_by_lean"],
        serde_json::Value::Bool(false)
    );
    assert_eq!(
        r["inconsistency"]["refutation"]["verdict"],
        serde_json::json!("clash_found_by_this_engine"),
        "the checker answered, so `refutation_written_not_yet_checked` is now false. {r}"
    );
    assert_eq!(
        r["inconsistency"]["refutation"]["checker"]["status"],
        serde_json::json!("rejected")
    );

    let mut strings = Vec::new();
    string_values(&r, &mut strings);
    for s in &strings {
        assert_ne!(
            s, "unsatisfiable_under_disjointness",
            "the checker's own verdict must never enter an engine field: {r}"
        );
        assert!(
            !s.contains("OOCert.refutation_fast_sound"),
            "the checker's theorem name must never enter an engine field: {s}"
        );
    }
}
