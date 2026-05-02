# Module A — Capture & Imaging

**Status:** Normative · v1.0
**Applies to:** All NAPH-compliant records, all tiers
**Defines:** outcome requirements for the digital capture step that produces the digital surrogate

## A.1 Purpose

Module A specifies what a digital surrogate must satisfy at the moment of capture so that downstream processing — storage, packaging, serving, computational reuse — does not lose information that is recoverable only by re-imaging the physical artefact.

Most existing digitisation guidance addresses *processes* (resolution settings, colour profiles, file naming). Module A specifies *outcomes* — what the surrogate must demonstrate, regardless of the workflow that produced it. This allows institutions to reuse existing equipment, vendors, and workflows.

## A.2 Outcome requirements

### A.2.1 Baseline

A Baseline-compliant digital surrogate MUST:

- **A.B.1** Be a single uncompressed or losslessly-compressed image file (TIFF, JP2 lossless, PNG)
- **A.B.2** Have a documented capture date with at least year-month precision (`xsd:gYearMonth` or higher)
- **A.B.3** Identify the institution responsible for capture (`naph:digitisedBy`)
- **A.B.4** Use a globally unique stable identifier (Module B.B.1)
- **A.B.5** Be rights-tagged (Module C.B.1) at the point of capture

A Baseline-compliant digital surrogate SHOULD:

- **A.B.6** Have minimum 300 DPI for archival use
- **A.B.7** Embed colour profile metadata (sRGB, AdobeRGB, or wider gamut)
- **A.B.8** Preserve EXIF or equivalent technical metadata if present in the source

### A.2.2 Enhanced

An Enhanced-compliant digital surrogate MUST:

- **A.E.1** Be available in at least two surrogate variants: a preservation master and at least one access copy
- **A.E.2** Document the relationship between variants (`prov:wasDerivedFrom`)
- **A.E.3** Record the digitisation event explicitly (`naph:hasDigitisationEvent`) with operator, equipment, settings
- **A.E.4** Have minimum 600 DPI for the preservation master

An Enhanced-compliant digital surrogate SHOULD:

- **A.E.5** Capture and document all colour-calibration steps applied
- **A.E.6** Record any human or automated post-capture corrections (rotation, deskew, crop) as separate `prov:Activity` instances chained to the original capture event
- **A.E.7** Include a SHA-256 checksum at the time of preservation master creation

### A.2.3 Aspirational

An Aspirational-compliant digital surrogate MUST:

- **A.A.1** Use a standardised image format with full structural metadata (TIFF 6.0 with appropriate baseline tags, or JP2 with mandatory boxes)
- **A.A.2** Embed colour profile metadata in the file (not only externally documented)
- **A.A.3** Provide IIIF Image API service endpoints for the access copy ([Module D.A.3](D-packaging-publication.md))

An Aspirational-compliant digital surrogate SHOULD:

- **A.A.4** Capture and embed device-specific calibration data (sensor characterisation, geometric distortion correction parameters)
- **A.A.5** Provide ImageMagick / Vips compatible technical metadata for automated processing pipelines

## A.3 Why these are outcome requirements, not method requirements

Module A deliberately avoids specifying:

- A specific scanning device or vendor
- A required scan resolution beyond minimums (because requirements vary by source size and intended use)
- A specific file format among those that satisfy the constraints
- A specific colour-management workflow

This is because:

1. **Institutions have existing equipment and contracts.** Mandating a specific vendor would force uneconomical replacement.
2. **Best-practice workflows evolve.** Specifying outcomes lets the standard remain valid as workflows improve.
3. **Source materials differ.** A panchromatic 35mm aerial negative requires different handling from a glass plate or a born-digital file.

## A.4 Worked examples

### A.4.1 Baseline-compliant capture record

```turtle
@prefix naph: <https://w3id.org/naph/ontology#> .
@prefix prov: <http://www.w3.org/ns/prov#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:photo-001-master a naph:DigitalSurrogate ;
    naph:digitisedOn "2024-03-12"^^xsd:date ;
    naph:scanResolution 600 ;
    naph:fileFormat "image/tiff" ;
    naph:digitisedBy ex:NCAP ;
    naph:hasIdentifier "https://w3id.org/naph/example/photo-001/master" .
```

### A.4.2 Enhanced — preservation master + access copy

```turtle
ex:photo-001-master a naph:DigitalSurrogate ;
    naph:digitisedOn "2024-03-12"^^xsd:date ;
    naph:scanResolution 1200 ;
    naph:fileFormat "image/tiff" ;
    naph:digitisedBy ex:NCAP ;
    naph:colourProfile "AdobeRGB" ;
    naph:hasChecksum "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855" .

ex:photo-001-access a naph:DigitalSurrogate ;
    prov:wasDerivedFrom ex:photo-001-master ;
    naph:digitisedOn "2024-03-12"^^xsd:date ;
    naph:scanResolution 600 ;
    naph:fileFormat "image/jp2" ;
    naph:digitisedBy ex:NCAP .

ex:photo-001-rotation a prov:Activity ;
    rdfs:label "Auto-deskew rotation" ;
    prov:wasInformedBy ex:photo-001-master ;
    prov:generatedAtTime "2024-03-12T14:23:00Z"^^xsd:dateTime .
```

## A.5 Validation

A SHACL shape (`naph:DigitalSurrogateShape`) checks:

- `naph:digitisedOn` is present and is a valid date
- `naph:scanResolution` is present and ≥300 (Baseline) / ≥600 (Enhanced master)
- `naph:fileFormat` is present and is a recognised MIME type

See [`ontology/naph-shapes.ttl`](../../../ontology/naph-shapes.ttl) for the canonical shape definition.

## A.6 Common errors

| Error | Why it matters | Remediation |
|---|---|---|
| Free-text date ("March 2018") instead of ISO | Breaks date queries and aggregation | Normalise via [date decision tree](../../04-adoption-guidance/decision-trees/date-normalisation.md) |
| Lossy JPEG used as preservation master | Information loss is irreversible | Re-scan original, store TIFF or lossless JP2 |
| Missing colour profile | Breaks colour-accurate downstream rendering | Embed sRGB by default; AdobeRGB for fine art |
| Operator name only on contract paperwork, not in metadata | Provenance gap | Use `naph:digitisedBy` to link to a `prov:Agent` |

## A.7 Cross-references

- [Module B](B-metadata-data-structures.md) — descriptive and structural metadata
- [Module D](D-packaging-publication.md) — packaging and IIIF service binding
- [Module E](E-paradata-workflow.md) — workflow documentation
- [Date normalisation decision tree](../../04-adoption-guidance/decision-trees/date-normalisation.md)
