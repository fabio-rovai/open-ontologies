//! Tool exposure filter for the MCP server.
//!
//! This lets operators restrict which `onto_*` tools are advertised over MCP
//! (and which can actually be invoked) via configuration or CLI flags.
//!
//! Three modes:
//!  - `Mode::All`   — all registered tools exposed (default).
//!  - `Mode::Allow` — only tools in `list` (or expanded from `groups`) exposed.
//!  - `Mode::Deny`  — all tools except those in `list` (or `groups`) exposed.
//!
//! Implementation: applied by removing routes from the rmcp `ToolRouter`
//! before the server is constructed. Removed tools are not advertised via
//! `tools/list` and cannot be invoked via `tools/call`.

use serde::Deserialize;
use std::collections::HashSet;

use rmcp::handler::server::tool::ToolRouter;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    All,
    Allow,
    Deny,
}

impl Mode {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "all" | "" => Ok(Mode::All),
            "allow" | "allowlist" | "whitelist" => Ok(Mode::Allow),
            "deny" | "denylist" | "blacklist" => Ok(Mode::Deny),
            other => Err(format!("unknown tool filter mode: {}", other)),
        }
    }
}

/// User-facing filter spec.
#[derive(Debug, Clone, Default)]
pub struct ToolFilter {
    pub mode: Mode,
    /// Explicit tool names.
    pub list: Vec<String>,
    /// Group names that expand to a curated set of tool names.
    pub groups: Vec<String>,
}

impl ToolFilter {
    pub fn all() -> Self {
        Self::default()
    }

    pub fn allow_only(names: impl IntoIterator<Item = String>) -> Self {
        Self {
            mode: Mode::Allow,
            list: names.into_iter().collect(),
            groups: vec![],
        }
    }

    pub fn deny(names: impl IntoIterator<Item = String>) -> Self {
        Self {
            mode: Mode::Deny,
            list: names.into_iter().collect(),
            groups: vec![],
        }
    }

    /// Resolve the effective set of explicit names (list ∪ expand(groups)).
    fn resolved_names(&self) -> HashSet<String> {
        let mut out: HashSet<String> = self.list.iter().cloned().collect();
        for g in &self.groups {
            for n in expand_group(g) {
                out.insert(n.to_string());
            }
        }
        out
    }

    /// Decide whether `tool_name` should be exposed.
    pub fn allows(&self, tool_name: &str) -> bool {
        let names = self.resolved_names();
        match self.mode {
            Mode::All => true,
            Mode::Allow => names.contains(tool_name),
            Mode::Deny => !names.contains(tool_name),
        }
    }

    /// Apply the filter to a `ToolRouter` by removing disallowed routes.
    /// Returns the list of removed tool names (for logging/inspection).
    ///
    /// This is the OPERATOR's filter. [`remove_unavailable`] runs first and is
    /// not optional: a tool the build cannot serve is never advertised,
    /// whatever the operator asked for.
    pub fn apply<S>(&self, router: &mut ToolRouter<S>) -> Vec<String>
    where
        S: Send + Sync + 'static,
    {
        if self.mode == Mode::All {
            return Vec::new();
        }
        let names = self.resolved_names();
        let all: Vec<String> = router
            .list_all()
            .into_iter()
            .map(|t| t.name.to_string())
            .collect();
        let mut removed = Vec::new();
        for name in all {
            let keep = match self.mode {
                Mode::All => true,
                Mode::Allow => names.contains(&name),
                Mode::Deny => !names.contains(&name),
            };
            if !keep {
                router.remove_route(&name);
                removed.push(name);
            }
        }
        removed
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Tools this build cannot serve
// ───────────────────────────────────────────────────────────────────────────

/// The tools whose implementation is behind a Cargo feature, with the feature
/// each one needs.
///
/// Eight of the registered tools have a body that is `#[cfg(not(feature =
/// ...))] { return "Compiled without X feature" }`. Advertising one of those
/// over `tools/list` is a promise the build cannot keep: a client reads the
/// description, calls the tool, and gets an error that has nothing to do with
/// its input. That is the same defect as a verdict claimed but not earned, one
/// layer up — the advertisement and the capability have to agree, and the only
/// honest way to make them agree is to stop advertising what cannot be served.
///
/// `onto_import_schema` and `onto_sql_ingest` need EITHER `postgres` or
/// `duckdb`: they dispatch on the connection URL and the two schemes they
/// accept are the two features. A build with one of them keeps both tools,
/// because one scheme still works and the other reports a clear error about
/// the scheme rather than about the tool.
pub const FEATURE_GATED_TOOLS: &[(&str, &str)] = &[
    ("onto_plugin_list", "plugins"),
    ("onto_plugin_call", "plugins"),
    ("onto_embed", "embeddings"),
    ("onto_hnsw_build", "embeddings"),
    ("onto_search", "embeddings"),
    ("onto_similarity", "embeddings"),
    ("onto_import_schema", "postgres or duckdb"),
    ("onto_sql_ingest", "postgres or duckdb"),
];

/// Of those, the ones THIS build cannot serve. Empty when every feature is on.
pub fn unavailable_in_this_build() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    if !cfg!(feature = "plugins") {
        out.extend(["onto_plugin_list", "onto_plugin_call"]);
    }
    if !cfg!(feature = "embeddings") {
        out.extend(["onto_embed", "onto_hnsw_build", "onto_search", "onto_similarity"]);
    }
    if !cfg!(feature = "postgres") && !cfg!(feature = "duckdb") {
        out.extend(["onto_import_schema", "onto_sql_ingest"]);
    }
    out.sort_unstable();
    out
}

/// Remove every tool this build cannot serve from `router`, returning what was
/// removed with the feature that would bring each one back.
///
/// Called before the operator's own filter and independently of its mode, so a
/// `Mode::All` server still does not advertise a tool guaranteed to fail.
pub fn remove_unavailable<S>(router: &mut ToolRouter<S>) -> Vec<(&'static str, &'static str)>
where
    S: Send + Sync + 'static,
{
    let gone = unavailable_in_this_build();
    let mut removed = Vec::new();
    for (name, feature) in FEATURE_GATED_TOOLS {
        if gone.contains(name) {
            router.remove_route(name);
            removed.push((*name, *feature));
        }
    }
    removed
}

/// Curated tool groups. Tool names must match the ones registered with
/// `#[tool(name = "...")]` in `src/server.rs`.
pub fn expand_group(name: &str) -> &'static [&'static str] {
    match name {
        // Read-only inspection tools (safe to expose to untrusted callers).
        "read_only" | "read" => &[
            "onto_status",
            "onto_validate",
            "onto_query",
            "onto_stats",
            "onto_diff",
            "onto_lint",
            "onto_history",
            "onto_lineage",
            "onto_cache_status",
            "onto_cache_list",
            "onto_repo_list",
            "onto_dl_check",
            "onto_dl_explain",
            "onto_search",
            "onto_similarity",
        ],
        // Tools that mutate the in-memory store but not external systems.
        "mutating" | "write" => &[
            "onto_load",
            "onto_clear",
            "onto_save",
            "onto_convert",
            "onto_pull",
            "onto_import",
            "onto_marketplace",
            "onto_version",
            "onto_rollback",
            "onto_ingest",
            "onto_sql_ingest",
            "onto_map",
            "onto_shacl",
            "onto_vocab_check",
            "onto_reason",
            "onto_extend",
            "onto_unload",
            "onto_recompile",
            "onto_cache_remove",
            "onto_repo_load",
        ],
        // Tools that change governance / lifecycle state.
        "governance" => &[
            "onto_plan",
            "onto_apply",
            "onto_lock",
            "onto_drift",
            "onto_enforce",
            "onto_monitor",
            "onto_monitor_clear",
            "onto_align",
            "onto_align_feedback",
            "onto_lint_feedback",
            "onto_enforce_feedback",
        ],
        // Tools that talk to external systems.
        "remote" => &[
            "onto_pull",
            "onto_push",
            "onto_marketplace",
            "onto_import",
        ],
        // Embedding / semantic search tools.
        "embeddings" => &[
            "onto_embed",
            "onto_search",
            "onto_similarity",
        ],
        // SQL data backbone tools (PostgreSQL / DuckDB).
        "sql" => &[
            "onto_import_schema",
            "onto_sql_ingest",
        ],
        _ => &[],
    }
}

/// Parse a comma-separated list of tool/group identifiers into (names, groups).
/// Identifiers prefixed with `@` are treated as group names.
pub fn parse_csv(spec: &str) -> (Vec<String>, Vec<String>) {
    let mut names = Vec::new();
    let mut groups = Vec::new();
    for raw in spec.split(',') {
        let item = raw.trim();
        if item.is_empty() {
            continue;
        }
        if let Some(g) = item.strip_prefix('@') {
            groups.push(g.to_string());
        } else {
            names.push(item.to_string());
        }
    }
    (names, groups)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_parse() {
        assert_eq!(Mode::parse("all").unwrap(), Mode::All);
        assert_eq!(Mode::parse("ALLOW").unwrap(), Mode::Allow);
        assert_eq!(Mode::parse("deny").unwrap(), Mode::Deny);
        assert!(Mode::parse("nope").is_err());
    }

    #[test]
    fn allow_filter_only_lets_listed_tools_through() {
        let f = ToolFilter::allow_only(vec!["onto_status".to_string(), "onto_query".to_string()]);
        assert!(f.allows("onto_status"));
        assert!(f.allows("onto_query"));
        assert!(!f.allows("onto_load"));
    }

    #[test]
    fn deny_filter_blocks_listed_tools() {
        let f = ToolFilter::deny(vec!["onto_clear".to_string()]);
        assert!(f.allows("onto_status"));
        assert!(!f.allows("onto_clear"));
    }

    #[test]
    fn group_expansion() {
        let f = ToolFilter {
            mode: Mode::Allow,
            list: vec![],
            groups: vec!["read_only".to_string()],
        };
        assert!(f.allows("onto_status"));
        assert!(f.allows("onto_query"));
        assert!(!f.allows("onto_load"));
    }

    #[test]
    fn csv_parser_splits_names_and_groups() {
        let (n, g) = parse_csv("onto_status, onto_query, @read_only,  ");
        assert_eq!(n, vec!["onto_status", "onto_query"]);
        assert_eq!(g, vec!["read_only"]);
    }

    #[test]
    fn unknown_group_expands_to_empty() {
        assert!(expand_group("does-not-exist").is_empty());
    }
}
