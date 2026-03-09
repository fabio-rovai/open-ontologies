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

### Verify

6. Call `onto_lint` to check for missing labels, comments, domains, ranges — fix any issues found
7. Call `onto_query` with SPARQL to verify structure:
   - Are all expected classes present?
   - Do subclass hierarchies match the spec?
   - Can competency questions be answered?
8. If a reference ontology exists, call `onto_diff` to compare

### Iterate

9. If any step above reveals problems, fix the Turtle and restart from step 3
10. This loop continues until validation passes, stats match, lint is clean, and SPARQL queries return expected results

### Persist

11. Call `onto_save` to write the final ontology to a .ttl file
12. Call `onto_version` to save a named snapshot for rollback

### Key Principle

Claude dynamically decides the next tool call based on what the previous tool returned. If `onto_validate` fails, Claude fixes and retries. If `onto_stats` shows wrong counts, Claude regenerates. If `onto_lint` finds missing labels, Claude adds them. The MCP tools are individual operations — Claude is the orchestrator.

## Tool Reference

| Tool | When to use |
| ---- | ----------- |
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

## Enforcer Rules

OpenCheir enforces workflow safety rules automatically:

- **onto_validate_after_save** — warns if you save 3+ times without validating
- **onto_version_before_push** — warns if you push without saving a version snapshot first

## Benchmarks

This repo contains reference ontologies and comparison scripts in `benchmark/`. Use them as starting points or to verify the AI-native approach against traditional methods.
