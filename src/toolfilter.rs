//! Tool exposure filter for the MCP server.
//!
//! This lets operators restrict which `onto_*` tools are advertised over MCP
//! (and which can actually be invoked) via configuration or CLI flags.
//!
//! TWO AXES, AND THEY COMPOSE.
//!
//! The first axis is the PROFILE: a named subset of the tool surface sized for
//! one job. A client that is aligning two vocabularies has no use for most of
//! what `src/server.rs` registers, and every tool it has no use for is one more
//! chance to call the wrong one. `expand_profile` is the table and `profiles()`
//! lists the names. No count is written here on purpose: the total is measured
//! from the router by `tests/toolfilter_profiles_test.rs` and stated in
//! `docs/tool-reference.md`, which is the one document allowed to state it.
//!
//! The second axis is the MODE, which is the older mechanism and is unchanged:
//!  - `Mode::All`   — every tool the profile left standing (default).
//!  - `Mode::Allow` — only tools in `list` (or expanded from `groups`).
//!  - `Mode::Deny`  — all but the tools in `list` (or `groups`).
//!
//! The profile narrows FIRST and the mode narrows what is left, so
//! `--tool-profile alignment --tools-deny onto_push` is the alignment profile
//! minus one tool and neither axis can widen the other. The `groups` on the
//! mode axis (`read_only`, `mutating`, …) cut along a different line: they
//! answer what a caller may be trusted to do, not what it is here to do.
//!
//! THE DEFAULT DOES NOT MOVE. No profile means `full`, which is every tool this
//! build can serve, which is what every existing client already sees.
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
    /// Task profile. `None` and `Some("full")` both mean every tool, which is
    /// the default and what every client saw before profiles existed.
    pub profile: Option<String>,
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
            profile: None,
        }
    }

    pub fn deny(names: impl IntoIterator<Item = String>) -> Self {
        Self {
            mode: Mode::Deny,
            list: names.into_iter().collect(),
            groups: vec![],
            profile: None,
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

    /// A filter that exposes exactly one profile. Errors on an unknown name
    /// rather than falling back to `full`.
    pub fn with_profile(name: &str) -> Result<Self, String> {
        if !is_profile(name) {
            return Err(format!(
                "unknown tool profile {:?}; known: {}",
                name,
                profiles().join(", ")
            ));
        }
        Ok(Self {
            profile: Some(name.to_string()),
            ..Default::default()
        })
    }

    /// The profile in force, `full` when none was chosen.
    pub fn profile_name(&self) -> &str {
        self.profile.as_deref().unwrap_or(FULL)
    }

    /// Whether the PROFILE axis alone lets `tool_name` through.
    fn profile_allows(&self, tool_name: &str) -> bool {
        match self.profile.as_deref() {
            None | Some(FULL) => true,
            Some(p) => match expand_profile(p) {
                Some(names) => names.contains(&tool_name),
                // Unreachable through the CLI, the env var and the config file,
                // all three of which refuse an unknown name at startup. Reached
                // only by building the struct by hand, and the conservative
                // reading of a name nobody recognises is that it names nothing.
                None => false,
            },
        }
    }

    /// What a client is looking at, for `onto_status` and the startup log.
    ///
    /// `servable` is counted AFTER `remove_unavailable` and `compiled_in`
    /// before it, because in a default build eight of the registered tools are
    /// behind a Cargo feature and are already gone by the time the operator's
    /// filter runs. Folding the two into one number would report the feature
    /// gate as something the profile withheld, and the field a user reads to
    /// answer "why can I not see it" would name the wrong cause.
    pub fn describe(&self, compiled_in: usize, servable: usize, exposed: usize) -> ToolSurface {
        ToolSurface {
            profile: self.profile_name().to_string(),
            mode: match self.mode {
                Mode::All => "all",
                Mode::Allow => "allow",
                Mode::Deny => "deny",
            },
            compiled_in,
            servable_in_this_build: servable,
            exposed,
            withheld_by_filter: servable.saturating_sub(exposed),
        }
    }

    /// Decide whether `tool_name` should be exposed.
    ///
    /// The profile narrows first and the mode narrows what is left, so naming a
    /// tool in `--tools-allow` that the profile does not carry does not bring it
    /// back.
    pub fn allows(&self, tool_name: &str) -> bool {
        self.allows_given(&self.resolved_names(), tool_name)
    }

    /// `allows` with the resolved name set supplied, so `apply` builds it once
    /// for the whole router instead of once per tool. The DECISION lives here
    /// and nowhere else: `apply` used to carry its own copy of the match, which
    /// is two places for one rule to be changed in.
    fn allows_given(&self, names: &HashSet<String>, tool_name: &str) -> bool {
        if !self.profile_allows(tool_name) {
            return false;
        }
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
        // The fast path is the DEFAULT path and it has to stay exact: no
        // profile and `Mode::All` must remove nothing at all.
        if self.mode == Mode::All && matches!(self.profile.as_deref(), None | Some(FULL)) {
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
            if !self.allows_given(&names, &name) {
                router.remove_route(&name);
                removed.push(name);
            }
        }
        removed
    }
}

/// The filtered surface as reported to clients.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolSurface {
    /// `full` when nothing was selected.
    pub profile: String,
    /// The second axis: `all`, `allow` or `deny`.
    pub mode: &'static str,
    /// Tools the binary registers, whatever this build can serve.
    pub compiled_in: usize,
    /// Of those, the ones this feature set can actually serve.
    pub servable_in_this_build: usize,
    /// Of those, the ones that survived the operator's filter.
    pub exposed: usize,
    /// The filter's own doing. Non-zero here is the answer to "why can I not
    /// see it", and `compiled_in - servable_in_this_build` is the other answer.
    pub withheld_by_filter: usize,
}

/// The name that means "everything".
pub const FULL: &str = "full";

/// Tools present in EVERY profile.
///
/// Not a category, a floor. Nothing else in any profile can do anything until a
/// graph is loaded, and a client that cannot check what it loaded or clear a
/// bad load is stuck inside its own profile. Six tools is the price of every
/// profile being self-sufficient, and a profile that is not self-sufficient
/// sends the user back to `full`, which defeats the exercise.
pub const SPINE: &[&str] = &[
    "onto_status",
    "onto_validate",
    "onto_load",
    "onto_query",
    "onto_stats",
    "onto_clear",
];

/// The task-oriented profiles, each the tools for one job, on top of `SPINE`.
///
/// Read off what the tools do in `src/server.rs`, which is why they OVERLAP:
/// `onto_shacl` belongs to judging data and to landing it, `onto_module_extract`
/// to authoring, reasoning and retrieval, and forcing a partition would have
/// left one of those jobs a tool short. The only rule the set has to satisfy is
/// COVERAGE, enforced by
/// `tests/toolfilter_profiles_test.rs::every_registered_tool_is_in_some_profile`:
/// a tool added later and filed under nothing fails the build rather than
/// becoming reachable only from `full` and therefore never missed.
const PROFILES: &[(&str, &str, &[&str])] = &[
    (
        "authoring",
        "Write and edit an ontology: validate, lint, enforce patterns, version, \
         extract modules, and pull in vocabularies to build on.",
        &[
            "onto_save", "onto_convert", "onto_diff", "onto_lint", "onto_lint_feedback",
            "onto_defects", "onto_dl_check", "onto_dl_explain", "onto_version", "onto_history",
            "onto_rollback", "onto_marketplace", "onto_import", "onto_pull", "onto_push",
            "onto_repo_list", "onto_repo_load", "onto_unload", "onto_recompile",
            "onto_cache_status", "onto_cache_list", "onto_cache_remove", "onto_enforce",
            "onto_enforce_feedback", "onto_ossie_import", "onto_import_schema",
            "onto_plugin_list", "onto_plugin_call", "onto_induce", "onto_module_extract",
            "onto_conservative_check",
        ],
    ),
    (
        "validation",
        "Judge data against an ontology: SHACL shapes, closed-world vocabulary \
         checks, shape induction, what the rule table can see, and the \
         provenance of what the graph claims.",
        &[
            "onto_shacl", "onto_shacl_check", "onto_vocab_check", "onto_lint", "onto_defects",
            "onto_dlp_boundary", "onto_justify", "onto_coevolve_dependency_graph",
            "onto_owl_shacl_coevolve_check", "onto_owl_shacl_coevolve_incremental",
            "onto_shape_induce", "onto_shape_combinatorics", "onto_ingest", "onto_map",
            "onto_extend", "onto_support_check", "onto_support_verdict", "onto_support_report",
            "onto_plugin_list", "onto_plugin_call",
        ],
    ),
    (
        "reasoning",
        "Derive consequences and produce the artefacts that can be checked: \
         certificates, justifications, first-order export and proof checking, \
         DL tableaux, model finding.",
        &[
            "onto_reason", "onto_reason_incremental", "onto_rules_import", "onto_defects",
            "onto_dlp_boundary", "onto_trace_label", "onto_justify", "onto_provenance",
            "onto_classify_el",
            "onto_dl_check", "onto_dl_explain", "onto_fol_export", "onto_fol_model",
            "onto_fol_prove", "onto_closure_diff", "onto_module_extract",
            "graph_projection_entailment_check", "graph_projection_lossy_check", "onto_extend",
        ],
    ),
    (
        "alignment",
        "Relate two vocabularies: alignment candidates and their feedback loop, \
         clinical crosswalks, and the embeddings the scorer uses.",
        &[
            "onto_align", "onto_align_feedback", "onto_align_fuzzy", "onto_align_flora",
            "onto_eval_alignment", "onto_diff", "onto_crosswalk", "onto_enrich",
            "onto_validate_clinical", "onto_embed", "onto_search", "onto_similarity",
            "onto_hnsw_build", "borderline_partition", "borderline_record_verdict",
        ],
    ),
    (
        "governance",
        "Change an ontology in production and answer for it afterwards: plan, \
         apply, lock, drift, monitor, policy, packs, conservativity and the \
         lineage trail.",
        &[
            "onto_plan", "onto_apply", "onto_lock", "onto_drift", "onto_enforce",
            "onto_enforce_feedback", "onto_monitor", "onto_monitor_clear", "onto_lineage",
            "onto_provenance", "onto_conservative_check", "onto_version", "onto_history",
            "onto_rollback", "onto_pack", "onto_unpack", "onto_policy_register",
            "onto_policy_list", "onto_policy_check", "onto_support_report",
            "onto_temporal_snapshot", "onto_temporal_query", "onto_temporal_conflicts",
        ],
    ),
    (
        "data",
        "Land external data in the graph and keep it there: file and SQL \
         ingest, schema induction, mapping, CDC watermarks, and the two \
         temporal clocks.",
        &[
            "onto_ingest", "onto_induce", "onto_map", "onto_extend", "onto_convert",
            "onto_sql_ingest", "onto_import_schema", "onto_sql_sync_state",
            "onto_sql_sync_reset", "onto_sql_sync_states_list", "onto_pull", "onto_push",
            "onto_shacl", "onto_vocab_check", "onto_temporal_snapshot", "onto_temporal_query",
            "onto_temporal_conflicts",
        ],
    ),
    (
        "planning",
        "Act on the graph under laws: registered actions, BC+ invariants and \
         defaults, PDDL compilation and plan validation.",
        &[
            "onto_action_register", "onto_action_list", "onto_action_applicable",
            "onto_action_apply", "onto_action_apply_concurrent", "onto_certify_action",
            "onto_invariant_register", "onto_invariant_list", "onto_invariant_remove",
            "onto_invariant_check", "onto_default_register", "onto_default_apply",
            "onto_plan_classical", "onto_plan_compile_pddl", "onto_plan_validate",
            "onto_policy_check", "onto_lineage",
        ],
    ),
    (
        "retrieval",
        "Ground a model on the graph: neighbourhood slices, modules, community \
         skeletons, schema-guided extraction, and whether a slice kept what the \
         answer rests on.",
        &[
            "onto_segment_retrieve", "onto_communities", "onto_extract_scaffold",
            "onto_extract_validate", "onto_module_extract", "onto_embed", "onto_search",
            "onto_similarity", "onto_hnsw_build", "graph_projection_lossy_check",
            "graph_projection_entailment_check", "onto_closure_diff",
        ],
    ),
    (
        "evaluation",
        "Score the thing rather than run it: competency questions, mmRAG and \
         OAEI metrics, and the borderline-verdict loop.",
        &[
            "onto_cq_run", "onto_verify_cq", "onto_cq_verdicts_list", "eval_rag",
            "eval_rag_mmrag", "onto_eval_alignment", "borderline_partition",
            "borderline_record_verdict", "onto_shape_induce", "onto_shape_combinatorics",
            "onto_diff",
        ],
    ),
];

/// Every profile name, `full` first. `full` is not in `PROFILES` because it is
/// not a subset: it is the absence of one. Giving it a list would mean a tool
/// added to `src/server.rs` had to be added to `full` by hand to stay visible,
/// which is the unreachability this whole exercise is against.
pub fn profiles() -> Vec<&'static str> {
    let mut out = vec![FULL];
    out.extend(PROFILES.iter().map(|(n, _, _)| *n));
    out
}

/// One line saying what a profile is for, or `None` if the name is unknown.
pub fn profile_description(name: &str) -> Option<&'static str> {
    if name == FULL {
        return Some(
            "Every tool this build can serve. The default, and what every \
             client saw before profiles existed.",
        );
    }
    PROFILES.iter().find(|(n, _, _)| *n == name).map(|(_, d, _)| *d)
}

/// The tools a profile exposes: `SPINE` plus its own, deduplicated and sorted.
///
/// `None` for an unknown name, and the distinction matters: a typo must be an
/// error at startup and must NOT fall back to exposing everything, which is the
/// one wrong answer a misspelled `--tool-profile` could give.
pub fn expand_profile(name: &str) -> Option<Vec<&'static str>> {
    if name == FULL {
        return None;
    }
    let (_, _, own) = PROFILES.iter().find(|(n, _, _)| *n == name)?;
    let mut out: Vec<&'static str> = SPINE.iter().copied().chain(own.iter().copied()).collect();
    out.sort_unstable();
    out.dedup();
    Some(out)
}

/// Whether `name` names a profile, `full` included.
pub fn is_profile(name: &str) -> bool {
    name == FULL || PROFILES.iter().any(|(n, _, _)| *n == name)
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
            "onto_induce",
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
            profile: None,
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
