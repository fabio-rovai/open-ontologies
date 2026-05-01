# Changelog

All notable changes to Open Ontologies are documented here.

## [Unreleased]

### Added
- **DuckDB SQL data backbone**. New optional `duckdb` Cargo feature (and `sql` umbrella combining `postgres` + `duckdb`) wires DuckDB in alongside PostgreSQL as a *data integration* backbone — not as a SPARQL parser. DuckDB's extensions (`httpfs`, `parquet`, `csv`, `json`, `postgres_scanner`, `iceberg`, `delta`, …) let one SQL query federate over remote files, object stores, and other databases; rows then flow into the existing mapping/SHACL/reason pipeline.
- **New MCP tool `onto_sql_ingest`** — runs a SQL `SELECT` against PostgreSQL or DuckDB and ingests result rows into the triple store using the same `MappingConfig` shape as `onto_ingest`. Connection-string scheme is auto-detected (`postgres://`, `postgresql://`, `duckdb://`, `:memory:`, or a `*.duckdb` / `*.ddb` file path).
- **New CLI command `sql-ingest`** mirroring the MCP tool, with `--mapping`, `--inline-mapping`, `--base-iri`, and `-` (stdin) for the SQL.
- **`onto_import_schema` extended to DuckDB**. The same MCP tool / `import-schema` CLI now dispatches on the connection-string scheme: PostgreSQL via `sqlx`, DuckDB via the `duckdb` crate's `information_schema` + `duckdb_constraints()` introspection. The generated OWL is identical in shape (classes, datatype/object properties, NOT NULL → `owl:minCardinality 1`).
- **New `sql` tool group** in `[tools]` filter (`@sql` expands to `onto_import_schema` + `onto_sql_ingest`).
- **`SchemaIntrospector::sql_to_xsd` extended** to handle DuckDB-native types (HUGEINT, U{TINY,SMALL,}INT, DOUBLE, parameterised DECIMAL/VARCHAR, DATETIME, UUID, TIME).
- New tests: `tests/sqlsource_test.rs` (driver detection, no features required) and `tests/duckdb_test.rs` (introspection + query → row extraction, gated by the `duckdb` feature).

### Documentation
- `docs/data-pipeline.md` rewritten to cover both file-based and SQL-based ingest paths, the supported connection-string forms, federation examples (Parquet on S3 + Postgres scanner + remote CSV in one query), and a build matrix for the new feature flags.
- `SKILL.md`, `skills/ontology-engineering/SKILL.md`, `skills/ontology-engineer.md`, and `CLAUDE.md` Tool Reference tables expanded to cover the SQL backbone tools and previously-missing tools (`onto_status`, `onto_marketplace`, `onto_unload`, `onto_recompile`, `onto_cache_status`, `onto_cache_list`, `onto_cache_remove`, `onto_repo_list`, `onto_repo_load`, `onto_embed`, `onto_search`, `onto_similarity`, `onto_dl_explain`, `onto_dl_check`, `onto_import_schema`, `onto_sql_ingest`).

## [0.1.13] - 2026-05-01

### Added
- **Compile cache + TTL eviction + tool-exposure filter** (PR #1). Parsed ontologies are serialized to N-Triples on disk and reused on subsequent loads. A background evictor unloads idle ontologies after `[cache] idle_ttl_secs` (alias `unload_timeout_secs`); the on-disk cache is preserved and reloaded transparently on the next query. New `[tools]` config and `--tools-allow` / `--tools-deny` CLI flags restrict which `onto_*` tools the MCP server advertises (groups: `read_only`, `mutating`, `governance`, `remote`, `embeddings`).
- **New MCP tools**: `onto_cache_status`, `onto_cache_list`, `onto_cache_remove`, plus optional `name` parameter on `onto_unload` / `onto_recompile` for per-name cache management.
- **Ontology repository directories** (PR #2). New `[general] ontology_dirs` config (alias `data_dirs`) and `OPEN_ONTOLOGIES_ONTOLOGY_DIRS` env var let containerized deployments mount a folder of ontologies. Two new MCP tools enumerate and load from those directories with path-traversal guards: `onto_repo_list`, `onto_repo_load`.
- **OpenAI-compatible embeddings provider** (PR #3). New `[embeddings] provider = "openai"` mode targets any OpenAI-compatible gateway (official OpenAI, Azure, Ollama, vLLM, LocalAI, LM Studio, Together, …). Config fields: `api_base` (alias `base_url`), `api_key`, `model`, `dimensions`, `request_timeout_secs`. Env-var precedence: `OPEN_ONTOLOGIES_EMBEDDINGS_*` > `OPENAI_API_KEY` (for the key) > config > defaults. Remote responses are L2-normalized to remain comparable with local ONNX embeddings.
- **Surfaced operational config** (PR #4). New `[webhook]`, `[http]`, `[monitor]`, `[reasoner]`, `[feedback]`, `[imports]`, `[repo]`, `[socket]`, `[logging]` config sections expose previously hardcoded limits (tableaux depth/nodes, RDFS/OWL-RL fixpoint iterations, monitor interval, webhook timeout, import depth and remote-follow policy, feedback suppress/downgrade thresholds, etc.). A `0` value in the timeout / iteration fields is a sentinel that falls back to the documented default.
- New tests: `tests/registry_test.rs`, `tests/cache_management_test.rs`, `tests/toolfilter_test.rs`, `tests/repo_test.rs`, plus inline tests for embeddings config parsing and runtime knob initialization.

### Documentation
- New `docs/cache-and-registry.md` covering the compile cache, TTL eviction, tool-exposure filter, and ontology repository directories.
- `docs/embeddings.md` expanded with the OpenAI-compatible provider, supported gateways, config block, and env-var precedence.
- `CLAUDE.md` and `SKILL.md` Tool Reference tables updated with the seven new tools.

## [0.1.12] - 2026-03-27

### Added
- Virtualized tree view replacing D3/3D graph (handles 1500+ classes)
- Hierarchy connector lines, breadcrumb, and connections panel
- 13-step deep builder (`/build` command) producing IES-level ontologies
- `/sketch` command for quick prototyping
- `rdfs:Class` and `rdf:Property` support in Studio (not just `owl:Class`)
- Shared cargo target directory

### Fixed
- Static Linux binary via musl target (closes #2)

## [0.1.11] - 2026-03-25

### Added
- IES marketplace presets (`ies-top`, `ies-core`, `ies`)
- IES Building Extension (525 classes, clean-room)
- RDFS inference depth benchmark (662 vs 621)
- Head-to-head IRIS comparison
- Hierarchy enforce rule pack
- EPC benchmark (36/36 vs 18/36)

### Changed
- Default features off (lean build — drops tract-onnx and sqlx from default)

## [0.1.10] - 2026-03-13

### Added
- Quickstart guide (`docs/quickstart.md`)
- Server round-trip integration test (`tests/server_roundtrip_test.rs`)
- Complete architecture table in CONTRIBUTING.md (26 modules)

### Fixed
- Inconsistent CLI output: version/history/rollback/enrich/validate-clinical now respect `--pretty`
- CONTRIBUTING.md architecture table missing 10 modules (error, config, inputs, lineage, mapping, state, schema, embed, structembed)

## [0.1.9] - 2026-03-13

### Added
- Embedding similarity as alignment signal #7 (`onto_align` now uses text+structural embeddings when available)
- `onto_embed`, `onto_search`, `onto_similarity` MCP tools for semantic search
- End-to-end embedding pipeline test
- Embedding tools in architecture diagram and workflow documentation

### Fixed
- Feature gating for `tool_router` macro, clippy warnings, and tokenizer download
- Linux binary now built on ubuntu-22.04 for wider glibc compatibility

## [0.1.8] - 2026-03-12

### Added
- Poincare structural embedding trainer (Riemannian SGD for hierarchy layout)
- ONNX text embedder with tract (bge-small-en-v1.5, downloaded on init)
- Dual-space vector store with cosine + Poincare search and SQLite persistence
- Poincare ball geometry module (distance, exp_map, Riemannian SGD)

### Fixed
- Release binary naming now includes target triple
- Replaced deprecated macos-13 runner with macos-14

## [0.1.6] - 2026-03-11

### Added
- Glama server metadata and author verification

### Fixed
- Docker runtime libs and removed init from Dockerfile

## [0.1.5] - 2026-03-11

### Fixed
- Added build-essential and clang to Docker builder for oxrocksdb-sys compilation

## [0.1.4] - 2026-03-11

### Fixed
- Installed OpenSSL and libpq dev headers in Docker builder stage

## [0.1.3] - 2026-03-10

### Fixed
- Use latest Rust image in Dockerfile (dependencies need Rust 1.88+)

## [0.1.2] - 2026-03-10

### Fixed
- Free disk space in Docker workflow and optimize build
- Bumped server.json to v0.1.1

## [0.1.1] - 2026-03-09

### Added
- MCP Registry server.json, Docker publish workflow, and OCI label
- Streamable HTTP transport (`serve-http` command)
- MCP prompts (build_ontology, validate_ontology, compare_ontologies, ingest_data, explore_ontology)
- Dockerfile for containerized deployment
- OntoAxiom benchmark showdown (tool-augmented vs bare LLMs)
- Claude Code plugin package and ClawHub skill wrapper
- Bare Claude and hybrid benchmarks for three-way comparison
- Self-calibrating feedback for lint and enforce (dismiss 3x to suppress)
- Ontology alignment (`onto_align`, `onto_align_feedback`) with 6 weighted signals
- Terraform-style lifecycle: plan, apply, lock, drift, enforce, monitor, lineage
- Data pipeline: ingest, map, SHACL validate, reason, extend
- Clinical crosswalks (ICD-10, SNOMED, MeSH)
- OWL2-DL SHOIQ tableaux reasoner with parallel classification
- Design pattern enforcement (generic, BORO, value_partition)
- Version snapshots and rollback
- Core ontology tools: validate, load, save, query, stats, diff, lint, convert, clear, pull, push, import

### Fixed
- Clippy `io_other_error` warning breaking CI
- MCP benchmark scoring (camelCase normalization, pair order)
