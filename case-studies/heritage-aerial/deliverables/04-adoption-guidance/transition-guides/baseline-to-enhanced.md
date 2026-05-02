# Tier Transition Guide: Baseline → Enhanced

You have a NAPH Baseline-compliant collection. This guide walks through what's needed to upgrade to Enhanced tier.

## What Enhanced adds

Three categories of metadata not required at Baseline:

1. **Digital surrogate variants** — preservation master + access copy (per record)
2. **Capture context** — altitude, camera type, sortie metadata (per record)
3. **Provenance chain** — documented lineage from creation to current holding (per record)

## Effort at scale

For a 100,000-record collection (notional figures):

| Activity | Per-record effort | Total at 100k |
|---|---|---|
| Generate access copies from existing masters | Largely automatable | ~2 FTE-days |
| Extract capture context from existing internal data | 0.0005/record | ~50 FTE-days |
| Document provenance chains (per sortie) | 0.5 FTE-days/sortie × 200 sorties | ~100 FTE-days |
| QA + sampling-based review | 0.00005/record (5%) | ~5 FTE-days |
| Total | | ~157 FTE-days |

At a notional £700/day senior-developer rate, the upgrade for 100k records is roughly **£110,000** in capital cost.

The largest cost driver is provenance documentation — domain-expert work that doesn't scale automatically.

## Sequencing

### Phase 1 — Surrogate variants (week 1-2)

If your existing preservation masters are 600+ DPI TIFFs:

1. Generate access copies (typically 600 DPI JP2 or smaller)
2. Compute checksums (SHA-256) for both
3. Add `naph:hasDigitalSurrogate` triples for each variant
4. Link variants via `prov:wasDerivedFrom`

Tooling: ImageMagick, Vips, KDU (KDU JPEG2000) for batch generation.

### Phase 2 — Capture context (weeks 3-6)

Extract from existing internal data:

- Sortie metadata (squadron, aircraft, mission type) — typically in the sortie log
- Altitude and camera type — typically in the same log or photographer's notes
- Geographic context for the sortie as a whole

Bulk-update via SPARQL `INSERT DATA` or by re-ingesting from updated CSV.

Where capture context isn't available in source records, document the gap explicitly:

```turtle
ex:photo-X naph:hasCaptureEvent ex:capture-X .
ex:capture-X a naph:CaptureEvent ;
    naph:flightAltitude 0.0 ;  # placeholder — flag as unknown
    rdfs:comment "Altitude not recorded; collection-level estimate ~9000m for 1943-1944 RAF reconnaissance." .
```

### Phase 3 — Provenance chains (weeks 7-12)

This is the heaviest lift. Per sortie, document:

- Original capture event (date, operator, equipment)
- Each transfer of custody (when, between whom, why)
- Declassification events (where applicable)
- Digitisation events

For RAF reconnaissance the typical chain is:

```
RAF capture (date) → Air Ministry archive (post-flight) → MoD (post-merger)
→ declassification (1972 under 30-year rule) → NCAP transfer (2008)
→ digitisation (2018)
```

For NARA-sourced material:

```
USAAF capture → US Strategic Bombing Survey → NARA accession
→ NCAP digitisation partnership (2016)
```

Document at the sortie level where possible; record-level provenance is rarely necessary.

### Phase 4 — QA + validation (week 13)

For each upgraded record:

1. Run SHACL validation against the Enhanced shape
2. Sample 5% for human review
3. Generate validation report
4. Update tier compliance declaration

## Common challenges

### Internal data isn't in a structured form

If your sortie logs are scanned PDFs, not databases:

1. OCR the logs (Tesseract, Azure Form Recogniser, Adobe Acrobat OCR)
2. Manually validate sample
3. Transcribe relevant fields to CSV
4. Ingest

Plan extra time — OCR of historic typescript is rarely > 95% accurate without manual correction.

### Provenance chains differ across sub-collections

Different sub-collections may have different transfer histories:

- Domestic UK material: Air Ministry → MoD → NCAP
- US-partnership material: USAAF → NARA → NCAP digitisation partnership
- Captured / seized material: Luftwaffe → US Strategic Bombing Survey → NARA → NCAP

Document a standard provenance template per sub-collection. Apply the relevant template per record.

### Older digitisations don't meet preservation-master standard

If you have records digitised pre-2010 at 300 DPI JPEG:

1. Mark the existing surrogate as `prov:wasDerivedFrom` of the original physical artefact
2. Plan re-digitisation at preservation-grade for high-value records
3. Don't claim Enhanced tier for records with non-archival surrogates

### Provenance disputes

If different sources disagree on the provenance chain (e.g. transfer date discrepancy between archival records and oral history):

1. Document the more authoritative source as the primary chain
2. Note the discrepancy in `rdfs:comment`
3. Don't fabricate a single confident answer — note the uncertainty

## Validation checklist

Before claiming Enhanced tier for any record:

- [ ] At least 2 surrogate variants present
- [ ] Master is 600+ DPI lossless format (TIFF or JP2 lossless)
- [ ] Access copy is documented as derived from master
- [ ] CaptureEvent is present with at least altitude and camera type
- [ ] Sortie has squadron and aircraft (where known)
- [ ] ProvenanceChain has at least 2 events (capture + current holding)
- [ ] All Activity instances have time + agent
- [ ] SHACL validation passes for Enhanced shape

## Pitfalls

### Falsifying provenance for records you don't actually know

Don't claim a record went through specific transfer events if you can't evidence them. Better to leave the chain shorter and accurate than to fabricate.

### Marking digitisation events as "DigitisationEvent" without metadata

A naph:DigitisationEvent must have at least:
- `prov:atTime` or `prov:startedAtTime`
- `prov:wasAssociatedWith` (operator or equipment)
- `prov:generated` (the surrogate it produced)

Without these, the SHACL validation fails.

### Over-detailed provenance

Ten transfer events for a record where five would do. The standard wants enough provenance to support audit, not exhaustive bureaucratic detail.

A reasonable rule: include events that materially changed the record's status (transfer of custody, classification change, digitisation). Skip routine internal cataloguing events.

## Next steps

After Enhanced rollout:

1. Update [compliance registry submission](../../../registry/compliance-declaration-template.ttl) with new tier counts
2. Plan Aspirational tier for high-research-value subset
3. Consider IIIF Presentation 3.0 manifest publication (if not already)
4. Consider SPARQL endpoint operation for federation participation

See [Tier Transition Guide: Enhanced → Aspirational](enhanced-to-aspirational.md).

## Cross-references

- [Tutorial 2 — Upgrading to Enhanced](../tutorials/02-upgrading-to-enhanced.md) — single-record walkthrough
- [Module E — Paradata & Workflow](../../01-standard/modules/E-paradata-workflow.md) — provenance specification
- [Cost & effort analysis](../../docs/cost-effort-analysis.md) — full cost breakdown
- [Validation checklist](../validation-checklists.md)
