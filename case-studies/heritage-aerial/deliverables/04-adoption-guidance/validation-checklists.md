# Validation Checklists

Self-assessment checklists for each tier and module. Use these for pre-flight checks before running formal SHACL validation, and as audit instruments.

## How to use these checklists

For each module, work through the checklist for your target tier. A `MUST` item that is missing means non-compliance — you cannot claim that tier.

The companion automated tool is [`pipeline/self-assessment.py`](../../pipeline/self-assessment.py) which runs equivalent checks programmatically and produces a JSON report.

---

## Tier-level summary checklist

Tick the column corresponding to your target tier.

| Item | B | E | A | Where to fix |
|---|---|---|---|---|
| Every record has a `naph:hasIdentifier` URI | ✅ | ✅ | ✅ | [Module B.B.1](../01-standard/modules/B-metadata-data-structures.md) |
| Every identifier resolves to RDF (Turtle / JSON-LD) | ✅ | ✅ | ✅ | [Module D.B.2](../01-standard/modules/D-packaging-publication.md) |
| Every record has `naph:capturedOn` (ISO date) | ✅ | ✅ | ✅ | [Module B.B.2](../01-standard/modules/B-metadata-data-structures.md) |
| Every record has WGS84 polygon footprint | ✅ | ✅ | ✅ | [Module B.B.3](../01-standard/modules/B-metadata-data-structures.md) |
| Every record has machine-readable rights URI | ✅ | ✅ | ✅ | [Module C.B.1](../01-standard/modules/C-rights-licensing-ethics.md) |
| Every record links to its sortie / accession event | ✅ | ✅ | ✅ | [Module B.B.4](../01-standard/modules/B-metadata-data-structures.md) |
| Every record links to its collection | ✅ | ✅ | ✅ | [Module B.B.5](../01-standard/modules/B-metadata-data-structures.md) |
| Collection has a published manifest (BagIt / RO-Crate / DCAT) | ✅ | ✅ | ✅ | [Module D.B.1](../01-standard/modules/D-packaging-publication.md) |
| Bulk download mechanism is documented | ✅ | ✅ | ✅ | [Module D.B.4](../01-standard/modules/D-packaging-publication.md) |
| SHACL validation passes for Baseline shape | ✅ | ✅ | ✅ | run validation toolkit |
| Validation report published at stable URL | ✅ | ✅ | ✅ | [Module F.B.2](../01-standard/modules/F-qa-validation.md) |
| Every record has at least one digital surrogate | | ✅ | ✅ | [Module B.E.1](../01-standard/modules/B-metadata-data-structures.md) |
| Every record has a CaptureEvent | | ✅ | ✅ | [Module B.E.2](../01-standard/modules/B-metadata-data-structures.md) |
| Every record has a ProvenanceChain | | ✅ | ✅ | [Module B.E.3](../01-standard/modules/B-metadata-data-structures.md) |
| Digitisation provenance documented per record | | ✅ | ✅ | [Module E.E.3](../01-standard/modules/E-paradata-workflow.md) |
| Quarterly re-validation in place | | ✅ | ✅ | [Module F.B.3](../01-standard/modules/F-qa-validation.md) |
| Collection has a SPARQL endpoint | | (S) | (S) | [Module D.E.4](../01-standard/modules/D-packaging-publication.md) |
| Records have IIIF Presentation 3.0 manifests | | (S) | ✅ | [Module D.A.3](../01-standard/modules/D-packaging-publication.md) |
| Every record has subject classification (`naph:depicts`) | | | ✅ | [Module B.A.1](../01-standard/modules/B-metadata-data-structures.md) |
| Every Place has authority URI (GeoNames / Wikidata) | | | ✅ | [Module B.A.2](../01-standard/modules/B-metadata-data-structures.md) |
| Every record has cross-collection links (`naph:linkedRecord`) | | | ✅ | [Module B.A.3](../01-standard/modules/B-metadata-data-structures.md) |
| AI-derived fields have confidence + tool provenance | | | ✅ | [Module E.A.1](../01-standard/modules/E-paradata-workflow.md) |
| Computational reuse tests pass | | | ✅ | [Module F.A.1](../01-standard/modules/F-qa-validation.md) |
| External link health monitored | | | ✅ | [Module F.A.3](../01-standard/modules/F-qa-validation.md) |

Legend: ✅ = MUST · (S) = SHOULD · blank = not required for this tier

---

## Module-by-module detailed checklists

### Module A — Capture & Imaging

#### A-Baseline checklist

- [ ] Every digital surrogate is in TIFF, JP2-lossless, or PNG (no JPEG masters)
- [ ] Every digital surrogate has `naph:digitisedOn` ≥ year-month precision
- [ ] Every digital surrogate has `naph:digitisedBy`
- [ ] Resolution ≥ 300 DPI for the preservation master
- [ ] Colour profile documented (embedded or in metadata)

#### A-Enhanced checklist

- [ ] Every record has at least 2 surrogate variants (preservation master + access copy)
- [ ] Surrogate variants linked via `prov:wasDerivedFrom`
- [ ] DigitisationEvent recorded with operator, equipment, settings
- [ ] Preservation master ≥ 600 DPI

#### A-Aspirational checklist

- [ ] Image format includes full structural metadata (TIFF 6.0 or JP2 boxes)
- [ ] Colour profile embedded in file
- [ ] IIIF Image API service running and resolvable

### Module B — Metadata

#### B-Baseline checklist

- [ ] Every record has `naph:hasIdentifier` (URI)
- [ ] Identifiers resolve via HTTP(S) to RDF representation
- [ ] Every record has `naph:capturedOn` (xsd:date / gYearMonth / gYear)
- [ ] Every record has `naph:coversArea` → `naph:GeographicFootprint` with WKT polygon
- [ ] WGS84 (CRS84) coordinate ordering: longitude first, latitude second
- [ ] Coordinates within valid ranges (lat -90 to 90, lon -180 to 180)
- [ ] Every record has `naph:partOfSortie` (or equivalent acquisition link)
- [ ] Every record has `naph:belongsToCollection`
- [ ] Every record has RDF type assertion (`naph:Photograph` / `naph:Manuscript` / etc.)
- [ ] Every record has `dcterms:type` (DCMI Type vocabulary)
- [ ] Every record has `rdfs:label`

#### B-Enhanced checklist

- [ ] Every record has `naph:hasDigitalSurrogate`
- [ ] Every record has `naph:hasCaptureEvent` with altitude + camera type
- [ ] Every record has `naph:hasProvenanceChain`
- [ ] Multiple surrogate variants present (preservation master + access)

#### B-Aspirational checklist

- [ ] Every record has at least one `naph:depicts` link
- [ ] Every `naph:Place` has `naph:placeAuthorityURI` (GeoNames / Wikidata)
- [ ] Every record has at least one `naph:linkedRecord` cross-reference
- [ ] Subject `skos:exactMatch` or `skos:closeMatch` to authority

### Module C — Rights, Licensing & Ethics

#### C-Baseline checklist

- [ ] Every record has `naph:hasRightsStatement`
- [ ] Every RightsStatement has `naph:rightsURI` (registered authority)
- [ ] Every RightsStatement has `naph:rightsLabel`
- [ ] Rights URIs use canonical `/vocab/` form (rightsstatements.org)

#### C-Enhanced checklist

- [ ] Rights determination basis documented (`naph:rightsReviewBasis`)
- [ ] `naph:rightsReviewedOn` present on every RightsStatement
- [ ] `naph:rightsReviewedBy` present
- [ ] Ethics statement attached to sensitive material (where applicable)

#### C-Aspirational checklist

- [ ] TK / BC labels applied where culturally appropriate
- [ ] Data subject linkage for records with identifiable living individuals

### Module D — Packaging & Publication

#### D-Baseline checklist

- [ ] Manifest published (BagIt / RO-Crate / DCAT)
- [ ] Each record's URI returns RDF on content negotiation
- [ ] Sitemap published listing all record URIs
- [ ] Bulk download URL documented

#### D-Enhanced checklist

- [ ] Standardised packaging (BagIt or RO-Crate) with checksums
- [ ] Packaging format and version explicit in `bag-info.txt` or equivalent
- [ ] SPARQL endpoint (recommended)
- [ ] IIIF Presentation 3.0 manifests for imagery (recommended)

#### D-Aspirational checklist

- [ ] IIIF Image API 3.0 service running for each access surrogate
- [ ] Federated SPARQL endpoint with VOID descriptor
- [ ] OAI-PMH harvesting supported

### Module E — Paradata & Workflow

#### E-Enhanced checklist

- [ ] ProvenanceChain documented per record (or per sortie if uniform)
- [ ] Each transformation expressed as `prov:Activity`
- [ ] Each Activity has `prov:atTime` or `prov:startedAtTime`
- [ ] Each Activity has `prov:wasAssociatedWith`
- [ ] Workflow document referenced from records

#### E-Aspirational checklist

- [ ] AI-derived fields have `naph:confidence`
- [ ] AI-derived fields have `prov:wasGeneratedBy` linking to model+version
- [ ] Human validation events recorded as `prov:Activity` with validator
- [ ] Complete activity graph reachable from each record

### Module F — QA & Validation

#### F-Baseline checklist

- [ ] SHACL validation passes for Baseline shape
- [ ] Validation report published at stable URL
- [ ] Validation re-run quarterly or on change
- [ ] 1% sample human review per audit

#### F-Enhanced checklist

- [ ] Structured `naph:ConformanceReport` produced
- [ ] Deviations documented with rationale
- [ ] Validation runs in CI/CD if collection updated programmatically
- [ ] 5% sample human review per cycle

#### F-Aspirational checklist

- [ ] Computational reuse tests run as part of validation
- [ ] Test fixture set of 50+ records with expected outcomes maintained
- [ ] Cross-collection links validated (broken links flagged)
- [ ] 10% stratified sample human review per cycle

---

## Common pre-flight failures

Things that frequently cause SHACL validation to fail:

1. **Free-text dates** — "March 1944" instead of `xsd:gYearMonth` `1944-03`
2. **Lat/lon order** — using `POLYGON((lat lon, ...))` instead of `POLYGON((lon lat, ...))`
3. **Identifiers with `/page/` form** — using `https://rightsstatements.org/page/` instead of `http://rightsstatements.org/vocab/`
4. **Missing `dcterms:type`** — every record SHOULD have `dctype:StillImage` (or equivalent)
5. **Polygon doesn't close** — first and last coordinates of the WKT polygon must be identical
6. **Missing collection link** — record has sortie but no `naph:belongsToCollection`
7. **Mixed tier compliance claim** — record claims `naph:TierEnhanced` but lacks DigitalSurrogate
8. **Stale rights review** — no `naph:rightsReviewedOn` more recent than spec policy

The [self-assessment tool](../../pipeline/self-assessment.py) catches all of these.

## Cross-references

- [Module specifications](../01-standard/modules/)
- [Decision trees](decision-trees/)
- [Worked examples](worked-examples.md)
- [Self-assessment script](../../pipeline/self-assessment.py)
