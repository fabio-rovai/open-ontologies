# Open Ontologies

## Ontology Engineering Workflow

When building or modifying ontologies, follow this workflow. Claude decides which tools to call and in what order based on results — this is not a fixed pipeline.

### Generate

1. Understand the domain requirements (natural language, competency questions, methodology constraints)
2. Generate Turtle/OWL directly — Claude knows OWL, RDF, BORO, 4D modeling natively

### Validate and Load

3. Call `onto_validate` on the generated Turtle — if it fails, fix the syntax errors and re-validate
4. Call `onto_load` to load into the Oxigraph triple store
5. Call `onto_stats` to verify class count, property count, triple count match expectations

### Reason

6. Call `onto_reason` with profile `rdfs` or `owl-rl` to materialize inferred triples (transitive subclass chains, domain/range propagation, equivalentClass expansion)
7. Call `onto_stats` again to verify inferred triple counts are reasonable

### Verify

8. Call `onto_lint` to check for missing labels, comments, domains, ranges — fix any issues found
9. Call `onto_enforce` with rule pack `generic` to check design pattern compliance — fix any violations
10. Call `onto_query` with SPARQL to verify structure:
    - Are all expected classes present?
    - Do subclass hierarchies match the spec?
    - Can competency questions be answered?
11. If a reference ontology exists, call `onto_diff` to compare

### Iterate

12. If any step above reveals problems, fix the Turtle and restart from step 3
13. This loop continues until validation passes, stats match, lint is clean, enforce has no violations, and SPARQL queries return expected results

### Persist

14. Call `onto_save` to write the final ontology to a .ttl file
15. Call `onto_version` to save a named snapshot for rollback — always version after save

### Key Principle

Claude dynamically decides the next tool call based on what the previous tool returned. If `onto_validate` fails, Claude fixes and retries. If `onto_stats` shows wrong counts, Claude regenerates. If `onto_lint` finds missing labels, Claude adds them. The MCP tools are individual operations — Claude is the orchestrator.

## Tool Reference

| Tool | When to use |
| ---- | ----------- |
| `onto_status` | To check if the server is running and healthy |
| `onto_validate` | After generating or modifying Turtle — always validate first |
| `onto_load` | After validation passes — loads into triple store for querying |
| `onto_stats` | After loading — sanity check on class/property/triple counts |
| `onto_lint` | After loading — catches missing labels, domains, ranges |
| `onto_query` | To verify structure, answer competency questions, explore the ontology |
| `onto_diff` | To compare against a reference or previous version |
| `onto_save` | To persist the ontology to a file |
| `onto_convert` | To convert between formats (Turtle, N-Triples, RDF/XML, N-Quads, TriG) |
| `onto_clear` | To reset the store before loading a different ontology |
| `onto_pull` | To fetch an ontology from a remote URL or SPARQL endpoint |
| `onto_push` | To push an ontology to a SPARQL endpoint |
| `onto_import` | To resolve and load owl:imports chains |
| `onto_version` | To save a named snapshot before making changes |
| `onto_history` | To list saved version snapshots |
| `onto_rollback` | To restore a previous version if something goes wrong |
| `onto_ingest` | To parse structured data (CSV, JSON, NDJSON, XML, YAML, XLSX, Parquet) into RDF and load into the store |
| `onto_map` | To generate a mapping config from data schema + loaded ontology for review |
| `onto_shacl` | To validate loaded data against SHACL shapes (cardinality, datatypes, classes) |
| `onto_reason` | To run RDFS or OWL-RL inference, materializing inferred triples |
| `onto_extend` | To run the full pipeline: ingest → SHACL validate → reason in one call |
| `onto_import_schema` | To import a PostgreSQL database schema as an OWL ontology (requires postgres feature) |
| `onto_plan` | Before applying changes — shows added/removed classes, blast radius, risk score |
| `onto_apply` | After plan + enforce — applies changes in `safe` or `migrate` mode |
| `onto_lock` | To protect production IRIs from removal |
| `onto_drift` | To compare two versions — rename detection, drift velocity, self-calibrating confidence |
| `onto_enforce` | After loading — design pattern checks: `generic`, `boro`, `value_partition`, or custom rules |
| `onto_monitor` | After apply — run SPARQL watchers with threshold alerts |
| `onto_monitor_clear` | To clear blocked state after resolving monitor alerts |
| `onto_crosswalk` | To look up clinical terminology mappings (ICD-10 ↔ SNOMED ↔ MeSH) |
| `onto_enrich` | To add skos:exactMatch triples linking classes to clinical codes |
| `onto_validate_clinical` | To check class labels against clinical crosswalk terminology |
| `onto_align` | To detect alignment candidates (equivalentClass, exactMatch, subClassOf) between two ontologies using 7 weighted signals (6 structural + embedding similarity when embeddings are loaded) |
| `onto_align_feedback` | To accept/reject alignment candidates for self-calibrating confidence weights |
| `onto_lineage` | To view the session's lineage trail (plan → enforce → apply → monitor → drift) |
| `onto_lint_feedback` | To accept/dismiss a lint issue — teaches lint to suppress repeatedly dismissed warnings |
| `onto_enforce_feedback` | To accept/dismiss an enforce violation — teaches enforce to suppress repeatedly dismissed violations |
| `onto_dl_explain` | To explain why a class is unsatisfiable using DL tableaux reasoning — returns clash trace |
| `onto_dl_check` | To check if one class is subsumed by another using DL tableaux reasoning |
| `onto_embed` | After loading an ontology — generates text + Poincaré structural embeddings for all classes |
| `onto_search` | To find classes by natural language description — requires onto_embed first |
| `onto_similarity` | To compute embedding similarity between two specific IRIs |

## Ontology Lifecycle

When evolving an ontology in production, follow this Terraform-style cycle. Claude decides which steps to include based on the change.

### Plan

1. Call `onto_plan` with the proposed Turtle — returns added/removed classes/properties, blast radius, risk score
2. If any IRIs are locked (`onto_lock`), locked violations will appear in the plan — resolve before proceeding
3. Review the risk score: `low` (additions only), `medium` (modifications), `high` (removals with dependents)

### Enforce

4. Call `onto_enforce` with a rule pack (`generic`, `boro`, `value_partition`) — checks design pattern compliance
5. Fix any violations before applying

### Apply

6. Call `onto_apply` with mode `safe` (clear + reload) or `migrate` (add owl:equivalentClass/Property bridges)
7. Lineage is recorded automatically

### Monitor

8. Call `onto_monitor` to run SPARQL watchers — alerts trigger notify, block, or auto-rollback actions
9. If blocked, resolve the issue and call `onto_monitor_clear`

### Drift

1. Call `onto_drift` to compare versions — drift velocity, rename detection with self-calibrating confidence
2. Feed back rename accuracy to improve future confidence scores

## Data Extension Workflow

When applying an ontology to external data:

### Inspect and Map

1. Call `onto_map` with the data file — it returns field names, ontology classes/properties, and a suggested mapping
2. Review the mapping — adjust predicates, set the class, mark lookup fields
3. Optionally save the mapping to a file for reuse

### Ingest

4. Call `onto_ingest` with the data file and mapping — it generates RDF triples and loads them into the store
5. Call `onto_stats` to verify triple counts match expectations

### Validate

6. Call `onto_shacl` with SHACL shapes to validate the data against constraints
7. Fix any violations (adjust mapping or data), re-ingest if needed

### Reason

8. Call `onto_reason` with profile `rdfs` or `owl-rl` to infer new triples
9. Call `onto_query` to verify inferred knowledge is correct

### Or use the convenience pipeline

10. Call `onto_extend` to run ingest → SHACL → reason in one call

## Semantic Search & Embedding Workflow

When exploring or aligning ontologies using semantic embeddings:

### Setup

1. Ensure the embedding model is downloaded (`open-ontologies init`)
2. Call `onto_load` to load the ontology
3. Call `onto_embed` to generate text + structural embeddings for all classes

### Search

4. Call `onto_search` with a natural language query — returns most similar classes
5. Use `mode: "text"` for label/definition similarity, `mode: "structure"` for hierarchy position, `mode: "product"` for combined

### Compare

6. Call `onto_similarity` with two IRIs to see cosine + Poincaré distance between them

### Alignment Enhancement

7. When running `onto_align`, embedding similarity is automatically used as signal #7 if embeddings are loaded
8. This catches semantically equivalent classes that have different labels (e.g., Vehicle ↔ Automobile)

## Enforcer Rules (Optional)

If [OpenCheir](https://github.com/fabio-rovai/opencheir) is also connected as an MCP server, its enforcer rules provide workflow safety:

- **onto_validate_after_save** — warns if you save 3+ times without validating
- **onto_version_before_push** — warns if you push without saving a version snapshot first

These rules are optional — Open Ontologies works perfectly without OpenCheir.

## Benchmarks

This repo contains reference ontologies and comparison scripts in `benchmark/`. Use them as starting points or to verify the AI-native approach against traditional methods.
