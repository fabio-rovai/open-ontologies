//! **The W3C RDFC-1.0 suite, run against `open_ontologies::canon`.**
//!
//! A canonicaliser is a trusted component. If it gives two different datasets
//! one address, a content-addressed certificate vouches for an input nobody
//! supplied. This module is written here rather than taken from a crate, for
//! the reasons `src/canon.rs` records, and that choice is only defensible if
//! it is measured against the Recommendation's own tests rather than against
//! its author's understanding of them.
//!
//! The suite is vendored under `tests/w3c-rdfc10/` so the gate runs offline.
//! Three kinds of entry, and all three are run:
//!
//!   * `RDFC10EvalTest` canonicalises an input and compares the N-Quads byte
//!     for byte.
//!   * `RDFC10MapTest` compares the blank-node label MAP, which catches an
//!     implementation that produces the right lines under the wrong labels.
//!   * `RDFC10NegativeEvalTest` is the poisoned dataset. It must be REFUSED.
//!     A canonicaliser with no work budget passes every other test in this
//!     suite and hangs on that one.

use oxigraph::io::{RdfFormat, RdfParser};
use oxigraph::model::Quad;
use std::path::{Path, PathBuf};

fn suite() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("w3c-rdfc10")
}

fn parse_nquads(path: &Path) -> Vec<Quad> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    RdfParser::from_format(RdfFormat::NQuads)
        .for_reader(bytes.as_slice())
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

struct Entry {
    id: String,
    kind: String,
    action: String,
    result: Option<String>,
    /// The suite carries a SHA-384 variant of the diamond test. Reading the
    /// algorithm from the manifest rather than assuming SHA-256 is what makes
    /// that test meaningful instead of a known failure.
    hash: open_ontologies::canon::HashAlgorithm,
}

fn entries() -> Vec<Entry> {
    let raw = std::fs::read_to_string(suite().join("manifest.jsonld")).expect("read manifest");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("manifest is JSON");
    let list = v["entries"].as_array().expect("manifest has entries");
    let out: Vec<Entry> = list
        .iter()
        .map(|e| Entry {
            id: e["id"].as_str().unwrap_or_default().to_string(),
            kind: e["type"].as_str().unwrap_or_default().to_string(),
            action: e["action"].as_str().unwrap_or_default().to_string(),
            result: e["result"].as_str().map(str::to_string),
            hash: match e["hashAlgorithm"].as_str() {
                Some("SHA384") => open_ontologies::canon::HashAlgorithm::Sha384,
                _ => open_ontologies::canon::HashAlgorithm::Sha256,
            },
        })
        .collect();
    assert!(out.len() >= 80, "only {} entries; the suite is not loaded", out.len());
    out
}

#[test]
fn every_eval_test_canonicalises_to_the_expected_nquads() {
    let mut ran = 0;
    let mut failed: Vec<String> = Vec::new();
    for e in entries() {
        if e.kind != "rdfc:RDFC10EvalTest" {
            continue;
        }
        let Some(expected_path) = e.result.as_ref() else { continue };
        let quads = parse_nquads(&suite().join(&e.action));
        let expected = std::fs::read_to_string(suite().join(expected_path))
            .unwrap_or_else(|err| panic!("read {expected_path}: {err}"));
        match open_ontologies::canon::canonical_nquads(&quads, open_ontologies::canon::DEFAULT_CALL_BUDGET, e.hash) {
            Ok(got) if got == expected => ran += 1,
            Ok(got) => {
                ran += 1;
                failed.push(format!(
                    "{}: canonical form differs\n  expected {} lines\n  got      {} lines",
                    e.id,
                    expected.lines().count(),
                    got.lines().count()
                ));
            }
            Err(err) => {
                ran += 1;
                failed.push(format!("{}: refused: {err}", e.id));
            }
        }
    }
    assert!(ran >= 60, "only {ran} eval tests ran; the walk is not finding them");
    assert!(
        failed.is_empty(),
        "{} of {ran} RDFC-1.0 eval tests failed:\n  {}",
        failed.len(),
        failed.join("\n  ")
    );
}

#[test]
fn every_map_test_produces_the_expected_labels() {
    let mut ran = 0;
    let mut failed: Vec<String> = Vec::new();
    for e in entries() {
        if e.kind != "rdfc:RDFC10MapTest" {
            continue;
        }
        let Some(expected_path) = e.result.as_ref() else { continue };
        let quads = parse_nquads(&suite().join(&e.action));
        let raw = std::fs::read_to_string(suite().join(expected_path))
            .unwrap_or_else(|err| panic!("read {expected_path}: {err}"));
        let expected: serde_json::Value = serde_json::from_str(&raw).expect("map is JSON");
        match open_ontologies::canon::canonical_labels(&quads, open_ontologies::canon::DEFAULT_CALL_BUDGET, e.hash) {
            Ok(labels) => {
                ran += 1;
                let obj = expected.as_object().expect("map is an object");
                for (k, v) in obj {
                    let want = v.as_str().unwrap_or_default();
                    match labels.get(k) {
                        Some(got) if got == want => {}
                        Some(got) => failed.push(format!("{}: {k} mapped to {got}, expected {want}", e.id)),
                        None => failed.push(format!("{}: {k} was not mapped at all", e.id)),
                    }
                }
            }
            Err(err) => {
                ran += 1;
                failed.push(format!("{}: refused: {err}", e.id));
            }
        }
    }
    assert!(ran >= 15, "only {ran} map tests ran");
    assert!(failed.is_empty(), "{} label mismatches:\n  {}", failed.len(), failed.join("\n  "));
}

/// The poisoned dataset. A canonicaliser with no work budget passes every
/// other test here and does not return on this one.
#[test]
fn the_poisoned_dataset_is_refused_rather_than_answered() {
    let mut ran = 0;
    for e in entries() {
        if e.kind != "rdfc:RDFC10NegativeEvalTest" {
            continue;
        }
        ran += 1;
        let quads = parse_nquads(&suite().join(&e.action));
        let got = open_ontologies::canon::canonical_nquads(&quads, open_ontologies::canon::DEFAULT_CALL_BUDGET, e.hash);
        assert!(
            got.is_err(),
            "{} is the poisoned dataset and must be refused. Answering it means the budget is \
             not being enforced, and a canonical form computed under a cut-off is not canonical",
            e.id
        );
    }
    assert_eq!(ran, 1, "expected exactly one negative test in the suite, ran {ran}");
}

/// The budget must DISCRIMINATE, not merely refuse.
///
/// A canonicaliser that returned `BudgetExhausted` for everything would pass
/// the negative test above and be useless, so this pins both directions at a
/// budget far below the default: the poisoned dataset is still refused, and an
/// ordinary dataset with blank nodes still canonicalises. The gap between what
/// the two cost is the whole justification for having a budget.
#[test]
fn a_small_budget_refuses_the_poison_and_still_answers_ordinary_data() {
    use open_ontologies::canon::{canonical_nquads, HashAlgorithm};
    const TINY: usize = 12;

    let mut poisoned = None;
    let mut ordinary = Vec::new();
    for e in entries() {
        let quads = parse_nquads(&suite().join(&e.action));
        if e.kind == "rdfc:RDFC10NegativeEvalTest" {
            poisoned = Some(quads);
        } else if e.kind == "rdfc:RDFC10EvalTest" && !quads.is_empty() {
            ordinary.push((e.id.clone(), quads, e.hash));
        }
    }

    let poisoned = poisoned.expect("the suite carries a poisoned dataset");
    assert!(
        canonical_nquads(&poisoned, TINY, HashAlgorithm::Sha256).is_err(),
        "the poisoned dataset must be refused at a budget of {TINY}"
    );

    let answered = ordinary
        .iter()
        .filter(|(_, q, h)| canonical_nquads(q, TINY, *h).is_ok())
        .count();
    assert!(
        answered > 40,
        "only {answered} of {} ordinary datasets canonicalised at a budget of {TINY}. A budget \
         that refuses ordinary data is not a poison guard, it is a broken canonicaliser",
        ordinary.len()
    );
}
