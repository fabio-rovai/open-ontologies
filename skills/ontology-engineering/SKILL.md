---
name: ontology-engineering
description: Build, validate, and govern RDF/OWL ontologies using the Open Ontologies MCP server. Use when the user asks to create, modify, query, or manage ontologies, knowledge graphs, or RDF data.
---

# Ontology Engineering Workflow

You have access to the Open Ontologies MCP server, which provides 50+ tools for AI-native ontology engineering backed by an in-memory Oxigraph triple store.

## Core Workflow

When building or modifying ontologies, follow this workflow. Decide which tools to call and in what order based on results -- this is not a fixed pipeline.

### 1. Generate

- Understand the domain requirements (natural language, competency questions, methodology constraints)
- Generate Turtle/OWL directly -- you know OWL, RDF, BORO, 4D modeling natively

### 2. Validate and Load

- Call `onto_validate` on the generated Turtle -- if it fails, fix syntax errors and re-validate
- Call `onto_load` to load into the Oxigraph triple store. For repos that mount a folder of `.ttl` files via `[general] ontology_dirs`, prefer `onto_repo_load` so the same compile-cache / TTL-eviction path is exercised. Use `onto_repo_list` to discover candidate files.
- Call `onto_stats` to verify class count, property count, triple count match expectations

### 3. Verify

- Call `onto_lint` to check for missing labels, comments, domains, ranges -- fix any issues found
- Call `onto_query` with SPARQL to verify structure (expected classes, subclass hierarchies, competency questions)
- If a reference ontology exists, call `onto_diff` to compare

### 4. Iterate

- If any step reveals problems, fix the Turtle and restart from step 2
- Continue until validation passes, stats match, lint is clean, and SPARQL queries return expected results

### 5. Persist

- Call `onto_save` to write the final ontology to a .ttl file
- Call `onto_version` to save a named snapshot for rollback

## Cache and Multi-Ontology Loading

The server keeps a single *active* ontology in memory plus an on-disk N-Triples
compile cache for everything it has parsed. Switch between several ontologies
without paying re-parse costs:

- `onto_repo_list` — enumerate `.ttl` / `.owl` / `.nt` / `.rdf` / `.nq` / `.trig` / `.jsonld` files configured under `[general] ontology_dirs`. Container-friendly: mount a host folder of TTL files and discover them at runtime without hardcoded paths.
- `onto_repo_load` — load by bare file stem, relative path, or absolute path inside a configured repo dir. Reuses the same compile-cache / TTL-eviction path as `onto_load`.
- `onto_cache_status` / `onto_cache_list` — inspect what is cached, what is currently active, and the effective `[cache]` configuration (TTL, auto_refresh, dir).
- `onto_cache_remove` — drop a cached entry by name (pass `delete_file=false` to keep the on-disk N-Triples for a later reload).
- `onto_unload` — drop the active ontology (or a specific cached entry by name) from memory; the on-disk cache is preserved unless `delete_cache=true`.
- `onto_recompile` — force a re-parse from source, ignoring the cache. Without `name`, recompiles the active ontology and reloads it; with `name`, rebuilds a non-active entry without disturbing the active in-memory store.

## Ontology Lifecycle (Terraform-style)

For evolving ontologies in production:

1. **Plan** — `onto_plan` shows added/removed classes, blast radius, risk score. Check `onto_lock` for protected IRIs.
2. **Enforce** — `onto_enforce` with a rule pack (`generic`, `boro`, `value_partition`, `hierarchy`) checks design pattern compliance.
3. **Apply** — `onto_apply` with mode `safe` (clear + reload) or `migrate` (add equivalentClass bridges).
4. **Monitor** — `onto_monitor` runs SPARQL watchers with threshold alerts. Use `onto_monitor_clear` if blocked.
5. **Drift** — `onto_drift` compares versions with rename detection and self-calibrating confidence.
6. **Lineage** — `onto_lineage` shows the full plan → enforce → apply → monitor → drift trail for the current session.

## Data Extension Workflow

When applying an ontology to external data:

1. `onto_map` — generate mapping config from data schema + loaded ontology
2. `onto_ingest` — parse a structured *file* (CSV, JSON, NDJSON, XML, YAML, XLSX, Parquet) into RDF
3. `onto_sql_ingest` — run a SQL query against PostgreSQL or DuckDB (via `postgres://`, `duckdb:///path.duckdb`, `:memory:`, or a `*.duckdb` file path) and ingest result rows. Use this when the source data lives in a database, or when you want to use DuckDB as a federation layer over remote Parquet/CSV/JSON via the `httpfs`, `postgres_scanner`, `iceberg`, etc. extensions.
4. `onto_import_schema` — introspect a PostgreSQL or DuckDB schema and generate OWL classes/properties/cardinality from tables/columns/PKs/FKs.
5. `onto_shacl` — validate against SHACL shapes
6. `onto_reason` — run RDFS or OWL-RL inference
7. Or use `onto_extend` to run the full file-based pipeline (ingest + SHACL + reason) in one call

## Reasoning and DL Explanation

- `onto_reason` — RDFS / OWL-RL forward-chaining materialisation
- `onto_dl_check` — check `subClass ⊑ superClass` using DL tableaux
- `onto_dl_explain` — return the clash trace explaining why a class is unsatisfiable

## Semantic Search and Embeddings

After loading, generate embeddings to enable natural-language search:

- `onto_embed` — generate text + Poincaré structural embeddings for every class. Honours `[embeddings] provider = "local" | "openai"` and the `OPEN_ONTOLOGIES_EMBEDDINGS_*` env vars.
- `onto_search` — natural-language query → most-similar classes (`mode: "text" | "structure" | "product"`).
- `onto_similarity` — compute cosine + Poincaré distance between two specific IRIs.

When embeddings exist, `onto_align` automatically uses them as a 7th alignment signal, catching semantically equivalent classes whose labels differ.

## Tool Reference

| Tool | When to use |
| ---- | ----------- |
| `onto_status` | Check that the server is running and how many triples are loaded |
| `onto_validate` | After generating or modifying Turtle -- always validate first |
| `onto_load` | Load Turtle/N-Triples/RDF-XML into the triple store |
| `onto_stats` | Sanity-check class / property / triple counts |
| `onto_lint` | Catch missing labels, comments, domains, ranges |
| `onto_query` | Verify structure, answer competency questions |
| `onto_diff` | Compare against a reference or previous version |
| `onto_save` | Persist the active ontology to a file |
| `onto_convert` | Convert between Turtle, N-Triples, RDF/XML, N-Quads, TriG |
| `onto_clear` | Reset the in-memory store |
| `onto_pull` | Fetch ontology from a remote URL or SPARQL endpoint |
| `onto_push` | Push triples to a SPARQL endpoint |
| `onto_import` | Resolve and load `owl:imports` chains |
| `onto_marketplace` | Browse / install standard ontologies from the curated catalogue |
| `onto_version` | Save a named snapshot before making changes |
| `onto_history` | List saved snapshots |
| `onto_rollback` | Restore a previous snapshot |
| `onto_unload` | Drop the active (or named) ontology from memory |
| `onto_recompile` | Re-parse the source, ignoring the on-disk compile cache |
| `onto_cache_status` | Inspect compile cache: active slot, all entries, `[cache]` config |
| `onto_cache_list` | Lighter version of cache status — list cached ontologies with metadata |
| `onto_cache_remove` | Remove a cached ontology by name |
| `onto_repo_list` | Enumerate RDF/OWL files in configured `[general] ontology_dirs` |
| `onto_repo_load` | Load by name / relative path / absolute path inside a configured repo dir |
| `onto_ingest` | Parse a file (CSV, JSON, NDJSON, XML, YAML, XLSX, Parquet) into RDF |
| `onto_sql_ingest` | **NEW** — SQL `SELECT` against PostgreSQL or DuckDB → RDF (uses the same mapping format as `onto_ingest`). DuckDB acts as a federation layer over CSV/Parquet/JSON/HTTPFS/postgres scanner via its extensions. |
| `onto_import_schema` | Introspect PostgreSQL or DuckDB schema → generate OWL |
| `onto_map` | Auto-generate mapping config from data schema + loaded ontology |
| `onto_shacl` | Validate against SHACL shapes |
| `onto_reason` | Run RDFS or OWL-RL inference |
| `onto_extend` | File-based convenience: ingest + SHACL + reason |
| `onto_dl_check` | Check `subClass ⊑ superClass` via DL tableaux |
| `onto_dl_explain` | Explain why a class is unsatisfiable (DL clash trace) |
| `onto_plan` | Show added/removed classes, blast radius, risk score |
| `onto_apply` | Apply changes in `safe` or `migrate` mode |
| `onto_lock` | Protect production IRIs from removal |
| `onto_drift` | Compare versions with rename detection |
| `onto_enforce` | Design pattern checks (`generic`, `boro`, `value_partition`, `hierarchy`) |
| `onto_monitor` | Run SPARQL watchers with threshold alerts |
| `onto_monitor_clear` | Clear blocked state after resolving alerts |
| `onto_lineage` | View session lineage trail |
| `onto_crosswalk` | Look up clinical terminology mappings (ICD-10, SNOMED, MeSH) |
| `onto_enrich` | Add `skos:exactMatch` triples linking classes to clinical codes |
| `onto_validate_clinical` | Check class labels against clinical crosswalk terminology |
| `onto_align` | Detect alignment candidates between two ontologies (uses embeddings if loaded) |
| `onto_align_feedback` | Accept/reject alignment candidates for self-calibrating weights |
| `onto_lint_feedback` | Accept/dismiss a lint issue (teaches lint to suppress repeated false positives) |
| `onto_enforce_feedback` | Accept/dismiss an enforce violation (same self-calibration mechanism) |
| `onto_embed` | Generate text + Poincaré structural embeddings for all classes |
| `onto_search` | Natural-language query → most-similar classes (text / structure / product) |
| `onto_similarity` | Cosine + Poincaré distance between two IRIs |

## Key Principle

Dynamically decide the next tool call based on what the previous tool returned. If `onto_validate` fails, fix and retry. If `onto_stats` shows wrong counts, regenerate. If `onto_lint` finds missing labels, add them. The MCP tools are individual operations -- you are the orchestrator.

