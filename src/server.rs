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
    #[cfg(feature = "embeddings")]
    vecstore: Arc<std::sync::Mutex<crate::vecstore::VecStore>>,
    #[cfg(feature = "embeddings")]
    text_embedder: Option<Arc<crate::embed::TextEmbedder>>,
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
        let lineage = crate::lineage::LineageLog::with_governance_webhook(db.clone(), governance_webhook.clone());
        let session_id = lineage.new_session();

        #[cfg(feature = "embeddings")]
        let (vecstore, text_embedder) = {
            let mut vs = crate::vecstore::VecStore::new(db.clone());
            let _ = vs.load_from_db();

            let model_dir = dirs::home_dir()
                .map(|h| h.join(".open-ontologies/models"));
            let embedder = model_dir.and_then(|dir| {
                let model_path = dir.join("bge-small-en-v1.5.onnx");
                let tokenizer_path = dir.join("tokenizer.json");
                if model_path.exists() && tokenizer_path.exists() {
                    crate::embed::TextEmbedder::load(&model_path, &tokenizer_path).ok()
                } else {
                    None
                }
            });
            (
                Arc::new(std::sync::Mutex::new(vs)),
                embedder.map(Arc::new),
            )
        };

        Self {
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
            db,
            graph,
            session_id,
            governance_webhook,
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

    #[tool(name = "onto_load", description = "Load an RDF file or inline Turtle content into the in-memory ontology store for querying")]
    async fn onto_load(&self, Parameters(input): Parameters<OntoLoadInput>) -> String {
        if let Some(turtle) = input.turtle {
            match self.graph.load_turtle(&turtle, None) {
                Ok(count) => format!(r#"{{"ok":true,"triples_loaded":{},"source":"inline"}}"#, count),
                Err(e) => format!(r#"{{"error":"{}"}}"#, e),
            }
        } else if let Some(path) = input.path {
            let path = expand_tilde(&path);
            match self.graph.load_file(&path) {
                Ok(count) => format!(r#"{{"ok":true,"triples_loaded":{},"path":"{}"}}"#, count, path),
                Err(e) => format!(r#"{{"error":"{}"}}"#, e),
            }
        } else {
            r#"{"error":"Either 'path' or 'turtle' must be provided"}"#.to_string()
        }
    }

    #[tool(name = "onto_query", description = "Run a SPARQL query against the loaded ontology store")]
    async fn onto_query(&self, Parameters(input): Parameters<OntoQueryInput>) -> String {
        self.graph.sparql_select(&input.query).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
    }

    #[tool(name = "onto_save", description = "Save the current ontology store to a file")]
    async fn onto_save(&self, Parameters(input): Parameters<OntoSaveInput>) -> String {
        let format = input.format.as_deref().unwrap_or("turtle");
        let path = expand_tilde(&input.path);
        match self.graph.save_file(&path, format) {
            Ok(_) => format!(r#"{{"ok":true,"path":"{}","format":"{}"}}"#, path, format),
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
        OntologyService::lint_with_feedback(&content, Some(&self.db)).unwrap_or_else(|e| format!(r#"{{"error":"{}"}}"#, e))
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

    #[tool(name = "onto_marketplace", description = "Browse and install standard ontologies from a curated catalogue of 29 W3C/ISO/industry standards. Actions: 'list' (browse catalogue, optional domain filter) or 'install' (fetch and load by ID)")]
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

    #[tool(name = "onto_crosswalk", description = "Look up clinical crosswalk mappings for a code and system (ICD10, SNOMED, MeSH). Requires data/crosswalks.parquet.")]
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
        match engine.record_feedback(&input.source_iri, &input.target_iri, "user_feedback", input.accepted) {
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

        let mut vecstore = self.vecstore.lock().unwrap();
        let mut embedded_count = 0;
        let mut errors: Vec<String> = Vec::new();

        for (iri, label) in &class_labels {
            match embedder.embed(label) {
                Ok(text_vec) => {
                    let struct_vec = struct_embeddings.get(iri)
                        .cloned()
                        .unwrap_or_else(|| vec![0.0; struct_dim]);
                    vecstore.upsert(iri, &text_vec, &struct_vec);
                    embedded_count += 1;
                }
                Err(e) => errors.push(format!("{}: {}", iri, e)),
            }
        }

        if let Err(e) = vecstore.persist() {
            return format!(r#"{{"error":"failed to persist embeddings: {}"}}"#, e);
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

        let query_vec = match embedder.embed(&input.query) {
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
            .with_instructions("Open Ontologies: AI-native ontology engine — RDF/OWL/SPARQL MCP server with 39 tools and 5 workflow prompts for ontology engineering, validation, comparison, data ingestion, and exploration.")
    }
}
