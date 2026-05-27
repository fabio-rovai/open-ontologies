use open_ontologies::drift::DriftDetector;
use open_ontologies::kgcl::KgclChange;
use open_ontologies::state::StateDb;
use tempfile::NamedTempFile;

fn setup() -> StateDb {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    StateDb::open(&path).unwrap()
}

#[test]
fn end_to_end_kgcl_addition_and_removal() {
    let db = setup();
    let detector = DriftDetector::new(db);

    let v1 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex:  <http://example.org/> .
        ex:Dog a owl:Class .
        ex:Cat a owl:Class .
    "#;
    let v2 = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex:  <http://example.org/> .
        ex:Dog a owl:Class .
        ex:Bird a owl:Class .
    "#;

    let report = detector.detect_kgcl(v1, v2, 0.7).expect("detect_kgcl");

    // We expect Cat obsoleted, Bird created.
    let obsoleted: Vec<&str> = report
        .changes
        .iter()
        .filter_map(|c| match c {
            KgclChange::NodeObsoletion { about_node, .. } => Some(about_node.as_str()),
            _ => None,
        })
        .collect();
    let created: Vec<&str> = report
        .changes
        .iter()
        .filter_map(|c| match c {
            KgclChange::NodeCreation { about_node, .. } => Some(about_node.as_str()),
            _ => None,
        })
        .collect();

    assert!(
        obsoleted.iter().any(|s| s.contains("Cat")),
        "expected Cat in obsoletions; got {:?}",
        obsoleted
    );
    assert!(
        created.iter().any(|s| s.contains("Bird")),
        "expected Bird in creations; got {:?}",
        created
    );
}

#[test]
fn end_to_end_kgcl_high_confidence_rename_emits_replacement() {
    let db = setup();
    let detector = DriftDetector::new(db);

    // Same property, renamed — the existing drift test (test_drift_detects_likely_rename_by_domain_range)
    // produces a high-confidence rename here because domain+range match exactly.
    let v1 = r#"
        @prefix owl:  <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex:   <http://example.org/> .
        ex:authoredBy a owl:ObjectProperty ;
            rdfs:domain ex:Paper ;
            rdfs:range  ex:Person .
    "#;
    let v2 = r#"
        @prefix owl:  <http://www.w3.org/2002/07/owl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex:   <http://example.org/> .
        ex:writtenBy a owl:ObjectProperty ;
            rdfs:domain ex:Paper ;
            rdfs:range  ex:Person .
    "#;

    let report = detector.detect_kgcl(v1, v2, 0.5).expect("detect_kgcl");

    // Find the authoredBy obsoletion — must carry has_direct_replacement = writtenBy
    let obs = report.changes.iter().find_map(|c| match c {
        KgclChange::NodeObsoletion {
            about_node,
            has_direct_replacement,
            confidence,
            ..
        } if about_node.contains("authoredBy") => {
            Some((has_direct_replacement.clone(), *confidence))
        }
        _ => None,
    });
    let (replacement, confidence) =
        obs.expect("expected obsoletion of authoredBy to be present");
    let repl = replacement.expect("expected has_direct_replacement on a confident rename");
    assert!(
        repl.contains("writtenBy"),
        "replacement should point at writtenBy; got {}",
        repl
    );
    assert!(
        confidence.expect("confidence should be carried") > 0.5,
        "confidence should be above threshold"
    );

    // The CNL should contain the canonical "with replacement" phrasing.
    let cnl = report.to_cnl();
    assert!(
        cnl.contains("with replacement"),
        "expected 'with replacement' in CNL; got:\n{}",
        cnl
    );
}

#[test]
fn end_to_end_kgcl_no_changes_emits_empty_report() {
    let db = setup();
    let detector = DriftDetector::new(db);

    let v = r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex:  <http://example.org/> .
        ex:Dog a owl:Class .
        ex:Cat a owl:Class .
    "#;

    let report = detector.detect_kgcl(v, v, 0.7).expect("detect_kgcl");
    assert!(
        report.changes.is_empty(),
        "no-change drift should produce empty KGCL report; got {:?}",
        report.changes
    );
}
