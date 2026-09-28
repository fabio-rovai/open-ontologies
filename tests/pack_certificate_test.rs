//! Proof-carrying packs: the receiver re-runs the checker and refuses the load.
//!
//! `onto_pack` used to write a graph, a checksum over it, and the lint and
//! enforce results the SENDER'S engine produced. A receiver could verify
//! integrity and nothing else: the evidence was an assertion by a machine they
//! had to trust. A pack now carries the derivation certificate too, bound to
//! the graph by a second digest, and `onto_unpack` runs this machine's Lean
//! checker over it.
//!
//! Four outcomes have to stay apart, and the fourth is the one that matters:
//!
//!   1. no certificate in the pack;
//!   2. a certificate this machine's checker ACCEPTED;
//!   3. a certificate it REFUSED, which stops the load;
//!   4. no checker in this build, so NOTHING WAS CHECKED.
//!
//! The last is not a pass, and the assertion that pins it is the one that looks
//! for the accepted word in the whole serialised report and requires it to be
//! absent.
//!
//! The two legs that need a real `oo-cert` skip loudly through
//! `common::skip_unless`; the `lean` job runs this file with
//! `OO_REQUIRE_FIXTURES=1`, which turns that skip into a failure. Everything
//! else runs with no toolchain, by handing the checker in as a path.

mod common;

use open_ontologies::graph::GraphStore;
use open_ontologies::pack::{PackCertVerdict, PackRequest, PackedCertificate, Packer, UnpackOptions, content_digest};
use open_ontologies::reason::{InferenceTarget, Reasoner};
use open_ontologies::verdict::CHECKER_OWNED_WORDS;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

// ───────────────────────────────────────────────────────────────────────────
// Fixtures
// ───────────────────────────────────────────────────────────────────────────

const PREFIXES: &str = r#"
    @prefix : <http://ex.org/> .
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
"#;

/// An ontology whose `owl:intersectionOf` puts BLANK NODES in both the graph
/// and `asserted.tsv`.
///
/// Deliberate. The coverage check compares the certificate's `asserted.tsv`
/// against `reason::asserted_bytes` over a store staged from the packed
/// N-Triples, so it depends on `serialize("ntriples")` then `load_ntriples`
/// preserving blank-node labels. `GraphStore::load_lines` does not rename them,
/// so it does today. If that ever changes, every pack whose ontology uses an
/// `rdf:List` starts reporting `certificate_not_about_this_pack` and refusing
/// to load, and this file is what says so rather than a support ticket.
const LIST_BEARING: &str = r#"
    :A rdfs:subClassOf :B . :B rdfs:subClassOf :C . :a a :A .
    :I owl:intersectionOf ( :M1 :M2 ) . :i a :M1 , :M2 .
"#;

/// A different ontology, for the certificate that is about somebody else's
/// graph.
const OTHER_GRAPH: &str = r#"
    :P rdfs:subClassOf :Q . :Q rdfs:subClassOf :R . :p a :P .
"#;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn scratch(name: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let serial = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "oo-packcert-{name}-{}-{serial}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Reason over `ttl` with a certificate, and return (store, certificate dir).
fn certify(ttl: &str, dir: &Path) -> Arc<GraphStore> {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(&format!("{PREFIXES}{ttl}"), None).unwrap();
    Reasoner::run_full(
        &store,
        "owl-rl-ext",
        true,
        InferenceTarget::DefaultGraph,
        Some(dir),
    )
    .unwrap();
    store
}

/// Write a pack carrying the certificate in `cert_dir`, and return its path.
fn pack_with_certificate(store: &Arc<GraphStore>, cert_dir: &Path, out: &Path) -> Value {
    let path = out.join("carried.oopack");
    let json = Packer::new(store.clone())
        .pack(&PackRequest {
            path: path.to_str().unwrap(),
            name: "carried",
            version: "1.0.0",
            evidence: None,
            certificate_dir: Some(cert_dir),
            profile_claimed: Some("owl-rl-ext"),
        })
        .expect("pack with a certificate");
    serde_json::from_str(&json).expect("the pack result is JSON")
}

fn read_pack(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn write_pack(path: &Path, pack: &Value) {
    std::fs::write(path, serde_json::to_string_pretty(pack).unwrap()).unwrap();
}

/// Unpack into a fresh store, so nothing a previous leg loaded can be mistaken
/// for what this one loaded.
fn unpack(path: &Path, opts: &UnpackOptions) -> Value {
    let store = Arc::new(GraphStore::new());
    let gate = common::exec_gate();
    let out = Packer::new(store)
        .unpack(path.to_str().unwrap(), opts)
        .expect("unpack returns a report rather than an error");
    drop(gate);
    serde_json::from_str(&out).expect("the unpack report is JSON")
}

fn options(checker: Option<PathBuf>) -> UnpackOptions {
    UnpackOptions {
        verify_only: false,
        check_certificate: true,
        checker,
        certificate_out_dir: None,
        load_even_if_refused: false,
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Checkers, fake and real
// ───────────────────────────────────────────────────────────────────────────

/// A script that prints what a checker prints and exits with `code`.
///
/// A fresh path per call. `cargo test` runs these as parallel threads of one
/// process, and a shared path means one thread rewriting a script while another
/// execs it, which on Linux is ETXTBSY. `common::exec_gate` covers the other
/// half of the same race: any write while any fork is in flight.
fn fake_checker(code: i32) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join("oo-packcert-fake-checkers");
    std::fs::create_dir_all(&dir).unwrap();
    let ext = if cfg!(windows) { "cmd" } else { "sh" };
    let serial = NEXT.fetch_add(1, Ordering::Relaxed);
    let p = dir.join(format!("exit{code}_{}_{serial}.{ext}", std::process::id()));
    // It PRINTS the theorem, because a real checker does and the mint reads it.
    // A script that exits zero in silence names nothing and mints nothing.
    let say = r#"{"ok":true,"theorem":"OOCert.certificate_sound","checker":{"name":"fake","sha256":"0"}}"#;
    let body = if cfg!(windows) {
        format!("@echo off\r\necho {say}\r\nexit /b {code}\r\n")
    } else {
        format!("#!/bin/sh\necho '{say}'\nexit {code}\n")
    };
    {
        let _gate = common::exec_gate();
        std::fs::write(&p, body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    p
}

fn lean_dir() -> PathBuf {
    repo().join("lean")
}

fn lake_available() -> bool {
    // `.current_dir(lean_dir())` is not cosmetic: elan resolves the toolchain
    // from the working directory's `lean-toolchain`, and the crate root has
    // none in its ancestry.
    Command::new("lake")
        .arg("--version")
        .current_dir(lean_dir())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn skip() -> bool {
    common::skip_unless(
        lake_available(),
        "lake (the Lean 4 build tool)",
        "install elan from https://github.com/leanprover/elan; lean/lean-toolchain pins the version",
    )
}

/// The real `oo-cert`, built once per test binary. The proofs are part of the
/// build, so a failure here is a failure and never a skip.
fn real_checker() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT
        .get_or_init(|| {
            let out = Command::new("lake")
                .arg("build")
                .current_dir(lean_dir())
                .output()
                .expect("run lake build");
            assert!(
                out.status.success(),
                "lake build failed:\n{}\n{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            let exe = lean_dir()
                .join(".lake")
                .join("build")
                .join("bin")
                .join("oo-cert");
            assert!(exe.exists(), "checker binary missing at {}", exe.display());
            exe
        })
        .clone()
}

/// The forged line from `tests/lean_certificate_test.rs`: an `rdfs9` step whose
/// premises do not support its conclusion.
const FORGED_RDFS9: &str = "rdfs9\t<http://ex.org/a>\t<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>\t<http://ex.org/Z>\t<http://ex.org/a>\t<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>\t<http://ex.org/A>\t<http://ex.org/A>\t<http://www.w3.org/2000/01/rdf-schema#subClassOf>\t<http://ex.org/B>\n";

// ───────────────────────────────────────────────────────────────────────────
// 1. Backward compatibility
// ───────────────────────────────────────────────────────────────────────────

/// A pack written before any of this existed still loads, and says it carries
/// no proof rather than failing.
///
/// Written BY HAND, with exactly the six manifest fields the old `Manifest`
/// had, so it is a genuine pre-change artefact and not a new pack with a field
/// omitted.
///
/// WHAT THIS DOES AND DOES NOT REST ON, measured rather than argued. Deleting
/// `#[serde(default)]` from both new manifest fields leaves this test GREEN:
/// serde's derive already reads a MISSING field of type `Option<T>` as `None`,
/// so the attribute is explicitness and not the guarantee. The design note this
/// was built from claimed the opposite, and the mutation disproved it. What the
/// guarantee actually rests on is the `(None, None)` arm of the digest match in
/// `Packer::unpack`, which lets a pack with no certificate and no content
/// digest through. Making that arm refuse, which is the realistic regression
/// the moment somebody decides every pack must carry the new digest, turns this
/// test red.
#[test]
fn an_old_pack_loads_and_reports_that_it_carries_no_certificate() {
    let dir = scratch("old-pack");
    let graph = "<http://ex.org/a> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://ex.org/A> .\n";
    let mut h = <sha2::Sha256 as sha2::Digest>::new();
    sha2::Digest::update(&mut h, graph.as_bytes());
    let sha = format!("{:x}", sha2::Digest::finalize(h));
    let old = serde_json::json!({
        "manifest": {
            "name": "legacy",
            "version": "1.0.0",
            "created_at": "2026-01-01T00:00:00+00:00",
            "tool_version": "1.6.0",
            "triples": 1,
            "sha256": sha,
        },
        "graph": graph,
    });
    let path = dir.join("legacy.oopack");
    write_pack(&path, &old);

    let r = unpack(&path, &options(None));
    assert_eq!(r["verified"], true, "{r:#}");
    assert_eq!(r["loaded"], true, "an old pack must still load: {r:#}");
    assert_eq!(r["triples_loaded"], 1, "{r:#}");
    assert_eq!(r["certificate"]["present"], false, "{r:#}");
    assert_eq!(
        r["certificate"]["verdict"], "no_certificate_in_pack",
        "an old pack carries no proof and the report has to say which: {r:#}"
    );
    assert!(
        r["certificate"]["theorem"].is_null(),
        "no theorem without a checker: {r:#}"
    );
    // The manifest comes back as it was written. A future change that invents
    // an empty certificate object for an old pack would be reporting a proof
    // nobody shipped.
    let m = r["manifest"].as_object().expect("a manifest object");
    assert!(
        !m.contains_key("certificate") && !m.contains_key("content_sha256"),
        "an old pack's manifest must not grow fields it never had: {r:#}"
    );

    // And a pack this build writes with no certificate lands in the same place,
    // while still carrying the new digest.
    let fresh = dir.join("fresh.oopack");
    let store = Arc::new(GraphStore::new());
    store.load_ntriples(graph).unwrap();
    let written: Value = serde_json::from_str(
        &Packer::new(store)
            .pack(&PackRequest {
                path: fresh.to_str().unwrap(),
                name: "fresh",
                version: "1.0.0",
                evidence: None,
                certificate_dir: None,
                profile_claimed: None,
            })
            .unwrap(),
    )
    .unwrap();
    assert!(
        written["content_sha256"].is_string(),
        "every pack this build writes is bound: {written:#}"
    );
    assert!(written["certificate"].is_null(), "{written:#}");
    let back = unpack(&fresh, &options(None));
    assert_eq!(back["certificate"]["verdict"], "no_certificate_in_pack", "{back:#}");
    assert_eq!(back["loaded"], true, "{back:#}");
}

// ───────────────────────────────────────────────────────────────────────────
// 2. The four outcomes
// ───────────────────────────────────────────────────────────────────────────

/// The receiver can tell the four outcomes apart, and "nothing was checked"
/// never renders as "it passed".
#[test]
fn the_receiver_outcomes_are_distinguishable() {
    let cert_dir = scratch("outcomes-cert");
    let out = scratch("outcomes");
    let store = certify(LIST_BEARING, &cert_dir);
    let packed = pack_with_certificate(&store, &cert_dir, &out);
    let path = out.join("carried.oopack");

    assert_eq!(packed["certificate"]["checked_by_this_packer"], false, "{packed:#}");
    assert!(
        packed["pack_json_bytes"].as_u64().unwrap() > 0,
        "the pack size is reported so the cost of carrying a proof is visible: {packed:#}"
    );

    // (a) a checker that exits zero and names the theorem.
    let accepted = unpack(&path, &options(Some(fake_checker(0))));
    assert_eq!(
        accepted["certificate"]["verdict"], "certificate_accepted",
        "{accepted:#}"
    );
    assert_eq!(
        accepted["certificate"]["theorem"], "OOCert.certificate_sound",
        "{accepted:#}"
    );
    assert_eq!(accepted["loaded"], true, "{accepted:#}");
    assert_eq!(accepted["certificate"]["checked_here"], true, "{accepted:#}");
    // The limits, in the receiver's report and not only in a comment.
    let does_not = accepted["certificate"]["does_not_mean"]
        .as_str()
        .expect("an acceptance states what it does not settle");
    assert!(
        does_not.contains("NOT proved") && does_not.contains("outside the fragment"),
        "the report must say the asserted triples are not proved and that axioms outside \
         the fragment are invisible: {does_not}"
    );
    assert!(
        accepted["certificate"]["checker_limit"]
            .as_str()
            .unwrap_or_default()
            .contains("binary THIS machine chose"),
        "the report must say the checker is a binary the receiver chose: {accepted:#}"
    );
    assert_eq!(
        accepted["certificate"]["checker_says_it_is"]["name"], "fake",
        "the checker's own self-identifying block is echoed so a reader can match its \
         sha256 against a release: {accepted:#}"
    );

    // The scratch directory is removed once the checker has read it, so the
    // command the report prints names files that are gone, and the report says
    // so rather than letting a reader find out.
    assert_eq!(
        accepted["certificate"]["certificate_dir_kept"], false,
        "{accepted:#}"
    );
    assert!(
        accepted["certificate"]["check_with_means"]
            .as_str()
            .unwrap_or_default()
            .contains("no longer exist"),
        "a command over a deleted directory must be labelled as one: {accepted:#}"
    );
    assert!(
        !PathBuf::from(
            accepted["certificate"]["certificate_dir"]
                .as_str()
                .unwrap()
        )
        .exists(),
        "the scratch directory must actually be gone: {accepted:#}"
    );

    // ... and with certificate_out_dir the files stay, so the command runs.
    let kept_dir = scratch("outcomes-kept").join("cert");
    let kept = unpack(
        &path,
        &UnpackOptions {
            certificate_out_dir: Some(kept_dir.clone()),
            ..options(Some(fake_checker(0)))
        },
    );
    assert_eq!(kept["certificate"]["certificate_dir_kept"], true, "{kept:#}");
    assert!(
        kept_dir.join("asserted.tsv").exists() && kept_dir.join("derivations.tsv").exists(),
        "the files a receiver was told to re-check must be there: {}",
        kept_dir.display()
    );
    assert!(
        kept["certificate"]["check_with_means"]
            .as_str()
            .unwrap_or_default()
            .contains("repeats the check exactly"),
        "{kept:#}"
    );

    // (b) exit 1: refused, and the load is blocked.
    let refused = unpack(&path, &options(Some(fake_checker(1))));
    assert_eq!(
        refused["certificate"]["verdict"], "certificate_refused",
        "{refused:#}"
    );
    assert_eq!(
        refused["loaded"], false,
        "a graph whose own proof is refused must not be promoted: {refused:#}"
    );
    assert!(refused["certificate"]["theorem"].is_null(), "{refused:#}");

    // ... unless the caller asks for it, which is the documented escape.
    let anyway = unpack(
        &path,
        &UnpackOptions {
            load_even_if_refused: true,
            ..options(Some(fake_checker(1)))
        },
    );
    assert_eq!(anyway["loaded"], true, "{anyway:#}");
    assert_eq!(
        anyway["certificate"]["verdict"], "certificate_refused",
        "loading it anyway must not change the verdict: {anyway:#}"
    );

    // (c) exit 2: unreadable, which is not a verdict in either direction.
    let unreadable = unpack(&path, &options(Some(fake_checker(2))));
    assert_eq!(
        unreadable["certificate"]["verdict"], "certificate_unreadable",
        "{unreadable:#}"
    );
    assert_eq!(unreadable["loaded"], false, "{unreadable:#}");

    // (d) no checker on this machine. The graph still loads, and the report
    // must not contain the accepted word anywhere in it.
    let absent = unpack(&path, &options(Some(PathBuf::from("/nonexistent/oo-cert"))));
    assert_eq!(
        absent["certificate"]["verdict"], "checker_absent_nothing_was_checked",
        "{absent:#}"
    );
    assert_eq!(
        absent["loaded"], true,
        "an absent checker must not make a proof-carrying pack unusable: {absent:#}"
    );
    assert!(absent["certificate"]["theorem"].is_null(), "{absent:#}");
    assert_eq!(absent["certificate"]["checked_here"], false, "{absent:#}");
    let whole = serde_json::to_string(&absent).unwrap();
    assert!(
        !whole.contains("certificate_accepted"),
        "we did not check, and the report says it passed somewhere: {absent:#}"
    );
    assert!(
        absent["certificate"]["means"]
            .as_str()
            .unwrap_or_default()
            .contains("NOTHING WAS CHECKED"),
        "{absent:#}"
    );

    // And the caller who turns the check off earns the same word, because the
    // two produce the same amount of evidence.
    let skipped = unpack(
        &path,
        &UnpackOptions {
            check_certificate: false,
            ..options(None)
        },
    );
    assert_eq!(
        skipped["certificate"]["verdict"], "checker_absent_nothing_was_checked",
        "{skipped:#}"
    );
    assert_eq!(skipped["certificate"]["skipped_by_caller"], true, "{skipped:#}");
}

// ───────────────────────────────────────────────────────────────────────────
// 3. The binding
// ───────────────────────────────────────────────────────────────────────────

/// A swapped certificate is caught by the manifest digest.
///
/// `sha256` covers the graph and nothing else, so it matches after the swap.
/// `content_sha256` covers the graph AND every field and file of the
/// certificate, and it is the only thing that can notice. The checker is never
/// spawned, which is the point: the refusal is the digest's and not the
/// checker's.
#[test]
fn a_swapped_certificate_is_caught_by_the_content_digest() {
    let out = scratch("swap");
    let mine = scratch("swap-mine-cert");
    let theirs = scratch("swap-theirs-cert");
    let store = certify(LIST_BEARING, &mine);
    let other = certify(OTHER_GRAPH, &theirs);
    pack_with_certificate(&store, &mine, &out);
    let other_out = scratch("swap-other");
    pack_with_certificate(&other, &theirs, &other_out);

    let path = out.join("carried.oopack");
    let mut pack = read_pack(&path);
    let other_pack = read_pack(&other_out.join("carried.oopack"));

    // Only the certificate changes. The graph, and therefore `sha256`, is
    // untouched.
    pack["manifest"]["certificate"] = other_pack["manifest"]["certificate"].clone();
    write_pack(&path, &pack);

    let r = unpack(&path, &options(Some(fake_checker(0))));
    assert_eq!(r["loaded"], false, "{r:#}");
    assert!(
        r["error"]
            .as_str()
            .unwrap_or_default()
            .contains("content checksum mismatch"),
        "a swapped certificate must be caught by the content digest: {r:#}"
    );
    assert!(
        r["certificate"].is_null(),
        "the checker must never be reached on a pack whose bodies do not match: {r:#}"
    );
}

/// A pack that carries a certificate and no `content_sha256` is REFUSED rather
/// than checked.
///
/// Without this the downgrade is free: strip the new field, paste in a
/// certificate that is sound over another graph, and the old, weaker
/// verification passes it.
#[test]
fn a_certificate_with_no_content_digest_is_refused_not_checked() {
    let cert_dir = scratch("unbound-cert");
    let out = scratch("unbound");
    let store = certify(LIST_BEARING, &cert_dir);
    pack_with_certificate(&store, &cert_dir, &out);

    let path = out.join("carried.oopack");
    let mut pack = read_pack(&path);
    pack["manifest"]
        .as_object_mut()
        .unwrap()
        .remove("content_sha256");
    assert!(
        pack["manifest"]["certificate"].is_object(),
        "the certificate has to stay, or this tests nothing"
    );
    write_pack(&path, &pack);

    let r = unpack(&path, &options(Some(fake_checker(0))));
    assert_eq!(r["loaded"], false, "{r:#}");
    assert!(
        r["error"]
            .as_str()
            .unwrap_or_default()
            .contains("content_sha256"),
        "the refusal must name the missing binding: {r:#}"
    );
    let whole = serde_json::to_string(&r).unwrap();
    assert!(
        !whole.contains("certificate_accepted"),
        "an unbound certificate must not earn an accepted word: {r:#}"
    );
}

// ───────────────────────────────────────────────────────────────────────────
// 4. The real checker
// ───────────────────────────────────────────────────────────────────────────

/// A conclusion forged INSIDE the packed certificate is refused by the real
/// `oo-cert`, whether or not the attacker repairs the digest.
#[test]
fn a_forged_conclusion_in_the_packed_certificate_is_refused() {
    if skip() {
        return;
    }
    let checker = real_checker();
    let cert_dir = scratch("forge-cert");
    let out = scratch("forge");
    let store = certify(LIST_BEARING, &cert_dir);
    let packed = pack_with_certificate(&store, &cert_dir, &out);
    let path = out.join("carried.oopack");

    // The honest case first. A forgery test whose honest case was never shown
    // to pass proves nothing.
    let honest = unpack(&path, &options(Some(checker.clone())));
    assert_eq!(
        honest["certificate"]["verdict"], "certificate_accepted",
        "the honest certificate must be accepted by the real checker first: {honest:#}"
    );
    assert_eq!(
        honest["certificate"]["theorem"], "OOCert.certificate_sound",
        "{honest:#}"
    );
    assert_eq!(honest["loaded"], true, "{honest:#}");
    assert_eq!(
        honest["certificate"]["coverage"]["in_certificate_only"], 0,
        "the certificate's premises are all in the pack: {honest:#}"
    );
    eprintln!(
        "PACK SIZE: graph {} bytes, certificate {} bytes, pack JSON {} bytes",
        packed["graph_bytes"], packed["certificate"]["bytes"], packed["pack_json_bytes"]
    );

    // (a) the lazy attacker: forge a line and leave the digest alone.
    let mut pack = read_pack(&path);
    let forged = format!(
        "{}{FORGED_RDFS9}",
        pack["manifest"]["certificate"]["files"]["derivations.tsv"]
            .as_str()
            .unwrap()
    );
    pack["manifest"]["certificate"]["files"]["derivations.tsv"] =
        serde_json::json!(forged.clone());
    write_pack(&path, &pack);
    let lazy = unpack(&path, &options(Some(checker.clone())));
    assert_eq!(lazy["loaded"], false, "{lazy:#}");
    assert!(
        lazy["error"]
            .as_str()
            .unwrap_or_default()
            .contains("content checksum mismatch"),
        "the binding must notice a forged line before any checker runs: {lazy:#}"
    );

    // (b) the competent attacker: repair every digest, so only the PROOF can
    // refuse it.
    let cert: PackedCertificate =
        serde_json::from_value(pack["manifest"]["certificate"].clone()).unwrap();
    pack["manifest"]["content_sha256"] = serde_json::json!(content_digest(
        pack["graph"].as_str().unwrap(),
        Some(&cert)
    ));
    write_pack(&path, &pack);
    let competent = unpack(&path, &options(Some(checker)));
    assert_eq!(
        competent["certificate"]["verdict"], "certificate_refused",
        "the real checker must refuse a forged conclusion even when every digest \
         matches: {competent:#}"
    );
    assert_eq!(competent["loaded"], false, "{competent:#}");
    let said = competent["certificate"]["checker_said"]
        .as_str()
        .unwrap_or_default();
    assert!(
        said.contains("\"ok\":false") && said.contains("first_rejected"),
        "the checker's own words must be quoted: {said}"
    );
}

/// A certificate the checker ACCEPTS, about a graph this pack does not carry,
/// is refused anyway.
///
/// The checker was right and the pack is still refused. `oo-cert` verifies the
/// steps against the triples inside the certificate and has no way to ask where
/// they came from, so this is the gate the checker cannot be.
#[test]
fn a_certificate_about_another_graph_is_not_about_this_pack() {
    if skip() {
        return;
    }
    let checker = real_checker();
    let mine = scratch("elsewhere-mine");
    let theirs = scratch("elsewhere-theirs");
    let out = scratch("elsewhere");
    let store = certify(LIST_BEARING, &mine);
    let other = certify(OTHER_GRAPH, &theirs);

    // `Packer::pack` refuses this at pack time, so the artefact is assembled by
    // hand: the receiver must not depend on the sender having refused it.
    pack_with_certificate(&store, &mine, &out);
    let other_out = scratch("elsewhere-other");
    pack_with_certificate(&other, &theirs, &other_out);
    let path = out.join("carried.oopack");
    let mut pack = read_pack(&path);
    pack["manifest"]["certificate"] =
        read_pack(&other_out.join("carried.oopack"))["manifest"]["certificate"].clone();
    let cert: PackedCertificate =
        serde_json::from_value(pack["manifest"]["certificate"].clone()).unwrap();
    pack["manifest"]["content_sha256"] = serde_json::json!(content_digest(
        pack["graph"].as_str().unwrap(),
        Some(&cert)
    ));
    write_pack(&path, &pack);

    let r = unpack(&path, &options(Some(checker)));
    assert_eq!(
        r["certificate"]["verdict"], "certificate_not_about_this_pack",
        "{r:#}"
    );
    assert_eq!(r["loaded"], false, "{r:#}");
    assert!(
        r["certificate"]["coverage"]["in_certificate_only"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "the refusal rests on premises the pack does not carry: {r:#}"
    );
    assert!(
        r["certificate"]["checker_said"]
            .as_str()
            .unwrap_or_default()
            .contains("\"ok\":true"),
        "the checker accepted it and the pack is refused anyway, which is the whole \
         point: {r:#}"
    );
    assert!(r["certificate"]["theorem"].is_null(), "{r:#}");
}

/// The sender learns at pack time, so the auditor does not learn at unpack
/// time. The receiver-side refusal above stays either way.
#[test]
fn pack_refuses_a_certificate_that_is_not_about_the_graph() {
    let mine = scratch("senderside-mine");
    let theirs = scratch("senderside-theirs");
    let out = scratch("senderside");
    let store = certify(LIST_BEARING, &mine);
    let _other = certify(OTHER_GRAPH, &theirs);

    let path = out.join("wrong.oopack");
    let err = Packer::new(store)
        .pack(&PackRequest {
            path: path.to_str().unwrap(),
            name: "wrong",
            version: "1.0.0",
            evidence: None,
            certificate_dir: Some(&theirs),
            profile_claimed: None,
        })
        .expect_err("packing somebody else's certificate must fail");
    let text = err.to_string();
    assert!(
        text.contains("not in the graph being packed"),
        "the sender must be told which way it is wrong: {text}"
    );
    assert!(
        !path.exists(),
        "a refused pack must not be left on disk: {}",
        path.display()
    );
}

// ───────────────────────────────────────────────────────────────────────────
// 5. Vocabulary
// ───────────────────────────────────────────────────────────────────────────

/// No word this module states is a word a Lean checker states.
///
/// `src/verdict.rs` asserts this over the vocabularies defined in that module
/// and cannot see this one.
#[test]
fn no_pack_verdict_word_is_a_checker_word() {
    for w in PackCertVerdict::WORDS {
        assert!(
            !CHECKER_OWNED_WORDS.contains(&w),
            "{w} is a word the Lean binaries print for themselves. An engine opinion and a \
             machine-checked result must never share a string"
        );
    }
    let mut seen: Vec<&str> = PackCertVerdict::WORDS.to_vec();
    seen.sort_unstable();
    let before = seen.len();
    seen.dedup();
    assert_eq!(before, seen.len(), "two verdicts share a word: {seen:?}");

    // The list cannot rot away from the enum: every variant that can be built
    // without a checker states a word that is on it.
    for v in [
        PackCertVerdict::Refused,
        PackCertVerdict::Unreadable,
        PackCertVerdict::NotAboutThisPack,
        PackCertVerdict::CheckerAbsent,
        PackCertVerdict::NoCertificateInPack,
    ] {
        assert!(
            PackCertVerdict::WORDS.contains(&v.word()),
            "{} is stated by a variant and missing from WORDS",
            v.word()
        );
        assert!(!v.is_checked(), "{v} carries no checker evidence");
    }
}
