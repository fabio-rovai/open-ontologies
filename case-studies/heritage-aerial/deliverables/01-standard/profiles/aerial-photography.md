# Photographic Profile

**Status:** Normative · v1.0
**Applies to:** photographic heritage collections — aerial, documentary, fine-art, family
**Implements:** the NAPH standard for image-as-record collections

## P.1 Profile scope

The Photographic Profile specialises NAPH for collections where the primary unit is a discrete photographic image. This includes:

- Aerial reconnaissance photography (the case study domain — NCAP, IWM, NARA archives)
- Studio and commercial photography archives
- News photography and photojournalism archives
- Fine-art photography
- Family and community photo archives
- Born-digital photographic archives

It does NOT cover:

- Manuscripts and archives — see [manuscripts-archives.md](manuscripts-archives.md)
- Heterogeneous thematic collections — see [integrated-thematic.md](integrated-thematic.md)

## P.2 Profile-specific class

```turtle
naph:Photograph rdfs:subClassOf naph:AerialPhotograph .  # AerialPhotograph kept as backwards-compatible
                                                          # alias; new collections SHOULD use Photograph
```

For aerial-specific subtype:

```turtle
naph:AerialPhotograph rdfs:subClassOf naph:Photograph .
naph:GroundPhotograph rdfs:subClassOf naph:Photograph .
naph:StudioPhotograph rdfs:subClassOf naph:Photograph .
```

## P.3 Profile-specific properties

### P.3.1 Capture-orientation

Photographs MAY be vertical (looking down) or oblique (looking forward at angle). For aerial work this is critical for downstream use.

```turtle
naph:captureOrientation a owl:DatatypeProperty ;
    rdfs:domain naph:Photograph ;
    rdfs:range xsd:string ;
    rdfs:label "capture orientation" ;
    rdfs:comment "vertical | high-oblique | low-oblique | ground-level" .
```

### P.3.2 Stereo pair

Aerial reconnaissance often produces overlapping stereo pairs for 3D analysis.

```turtle
naph:hasStereoPair a owl:ObjectProperty ;
    rdfs:domain naph:Photograph ;
    rdfs:range naph:Photograph ;
    rdfs:label "has stereo pair" ;
    owl:inverseOf naph:hasStereoPair .  # symmetric
```

### P.3.3 Film stock and physical characteristics

```turtle
naph:filmStock a owl:DatatypeProperty ;
    rdfs:label "film stock" ;
    rdfs:comment "panchromatic | panchromatic-IR | colour | black-and-white-orthochromatic | borne-digital" .

naph:physicalDimensions a owl:DatatypeProperty ;
    rdfs:label "physical dimensions" ;
    rdfs:comment "Dimensions of the original physical artefact, in mm (negative or print)" .

naph:focalLength a owl:DatatypeProperty ;
    rdfs:domain naph:CaptureEvent ;
    rdfs:range xsd:decimal ;
    rdfs:label "focal length (mm)" .
```

### P.3.4 Negative number

Where the catalogue distinguishes negative number from frame number (common in archive practice):

```turtle
naph:negativeNumber a owl:DatatypeProperty ;
    rdfs:label "negative number" ;
    rdfs:comment "Identifier of the physical negative, distinct from frame number." .
```

## P.4 Field-of-view derivation (aerial-specific)

For aerial photography, the geographic footprint MAY be derived geometrically from altitude, focal length, and image dimensions.

### P.4.1 Vertical photography

For a vertical aerial photograph:

```
ground_distance_per_image_edge = (image_size_mm × altitude_above_ground_metres) / focal_length_mm
```

Example: F.52 camera (36-inch / 914 mm focal length), 9 inch / 229 mm image, 9144 m altitude AGL:

```
ground_distance = (229 × 9144) / 914 = 2291 m = 2.29 km per image edge
```

This produces a square footprint of ~2.3km on a side at typical reconnaissance altitudes.

### P.4.2 Oblique photography

Oblique photography produces a trapezoidal footprint. Without field-of-view derivation, the footprint SHOULD be a bounding rectangle conservative enough to cover the actual photographed area.

### P.4.3 Implementation

The reference pipeline ([`pipeline/ingest.py`](../../../pipeline/ingest.py)) currently uses a simple point-with-buffer footprint. v0.3 of the pipeline will add proper field-of-view derivation for vertical aerial photography where altitude and focal length are present.

## P.5 Collection-specific identifier patterns

### P.5.1 NCAP three-component pattern

NCAP records have three identifying components:

- **Collection** — `RAF`, `NARA`, `DOS`, `JARIC`, `USAF`, etc.
- **Sortie** — flight mission reference (e.g. `106G/UK/1655`)
- **Frame** — sequential frame number (e.g. `4023`)

NAPH supports this with three properties:

```turtle
ex:sortie-X naph:collectionCode "RAF" ;
            naph:sortieReference "106G/UK/1655" .

ex:frame-X naph:partOfSortie ex:sortie-X ;
           naph:frameNumber 4023 .
```

The canonical identifier (`naph:hasIdentifier`) is composed:

```
https://w3id.org/naph/photo/RAF-106G-UK-1655-4023
```

### P.5.2 Other photographic collections

For institutions without sortie-based scheme:

- Single-photographer archives: `{photographer-id}/{accession-year}/{frame}`
- Studio archives: `{studio-id}/{contact-sheet-number}/{frame}`
- Press archives: `{publication-id}/{publication-date}/{photo-id}`

Module B's identifier requirements (persistence, resolvability, uniqueness) apply universally.

## P.6 Module application — Photographic profile specifics

### P.6.1 Module A (Capture & Imaging)

For photographic profile:

- TIFF strongly preferred over JP2 for preservation master (lossless TIFF is the archival standard for photography)
- Colour profile: AdobeRGB or wider for fine-art and recent material; sRGB for access copies
- For monochrome aerial, use 16-bit greyscale TIFF; do not convert to RGB unnecessarily

### P.6.2 Module B (Metadata)

Photographic profile MUST additionally provide:

- `naph:filmStock` (Baseline if known)
- `naph:captureOrientation` (Baseline if discernible from collection context)
- `naph:focalLength` on the CaptureEvent (Enhanced)
- `naph:physicalDimensions` (Enhanced)

### P.6.3 Module C (Rights)

Photography-specific rights considerations:

- Photographer's copyright is distinct from subject's copyright (separate `naph:rightsHolder` chains may apply)
- Personality rights for identifiable subjects (separate from the institution's rights to the photograph)
- For aerial photography of military or restricted areas: declassification status may differ from public-domain status

### P.6.4 Module E (Paradata)

For photographic profile, document:

- Whether the surrogate is from the original negative, an inter-positive, or a print
- Any retouching or restoration applied
- Optical density measurements where significant

## P.7 Worked example — NCAP-style aerial photograph

```turtle
@prefix naph: <https://w3id.org/naph/ontology#> .
@prefix dcterms: <http://purl.org/dc/terms/> .
@prefix dctype: <http://purl.org/dc/dcmitype/> .
@prefix prov: <http://www.w3.org/ns/prov#> .

ex:photo-001 a naph:AerialPhotograph ;
    dcterms:type dctype:StillImage ;
    rdfs:label "Berlin reconnaissance — 540 Squadron, 1944-03-28" ;
    naph:hasIdentifier "https://w3id.org/naph/photo/RAF-106G-UK-1655-4023" ;
    naph:partOfSortie ex:sortie-RAF-106G-UK-1655 ;
    naph:belongsToCollection ex:NCAP-RAF-collection ;
    naph:capturedOn "1944-03-28"^^xsd:date ;
    naph:coversArea ex:footprint-001 ;
    naph:captureOrientation "vertical" ;
    naph:filmStock "panchromatic" ;
    naph:hasRightsStatement ex:CrownCopyrightExpired ;
    naph:hasCaptureEvent ex:capture-001 ;
    naph:hasDigitalSurrogate ex:photo-001-master ;
    naph:hasProvenanceChain ex:provenance-001 ;
    naph:compliesWithTier naph:TierEnhanced .

ex:capture-001 a naph:CaptureEvent ;
    naph:flightAltitude 9144.0 ;
    naph:cameraType "F.52 split-pair vertical, 36-inch lens" ;
    naph:focalLength 914.0 .

ex:sortie-RAF-106G-UK-1655 a naph:Sortie ;
    naph:collectionCode "RAF" ;
    naph:sortieReference "106G/UK/1655" ;
    naph:squadron "540 Squadron" ;
    naph:aircraft "de Havilland Mosquito PR.IX" .
```

## P.8 Profile validation

The Photographic Profile uses the standard NAPH SHACL shapes plus profile-specific extensions defined in `ontology/profiles/photographic-shapes.ttl` (planned for v0.3).

## P.9 Cross-references

- [NAPH Standard v1.0](../NAPH-STANDARD.md)
- [Module A](../modules/A-capture-imaging.md) — capture
- [Module B](../modules/B-metadata-data-structures.md) — metadata
- [Module C](../modules/C-rights-licensing-ethics.md) — rights
- [Sample data](../../../data/sample-photographs.ttl) — reference implementation
