use std::sync::Arc;

use rmcp::{
    ServerHandler, RoleServer, tool, tool_handler, tool_router,
    prompt, prompt_handler, prompt_router,
    handler::server::{tool::ToolRouter, router::prompt::PromptRouter, wrapper::Parameters},
    model::{
        ServerCapabilities, ServerInfo, Tool,
        PromptMessage, PromptMessageRole, GetPromptResult,
        GetPromptRequestParams, PaginatedRequestParams, ListPromptsResult,
    },
    service::RequestContext,
};
use crate::config::expand_tilde;
use crate::graph::GraphStore;
use crate::inputs::*;
use crate::state::StateDb;

// ─── OpenOntologiesServer ───────────────────────────────────────────────────

/// MCP server that exposes all Open Ontologies tools to Claude via stdin/stdout.
#[derive(Clone)]
pub struct OpenOntologiesServer {
    tool_router: ToolRouter<Self>,
    prompt_router: PromptRouter<Self>,
    db: StateDb,
    graph: Arc<GraphStore>,
    session_id: String,
    governance_webhook: Option<String>,
    /// Registry tracking the active ontology + compile cache + TTL eviction.
    registry: Arc<crate::registry::OntologyRegistry>,
    /// Configured ontology repository directories, expanded and deduplicated.
    /// Empty when none are configured. Used by `onto_repo_list` /
    /// `onto_repo_load`.
    ontology_dirs: Arc<Vec<std::path::PathBuf>>,
    #[cfg(feature = "embeddings")]
    vecstore: Arc<std::sync::Mutex<crate::vecstore::VecStore>>,
    #[cfg(feature = "embeddings")]
    text_embedder: Option<Arc<crate::embed::TextEmbedderProvider>>,
}

impl OpenOntologiesServer {
    /// Create a new server with all tools wired to domain services.
    pub fn new(db: StateDb) -> Self {
        Self::new_with_options(db, Arc::new(GraphStore::new()), None)
    }

    /// Create a new server sharing an existing graph store (for HTTP mode where
    /// all sessions must see the same in-memory triples).
    pub fn new_with_graph(db: StateDb, graph: Arc<GraphStore>) -> Self {
        Self::new_with_options(db, graph, None)
    }

    /// Create a new server with all options including optional governance webhook.
    pub fn new_with_options(db: StateDb, graph: Arc<GraphStore>, governance_webhook: Option<String>) -> Self {
        Self::new_with_full_options(db, graph, governance_webhook, Default::default())
    }

    /// Create a new server with all options including embedding config.
    pub fn new_with_full_options(
        db: StateDb,
        graph: Arc<GraphStore>,
        governance_webhook: Option<String>,
        _embed_config: crate::config::EmbeddingsConfig,
    ) -> Self {
        Self::new_with_registry_options(
            db,
            graph,
            governance_webhook,
            _embed_config,
            crate::config::CacheConfig::default(),
            crate::toolfilter::ToolFilter::default(),
        )
    }

    /// Full constructor, including cache configuration and tool filter.
    pub fn new_with_registry_options(
        db: StateDb,
        graph: Arc<GraphStore>,
        governance_webhook: Option<String>,
        _embed_config: crate::config::EmbeddingsConfig,
        cache_config: crate::config::CacheConfig,
        tool_filter: crate::toolfilter::ToolFilter,
    ) -> Self {
        Self::new_with_repo_options(
            db,
            graph,
            governance_webhook,
            _embed_config,
            cache_config,
            tool_filter,
            Vec::new(),
        )
    }

    /// Full constructor with on-disk ontology repo directories.
    ///
    /// `ontology_dirs` lists host directories that the `onto_repo_list` and
    /// `onto_repo_load` tools enumerate. They are stored verbatim (already
    /// resolved by the caller through `crate::config::resolve_ontology_dirs`).
    pub fn new_with_repo_options(
        db: StateDb,
        graph: Arc<GraphStore>,
        governance_webhook: Option<String>,
        _embed_config: crate::config::EmbeddingsConfig,
        cache_config: crate::config::CacheConfig,
        tool_filter: crate::toolfilter::ToolFilter,
        ontology_dirs: Vec<std::path::PathBuf>,
    ) -> Self {
        let lineage = crate::lineage::LineageLog::with_governance_webhook(db.clone(), governance_webhook.clone());
        let session_id = lineage.new_session();

        // Build the registry. If construction fails (e.g. cache dir cannot be
        // created) fall back to a disabled registry so the server still starts.
        let registry = match crate::registry::OntologyRegistry::new(
            graph.clone(),
            db.clone(),
            cache_config.clone(),
        ) {
            Ok(r) => Arc::new(r),
            Err(e) => {
                tracing::warn!("ontology registry init failed: {}; cache disabled", e);
                let mut disabled = cache_config.clone();
                disabled.enabled = false;
                disabled.dir = std::env::temp_dir().to_string_lossy().to_string();
                Arc::new(
                    crate::registry::OntologyRegistry::new(graph.clone(), db.clone(), disabled)
                        .expect("temp_dir registry"),
                )
            }
        };

        // Apply tool filter by removing routes from the router.
        let mut tool_router = Self::tool_router();
        let removed = tool_filter.apply(&mut tool_router);
        if !removed.is_empty() {
            tracing::info!("tool filter removed {} tools: {:?}", removed.len(), removed);
        }

        #[cfg(feature = "embeddings")]
        let (vecstore, text_embedder) = {
            let mut vs = crate::vecstore::VecStore::new(db.clone());
            let _ = vs.load_from_db();

            let embedder = match crate::embed::TextEmbedderProvider::from_config(&_embed_config) {
                Ok(Some(e)) => {
                    tracing::info!(
                        "embeddings enabled (provider = {})",
                        e.provider_name()
                    );
                    Some(Arc::new(e))
                }
                Ok(None) => {
                    tracing::info!(
                        "embeddings configured but no provider available (model files missing or provider disabled)"
                    );
                    None
                }
                Err(e) => {
                    tracing::warn!("failed to initialise embedding provider: {}", e);
                    None
                }
            };
            (Arc::new(std::sync::Mutex::new(vs)), embedder)
        };

        Self {
            tool_router,
            prompt_router: Self::prompt_router(),
            db,
            graph,
            session_id,
            governance_webhook,
            registry,
            ontology_dirs: Arc::new(ontology_dirs),
            #[cfg(feature = "embeddings")]
            vecstore,
            #[cfg(feature = "embeddings")]
            text_embedder,
        }
    }

    /// Return the list of all registered tool definitions.
    pub fn list_tool_definitions(&self) -> Vec<Tool> {
        self.tool_router.list_all()
    }

    /// Access the ontology registry (for tests and the HTTP server eviction loop).
    pub fn registry(&self) -> Arc<crate::registry::OntologyRegistry> {
        self.registry.clone()
    }

    fn lineage(&self) -> crate::lineage::LineageLog {
        crate::lineage::LineageLog::with_governance_webhook(self.db.clone(), self.governance_webhook.clone())
    }

    fn monitor(&self) -> crate::monitor::Monitor {
        crate::monitor::Monitor::new(self.db.clone(), self.graph.clone())
    }
}

// ─── Tool definitions ───────────────────────────────────────────────────────

#[tool_router]
impl OpenOntologiesServer {

    // ── Status ──────────────────────────────────────────────────────────────

    #[tool(name = "onto_status", description = "Returns health status of the Open Ontologies server")]
    fn onto_status(&self) -> String {
        let tool_count = self.tool_router.list_all().len();
        let triple_count = self.graph.triple_count();
        serde_json::json!({
            "status": "ok",
            "version": env!("CARGO_PKG_VERSION"),
            "tools": tool_count,
            "triples_loaded": triple_count,
        })
        .to_string()
    }

    // ── Ontology ────────────────────────────────────────────────────────────

    #[tool(name = "onto_validate", description = "Validate RDF/OWL syntax. Accepts a file path or inline Turtle content.")]
    async fn onto_validate(&self, Parameters(input): Parameters<OntoValidateInput>) -> String {
        use crate::ontology::OntologyService;
        if input.inline.unwrap_or(false) {
            OntologyService::validate_string(&input.input).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
        } else {
            OntologyService::validate_file(&input.input).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
        }
    }

    #[tool(name = "onto_convert", description = "Convert an RDF file between formats: turtle, ntriples, rdfxml, nquads, trig")]
    async fn onto_convert(&self, Parameters(input): Parameters<OntoConvertInput>) -> String {
        let store = GraphStore::new();
        match store.load_file(&input.path) {
            Ok(_) => {
                match store.serialize(&input.to) {
                    Ok(content) => {
                        if let Some(output) = input.output {
                            match std::fs::write(&output, &content) {
                                Ok(_) => format!(r#"{{"ok":true,"path":"{}","format":"{}"}}"#, output, input.to),
                                Err(e) => format!(r#"{{"error":"{}"}}"#, e),
                            }
                        } else {
                            content
                        }
                    }
                    Err(e) => format!(r#"{{"error":"{}"}}"#, e),
                }
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_load", description = "Load an RDF file or inline Turtle content into the in-memory ontology store. When given a file path, the parsed graph is also written to a fast N-Triples compile cache (in `[cache] dir`) so subsequent loads from the same source skip parsing. Optional `name`, `auto_refresh`, and `force_recompile` flags control caching/refresh behavior.")]
    async fn onto_load(&self, Parameters(input): Parameters<OntoLoadInput>) -> String {
        if let Some(turtle) = input.turtle {
            // Inline turtle bypasses the registry/cache (no source file).
            match self.graph.load_turtle(&turtle, None) {
                Ok(count) => format!(r#"{{"ok":true,"triples_loaded":{},"source":"inline"}}"#, count),
                Err(e) => format!(r#"{{"error":"{}"}}"#, e),
            }
        } else if let Some(path) = input.path {
            let path = expand_tilde(&path);
            let opts = crate::registry::LoadOptions {
                name: input.name,
                auto_refresh: input.auto_refresh.unwrap_or(false),
                force_recompile: input.force_recompile.unwrap_or(false),
            };
            match self.registry.load_file(&path, opts) {
                Ok(res) => serde_json::json!({
                    "ok": true,
                    "triples_loaded": res.triple_count,
                    "path": res.source_path,
                    "name": res.name,
                    "origin": res.origin,
                    "cache_path": res.cache_path,
                }).to_string(),
                Err(e) => format!(r#"{{"error":"{}"}}"#, e.to_string().replace('"', "'")),
            }
        } else {
            r#"{"error":"Either 'path' or 'turtle' must be provided"}"#.to_string()
        }
    }

    #[tool(name = "onto_repo_list", description = "List RDF/OWL files in the configured ontology repository directories ([general] ontology_dirs). Returns metadata for each candidate file (path, name, size, mtime, is_cached, is_active). Use this in containerized/server deployments to discover ontologies without knowing their paths in advance. Optional `dir` (must be under a configured repo dir), `recursive`, `glob`, `limit`, `offset` filters.")]
    fn onto_repo_list(&self, Parameters(input): Parameters<OntoRepoListInput>) -> String {
        let repos = self.ontology_dirs.as_ref();
        if repos.is_empty() {
            return r#"{"error":"no ontology_dirs configured; set [general] ontology_dirs in config.toml or OPEN_ONTOLOGIES_ONTOLOGY_DIRS"}"#.to_string();
        }
        let recursive = input.recursive.unwrap_or(false);
        let limit = input.limit.unwrap_or(1000);
        let offset = input.offset.unwrap_or(0);

        let entries = if let Some(dir) = input.dir.as_deref() {
            match crate::repo::resolve_within_repos(dir, repos) {
                Ok((start, repo_root)) => crate::repo::list_one(&repo_root, &start, recursive),
                Err(e) => {
                    return format!(
                        r#"{{"error":"{}"}}"#,
                        e.to_string().replace('"', "'")
                    );
                }
            }
        } else {
            crate::repo::list_all(repos, recursive)
        };

        let filtered: Vec<&crate::repo::RepoEntry> = entries
            .iter()
            .filter(|e| {
                if let Some(g) = input.glob.as_deref() {
                    let name = e
                        .path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    crate::repo::glob_match(g, name)
                } else {
                    true
                }
            })
            .collect();
        let total = filtered.len();

        // Snapshot cached names + currently active name for is_cached / is_active.
        let cached_names: std::collections::HashSet<String> = self
            .registry
            .cache()
            .list()
            .unwrap_or_default()
            .into_iter()
            .map(|e| e.name)
            .collect();
        let active_name = self
            .registry
            .status()
            .get("active")
            .and_then(|a| a.get("name"))
            .and_then(|n| n.as_str())
            .map(|s| s.to_string());

        let items: Vec<serde_json::Value> = filtered
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|e| {
                serde_json::json!({
                    "path": e.path.to_string_lossy(),
                    "relative": e.relative.to_string_lossy(),
                    "repo_dir": e.repo_dir.to_string_lossy(),
                    "name": e.name,
                    "size": e.size,
                    "mtime": e.mtime_secs,
                    "is_cached": cached_names.contains(&e.name),
                    "is_active": active_name.as_deref() == Some(e.name.as_str()),
                })
            })
            .collect();

        let repo_dirs: Vec<String> = repos
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();

        serde_json::json!({
            "ok": true,
            "ontology_dirs": repo_dirs,
            "total": total,
            "offset": offset,
            "limit": limit,
            "count": items.len(),
            "items": items,
        })
        .to_string()
    }

    #[tool(name = "onto_repo_load", description = "Load an ontology from one of the configured repository directories ([general] ontology_dirs) into the active store. The `name` argument can be a bare file stem, a relative path, or an absolute path inside a configured repo. Reuses the same compile-cache / TTL-eviction path as `onto_load`.")]
    async fn onto_repo_load(&self, Parameters(input): Parameters<OntoRepoLoadInput>) -> String {
        let repos = self.ontology_dirs.as_ref();
        let path = match crate::repo::resolve_load_target(&input.name, repos) {
            Ok(p) => p,
            Err(e) => {
                return format!(
                    r#"{{"error":"{}"}}"#,
                    e.to_string().replace('"', "'")
                );
            }
        };
        let opts = crate::registry::LoadOptions {
            name: input.registry_name,
            auto_refresh: input.auto_refresh.unwrap_or(false),
            force_recompile: input.force_recompile.unwrap_or(false),
        };
        match self.registry.load_file(&path.to_string_lossy(), opts) {
            Ok(res) => serde_json::json!({
                "ok": true,
                "triples_loaded": res.triple_count,
                "path": res.source_path,
                "name": res.name,
                "origin": res.origin,
                "cache_path": res.cache_path,
            })
            .to_string(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e.to_string().replace('"', "'")),
        }
    }

    #[tool(name = "onto_query", description = "Run a SPARQL query against the loaded ontology store. If the active ontology has been evicted from memory (idle TTL), it is transparently reloaded from the compile cache before the query runs.")]
    async fn onto_query(&self, Parameters(input): Parameters<OntoQueryInput>) -> String {
        if let Err(e) = self.registry.ensure_loaded() {
            return format!(r#"{{"error":"ensure_loaded: {}"}}"#, e.to_string().replace('"', "'"));
        }
        self.graph.sparql_select(&input.query).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_save", description = "Save the current ontology store to a file")]
    async fn onto_save(&self, Parameters(input): Parameters<OntoSaveInput>) -> String {
        if let Err(e) = self.registry.ensure_loaded() {
            return format!(r#"{{"error":"ensure_loaded: {}"}}"#, e.to_string().replace('"', "'"));
        }
        let format = input.format.as_deref().unwrap_or("turtle");
        let path = expand_tilde(&input.path);
        match self.graph.save_file(&path, format) {
            Ok(_) => format!(r#"{{"ok":true,"path":"{}","format":"{}"}}"#, path, format),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_stats", description = "Get statistics about the loaded ontology (triple count, classes, properties, individuals)")]
    fn onto_stats(&self) -> String {
        if let Err(e) = self.registry.ensure_loaded() {
            return format!(r#"{{"error":"ensure_loaded: {}"}}"#, e.to_string().replace('"', "'"));
        }
        self.graph.get_stats().unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_diff", description = "Compare two ontology files and show added/removed triples")]
    async fn onto_diff(&self, Parameters(input): Parameters<OntoDiffInput>) -> String {
        use crate::ontology::OntologyService;
        let old = match std::fs::read_to_string(&input.old_path) {
            Ok(c) => c,
            Err(e) => return format!(r#"{{"error":"Cannot read {}: {}"}}"#, input.old_path, e),
        };
        let new = match std::fs::read_to_string(&input.new_path) {
            Ok(c) => c,
            Err(e) => return format!(r#"{{"error":"Cannot read {}: {}"}}"#, input.new_path, e),
        };
        OntologyService::diff(&old, &new).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_lint", description = "Check an ontology for quality issues: missing labels, comments, domains, ranges")]
    async fn onto_lint(&self, Parameters(input): Parameters<OntoLintInput>) -> String {
        use crate::ontology::OntologyService;
        let content = if input.inline.unwrap_or(false) {
            input.input.clone()
        } else {
            match std::fs::read_to_string(&input.input) {
                Ok(c) => c,
                Err(e) => return format!(r#"{{"error":"{}"}}"#, e),
            }
        };
        OntologyService::lint_with_feedback(&content, Some(&self.db)).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_clear", description = "Clear all triples from the in-memory ontology store and unload the active registry slot (cache file is preserved)")]
    fn onto_clear(&self) -> String {
        // Drop the active registry entry; this also clears the graph.
        let _ = self.registry.unload(false);
        match self.graph.clear() {
            Ok(_) => r#"{"ok":true,"message":"Store cleared"}"#.to_string(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_unload", description = "Unload an ontology from memory. With no `name`, operates on the active ontology. With `name`, targets that cached entry — clears in-memory store if it is currently active. The on-disk compile cache is preserved unless `delete_cache=true`.")]
    fn onto_unload(&self, Parameters(input): Parameters<OntoUnloadInput>) -> String {
        let del = input.delete_cache.unwrap_or(false);
        if let Some(name) = input.name.as_deref() {
            return match self.registry.unload_named(name, del) {
                Ok(true) => serde_json::json!({
                    "ok": true,
                    "unloaded": name,
                    "deleted_cache": del,
                }).to_string(),
                Ok(false) => serde_json::json!({
                    "ok": true,
                    "unloaded": null,
                    "name": name,
                    "message": "entry exists in cache but was not in memory; pass delete_cache=true to remove it",
                }).to_string(),
                Err(e) => format!(r#"{{"error":"{}"}}"#, e.to_string().replace('"', "'")),
            };
        }
        match self.registry.unload(del) {
            Ok(Some(name)) => serde_json::json!({"ok": true, "unloaded": name, "deleted_cache": del}).to_string(),
            Ok(None) => r#"{"ok":true,"unloaded":null,"message":"no active ontology"}"#.to_string(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_recompile", description = "Force-recompile an ontology from its source file, ignoring the on-disk cache. With no `name`, recompiles the active ontology (and reloads it into memory). With `name`, recompiles that cached entry; if it is not the active slot, the in-memory store is left untouched.")]
    fn onto_recompile(&self, Parameters(input): Parameters<OntoRecompileInput>) -> String {
        let res = match input.name.as_deref() {
            Some(name) => self.registry.recompile_named(name),
            None => self.registry.recompile(),
        };
        match res {
            Ok(res) => serde_json::json!({
                "ok": true,
                "name": res.name,
                "triples_loaded": res.triple_count,
                "origin": res.origin,
                "cache_path": res.cache_path,
            }).to_string(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e.to_string().replace('"', "'")),
        }
    }

    #[tool(name = "onto_cache_status", description = "Inspect the compile cache: active ontology, all cached entries, and the cache configuration (TTL, auto_refresh, dir).")]
    fn onto_cache_status(&self, Parameters(_input): Parameters<OntoCacheStatusInput>) -> String {
        self.registry.status().to_string()
    }

    #[tool(name = "onto_cache_list", description = "List all cached ontologies with metadata (name, source_path, triple_count, source_mtime, source_size, cache_path, compiled_at, last_access_at) and runtime flags (is_active, in_memory). Lighter than onto_cache_status when you only need the list.")]
    fn onto_cache_list(&self, Parameters(_input): Parameters<OntoCacheListInput>) -> String {
        match self.registry.list_cached() {
            Ok(entries) => serde_json::json!({
                "ok": true,
                "count": entries.len(),
                "entries": entries,
            }).to_string(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e.to_string().replace('"', "'")),
        }
    }

    #[tool(name = "onto_cache_remove", description = "Remove a cached ontology by name. If it is the active slot, the in-memory store is unloaded first. By default the on-disk N-Triples cache file is also deleted; pass delete_file=false to keep it on disk.")]
    fn onto_cache_remove(&self, Parameters(input): Parameters<OntoCacheRemoveInput>) -> String {
        let delete_file = input.delete_file.unwrap_or(true);
        match self.registry.unload_named(&input.name, delete_file) {
            Ok(true) => serde_json::json!({
                "ok": true,
                "removed": input.name,
                "deleted_file": delete_file,
            }).to_string(),
            Ok(false) => serde_json::json!({
                "ok": true,
                "removed": null,
                "name": input.name,
                "message": "entry was found but delete_file=false and it was not active, so nothing changed",
            }).to_string(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e.to_string().replace('"', "'")),
        }
    }

    #[tool(name = "onto_pull", description = "Fetch an ontology from a remote URL or SPARQL endpoint and load it into the store")]
    async fn onto_pull(&self, Parameters(input): Parameters<OntoPullInput>) -> String {
        use crate::graph::GraphStore;
        if input.sparql.unwrap_or(false) {
            let query = input.query.as_deref().unwrap_or("CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }");
            match GraphStore::fetch_sparql(&input.url, query).await {
                Ok(content) => {
                    match self.graph.load_turtle(&content, None) {
                        Ok(count) => format!(r#"{{"ok":true,"triples_loaded":{},"source":"{}"}}"#, count, input.url),
                        Err(e) => format!(r#"{{"error":"Parse error: {}"}}"#, e),
                    }
                }
                Err(e) => format!(r#"{{"error":"{}"}}"#, e),
            }
        } else {
            match GraphStore::fetch_url(&input.url).await {
                Ok(content) => {
                    match self.graph.load_turtle(&content, None) {
                        Ok(count) => format!(r#"{{"ok":true,"triples_loaded":{},"source":"{}"}}"#, count, input.url),
                        Err(e) => format!(r#"{{"error":"Parse error: {}"}}"#, e),
                    }
                }
                Err(e) => format!(r#"{{"error":"{}"}}"#, e),
            }
        }
    }

    #[tool(name = "onto_push", description = "Push the current ontology store to a remote SPARQL endpoint")]
    async fn onto_push(&self, Parameters(input): Parameters<OntoPushInput>) -> String {
        use crate::graph::GraphStore;
        match self.graph.serialize("ntriples") {
            Ok(content) => {
                match GraphStore::push_sparql(&input.endpoint, &content).await {
                    Ok(msg) => format!(r#"{{"ok":true,"message":"{}"}}"#, msg),
                    Err(e) => format!(r#"{{"error":"{}"}}"#, e),
                }
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_import", description = "Resolve and load all owl:imports from the currently loaded ontology")]
    async fn onto_import(&self, Parameters(input): Parameters<OntoImportInput>) -> String {
        use crate::graph::GraphStore;
        let max_depth = input.max_depth.unwrap_or(3);
        let mut imported = Vec::new();
        let mut to_import: Vec<String> = Vec::new();

        let query = "SELECT ?import WHERE { ?onto <http://www.w3.org/2002/07/owl#imports> ?import }";
        if let Ok(result) = self.graph.sparql_select(query)
            && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result)
                && let Some(results) = parsed["results"].as_array() {
                    for row in results {
                        if let Some(uri) = row["import"].as_str() {
                            let uri = uri.trim_matches(|c| c == '<' || c == '>');
                            to_import.push(uri.to_string());
                        }
                    }
                }

        let mut depth = 0;
        while !to_import.is_empty() && depth < max_depth {
            let batch = std::mem::take(&mut to_import);
            for url in batch {
                if imported.contains(&url) { continue; }
                match GraphStore::fetch_url(&url).await {
                    Ok(content) => {
                        match self.graph.load_turtle(&content, None) {
                            Ok(_count) => {
                                imported.push(url.clone());
                                if let Ok(result) = self.graph.sparql_select(query)
                                    && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result)
                                        && let Some(results) = parsed["results"].as_array() {
                                            for row in results {
                                                if let Some(uri) = row["import"].as_str() {
                                                    let uri = uri.trim_matches(|c| c == '<' || c == '>').to_string();
                                                    if !imported.contains(&uri) && !to_import.contains(&uri) {
                                                        to_import.push(uri);
                                                    }
                                                }
                                            }
                                        }
                            }
                            Err(e) => { imported.push(format!("FAILED:{}: {}", url, e)); }
                        }
                    }
                    Err(e) => { imported.push(format!("FAILED:{}: {}", url, e)); }
                }
            }
            depth += 1;
        }

        serde_json::json!({
            "ok": true,
            "imported": imported,
            "total": imported.len(),
            "depth": depth,
        }).to_string()
    }

    // ── Marketplace ────────────────────────────────────────────────────────

    #[tool(name = "onto_marketplace", description = "Browse and install standard ontologies from a curated catalogue of 32 W3C/ISO/industry standards. Actions: 'list' (browse catalogue, optional domain filter) or 'install' (fetch and load by ID)")]
    async fn onto_marketplace(&self, Parameters(input): Parameters<OntoMarketplaceInput>) -> String {
        use crate::marketplace;
        match input.action.as_str() {
            "list" => {
                let entries = marketplace::list(input.domain.as_deref());
                let items: Vec<serde_json::Value> = entries.iter().map(|e| {
                    serde_json::json!({
                        "id": e.id,
                        "name": e.name,
                        "description": e.description,
                        "domain": e.domain,
                        "url": e.url,
                        "format": marketplace::format_name(e.format),
                    })
                }).collect();
                serde_json::json!({
                    "ok": true,
                    "count": items.len(),
                    "ontologies": items,
                }).to_string()
            }
            "install" => {
                let id = match input.id.as_deref() {
                    Some(id) => id,
                    None => return r#"{"error":"'id' is required for install action"}"#.to_string(),
                };
                let entry = match marketplace::find(id) {
                    Some(e) => e,
                    None => {
                        let available: Vec<&str> = marketplace::CATALOGUE.iter().map(|e| e.id).collect();
                        return serde_json::json!({
                            "error": format!("Unknown ontology ID: '{}'. Use action 'list' to see available IDs.", id),
                            "available": available,
                        }).to_string();
                    }
                };
                match crate::graph::GraphStore::fetch_url(entry.url).await {
                    Ok(content) => {
                        match self.graph.load_content_with_base(&content, entry.format, Some(entry.url)) {
                            Ok(count) => {
                                let stats = self.graph.get_stats().unwrap_or_default();
                                let stats_val: serde_json::Value = serde_json::from_str(&stats).unwrap_or_default();
                                serde_json::json!({
                                    "ok": true,
                                    "installed": entry.id,
                                    "name": entry.name,
                                    "triples_loaded": count,
                                    "source": entry.url,
                                    "classes": stats_val["classes"],
                                    "properties": stats_val["properties"],
                                    "individuals": stats_val["individuals"],
                                }).to_string()
                            }
                            Err(e) => format!(r#"{{"error":"Parse error for {}: {}"}}"#, entry.id, e),
                        }
                    }
                    Err(e) => format!(r#"{{"error":"Fetch error for {}: {}"}}"#, entry.id, e),
                }
            }
            other => format!(r#"{{"error":"Unknown action '{}'. Use 'list' or 'install'."}}"#, other),
        }
    }

    #[tool(name = "onto_version", description = "Save a named snapshot of the current ontology store")]
    async fn onto_version(&self, Parameters(input): Parameters<OntoVersionInput>) -> String {
        use crate::ontology::OntologyService;
        OntologyService::save_version(&self.db, &self.graph, &input.label)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_history", description = "List all saved ontology version snapshots")]
    fn onto_history(&self) -> String {
        use crate::ontology::OntologyService;
        OntologyService::list_versions(&self.db)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_rollback", description = "Restore the ontology store to a previously saved version")]
    async fn onto_rollback(&self, Parameters(input): Parameters<OntoRollbackInput>) -> String {
        use crate::ontology::OntologyService;
        OntologyService::rollback_version(&self.db, &self.graph, &input.label)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    // ── Data ingestion & reasoning ─────────────────────────────────────────

    #[tool(name = "onto_ingest", description = "Parse a structured data file (CSV, JSON, NDJSON, XML, YAML, XLSX, Parquet) into RDF triples and load into the ontology store. Optionally uses a mapping config to control field-to-predicate mapping.")]
    async fn onto_ingest(&self, Parameters(input): Parameters<OntoIngestInput>) -> String {
        use crate::ingest::DataIngester;
        use crate::mapping::MappingConfig;

        let base_iri = input.base_iri.as_deref().unwrap_or("http://example.org/data/");

        // Parse data file
        let rows = match DataIngester::parse_file(&input.path) {
            Ok(r) => r,
            Err(e) => return format!(r#"{{"error":"Failed to parse {}: {}"}}"#, input.path, e),
        };

        if rows.is_empty() {
            return r#"{"ok":true,"triples_loaded":0,"warnings":["No data rows found"]}"#.to_string();
        }

        // Get or generate mapping
        let mapping = if let Some(ref mapping_str) = input.mapping {
            if input.inline_mapping.unwrap_or(false) {
                match serde_json::from_str::<MappingConfig>(mapping_str) {
                    Ok(m) => m,
                    Err(e) => return format!(r#"{{"error":"Invalid mapping JSON: {}"}}"#, e),
                }
            } else {
                match std::fs::read_to_string(mapping_str) {
                    Ok(content) => match serde_json::from_str::<MappingConfig>(&content) {
                        Ok(m) => m,
                        Err(e) => return format!(r#"{{"error":"Invalid mapping file: {}"}}"#, e),
                    },
                    Err(e) => return format!(r#"{{"error":"Cannot read mapping file: {}"}}"#, e),
                }
            }
        } else {
            let headers = DataIngester::extract_headers(&rows);
            MappingConfig::from_headers(&headers, base_iri, &format!("{}Thing", base_iri))
        };

        // Convert to N-Triples and load
        let ntriples = mapping.rows_to_ntriples(&rows);
        match self.graph.load_ntriples(&ntriples) {
            Ok(count) => {
                serde_json::json!({
                    "ok": true,
                    "triples_loaded": count,
                    "rows_processed": rows.len(),
                    "mapping_fields": mapping.mappings.len(),
                }).to_string()
            }
            Err(e) => format!(r#"{{"error":"Failed to load triples: {}"}}"#, e),
        }
    }

    #[tool(name = "onto_map", description = "Generate a mapping config by inspecting a data file's schema against the currently loaded ontology. Returns a JSON mapping that can be reviewed and passed to onto_ingest.")]
    async fn onto_map(&self, Parameters(input): Parameters<OntoMapInput>) -> String {
        use crate::ingest::DataIngester;
        use crate::mapping::MappingConfig;

        let rows = match DataIngester::parse_file(&input.data_path) {
            Ok(r) => r,
            Err(e) => return format!(r#"{{"error":"Failed to parse {}: {}"}}"#, input.data_path, e),
        };
        let headers = DataIngester::extract_headers(&rows);

        // Get ontology classes and properties from the store
        let classes_query = r#"SELECT DISTINCT ?c WHERE {
            { ?c a <http://www.w3.org/2002/07/owl#Class> }
            UNION
            { ?c a <http://www.w3.org/2000/01/rdf-schema#Class> }
        }"#;
        let props_query = r#"SELECT DISTINCT ?p WHERE {
            { ?p a <http://www.w3.org/2002/07/owl#ObjectProperty> }
            UNION
            { ?p a <http://www.w3.org/2002/07/owl#DatatypeProperty> }
            UNION
            { ?p a <http://www.w3.org/1999/02/22-rdf-syntax-ns#Property> }
        }"#;

        let classes = self.graph.sparql_select(classes_query).unwrap_or_default();
        let props = self.graph.sparql_select(props_query).unwrap_or_default();

        let extract_iris = |json: &str, var: &str| -> Vec<String> {
            serde_json::from_str::<serde_json::Value>(json)
                .ok()
                .and_then(|v| v["results"].as_array().cloned())
                .unwrap_or_default()
                .iter()
                .filter_map(|r| r[var].as_str().map(|s| s.trim_matches(|c| c == '<' || c == '>').to_string()))
                .collect()
        };

        let class_iris = extract_iris(&classes, "c");
        let prop_iris = extract_iris(&props, "p");

        let mapping = MappingConfig::from_headers(
            &headers,
            "http://example.org/data/",
            class_iris.first().map(|s| s.as_str()).unwrap_or("http://example.org/Thing"),
        );

        let result = serde_json::json!({
            "mapping": mapping,
            "data_fields": headers,
            "ontology_classes": class_iris,
            "ontology_properties": prop_iris,
        });

        if let Some(ref save_path) = input.save_path
            && let Ok(json) = serde_json::to_string_pretty(&mapping)
                && let Err(e) = std::fs::write(save_path, &json) {
                    return format!(r#"{{"error":"Cannot write mapping file: {}"}}"#, e);
                }

        result.to_string()
    }

    #[tool(name = "onto_shacl", description = "Validate the loaded ontology data against SHACL shapes. Checks cardinality (minCount/maxCount), datatypes, and class constraints. Returns a conformance report with violations.")]
    async fn onto_shacl(&self, Parameters(input): Parameters<OntoShaclInput>) -> String {
        use crate::shacl::ShaclValidator;
        let shapes = if input.inline.unwrap_or(false) {
            input.shapes.clone()
        } else {
            match std::fs::read_to_string(&input.shapes) {
                Ok(c) => c,
                Err(e) => return format!(r#"{{"error":"Cannot read shapes file: {}"}}"#, e),
            }
        };
        ShaclValidator::validate(&self.graph, &shapes)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_reason", description = "Run inference over the loaded ontology. Profiles: 'rdfs' (subclass, domain/range), 'owl-rl' (+ transitive/symmetric/inverse, sameAs, equivalentClass), 'owl-rl-ext' (+ someValuesFrom, allValuesFrom, hasValue, intersectionOf, unionOf), 'owl-dl' (Full OWL2-DL SHOIQ tableaux: satisfiability, classification, qualified number restrictions with node merging, inverse/symmetric roles, functional properties, parallel agent-based classification, explanation traces, ABox reasoning). Materializes inferred triples.")]
    async fn onto_reason(&self, Parameters(input): Parameters<OntoReasonInput>) -> String {
        use crate::reason::Reasoner;
        let profile = input.profile.as_deref().unwrap_or("rdfs");
        let materialize = input.materialize.unwrap_or(true);
        Reasoner::run(&self.graph, profile, materialize)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_dl_explain", description = "Explain why a class is unsatisfiable using DL tableaux reasoning. Returns an explanation trace showing the logical contradictions that make the class impossible to instantiate.")]
    async fn onto_dl_explain(&self, Parameters(input): Parameters<OntoDlExplainInput>) -> String {
        use crate::tableaux::DlReasoner;
        DlReasoner::explain_class(&self.graph, &input.class_iri)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_dl_check", description = "Check if one class is subsumed by another using DL tableaux reasoning. Returns whether sub_class is a subclass of super_class, with justification.")]
    async fn onto_dl_check(&self, Parameters(input): Parameters<OntoDlCheckInput>) -> String {
        use crate::tableaux::DlReasoner;
        DlReasoner::check_subsumption(&self.graph, &input.sub_class, &input.super_class)
            .unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    // ── v2: Lifecycle tools ─────────────────────────────────────────────────

    #[tool(name = "onto_plan", description = "Terraform-style plan: diff current store against proposed Turtle. Shows added/removed classes/properties, blast radius, risk score, and locked IRI violations.")]
    async fn onto_plan(&self, Parameters(input): Parameters<OntoPlanInput>) -> String {
        let planner = crate::plan::Planner::new(self.db.clone(), self.graph.clone());
        match planner.plan(&input.new_turtle) {
            Ok(result) => {
                self.lineage().record(&self.session_id, "P", "plan", "computed");
                result
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_apply", description = "Apply the last plan. Modes: 'safe' (clear+reload, checks monitor), 'force' (ignores monitor), 'migrate' (adds owl:equivalentClass/Property bridges for renames).")]
    async fn onto_apply(&self, Parameters(input): Parameters<OntoApplyInput>) -> String {
        let mode = input.mode.as_deref().unwrap_or("safe");
        let planner = crate::plan::Planner::new(self.db.clone(), self.graph.clone());
        match planner.apply(mode) {
            Ok(result) => {
                self.lineage().record(&self.session_id, "A", "apply", mode);
                let monitor_result = self.monitor().run_watchers();
                if monitor_result.status != "ok" {
                    let mut parsed: serde_json::Value = serde_json::from_str(&result).unwrap_or_default();
                    parsed["monitor"] = serde_json::to_value(&monitor_result).unwrap_or_default();
                    return parsed.to_string();
                }
                result
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_lock", description = "Lock IRIs to prevent removal during plan/apply. Locked IRIs will show as violations in plan output.")]
    async fn onto_lock(&self, Parameters(input): Parameters<OntoLockInput>) -> String {
        let planner = crate::plan::Planner::new(self.db.clone(), self.graph.clone());
        let reason = input.reason.as_deref().unwrap_or("locked");
        for iri in &input.iris {
            planner.lock_iri(iri, reason);
        }
        serde_json::json!({
            "ok": true,
            "locked": input.iris,
            "reason": reason,
        }).to_string()
    }

    #[tool(name = "onto_drift", description = "Detect drift between two ontology versions. Returns added/removed terms, likely renames with confidence scores, and drift velocity.")]
    async fn onto_drift(&self, Parameters(input): Parameters<OntoDriftInput>) -> String {
        let detector = crate::drift::DriftDetector::new(self.db.clone());
        match detector.detect(&input.version_a, &input.version_b) {
            Ok(result) => {
                self.lineage().record(&self.session_id, "D", "drift", "detected");
                result
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_enforce", description = "Enforce design patterns on the loaded ontology. Built-in packs: 'generic' (orphan classes, missing domain/range/label), 'boro' (BORO 4D patterns), 'value_partition' (disjoint/covering checks). Also runs any custom rules stored for the pack.")]
    async fn onto_enforce(&self, Parameters(input): Parameters<OntoEnforceInput>) -> String {
        let enforcer = crate::enforce::Enforcer::new(self.db.clone(), self.graph.clone());
        match enforcer.enforce_with_feedback(&input.rule_pack, Some(&self.db)) {
            Ok(result) => {
                self.lineage().record(&self.session_id, "E", "enforce", &input.rule_pack);
                result
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_monitor", description = "Run active monitoring watchers. Optionally add new watchers via inline JSON. Watchers with action=notify and a webhook_url will POST alerts to the URL. Returns ok/alert/blocked status with details.")]
    async fn onto_monitor(&self, Parameters(input): Parameters<OntoMonitorInput>) -> String {
        let monitor = self.monitor();

        // Add watchers if provided
        if let Some(ref watchers_json) = input.watchers
            && let Ok(watchers) = serde_json::from_str::<Vec<crate::monitor::Watcher>>(watchers_json) {
                for w in watchers {
                    monitor.add_watcher(w);
                }
            }

        let result = monitor.run_watchers();
        self.lineage().record(&self.session_id, "M", "monitor", &result.status);
        serde_json::to_string(&result).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_monitor_clear", description = "Clear the monitor blocked flag, allowing apply operations to proceed.")]
    fn onto_monitor_clear(&self) -> String {
        self.monitor().clear_blocked();
        r#"{"ok":true,"message":"Monitor block cleared"}"#.to_string()
    }

    #[tool(name = "onto_crosswalk", description = "Look up clinical crosswalk mappings for a code and system (ICD10, SNOMED, MeSH). Uses data/crosswalks.parquet (93-row sample included; run scripts/build_crosswalks.py to extend).")]
    async fn onto_crosswalk(&self, Parameters(input): Parameters<OntoCrosswalkInput>) -> String {
        match crate::clinical::ClinicalCrosswalks::load("data/crosswalks.parquet") {
            Ok(cw) => {
                let results = cw.lookup(&input.code, &input.source_system);
                serde_json::json!({
                    "code": input.code,
                    "system": input.source_system,
                    "mappings": results.iter().map(|r| serde_json::json!({
                        "target_code": r.target_code,
                        "target_system": r.target_system,
                        "relation": r.relation,
                        "source_label": r.source_label,
                        "target_label": r.target_label,
                    })).collect::<Vec<_>>(),
                }).to_string()
            }
            Err(e) => format!(r#"{{"error":"Crosswalks not loaded: {}. Run scripts/build_crosswalks.py first."}}"#, e),
        }
    }

    #[tool(name = "onto_enrich", description = "Enrich an ontology class with a SKOS mapping triple from the clinical crosswalks.")]
    async fn onto_enrich(&self, Parameters(input): Parameters<OntoEnrichInput>) -> String {
        match crate::clinical::ClinicalCrosswalks::load("data/crosswalks.parquet") {
            Ok(cw) => cw.enrich(&self.graph, &input.class_iri, &input.code, &input.system),
            Err(e) => format!(r#"{{"error":"Crosswalks not loaded: {}"}}"#, e),
        }
    }

    #[tool(name = "onto_validate_clinical", description = "Validate all class labels in the loaded ontology against clinical crosswalk data. Shows which terms match known clinical codes.")]
    fn onto_validate_clinical(&self) -> String {
        match crate::clinical::ClinicalCrosswalks::load("data/crosswalks.parquet") {
            Ok(cw) => cw.validate_clinical(&self.graph),
            Err(e) => format!(r#"{{"error":"Crosswalks not loaded: {}"}}"#, e),
        }
    }

    #[tool(name = "onto_lineage", description = "Get the compact lineage log for the current or specified session.")]
    async fn onto_lineage(&self, Parameters(input): Parameters<OntoLineageInput>) -> String {
        let session = input.session_id.as_deref().unwrap_or(&self.session_id);
        let events = self.lineage().get_compact(session);
        serde_json::json!({
            "session_id": session,
            "events": events.trim(),
        }).to_string()
    }

    #[tool(name = "onto_extend", description = "Convenience pipeline: ingest data → validate with SHACL → run OWL reasoning, all in one call. Combines onto_ingest + onto_shacl + onto_reason.")]
    async fn onto_extend(&self, Parameters(input): Parameters<OntoExtendInput>) -> String {
        use crate::ingest::DataIngester;
        use crate::mapping::MappingConfig;
        use crate::shacl::ShaclValidator;
        use crate::reason::Reasoner;

        let base_iri = input.base_iri.as_deref().unwrap_or("http://example.org/data/");

        // 1. Ingest
        let rows = match DataIngester::parse_file(&input.data_path) {
            Ok(r) => r,
            Err(e) => return format!(r#"{{"error":"Ingest failed: {}"}}"#, e),
        };

        let mapping = if let Some(ref mapping_str) = input.mapping {
            if input.inline_mapping.unwrap_or(false) {
                match serde_json::from_str::<MappingConfig>(mapping_str) {
                    Ok(m) => m,
                    Err(e) => return format!(r#"{{"error":"Invalid mapping: {}"}}"#, e),
                }
            } else {
                match std::fs::read_to_string(mapping_str) {
                    Ok(content) => match serde_json::from_str::<MappingConfig>(&content) {
                        Ok(m) => m,
                        Err(e) => return format!(r#"{{"error":"Invalid mapping file: {}"}}"#, e),
                    },
                    Err(e) => return format!(r#"{{"error":"Cannot read mapping: {}"}}"#, e),
                }
            }
        } else {
            let headers = DataIngester::extract_headers(&rows);
            MappingConfig::from_headers(&headers, base_iri, &format!("{}Thing", base_iri))
        };

        let ntriples = mapping.rows_to_ntriples(&rows);
        let triples_loaded = match self.graph.load_ntriples(&ntriples) {
            Ok(c) => c,
            Err(e) => return format!(r#"{{"error":"Failed to load triples: {}"}}"#, e),
        };

        // 2. SHACL (optional)
        let mut shacl_result = serde_json::json!({"skipped": true});
        if let Some(ref shapes_input) = input.shapes {
            let shapes = if input.inline_shapes.unwrap_or(false) {
                shapes_input.clone()
            } else {
                match std::fs::read_to_string(shapes_input) {
                    Ok(c) => c,
                    Err(e) => return format!(r#"{{"error":"Cannot read shapes: {}"}}"#, e),
                }
            };
            match ShaclValidator::validate(&self.graph, &shapes) {
                Ok(report) => {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&report) {
                        let stop = input.stop_on_violations.unwrap_or(true);
                        if stop && parsed["conforms"] == false {
                            return serde_json::json!({
                                "stage": "shacl",
                                "triples_ingested": triples_loaded,
                                "shacl": parsed,
                                "stopped": true,
                                "message": "Pipeline stopped due to SHACL violations",
                            }).to_string();
                        }
                        shacl_result = parsed;
                    }
                }
                Err(e) => return format!(r#"{{"error":"SHACL validation failed: {}"}}"#, e),
            }
        }

        // 3. Reasoning (optional)
        let mut reason_result = serde_json::json!({"skipped": true});
        if let Some(ref profile) = input.reason_profile {
            match Reasoner::run(&self.graph, profile, true) {
                Ok(report) => {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&report) {
                        reason_result = parsed;
                    }
                }
                Err(e) => return format!(r#"{{"error":"Reasoning failed: {}"}}"#, e),
            }
        }

        serde_json::json!({
            "ok": true,
            "triples_ingested": triples_loaded,
            "rows_processed": rows.len(),
            "shacl": shacl_result,
            "reasoning": reason_result,
        }).to_string()
    }

    #[tool(name = "onto_import_schema", description = "Import a PostgreSQL database schema as an OWL ontology. Introspects tables, columns, primary keys, and foreign keys, then generates OWL classes, datatype/object properties, and cardinality restrictions.")]
    async fn onto_import_schema(&self, Parameters(input): Parameters<OntoImportSchemaInput>) -> String {
        #[cfg(not(feature = "postgres"))]
        { let _ = input; return r#"{"error":"Compiled without postgres feature. Rebuild with --features postgres"}"#.to_string(); }
        #[cfg(feature = "postgres")]
        {
        use crate::schema::SchemaIntrospector;
        let base_iri = input.base_iri.as_deref().unwrap_or("http://example.org/db/");

        let tables = match SchemaIntrospector::introspect_postgres(&input.connection).await {
            Ok(t) => t,
            Err(e) => return format!(r#"{{"error":"Connection failed: {}"}}"#, e),
        };

        let turtle = SchemaIntrospector::generate_turtle(&tables, base_iri);

        // Validate + load
        if let Err(e) = GraphStore::validate_turtle(&turtle) {
            return format!(r#"{{"error":"Generated Turtle invalid: {}"}}"#, e);
        }

        match self.graph.load_turtle(&turtle, Some(base_iri)) {
            Ok(count) => serde_json::json!({
                "ok": true,
                "tables": tables.len(),
                "classes": tables.len(),
                "triples": count,
                "base_iri": base_iri,
            }).to_string(),
            Err(e) => format!(r#"{{"error":"Failed to load: {}"}}"#, e),
        }
        } // cfg(feature = "postgres")
    }

    #[tool(name = "onto_align", description = "Detect alignment candidates (owl:equivalentClass, skos:exactMatch, rdfs:subClassOf) between two ontologies using label similarity, property overlap, parent overlap, instance overlap, restriction patterns, and graph neighborhood. Auto-applies high-confidence matches above threshold.")]
    async fn onto_align(&self, Parameters(input): Parameters<OntoAlignInput>) -> String {
        let engine = crate::align::AlignmentEngine::new(self.db.clone(), self.graph.clone());

        // Read source (file path or inline)
        let source = if std::path::Path::new(&input.source).exists() {
            match std::fs::read_to_string(&input.source) {
                Ok(s) => s,
                Err(e) => return format!(r#"{{"error":"Failed to read source: {}"}}"#, e),
            }
        } else {
            input.source
        };

        // Read target (file path, inline, or None)
        let target = match input.target {
            Some(t) => {
                if std::path::Path::new(&t).exists() {
                    match std::fs::read_to_string(&t) {
                        Ok(s) => Some(s),
                        Err(e) => return format!(r#"{{"error":"Failed to read target: {}"}}"#, e),
                    }
                } else {
                    Some(t)
                }
            }
            None => None,
        };

        let min_conf = input.min_confidence.unwrap_or(0.85);
        let dry_run = input.dry_run.unwrap_or(false);

        match engine.align(&source, target.as_deref(), min_conf, dry_run) {
            Ok(result) => {
                self.lineage().record(&self.session_id, "AL", "align", &format!("threshold={}", min_conf));
                result
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_align_feedback", description = "Accept or reject an alignment candidate to improve future confidence scoring. Stores feedback in align_feedback table for self-calibrating weights.")]
    async fn onto_align_feedback(&self, Parameters(input): Parameters<OntoAlignFeedbackInput>) -> String {
        let engine = crate::align::AlignmentEngine::new(self.db.clone(), self.graph.clone());
        match engine.record_feedback(&input.source_iri, &input.target_iri, "user_feedback", input.accepted, input.signals.as_ref()) {
            Ok(result) => {
                self.lineage().record(&self.session_id, "AF", "align_feedback", if input.accepted { "accepted" } else { "rejected" });
                result
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_lint_feedback", description = "Accept or dismiss a lint issue to improve future lint runs. Dismissed issues are suppressed after 3 dismissals. Stores feedback for self-calibrating severity.")]
    async fn onto_lint_feedback(&self, Parameters(input): Parameters<OntoLintFeedbackInput>) -> String {
        match crate::feedback::record_tool_feedback(&self.db, "lint", &input.rule_id, &input.entity, input.accepted) {
            Ok(result) => {
                self.lineage().record(&self.session_id, "LF", "lint_feedback", if input.accepted { "accepted" } else { "dismissed" });
                result
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_enforce_feedback", description = "Accept or dismiss an enforce violation to improve future enforce runs. Dismissed violations are suppressed after 3 dismissals. Stores feedback for self-calibrating compliance.")]
    async fn onto_enforce_feedback(&self, Parameters(input): Parameters<OntoEnforceFeedbackInput>) -> String {
        match crate::feedback::record_tool_feedback(&self.db, "enforce", &input.rule_id, &input.entity, input.accepted) {
            Ok(result) => {
                self.lineage().record(&self.session_id, "EF", "enforce_feedback", if input.accepted { "accepted" } else { "dismissed" });
                result
            }
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_embed", description = "Generate text + structural Poincaré embeddings for all classes in the loaded ontology. Requires the embedding model (run `open-ontologies init` to download). Embeddings enable semantic search via onto_search and improve alignment accuracy.")]
    async fn onto_embed(&self, Parameters(input): Parameters<OntoEmbedInput>) -> String {
        #[cfg(not(feature = "embeddings"))]
        { let _ = input; return r#"{"error":"Compiled without embeddings feature. Rebuild with --features embeddings"}"#.to_string(); }
        #[cfg(feature = "embeddings")]
        {
        let embedder = match &self.text_embedder {
            Some(e) => e,
            None => return r#"{"error":"Embedding model not loaded. Run `open-ontologies init` to download."}"#.to_string(),
        };

        let struct_dim = input.struct_dim.unwrap_or(32);
        let struct_epochs = input.struct_epochs.unwrap_or(100);

        let classes_query = r#"
            SELECT DISTINCT ?class ?label WHERE {
                ?class a <http://www.w3.org/2002/07/owl#Class> .
                OPTIONAL { ?class <http://www.w3.org/2000/01/rdf-schema#label> ?label }
                FILTER(isIRI(?class))
            }
        "#;

        let result = match self.graph.sparql_select(classes_query) {
            Ok(r) => r,
            Err(e) => return format!(r#"{{"error":"{}"}}"#, e),
        };

        let parsed: serde_json::Value = match serde_json::from_str(&result) {
            Ok(v) => v,
            Err(e) => return format!(r#"{{"error":"{}"}}"#, e),
        };

        let mut class_labels: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        if let Some(rows) = parsed["results"].as_array() {
            for row in rows {
                if let Some(iri) = row["class"].as_str() {
                    let iri = iri.trim_matches(|c| c == '<' || c == '>').to_string();
                    let label = row["label"].as_str()
                        .map(|s| s.trim_matches('"').to_string())
                        .unwrap_or_else(|| {
                            iri.rsplit_once('#').or_else(|| iri.rsplit_once('/'))
                                .map(|(_, n)| n.to_string())
                                .unwrap_or_else(|| iri.clone())
                        });
                    class_labels.insert(iri, label);
                }
            }
        }

        let trainer = crate::structembed::StructuralTrainer::new(struct_dim, struct_epochs, 0.01);
        let struct_embeddings = match trainer.train(&self.graph) {
            Ok(e) => e,
            Err(e) => return format!(r#"{{"error":"structural training failed: {}"}}"#, e),
        };

        let mut embedded_count = 0;
        let mut errors: Vec<String> = Vec::new();

        for (iri, label) in &class_labels {
            // Compute the text embedding (may await an HTTP call) BEFORE
            // locking the non-Send VecStore mutex.
            match embedder.embed(label).await {
                Ok(text_vec) => {
                    let struct_vec = struct_embeddings.get(iri)
                        .cloned()
                        .unwrap_or_else(|| vec![0.0; struct_dim]);
                    let mut vecstore = self.vecstore.lock().unwrap();
                    vecstore.upsert(iri, &text_vec, &struct_vec);
                    embedded_count += 1;
                }
                Err(e) => errors.push(format!("{}: {}", iri, e)),
            }
        }

        {
            let vecstore = self.vecstore.lock().unwrap();
            if let Err(e) = vecstore.persist() {
                return format!(r#"{{"error":"failed to persist embeddings: {}"}}"#, e);
            }
        }

        serde_json::json!({
            "ok": true,
            "embedded": embedded_count,
            "total_classes": class_labels.len(),
            "text_dim": embedder.dim(),
            "struct_dim": struct_dim,
            "errors": errors,
        }).to_string()
        } // cfg(feature = "embeddings")
    }

    #[tool(name = "onto_search", description = "Semantic search over the loaded ontology using natural language. Returns the most similar classes by text meaning, structural position, or both. Requires onto_embed to have been run first.")]
    async fn onto_search(&self, Parameters(input): Parameters<OntoSearchInput>) -> String {
        #[cfg(not(feature = "embeddings"))]
        { let _ = input; return r#"{"error":"Compiled without embeddings feature. Rebuild with --features embeddings"}"#.to_string(); }
        #[cfg(feature = "embeddings")]
        {
        let top_k = input.top_k.unwrap_or(10);
        let mode = input.mode.as_deref().unwrap_or("product");
        let alpha = input.alpha.unwrap_or(0.5);

        let embedder = match &self.text_embedder {
            Some(e) => e,
            None => return r#"{"error":"Embedding model not loaded."}"#.to_string(),
        };

        let query_vec = match embedder.embed(&input.query).await {
            Ok(v) => v,
            Err(e) => return format!(r#"{{"error":"{}"}}"#, e),
        };

        let vecstore = self.vecstore.lock().unwrap();
        if vecstore.is_empty() {
            return r#"{"error":"No embeddings loaded. Run onto_embed first."}"#.to_string();
        }

        let results: Vec<serde_json::Value> = match mode {
            "text" => {
                vecstore.search_cosine(&query_vec, top_k)
                    .into_iter()
                    .map(|(iri, score)| serde_json::json!({"iri": iri, "score": (score * 1000.0).round() / 1000.0}))
                    .collect()
            }
            "structure" => {
                let text_hits = vecstore.search_cosine(&query_vec, 1);
                if let Some((anchor_iri, _)) = text_hits.first() {
                    if let Some(struct_vec) = vecstore.get_struct_vec(anchor_iri) {
                        vecstore.search_poincare(struct_vec, top_k)
                            .into_iter()
                            .map(|(iri, dist)| serde_json::json!({"iri": iri, "poincare_distance": (dist * 1000.0).round() / 1000.0}))
                            .collect()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            }
            _ => {
                let struct_dim = vecstore.search_cosine(&query_vec, 1)
                    .first()
                    .and_then(|(iri, _)| vecstore.get_struct_vec(iri).map(|v| v.len()))
                    .unwrap_or(32);
                let struct_query = vec![0.0f32; struct_dim];
                vecstore.search_product(&query_vec, &struct_query, top_k, alpha)
                    .into_iter()
                    .map(|(iri, score)| serde_json::json!({"iri": iri, "score": (score * 1000.0).round() / 1000.0}))
                    .collect()
            }
        };

        serde_json::json!({
            "results": results,
            "query": input.query,
            "mode": mode,
            "count": results.len(),
        }).to_string()
        } // cfg(feature = "embeddings")
    }

    #[tool(name = "onto_similarity", description = "Compute embedding similarity between two IRIs — returns cosine similarity (text), Poincaré distance (structural), and product score.")]
    async fn onto_similarity(&self, Parameters(input): Parameters<OntoSimilarityInput>) -> String {
        #[cfg(not(feature = "embeddings"))]
        { let _ = input; return r#"{"error":"Compiled without embeddings feature. Rebuild with --features embeddings"}"#.to_string(); }
        #[cfg(feature = "embeddings")]
        {
        let vecstore = self.vecstore.lock().unwrap();

        let text_a = vecstore.get_text_vec(&input.iri_a);
        let text_b = vecstore.get_text_vec(&input.iri_b);
        let struct_a = vecstore.get_struct_vec(&input.iri_a);
        let struct_b = vecstore.get_struct_vec(&input.iri_b);

        if text_a.is_none() || text_b.is_none() {
            return format!(r#"{{"error":"IRI not found in embeddings. Run onto_embed first. Missing: {}"}}"#,
                if text_a.is_none() { &input.iri_a } else { &input.iri_b });
        }

        let cos = crate::poincare::cosine_similarity(text_a.unwrap(), text_b.unwrap());
        let poinc = if let (Some(a), Some(b)) = (struct_a, struct_b) {
            crate::poincare::poincare_distance(a, b)
        } else {
            -1.0
        };

        let product = if poinc >= 0.0 {
            0.5 * cos + 0.5 / (1.0 + poinc)
        } else {
            cos
        };

        serde_json::json!({
            "iri_a": input.iri_a,
            "iri_b": input.iri_b,
            "cosine_similarity": (cos * 1000.0).round() / 1000.0,
            "poincare_distance": (poinc * 1000.0).round() / 1000.0,
            "product_score": (product * 1000.0).round() / 1000.0,
        }).to_string()
        } // cfg(feature = "embeddings")
    }
}

// ─── Prompt definitions ─────────────────────────────────────────────────────

#[prompt_router]
impl OpenOntologiesServer {
    /// Build an ontology from a domain description. Guides through the full workflow: generate Turtle, validate, load, lint, query, and persist.
    #[prompt(name = "build_ontology")]
    fn build_ontology(&self, Parameters(input): Parameters<BuildOntologyInput>) -> Result<GetPromptResult, rmcp::ErrorData> {
        let msg = format!(
            "Build an OWL ontology for the following domain:\n\n{}\n\n\
            Follow the Open Ontologies workflow:\n\
            1. Generate Turtle/OWL directly\n\
            2. Call onto_validate on the generated Turtle\n\
            3. Call onto_load to load into the triple store\n\
            4. Call onto_stats to verify counts\n\
            5. Call onto_lint to check for missing labels, comments, domains, ranges\n\
            6. Call onto_query with SPARQL to verify structure\n\
            7. Fix any issues and iterate until clean\n\
            8. Call onto_save to persist the final ontology",
            input.domain
        );
        Ok(GetPromptResult::new(vec![
            PromptMessage::new_text(PromptMessageRole::User, msg),
        ]).with_description("Build an ontology from a domain description"))
    }

    /// Validate and lint an existing ontology file. Loads it, runs validation and lint checks, reports all issues.
    #[prompt(name = "validate_ontology")]
    fn validate_ontology(&self, Parameters(input): Parameters<ValidateOntologyInput>) -> Result<GetPromptResult, rmcp::ErrorData> {
        let msg = format!(
            "Validate and lint the ontology at: {}\n\n\
            Steps:\n\
            1. Call onto_validate to check syntax\n\
            2. Call onto_load to load into the triple store\n\
            3. Call onto_stats to show class/property/triple counts\n\
            4. Call onto_lint to check for missing labels, domains, ranges\n\
            5. Report all issues found and suggest fixes",
            input.path
        );
        Ok(GetPromptResult::new(vec![
            PromptMessage::new_text(PromptMessageRole::User, msg),
        ]).with_description("Validate and lint an ontology file"))
    }

    /// Compare two versions of an ontology. Shows added/removed classes, properties, and drift analysis.
    #[prompt(name = "compare_ontologies")]
    fn compare_ontologies(&self, Parameters(input): Parameters<CompareOntologiesInput>) -> Result<GetPromptResult, rmcp::ErrorData> {
        let msg = format!(
            "Compare these two ontology versions:\n\
            - Old: {}\n\
            - New: {}\n\n\
            Steps:\n\
            1. Call onto_diff to see structural changes\n\
            2. Call onto_drift to analyze drift velocity and detect renames\n\
            3. Summarize: what was added, removed, renamed, and the overall risk",
            input.old_path, input.new_path
        );
        Ok(GetPromptResult::new(vec![
            PromptMessage::new_text(PromptMessageRole::User, msg),
        ]).with_description("Compare two ontology versions"))
    }

    /// Ingest external data into a loaded ontology. Maps data fields to ontology classes/properties and validates with SHACL.
    #[prompt(name = "ingest_data")]
    fn ingest_data(&self, Parameters(input): Parameters<IngestDataInput>) -> Result<GetPromptResult, rmcp::ErrorData> {
        let msg = format!(
            "Ingest data from {} into the currently loaded ontology.\n\n\
            Steps:\n\
            1. Call onto_map to inspect the data and suggest a mapping\n\
            2. Review and adjust the mapping\n\
            3. Call onto_ingest with the mapping to generate RDF triples\n\
            4. Call onto_stats to verify triple counts\n\
            5. Call onto_shacl to validate against SHACL shapes\n\
            6. Call onto_reason to infer additional triples\n\
            7. Call onto_query to verify the ingested data",
            input.data_path
        );
        Ok(GetPromptResult::new(vec![
            PromptMessage::new_text(PromptMessageRole::User, msg),
        ]).with_description("Ingest external data into a loaded ontology"))
    }

    /// Align two ontologies using hybrid neuro-symbolic matching. Runs structural alignment first, then asks you (the LLM) to adjudicate uncertain pairs.
    #[prompt(name = "align_ontologies")]
    fn align_ontologies(&self, Parameters(input): Parameters<AlignOntologiesInput>) -> Result<GetPromptResult, rmcp::ErrorData> {
        let msg = format!(
            "Align these two ontologies using hybrid neuro-symbolic matching:\n\
            - Source: {}\n\
            - Target: {}\n\n\
            Follow this pipeline:\n\n\
            **Step 1: Structural alignment**\n\
            Call onto_align with source, target, min_confidence=0.7, dry_run=true.\n\
            This returns candidates with confidence scores and signal breakdowns.\n\n\
            **Step 2: Auto-accept high-confidence matches**\n\
            Candidates with confidence >= 0.95 are reliable. List them as accepted.\n\n\
            **Step 3: LLM adjudication of uncertain pairs**\n\
            For candidates with confidence 0.7-0.95, YOU decide:\n\
            - Look at the source and target labels, their parent classes, and the signal breakdown\n\
            - Use your knowledge of the domain to judge if they refer to the same concept\n\
            - Accept the pair if they are genuinely equivalent; reject if they are false matches\n\
            - Example: \"levator auris longus\" (mouse muscle) <-> \"Auricularis\" (human muscle) = ACCEPT (same ear muscle, different species names)\n\
            - Example: \"tail\" <-> \"Tail_of_Pancreas\" = REJECT (different concepts despite shared word)\n\n\
            **Step 4: Apply accepted matches**\n\
            For each accepted pair (both auto-accepted and LLM-adjudicated), call onto_align_feedback with accepted=true.\n\
            For rejected pairs, call onto_align_feedback with accepted=false.\n\
            This trains the self-calibrating weights for future alignments.\n\n\
            **Step 5: Report**\n\
            Summarize: total candidates, auto-accepted, LLM-accepted, LLM-rejected, and final alignment count.",
            input.source_path, input.target_path
        );
        Ok(GetPromptResult::new(vec![
            PromptMessage::new_text(PromptMessageRole::User, msg),
        ]).with_description("Align two ontologies using hybrid neuro-symbolic matching (structural + LLM adjudication)"))
    }

    /// Explore a loaded ontology with SPARQL. Lists classes, properties, and answers competency questions.
    #[prompt(name = "explore_ontology")]
    fn explore_ontology(&self) -> Result<GetPromptResult, rmcp::ErrorData> {
        Ok(GetPromptResult::new(vec![
            PromptMessage::new_text(
                PromptMessageRole::User,
                "Explore the currently loaded ontology:\n\n\
                1. Call onto_stats to show overview counts\n\
                2. Call onto_query to list all classes with labels\n\
                3. Call onto_query to show the class hierarchy (subClassOf)\n\
                4. Call onto_query to list all properties with domains and ranges\n\
                5. Summarize the ontology structure and suggest competency questions it can answer",
            ),
        ]).with_description("Explore a loaded ontology with SPARQL"))
    }
}

// ─── ServerHandler ──────────────────────────────────────────────────────────

#[tool_handler]
#[prompt_handler]
impl ServerHandler for OpenOntologiesServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().enable_prompts().build())
            .with_instructions("Open Ontologies: AI-native ontology engine — RDF/OWL/SPARQL MCP server with 43 tools and 6 workflow prompts for ontology engineering, validation, comparison, alignment, data ingestion, and exploration.")
    }
}
