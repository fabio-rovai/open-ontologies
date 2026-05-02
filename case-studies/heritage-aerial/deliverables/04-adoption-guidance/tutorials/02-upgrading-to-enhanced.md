# Tutorial 2 — Upgrading to Enhanced Tier

You have a Baseline-compliant record (from [Tutorial 1](01-your-first-naph-record.md)). Now upgrade it to Enhanced tier.

By the end of this tutorial you will:

- Have an Enhanced-compliant record
- Understand the Enhanced-tier additional requirements
- Know how to model digitisation provenance, capture context, and provenance chains

**Estimated time:** 45-60 minutes
**Prerequisites:** Tutorial 1

## What changes from Baseline to Enhanced

Enhanced adds three structural pieces:

1. **Digital surrogates** — multiple variants (preservation master + access copy)
2. **Capture event details** — full capture context with operator, equipment
3. **Provenance chain** — documented lineage from creation to current state

Optional Enhanced additions:

4. Workflow document reference
5. Quality-control review records

## Step 1 — Add digital surrogates

A NAPH Enhanced record has at least two surrogate variants:

```turtle
ex:photo-001-master a naph:DigitalSurrogate ;
    rdfs:label "Preservation master" ;
    naph:digitisedOn "2018-04-12"^^xsd:date ;
    naph:scanResolution 1200 ;
    naph:fileFormat "image/tiff" ;
    naph:digitisedBy ex:NCAP ;
    naph:hasChecksum "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855" .

ex:photo-001-access a naph:DigitalSurrogate ;
    rdfs:label "Access copy" ;
    prov:wasDerivedFrom ex:photo-001-master ;
    naph:digitisedOn "2018-04-12"^^xsd:date ;
    naph:scanResolution 600 ;
    naph:fileFormat "image/jp2" ;
    naph:digitisedBy ex:NCAP .
```

Note:

- The master is at higher resolution (1200 DPI), the access copy is at lower (600 DPI)
- The master has a SHA-256 checksum for integrity verification
- The access copy is linked to the master via `prov:wasDerivedFrom`
- File formats are MIME types (`image/tiff`, `image/jp2`)

## Step 2 — Update the photograph to reference surrogates

```turtle
ex:photo-001 naph:hasDigitalSurrogate ex:photo-001-master, ex:photo-001-access .
```

The comma separates multiple values for the same property — this is Turtle shorthand.

## Step 3 — Document the digitisation event

The DigitisationEvent records who, when, where, with what equipment:

```turtle
ex:scan-photo-001 a naph:DigitisationEvent, prov:Activity ;
    prov:startedAtTime "2018-04-12T09:23:14Z"^^xsd:dateTime ;
    prov:endedAtTime "2018-04-12T09:24:48Z"^^xsd:dateTime ;
    prov:wasAssociatedWith ex:operator-A, ex:scanner-Phase-One-IQ4 ;
    prov:used ex:negative-RAF-106G-UK-1655-4023 ;
    prov:generated ex:photo-001-master .
```

And define the agents:

```turtle
ex:operator-A a prov:Agent ;
    foaf:name "[Operator Name or anonymous identifier]" ;
    naph:operatorRole "scanning-operator" .

ex:scanner-Phase-One-IQ4 a prov:Agent, prov:Entity ;
    rdfs:label "Phase One IQ4 150MP" ;
    naph:firmwareVersion "v3.2.1" .
```

## Step 4 — Document the provenance chain

The ProvenanceChain captures the artefact's history from creation to current state:

```turtle
ex:provenance-001 a naph:ProvenanceChain, prov:Bundle ;
    rdfs:label "RAF 1944 → Air Ministry → MoD → declassified 1972 → NCAP transfer 2008" ;
    prov:hadMember
        ex:capture-event-1944,
        ex:transfer-AirMin-1944,
        ex:declassify-1972,
        ex:transfer-MoD-2008,
        ex:scan-photo-001 .

ex:capture-event-1944 a naph:CaptureEvent, prov:Activity ;
    rdfs:label "Original aerial photographic exposure" ;
    prov:atTime "1944-03-28T11:23:00Z"^^xsd:dateTime ;
    prov:wasAssociatedWith ex:RAF-540-Squadron, ex:F-52-camera ;
    prov:generated ex:negative-RAF-106G-UK-1655-4023 ;
    naph:flightAltitude 9144.0 .

ex:declassify-1972 a prov:Activity ;
    rdfs:label "Declassification under UK 30-Year Rule" ;
    prov:atTime "1972-01-01T00:00:00Z"^^xsd:dateTime ;
    prov:wasAssociatedWith ex:UK-Public-Records-Office .
```

This provenance chain explicitly records:

- The original capture (1944)
- The transfer to Air Ministry holdings (1944)
- The declassification event (1972 — under the 30-year rule)
- The transfer to NCAP (2008)
- The scanning event (2018)

A researcher looking at the record can audit the entire history.

## Step 5 — Update the photograph to reference provenance

```turtle
ex:photo-001
    naph:hasCaptureEvent ex:capture-event-1944 ;
    naph:hasProvenanceChain ex:provenance-001 .
```

## Step 6 — Update tier compliance

Change the tier declaration:

```turtle
ex:photo-001 naph:compliesWithTier naph:TierEnhanced .
```

(Replacing the previous `naph:TierBaseline`.)

## Step 7 — Validate

Save and validate:

```bash
echo "clear
load /path/to/naph-core.ttl
load my-enhanced-record.ttl
shacl /path/to/naph-shapes.ttl" | open-ontologies batch
```

Expected output:

```json
"conforms": true,
"violation_count": 0,
```

If validation fails, common Enhanced-tier issues:

- "Enhanced tier requires at least one DigitalSurrogate" — missing `naph:hasDigitalSurrogate`
- "Enhanced tier requires CaptureEvent metadata" — missing `naph:hasCaptureEvent`
- "Enhanced tier requires a documented provenance chain" — missing `naph:hasProvenanceChain`
- "Scan resolution must be at least 300 DPI" — `naph:scanResolution` below threshold

## Step 8 — Self-assess

```bash
python3 /path/to/pipeline/self-assessment.py my-enhanced-record.ttl
```

Output:

```
Tier distribution:
  Enhanced       1
```

You now have an Enhanced-tier record.

## What you've gained

A researcher querying your collection can now:

- Filter by digitisation date (`naph:digitisedOn`) — e.g. "find records digitised after 2020"
- Audit provenance — "find all records that came via a NARA partnership"
- Trace lineage — "what happened to this artefact between 1944 and now?"
- Filter by capture context — "find records captured at altitudes above 6000m"

These queries are impossible at Baseline. Enhanced unlocks reproducible computational research.

## Common pitfalls

### Documenting events that didn't happen

If you don't have records for a transfer event, don't make one up. Document only what you actually know.

### Conflating capture date and digitisation date

`naph:capturedOn` is the original photographic exposure date.
`naph:digitisedOn` is when the digital surrogate was made.

These are typically decades apart. Don't confuse them.

### Resolution below 300 DPI

The Baseline shape allows surrogates of any resolution; the DigitalSurrogateShape requires 300+ DPI. Most institutional preservation masters are 600-1200 DPI.

If your only available surrogate is below 300 DPI, you can't claim Enhanced tier — re-scan or accept Baseline-only.

### Operator names and privacy

Operator names in `prov:wasAssociatedWith` may need to be anonymised for staff privacy. Use `naph:operatorRole` as a description and a stable identifier (`ex:operator-A`) without including a real name in `foaf:name` if your institutional policy doesn't permit it.

## Next steps

- **Tutorial 3** — Reach Aspirational tier (subject classification, place authorities)
- **Tutorial 4** — Bulk ingest from CSV
- Real-world examples: [sample-photographs.ttl](../../../data/sample-photographs.ttl) records 4-7 are Enhanced tier
