---
name: open-ontologies
version: "0.5.1"
description: >
  AI-native ontology engineering using 39+ MCP tools backed by an in-memory Oxigraph triple store.
  Build, validate, query, and govern RDF/OWL ontologies with a generate-validate-iterate loop.
  Use when building ontologies, knowledge graphs, RDF data, SPARQL queries, BORO/4D modeling,
  SHACL validation, clinical terminology mapping, or Terraform-style ontology lifecycle management.
tags:
  - ontology
  - rdf
  - owl
  - sparql
  - knowledge-graph
  - semantic-web
  - mcp
  - oxigraph
  - shacl
  - boro
metadata:
  openclaw:
    requires:
      mcp:
        - name: open-ontologies
          description: >
            Oxigraph-backed MCP server providing all onto_* tools.
            Install: cargo install open-ontologies OR download binary from
            https://github.com/fabio-rovai/open-ontologies/releases
          config:
            command: open-ontologies
            args: ["serve"]
      bins:
        - open-ontologies
    network:
      - description: "onto_pull fetches ontologies from remote URLs or SPARQL endpoints (user-provided URLs only)"
        direction: outbound
        optional: true
      - description: "onto_push sends triples to a user-specified SPARQL endpoint"
        direction: outbound
        optional: true
    notes: >
      All processing is local by default. The in-memory Oxigraph triple store runs inside the
      MCP server process -- no database, no JVM, no external services required.
      Network access is only used by onto_pull and onto_push when the user explicitly provides
      a remote URL or SPARQL endpoint. onto_monitor alerts are logged locally to stdout;
      no external notification services are contacted. No credentials or API keys are needed
      for core functionality.
---

# Open Ontologies

AI-native ontology engineering. Generate OWL/RDF directly, validate with MCP tools, iterate until clean, govern with a Terraform-style lifecycle.

## Prerequisites

This skill requires the **Open Ontologies MCP server** to provide the `onto_*` tools.

**Install:** `cargo install open-ontologies` or download from [GitHub releases](https://github.com/fabio-rovai/open-ontologies/releases)

**MCP config** (add to `.mcp.json` or Claude settings):
```json
{
  "mcpServers": {
    "open-ontologies": {
      "command": "open-ontologies",
      "args": ["serve"]
    }
  }
}
```

**No credentials needed.** All processing runs locally in an in-memory Oxigraph triple store. Network access is only used when you explicitly call `onto_pull` (fetch remote ontology) or `onto_push` (send to SPARQL endpoint) with a URL you provide. Monitor alerts (`onto_monitor`) are logged to stdout only.

## Core Workflow

When building or modifying ontologies, follow this workflow. Decide which tools to call and in what order based on results -- this is not a fixed pipeline.

### 1. Generate

- Understand the domain requirements (natural language, competency questions, methodology constraints)
- Generate Turtle/OWL directly -- Claude knows OWL, RDF, BORO, 4D modeling natively
- For complex methodologies, ask for background documents or constraints

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
3. **Apply** -- `onto_apply` with mode `safe` (clear + reload) or `migrate` (add owl:equivalentClass bridges).
4. **Monitor** -- `onto_monitor` runs SPARQL watchers with threshold alerts. Use `onto_monitor_clear` if blocked.
5. **Drift** -- `onto_drift` compares versions with rename detection and self-calibrating confidence.

## Data Extension Workflow

When applying an ontology to external data:

1. `onto_map` -- generate mapping config from data schema + loaded ontology
2. `onto_ingest` -- parse structured data (CSV, JSON, NDJSON, XML, YAML, XLSX, Parquet) into RDF
3. `onto_shacl` -- validate against SHACL shapes (cardinality, datatypes, classes)
4. `onto_reason` -- run RDFS or OWL-RL inference, materializing inferred triples
5. Or use `onto_extend` to run the full pipeline: ingest, SHACL validate, reason in one call

## Clinical Terminology Support

For healthcare ontologies:

- `onto_crosswalk` -- look up mappings between ICD-10, SNOMED CT, and MeSH
- `onto_enrich` -- add skos:exactMatch triples linking classes to clinical codes
- `onto_validate_clinical` -- check class labels against clinical crosswalk terminology

## Ontology Alignment

For aligning two ontologies:

- `onto_align` -- detect alignment candidates (equivalentClass, exactMatch, subClassOf) using 6 weighted signals
- `onto_align_feedback` -- accept/reject candidates to self-calibrate confidence weights

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
| `onto_enforce` | Design pattern checks: generic, boro, value_partition, or custom |
| `onto_monitor` | Run SPARQL watchers with threshold alerts |
| `onto_monitor_clear` | Clear blocked state after resolving alerts |
| `onto_crosswalk` | Look up clinical terminology mappings (ICD-10, SNOMED, MeSH) |
| `onto_enrich` | Add skos:exactMatch triples linking to clinical codes |
| `onto_validate_clinical` | Check class labels against clinical terminology |
| `onto_align` | Detect alignment candidates between two ontologies |
| `onto_align_feedback` | Accept/reject alignment candidates for self-calibrating weights |
| `onto_lineage` | View session lineage trail (plan, enforce, apply, monitor, drift) |
| `onto_lint_feedback` | Accept/dismiss lint issues to teach suppression |
| `onto_enforce_feedback` | Accept/dismiss enforce violations to teach suppression |
| `onto_unload` | Unload from memory. With `name` targets a specific cached entry; `delete_cache=true` also removes the on-disk file |
| `onto_recompile` | Re-parse the source. With `name` rebuilds a non-active cached entry without disturbing the active in-memory store |
| `onto_cache_status` | Inspect compile cache: active slot, all entries, effective `[cache]` config |
| `onto_cache_list` | List cached ontologies with metadata (`is_active`, `in_memory`, mtime, size) |
| `onto_cache_remove` | Remove a cached ontology by `name` (pass `delete_file=false` to keep the on-disk N-Triples) |
| `onto_repo_list` | List RDF/OWL files in configured `[general] ontology_dirs` directories |
| `onto_repo_load` | Load an ontology from a configured repo by bare name, relative path, or absolute path |

## Usage Examples

### Build a pizza ontology from scratch

```
Build me a pizza ontology with classes for Pizza, PizzaBase (ThinAndCrispy, DeepPan),
PizzaTopping (Mozzarella, Tomato, Pepperoni, Mushroom), and properties hasBase, hasTopping.
Include rdfs:labels and rdfs:comments on everything. Validate and run competency queries
to check I can ask "what toppings does a Margherita have?"
```

### Load and query an existing ontology

```
Load the ontology from https://www.w3.org/TR/owl-guide/wine.rdf, show me stats,
lint it, and run a SPARQL query to find all subclasses of Wine.
```

### Evolve an ontology safely

```
I need to add a new class "GlutenFreePizza" as a subclass of Pizza with a restriction
that hasBase only GlutenFreeBase. Plan the change, enforce against generic rules,
and apply in safe mode.
```

### Ingest CSV data into a knowledge graph

```
I have a CSV of employees with columns: name, department, role, start_date.
Map it to the loaded HR ontology and ingest it. Then validate with SHACL shapes
and run inference to materialize department hierarchies.
```

### Align two ontologies

```
Load schema.org and my company ontology. Run onto_align to find equivalentClass
and exactMatch candidates. I'll review and give feedback to calibrate the weights.
```

## Key Principle

Dynamically decide the next tool call based on what the previous tool returned. If `onto_validate` fails, fix and retry. If `onto_stats` shows wrong counts, regenerate. If `onto_lint` finds missing labels, add them. The MCP tools are individual operations -- Claude is the orchestrator.
