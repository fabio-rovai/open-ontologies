use std::sync::Arc;

use rmcp::{
    ServerHandler, tool, tool_handler, tool_router,
    handler::server::{tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo, Tool},
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::graph::GraphStore;
use crate::state::StateDb;

// ─── MCP tool input structs ─────────────────────────────────────────────────

#[derive(Deserialize, JsonSchema)]
pub struct OntoValidateInput {
    /// Path to an RDF file OR inline Turtle content
    pub input: String,
    /// If true, treat input as inline content rather than a file path
    pub inline: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoConvertInput {
    /// Path to source RDF file
    pub path: String,
    /// Target format: turtle, ntriples, rdfxml, nquads, trig
    pub to: String,
    /// Optional output file path (if omitted, returns content)
    pub output: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoLoadInput {
    /// Path to RDF file to load into the in-memory store
    pub path: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoQueryInput {
    /// SPARQL query string
    pub query: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoSaveInput {
    /// Output file path
    pub path: String,
    /// Format: turtle, ntriples, rdfxml, nquads, trig
    pub format: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoDiffInput {
    /// Path to the old/original ontology file
    pub old_path: String,
    /// Path to the new/modified ontology file
    pub new_path: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoLintInput {
    /// Path to RDF file to lint, OR inline Turtle content
    pub input: String,
    /// If true, treat input as inline content
    pub inline: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoPullInput {
    /// Remote URL or SPARQL endpoint to fetch ontology from
    pub url: String,
    /// If true, treat url as a SPARQL endpoint and run a CONSTRUCT query
    pub sparql: Option<bool>,
    /// Optional SPARQL CONSTRUCT query (required if sparql=true)
    pub query: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoPushInput {
    /// Remote SPARQL endpoint URL
    pub endpoint: String,
    /// Optional named graph IRI
    pub graph: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoImportInput {
    /// Resolve and load all owl:imports from the currently loaded ontology
    pub max_depth: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoVersionInput {
    /// Version label (e.g. "v1.0", "draft-2026-03-09")
    pub label: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoRollbackInput {
    /// Version label to restore
    pub label: String,
}

// ─── OpenOntologiesServer ───────────────────────────────────────────────────

/// MCP server that exposes all Open Ontologies tools to Claude via stdin/stdout.
#[derive(Clone)]
pub struct OpenOntologiesServer {
    tool_router: ToolRouter<Self>,
    db: StateDb,
    graph: Arc<GraphStore>,
}

impl OpenOntologiesServer {
    /// Create a new server with all tools wired to domain services.
    pub fn new(db: StateDb) -> Self {
        Self {
            tool_router: Self::tool_router(),
            db,
            graph: Arc::new(GraphStore::new()),
        }
    }

    /// Return the list of all registered tool definitions.
    pub fn list_tool_definitions(&self) -> Vec<Tool> {
        self.tool_router.list_all()
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

    #[tool(name = "onto_load", description = "Load an RDF file into the in-memory ontology store for querying")]
    async fn onto_load(&self, Parameters(input): Parameters<OntoLoadInput>) -> String {
        match self.graph.load_file(&input.path) {
            Ok(count) => format!(r#"{{"ok":true,"triples_loaded":{},"path":"{}"}}"#, count, input.path),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_query", description = "Run a SPARQL query against the loaded ontology store")]
    async fn onto_query(&self, Parameters(input): Parameters<OntoQueryInput>) -> String {
        self.graph.sparql_select(&input.query).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_save", description = "Save the current ontology store to a file")]
    async fn onto_save(&self, Parameters(input): Parameters<OntoSaveInput>) -> String {
        let format = input.format.as_deref().unwrap_or("turtle");
        match self.graph.save_file(&input.path, format) {
            Ok(_) => format!(r#"{{"ok":true,"path":"{}","format":"{}"}}"#, input.path, format),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        }
    }

    #[tool(name = "onto_stats", description = "Get statistics about the loaded ontology (triple count, classes, properties, individuals)")]
    fn onto_stats(&self) -> String {
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
        OntologyService::lint(&content).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_clear", description = "Clear all triples from the in-memory ontology store")]
    fn onto_clear(&self) -> String {
        match self.graph.clear() {
            Ok(_) => r#"{"ok":true,"message":"Store cleared"}"#.to_string(),
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
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
        if let Ok(result) = self.graph.sparql_select(query) {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result) {
                if let Some(results) = parsed["results"].as_array() {
                    for row in results {
                        if let Some(uri) = row["import"].as_str() {
                            let uri = uri.trim_matches(|c| c == '<' || c == '>');
                            to_import.push(uri.to_string());
                        }
                    }
                }
            }
        }

        let mut depth = 0;
        while !to_import.is_empty() && depth < max_depth {
            let batch = to_import.drain(..).collect::<Vec<_>>();
            for url in batch {
                if imported.contains(&url) { continue; }
                match GraphStore::fetch_url(&url).await {
                    Ok(content) => {
                        match self.graph.load_turtle(&content, None) {
                            Ok(_count) => {
                                imported.push(url.clone());
                                if let Ok(result) = self.graph.sparql_select(query) {
                                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&result) {
                                        if let Some(results) = parsed["results"].as_array() {
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
}

// ─── ServerHandler ──────────────────────────────────────────────────────────

#[tool_handler]
impl ServerHandler for OpenOntologiesServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("Open Ontologies: AI-native ontology engine — RDF/OWL/SPARQL MCP server")
    }
}
