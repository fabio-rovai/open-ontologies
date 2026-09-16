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
        hash_prefix_bytes: 64 * 1024,
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

// ────────────────────────────────────────────────────────────────────────────
// Tools this build cannot serve
// ────────────────────────────────────────────────────────────────────────────
//
// A tool in `tools/list` is a promise. Eight of the registered tools have a
// body that is `#[cfg(not(feature = ...))] { return "Compiled without X
// feature" }`, so on a build without that feature the promise cannot be kept
// for any input at all: the client reads a description about embeddings or
// WASM plugins or a Postgres URL, calls the tool, and gets an error that has
// nothing to do with what it asked. That is the advertisement and the
// capability disagreeing, and it is the same shape as a verdict claimed but
// not earned, one layer up.

/// The default filter exposes everything the build CAN serve, which is not the
/// same as everything that is registered.
#[test]
fn a_tool_this_build_cannot_serve_is_not_advertised() {
    let (_tmp, server) = build_server(ToolFilter::default());
    let names = tool_names(&server);
    for gone in open_ontologies::toolfilter::unavailable_in_this_build() {
        assert!(
            !names.contains(&gone.to_string()),
            "{gone} is advertised by a build that can only answer it with \"Compiled \
             without ... feature\""
        );
    }
    // And the ones it CAN serve are still there, so this is not a blanket
    // removal of the gated list.
    for (tool, _feature) in open_ontologies::toolfilter::FEATURE_GATED_TOOLS {
        let withheld = open_ontologies::toolfilter::unavailable_in_this_build().contains(tool);
        assert_eq!(
            names.contains(&tool.to_string()),
            !withheld,
            "{tool} advertised={} withheld={withheld}",
            names.contains(&tool.to_string())
        );
    }
}

/// The count, derived on both sides. `#[tool(name = ...)]` attributes in
/// `src/server.rs` are the registration, the router is the advertisement, and
/// the difference is exactly what this build cannot serve.
#[test]
fn the_advertised_count_is_the_registered_count_minus_what_cannot_be_served() {
    let registered = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/server.rs"),
    )
    .unwrap()
    .matches("#[tool(name = ")
    .count();
    let (_tmp, server) = build_server(ToolFilter::default());
    assert_eq!(
        tool_names(&server).len(),
        registered - open_ontologies::toolfilter::unavailable_in_this_build().len()
    );
}

/// The instructions string is what an MCP client reads before it calls
/// anything, and it used to state two hand-typed totals, 114 and 112, neither
/// of which was the number the router advertised. It is formatted from the
/// router now, so this checks the live server rather than the source text.
#[test]
fn the_instructions_string_states_the_count_it_advertises() {
    use rmcp::ServerHandler;
    let (_tmp, server) = build_server(ToolFilter::default());
    let advertised = tool_names(&server).len();
    let info = server.get_info();
    let text = info.instructions.expect("the server carries instructions");
    assert!(
        text.contains(&format!("MCP server with {advertised} tools")),
        "the instructions must state the number the router advertises: {text}"
    );
    for gone in open_ontologies::toolfilter::unavailable_in_this_build() {
        assert!(
            text.contains(gone),
            "a tool withheld from tools/list must be NAMED in the instructions, with the \
             feature that brings it back, or its absence is silent: {text}"
        );
    }
}
