//! Tests for the build-time modelling buffer.
//!
//! The canary suite is the one that matters commercially: it converts "the
//! model never sees proprietary values" from an assertion into a build gate.

use std::collections::HashSet;

use open_ontologies::buffer::{
    buffer, contamination_check, Classifier, Disposition, Term, TermKind, Vault,
};
use open_ontologies::state::StateDb;
use tempfile::TempDir;

fn vault(session: &str) -> (TempDir, Vault) {
    let dir = TempDir::new().unwrap();
    let db = StateDb::open(&dir.path().join("state.db")).unwrap();
    let v = Vault::open(db, session).unwrap();
    (dir, v)
}

fn classifier() -> Classifier {
    Classifier::new()
        .allow_generic(["Protein", "Candidate", "Assay", "Phase"])
        .with_surrogates("protein", ["EGFR", "TP53", "BRCA1"])
}

// ── Dispositions ────────────────────────────────────────────────────────

#[test]
fn instances_are_always_stripped() {
    let (_d, v) = vault("s1");
    let terms = vec![
        Term::new(TermKind::Instance, "Jane Okafor"),
        Term::new(TermKind::Instance, "42.7"),
        // Even when the value collides with an allowlisted generic term: it is
        // a row value, and row values are never needed to build a TBox.
        Term::new(TermKind::Instance, "Protein"),
    ];
    let out = buffer(&classifier(), &v, &terms).unwrap();
    for d in &out.decisions {
        assert_eq!(d.disposition, Disposition::Strip, "{:?}", d.term);
        assert!(d.replacement.is_none());
    }
    assert!(out.emitted().is_empty());
}

#[test]
fn generic_vocabulary_passes_through_unchanged() {
    let (_d, v) = vault("s1");
    let terms = vec![Term::new(TermKind::SchemaName, "Protein")];
    let out = buffer(&classifier(), &v, &terms).unwrap();
    assert_eq!(out.decisions[0].disposition, Disposition::Pass);
    assert_eq!(out.decisions[0].replacement.as_deref(), Some("Protein"));
}

#[test]
fn generic_matching_is_case_insensitive() {
    let (_d, v) = vault("s1");
    let terms = vec![Term::new(TermKind::SchemaName, "PROTEIN")];
    let out = buffer(&classifier(), &v, &terms).unwrap();
    assert_eq!(out.decisions[0].disposition, Disposition::Pass);
}

#[test]
fn identifiers_are_tokenised_opaquely() {
    let (_d, v) = vault("s1");
    let terms = vec![Term::new(TermKind::Identifier, "project_cardinal")];
    let out = buffer(&classifier(), &v, &terms).unwrap();
    assert_eq!(out.decisions[0].disposition, Disposition::Tokenise);
    let sent = out.decisions[0].replacement.clone().unwrap();
    assert!(sent.starts_with("ENT_"), "got {sent}");
    assert!(
        !sent.to_lowercase().contains("cardinal"),
        "token must not leak the original: {sent}"
    );
}

#[test]
fn non_generic_schema_names_with_a_pool_are_surrogated() {
    let (_d, v) = vault("s1");
    let terms = vec![Term::new(TermKind::SchemaName, "KDM5B_variant_7").with_hint("protein")];
    let out = buffer(&classifier(), &v, &terms).unwrap();
    assert_eq!(out.decisions[0].disposition, Disposition::Surrogate);
    let sent = out.decisions[0].replacement.clone().unwrap();
    assert!(
        ["EGFR", "TP53", "BRCA1"].contains(&sent.as_str()),
        "expected a pool member, got {sent}"
    );
}

/// The fail-safe. A surrogate with no available substitute must degrade toward
/// disclosing less, never toward passing the real term through.
#[test]
fn surrogate_without_a_pool_degrades_to_tokenise_not_pass() {
    let (_d, v) = vault("s1");
    let terms = vec![Term::new(TermKind::SchemaName, "Cardinal_Mechanism").with_hint("pathway")];
    let out = buffer(&classifier(), &v, &terms).unwrap();
    let d = &out.decisions[0];
    assert_eq!(d.disposition, Disposition::Tokenise);
    assert_ne!(d.disposition, Disposition::Pass);
    let sent = d.replacement.clone().unwrap();
    assert!(sent.starts_with("ENT_"));
    assert!(!sent.contains("Cardinal"));
}

#[test]
fn unrecognised_terms_default_to_strip() {
    let (_d, v) = vault("s1");
    let terms = vec![Term::new(TermKind::Label, "Internal note: see Cardinal memo")];
    let out = buffer(&classifier(), &v, &terms).unwrap();
    assert_eq!(out.decisions[0].disposition, Disposition::Strip);
    assert!(out.decisions[0].replacement.is_none());
}

// ── Canary: nothing withheld may appear on the wire ─────────────────────

/// The commercial claim, as a build gate. Every value the buffer decided not to
/// disclose must be absent from everything that leaves.
#[test]
fn canary_no_withheld_value_appears_in_the_emitted_payload() {
    let (_d, v) = vault("s1");
    let terms = vec![
        Term::new(TermKind::Instance, "CANARY_PATIENT_8f21"),
        Term::new(TermKind::Instance, "CANARY_VALUE_3310"),
        Term::new(TermKind::Identifier, "CANARY_PROJECT_cardinal"),
        Term::new(TermKind::SchemaName, "CANARY_TARGET_kdm5b").with_hint("protein"),
        Term::new(TermKind::Label, "CANARY_NOTE_secret"),
        Term::new(TermKind::SchemaName, "Protein"), // legitimately passes
    ];
    let out = buffer(&classifier(), &v, &terms).unwrap();

    let wire = out.emitted().join("\n");
    for withheld in out.withheld() {
        assert!(
            !wire.contains(withheld),
            "canary {withheld:?} leaked into the emitted payload:\n{wire}"
        );
    }
    // The allowlisted term is expected to be there; without this the test
    // would pass trivially if the buffer emitted nothing at all.
    assert!(wire.contains("Protein"));
}

/// The review rendering is shown to a human, but it is also a surface that can
/// leak if it is ever logged or forwarded. Originals appear there by design;
/// this test pins that intent so nobody "fixes" it by piping it outward.
#[test]
fn review_rendering_shows_originals_and_is_therefore_local_only() {
    let (_d, v) = vault("s1");
    let terms = vec![Term::new(TermKind::Identifier, "project_cardinal")];
    let out = buffer(&classifier(), &v, &terms).unwrap();
    let rendered = out.render_for_review();
    assert!(
        rendered.contains("project_cardinal"),
        "the reviewer must see what is being withheld"
    );
    assert!(rendered.contains("tokenise"));
}

// ── Vault behaviour ─────────────────────────────────────────────────────

#[test]
fn substitution_is_stable_within_a_session() {
    let (_d, v) = vault("s1");
    let a = v.tokenise("project_cardinal").unwrap();
    let b = v.tokenise("project_cardinal").unwrap();
    assert_eq!(a, b, "the model must be able to join on repeated terms");
}

#[test]
fn substitution_differs_across_sessions() {
    let dir = TempDir::new().unwrap();
    let db = StateDb::open(&dir.path().join("state.db")).unwrap();
    let v1 = Vault::open(db.clone(), "session-one").unwrap();
    let v2 = Vault::open(db, "session-two").unwrap();
    assert_ne!(
        v1.tokenise("project_cardinal").unwrap(),
        v2.tokenise("project_cardinal").unwrap(),
        "cross-session stability would accumulate a corpus at the provider"
    );
}

#[test]
fn resolve_maps_a_surrogate_back() {
    let (_d, v) = vault("s1");
    let token = v.tokenise("project_cardinal").unwrap();
    assert_eq!(
        v.resolve(&token).unwrap().as_deref(),
        Some("project_cardinal")
    );
    assert_eq!(v.resolve("ENT_deadbeef").unwrap(), None);
}

#[test]
fn desubstitute_restores_real_vocabulary() {
    let (_d, v) = vault("s1");
    let token = v.tokenise("project_cardinal").unwrap();
    let returned = format!("<{token}> a owl:Class .");
    assert_eq!(
        v.desubstitute(&returned).unwrap(),
        "<project_cardinal> a owl:Class ."
    );
}

/// Longest-first replacement. A short surrogate that is a prefix of a longer
/// one must not corrupt it during de-substitution.
#[test]
fn desubstitute_handles_overlapping_surrogates() {
    let (_d, v) = vault("s1");
    let short = v.surrogate("alpha", &["AB".to_string()]).unwrap().unwrap();
    let long = v.surrogate("beta", &["ABC".to_string()]).unwrap().unwrap();
    assert_eq!(short, "AB");
    assert_eq!(long, "ABC");
    assert_eq!(v.desubstitute("ABC").unwrap(), "beta");
}

// ── Contamination ───────────────────────────────────────────────────────

fn surrogate_set() -> HashSet<String> {
    ["EGFR".to_string()].into_iter().collect()
}

#[test]
fn structural_axioms_about_a_surrogate_are_accepted() {
    let triples = vec![
        (
            "EGFR".to_string(),
            "rdf:type".to_string(),
            "owl:Class".to_string(),
        ),
        (
            "EGFR".to_string(),
            "rdfs:subClassOf".to_string(),
            "Protein".to_string(),
        ),
    ];
    let report = contamination_check(&triples, &surrogate_set());
    assert!(report.is_clean(), "rejected: {:?}", report.rejected);
    assert_eq!(report.accepted_count, 2);
}

/// The hazard the pass exists for: a fact true of the substitute and false of
/// the term it stood in for.
#[test]
fn content_bearing_axioms_about_a_surrogate_are_rejected() {
    let triples = vec![(
        "EGFR".to_string(),
        "ex:inhibitedBy".to_string(),
        "ex:Erlotinib".to_string(),
    )];
    let report = contamination_check(&triples, &surrogate_set());
    assert!(!report.is_clean());
    assert_eq!(report.rejected.len(), 1);
    assert_eq!(report.rejected[0].surrogate, "EGFR");
}

#[test]
fn a_surrogate_in_object_position_also_contaminates() {
    let triples = vec![(
        "ex:Compound7".to_string(),
        "ex:bindsTo".to_string(),
        "EGFR".to_string(),
    )];
    let report = contamination_check(&triples, &surrogate_set());
    assert!(!report.is_clean());
    assert_eq!(report.rejected[0].surrogate, "EGFR");
}

#[test]
fn triples_not_mentioning_a_surrogate_are_untouched() {
    let triples = vec![(
        "ex:Compound7".to_string(),
        "ex:bindsTo".to_string(),
        "ex:Compound9".to_string(),
    )];
    let report = contamination_check(&triples, &surrogate_set());
    assert!(report.is_clean());
    assert_eq!(report.accepted_count, 1);
}

#[test]
fn full_iri_predicates_are_recognised_as_structural() {
    let triples = vec![(
        "EGFR".to_string(),
        "http://www.w3.org/2000/01/rdf-schema#subClassOf".to_string(),
        "Protein".to_string(),
    )];
    assert!(contamination_check(&triples, &surrogate_set()).is_clean());
}
