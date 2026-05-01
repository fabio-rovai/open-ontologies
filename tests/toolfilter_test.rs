//! Integration tests for the MCP tool exposure filter (`src/toolfilter.rs`)
//! and its application to `OpenOntologiesServer`.
//!
//! Covers feature 5: limiting which `onto_*` tools the MCP server advertises.

use std::sync::Arc;

use open_ontologies::config::{CacheConfig, EmbeddingsConfig};
use open_ontologies::graph::GraphStore;
use open_ontologies::server::OpenOntologiesServer;
use open_ontologies::state::StateDb;
use open_ontologies::toolfilter::{Mode, ToolFilter, parse_csv};

fn fresh_db() -> (tempfile::TempDir, StateDb) {
    let tmp = tempfile::tempdir().unwrap();
    let db = StateDb::open(&tmp.path().join("s.db")).unwrap();
    (tmp, db)
}

fn build_server(filter: ToolFilter) -> (tempfile::TempDir, OpenOntologiesServer) {
    let (tmp, db) = fresh_db();
    let graph = Arc::new(GraphStore::new());
    let cache = CacheConfig {
        enabled: true,
        dir: tmp.path().join("cache").to_string_lossy().into_owned(),
        idle_ttl_secs: 0,
        evictor_interval_secs: 30,
        auto_refresh: false,
    };
    let server = OpenOntologiesServer::new_with_registry_options(
        db,
        graph,
        None,
        EmbeddingsConfig::default(),
        cache,
        filter,
    );
    (tmp, server)
}

fn tool_names(server: &OpenOntologiesServer) -> Vec<String> {
    server
        .list_tool_definitions()
        .into_iter()
        .map(|t| t.name.to_string())
        .collect()
}

// ────────────────────────────────────────────────────────────────────────────
// Default — all tools exposed
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn default_filter_exposes_everything() {
    let (_tmp, server) = build_server(ToolFilter::default());
    let names = tool_names(&server);
    // Sanity: a handful of well-known tools are present.
    for must in &[
        "onto_status",
        "onto_load",
        "onto_query",
        "onto_save",
        "onto_clear",
        "onto_unload",
        "onto_recompile",
        "onto_cache_status",
    ] {
        assert!(names.contains(&must.to_string()), "missing {}", must);
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Allowlist
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn allow_filter_exposes_only_listed_tools() {
    let filter = ToolFilter::allow_only(vec![
        "onto_status".to_string(),
        "onto_query".to_string(),
        "onto_stats".to_string(),
    ]);
    let (_tmp, server) = build_server(filter);
    let names = tool_names(&server);
    assert_eq!(names.len(), 3);
    assert!(names.contains(&"onto_status".to_string()));
    assert!(names.contains(&"onto_query".to_string()));
    assert!(names.contains(&"onto_stats".to_string()));
    assert!(!names.contains(&"onto_load".to_string()));
    assert!(!names.contains(&"onto_clear".to_string()));
}

#[test]
fn allow_filter_with_unknown_name_yields_empty_set() {
    let filter = ToolFilter::allow_only(vec!["nope_no_such_tool".to_string()]);
    let (_tmp, server) = build_server(filter);
    assert_eq!(tool_names(&server).len(), 0);
}

// ────────────────────────────────────────────────────────────────────────────
// Denylist
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn deny_filter_blocks_listed_tools() {
    let filter = ToolFilter::deny(vec![
        "onto_clear".to_string(),
        "onto_load".to_string(),
    ]);
    let (_tmp, server) = build_server(filter);
    let names = tool_names(&server);
    assert!(!names.contains(&"onto_clear".to_string()));
    assert!(!names.contains(&"onto_load".to_string()));
    // Other tools are still exposed.
    assert!(names.contains(&"onto_status".to_string()));
    assert!(names.contains(&"onto_query".to_string()));
}

// ────────────────────────────────────────────────────────────────────────────
// Group expansion
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn allow_filter_with_read_only_group() {
    let filter = ToolFilter {
        mode: Mode::Allow,
        list: vec![],
        groups: vec!["read_only".to_string()],
    };
    let (_tmp, server) = build_server(filter);
    let names = tool_names(&server);
    // read_only group must include status & query.
    assert!(names.contains(&"onto_status".to_string()));
    assert!(names.contains(&"onto_query".to_string()));
    // and must exclude write tools.
    assert!(!names.contains(&"onto_load".to_string()));
    assert!(!names.contains(&"onto_clear".to_string()));
}

#[test]
fn deny_filter_with_governance_group() {
    let filter = ToolFilter {
        mode: Mode::Deny,
        list: vec![],
        groups: vec!["governance".to_string()],
    };
    let (_tmp, server) = build_server(filter);
    let names = tool_names(&server);
    assert!(!names.contains(&"onto_apply".to_string()));
    assert!(!names.contains(&"onto_plan".to_string()));
    assert!(names.contains(&"onto_status".to_string()));
}

// ────────────────────────────────────────────────────────────────────────────
// CSV parsing
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn parse_csv_handles_groups_and_names() {
    let (n, g) = parse_csv("onto_status,@read_only,onto_query");
    assert_eq!(n, vec!["onto_status", "onto_query"]);
    assert_eq!(g, vec!["read_only"]);
}

#[test]
fn parse_csv_trims_whitespace_and_skips_empty() {
    let (n, g) = parse_csv("  onto_status , , @read_only ,, onto_query ");
    assert_eq!(n, vec!["onto_status", "onto_query"]);
    assert_eq!(g, vec!["read_only"]);
}

// ────────────────────────────────────────────────────────────────────────────
// Mode parsing
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn mode_parse_is_case_insensitive_and_supports_aliases() {
    assert_eq!(Mode::parse("ALL").unwrap(), Mode::All);
    assert_eq!(Mode::parse("allowlist").unwrap(), Mode::Allow);
    assert_eq!(Mode::parse("denylist").unwrap(), Mode::Deny);
    assert!(Mode::parse("garbage").is_err());
}
