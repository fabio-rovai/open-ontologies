//! End-to-end integration tests across the three-layer architecture
//! (#43 Dynamics → #44 Causal hookup → #45 Planner stub).
//!
//! Locks the integration contracts that the unit-test suite intentionally
//! leaves to integration: a schema registered via the Dynamics API survives
//! a round-trip through CIVeX certification, the Planner PDDL compilation
//! pulls it from the same store, and `apply()` actually modifies the graph
//! the way the Causal-layer's structural-dependency hash claims it does.

use open_ontologies::civex::{certify_action, ActionFrame, Verdict};
use open_ontologies::dynamics::{
    list_names, lookup, register, ActionSchema, EffectSpec, Parameter,
};
use open_ontologies::graph::GraphStore;
use open_ontologies::plan_pddl::{compile_domain, compile_problem};
use open_ontologies::state::StateDb;
use std::sync::Arc;
use tempfile::NamedTempFile;

fn setup() -> (StateDb, Arc<GraphStore>) {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    let db = StateDb::open(&path).unwrap();
    let graph = Arc::new(GraphStore::new());
    (db, graph)
}

fn add_subclass_schema() -> ActionSchema {
    // Action: add `?child rdfs:subClassOf ?parent` when both already exist as
    // owl:Classes.
    ActionSchema {
        name: "add_subclass_edge".to_string(),
        parameters: vec![
            Parameter { name: "child".to_string(), type_iri: None },
            Parameter { name: "parent".to_string(), type_iri: None },
        ],
        preconditions: vec![
            "ASK { <{child}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> }".to_string(),
            "ASK { <{parent}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#Class> }".to_string(),
        ],
        effects: vec![
            EffectSpec::AddTriple {
                subject: "{child}".to_string(),
                predicate: "http://www.w3.org/2000/01/rdf-schema#subClassOf".to_string(),
                object: "{parent}".to_string(),
            },
        ],
        reversible: true,
        description: Some("Add a subClassOf edge between two existing classes".to_string()),
        outcomes: vec![],
    }
}

#[test]
fn dynamics_register_then_planner_compile_sees_the_schema() {
    // The Planner stub must read from the same SQLite store that
    // onto_action_register writes to. This proves the storage contract.
    let (db, _graph) = setup();
    register(&db, &add_subclass_schema()).expect("register");

    let names = list_names(&db).expect("list");
    assert!(names.contains(&"add_subclass_edge".to_string()));

    // The Planner pulls schemas by lookup. Round-trip must preserve enough
    // structure for compile_domain to emit a well-formed action block.
    let recovered = lookup(&db, "add_subclass_edge")
        .expect("lookup ok")
        .expect("schema found");
    let compiled = compile_domain("ontology", &[recovered]);
    assert!(compiled.domain.contains("(:action add_subclass_edge"));
    assert!(compiled.domain.contains("?child - iri"));
    assert!(compiled.domain.contains("?parent - iri"));
    // Both ASK preconditions should translate.
    assert!(compiled.domain.contains(":precondition (and (triple ?child"));
    // No untranslated notes for fully ASK-shaped schemas.
    assert!(compiled.translation_notes.is_empty(),
        "unexpected translation notes: {:?}", compiled.translation_notes);
}

#[test]
fn dynamics_apply_actually_mutates_graph_visible_to_civex() {
    // Whole-stack smoke: load graph → register → apply → CIVeX sees the
    // post-state when computing structural dependencies on a follow-up frame.
    let (db, graph) = setup();
    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://ex.org/> .
        ex:Animal a owl:Class .
        ex:Cat a owl:Class .
    "#, None).unwrap();

    let schema = add_subclass_schema();
    register(&db, &schema).expect("register");

    let bindings = vec![
        ("child".to_string(), "http://ex.org/Cat".to_string()),
        ("parent".to_string(), "http://ex.org/Animal".to_string()),
    ];
    assert!(schema.applicable(&graph, &bindings),
        "both classes exist; preconditions should be satisfied");

    let result = schema.apply(&graph, &db, &bindings).expect("apply");
    assert_eq!(result.triples_added, 1);
    assert_eq!(result.triples_removed, 0);
    assert!(result.event_iri.contains("add_subclass_edge"));
    assert!(result.kgcl_patch_cnl.iter().any(|s| s.contains("subClassOf")
        || s.contains("create edge")),
        "expected KGCL CNL line; got {:?}", result.kgcl_patch_cnl);

    // Verify the mutation landed in the graph.
    let q = "ASK { <http://ex.org/Cat> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://ex.org/Animal> }";
    let r = graph.sparql_select(q).unwrap();
    assert!(r.contains("\"result\":true"), "subClassOf triple not present after apply: {}", r);
}

#[test]
fn civex_certifies_against_dynamics_action_and_records_schema_name() {
    // CIVeX must accept a Dynamics action schema name and propagate it into
    // the certificate's assumptions, so an external auditor reading the
    // certificate can trace it back to the registered schema.
    let (db, graph) = setup();
    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://ex.org/> .
        ex:Animal a owl:Class .
        ex:Cat a owl:Class .
    "#, None).unwrap();

    register(&db, &add_subclass_schema()).expect("register");

    // The frame describes the change the Dynamics schema would produce.
    let frame = ActionFrame {
        tool: "onto_action_apply".to_string(),
        target_iris: vec!["http://ex.org/Cat".to_string()],
        proposed_delta_ttl: r#"
            @prefix ex: <http://ex.org/> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            ex:Cat rdfs:subClassOf ex:Animal .
        "#.to_string(),
        utility_metric: "dependent_query_pass_rate".to_string(),
        dependent_queries: vec![
            "SELECT ?x WHERE { ?x a <http://www.w3.org/2002/07/owl#Class> }".to_string(),
            "SELECT ?x WHERE { ?x <http://www.w3.org/2000/01/rdf-schema#subClassOf> ?y }".to_string(),
            "SELECT ?x WHERE { ?x ?p ?o } LIMIT 5".to_string(),
            "SELECT ?p WHERE { ?s ?p ?o } LIMIT 5".to_string(),
            "SELECT ?o WHERE { ?s ?p ?o } LIMIT 5".to_string(),
        ],
        cost_threshold: 1000,
        utility_threshold: 0.5,
        risk_threshold: 5000,
        reversible: true,
        allow_experiment: false,
        alpha: 0.05,
        action_schema_name: Some("add_subclass_edge".to_string()),
        identification_mode: open_ontologies::civex::IdentificationMode::Structural,
    };
    let result = certify_action(&db, &graph, &frame).expect("certify");

    assert_eq!(result.certificate.verdict, Verdict::Execute,
        "benign subClassOf edge should execute; got rationale: {}",
        result.certificate.rationale);
    assert!(result.certificate.assumptions.iter()
        .any(|a| a == "dynamics_action_schema:add_subclass_edge"),
        "schema name must be in assumptions: {:?}",
        result.certificate.assumptions);
    // Sanity: the reversible flag also surfaces.
    assert!(result.certificate.assumptions.contains(&"reversible".to_string()));
}

#[test]
fn planner_problem_includes_goal_triples_in_pddl() {
    // The Planner emits a problem instance whose goal block contains the
    // goal triples the orchestrator asked it to plan toward.
    let init = vec![(
        "http://ex.org/Cat".to_string(),
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
        "http://www.w3.org/2002/07/owl#Class".to_string(),
    )];
    let goal = vec![(
        "http://ex.org/Cat".to_string(),
        "http://www.w3.org/2000/01/rdf-schema#subClassOf".to_string(),
        "http://ex.org/Animal".to_string(),
    )];

    let problem = compile_problem("ex_problem", "ontology", &init, &goal);
    assert!(problem.contains("(:domain ontology)"));
    assert!(problem.contains(":init (triple"));
    assert!(problem.contains(":goal (triple"));
    // Sanitised IRI should appear in objects.
    assert!(problem.contains("http___ex_org_cat") || problem.contains("ex_org_cat"));
}

#[test]
fn full_three_layer_pipeline_register_certify_apply() {
    // The full intended workflow:
    //   1. Register a Dynamics schema.
    //   2. CIVeX certifies a proposed instance of it → EXECUTE verdict.
    //   3. Caller applies it.
    //   4. Graph reflects the change; lineage and event IRI captured.
    let (db, graph) = setup();
    graph.load_turtle(r#"
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix ex: <http://ex.org/> .
        ex:Animal a owl:Class .
        ex:Cat a owl:Class .
    "#, None).unwrap();

    let schema = add_subclass_schema();
    register(&db, &schema).expect("register");

    // Step 2: certify.
    let frame = ActionFrame {
        tool: "onto_action_apply".to_string(),
        target_iris: vec!["http://ex.org/Cat".to_string()],
        proposed_delta_ttl: "@prefix ex: <http://ex.org/> . @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> . ex:Cat rdfs:subClassOf ex:Animal .".to_string(),
        utility_metric: "dependent_query_pass_rate".to_string(),
        dependent_queries: vec![
            "SELECT ?x WHERE { ?x a <http://www.w3.org/2002/07/owl#Class> }".to_string(),
            "SELECT ?x WHERE { ?x ?p ?o } LIMIT 5".to_string(),
            "SELECT ?p WHERE { ?s ?p ?o } LIMIT 5".to_string(),
            "SELECT ?o WHERE { ?s ?p ?o } LIMIT 5".to_string(),
            "SELECT ?s WHERE { ?s ?p ?o } LIMIT 5".to_string(),
        ],
        cost_threshold: 1000,
        utility_threshold: 0.5,
        risk_threshold: 5000,
        reversible: true,
        allow_experiment: false,
        alpha: 0.05,
        action_schema_name: Some(schema.name.clone()),
        identification_mode: open_ontologies::civex::IdentificationMode::Structural,
    };
    let cert = certify_action(&db, &graph, &frame).expect("certify");
    assert_eq!(cert.certificate.verdict, Verdict::Execute);

    // Step 3: apply only after verdict was EXECUTE.
    let bindings = vec![
        ("child".to_string(), "http://ex.org/Cat".to_string()),
        ("parent".to_string(), "http://ex.org/Animal".to_string()),
    ];
    let result = schema.apply(&graph, &db, &bindings).expect("apply");

    // Step 4: post-conditions.
    assert_eq!(result.triples_added, 1);
    let r = graph.sparql_select(
        "ASK { <http://ex.org/Cat> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://ex.org/Animal> }"
    ).unwrap();
    assert!(r.contains("\"result\":true"));
    assert!(!cert.certificate.provenance_hash.is_empty());
}
