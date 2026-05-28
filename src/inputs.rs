use schemars::JsonSchema;
use serde::Deserialize;

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
    /// Path to RDF file, OR inline Turtle/RDF content
    pub path: Option<String>,
    /// Inline Turtle content to load (alternative to path)
    pub turtle: Option<String>,
    /// Optional name for this ontology in the registry. Defaults to the file
    /// stem of `path`. When omitted for inline turtle, defaults to "default".
    pub name: Option<String>,
    /// When true, every subsequent read tool checks the source file's mtime
    /// and recompiles if it changed. Has no effect for inline turtle.
    pub auto_refresh: Option<bool>,
    /// When true, ignore the on-disk compile cache and re-parse from source.
    pub force_recompile: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoUnloadInput {
    /// When true, also delete the on-disk compile cache file.
    pub delete_cache: Option<bool>,
    /// Optional ontology name. When omitted, operates on the currently active
    /// ontology. When provided, targets that named cache entry — if it is the
    /// active slot the in-memory store is cleared; otherwise only the on-disk
    /// cache is touched (and only when `delete_cache` is true).
    pub name: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoRecompileInput {
    /// Optional ontology name. When omitted, recompiles the active ontology.
    /// When provided, recompiles that cached entry from its recorded source
    /// path; if the entry is not active, the active in-memory store is left
    /// untouched.
    pub name: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoRepoListInput {
    /// Optional subdirectory to scan instead of every configured ontology
    /// repo. Must resolve under one of the configured `ontology_dirs`
    /// entries; arbitrary host paths are rejected (path-traversal guard).
    pub dir: Option<String>,
    /// Walk subdirectories recursively. Defaults to false (top-level only).
    pub recursive: Option<bool>,
    /// Optional filename glob filter (e.g. `*.ttl`, `foo*`). Matches the
    /// filename only, not the full path.
    pub glob: Option<String>,
    /// Maximum number of entries to return. Default 1000.
    pub limit: Option<usize>,
    /// Skip the first `offset` entries (for pagination). Default 0.
    pub offset: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoRepoLoadInput {
    /// Identifier of the ontology to load. Accepts:
    ///   - a bare name (e.g. `pizza`) matching a file stem under any
    ///     configured `ontology_dirs`,
    ///   - a relative path (e.g. `subdir/pizza.ttl`) resolved against the
    ///     configured directories,
    ///   - an absolute path inside one of the configured directories.
    ///
    /// Paths outside the configured `ontology_dirs` are rejected.
    pub name: String,
    /// Optional registry name override (defaults to the file stem).
    pub registry_name: Option<String>,
    /// When true, every subsequent read tool checks the source file's mtime
    /// and recompiles if it changed.
    pub auto_refresh: Option<bool>,
    /// When true, ignore the on-disk compile cache and re-parse from source.
    pub force_recompile: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoCacheStatusInput {}

#[derive(Deserialize, JsonSchema)]
pub struct OntoCacheListInput {}

#[derive(Deserialize, JsonSchema)]
pub struct OntoCacheRemoveInput {
    /// Name of the cached ontology to remove.
    pub name: String,
    /// When true (default), also delete the on-disk N-Triples cache file.
    /// When false, only the metadata row is removed.
    pub delete_file: Option<bool>,
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

#[derive(Deserialize, JsonSchema)]
pub struct OntoIngestInput {
    /// Path to the data file (CSV, JSON, NDJSON, XML, YAML, XLSX, Parquet)
    pub path: String,
    /// Data format (auto-detected from extension if omitted): csv, json, ndjson, xml, yaml, xlsx, parquet
    pub format: Option<String>,
    /// Mapping config as JSON string or path to mapping JSON file
    pub mapping: Option<String>,
    /// If true, treat mapping as inline JSON (default: false = file path)
    pub inline_mapping: Option<bool>,
    /// Base IRI for generated instances (default: http://example.org/data/)
    pub base_iri: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoMapInput {
    /// Path to sample data file to generate mapping for
    pub data_path: String,
    /// Data format (auto-detected if omitted)
    pub format: Option<String>,
    /// Optional path to save the generated mapping config
    pub save_path: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoShaclInput {
    /// Path to SHACL shapes file OR inline SHACL Turtle content
    pub shapes: String,
    /// If true, treat shapes as inline Turtle content
    pub inline: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoShaclCheckInput {
    /// Path to SHACL shapes file OR inline SHACL Turtle content to dry-run-validate
    /// against the currently loaded ontology. Checks that the shapes parse and that
    /// every IRI they reference (`sh:targetClass`, `sh:path`, `sh:class`) actually
    /// exists in the ontology, plus a lightweight XSD-prefix check on `sh:datatype`.
    /// Does NOT apply or run the shapes — that's `onto_shacl`.
    pub shapes: String,
    /// If true, treat shapes as inline Turtle content (default false = file path).
    pub inline: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoReasonInput {
    /// Reasoning profile: rdfs (default), owl-rl
    pub profile: Option<String>,
    /// If true (default), add inferred triples to the store. If false, dry-run only.
    pub materialize: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoDlExplainInput {
    /// IRI of the class to explain unsatisfiability for
    pub class_iri: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoDlCheckInput {
    /// IRI of the sub-class (the more specific class)
    pub sub_class: String,
    /// IRI of the super-class (the more general class)
    pub super_class: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoExtendInput {
    /// Path to the data file
    pub data_path: String,
    /// Data format (auto-detected if omitted)
    pub format: Option<String>,
    /// Mapping config (inline JSON or file path)
    pub mapping: Option<String>,
    /// If true, treat mapping as inline JSON
    pub inline_mapping: Option<bool>,
    /// Base IRI for generated instances
    pub base_iri: Option<String>,
    /// Path to SHACL shapes file or inline Turtle
    pub shapes: Option<String>,
    /// If true, treat shapes as inline Turtle
    pub inline_shapes: Option<bool>,
    /// Reasoning profile (rdfs, owl-rl). Omit to skip reasoning.
    pub reason_profile: Option<String>,
    /// If true (default), stop pipeline on SHACL violations
    pub stop_on_violations: Option<bool>,
}

// ─── v2 input structs ───────────────────────────────────────────────────────

#[derive(Deserialize, JsonSchema)]
pub struct OntoPlanInput {
    /// New ontology as inline Turtle content
    pub new_turtle: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoApplyInput {
    /// Apply mode: "safe" (default), "force" (ignores monitor), "migrate" (adds bridges)
    pub mode: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoLockInput {
    /// IRIs to lock (prevent removal)
    pub iris: Vec<String>,
    /// Reason for locking
    pub reason: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoDriftInput {
    /// First version as inline Turtle
    pub version_a: String,
    /// Second version as inline Turtle
    pub version_b: String,
    /// Output format. One of: "json" (default, existing schema with added/removed/likely_renames),
    /// "kgcl" (KGCL Controlled Natural Language, one change per line),
    /// "kgcl_json" (KGCL changes as structured JSON-LD).
    #[serde(default)]
    pub format: Option<String>,
    /// Confidence threshold above which a likely_rename is emitted as a KGCL
    /// obsoletion-with-replacement instead of a plain add+remove pair. Default 0.7.
    /// Only consulted when `format` is "kgcl" or "kgcl_json".
    #[serde(default)]
    pub rename_threshold: Option<f64>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoEnforceInput {
    /// Rule pack to enforce: "generic", "boro", "value_partition", or custom pack name
    pub rule_pack: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoMonitorInput {
    /// Inline JSON array of watchers to add, or omit to just run existing watchers
    pub watchers: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoCrosswalkInput {
    /// Clinical code to look up (e.g. "I10")
    pub code: String,
    /// Source system (e.g. "ICD10", "SNOMED", "MeSH")
    pub source_system: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoEnrichInput {
    /// IRI of the ontology class to enrich
    pub class_iri: String,
    /// Clinical code to map to
    pub code: String,
    /// Code system (e.g. "ICD10")
    pub system: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoLineageInput {
    /// Session ID to query (omit for current session)
    pub session_id: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoImportSchemaInput {
    /// Database connection string. Supported:
    ///   - `postgres://user:pass@host/db` (requires `postgres` feature)
    ///   - `duckdb:///path/to/file.duckdb` or bare `/path/to/file.duckdb` (requires `duckdb` feature)
    ///   - `:memory:` for an in-memory DuckDB database (requires `duckdb` feature)
    pub connection: String,
    /// Base IRI for generated classes (default: http://example.org/db/)
    pub base_iri: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoSqlIngestInput {
    /// Database connection string. Same forms as `onto_import_schema`:
    /// `postgres://…`, `duckdb:///path/to.duckdb`, `:memory:`, or a bare
    /// `*.duckdb` file path.
    pub connection: String,
    /// SQL SELECT statement to run. Returned rows are converted to RDF using
    /// the supplied mapping (or an auto-generated one).
    pub sql: String,
    /// Mapping config as JSON string or path to a mapping JSON file.
    /// Same shape as `onto_ingest`. Optional — if omitted, an auto-mapping
    /// is generated from the column names.
    pub mapping: Option<String>,
    /// If true, treat `mapping` as inline JSON (default: false = file path).
    pub inline_mapping: Option<bool>,
    /// Base IRI for generated instances (default: http://example.org/data/)
    pub base_iri: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoAlignInput {
    /// Source ontology: inline Turtle content or file path
    pub source: String,
    /// Target ontology: inline Turtle content or file path. If omitted, aligns against loaded store
    pub target: Option<String>,
    /// Minimum confidence threshold for auto-apply (default 0.85). Back-compat alias for
    /// `high_threshold` — if both are set, `high_threshold` wins.
    pub min_confidence: Option<f64>,
    /// Confidence threshold above which a candidate is auto-applied (default 0.85, or
    /// `min_confidence` if provided for back-compat). Candidates above this land in
    /// `auto_applied`.
    pub high_threshold: Option<f64>,
    /// Confidence threshold below which a candidate is dropped entirely (default 0.4).
    /// Candidates in [low_threshold, high_threshold] are surfaced in `borderline` with
    /// enriched context (parents, siblings, labels) so the calling LLM can judge them
    /// and record verdicts via `onto_align_feedback`.
    pub low_threshold: Option<f64>,
    /// If true, return candidates only without inserting triples (default false)
    pub dry_run: Option<bool>,
    /// Fusion strategy for combining the per-signal scores into a confidence score.
    /// One of "weighted_sum" (default — learned weights over the 7 signals, cold-start
    /// equal-weighted) or "rrf" (Reciprocal Rank Fusion at k=60, validated by Agent-OM
    /// at VLDB 2025). RRF doesn't need learned weights so it's a sensible cold-start
    /// choice; the weighted_sum self-calibrates from `onto_align_feedback` over time.
    #[serde(default)]
    pub fusion: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoAlignFeedbackInput {
    /// Source class IRI from the alignment candidate
    pub source_iri: String,
    /// Target class IRI from the alignment candidate
    pub target_iri: String,
    /// Whether the alignment candidate was correct
    pub accepted: bool,
    /// Signal values from the alignment candidate (copied from the "signals" field in align output)
    pub signals: Option<std::collections::HashMap<String, f64>>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoLintFeedbackInput {
    /// The lint rule ID (e.g. "missing_label", "missing_comment", "missing_domain", "missing_range")
    pub rule_id: String,
    /// The entity IRI that triggered the lint issue
    pub entity: String,
    /// true = this is a real issue, false = dismiss/ignore
    pub accepted: bool,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoEnforceFeedbackInput {
    /// The enforce rule ID (e.g. "orphan_class", "missing_domain", "missing_range", "missing_label", or custom rule ID)
    pub rule_id: String,
    /// The entity IRI that triggered the violation
    pub entity: String,
    /// true = this is a real violation, false = dismiss/override
    pub accepted: bool,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoEmbedInput {
    /// Structural embedding dimension. Default: 32
    pub struct_dim: Option<usize>,
    /// Structural training epochs. Default: 100
    pub struct_epochs: Option<usize>,
    /// Optional map from class IRI to a free-text description used for text-embedding
    /// in place of the class's rdfs:label. When set, classes present in the map are
    /// embedded from their description (richer semantic context); classes absent from
    /// the map fall back to the existing label-based embedding. This is the
    /// MCP-native form of the GenOM pattern (Mensa et al. 2025, accepted World Wide
    /// Web Journal): instead of the server calling an LLM to author descriptions, the
    /// connected orchestrator (Claude) authors them in-conversation and passes them
    /// in this map. Net new dependencies: zero.
    pub descriptions: Option<std::collections::HashMap<String, String>>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoHnswBuildInput {
    /// HNSW `ef_construction` — size of the dynamic candidate list during
    /// graph construction. Higher values yield better recall at the cost of
    /// slower build. instant-distance's default (100) is a sensible starting
    /// point; tune upward (e.g. 200-400) for high-recall workloads.
    pub ef_construction: Option<usize>,
    /// HNSW `ef_search` — size of the dynamic candidate list during query.
    /// Higher values yield better recall per query at the cost of slower
    /// search. instant-distance's default works for most ontologies; raise
    /// (e.g. 100-200) when search recall matters more than latency.
    pub ef_search: Option<usize>,
    /// When true (default), persist the built index to SQLite so subsequent
    /// process restarts can skip the rebuild via `VecStore::load_cosine_index`.
    /// Set false for ephemeral one-shot builds.
    #[serde(default)]
    pub persist: Option<bool>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoSearchInput {
    /// Natural language query
    pub query: String,
    /// Number of results. Default: 10
    pub top_k: Option<usize>,
    /// Search mode: "text", "structure", or "product". Default: "product"
    pub mode: Option<String>,
    /// Weight for text vs structure in product mode (0.0-1.0). Default: 0.5
    pub alpha: Option<f32>,
    /// When true (text mode only), route the search through the HNSW cosine
    /// index instead of the brute-force linear scan. Recommended for ontologies
    /// with more than a few hundred classes. Default: false.
    pub use_hnsw: Option<bool>,
    /// Optional HNSW `ef_search` override. When provided AND `use_hnsw` is true,
    /// the index is rebuilt with this `ef_search` before searching.
    /// **Caveat:** `instant-distance` bakes `ef_search` into the HNSW index at
    /// build time and does not support per-query overrides, so changing this
    /// value triggers a rebuild. Prefer setting `ef_search` once via
    /// `onto_hnsw_build` if you query frequently with the same value.
    pub ef_search: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoSimilarityInput {
    /// First IRI
    pub iri_a: String,
    /// Second IRI
    pub iri_b: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoMarketplaceInput {
    /// Action: "list" to browse available ontologies, "install" to fetch and load one
    pub action: String,
    /// Ontology ID to install (e.g. "prov-o", "schema-org", "foaf"). Required for "install".
    pub id: Option<String>,
    /// Filter list by domain (e.g. "foundational", "metadata", "iot", "geospatial")
    pub domain: Option<String>,
}

// ─── Prompt input structs ───────────────────────────────────────────────────

#[derive(Deserialize, JsonSchema)]
pub struct BuildOntologyInput {
    /// Description of the domain to model (e.g. "A pizza ontology with toppings, bases, and named pizzas")
    pub domain: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct ValidateOntologyInput {
    /// Path to the ontology file to validate
    pub path: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct CompareOntologiesInput {
    /// Path to the old/original ontology file
    pub old_path: String,
    /// Path to the new/modified ontology file
    pub new_path: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct IngestDataInput {
    /// Path to the data file (CSV, JSON, NDJSON, XML, YAML, XLSX, Parquet)
    pub data_path: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct AlignOntologiesInput {
    /// Path to the source ontology file
    pub source_path: String,
    /// Path to the target ontology file
    pub target_path: String,
}

/// Input for the CIVeX-style action certification tool (`onto_certify_action`).
/// Mirrors the paper's action frame structure plus the policy thresholds needed
/// for the triage step. See `src/civex.rs` for the full semantic.
#[derive(Deserialize, JsonSchema)]
pub struct OntoCertifyActionInput {
    /// Name of the state-changing onto_* tool whose execution this gates.
    pub tool: String,
    /// The IRIs being targeted by the proposed change.
    pub target_iris: Vec<String>,
    /// The proposed change as Turtle.
    pub proposed_delta_ttl: String,
    /// Utility metric name. One of "dependent_query_pass_rate" (default — caller
    /// supplies `dependent_queries`), "triple_count_delta", "class_count_delta",
    /// "property_count_delta".
    #[serde(default = "default_utility_metric")]
    pub utility_metric: String,
    /// SPARQL queries that should remain answerable post-change. Used when
    /// `utility_metric == "dependent_query_pass_rate"`.
    #[serde(default)]
    pub dependent_queries: Vec<String>,
    /// Cost budget (triples-affected). Action is REJECTED if cost > this.
    pub cost_threshold: u64,
    /// Utility threshold for EXECUTE. LCB must clear this.
    pub utility_threshold: f64,
    /// Risk threshold. Hard reject if cost exceeds this (even within budget).
    pub risk_threshold: u64,
    /// Whether the action is reversible.
    pub reversible: bool,
    /// Authorise the EXPERIMENT verdict (caller commits to running a sandbox replay).
    #[serde(default)]
    pub allow_experiment: bool,
    /// One-sided confidence level α for the LCB. Default 0.05.
    #[serde(default = "default_alpha_pub")]
    pub alpha: f64,
    /// Optional name of a registered Dynamics action schema (#43) — echoed
    /// into the certificate's assumptions so the audit trail is explicit
    /// about which action was certified.
    #[serde(default)]
    pub action_schema_name: Option<String>,
    /// Identification mode (#48, v0.5). One of `"structural"` (default,
    /// the v0.4 behaviour) or `"do_calculus_backdoor"`. The latter only
    /// takes effect when the server was built with the `causal-pywhy`
    /// Cargo feature; otherwise the verifier silently falls back to
    /// structural identification and records the reason in assumptions.
    #[serde(default)]
    pub identification_mode: Option<String>,
}

fn default_utility_metric() -> String {
    "dependent_query_pass_rate".to_string()
}

fn default_alpha_pub() -> f64 {
    0.05
}

/// Input for `graph_projection_lossy_check` (#35) — audits whether a projected
/// Turtle slice has dropped predicates/objects vs the full source neighbourhood
/// of the seed IRIs.
#[derive(Deserialize, JsonSchema)]
pub struct GraphProjectionLossyCheckInput {
    /// Seed IRIs whose neighbourhoods should be preserved by the projection.
    pub source_iris: Vec<String>,
    /// The projected Turtle slice that's being passed to a downstream consumer.
    pub projected_ttl: String,
}

// ─── Dynamics layer (#43) — action schemas, applicability, apply ────────────

/// Input for `onto_action_register` — persist an action schema by name.
/// Schema is supplied as inline JSON matching `dynamics::ActionSchema`.
#[derive(Deserialize, JsonSchema)]
pub struct OntoActionRegisterInput {
    /// Inline JSON for the action schema. Must deserialize into the structure:
    /// `{ "name": "...", "parameters": [...], "preconditions": [...],
    ///    "effects": [{"kind": "add_triple"|"remove_triple"|"add_class", ...}],
    ///    "reversible": true|false, "description": "..." }`.
    pub schema_json: String,
}

/// Input for `onto_action_applicable` — evaluate a registered action's
/// preconditions against the loaded graph under the given parameter bindings.
#[derive(Deserialize, JsonSchema)]
pub struct OntoActionApplicableInput {
    /// Name of a previously registered action schema.
    pub action_name: String,
    /// Bindings: `{ "param_name": "iri-or-literal" }`. Substituted into
    /// precondition SPARQL via `{param_name}` placeholders.
    pub bindings: std::collections::BTreeMap<String, String>,
}

/// Input for `onto_action_apply` — execute a registered action's effects,
/// returning the KGCL patch and an IES4-style event IRI.
#[derive(Deserialize, JsonSchema)]
pub struct OntoActionApplyInput {
    /// Name of a previously registered action schema.
    pub action_name: String,
    /// Parameter bindings (same shape as in `onto_action_applicable`).
    pub bindings: std::collections::BTreeMap<String, String>,
    /// If `true` (default), re-check preconditions before applying and abort
    /// if they don't hold. Set `false` only if you have already certified the
    /// action via `onto_certify_action`.
    #[serde(default = "default_true")]
    pub check_preconditions: bool,
    /// Ramification (#47): if `Some(profile)`, run the reasoner after `apply`
    /// to materialise downstream entailments. Accepted profiles: `"rdfs"`,
    /// `"owl-rl"`, `"owl-rl-ext"`, `"owl-dl"`. `None` (default) skips
    /// ramification — the literal effects land and that's it.
    #[serde(default)]
    pub ramify: Option<String>,
    /// Non-deterministic outcomes (#49): when the registered schema has a
    /// non-empty `outcomes` list, this seed makes the sample reproducible.
    /// `None` (default) uses `SystemTime::now()` for non-reproducible
    /// sampling. Ignored for deterministic schemas (empty `outcomes`).
    #[serde(default)]
    pub seed: Option<u64>,
}

fn default_true() -> bool {
    true
}

/// Input for `onto_plan_classical` (#50, Planner #45 follow-up) — invoke
/// Fast Downward as a subprocess on a precompiled PDDL domain + problem and
/// return the parsed sas_plan.
#[derive(Deserialize, JsonSchema)]
pub struct OntoPlanClassicalInput {
    /// PDDL domain text (as produced by `onto_plan_compile_pddl`).
    pub domain: String,
    /// PDDL problem text (as produced by `onto_plan_compile_pddl`).
    pub problem: String,
    /// Optional path to the Fast Downward binary (or wrapper script).
    /// Resolution order: this field > `FAST_DOWNWARD_BIN` env var >
    /// `fast-downward.py` on PATH.
    #[serde(default)]
    pub fast_downward_bin: Option<String>,
    /// Fast Downward search-engine string (e.g. `"lama-first"`,
    /// `"astar(lmcut())"`). Default `"lama-first"`.
    #[serde(default)]
    pub search: Option<String>,
}

/// Input for `onto_plan_validate` (#45 — LLM-Modulo validator) — check that
/// a candidate plan (typically produced by a client-side solver) actually
/// executes step-by-step against the loaded graph in a sandbox, without
/// mutating the real store.
#[derive(Deserialize, JsonSchema)]
pub struct OntoPlanValidateInput {
    /// Candidate plan: an ordered list of `{action_name, bindings}` steps.
    /// `bindings` is a `{param_name: iri-or-literal}` map.
    pub steps: Vec<PlanStepInput>,
    /// Optional goal triples. Each is `[s, p, o]` in N-Triple-position form
    /// (e.g. `["<http://ex.org/Cat>", "<http://www.w3.org/2000/01/rdf-schema#subClassOf>", "<http://ex.org/Animal>"]`).
    /// Reported in `unsatisfied_goals` if any goal does not hold post-plan.
    #[serde(default)]
    pub goal_facts: Vec<Vec<String>>,
}

#[derive(Deserialize, JsonSchema)]
pub struct PlanStepInput {
    pub action_name: String,
    #[serde(default)]
    pub bindings: std::collections::BTreeMap<String, String>,
}

/// Input for `onto_plan_compile_pddl` (#45 — Planner v0.6 stub) — emit a PDDL
/// domain from registered action schemas plus a problem instance from the
/// current graph + goal triples.
#[derive(Deserialize, JsonSchema)]
pub struct OntoPlanCompilePddlInput {
    /// Domain name to embed in `(define (domain ...))`. Default `"ontology"`.
    #[serde(default)]
    pub domain_name: Option<String>,
    /// Subset of registered action schema names to include. Omit / empty for
    /// "include every registered schema".
    #[serde(default)]
    pub action_names: Vec<String>,
    /// Goal triples in Turtle that must hold in the post-state.
    #[serde(default)]
    pub goal_ttl: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn onto_embed_input_accepts_optional_descriptions_map() {
        // GenOM enrichment: caller supplies {iri → description} mappings.
        // The input must deserialize when the map is provided.
        let json = serde_json::json!({
            "struct_dim": 32,
            "descriptions": {
                "http://ex.org/Cat": "A domestic feline, kept as a companion animal.",
                "http://ex.org/Dog": "A domestic canid, kept as a companion or working animal."
            }
        });
        let parsed: OntoEmbedInput = serde_json::from_value(json).expect("deserialize");
        let desc = parsed.descriptions.expect("descriptions field present");
        assert_eq!(desc.len(), 2);
        assert!(desc.get("http://ex.org/Cat").unwrap().contains("feline"));
    }

    #[test]
    fn onto_embed_input_descriptions_default_is_none() {
        // Existing callers that don't pass `descriptions` must still
        // deserialize correctly (back-compat).
        let json = serde_json::json!({});
        let parsed: OntoEmbedInput = serde_json::from_value(json).expect("deserialize");
        assert!(parsed.descriptions.is_none());
        assert!(parsed.struct_dim.is_none());
    }
}
