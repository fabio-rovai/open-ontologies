//! One graph per clash detector, and a near-miss twin for each that must stay clean.
//!
//! Issue #160: all 295 tracked RDF files sweep clean under the ten clash
//! detectors, so the shipped corpus exercises the refutation producer **not at
//! all**, and a single synthetic round-trip test was the only thing touching
//! it. Ten more synthetic tests beside that one would leave the same hole:
//! nothing would notice if a detector stopped firing on a shape it used to
//! catch.
//!
//! So the fixtures live in `tests/fixtures/clash-coverage/`, one named for each
//! detector, and this file asserts three things that together make a removed
//! detector turn exactly one test red.
//!
//! **The list of detectors is read from the source, not typed here.**
//! [`detectors_in_the_source`] scans the body of `find_clashes` in
//! `src/reason.rs` for the rule names it constructs. An eleventh detector added
//! without a fixture turns this file red on the day it lands, which a typed
//! list of ten could never do.
//!
//! **Each fixture fires its own detector and no other.** Not "fires at least
//! its own": the set of rules in `by_rule` must equal the singleton of the rule
//! the file is named for. That makes the fixture-to-detector map injective, so
//! removing one detector can only silence one fixture.
//!
//! **Each fixture ships with a twin that must come back clean.** A fixture that
//! refutes proves nothing on its own, because a graph can be contradictory for
//! a reason the fixture's author did not intend. `<rule>.clean.ttl` is the same
//! graph with the one premise that closes the contradiction removed, and it
//! must yield no clash at all. That is what makes each fixture evidence about
//! the premise it was written for rather than about its vocabulary.
//!
//! These are TEST DATA and are excluded from the corpus, exactly as
//! `tests/fixtures/horn-coverage/` is, for the reason that directory's README
//! gives: a detector that earns its place on a file written to make it fire has
//! earned nothing. [`the_fixtures_are_tracked_and_are_not_corpus`] is the guard
//! that the exclusion is real rather than a stale path.

mod common;

use open_ontologies::graph::GraphStore;
use open_ontologies::reason::{InferenceTarget, Reasoner};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture_dir() -> PathBuf {
    repo().join("tests").join("fixtures").join("clash-coverage")
}

/// The rule names `find_clashes` actually constructs, read out of the source.
///
/// The scan is bounded to that one function: it starts at the signature and
/// stops at the next item at column zero, so a `rule: "..."` literal anywhere
/// else in the file cannot pad the list. If the scan ever returns an
/// implausible number the caller fails rather than proceeding with a short
/// list, because a source scan that silently matches nothing is a gate that
/// cannot fail.
fn detectors_in_the_source() -> BTreeSet<String> {
    let src = std::fs::read_to_string(repo().join("src").join("reason.rs"))
        .expect("read src/reason.rs");
    let start = src.find("fn find_clashes(").expect(
        "`fn find_clashes(` is gone from src/reason.rs. If it was renamed, this scan is \
         reading nothing and every assertion below is vacuous",
    );
    let body = &src[start..];
    // The next item at column zero ends the function.
    let end = body[1..]
        .find("\nfn ")
        .or_else(|| body[1..].find("\npub fn "))
        .map(|i| i + 1)
        .unwrap_or(body.len());
    let body = &body[..end];

    let mut out = BTreeSet::new();
    for (i, _) in body.match_indices("rule: \"") {
        let rest = &body[i + "rule: \"".len()..];
        if let Some(q) = rest.find('"') {
            out.insert(rest[..q].to_string());
        }
    }
    assert!(
        out.len() >= 8,
        "the scan of `find_clashes` found only {} rule names ({:?}). The construction \
         spelling has changed and this file is now measuring nothing",
        out.len(),
        out
    );
    out
}

/// Reason over one fixture and return the set of clash rules that fired.
///
/// `owl-rl` because a clash needs the OWL vocabulary, and `materialize` is
/// false so the fixture on disk is never written back into.
fn rules_that_fire(path: &Path) -> BTreeSet<String> {
    let store = Arc::new(GraphStore::new());
    store
        .load_file(&path.display().to_string())
        .unwrap_or_else(|e| panic!("load {}: {e}", path.display()));
    let out = Reasoner::run_full(&store, "owl-rl", false, InferenceTarget::DefaultGraph, None)
        .unwrap_or_else(|e| panic!("reason over {}: {e}", path.display()));
    let r: serde_json::Value = serde_json::from_str(&out).unwrap();
    if r["inconsistency"]["found"] != serde_json::Value::Bool(true) {
        return BTreeSet::new();
    }
    r["inconsistency"]["by_rule"]
        .as_object()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

#[test]
fn every_detector_in_the_source_has_a_fixture_named_for_it() {
    let detectors = detectors_in_the_source();
    let mut missing = Vec::new();
    for d in &detectors {
        let f = fixture_dir().join(format!("{d}.ttl"));
        if !f.exists() {
            missing.push(f.display().to_string());
        }
        let twin = fixture_dir().join(format!("{d}.clean.ttl"));
        if !twin.exists() {
            missing.push(twin.display().to_string());
        }
    }
    assert!(
        missing.is_empty(),
        "a clash detector exists in src/reason.rs with no fixture to exercise it. \
         Write one graph that makes it fire and one twin that does not:\n  {}",
        missing.join("\n  ")
    );

    // And the other direction: a fixture naming a detector that no longer
    // exists is dead weight that reads as coverage.
    let mut orphans = Vec::new();
    for e in std::fs::read_dir(fixture_dir()).expect("read the fixture directory") {
        let p = e.unwrap().path();
        let Some(stem) = p.file_name().and_then(|s| s.to_str()) else { continue };
        let Some(stem) = stem.strip_suffix(".ttl") else { continue };
        let stem = stem.strip_suffix(".clean").unwrap_or(stem);
        if !detectors.contains(stem) {
            orphans.push(p.display().to_string());
        }
    }
    assert!(
        orphans.is_empty(),
        "these fixtures are named for a rule `find_clashes` does not construct:\n  {}",
        orphans.join("\n  ")
    );
    println!("{} detectors, each with a fixture: {detectors:?}", detectors.len());
}

#[test]
fn each_fixture_fires_its_own_detector_and_no_other() {
    let detectors = detectors_in_the_source();
    let mut table = Vec::new();
    for d in &detectors {
        let fired = rules_that_fire(&fixture_dir().join(format!("{d}.ttl")));
        let want: BTreeSet<String> = std::iter::once(d.clone()).collect();
        assert_eq!(
            fired, want,
            "tests/fixtures/clash-coverage/{d}.ttl is supposed to fire {d} and nothing else. \
             It fired {fired:?}. A fixture that trips a second detector makes the map from \
             detectors to fixtures non-injective, and then removing a detector can be masked \
             by another fixture still covering it"
        );
        table.push(d.clone());
    }
    println!("each of {} fixtures fires exactly its own rule: {table:?}", table.len());
}

#[test]
fn each_twin_is_clean_so_the_fixture_is_evidence_about_its_premise() {
    let detectors = detectors_in_the_source();
    for d in &detectors {
        let fired = rules_that_fire(&fixture_dir().join(format!("{d}.clean.ttl")));
        assert!(
            fired.is_empty(),
            "tests/fixtures/clash-coverage/{d}.clean.ttl is the near miss and must not be \
             refuted. It fired {fired:?}. Either the twin still carries the contradiction, \
             in which case {d}.ttl is not evidence about the premise it removed, or a \
             detector fires on the vocabulary alone"
        );
    }
    println!("all {} twins are clean", detectors.len());
}

/// The exclusion is real, in the pattern `tests/fixtures/horn-coverage/` set.
///
/// These graphs were written to make a detector fire. Counted as corpus they
/// would turn "the shipped corpus sweeps clean under the ten detectors" from a
/// measurement into a statement about this directory.
#[test]
fn the_fixtures_are_tracked_and_are_not_corpus() {
    const PREFIX: &str = "tests/fixtures/clash-coverage/";
    let out = std::process::Command::new("git")
        .args(["ls-files", "-z", "*.ttl"])
        .current_dir(repo())
        .output()
        .expect("run git ls-files");
    assert!(out.status.success(), "git ls-files failed");
    let tracked: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(|p| p.to_string())
        .collect();
    let here = tracked.iter().filter(|p| p.starts_with(PREFIX)).count();
    assert!(
        here >= 2 * detectors_in_the_source().len(),
        "expected a fixture and a twin per detector tracked under {PREFIX}, found {here}. \
         Untracked fixtures are invisible to every corpus sweep, including the ones that \
         are supposed to exclude them"
    );

    // Every corpus walker must exclude them. The walkers enumerate from
    // `git ls-files`, so an exclusion is a string in the walker's source; a
    // walker that gained a corpus sweep without gaining the exclusion is the
    // failure this checks for.
    let walkers = [
        "tests/projection_monotonicity_corpus_test.rs",
        "tests/reason_rl_coverage_test.rs",
        "tests/lean_certificate_test.rs",
    ];
    let mut blind = Vec::new();
    for w in walkers {
        let src = std::fs::read_to_string(repo().join(w)).unwrap_or_default();
        if !src.contains(PREFIX) {
            blind.push(w);
        }
    }
    assert!(
        blind.is_empty(),
        "these corpus sweeps enumerate tracked RDF and do not exclude {PREFIX}, so graphs \
         written to be contradictory are being swept as if they were real ontologies:\n  {}",
        blind.join("\n  ")
    );
    println!("{here} fixture files tracked and excluded from {} sweeps", walkers.len());
}
