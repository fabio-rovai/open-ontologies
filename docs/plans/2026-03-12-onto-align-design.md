# onto_align — Schema Alignment Tool

**Date:** 2026-03-12
**Status:** Approved
**Branch:** `onto_align` (separate from main)

## Purpose

Detect and propose semantic alignment candidates (`owl:equivalentClass`, `skos:exactMatch`, `rdfs:subClassOf`) between two ontologies. Optionally auto-apply high-confidence matches.

This is the single bottleneck that broke every previous semantic web attempt (W3C, DBpedia, Google KG). Modern advantage: AI-assisted schema discovery + Rust infra + cheap compute.

## Input Model

Two modes via a single MCP tool (`onto_align`):

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `source` | string | yes | Turtle string OR file path of ontology A |
| `target` | string | no | Turtle string OR file path of ontology B. If omitted, aligns `source` against the loaded store |
| `min_confidence` | float | no | Auto-apply threshold (default 0.85). Candidates above this get inserted as triples |
| `dry_run` | bool | no | If true, return candidates only, never insert (default false) |

## Output Model

```json
{
  "candidates": [
    {
      "source_iri": "http://example.org/Dog",
      "target_iri": "http://other.org/Canine",
      "relation": "owl:equivalentClass",
      "confidence": 0.92,
      "signals": {
        "label_similarity": 0.78,
        "property_overlap": 0.95,
        "parent_overlap": 0.88,
        "instance_overlap": 0.60,
        "restriction_similarity": 0.70,
        "neighborhood_similarity": 0.55
      },
      "applied": true
    }
  ],
  "applied_count": 3,
  "total_candidates": 7,
  "threshold": 0.85
}
```

- `relation`: one of `owl:equivalentClass`, `skos:exactMatch`, `rdfs:subClassOf`
- `applied: true` only when confidence >= threshold AND dry_run is false

## Signals

Six signals, weighted and combined via self-calibrating confidence (same pattern as drift.rs):

| # | Signal | Method | Default Weight |
|---|--------|--------|---------------|
| 1 | Label similarity | Jaro-Winkler on rdfs:label, skos:prefLabel, skos:altLabel (reused from drift.rs) | 0.25 |
| 2 | Property signature overlap | Jaccard similarity on domain/range property sets via SPARQL | 0.20 |
| 3 | Parent/subclass overlap | Shared rdfs:subClassOf parents | 0.15 |
| 4 | Instance overlap | Shared individuals typed under both candidate classes | 0.15 |
| 5 | Restriction patterns | Compare owl:someValuesFrom / owl:allValuesFrom restrictions | 0.15 |
| 6 | Graph neighborhood | 2-hop property graph comparison | 0.10 |

### Label normalization

- Lowercase, strip whitespace
- CamelCase splitting ("DomesticCat" -> "domestic cat")
- Compare all label variants (rdfs:label, skos:prefLabel, skos:altLabel)

### Relation classification

- High label + high property overlap -> `owl:equivalentClass`
- High label + low property overlap -> `skos:exactMatch`
- Moderate label + shared parent -> `rdfs:subClassOf`

## Self-Calibrating Confidence

New `align_feedback` SQLite table (same pattern as `drift_feedback`):

```sql
CREATE TABLE IF NOT EXISTS align_feedback (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_iri TEXT NOT NULL,
    target_iri TEXT NOT NULL,
    predicted_relation TEXT NOT NULL,
    accepted BOOLEAN NOT NULL,
    timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);
```

Feedback tool: `onto_align_feedback` — accepts candidate source_iri, target_iri, and accepted (bool).

When enough feedback accumulates, `get_learned_weights()` adjusts signal weights based on which signals predicted correctly for accepted vs rejected candidates.

## Architecture

```
src/align.rs          -- AlignmentEngine struct, signal computation, candidate ranking
src/server.rs         -- onto_align + onto_align_feedback tool registration
src/state.rs          -- align_feedback table added to SCHEMA
src/drift.rs          -- jaro_winkler() stays here, imported by align.rs
tests/cli_test.rs     -- test_cli_align, test_cli_align_feedback
```

New file: `src/align.rs` only. Everything else is edits to existing files.

### Data flow

1. Parse source (and target if provided) into temporary Oxigraph graphs
2. Extract class IRIs + labels + property signatures via SPARQL
3. Cartesian product of source x target classes, compute 6 signals per pair
4. Score and rank candidates
5. Classify relation type based on signal pattern
6. Filter by min_confidence for auto-apply
7. Insert triples into main store (if not dry_run)
8. Record lineage event
9. Return JSON

## CLI

```bash
# Align two files
open-ontologies align source.ttl target.ttl --min-confidence 0.85

# Align file against loaded store
open-ontologies align source.ttl --min-confidence 0.9 --dry-run

# Accept/reject a candidate
open-ontologies align-feedback --source <iri> --target <iri> --accept
open-ontologies align-feedback --source <iri> --target <iri> --reject
```

## MCP Tools

### onto_align

```
#[tool(name = "onto_align", description = "Detect alignment candidates between ontologies")]
```

Parameters: source, target (optional), min_confidence (optional, default 0.85), dry_run (optional, default false)

### onto_align_feedback

```
#[tool(name = "onto_align_feedback", description = "Accept or reject an alignment candidate to improve future confidence")]
```

Parameters: source_iri, target_iri, accepted (bool)
