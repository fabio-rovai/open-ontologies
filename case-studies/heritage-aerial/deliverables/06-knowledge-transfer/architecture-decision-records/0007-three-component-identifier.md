# ADR-0007: Three-component identifier model (Collection / Sortie / Frame)

**Status:** Accepted
**Date:** 2026-04-30
**Decider:** Editorial team after red-team review
**Supersedes:** initial two-component model

## Context

Aerial photography heritage institutions, particularly NCAP, use a **three-component** identifier model:

- **Collection** — institutional accession namespace (`RAF`, `NARA`, `DOS`, `JARIC`, `USAF`, etc.)
- **Sortie** — flight mission reference (`106G/UK/1655`, `US7/LOC/0001D/LIB`)
- **Frame** — sequential frame number within the sortie (`4023`, `0085`)

The initial NAPH draft conflated Collection and Sortie into a single `naph:sortieReference` string. The red-team review identified this as a misalignment with real-world institutional practice — a Collection prefix is not part of the sortie reference; it's a separate level of identification.

## Decision

NAPH provides three distinct properties:

```turtle
naph:collectionCode    xsd:string  # Institutional collection prefix
naph:sortieReference   xsd:string  # Sortie reference within the collection
naph:frameNumber       xsd:integer # Frame number within the sortie
```

A Sortie has `naph:collectionCode` and `naph:sortieReference`. A photograph has `naph:partOfSortie` and `naph:frameNumber`.

The canonical NAPH identifier (`naph:hasIdentifier`) composes these:

```
{namespace}/photo/{collection}-{sortie-slug}-{frame}
```

## Consequences

### Positive

- **Aligns with NCAP's actual cataloguing practice** — institutions can map their existing data without information loss
- **Federation-friendly** — researchers can query "all RAF holdings across institutions" via `naph:collectionCode = "RAF"`
- **Cross-institution compatibility** — both NCAP and NARA might hold material with `collectionCode = "USAAF"`; the same code identifies the institutional origin
- **Provenance integration** — the Collection prefix often signals the original holding agency, useful for provenance reasoning

### Negative

- **Three properties instead of one** — slightly more verbose
- **Requires institutions to split existing single-string identifiers** during migration
- **Composite identifier rules need documentation** — exactly how to slug the sortie reference

### Neutral

- The composed identifier (the URI in `naph:hasIdentifier`) is still a single string. Institutions can keep their existing URI format — they just need to add the three semantic components alongside.

## Alternatives considered

### Alternative 1: Single string identifier (the original)

Rejected because:

- Doesn't match real institutional practice
- Forces a flat namespace where institutions actually have hierarchy
- Loses queryability of the Collection level

### Alternative 2: Embed all three in URI but no separate properties

Considered. URI alone (e.g. `https://w3id.org/naph/photo/RAF-106G-UK-1655-4023`) already encodes the three components.

Rejected because:

- URIs should be opaque identifiers; parsing them to extract semantic components is fragile
- Federation queries become more complex (string parsing instead of property lookup)
- Doesn't help institutions whose URIs use different patterns

### Alternative 3: Hierarchy via separate Collection class with parent-of relationships

Considered. Could model `naph:Collection -> naph:Sortie -> naph:Photograph` as a chain.

Rejected because:

- More complex modelling than necessary
- The Collection-as-identifier-prefix is conceptually different from Collection-as-grouping-of-photographs
- We already have `naph:Collection` (institutional grouping) and `naph:partOfSortie` — three-level hierarchy adds a third concept without clear value

The chosen design separates `naph:Collection` (institutional grouping) from `naph:collectionCode` (identifier prefix). They're related but distinct.

## Migration

For institutions already using single-string identifiers:

1. Parse the existing string to extract Collection prefix
2. Add `naph:collectionCode` to existing Sortie records
3. Continue using existing URIs unchanged

This is a non-breaking migration — no URI changes required.

## Validation

The decision is validated by:

- The case study sample data demonstrates the three-component pattern working
- The CSV ingest pipeline can be extended to extract Collection prefix from existing sortie strings
- Real NCAP examples (DOS/CAS/6366/0158, NARA/US7/LOC/0001D/LIB/0085) map cleanly

## Cross-references

- [Module B §B.3.4 — Composite identifier compatibility](../../01-standard/modules/B-metadata-data-structures.md#b34-composite-identifier-compatibility)
- [Aerial Photography Profile §P.5](../../01-standard/profiles/aerial-photography.md#p5-identifier-scheme--three-component)
- [Reconnaissance sub-profile §R.3](../../01-standard/profiles/aerial-subprofiles/reconnaissance.md#r3-identifier-scheme--wartime-conventions)
- [Identifier Policy decision tree](../../04-adoption-guidance/decision-trees/identifier-policy.md)
- [Red-team report — High §3](../../../docs/red-team-report.md)
