# Ontology Extension Tools — Design

**Date:** 2026-03-10
**Status:** Approved

## Summary

Add 5 new MCP tools to Open Ontologies for feeding structured datasets and applying ontology rules (mapping, SHACL validation, OWL reasoning). Extends the project from "build ontologies" to "build and apply ontologies."

## New Tools

### `onto_ingest` — Parse structured data into RDF triples

**Inputs:**
- `path` — file path to data
- `format` — csv, json, ndjson, xml, parquet, xlsx, yaml (auto-detected from extension if omitted)
- `mapping` — optional JSON mapping config (inline or file path)
- `base_iri` — optional base IRI for generated instances (default: `http://example.org/data/`)
- `inline_mapping` — bool, treat mapping as inline JSON

**Behavior:**
- If `mapping` provided, use it to map fields → predicates/classes
- If no mapping, generate naive 1:1 mapping (field names become predicates, rows become instances)
- Loads resulting triples into the existing Oxigraph store

**Output:** `{ok, triples_loaded, unmapped_fields[], warnings[]}`

**Supported formats:**
| Format | Extension | Rust crate |
|--------|-----------|------------|
| CSV | .csv | `csv` |
| JSON | .json | `serde_json` |
| NDJSON | .ndjson, .jsonl | `serde_json` (line-by-line) |
| XML | .xml | `quick-xml` |
| Parquet | .parquet | `parquet` (arrow) |
| Excel | .xlsx | `calamine` |
| YAML | .yaml, .yml | `serde_yaml` |

### `onto_map` — Generate mapping config from data + ontology

**Inputs:**
- `data_path` — path to sample data file
- `format` — data format (auto-detected if omitted)
- `save_path` — optional path to save mapping config

**Behavior:**
- Reads data schema (CSV headers, JSON keys, etc.)
- Reads classes/properties from the currently loaded ontology in the store
- Produces a JSON mapping config with field-to-predicate suggestions
- Claude reviews and refines the mapping before passing to `onto_ingest`

**Output:**
```json
{
  "mappings": [
    {"field": "name", "predicate": "rdfs:label", "datatype": "xsd:string"},
    {"field": "category", "class": "ex:Category", "predicate": "rdf:type"}
  ],
  "unmapped_fields": ["internal_id"],
  "ontology_classes": ["ex:Building", "ex:Category"],
  "ontology_properties": ["ex:hasName", "ex:hasCategory"]
}
```

### `onto_shacl` — Validate triples against SHACL shapes

**Inputs:**
- `shapes` — path to SHACL file OR inline Turtle shapes
- `inline` — bool, treat shapes as inline content

**Behavior:**
- Runs SHACL validation against the current Oxigraph store
- Implemented via SPARQL queries covering core SHACL constraints:
  - `sh:minCount`, `sh:maxCount` (cardinality)
  - `sh:datatype` (type checking)
  - `sh:class` (class membership)
  - `sh:pattern` (regex on values)
  - `sh:in` (allowed values)
  - `sh:nodeKind` (IRI vs literal vs blank)
- Reports violations with severity, focus node, path, message

**Output:**
```json
{
  "conforms": false,
  "violation_count": 3,
  "violations": [
    {
      "severity": "Violation",
      "focus_node": "ex:building-42",
      "path": "ex:hasName",
      "message": "Missing required property ex:hasName (sh:minCount 1)"
    }
  ]
}
```

### `onto_reason` — Run RDFS/OWL inference

**Inputs:**
- `profile` — `rdfs` (default), `owl-rl`, `owl-el`
- `materialize` — bool, add inferred triples to store (default: true)

**Behavior:**
- Applies inference rules via iterative SPARQL INSERT queries
- RDFS profile:
  - Subclass transitivity (rdfs9)
  - Domain inference (rdfs2)
  - Range inference (rdfs3)
  - Subproperty transitivity (rdfs5, rdfs7)
- OWL RL profile (adds to RDFS):
  - Transitive properties
  - Symmetric properties
  - Inverse properties
  - Property chain axioms
  - HasValue restrictions
  - SomeValuesFrom / AllValuesFrom (partial)
- Runs rules in a fixpoint loop until no new triples are inferred

**Output:**
```json
{
  "profile_used": "rdfs",
  "inferred_count": 47,
  "iterations": 3,
  "sample_inferences": [
    "ex:building-42 rdf:type ex:Asset (via rdfs:subClassOf ex:Building rdfs:subClassOf ex:Asset)"
  ]
}
```

### `onto_extend` — Convenience pipeline

**Inputs:** Combines ingest + shacl + reason inputs:
- `data_path`, `format`, `mapping` (from onto_ingest)
- `shapes` (from onto_shacl, optional)
- `reason_profile` (from onto_reason, optional)
- `stop_on_violations` — bool, halt pipeline if SHACL fails (default: true)

**Behavior:** Runs: ingest → shacl (if shapes provided) → reason (if profile provided)

**Output:** Combined report from all stages.

## Data Flow

```
Data (CSV/JSON/NDJSON/XML/Parquet/XLSX/YAML)
    │
    ▼
onto_map ──→ mapping config (Claude reviews/edits)
    │
    ▼
onto_ingest ──→ RDF triples loaded into Oxigraph store
    │
    ▼
onto_shacl ──→ validation report (conforms / violations)
    │
    ▼
onto_reason ──→ inferred triples added to store
    │
    ▼
onto_query / onto_save ──→ query results / persist
```

## Architecture

No new modules needed for state/graph. New source files:

- `src/ingest.rs` — data parsing (CSV, JSON, NDJSON, XML, Parquet, XLSX, YAML) and RDF generation
- `src/shacl.rs` — SHACL validation via SPARQL
- `src/reason.rs` — RDFS/OWL inference rules via SPARQL INSERT
- `src/mapping.rs` — mapping config generation and application

All tools use the existing `GraphStore` (Oxigraph) and `StateDb` (SQLite). No separate stores.

## New Cargo Dependencies

```toml
csv = "1"
quick-xml = "0.37"
serde_yaml = "0.9"
calamine = "0.26"
parquet = "54"      # via arrow-rs
arrow = "54"
```

## Mapping Config Format

```json
{
  "base_iri": "http://example.org/data/",
  "id_field": "id",
  "class": "ex:Building",
  "mappings": [
    {
      "field": "name",
      "predicate": "rdfs:label",
      "datatype": "xsd:string"
    },
    {
      "field": "category",
      "predicate": "rdf:type",
      "class": "ex:Category",
      "lookup": true
    },
    {
      "field": "latitude",
      "predicate": "geo:lat",
      "datatype": "xsd:decimal"
    }
  ]
}
```

## Tool Count After Implementation

Current: 16 tools → New total: **21 tools**
