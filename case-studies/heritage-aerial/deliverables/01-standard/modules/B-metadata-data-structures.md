# Module B — Metadata & Data Structures

**Status:** Normative · v1.0
**Applies to:** All NAPH-compliant records, all tiers
**Defines:** the structural and descriptive metadata required for computational discovery, identification, and retrieval

## B.1 Purpose

Module B is the technical heart of the standard. It specifies what metadata fields a record must expose, in what form, with what semantics, to be **computation-ready** — meaning a researcher can find, filter, aggregate, and reason over the record using any RDF-aware tool without manual interpretation.

The bar is: a record's metadata must answer competency questions ([§B.5](#b5-competency-questions)) using a machine-readable query, not a human reading free-text fields.

## B.2 Outcome requirements

### B.2.1 Baseline (B-baseline)

A Baseline-compliant record MUST have:

- **B.B.1** A `naph:hasIdentifier` URI that resolves and is globally unique
- **B.B.2** A `naph:capturedOn` value of type `xsd:date`, `xsd:gYearMonth`, or `xsd:gYear`
  - Free-text dates are NOT permitted, even when partial
- **B.B.3** A `naph:coversArea` link to a `naph:GeographicFootprint` with a `geo:wktLiteral` polygon in WGS84
- **B.B.4** A `naph:partOfSortie` (or equivalent acquisition-event link) connecting the record to a documented mission/event/accession
- **B.B.5** A `naph:belongsToCollection` link to a documented Collection
- **B.B.6** An RDF type assertion: `naph:AerialPhotograph` (for photographic profile) or equivalent profile class

A Baseline-compliant record SHOULD have:

- **B.B.7** `dcterms:type` pointing to a DCMI Type vocabulary term (`dctype:StillImage`, `dctype:Text`, etc.)
- **B.B.8** `rdfs:label` providing a human-readable label

### B.2.2 Enhanced (B-enhanced)

An Enhanced-compliant record MUST additionally have:

- **B.E.1** `naph:hasDigitalSurrogate` linking to a documented `naph:DigitalSurrogate` (per [Module A](A-capture-imaging.md))
- **B.E.2** `naph:hasCaptureEvent` linking to a `naph:CaptureEvent` recording flight altitude, camera type, and any sortie context
- **B.E.3** `naph:hasProvenanceChain` linking to a documented `naph:ProvenanceChain` (per [Module E](E-paradata-workflow.md))

An Enhanced-compliant record SHOULD additionally have:

- **B.E.4** Multiple surrogate variants — preservation master + access copy
- **B.E.5** `dcterms:format` for the access surrogate
- **B.E.6** `dcterms:extent` for original physical artefact (cm or pixels)

### B.2.3 Aspirational (B-aspirational)

An Aspirational-compliant record MUST additionally have:

- **B.A.1** At least one `naph:depicts` link to a `naph:Subject` (Place, HistoricEvent, etc.)
- **B.A.2** Each `naph:Place` MUST have `naph:placeAuthorityURI` pointing to GeoNames, Wikidata, or equivalent gazetteer
- **B.A.3** At least one `naph:linkedRecord` cross-reference to another collection (canonical authority records)

An Aspirational-compliant record SHOULD additionally have:

- **B.A.4** `skos:exactMatch` to Wikidata QIDs for places and events where a strong identity claim is possible
- **B.A.5** `skos:closeMatch` for less certain matches
- **B.A.6** Cross-cutting subjects expressed as `naph:Subject` instances with `skos:Concept` semantics

## B.3 Identifier requirements

### B.3.1 Persistence

Identifiers MUST be permanent. Once assigned, an identifier MUST NOT be re-assigned to a different record. A record that is withdrawn or merged MUST retain its original identifier, with appropriate `owl:deprecated`, redirect, or successor metadata.

### B.3.2 Resolvability

Identifiers MUST be HTTP(S) URIs that, when dereferenced, return a representation of the record. Acceptable representations include HTML (for humans), Turtle, JSON-LD, RDF/XML.

### B.3.3 Uniqueness

Identifiers MUST be globally unique. The recommended way to guarantee this is to root identifiers in a permanent URL service ([w3id.org](https://w3id.org/), [purl.org](https://purl.org/), [doi.org](https://doi.org/)) under a namespace owned by the institution.

### B.3.4 Composite identifier compatibility

For institutions with existing composite identifier schemes (e.g. NCAP's Collection / Sortie / Frame), NAPH provides distinct properties so the composite can be recovered:

- `naph:collectionCode` — the institutional collection prefix (e.g. `RAF`, `NARA`, `DOS`)
- `naph:sortieReference` — the sortie identifier within the collection
- `naph:frameNumber` — the sequential frame number within the sortie

Only `naph:hasIdentifier` is the canonical record identifier; the composite components are descriptive.

## B.4 Date handling

### B.4.1 Precision levels

NAPH supports three date precision levels:

| Precision | XSD datatype | Example | When to use |
|---|---|---|---|
| Day | `xsd:date` | `1944-03-28`^^xsd:date | Capture date is known exactly |
| Month | `xsd:gYearMonth` | `1944-03`^^xsd:gYearMonth | Year and month known, day not |
| Year | `xsd:gYear` | `1944`^^xsd:gYear | Only year known |

Free-text dates are NOT permitted. If a date cannot be resolved to at least year precision, the record cannot use `naph:capturedOn` and SHOULD use `naph:dateUncertaintyNote` (a free-text annotation) with structured fallback.

### B.4.2 Uncertainty

Approximate dates ("c. 1944") MUST be expressed using:

```turtle
ex:photo-X naph:capturedOn "1944"^^xsd:gYear ;
           naph:dateUncertainty "approximate"^^xsd:string ;
           naph:dateUncertaintyNote "c. 1944 — pre-war reconnaissance archive notes" .
```

### B.4.3 Date ranges

For a record covering a date range (e.g. an exposure series over 1943-1944), use `dcterms:temporal` with a `dcterms:PeriodOfTime`:

```turtle
ex:sortie-X dcterms:temporal [
    a dcterms:PeriodOfTime ;
    dcat:startDate "1943-09-01"^^xsd:date ;
    dcat:endDate "1944-03-31"^^xsd:date
] .
```

## B.5 Competency questions

The following research questions MUST be answerable by a single SPARQL query against any NAPH-compliant collection at the relevant tier. See [`docs/competency-questions.md`](../../../docs/competency-questions.md) for full SPARQL examples.

| ID | Question | Required tier |
|---|---|---|
| CQ1 | "How many records were captured in a given year range?" | Baseline |
| CQ2 | "Which records cover a given geographic polygon?" | Baseline (with GeoSPARQL-aware engine) |
| CQ3 | "Which records have rights statements compatible with open-access publication?" | Baseline |
| CQ4 | "Which records are linked to a given Wikidata historic event?" | Aspirational |
| CQ5 | "Which records came from a given provenance partnership?" | Enhanced |
| CQ6 | "What is the tier compliance distribution of this collection?" | Baseline |
| CQ7 | "Which records meet criteria X AND Y AND Z?" | Variable |
| CQ8 | "Which records were captured by aircraft/equipment matching criteria?" | Enhanced |

## B.6 Geospatial structure

### B.6.1 WKT polygon format

`naph:GeographicFootprint` instances MUST express the footprint as a WGS84 polygon in WKT (Well-Known Text):

```turtle
ex:footprint-001 a naph:GeographicFootprint ;
    naph:asWKT "POLYGON((-3.21 55.94, -3.16 55.94, -3.16 55.97, -3.21 55.97, -3.21 55.94))"^^geo:wktLiteral .
```

### B.6.2 Coordinate ordering

WKT in the `geo:wktLiteral` datatype follows GeoSPARQL convention: **longitude first, latitude second** (X Y, not Y X). This is the opposite of the human convention "lat, lon."

### B.6.3 Field-of-view derivation

For aerial imagery captured at known altitude and camera focal length, the geographic footprint can be derived geometrically. NAPH does not require this derivation but RECOMMENDS it where feasible because:

- It produces tighter footprints than rectangle-around-centroid
- It enables stereo overlap detection across adjacent frames
- It makes spatial queries more precise

The [photographic profile](../profiles/photographic.md) §3.2 specifies the recommended derivation for aerial photography.

## B.7 Worked examples

### B.7.1 Baseline record

```turtle
ex:photo-001 a naph:AerialPhotograph ;
    dcterms:type dctype:StillImage ;
    rdfs:label "Berlin reconnaissance frame 4023" ;
    naph:hasIdentifier "https://w3id.org/naph/photo/RAF-106G-UK-1655-4023" ;
    naph:partOfSortie ex:sortie-RAF-106G-UK-1655 ;
    naph:belongsToCollection ex:NCAPCollection ;
    naph:capturedOn "1944-03-28"^^xsd:date ;
    naph:coversArea ex:footprint-001 ;
    naph:hasRightsStatement ex:CrownCopyrightExpired ;
    naph:compliesWithTier naph:TierBaseline .
```

### B.7.2 Enhanced upgrade (adds digitisation, capture, provenance)

```turtle
ex:photo-001
    naph:hasDigitalSurrogate ex:photo-001-master, ex:photo-001-access ;
    naph:hasCaptureEvent ex:capture-001 ;
    naph:hasProvenanceChain ex:provenance-001 ;
    naph:compliesWithTier naph:TierEnhanced .
```

### B.7.3 Aspirational upgrade (adds semantic enrichment)

```turtle
ex:photo-001
    naph:depicts ex:place-Berlin ;
    naph:relatedToEvent ex:event-WWII-aerial-recon ;
    naph:linkedRecord <https://www.iwm.org.uk/collections/item/object/...> ;
    naph:compliesWithTier naph:TierAspirational .

ex:place-Berlin a naph:Place ;
    rdfs:label "Berlin" ;
    naph:placeAuthorityURI <https://www.geonames.org/2950159/berlin.html> ;
    skos:exactMatch <https://www.wikidata.org/wiki/Q64> .
```

## B.8 Cross-references

- [Module A — Capture & Imaging](A-capture-imaging.md)
- [Module C — Rights, Licensing & Ethics](C-rights-licensing-ethics.md)
- [Module D — Packaging & Publication](D-packaging-publication.md)
- [Module E — Paradata & Workflow](E-paradata-workflow.md)
- [Module F — QA & Validation](F-qa-validation.md)
- [Identifier policy decision tree](../../04-adoption-guidance/decision-trees/identifier-policy.md)
- [Date normalisation decision tree](../../04-adoption-guidance/decision-trees/date-normalisation.md)
