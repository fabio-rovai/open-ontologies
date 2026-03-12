---
name: ontology-engineering
description: Build, validate, and govern RDF/OWL ontologies using the Open Ontologies MCP server. Use when the user asks to create, modify, query, or manage ontologies, knowledge graphs, or RDF data.
---

# Ontology Engineering Workflow

You have access to the Open Ontologies MCP server, which provides 39+ tools for AI-native ontology engineering backed by an in-memory Oxigraph triple store.

## Core Workflow

When building or modifying ontologies, follow this workflow. Decide which tools to call and in what order based on results -- this is not a fixed pipeline.

### 1. Generate

- Understand the domain requirements (natural language, competency questions, methodology constraints)
- Generate Turtle/OWL directly -- you know OWL, RDF, BORO, 4D modeling natively

### 2. Validate and Load

- Call `onto_validate` on the generated Turtle -- if it fails, fix syntax errors and re-validate
- Call `onto_load` to load into the Oxigraph triple store
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

## Ontology Lifecycle (Terraform-style)

For evolving ontologies in production:

1. **Plan** -- `onto_plan` shows added/removed classes, blast radius, risk score. Check `onto_lock` for protected IRIs.
2. **Enforce** -- `onto_enforce` with a rule pack (`generic`, `boro`, `value_partition`) checks design pattern compliance.
3. **Apply** -- `onto_apply` with mode `safe` (clear + reload) or `migrate` (add equivalentClass bridges).
4. **Monitor** -- `onto_monitor` runs SPARQL watchers with threshold alerts. Use `onto_monitor_clear` if blocked.
5. **Drift** -- `onto_drift` compares versions with rename detection and self-calibrating confidence.

## Data Extension Workflow

When applying an ontology to external data:

1. `onto_map` -- generate mapping config from data schema + loaded ontology
2. `onto_ingest` -- parse structured data (CSV, JSON, NDJSON, XML, YAML, XLSX, Parquet) into RDF
3. `onto_shacl` -- validate against SHACL shapes
4. `onto_reason` -- run RDFS or OWL-RL inference
5. Or use `onto_extend` to run the full pipeline in one call

## Tool Reference

| Tool | When to use |
| ---- | ----------- |
| `onto_validate` | After generating or modifying Turtle -- always validate first |
| `onto_load` | After validation passes -- loads into triple store |
| `onto_stats` | After loading -- sanity check on counts |
| `onto_lint` | After loading -- catches missing labels, domains, ranges |
| `onto_query` | Verify structure, answer competency questions |
| `onto_diff` | Compare against a reference or previous version |
| `onto_save` | Persist ontology to a file |
| `onto_convert` | Convert between formats (Turtle, N-Triples, RDF/XML, N-Quads, TriG) |
| `onto_clear` | Reset the store before loading a different ontology |
| `onto_pull` | Fetch ontology from a remote URL or SPARQL endpoint |
| `onto_push` | Push ontology to a SPARQL endpoint |
| `onto_import` | Resolve and load owl:imports chains |
| `onto_version` | Save a named snapshot before making changes |
| `onto_history` | List saved version snapshots |
| `onto_rollback` | Restore a previous version |
| `onto_ingest` | Parse structured data into RDF and load into store |
| `onto_map` | Generate mapping config from data schema + ontology |
| `onto_shacl` | Validate data against SHACL shapes |
| `onto_reason` | Run RDFS or OWL-RL inference |
| `onto_extend` | Full pipeline: ingest, SHACL validate, reason |
| `onto_plan` | Show added/removed classes, blast radius, risk score |
| `onto_apply` | Apply changes in safe or migrate mode |
| `onto_lock` | Protect production IRIs from removal |
| `onto_drift` | Compare versions with rename detection |
| `onto_enforce` | Design pattern checks |
| `onto_monitor` | Run SPARQL watchers with threshold alerts |
| `onto_monitor_clear` | Clear blocked state after resolving alerts |
| `onto_crosswalk` | Look up clinical terminology mappings (ICD-10, SNOMED, MeSH) |
| `onto_enrich` | Add skos:exactMatch triples linking to clinical codes |
| `onto_validate_clinical` | Check class labels against clinical terminology |
| `onto_align` | Detect alignment candidates between two ontologies |
| `onto_align_feedback` | Accept/reject alignment candidates for self-calibrating weights |
| `onto_lineage` | View session lineage trail |
| `onto_lint_feedback` | Accept/dismiss lint issues to teach suppression |
| `onto_enforce_feedback` | Accept/dismiss enforce violations to teach suppression |

## Key Principle

Dynamically decide the next tool call based on what the previous tool returned. If `onto_validate` fails, fix and retry. If `onto_stats` shows wrong counts, regenerate. If `onto_lint` finds missing labels, add them. The MCP tools are individual operations -- you are the orchestrator.
