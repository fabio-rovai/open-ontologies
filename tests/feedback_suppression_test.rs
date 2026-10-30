//! Feedback may waive warnings and infos, never errors, and never silently.
//!
//! Issue #325: once a (tool, rule, entity) had been dismissed
//! `suppress_threshold` times (default 3), lint and enforce dropped it whatever
//! its severity, and the only trace left was `suppressed_count`. The caller that
//! dismisses is the same orchestrator that loops until the report is clean, so
//! three dismissals of an error produced a clean report with no record of which
//! rule or entity had been waived.
//!
//! Pinned:
//!   1. an error dismissed `suppress_threshold` times is still in `violations`
//!      at severity `error`, not downgraded;
//!   2. a warning dismissed that many times leaves `issues`/`violations` and
//!      appears in `suppressed[]` with rule, entity, original severity and
//!      dismissal count;
//!   3. `issue_count` is still computed over `issues` only.

use open_ontologies::enforce::Enforcer;
use open_ontologies::feedback::record_tool_feedback;
use open_ontologies::graph::GraphStore;
use open_ontologies::ontology::OntologyService;
use open_ontologies::state::StateDb;
use std::sync::Arc;

fn db() -> StateDb {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    StateDb::open(&path).unwrap()
}

fn threshold() -> i64 {
    open_ontologies::runtime::feedback_suppress_threshold()
}

fn dismiss(db: &StateDb, tool: &str, rule: &str, entity: &str) {
    for _ in 0..threshold() {
        record_tool_feedback(db, tool, rule, entity, false).unwrap();
    }
}

fn find<'a>(list: &'a serde_json::Value, key: &str, rule: &str, entity_part: &str) -> Option<&'a serde_json::Value> {
    list.as_array().unwrap().iter().find(|v| {
        v[key].as_str() == Some(rule) && v["entity"].as_str().unwrap_or("").contains(entity_part)
    })
}

const IES4_OVERLAP: &str = r#"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ies: <http://ies.data.gov.uk/ontology/ies4#> .
@prefix ex: <http://example.org/> .
ex:Clash a owl:Class ; rdfs:subClassOf ies:Particular , ies:ClassOfEntity .
"#;

#[test]
fn enforce_error_dismissed_to_threshold_stays_visible_at_error() {
    let db = db();
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(IES4_OVERLAP, None).unwrap();
    let enforcer = Enforcer::new(db.clone(), graph);

    let before: serde_json::Value =
        serde_json::from_str(&enforcer.enforce_with_feedback("ies4", Some(&db)).unwrap()).unwrap();
    let v = find(&before["violations"], "rule", "ies4_particular_class_overlap", "example.org/Clash")
        .expect("fixture must raise the error-severity overlap violation");
    assert_eq!(v["severity"], "error");
    let entity = v["entity"].as_str().unwrap().to_string();

    dismiss(&db, "enforce", "ies4_particular_class_overlap", &entity);

    let after: serde_json::Value =
        serde_json::from_str(&enforcer.enforce_with_feedback("ies4", Some(&db)).unwrap()).unwrap();
    let v = find(&after["violations"], "rule", "ies4_particular_class_overlap", "example.org/Clash")
        .expect("an error must never be suppressed by feedback");
    assert_eq!(v["severity"], "error");
    assert!(v.get("adjusted_severity").is_none(), "an error must not be downgraded: {v}");
    assert_eq!(v["dismissals"], threshold());
    assert_eq!(after["suppressed_count"], 0);
}

#[test]
fn enforce_warning_dismissed_to_threshold_moves_to_suppressed() {
    let db = db();
    let graph = Arc::new(GraphStore::new());
    graph
        .load_turtle("@prefix owl: <http://www.w3.org/2002/07/owl#> . <http://example.org/Dog> a owl:Class .", None)
        .unwrap();
    let enforcer = Enforcer::new(db.clone(), graph);

    let before: serde_json::Value =
        serde_json::from_str(&enforcer.enforce_with_feedback("generic", Some(&db)).unwrap()).unwrap();
    let v = find(&before["violations"], "rule", "missing_label", "example.org/Dog").unwrap();
    let severity = v["severity"].as_str().unwrap().to_string();
    assert_ne!(severity, "error", "fixture must pick a non-error rule");
    let entity = v["entity"].as_str().unwrap().to_string();

    dismiss(&db, "enforce", "missing_label", &entity);

    let after: serde_json::Value =
        serde_json::from_str(&enforcer.enforce_with_feedback("generic", Some(&db)).unwrap()).unwrap();
    assert!(find(&after["violations"], "rule", "missing_label", "example.org/Dog").is_none());
    let s = find(&after["suppressed"], "rule", "missing_label", "example.org/Dog")
        .expect("a suppressed violation must be listed in suppressed[]");
    assert_eq!(s["entity"], entity.as_str());
    assert_eq!(s["original_severity"], severity.as_str());
    assert_eq!(s["dismissals"], threshold());
    assert_eq!(after["suppressed_count"], after["suppressed"].as_array().unwrap().len());
}

#[test]
fn lint_warning_dismissed_to_threshold_moves_to_suppressed() {
    let db = db();
    let ttl = "@prefix owl: <http://www.w3.org/2002/07/owl#> . <http://example.org/Dog> a owl:Class .";

    let before: serde_json::Value =
        serde_json::from_str(&OntologyService::lint_with_feedback(ttl, Some(&db)).unwrap()).unwrap();
    let issue = find(&before["issues"], "type", "missing_label", "example.org/Dog").unwrap();
    assert_eq!(issue["severity"], "warning");
    let entity = issue["entity"].as_str().unwrap().to_string();
    let count_before = before["issue_count"].as_u64().unwrap();

    dismiss(&db, "lint", "missing_label", &entity);

    let after: serde_json::Value =
        serde_json::from_str(&OntologyService::lint_with_feedback(ttl, Some(&db)).unwrap()).unwrap();
    assert!(find(&after["issues"], "type", "missing_label", "example.org/Dog").is_none());
    let s = find(&after["suppressed"], "rule", "missing_label", "example.org/Dog")
        .expect("a suppressed lint issue must be listed in suppressed[]");
    assert_eq!(s["entity"], entity.as_str());
    assert_eq!(s["original_severity"], "warning");
    assert_eq!(s["dismissals"], threshold());
    // issue_count covers `issues` only, as before.
    assert_eq!(after["issue_count"].as_u64().unwrap(), count_before - 1);
    assert_eq!(after["issue_count"], after["issues"].as_array().unwrap().len());
}
