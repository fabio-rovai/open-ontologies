# Sub-profile: Satellite Imagery

**Status:** Normative · v1.0
**Applies to:** declassified national satellite reconnaissance archives
**Parent:** [Aerial Photography Profile](../aerial-photography.md)

## S.1 Scope

This sub-profile specialises NAPH for satellite reconnaissance imagery from declassified national programmes. The defining characteristics:

- Born-analogue (film) but recovered to digital via specialised scanning
- Specific declassification mechanisms (e.g. US Executive Order 12951)
- Strip-based acquisition rather than discrete frames
- Wide-area coverage with high temporal cadence
- Public-domain status post-declassification

Programmes covered:

- **CORONA** (US, 1959-1972) — KH-1 through KH-4B series
- **GAMBIT** (US, 1963-1984) — KH-7 and KH-8
- **HEXAGON** (US, 1971-1986) — KH-9 "Big Bird"
- **ZENIT** (USSR, 1962-1994) — declassified post-1991
- **National Reconnaissance Programme partner imagery** that has been declassified

Held principally by USGS Earth Explorer, NARA, and (digitisation-partnered) NCAP.

## S.2 Sub-profile additions

### S.2.1 Satellite acquisition

`naph:SatelliteAcquisition` is a subclass of `naph:Sortie`:

```turtle
naph:SatelliteAcquisition rdfs:subClassOf naph:Sortie ;
    rdfs:label "Satellite Acquisition" ;
    rdfs:comment "A pass of a reconnaissance satellite producing a strip of images." .

naph:satelliteSystem a owl:DatatypeProperty ;
    rdfs:domain naph:SatelliteAcquisition ;
    rdfs:label "satellite system" ;
    rdfs:comment "'CORONA' | 'GAMBIT' | 'HEXAGON' | 'ZENIT' | etc." .

naph:cameraSystem a owl:DatatypeProperty ;
    rdfs:domain naph:SatelliteAcquisition ;
    rdfs:label "camera system" ;
    rdfs:comment "'KH-4B' | 'KH-7' | 'KH-8' | 'KH-9' | etc." .

naph:missionNumber a owl:DatatypeProperty ;
    rdfs:domain naph:SatelliteAcquisition ;
    rdfs:label "mission number" ;
    rdfs:comment "Mission/operations identifier within the satellite system (e.g. '1117-1' for CORONA)." .

naph:downloadFromUSGS a owl:DatatypeProperty ;
    rdfs:domain naph:SatelliteAcquisition ;
    rdfs:label "USGS Earth Explorer entity ID" ;
    rdfs:comment "USGS canonical entity ID for the acquisition (e.g. 'DS1117-1019DA037')." .
```

### S.2.2 Camera-specific frame addressing

Satellite imagery uses different frame addressing depending on system:

```turtle
naph:cameraSide a owl:DatatypeProperty ;
    rdfs:label "camera side" ;
    rdfs:comment "For dual-panoramic systems (CORONA KH-4B, HEXAGON KH-9): 'forward' | 'aft' | 'mapping'" .

naph:passNumber a owl:DatatypeProperty ;
    rdfs:label "pass number" ;
    rdfs:comment "For satellites with multiple passes per mission." .
```

### S.2.3 Resolution and area

```turtle
naph:groundCoveragePerFrame a owl:DatatypeProperty ;
    rdfs:label "ground coverage per frame (km²)" ;
    rdfs:comment "Approximate coverage area — e.g. CORONA KH-4B: ~250 × 16 km strips" .
```

## S.3 Declassification

US satellite reconnaissance was declassified under specific Executive Orders:

- **EO 12951 (1995)** — declassified CORONA, ARGON, LANYARD
- **EO 13526 (2009)** — declassified KH-7, KH-9 partial
- **Subsequent administrative releases** — additional KH-9, IGLOO, HEXAGON

Provenance MUST record the specific EO:

```turtle
ex:declassify-EO-12951 a naph:DeclassificationEvent ;
    rdfs:label "Declassification under EO 12951 (1995)" ;
    prov:atTime "1995-02-22T00:00:00Z"^^xsd:dateTime ;
    naph:declassificationOrder "Executive Order 12951" ;
    naph:declassificationOrderURL <https://www.archives.gov/about/laws/eo-12951> ;
    naph:originalClassification "TOP SECRET / TALENT KEYHOLE" .
```

## S.4 Geographic footprint specifics

Satellite footprints are large strips, not bounded frames. The geographic footprint MUST express the strip:

```turtle
ex:hexagon-001-footprint a naph:GeographicFootprint ;
    naph:asWKT "POLYGON((-3.5 55.5, -3.5 56.5, -2.5 56.5, -2.5 55.5, -3.5 55.5))"^^geo:wktLiteral ;
    naph:stripGeometry true ;
    rdfs:comment "Approximate strip — actual coverage subject to platform geometry. For higher precision use the USGS-provided shapefile." .
```

For research-grade applications, link to a USGS-provided footprint shapefile rather than relying on bounding-box approximation.

## S.5 Rights — straightforward but with caveats

US satellite imagery in the declassified archives is in the public domain. Use:

```turtle
ex:satellite-rights a naph:RightsStatement ;
    naph:rightsURI <http://rightsstatements.org/vocab/NoC-US/1.0/> ;
    naph:rightsLabel "No Copyright — United States" ;
    naph:rightsNote "Declassified satellite reconnaissance — public domain in US per declassification authority. Status outside US not asserted." .
```

For Soviet/Russian declassified material (ZENIT), rights regime is more variable. Consult the holding repository for specifics.

## S.6 Cross-collection linking — USGS Earth Explorer

USGS provides canonical entity IDs and resolution. NAPH-compliant satellite records SHOULD link to USGS:

```turtle
ex:hexagon-001 naph:linkedRecord <https://earthexplorer.usgs.gov/scene/metadata/full/declass3/DS1117-1019DA037> .
```

This enables federated discovery — researchers using USGS Earth Explorer can find NCAP-held satellite material via NAPH link-back, and vice versa.

## S.7 Worked example — HEXAGON KH-9 frame

```turtle
@prefix naph: <https://w3id.org/naph/ontology#> .

ex:hexagon-001 a naph:AerialPhotograph ;
    dcterms:type dctype:StillImage ;
    rdfs:label "HEXAGON KH-9 mission 1117-1 forward camera, frame 0037" ;
    naph:hasIdentifier "https://w3id.org/naph/photo/HEXAGON-1117-1-DA-0037" ;
    naph:partOfSortie ex:hexagon-1117-1 ;
    naph:frameNumber 37 ;
    naph:belongsToCollection ex:USGS-declassified-collection ;
    naph:capturedOn "1980-09-12"^^xsd:date ;
    naph:coversArea ex:hexagon-001-footprint ;
    naph:captureOrientation "vertical" ;
    naph:filmStock "panchromatic" ;
    naph:groundSampleDistance 0.6 ;
    naph:hasRightsStatement ex:satellite-rights ;
    naph:hasCaptureEvent ex:hexagon-001-capture ;
    naph:hasDigitalSurrogate ex:hexagon-001-master ;
    naph:hasProvenanceChain ex:hexagon-001-provenance ;
    naph:linkedRecord <https://earthexplorer.usgs.gov/scene/metadata/full/declass3/DS1117-1019DA037> ;
    naph:compliesWithTier naph:TierAspirational .

ex:hexagon-1117-1 a naph:SatelliteAcquisition ;
    naph:collectionCode "HEXAGON" ;
    naph:satelliteSystem "HEXAGON" ;
    naph:cameraSystem "KH-9" ;
    naph:cameraSide "forward" ;
    naph:missionNumber "1117-1" ;
    naph:downloadFromUSGS "DS1117-1019DA037" ;
    naph:sortieReference "1117-1" .

ex:hexagon-001-provenance a naph:ProvenanceChain ;
    rdfs:label "Acquired 1980 → declassified 2011 (KH-9 partial release) → USGS Earth Explorer 2011 → NCAP digitisation partnership 2019" ;
    prov:hadMember ex:hexagon-001-capture, ex:declassify-2011, ex:transfer-USGS-2011, ex:digitisation-NCAP-2019 .

ex:declassify-2011 a naph:DeclassificationEvent ;
    rdfs:label "KH-9 partial declassification (administrative release)" ;
    prov:atTime "2011-06-26T00:00:00Z"^^xsd:dateTime ;
    naph:declassificationOrder "NRO administrative release per EO 13526 review" ;
    naph:originalClassification "TOP SECRET / RUFF" .
```

## S.8 Cross-references

- [Aerial Photography Profile](../aerial-photography.md) (parent)
- [Reconnaissance sub-profile](reconnaissance.md) (companion for declassified material)
- [USGS Earth Explorer Declass collections](https://earthexplorer.usgs.gov/)
