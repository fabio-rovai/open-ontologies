# Open Ontologies — Design Document

**Date:** 2026-03-08
**Status:** Implemented (Phase 1)

## Vision

Open Ontologies is an AI-native ontology engine. Claude IS the ontology intelligence — it knows OWL, BORO, 4D modeling, every methodology. The engine only handles what Claude physically cannot: RDF parsing/validation, SPARQL execution, format conversion, and persistence.

**Built as a module of [OpenCheir](https://github.com/fabio-rovai/opencheir).**

## Principles

1. **Claude is the brain** — the engine is just hands. No embedded reasoning, no NLP, no methodology logic.
2. **Domain-agnostic** — works for healthcare, IoT, finance, logistics, 4D ontologies, anything.
3. **Lean** — 9 tools, not 246. Everything else is Claude's native capability.
4. **Fast** — Rust + Oxigraph. No JVM, no startup lag, no GC pauses.
5. **Stateful** — in-memory graph store persists across tool calls within a session.

## Architecture

```
Claude (ontology intelligence)
  │
  ├── onto_validate   → OntologyService → GraphStore::validate_turtle
  ├── onto_convert    → GraphStore::serialize
  ├── onto_load       → GraphStore::load_file
  ├── onto_query      → GraphStore::sparql_select
  ├── onto_save       → GraphStore::save_file
  ├── onto_stats      → GraphStore::get_stats
  ├── onto_diff       → OntologyService::diff
  ├── onto_lint       → OntologyService::lint
  └── onto_clear      → GraphStore::clear
```

### Layers

| Layer | Module | Purpose |
|-------|--------|---------|
| Gateway | `gateway/server.rs` | MCP tool definitions, input validation |
| Domain | `domain/ontology.rs` | Business logic (validate, convert, diff, lint) |
| Store | `store/graph.rs` | Oxigraph wrapper, RDF I/O, SPARQL execution |

### Tech Stack

| Component | Choice | Why |
|-----------|--------|-----|
| Language | Rust (edition 2024) | Performance, safety, single binary |
| RDF/SPARQL | Oxigraph 0.4 | Pure Rust, covers RDF + SPARQL + format conversion |
| MCP | rmcp 1.x | stdio transport, `#[tool]` macro |
| Metadata | SQLite (rusqlite) | Existing OpenCheir state DB |

## What Claude Does (No Tools Needed)

- OWL class hierarchies, property definitions, restrictions
- 4D / BORO / perdurantist modeling patterns
- Temporal parts, states, events, processes
- SHACL shapes, SKOS vocabularies, Dublin Core metadata
- Ontology design patterns (N-ary relations, reification)
- Methodology selection and application
- Natural language to formal ontology translation
- Competency question analysis and mapping

## What The Engine Does (Tools Required)

- Parse and validate RDF/Turtle/RDF-XML/N-Triples syntax
- Execute SPARQL queries against loaded graphs
- Convert between RDF serialization formats
- Compute triple-level diffs between ontology versions
- Lint for missing labels, comments, domains/ranges
- Persist and load ontology files
- Report statistics (triple count, classes, properties, individuals)

## Enforcer Integration

Rule `onto_validate_after_save` warns if ontology is saved 3+ times without validation, encouraging quality checks in the workflow.

## Benchmark Results

IES4 (UK Information Exchange Standard) building domain extension:
- **100% compliance** (86/86 checks passed)
- 318 triples, 36 classes, 12 properties
- Full 4D/BORO patterns: Entity+State pairs, BoundingStates, ClassOf
- All 9 competency questions covered
- Zero external tools needed — Claude generated the Turtle directly

## Phase 2 (Implemented)

Remote sync and versioning tools:

- `onto_pull` — fetch ontology from remote URL or SPARQL endpoint
- `onto_push` — push ontology to a SPARQL endpoint
- `onto_import` — resolve and load owl:imports chain
- `onto_version` — save a named snapshot of the current store
- `onto_history` — list saved version snapshots
- `onto_rollback` — restore a previous version

Enforcer rule: `onto_version_before_push` warns if pushing without a saved version snapshot.
