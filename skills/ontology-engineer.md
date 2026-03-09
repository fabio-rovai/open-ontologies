---
name: ontology-engineer
description: Use when building, modifying, or validating ontologies. Orchestrates the onto_* MCP tools in a generate-validate-iterate loop. Triggers on "ontology", "OWL", "RDF", "Turtle", "SPARQL", "BORO", "4D modeling", or /ontology-engineer.
---

# Ontology Engineer

AI-native ontology engineering using OpenCheir's `onto_*` MCP tools.

## When to Use

- User asks to build, extend, or modify an ontology
- User asks to validate, lint, or query an existing ontology
- User mentions OWL, RDF, Turtle, SPARQL, BORO, 4D modeling, IES4
- User wants to compare ontologies or run competency questions

## Prerequisites

The `onto_*` tools must be available via the OpenCheir MCP server. If they're not available, tell the user to install OpenCheir.

## Workflow

Claude dynamically decides which tool to call next based on results. This is NOT a fixed pipeline — adapt based on what each tool returns.

### Phase 1: Understand

- What domain? (Pizza, buildings, healthcare, etc.)
- What methodology? (standard OWL, BORO/4D, SKOS, etc.)
- What are the competency questions? (what should the ontology be able to answer?)
- Is there a reference ontology to compare against?

### Phase 2: Generate

- Generate Turtle/OWL directly from domain knowledge
- Claude knows OWL, RDF, BORO, 4D modeling, every methodology natively
- For complex methodologies, ask the user for background documents or constraints

### Phase 3: Validate (loop until clean)

```
onto_validate  →  syntax errors?  →  fix Turtle, re-validate
onto_load      →  loaded ok?      →  proceed to verification
onto_stats     →  counts match?   →  if not, regenerate missing parts
onto_lint      →  issues found?   →  fix labels/domains/ranges, re-load
```

### Phase 4: Verify (loop until correct)

```
onto_query     →  run SPARQL to check:
                   - all expected classes present?
                   - subclass hierarchies correct?
                   - competency questions answerable?
onto_diff      →  if reference exists, compare and report gaps
```

### Phase 5: Persist

```
onto_version   →  save snapshot before finalizing
onto_save      →  write to .ttl file
```

## Key Rules

1. **Always validate before loading** — `onto_validate` catches syntax errors that would silently fail
2. **Always check stats after loading** — `onto_stats` catches missing classes/properties
3. **Always lint after loading** — `onto_lint` catches missing labels and domains
4. **Version before pushing** — `onto_version` before `onto_push` (enforcer rule)
5. **Iterate, don't declare done** — if any check fails, fix and re-run from Phase 3

## Tool Quick Reference

| Tool | Purpose |
| ---- | ------- |
| `onto_validate` | Check OWL/RDF syntax (file or inline Turtle) |
| `onto_load` | Load into Oxigraph triple store |
| `onto_stats` | Class/property/triple counts |
| `onto_lint` | Missing labels, comments, domains |
| `onto_query` | Run SPARQL queries |
| `onto_diff` | Compare two ontologies |
| `onto_save` | Persist to file |
| `onto_convert` | Format conversion |
| `onto_clear` | Reset the store |
| `onto_pull` | Fetch from URL or SPARQL endpoint |
| `onto_push` | Push to SPARQL endpoint |
| `onto_import` | Resolve owl:imports |
| `onto_version` | Save named snapshot |
| `onto_history` | List snapshots |
| `onto_rollback` | Restore previous version |
