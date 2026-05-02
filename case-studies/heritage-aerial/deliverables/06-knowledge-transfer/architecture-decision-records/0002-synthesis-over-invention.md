# ADR-0002: Synthesis over invention — subclass alignment to existing standards

**Status:** Accepted
**Date:** 2026-04-30
**Decider:** Editorial team
**Supersedes:** N/A

## Context

A standard for aerial photography heritage could:

(a) Define new classes and properties from first principles
(b) Subsume and extend existing W3C/OGC standards via subclass alignment

The choice has long-term consequences. Option (a) gives more design freedom; option (b) gains compatibility with the wider linked-data ecosystem at the cost of accepting parent-standard semantics.

## Decision

NAPH uses **synthesis over invention**: every NAPH class with an existing W3C/OGC equivalent is declared a subclass of that equivalent, and we defer to the parent standard's semantics.

Specifically:

```turtle
naph:AerialPhotograph rdfs:subClassOf dcat:Resource ;
                      [adds dcterms:type dctype:StillImage]
naph:Collection      rdfs:subClassOf dcat:Catalog
naph:CaptureEvent    rdfs:subClassOf prov:Activity
naph:DigitisationEvent rdfs:subClassOf prov:Activity
naph:DigitalSurrogate  rdfs:subClassOf prov:Entity
naph:ProvenanceChain   rdfs:subClassOf prov:Bundle
naph:GeographicFootprint rdfs:subClassOf geo:Geometry
naph:Place           rdfs:subClassOf skos:Concept
naph:HistoricEvent   rdfs:subClassOf skos:Concept
naph:Subject         rdfs:subClassOf skos:Concept
naph:CustodialInstitution rdfs:subClassOf foaf:Organization
```

Eleven subclass declarations cover the integration surface.

## Consequences

### Positive

- **Automatic linked-data ecosystem participation:** every NAPH-compliant record is simultaneously a valid PROV record, DCAT record, SKOS record, etc. Tools that consume these standards work with NAPH records without bespoke integration.
- **Lower long-term maintenance:** as parent standards evolve, NAPH inherits improvements automatically (within compatible MINOR releases of the parent standard).
- **Reduced learning curve:** developers familiar with PROV/DCAT/SKOS can read NAPH and immediately understand the modelling intent.
- **Cleaner federated queries:** SPARQL federation across NAPH and Wikidata / GeoNames / CKAN works because the basic types align.

### Negative

- **Constrained design freedom:** when parent standards have constraints we'd prefer to relax, we must accept them or break the alignment.
- **Vulnerability to parent-standard breaking changes:** if W3C deprecates DCAT, NAPH must respond. (Realistic risk: low. W3C standards rarely break.)
- **Potential for misalignment errors:** picking the wrong parent class (initial draft used `dcat:Dataset` instead of `dcat:Resource` for individual photographs — see [ADR-0003](0003-aerial-photograph-as-dcat-resource.md)) propagates broadly. The red-team review caught one such error before publication.

### Neutral

- The chosen parent standards are themselves stable, maintained by W3C/OGC/IIIF Consortium with multi-year governance. Parent-standard turnover risk is low.

## Alternatives considered

### Alternative 1: Define new classes from first principles

Rejected because:

- Reinventing what W3C/OGC already define is unnecessary work
- Records modelled in a NAPH-only ontology don't participate in the wider linked-data ecosystem without bespoke crosswalks
- Each crosswalk is additional ongoing maintenance

### Alternative 2: Hybrid — subclass for some classes, originate others

Considered briefly. Rejected because:

- The choice of which classes to subsume vs. originate would itself be a new modelling problem
- Hybrid models tend to grow in complexity over time
- The discipline of "subsume wherever possible" produces simpler, more predictable design

### Alternative 3: Use existing standards directly (no NAPH-specific classes)

Rejected because:

- Aerial photography has domain-specific concepts (Sortie, AerialPhotograph, GroundSampleDistance) that don't have direct equivalents in W3C/OGC
- A standard with no classes of its own is not a standard
- The aerial-domain depth requires aerial-domain typing

## Implementation

Each NAPH class declaration includes its parent class. Parent classes are referenced via canonical prefixes (DCAT, PROV, etc.). The ontology imports nothing — relies on RDFS inference at query time to materialise parent-class membership.

```turtle
@prefix dcat: <http://www.w3.org/ns/dcat#> .
@prefix prov: <http://www.w3.org/ns/prov#> .
naph:AerialPhotograph rdfs:subClassOf dcat:Resource .
```

This is verified by the [reasoning-inference test](../../docs/reasoning-inference.md) which confirms that loading NAPH ontology + sample data + RDFS reasoner produces the expected parent-class memberships.

## Validation

The synthesis approach is validated by the reasoning test:

- Loading NAPH-compliant data produces correct PROV-O activity assertions
- Loading NAPH-compliant data produces correct DCAT resource assertions
- Loading NAPH-compliant data produces correct SKOS concept assertions

These were verified in the red-team review and persist after the dcat:Dataset → dcat:Resource correction.

## Revisiting

This ADR may be revisited if:

- A parent standard makes a breaking change that propagates to NAPH
- The community develops a more authoritative aerial-specific parent standard (e.g. an OGC aerial-imagery standard)

Reasonable revisitation timeframe: post-2030.

## Cross-references

- [Reasoning inference documentation](../../../docs/reasoning-inference.md)
- [Standards alignment in NAPH-STANDARD §6](../../01-standard/NAPH-STANDARD.md)
- [Red-team report](../../../docs/red-team-report.md)
