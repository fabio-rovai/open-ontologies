use open_ontologies::graph::GraphStore;
use open_ontologies::shacl::ShaclValidator;
use oxigraph::io::RdfFormat;
use std::sync::Arc;

fn make_store_with_data() -> Arc<GraphStore> {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        ex:b1 a ex:Building ; rdfs:label "Tower Bridge" ; ex:height "65"^^xsd:integer .
        ex:b2 a ex:Building ; ex:height "96"^^xsd:integer .
    "#;
    store.load_turtle(ttl, None).unwrap();
    store
}

#[test]
fn test_shacl_mincount_violation() {
    let store = make_store_with_data();
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:BuildingShape a sh:NodeShape ;
            sh:targetClass ex:Building ;
            sh:property [
                sh:path rdfs:label ;
                sh:minCount 1 ;
                sh:message "Building must have a label" ;
            ] .
    "#;
    let result = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["conforms"], false);
    assert!(parsed["violation_count"].as_u64().unwrap() >= 1);
    // b2 has no label
    let violations = parsed["violations"].as_array().unwrap();
    assert!(violations.iter().any(|v| {
        v["focus_node"].as_str().unwrap().contains("b2")
    }));
}

#[test]
fn test_shacl_all_pass() {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:b1 a ex:Building ; rdfs:label "Tower Bridge" .
        ex:b2 a ex:Building ; rdfs:label "Big Ben" .
    "#;
    store.load_turtle(ttl, None).unwrap();
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:BuildingShape a sh:NodeShape ;
            sh:targetClass ex:Building ;
            sh:property [
                sh:path rdfs:label ;
                sh:minCount 1 ;
            ] .
    "#;
    let result = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["conforms"], true);
    assert_eq!(parsed["violation_count"], 0);
}

#[test]
fn test_shacl_maxcount_violation() {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:b1 a ex:Building ; rdfs:label "Tower Bridge" ; rdfs:label "Le pont de la Tour" .
    "#;
    store.load_turtle(ttl, None).unwrap();
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:BuildingShape a sh:NodeShape ;
            sh:targetClass ex:Building ;
            sh:property [
                sh:path rdfs:label ;
                sh:maxCount 1 ;
            ] .
    "#;
    let result = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["conforms"], false);
    assert!(parsed["violation_count"].as_u64().unwrap() >= 1);
}

#[test]
fn test_shacl_datatype_violation() {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        ex:b1 a ex:Building ; ex:height "sixty-five" .
    "#;
    store.load_turtle(ttl, None).unwrap();
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
        ex:BuildingShape a sh:NodeShape ;
            sh:targetClass ex:Building ;
            sh:property [
                sh:path ex:height ;
                sh:datatype xsd:integer ;
            ] .
    "#;
    let result = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["conforms"], false);
    assert!(parsed["violation_count"].as_u64().unwrap() >= 1);
}

#[test]
fn test_shacl_inverse_path_mincount() {
    // A registrant must be pointed at by at least one fund via ^fundOf.
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:reg1 a ex:Registrant .
        ex:reg2 a ex:Registrant .
        ex:fund1 a ex:Fund ; ex:fundOf ex:reg1 .
    "#;
    store.load_turtle(ttl, None).unwrap();
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:RegistrantShape a sh:NodeShape ;
            sh:targetClass ex:Registrant ;
            sh:property [
                sh:path [ sh:inversePath ex:fundOf ] ;
                sh:minCount 1 ;
                sh:message "Registrant with no fund series attached" ;
            ] .
    "#;
    let result = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["conforms"], false);
    let violations = parsed["violations"].as_array().unwrap();
    // reg2 has no inbound fundOf; reg1 does.
    assert!(violations.iter().any(|v| v["focus_node"].as_str().unwrap().contains("reg2")));
    assert!(!violations.iter().any(|v| v["focus_node"].as_str().unwrap().contains("reg1")));
}

#[test]
fn test_shacl_pattern_violation() {
    // LEI-style syntax rule: 18 alphanumerics + 2 decimal check digits.
    // id1 conforms; id2 is 19 characters (a truncation) and must fire.
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:id1 a ex:LEI ; ex:value "969500AAAAAAAAAAAA75" .
        ex:id2 a ex:LEI ; ex:value "969500BBBBBBBBBBB12" .
    "#;
    store.load_turtle(ttl, None).unwrap();
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:LEIShape a sh:NodeShape ;
            sh:targetClass ex:LEI ;
            sh:property [
                sh:path ex:value ;
                sh:pattern "^[A-Z0-9]{18}[0-9]{2}$" ;
                sh:message "LEI must be 20 characters (ISO 17442)" ;
            ] .
    "#;
    let result = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["conforms"], false);
    let violations = parsed["violations"].as_array().unwrap();
    assert!(violations.iter().any(|v| v["focus_node"].as_str().unwrap().contains("id2")));
    assert!(!violations.iter().any(|v| v["focus_node"].as_str().unwrap().contains("id1")));
    assert!(violations.iter().any(|v| v["constraint"] == "pattern"));
}

#[test]
fn test_shacl_has_value_violation() {
    // Checksum policy: every LEI node must record ex:checksumValid true.
    // id1 records true; id2 records false; id3 records nothing.
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:id1 a ex:LEI ; ex:checksumValid true .
        ex:id2 a ex:LEI ; ex:checksumValid false .
        ex:id3 a ex:LEI .
    "#;
    store.load_turtle(ttl, None).unwrap();
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:LEIChecksumShape a sh:NodeShape ;
            sh:targetClass ex:LEI ;
            sh:property [
                sh:path ex:checksumValid ;
                sh:hasValue true ;
                sh:message "LEI check digits fail or are unrecorded" ;
            ] .
    "#;
    let result = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["conforms"], false);
    let violations = parsed["violations"].as_array().unwrap();
    assert!(violations.iter().any(|v| v["focus_node"].as_str().unwrap().contains("id2")));
    assert!(violations.iter().any(|v| v["focus_node"].as_str().unwrap().contains("id3")));
    assert!(!violations.iter().any(|v| v["focus_node"].as_str().unwrap().contains("id1")));
    assert!(violations.iter().any(|v| v["constraint"] == "hasValue"));
}

#[test]
fn test_shacl_unsupported_path_skipped_not_fatal() {
    // A sequence path is not executable; it must be reported as skipped
    // while the rest of the shapes still run.
    let store = make_store_with_data();
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        ex:BuildingShape a sh:NodeShape ;
            sh:targetClass ex:Building ;
            sh:property [
                sh:path ( ex:inZone ex:zoneName ) ;
                sh:minCount 1 ;
            ] ;
            sh:property [
                sh:path rdfs:label ;
                sh:minCount 1 ;
            ] .
    "#;
    let result = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    // The label constraint still fires on b2.
    assert!(parsed["violations"].as_array().unwrap().iter().any(|v| {
        v["focus_node"].as_str().unwrap().contains("b2")
    }));
    assert!(parsed["skipped_constraints"].as_array().unwrap().len() == 1);
}

// Regression tests for four false-clean defects found while validating the
// Scotland land register ontology. In every case the validator previously
// returned `conforms: true` over data that violated the shape, or over a shapes
// graph it could not evaluate at all. A false clean is worse than an error,
// because the caller has no signal that anything was missed.

fn store_with(ttl: &str) -> Arc<GraphStore> {
    let store = Arc::new(GraphStore::new());
    store.load_turtle(ttl, None).unwrap();
    store
}

const NOT_DATA: &str = r#"
    @prefix ex: <http://example.org/> .
    ex:a a ex:Thing ; ex:mech ex:bad .
"#;

#[test]
fn test_sh_not_is_not_silently_ignored() {
    // `sh:not` is not implemented. It must be reported as skipped, which
    // suppresses the verdict, rather than passing silently.
    let store = store_with(NOT_DATA);
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:mech ; sh:not [ sh:hasValue ex:bad ] ] .
    "#;
    let report = ShaclValidator::validate(&store, shapes).unwrap();
    let v: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert!(v["conforms"].is_null(), "must not claim a pass: {v}");
    assert!(
        v["skipped_constraints"]
            .as_array()
            .is_some_and(|a| !a.is_empty()),
        "sh:not must be recorded as skipped: {v}"
    );
}

#[test]
fn test_target_node_shape_does_not_report_a_pass() {
    // `sh:targetNode` selects no focus nodes here, so the shapes graph yields
    // nothing evaluable. Previously this reported conforms: true.
    let store = store_with("@prefix ex: <http://example.org/> . ex:b a ex:Thing .");
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetNode ex:b ;
            sh:property [ sh:path ex:p ; sh:minCount 1 ] .
    "#;
    let report = ShaclValidator::validate(&store, shapes).unwrap();
    let v: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert!(v["conforms"].is_null(), "must not claim a pass: {v}");
}

#[test]
fn test_implicit_class_target_is_evaluated() {
    // A node that is both sh:NodeShape and rdfs:Class targets its instances.
    // This is now implemented, so it must find the real violation.
    let store = store_with("@prefix ex: <http://example.org/> . ex:b a ex:Thing .");
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix ex: <http://example.org/> .
        ex:Thing a sh:NodeShape, rdfs:Class ;
            sh:property [ sh:path ex:p ; sh:minCount 1 ] .
    "#;
    let report = ShaclValidator::validate(&store, shapes).unwrap();
    let v: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(v["conforms"], serde_json::Value::Bool(false), "report: {v}");
    assert_eq!(v["violation_count"], 1, "report: {v}");
}

#[test]
fn test_target_class_still_works() {
    // Guard against the fixes above regressing the one target form that always
    // worked correctly.
    let store = store_with("@prefix ex: <http://example.org/> . ex:b a ex:Thing .");
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:p ; sh:minCount 1 ] .
    "#;
    let report = ShaclValidator::validate(&store, shapes).unwrap();
    let v: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(v["conforms"], serde_json::Value::Bool(false), "report: {v}");
    assert_eq!(v["violation_count"], 1, "report: {v}");
}

// Regression tests for the node-shape gap (issue #108). The unimplemented
// constraint check above reached one sh:property hop below the shape and no
// further, so a constraint asserted on the node shape itself was never
// evaluated and never recorded: `sh:closed true` over data carrying an
// undeclared predicate returned `conforms: true`.

const SH_NS: &str = "http://www.w3.org/ns/shacl#";

fn skipped_names(v: &serde_json::Value, shape: &str, constraint: &str) -> bool {
    v["skipped_constraints"].as_array().is_some_and(|a| {
        a.iter()
            .any(|e| e["shape"] == shape && e["constraint"] == constraint)
    })
}

#[test]
fn test_node_shape_constraints_reach_the_tri_state_verdict() {
    // sh:closed, a node-level sh:not and sh:nodeKind are not implemented. Each
    // must be recorded as skipped under its own IRI, and the verdict must be
    // null, not a pass. Data: ex:a carries ex:mech, undeclared by any shape.
    let store = store_with(NOT_DATA);
    let cases = [
        ("sh:closed true", "closed"),
        (
            "sh:not [ sh:property [ sh:path ex:mech ; sh:hasValue ex:bad ] ]",
            "not",
        ),
        ("sh:nodeKind sh:IRI", "nodeKind"),
    ];
    for (constraint_ttl, local) in cases {
        let shapes = format!(
            r#"
            @prefix sh: <http://www.w3.org/ns/shacl#> .
            @prefix ex: <http://example.org/> .
            ex:S a sh:NodeShape ; sh:targetClass ex:Thing ; {constraint_ttl} .
        "#
        );
        let report = ShaclValidator::validate(&store, &shapes).unwrap();
        let v: serde_json::Value = serde_json::from_str(&report).unwrap();
        assert!(
            v["conforms"].is_null(),
            "{local}: must not claim a pass: {v}"
        );
        assert!(
            skipped_names(&v, "http://example.org/S", &format!("{SH_NS}{local}")),
            "{local}: must be recorded as skipped on ex:S: {v}"
        );
    }
}

#[test]
fn test_tri_state_verdict_is_reachable_from_node_shape_property_shape_and_target() {
    // A validator has three answers, not two, and every path that can select
    // nothing or execute nothing has to reach the third one. This pins that
    // the null verdict is reachable from each of the three places a SHACL
    // construct can sit, each with its own skipped entry naming the construct,
    // so the next construct added has somewhere obvious to fail.
    let store = store_with(NOT_DATA);
    let cases = [
        (
            "node shape",
            r#"
            @prefix sh: <http://www.w3.org/ns/shacl#> .
            @prefix ex: <http://example.org/> .
            ex:S a sh:NodeShape ; sh:targetClass ex:Thing ; sh:closed true .
            "#,
            format!("{SH_NS}closed"),
        ),
        (
            "property shape",
            r#"
            @prefix sh: <http://www.w3.org/ns/shacl#> .
            @prefix ex: <http://example.org/> .
            ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
                sh:property [ sh:path ex:mech ; sh:not [ sh:hasValue ex:bad ] ] .
            "#,
            format!("{SH_NS}not"),
        ),
        (
            "target",
            r#"
            @prefix sh: <http://www.w3.org/ns/shacl#> .
            @prefix ex: <http://example.org/> .
            ex:S a sh:NodeShape ; sh:targetNode ex:a ;
                sh:property [ sh:path ex:mech ; sh:minCount 1 ] .
            "#,
            "sh:targetNode".to_string(),
        ),
    ];
    for (place, shapes, construct) in cases {
        let report = ShaclValidator::validate(&store, shapes).unwrap();
        let v: serde_json::Value = serde_json::from_str(&report).unwrap();
        assert!(
            v["conforms"].is_null(),
            "{place}: must not claim a pass: {v}"
        );
        assert!(
            skipped_names(&v, "http://example.org/S", &construct),
            "{place}: skipped entry must name {construct}: {v}"
        );
    }
}

#[test]
fn test_node_shape_annotations_and_class_axioms_keep_a_boolean_verdict() {
    // The node-shape complement must not mistake what it does read for a
    // constraint. Annotation predicates on an explicit shape, and the class's
    // own axioms on an implicit class target (which carries rdfs:subClassOf,
    // rdfs:label and rdfs:comment on the same subject), leave the verdict
    // boolean and the skipped list absent.
    let store = store_with("@prefix ex: <http://example.org/> . ex:b a ex:Thing .");
    let cases = [
        (
            "annotated explicit shape",
            r#"
            @prefix sh: <http://www.w3.org/ns/shacl#> .
            @prefix ex: <http://example.org/> .
            ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
                sh:name "Thing shape" ; sh:description "Every thing has a p." ;
                sh:severity sh:Warning ;
                sh:property [ sh:path ex:p ; sh:minCount 1 ] .
            "#,
        ),
        (
            "implicit class target with class axioms",
            r#"
            @prefix sh: <http://www.w3.org/ns/shacl#> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            @prefix ex: <http://example.org/> .
            ex:Thing a sh:NodeShape, rdfs:Class ;
                rdfs:label "Thing" ; rdfs:comment "A thing." ;
                rdfs:subClassOf ex:Top ;
                sh:property [ sh:path ex:p ; sh:minCount 1 ] .
            "#,
        ),
    ];
    for (case, shapes) in cases {
        let report = ShaclValidator::validate(&store, shapes).unwrap();
        let v: serde_json::Value = serde_json::from_str(&report).unwrap();
        assert_eq!(v["conforms"], serde_json::Value::Bool(false), "{case}: {v}");
        assert_eq!(v["violation_count"], 1, "{case}: {v}");
        assert!(
            v.get("skipped_constraints").is_none(),
            "{case}: nothing here is a constraint to skip: {v}"
        );
    }
}

#[test]
fn test_deactivated_shape_is_recorded_as_skipped() {
    // SHACL says a deactivated shape must not be evaluated. This validator
    // still evaluates it, so `sh:deactivated` is not implemented and must land
    // in skipped rather than be whitelisted as if it were honoured.
    let store = store_with("@prefix ex: <http://example.org/> . ex:b a ex:Thing .");
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ; sh:deactivated true ;
            sh:property [ sh:path ex:p ; sh:minCount 1 ] .
    "#;
    let report = ShaclValidator::validate(&store, shapes).unwrap();
    let v: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert!(v["conforms"].is_null(), "must not claim a verdict: {v}");
    assert!(
        skipped_names(&v, "http://example.org/S", &format!("{SH_NS}deactivated")),
        "sh:deactivated must be recorded as skipped: {v}"
    );
}

#[test]
fn test_blank_node_shape_is_bound_as_itself_not_as_a_wildcard() {
    // A shape written `[] a sh:NodeShape` is a blank node, and a blank-node
    // label spliced into a SPARQL query is a non-distinguished variable, not
    // a name. A node complement built by splicing the shape term enumerated
    // every predicate in the shapes graph, routed the property shape's own
    // sh:path and sh:minCount to skipped, and turned a boolean verdict into
    // null. The shape must be bound as itself: a blank-node shape carrying
    // only property constraints keeps its boolean verdict, and one carrying
    // sh:closed still reaches skipped under its own label.
    let store = store_with(NOT_DATA);
    let property_only = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        [] a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:p ; sh:minCount 1 ] .
    "#;
    let report = ShaclValidator::validate(&store, property_only).unwrap();
    let v: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(
        v["conforms"],
        serde_json::Value::Bool(false),
        "property only: {v}"
    );
    assert_eq!(v["violation_count"], 1, "property only: {v}");
    assert!(
        v.get("skipped_constraints").is_none(),
        "property only: nothing here is a constraint to skip: {v}"
    );

    let closed = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        [] a sh:NodeShape ; sh:targetClass ex:Thing ; sh:closed true .
    "#;
    let report = ShaclValidator::validate(&store, closed).unwrap();
    let v: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert!(
        v["conforms"].is_null(),
        "closed: must not claim a pass: {v}"
    );
    let entries: Vec<&serde_json::Value> = v["skipped_constraints"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|e| e["constraint"] == format!("{SH_NS}closed"))
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(entries.len(), 1, "closed: exactly one entry: {v}");
    assert!(
        entries[0]["shape"]
            .as_str()
            .is_some_and(|s| s.starts_with("_:")),
        "closed: the entry names the blank-node shape: {v}"
    );
}

#[test]
fn test_node_constraint_is_recorded_once_however_many_target_classes() {
    // Shape discovery yields one row per target class. A node constraint
    // belongs to the shape, not to the pair, so a shape with two target
    // classes reports its sh:closed once, not once per class.
    let store =
        store_with("@prefix ex: <http://example.org/> . ex:b a ex:Thing . ex:c a ex:Other .");
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing, ex:Other ; sh:closed true .
    "#;
    let report = ShaclValidator::validate(&store, shapes).unwrap();
    let v: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert!(v["conforms"].is_null(), "must not claim a pass: {v}");
    let closed_entries = v["skipped_constraints"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|e| {
                    e["shape"] == "http://example.org/S"
                        && e["constraint"] == format!("{SH_NS}closed")
                })
                .count()
        })
        .unwrap_or(0);
    assert_eq!(closed_entries, 1, "sh:closed recorded once: {v}");
}

#[test]
fn test_node_constraint_on_a_shape_without_target_is_not_a_gap() {
    // A node shape with no target selects no focus nodes under SHACL, so its
    // constraints not running is what the specification asks for, not a
    // gap. Its sh:closed must not be recorded as skipped, or every shapes
    // graph carrying an unreferenced helper shape would read undetermined.
    let store = store_with("@prefix ex: <http://example.org/> . ex:b a ex:Thing ; ex:p ex:x .");
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:p ; sh:minCount 1 ] .
        ex:Helper a sh:NodeShape ; sh:closed true .
    "#;
    let report = ShaclValidator::validate(&store, shapes).unwrap();
    let v: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(v["conforms"], serde_json::Value::Bool(true), "report: {v}");
    assert!(
        v.get("skipped_constraints").is_none(),
        "a targetless shape has nothing to evaluate: {v}"
    );
}

#[test]
fn test_false_valued_controls_keep_a_boolean_verdict() {
    // `sh:closed false` is the SHACL default and restricts nothing;
    // `sh:deactivated false` says evaluate this shape, which this validator
    // does. Both are honoured in full, so neither is a constraint that was
    // missed and neither may suppress the verdict. A null verdict on a run
    // where nothing went unevaluated is a false undetermined, and it costs
    // the same as the false clean the complement exists to prevent.
    //
    // The value is read by value, not by lexical form, so "0"^^xsd:boolean is
    // the same false as `false`. Reverting the guard to the name-only filter
    // reddens every case here.
    let store = store_with(NOT_DATA);
    let cases = [
        (
            "canonical false",
            r#"sh:closed false ; sh:deactivated false"#,
        ),
        (
            "alternative lexical form",
            r#"sh:closed "0"^^<http://www.w3.org/2001/XMLSchema#boolean>"#,
        ),
    ];
    for (case, controls) in cases {
        let shapes = format!(
            r#"
            @prefix sh: <http://www.w3.org/ns/shacl#> .
            @prefix ex: <http://example.org/> .
            ex:S a sh:NodeShape ; sh:targetClass ex:Thing ; {controls} ;
                sh:property [ sh:path ex:p ; sh:minCount 1 ] .
            "#
        );
        let report = ShaclValidator::validate(&store, &shapes).unwrap();
        let v: serde_json::Value = serde_json::from_str(&report).unwrap();
        assert_eq!(v["conforms"], serde_json::Value::Bool(false), "{case}: {v}");
        assert_eq!(v["violation_count"], 1, "{case}: {v}");
        assert!(
            v.get("skipped_constraints").is_none(),
            "{case}: a false control was honoured, not skipped: {v}"
        );
    }
}

#[test]
fn test_a_control_this_validator_cannot_read_stays_skipped() {
    // The exemption is for a false BOOLEAN, not for the predicate. A value
    // that is a plain string, or not a literal at all, is a value this
    // validator cannot read and therefore cannot honour, so it must stay in
    // skipped and suppress the verdict.
    //
    // Reverting the exemption to a name-only filter leaves this test green,
    // and that is correct: it pins the boundary of the exemption, not the
    // exemption itself. Dropping the isLiteral/datatype guard in front of the
    // equality also leaves it green, which was measured rather than assumed.
    // Oxigraph answers `"false"^^xsd:string = false` with false instead of
    // the type error SPARQL 1.1 allows, so the guard changes no answer on
    // this evaluator and cannot be mutation-covered here. The reason it stays
    // in the query is in the comment beside it: on an evaluator that does
    // raise, the error would propagate out of the FILTER, drop the solution
    // and silently stop skipping the controls this test is about.
    let store = store_with(NOT_DATA);
    let cases = [
        ("plain string", r#""false""#, "closed"),
        (
            "typed as a string",
            r#""false"^^<http://www.w3.org/2001/XMLSchema#string>"#,
            "closed",
        ),
        (
            "an IRI, not a literal",
            "<http://example.org/Maybe>",
            "closed",
        ),
        ("deactivated as a string", r#""false""#, "deactivated"),
    ];
    for (case, value, predicate) in cases {
        let shapes = format!(
            r#"
            @prefix sh: <http://www.w3.org/ns/shacl#> .
            @prefix ex: <http://example.org/> .
            ex:S a sh:NodeShape ; sh:targetClass ex:Thing ; sh:{predicate} {value} ;
                sh:property [ sh:path ex:p ; sh:minCount 1 ] .
            "#
        );
        let report = ShaclValidator::validate(&store, &shapes).unwrap();
        let v: serde_json::Value = serde_json::from_str(&report).unwrap();
        assert!(
            v["conforms"].is_null(),
            "{case}: must not claim a verdict: {v}"
        );
        assert!(
            skipped_names(&v, "http://example.org/S", &format!("{SH_NS}{predicate}")),
            "{case}: an unreadable control must stay skipped: {v}"
        );
    }
}

#[test]
fn test_a_true_control_is_still_skipped_beside_a_false_one() {
    // The exemption is per value, not per shape and not per predicate. A
    // shape carrying `sh:closed true` and `sh:deactivated false` has one
    // construct that was not evaluated and one that was honoured, and the
    // report must say exactly that: sh:closed skipped, sh:deactivated absent,
    // verdict null. Exempting the predicate rather than the value reddens it.
    let store = store_with(NOT_DATA);
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
            sh:closed true ; sh:deactivated false ;
            sh:property [ sh:path ex:p ; sh:minCount 1 ] .
    "#;
    let report = ShaclValidator::validate(&store, shapes).unwrap();
    let v: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert!(v["conforms"].is_null(), "sh:closed true was not run: {v}");
    assert!(
        skipped_names(&v, "http://example.org/S", &format!("{SH_NS}closed")),
        "sh:closed true must be recorded: {v}"
    );
    assert!(
        !skipped_names(&v, "http://example.org/S", &format!("{SH_NS}deactivated")),
        "sh:deactivated false was honoured and must not be recorded: {v}"
    );
}

fn store_from_trig(dataset: &str) -> Arc<GraphStore> {
    let store = Arc::new(GraphStore::new());
    store
        .load_content(dataset, RdfFormat::TriG)
        .expect("load TriG");
    store
}

const ONE_SHAPE: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    ex:S a sh:NodeShape ; sh:targetClass ex:Thing ;
        sh:property [ sh:path ex:p ; sh:minCount 1 ] .
"#;

#[test]
fn test_instance_data_in_a_named_graph_is_validated() {
    // Every data-side query ran over the store's default graph alone, so the
    // verdict depended on the serialisation the data arrived in. The same
    // triples in Turtle validated and in TriG selected no focus nodes at all,
    // and the report said `nothing_matched` with a null verdict: the right
    // answer to a question nobody asked, which is why this never arrived as a
    // bug report. Reverting `sparql_select_union` to `sparql_select` reddens
    // this test.
    let store = store_from_trig(
        r#"
        @prefix ex: <http://example.org/> .
        ex:data { ex:a a ex:Thing . }
        "#,
    );
    let report = ShaclValidator::validate(&store, ONE_SHAPE).unwrap();
    let v: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(v["focus_nodes"], 1, "the instance must be selected: {v}");
    assert_eq!(
        v["conforms"],
        serde_json::Value::Bool(false),
        "ex:a has no ex:p, so this is a violation and not an absence: {v}"
    );
    assert_eq!(v["violation_count"], 1, "report: {v}");
    assert!(
        v["unmatched_shapes"]
            .as_array()
            .is_some_and(|a| a.is_empty()),
        "the shape matched, so it is not unmatched: {v}"
    );
}

#[test]
fn test_data_split_across_graphs_is_one_dataset_and_not_many() {
    // The union is the RDF merge of every graph read as one default graph, not
    // the same query run once per graph. A focus node typed in one graph whose
    // required value sits in another must satisfy sh:minCount: per-graph
    // evaluation would report a violation here, because neither graph carries
    // both halves. This is the case that tells the two readings apart, and the
    // one that would make a fix look like it worked while quietly reporting
    // violations nobody has.
    let store = store_from_trig(
        r#"
        @prefix ex: <http://example.org/> .
        ex:types  { ex:a a ex:Thing . }
        ex:values { ex:a ex:p "present" . }
        "#,
    );
    let report = ShaclValidator::validate(&store, ONE_SHAPE).unwrap();
    let v: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(v["focus_nodes"], 1, "report: {v}");
    assert_eq!(
        v["conforms"],
        serde_json::Value::Bool(true),
        "the value is in the store, so the shape is satisfied: {v}"
    );
}

#[test]
fn test_the_default_graph_is_still_read() {
    // Widening to the union must not trade one blindness for the other. A
    // store loaded from Turtle puts everything in the default graph, which is
    // the overwhelmingly common case, and it must answer exactly as before.
    let store = store_from_trig(
        r#"
        @prefix ex: <http://example.org/> .
        ex:a a ex:Thing .
        ex:named { ex:b a ex:Thing ; ex:p "present" . }
        "#,
    );
    let report = ShaclValidator::validate(&store, ONE_SHAPE).unwrap();
    let v: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(v["focus_nodes"], 2, "both instances count: {v}");
    assert_eq!(
        v["violation_count"], 1,
        "ex:a violates and ex:b does not: {v}"
    );
    assert_eq!(
        v["violations"][0]["focus_node"], "http://example.org/a",
        "the default-graph instance is the one that violates: {v}"
    );
}

#[test]
fn test_the_report_names_the_scope_it_selected_over() {
    // A verdict that does not say what it selected over cannot be replayed or
    // compared against the next one. The key is added before temporal scoping
    // arrives rather than on the run where it first matters.
    let store = store_from_trig("@prefix ex: <http://example.org/> . ex:a a ex:Thing .");
    let report = ShaclValidator::validate(&store, ONE_SHAPE).unwrap();
    let v: serde_json::Value = serde_json::from_str(&report).unwrap();
    assert_eq!(v["scope"], "all_graphs", "report: {v}");
}

// Range constraints. Before these were implemented, a shape carrying
// sh:minInclusive or sh:maxInclusive was reported in skipped_constraints and
// suppressed `conforms` to null, so a validator run could neither pass nor fail
// on a numeric bound. Found while validating the trade remedy ontology, where a
// resolution-cardinality bound was the whole point of the shape.

fn store_with_numbers() -> Arc<GraphStore> {
    let store = Arc::new(GraphStore::new());
    let ttl = r#"
        @prefix ex: <http://example.org/> .
        ex:a a ex:Thing ; ex:n 0 .
        ex:b a ex:Thing ; ex:n 1 .
        ex:c a ex:Thing ; ex:n 2 .
        ex:d a ex:Thing ; ex:n 5 .
        ex:e a ex:Thing ; ex:n -3 .
    "#;
    store.load_turtle(ttl, None).unwrap();
    store
}

const RANGE_SHAPE: &str = r#"
    @prefix sh: <http://www.w3.org/ns/shacl#> .
    @prefix ex: <http://example.org/> .
    ex:RangeShape a sh:NodeShape ;
        sh:targetClass ex:Thing ;
        sh:property [
            sh:path ex:n ;
            sh:minInclusive 1 ;
            sh:maxInclusive 2 ;
            sh:message "n must be between 1 and 2 inclusive" ;
        ] .
"#;

#[test]
fn test_shacl_inclusive_range_matches_pyshacl() {
    let store = store_with_numbers();
    let result = ShaclValidator::validate(&store, RANGE_SHAPE).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    // pyshacl 0.40.1 flags exactly a, d and e on this fixture.
    assert_eq!(parsed["conforms"], false);
    assert_eq!(parsed["violation_count"].as_u64().unwrap(), 3);
    let v = parsed["violations"].as_array().unwrap();
    for expected in ["a", "d", "e"] {
        assert!(
            v.iter().any(|x| x["focus_node"].as_str().unwrap().ends_with(expected)),
            "expected {} to violate the range", expected
        );
    }
    assert!(!v.iter().any(|x| x["focus_node"].as_str().unwrap().ends_with("b")));
    assert!(!v.iter().any(|x| x["focus_node"].as_str().unwrap().ends_with("c")));
}

#[test]
fn test_shacl_range_constraints_are_no_longer_skipped() {
    let store = store_with_numbers();
    let result = ShaclValidator::validate(&store, RANGE_SHAPE).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    let skipped = parsed["skipped_constraints"].as_array();
    if let Some(s) = skipped {
        for item in s {
            let c = item["constraint"].as_str().unwrap_or("");
            assert!(!c.contains("Inclusive"), "range constraint still skipped: {}", c);
            assert!(!c.contains("Exclusive"), "range constraint still skipped: {}", c);
        }
    }
    // conforms must be a real boolean, never null, once nothing is skipped.
    assert!(parsed["conforms"].is_boolean());
}

#[test]
fn test_shacl_exclusive_range() {
    let store = store_with_numbers();
    let shapes = r#"
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix ex: <http://example.org/> .
        ex:ExShape a sh:NodeShape ;
            sh:targetClass ex:Thing ;
            sh:property [ sh:path ex:n ; sh:minExclusive 1 ; sh:maxExclusive 5 ] .
    "#;
    let result = ShaclValidator::validate(&store, shapes).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    // Violations: 0, 1 (not > 1), 5 (not < 5) and -3. Passing: 2.
    assert_eq!(parsed["conforms"], false);
    assert_eq!(parsed["violation_count"].as_u64().unwrap(), 4);
}
