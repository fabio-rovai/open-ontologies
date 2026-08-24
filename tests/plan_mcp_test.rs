//! The MCP `onto_plan` / `onto_apply` handlers must share plan state.
//!
//! Part of the #91 regression set. `onto_plan` and `onto_apply` are independent
//! `#[tool]` handlers and each builds its own `Planner`, so nothing about the
//! long-lived server struct saved them: the plan has to be in the state db.
//!
//! This drives the handlers themselves rather than a `Planner`, because the
//! handler pair *is* the surface an MCP client hits.

use open_ontologies::graph::GraphStore;
use open_ontologies::inputs::{OntoApplyInput, OntoPlanInput};
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;
use rmcp::handler::server::wrapper::Parameters;
use std::sync::Arc;

const BASE: &str = r#"
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix ex: <https://example.org/> .
    ex:Persona a owl:Class .
"#;

const PROPOSED: &str = r#"
    @prefix owl: <http://www.w3.org/2002/07/owl#> .
    @prefix ex: <https://example.org/> .
    ex:Persona a owl:Class .
    ex:Organizacion a owl:Class .
"#;

fn server() -> (tempfile::TempDir, OpenOntologiesServer, Arc<GraphStore>) {
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("state.db")).unwrap();
    let graph = Arc::new(GraphStore::new());
    graph.load_turtle(BASE, None).unwrap();
    let server = OpenOntologiesServer::new_with_graph(db, graph.clone());
    (tmp, server, graph)
}

#[tokio::test]
async fn onto_apply_sees_the_plan_onto_plan_computed() {
    let (_tmp, server, _graph) = server();

    let plan: serde_json::Value = serde_json::from_str(
        &server
            .onto_plan(Parameters(OntoPlanInput {
                new_turtle: PROPOSED.to_string(),
            }))
            .await,
    )
    .unwrap();
    assert_eq!(plan["added_classes"].as_array().unwrap().len(), 1);

    let applied: serde_json::Value = serde_json::from_str(
        &server
            .onto_apply(Parameters(OntoApplyInput {
                mode: Some("safe".to_string()),
                plan_id: None,
            }))
            .await,
    )
    .unwrap();
    assert!(
        applied["error"].is_null(),
        "onto_apply could not see onto_plan's plan: {applied}"
    );
    assert_eq!(applied["ok"], true);
}

#[tokio::test]
async fn onto_apply_accepts_an_explicit_plan_id() {
    let (_tmp, server, _graph) = server();

    let plan: serde_json::Value = serde_json::from_str(
        &server
            .onto_plan(Parameters(OntoPlanInput {
                new_turtle: PROPOSED.to_string(),
            }))
            .await,
    )
    .unwrap();
    let plan_id = plan["plan_id"].as_str().unwrap().to_string();

    let applied: serde_json::Value = serde_json::from_str(
        &server
            .onto_apply(Parameters(OntoApplyInput {
                mode: Some("safe".to_string()),
                plan_id: Some(plan_id.clone()),
            }))
            .await,
    )
    .unwrap();
    assert_eq!(applied["plan_id"].as_str().unwrap(), plan_id);
}

#[tokio::test]
async fn onto_apply_without_a_plan_reports_the_error() {
    let (_tmp, server, _graph) = server();
    let applied: serde_json::Value = serde_json::from_str(
        &server
            .onto_apply(Parameters(OntoApplyInput {
                mode: Some("safe".to_string()),
                plan_id: None,
            }))
            .await,
    )
    .unwrap();
    assert!(
        applied["error"].as_str().unwrap_or_default().contains("No plan found"),
        "expected a no-plan error: {applied}"
    );
}
