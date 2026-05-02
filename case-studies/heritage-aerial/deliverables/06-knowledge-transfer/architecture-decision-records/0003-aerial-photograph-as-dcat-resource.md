# ADR-0003: AerialPhotograph as dcat:Resource (not dcat:Dataset)

**Status:** Accepted
**Date:** 2026-04-30
**Decider:** Editorial team after red-team review
**Supersedes:** initial draft modelling decision

## Context

The initial draft of NAPH declared:

```turtle
naph:AerialPhotograph rdfs:subClassOf dcat:Dataset .
```

The red-team review (see [`docs/red-team-report.md`](../../../docs/red-team-report.md)) identified this as incorrect per the W3C DCAT 3 specification.

## DCAT 3 spec on Dataset vs Resource

From the DCAT 3 specification:

> A dataset is "a collection of data, published or curated by a single agent" — emphasising that datasets are inherently aggregations.

For individual data items (photographs, documents):

> Use `dcat:Resource` (the parent class), with `dcterms:type` pointing to a DCMI Type vocabulary term such as `dctype:StillImage`.

`dcat:Dataset` is for **collections** (aggregations of data); `dcat:Resource` is the parent class for **individual** data items.

## Decision

`naph:AerialPhotograph` is a subclass of `dcat:Resource`, not `dcat:Dataset`. Each instance additionally declares its DCMI type:

```turtle
naph:AerialPhotograph rdfs:subClassOf dcat:Resource ;
    rdfs:comment "Each instance SHOULD declare dcterms:type dctype:StillImage." .

ex:photo-001 a naph:AerialPhotograph ;
    dcterms:type dctype:StillImage ;
    ... .
```

`naph:Collection` (the institutional grouping) remains a subclass of `dcat:Catalog` (which is a refinement of `dcat:Dataset` in DCAT). The overall NAPH-DCAT mapping is:

```
NAPH                     DCAT
─────────────────       ───────────────
AerialPhotograph    →   dcat:Resource (with dcterms:type dctype:StillImage)
Collection          →   dcat:Catalog
DigitalSurrogate    →   dcat:Distribution (alternative pathway)
```

## Consequences

### Positive

- Correct DCAT alignment
- Compatible with DCAT-aware tools and aggregators (Europeana, DPLA)
- `dcterms:type dctype:StillImage` makes resource type explicit and queryable
- RDFS inference produces correct DCAT class memberships

### Negative

- Initial documentation had to be corrected (ADR-0003 supersedes initial decision)
- Reasoning-inference doc had to be updated to reflect that only Collections produce `dcat:Dataset` inferences, not individual photographs

### Neutral

- All sample data was updated to include `dcterms:type dctype:StillImage`
- Pipeline ([`pipeline/ingest.py`](../../../pipeline/ingest.py)) was updated to emit the correct typing

## Alternatives considered

### Alternative 1: Keep `rdfs:subClassOf dcat:Dataset` (the original)

Rejected because the W3C DCAT 3 specification explicitly says this is wrong. Continuing the error would propagate misalignment to every NAPH adopter.

### Alternative 2: Use `dcat:DataService` for photographs

Rejected because DataService is for service endpoints (APIs, query interfaces), not data items.

### Alternative 3: Don't subsume DCAT at all for photographs

Rejected because aggregator participation is a key value proposition. DCAT alignment is the mechanism. The fix is to subsume the right class, not abandon DCAT.

## Validation

After the correction:

- SHACL validation continues to pass with 0 violations
- All 7 competency-question queries continue to return expected results
- RDFS inference correctly produces:
  - 1 dcat:Dataset (the Collection only)
  - 11 dcat:Resource (10 photographs + 1 Collection inheriting from Dataset which is a Resource)
  - All previous PROV / SKOS / GeoSPARQL inferences

## Lessons learned

This decision was made in error initially, caught by the red-team review, and corrected before any external publication. The lesson:

- **Verify standards alignment claims against the canonical spec** — don't rely on intuition or memory
- **Red-team reviews catch real errors** — building review into the development process is valuable
- **Documentation of decisions (ADRs) makes it easier to correct course** — the original wasn't documented as a deliberate decision, which made it hard to evaluate

## Cross-references

- [DCAT 3 specification](https://www.w3.org/TR/vocab-dcat-3/)
- [DCMI Type vocabulary](https://www.dublincore.org/specifications/dublin-core/dcmi-terms/#http://purl.org/dc/dcmitype/)
- [Red-team report](../../../docs/red-team-report.md)
- [ADR-0002 — Synthesis over invention](0002-synthesis-over-invention.md)
- [Reasoning inference documentation](../../../docs/reasoning-inference.md)
