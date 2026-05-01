---
name: ontology-engineer
description: Use when building, modifying, or validating ontologies. Orchestrates the onto_* MCP tools in a generate-validate-iterate loop. Triggers on "ontology", "OWL", "RDF", "Turtle", "SPARQL", "BORO", "4D modeling", or /ontology-engineer.
---

# Ontology Engineer

AI-native ontology engineering using OpenCheir's `onto_*` MCP tools (50+ tools).

## When to Use

- User asks to build, extend, or modify an ontology
- User asks to validate, lint, or query an existing ontology
- User mentions OWL, RDF, Turtle, SPARQL, BORO, 4D modeling, IES4
- User wants to compare ontologies or run competency questions
- User needs to ingest tabular data, SQL query results, or a database schema into RDF
- User wants to run reasoning (RDFS / OWL-RL) or DL satisfiability checks

## Prerequisites

The `onto_*` tools must be available via the OpenCheir MCP server. If they're not available, tell the user to install OpenCheir.

## Workflow

Claude dynamically decides which tool to call next based on results. This is NOT a fixed pipeline — adapt based on what each tool returns.

### Phase 1: Understand

- What domain? (Pizza, buildings, healthcare, etc.)
- What methodology? (standard OWL, BORO/4D, SKOS, etc.)
- What are the competency questions? (what should the ontology be able to answer?)
- Is there a reference ontology to compare against?
- Is the source data a file, a SQL database (Postgres / DuckDB), or a federated SQL query?

### Phase 2: Generate or Import

- For greenfield: generate Turtle/OWL directly from domain knowledge.
- For an existing relational schema: call `onto_import_schema` with a Postgres or DuckDB connection string — it generates OWL classes, datatype/object properties, and cardinality from tables/columns/PKs/FKs.
- For an existing on-disk corpus: configure `[general] ontology_dirs` and use `onto_repo_list` / `onto_repo_load` to discover and load.

### Phase 3: Validate (loop until clean)

```
onto_validate  →  syntax errors?  →  fix Turtle, re-validate
onto_load      →  loaded ok?      →  proceed to verification
onto_stats     →  counts match?   →  if not, regenerate missing parts
onto_lint      →  issues found?   →  fix labels/domains/ranges, re-load
```

### Phase 4: Verify (loop until correct)

```
onto_query     →  run SPARQL to check structure / competency questions
onto_diff      →  compare vs a reference if one exists
onto_dl_check  →  spot-check tricky subClass relations via DL tableaux
onto_dl_explain →  if a class is unsatisfiable, get the clash trace
```

### Phase 5: Apply Data (optional)

```
onto_map        →  auto-generate a mapping from sample data + ontology
onto_ingest     →  file-based: CSV/JSON/NDJSON/XML/YAML/XLSX/Parquet → RDF
onto_sql_ingest →  SQL-based: SELECT against Postgres or DuckDB → RDF
onto_shacl      →  validate ingested data against shapes
onto_reason     →  materialise inferred triples (rdfs / owl-rl)
onto_extend     →  file-based shortcut: ingest + SHACL + reason in one call
```

### Phase 6: Persist + Govern

```
onto_version   →  save snapshot before finalizing
onto_save      →  write to .ttl file
onto_plan / onto_enforce / onto_apply / onto_monitor / onto_drift
               →  Terraform-style lifecycle for evolving ontologies
```

## Key Rules

1. **Always validate before loading** — `onto_validate` catches syntax errors that would silently fail
2. **Always check stats after loading** — `onto_stats` catches missing classes/properties
3. **Always lint after loading** — `onto_lint` catches missing labels and domains
4. **Version before pushing** — `onto_version` before `onto_push` (enforcer rule)
5. **Iterate, don't declare done** — if any check fails, fix and re-run from Phase 3
6. **Use embeddings for semantic search and alignment** — call `onto_embed` once after `onto_load`, then `onto_search` / `onto_similarity` work, and `onto_align` automatically uses embeddings as a 7th signal.
7. **Cache, don't re-parse** — when juggling several `.ttl` files, use `onto_cache_list` / `onto_repo_load` to leverage the on-disk N-Triples cache.

## Tool Quick Reference

### Core ontology
| Tool | Purpose |
| ---- | ------- |
| `onto_status` | Server health and loaded triple count |
| `onto_validate` | Check OWL/RDF syntax (file or inline Turtle) |
| `onto_load` | Load into Oxigraph triple store |
| `onto_stats` | Class/property/triple counts |
| `onto_lint` | Missing labels, comments, domains |
| `onto_query` | Run SPARQL queries |
| `onto_diff` | Compare two ontologies |
| `onto_save` | Persist to file |
| `onto_convert` | Format conversion |
| `onto_clear` | Reset the store |

### Cache and multi-file loading
| Tool | Purpose |
| ---- | ------- |
| `onto_unload` | Drop the active (or named) ontology from memory |
| `onto_recompile` | Re-parse the source, ignoring the cache |
| `onto_cache_status` / `onto_cache_list` | Inspect compile cache |
| `onto_cache_remove` | Drop a cached entry |
| `onto_repo_list` | Enumerate `.ttl/.owl/.nt/.rdf/.nq/.trig/.jsonld` files in `[general] ontology_dirs` |
| `onto_repo_load` | Load by name / relative path / absolute path inside a repo dir |

### Remote and lifecycle
| Tool | Purpose |
| ---- | ------- |
| `onto_pull` / `onto_push` | Fetch from / send to a remote URL or SPARQL endpoint |
| `onto_marketplace` | Browse / install standard ontologies |
| `onto_import` | Resolve `owl:imports` |
| `onto_version` / `onto_history` / `onto_rollback` | Snapshots |
| `onto_plan` / `onto_enforce` / `onto_apply` / `onto_lock` / `onto_drift` / `onto_monitor` / `onto_monitor_clear` / `onto_lineage` | Terraform-style lifecycle |
| `onto_lint_feedback` / `onto_enforce_feedback` / `onto_align_feedback` | Self-calibrating suppression |

### Data pipeline (file + SQL)
| Tool | Purpose |
| ---- | ------- |
| `onto_map` | Generate mapping from data + ontology |
| `onto_ingest` | File → RDF (CSV/JSON/NDJSON/XML/YAML/XLSX/Parquet) |
| `onto_sql_ingest` | SQL `SELECT` against Postgres or DuckDB → RDF (same mapping format) |
| `onto_import_schema` | Postgres or DuckDB schema → OWL |
| `onto_shacl` | SHACL validation |
| `onto_reason` | RDFS / OWL-RL inference |
| `onto_extend` | File-based: ingest + SHACL + reason |
| `onto_dl_check` / `onto_dl_explain` | DL tableaux subsumption / unsat explanation |

### Clinical and alignment
| Tool | Purpose |
| ---- | ------- |
| `onto_crosswalk` / `onto_enrich` / `onto_validate_clinical` | ICD-10 / SNOMED / MeSH crosswalks |
| `onto_align` | Cross-ontology alignment (uses embeddings if loaded) |

### Embeddings and semantic search
| Tool | Purpose |
| ---- | ------- |
| `onto_embed` | Generate text + Poincaré structural embeddings for all classes |
| `onto_search` | Natural-language query → most-similar classes |
| `onto_similarity` | Cosine + Poincaré distance between two IRIs |

## SQL Backbone Notes

`onto_sql_ingest` and `onto_import_schema` accept any of:

- `postgres://user:pass@host/db` (requires the `postgres` Cargo feature)
- `duckdb:///path/to/file.duckdb`, `:memory:`, or a bare `*.duckdb` / `*.ddb` file path (requires the `duckdb` Cargo feature)

DuckDB is wired in as a *data integration backbone*, not as a SPARQL parser:
its `httpfs`, `parquet`, `csv`, `json`, `postgres_scanner`, `iceberg`, and
`delta` extensions let one SQL query federate over remote files, object
stores, and other databases — all of which then flow into the same mapping +
SHACL + reasoning pipeline used for plain CSV/Parquet inputs. See
`docs/data-pipeline.md` for full examples.

