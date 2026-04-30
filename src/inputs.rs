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
    /// Database connection string (e.g. postgres://user:pass@host/db)
    pub connection: String,
    /// Base IRI for generated classes (default: http://example.org/db/)
    pub base_iri: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct OntoAlignInput {
    /// Source ontology: inline Turtle content or file path
    pub source: String,
    /// Target ontology: inline Turtle content or file path. If omitted, aligns against loaded store
    pub target: Option<String>,
    /// Minimum confidence threshold for auto-apply (default 0.85)
    pub min_confidence: Option<f64>,
    /// If true, return candidates only without inserting triples (default false)
    pub dry_run: Option<bool>,
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
