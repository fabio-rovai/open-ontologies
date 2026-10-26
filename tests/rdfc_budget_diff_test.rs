//! **Issue #293: diff and drift on a poisoned blank-node graph.**
//!
//! Both commands canonicalise blank nodes with RDFC-1.0 before comparing.
//! Through oxrdf's unbounded canonicaliser a 7-node blank-node clique took 17
//! to 23 seconds to diff against itself and an 8-node clique did not finish in
//! three minutes. The fix runs the budgeted `open_ontologies::canon` and, when
//! the budget runs out, compares only the ground triples and says so.
//!
//! The time bounds below are deliberately generous. On the fixed code the
//! clique runs in well under a second; on the unfixed code it does not finish
//! at all, so a bound of a minute separates the two without flaking on a slow
//! CI runner. The assertions that carry the meaning are the `exact` flags.

use open_ontologies::drift::DriftDetector;
use open_ontologies::ontology::OntologyService;
use open_ontologies::state::StateDb;
use std::collections::BTreeSet;
use std::time::Duration;

const GENEROUS: Duration = Duration::from_secs(60);

/// `_:bi ex:p _:bj` for every ordered pair i != j: every node has the same
/// first-degree hash and every permutation of its neighbours looks alike,
/// which is the RDFC-1.0 section 4.4 poison shape.
fn clique(n: usize) -> String {
    let mut ttl = String::from("@prefix ex: <http://example.org/> .\n");
    for i in 0..n {
        for j in 0..n {
            if i != j {
                ttl.push_str(&format!("_:b{i} ex:p _:b{j} .\n"));
            }
        }
    }
    ttl
}

fn pizza() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("benchmark/ontoaxiom/data/ontoaxiom/ontologies/pizza.ttl");
    std::fs::read_to_string(path).unwrap()
}

/// Run `f` on its own thread and fail, rather than hang, if it overruns.
fn within<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(GENEROUS)
        .unwrap_or_else(|_| panic!("did not finish within {GENEROUS:?}: the RDFC-1.0 work is unbounded"))
}

fn diff(old: String, new: String) -> serde_json::Value {
    within(move || serde_json::from_str(&OntologyService::diff(&old, &new).unwrap()).unwrap())
}

fn drift(v1: String, v2: String) -> serde_json::Value {
    within(move || {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let detector = DriftDetector::new(StateDb::open(tmp.path()).unwrap());
        serde_json::from_str(&detector.detect(&v1, &v2).unwrap()).unwrap()
    })
}

#[test]
fn eight_node_clique_diff_is_bounded_and_marked_approximate() {
    let report = diff(clique(8), clique(8));
    assert_eq!(report["exact"], false, "{report}");
    let reason = report["approximate_reason"].as_str().unwrap();
    assert!(reason.contains("budget"), "{reason}");
    // All 56 triples mention a blank node, so none was compared, and the
    // report says that rather than letting `added: 0` read as "no changes".
    assert_eq!(report["omitted_old"], 56);
    assert_eq!(report["omitted_new"], 56);
    assert_eq!(report["added"], 0);
    assert_eq!(report["removed"], 0);
}

/// Two identical stars, `_:hub ex:p _:leaf` with 12 leaves each. The hubs
/// tie, and hashing a hub permutes its 12 interchangeable leaves. Built
/// eagerly that was 12! (479 million) orderings in memory before the first
/// budget check; at 11 leaves it already took 10 s and 1.78 GB.
#[test]
fn wide_symmetric_star_is_bounded_and_marked_approximate() {
    let mut ttl = String::from("@prefix ex: <http://example.org/> .\n");
    for hub in 0..2 {
        for leaf in 0..12 {
            ttl.push_str(&format!("_:h{hub} ex:p _:l{hub}_{leaf} .\n"));
        }
    }
    let report = diff(ttl.clone(), ttl);
    assert_eq!(report["exact"], false, "{report}");
    assert_eq!(report["omitted_old"], 24);
}

#[test]
fn approximate_diff_still_reports_ground_changes() {
    let old = format!("{}<http://example.org/x> <http://example.org/label> \"old\" .\n", clique(8));
    let new = format!("{}<http://example.org/x> <http://example.org/label> \"new\" .\n", clique(8));
    let report = diff(old, new);
    assert_eq!(report["exact"], false);
    assert_eq!(report["added"], 1, "{report}");
    assert_eq!(report["removed"], 1, "{report}");
    assert!(report["added_triples"][0].as_str().unwrap().ends_with("\"new\""));
}

/// One poisoned side cuts BOTH sides to ground. Canonical labels on the exact
/// side against nothing on the other would report the pizza ontology's
/// anonymous classes as removed, which is a change nobody made.
#[test]
fn one_poisoned_side_does_not_invent_churn_on_the_other() {
    let old = pizza();
    let new = format!("{old}\n{}", clique(8).lines().skip(1).collect::<Vec<_>>().join("\n"));
    let new = format!("@prefix ex: <http://example.org/> .\n{new}");
    let report = diff(old, new);
    assert_eq!(report["exact"], false);
    assert_eq!(report["added"], 0, "{report}");
    assert_eq!(report["removed"], 0, "{report}");
    let omitted_old = report["omitted_old"].as_u64().unwrap();
    assert!(omitted_old > 0);
    assert_eq!(report["omitted_new"].as_u64().unwrap(), omitted_old + 56);
}

/// pizza.ttl is lists and restrictions throughout, the blank nodes a real
/// ontology has. It must stay exact, with no churn against itself, and the
/// budgeted canonicaliser must give it the labels oxrdf's unbounded one gave,
/// so that switching implementation moves no ordinary diff.
#[test]
fn ordinary_blank_node_ontology_stays_exact_with_unchanged_labels() {
    let report = diff(pizza(), pizza());
    assert_eq!(report["exact"], true, "{report}");
    assert!(report.get("approximate_reason").is_none());
    assert_eq!(report["added"], 0);
    assert_eq!(report["removed"], 0);

    use oxigraph::io::{RdfFormat, RdfParser};
    use oxigraph::model::dataset::{CanonicalizationAlgorithm, CanonicalizationHashAlgorithm};
    use oxigraph::model::{Dataset, Quad};
    let quads: Vec<Quad> = RdfParser::from_format(RdfFormat::Turtle)
        .for_reader(pizza().as_bytes())
        .map(|q| q.unwrap())
        .collect();
    let ours: BTreeSet<String> = open_ontologies::canon::canonical_nquads(
        &quads,
        open_ontologies::canon::DEFAULT_CALL_BUDGET,
        open_ontologies::canon::HashAlgorithm::Sha256,
    )
    .unwrap()
    .lines()
    .map(str::to_string)
    .collect();
    assert!(ours.iter().any(|l| l.starts_with("_:c14n")));

    let mut dataset: Dataset = quads.iter().collect();
    dataset.canonicalize(CanonicalizationAlgorithm::Rdfc10 {
        hash_algorithm: CanonicalizationHashAlgorithm::Sha256,
    });
    let reference: BTreeSet<String> = dataset.iter().map(|q| format!("{q} .")).collect();
    let only_ours: Vec<_> = ours.difference(&reference).collect();
    let only_reference: Vec<_> = reference.difference(&ours).collect();
    assert!(
        only_ours.is_empty() && only_reference.is_empty(),
        "labels moved: {} lines only in canon, {} only in oxrdf; first: {:?} / {:?}",
        only_ours.len(),
        only_reference.len(),
        only_ours.first(),
        only_reference.first()
    );
}

#[test]
fn eight_node_clique_drift_is_bounded_and_marked_approximate() {
    let report = drift(clique(8), clique(8));
    assert_eq!(report["exact"], false, "{report}");
    assert!(report["approximate_reason"].as_str().unwrap().contains("budget"));
    assert_eq!(report["omitted_v1"], 56);
    assert_eq!(report["omitted_v2"], 56);
}

#[test]
fn ordinary_blank_node_ontology_drift_stays_exact() {
    let report = drift(pizza(), pizza());
    assert_eq!(report["exact"], true, "{report}");
    assert!(report.get("approximate_reason").is_none());
    assert_eq!(report["added"], serde_json::json!([]));
    assert_eq!(report["removed"], serde_json::json!([]));
}

/// KGCL is built from the drift JSON, so it must carry the flag too, or the
/// `kgcl_json` format would present the cut as an exact change set.
#[test]
fn kgcl_report_carries_the_approximation() {
    let report = within(|| {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let detector = DriftDetector::new(StateDb::open(tmp.path()).unwrap());
        detector.detect_kgcl(&clique(8), &clique(8), 0.7).unwrap()
    });
    assert!(report.approximate_reason.as_deref().unwrap().contains("budget"));
    assert_eq!(report.to_json()["exact"], false);
}
