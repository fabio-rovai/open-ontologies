# Module E — Paradata & Workflow Documentation

**Status:** Normative · v1.0
**Applies to:** Enhanced and Aspirational tiers
**Defines:** the documentation of how digitisation, transcription, and processing decisions were made

## E.1 Purpose

**Paradata** is the documentation of process — what decisions were made, by whom, on what basis, with what tools, with what known limitations. Without paradata, computational research depends on records whose interpretation is opaque.

Module E specifies the minimum paradata that must accompany Enhanced and Aspirational records so:

- Researchers can assess fitness-for-purpose without contacting the institution
- Errors and biases in the digitisation pipeline are documented and traceable
- Re-processing decisions can be made with knowledge of how the original was made
- Audit trails support rights review, declassification work, and repatriation requests

## E.2 What paradata is NOT

Paradata is **not** descriptive metadata about the *artefact* (which is Module B). Paradata is metadata about the *process* of making and managing the digital surrogate.

| Metadata | Module |
|---|---|
| When the photograph was taken | Module B (`naph:capturedOn`) |
| When the photograph was scanned | Module E (DigitisationEvent.atTime) |
| Who is depicted | Module B (`naph:depicts`) |
| Who scanned the original | Module E (DigitisationEvent.wasAssociatedWith) |
| The geographic footprint | Module B (`naph:coversArea`) |
| How the geographic footprint was derived | Module E (process documentation) |

## E.3 Outcome requirements

### E.3.1 Enhanced (E-enhanced)

An Enhanced-compliant record MUST have:

- **E.E.1** A `naph:hasProvenanceChain` link to a `naph:ProvenanceChain` documenting the path from original artefact to current holding
- **E.E.2** Each significant transformation (scan, OCR, deskew, format conversion) MUST be expressed as a `prov:Activity`
- **E.E.3** Each transformation MUST link to the equipment, software, or operator responsible (`prov:wasAssociatedWith`)
- **E.E.4** Where automated tooling produced any field (e.g. AI-extracted text, computer-vision-derived footprint), the tool, version, and confidence MUST be recorded

An Enhanced-compliant record SHOULD have:

- **E.E.5** A workflow document (`naph:workflowDocument`) referencing the institution's published digitisation methodology for the relevant collection
- **E.E.6** Quality-control records (`naph:qcReview`) noting the reviewer and outcome

### E.3.2 Aspirational (E-aspirational)

An Aspirational-compliant record MUST additionally have:

- **E.A.1** Each AI-derived classification (subject, place, event linkage) MUST be marked with `prov:wasGeneratedBy` linking to the model+version, and `naph:confidence` (0.0–1.0)
- **E.A.2** Human validation events for AI-derived fields MUST be recorded as separate `prov:Activity` instances with the validator's identifier
- **E.A.3** A complete activity graph MUST be reachable from the record (every claim has a documented origin)

## E.4 Provenance chain structure

A `naph:ProvenanceChain` is a `prov:Bundle` of `prov:Activity` instances representing the lineage of the artefact from creation to current state.

### E.4.1 Minimum structure

```turtle
ex:provenance-001 a naph:ProvenanceChain, prov:Bundle ;
    rdfs:label "RAF 1944 → Air Ministry → MoD → NCAP transfer 2008" ;
    prov:hadMember ex:transfer-1944, ex:transfer-1946, ex:transfer-2008 .

ex:transfer-1944 a prov:Activity ;
    prov:atTime "1944-04-01T00:00:00Z"^^xsd:dateTime ;
    prov:wasAssociatedWith ex:RAF ;
    prov:generated ex:photo-001 .

ex:transfer-2008 a prov:Activity ;
    prov:startedAtTime "2008-01-01T00:00:00Z"^^xsd:dateTime ;
    prov:wasAssociatedWith ex:NCAP, ex:MoD ;
    prov:used ex:photo-001 .
```

### E.4.2 Significant events

The provenance chain MUST capture, at minimum:

- Original creation event (capture)
- Each formal transfer of custody between institutions
- Declassification or rights-status change events
- The digitisation event(s)
- Any post-digitisation transformations that produced derivative surrogates

## E.5 Digitisation event structure

```turtle
ex:scan-photo-001 a naph:DigitisationEvent, prov:Activity ;
    prov:startedAtTime "2024-03-12T09:23:14Z"^^xsd:dateTime ;
    prov:endedAtTime "2024-03-12T09:24:48Z"^^xsd:dateTime ;
    prov:wasAssociatedWith ex:operator-A, ex:scanner-Phase-One-IQ4 ;
    prov:used ex:negative-RAF-106G-UK-1655-4023 ;
    prov:generated ex:photo-001-master ;
    naph:scannerSettings [
        naph:resolution 1200 ;
        naph:colourMode "RGB" ;
        naph:colourProfile "AdobeRGB"
    ] .

ex:scanner-Phase-One-IQ4 a prov:Agent, prov:Entity ;
    rdfs:label "Phase One IQ4 150MP Achromatic" ;
    naph:firmwareVersion "v3.2.1" .

ex:operator-A a prov:Agent ;
    foaf:name "[Operator Name or anonymous identifier]" ;
    naph:operatorRole "scanning-operator" .
```

## E.6 AI-derived fields

For Aspirational tier, any field derived by automated/AI tooling MUST include:

```turtle
ex:photo-X naph:depicts ex:place-Y ;
    naph:placeDerivedBy ex:ai-classifier-001 ;
    naph:placeConfidence 0.87 .

ex:ai-classifier-001 a prov:SoftwareAgent, prov:Activity ;
    rdfs:label "Place classifier (vision-language model)" ;
    naph:modelName "gpt-4-vision" ;
    naph:modelVersion "2024-04-09" ;
    prov:atTime "2024-04-15T14:23:00Z"^^xsd:dateTime ;
    prov:used ex:photo-X-thumbnail .

ex:photo-X-Y-validation a prov:Activity ;
    rdfs:label "Human validation of AI-derived place" ;
    prov:atTime "2024-04-22T10:11:00Z"^^xsd:dateTime ;
    prov:wasAssociatedWith ex:reviewer-B ;
    prov:used ex:photo-X ;
    naph:validationOutcome "accepted" .
```

The validation event is critical — without it, an AI-derived claim is unverified and SHOULD be marked accordingly in queries.

## E.7 Workflow document

A workflow document is the institution's published methodology for the digitisation of a collection. It is referenced by URL, not embedded in every record.

```turtle
ex:photo-X naph:workflowDocument ex:NCAP-aerial-workflow-v3 .

ex:NCAP-aerial-workflow-v3 a foaf:Document ;
    dcterms:title "NCAP Aerial Photography Digitisation Workflow v3" ;
    dcterms:issued "2024-01-15"^^xsd:date ;
    dcterms:format "application/pdf" ;
    naph:applicableFrom "2024-01-15"^^xsd:date ;
    naph:applicableUntil "" ;
    foaf:page <https://example.org/ncap-workflow-v3.pdf> .
```

## E.8 Quality-control records

```turtle
ex:photo-X-qc-001 a prov:Activity ;
    rdfs:label "Quality control review" ;
    prov:atTime "2024-03-15T11:00:00Z"^^xsd:dateTime ;
    prov:wasAssociatedWith ex:qc-reviewer-A ;
    naph:qcLevel "frame" ;
    naph:qcOutcome "accepted" ;
    naph:qcNotes "Minor dust spots noted, within acceptable range for 1944 negative" .
```

QC levels:

- `frame` — individual frame review
- `feature-class` — review of all records of one type within a sortie
- `independent-manual` — independent review by a second reviewer

## E.9 Worked example: full Enhanced paradata

```turtle
@prefix naph: <https://w3id.org/naph/ontology#> .
@prefix prov: <http://www.w3.org/ns/prov#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:photo-004 naph:hasProvenanceChain ex:provenance-004 ;
    naph:hasDigitisationEvent ex:scan-photo-004 ;
    naph:workflowDocument ex:NCAP-aerial-workflow-v3 .

ex:provenance-004 a naph:ProvenanceChain, prov:Bundle ;
    rdfs:label "RAF 1943 → Air Ministry → MoD → NCAP transfer 2008" ;
    prov:hadMember
        ex:capture-1943,
        ex:transfer-AirMin-1944,
        ex:declassify-1972,
        ex:transfer-NCAP-2008,
        ex:scan-photo-004 .

ex:capture-1943 a naph:CaptureEvent, prov:Activity ;
    rdfs:label "Original aerial photographic exposure" ;
    prov:startedAtTime "1943-07-30T11:23:00Z"^^xsd:dateTime ;
    prov:wasAssociatedWith ex:RAF-541-Squadron, ex:F-52-camera ;
    prov:generated ex:negative-RAF-541-HAM-1287 ;
    naph:flightAltitude 9144.0 ;
    naph:cameraType "F.52 split-pair vertical, 36-inch lens" .

ex:declassify-1972 a prov:Activity ;
    rdfs:label "Declassification under 30-year rule" ;
    prov:atTime "1972-01-01T00:00:00Z"^^xsd:dateTime ;
    prov:wasAssociatedWith ex:UK-Public-Records-Office .

ex:scan-photo-004 a naph:DigitisationEvent, prov:Activity ;
    rdfs:label "Preservation digitisation" ;
    prov:startedAtTime "2018-04-12T09:17:00Z"^^xsd:dateTime ;
    prov:wasAssociatedWith ex:NCAP-operator-12, ex:scanner-Phase-One-IQ3 ;
    prov:used ex:negative-RAF-541-HAM-1287 ;
    prov:generated ex:photo-004-master ;
    naph:scannerSettings [
        naph:resolution 1200 ;
        naph:colourMode "RGB" ;
        naph:colourProfile "AdobeRGB"
    ] .
```

## E.10 Validation

A SHACL shape (`naph:ProvenanceChainShape`) checks:

- Every Enhanced record has a `naph:hasProvenanceChain`
- Each `prov:Activity` in the chain has `prov:atTime` or `prov:startedAtTime`
- Each `prov:Activity` has `prov:wasAssociatedWith`
- For Aspirational AI-derived fields: `naph:confidence` and `prov:wasGeneratedBy` are present

## E.11 Common errors

| Error | Why it matters | Remediation |
|---|---|---|
| Provenance chain only at collection level, not per-record | Cannot trace individual artefact lineage | Link each record to its provenance bundle |
| AI-derived claim without confidence score | Cannot distinguish high-confidence from speculative | Add `naph:confidence` |
| Missing operator/equipment | Audit trail incomplete | Document the digitisation event with `prov:wasAssociatedWith` |
| Free-text "scanned 2018" instead of structured event | Not queryable | Use `naph:DigitisationEvent` |

## E.12 Cross-references

- [Module A — Capture & Imaging](A-capture-imaging.md)
- [Module B — Metadata & Data Structures](B-metadata-data-structures.md)
- [Module C — Rights, Licensing & Ethics](C-rights-licensing-ethics.md) (declassification events feed rights determinations)
- [PROV-O specification](https://www.w3.org/TR/prov-o/)
