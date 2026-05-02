# Sub-profile: Photogrammetric Mapping Survey

**Status:** Normative · v1.0
**Applies to:** photogrammetric mapping survey collections (pre-LiDAR topographic capture)
**Parent:** [Aerial Photography Profile](../aerial-photography.md)

## G.1 Scope

This sub-profile specialises NAPH for aerial photography that was captured for the explicit purpose of photogrammetric mapping — producing topographic maps, contour data, or 3D models. The defining characteristics:

- Vertical-only photography with high precision (typically 60% along-track, 30% across-track overlap)
- Calibrated cameras with documented focal length and lens distortion
- Linked to derived map products
- Often part of national mapping programmes (Ordnance Survey, IGN, USGS, IGM)

Examples:

- Ordnance Survey aerial mapping (UK, 1940s-2000s)
- IGN photogrammetric campaigns (France)
- USGS National High-Altitude Photography (US, 1980s)
- Directorate of Overseas Surveys (DOS) — colonial mapping
- Regional aerial mapping campaigns

## G.2 Sub-profile additions

### G.2.1 Mapping survey

```turtle
naph:MappingSurvey rdfs:subClassOf naph:Sortie ;
    rdfs:label "Mapping Survey" ;
    rdfs:comment "An aerial mission specifically for photogrammetric mapping output." .

naph:surveyProgramme a owl:DatatypeProperty ;
    rdfs:domain naph:MappingSurvey ;
    rdfs:label "survey programme" ;
    rdfs:comment "Named campaign, e.g. 'OS Aerial Survey 1947-50' or 'NHAP 1985'" .

naph:surveyArea a owl:DatatypeProperty ;
    rdfs:domain naph:MappingSurvey ;
    rdfs:label "survey area name" ;
    rdfs:comment "Geographic block reference within the programme." .

naph:flightLine a owl:DatatypeProperty ;
    rdfs:domain naph:Sortie ;
    rdfs:label "flight line identifier" ;
    rdfs:comment "Within a mapping block, the specific flight line (e.g. 'L042')" .
```

### G.2.2 Camera calibration data

Calibrated cameras have specific lens characteristics that matter for analytical work:

```turtle
naph:cameraCalibrationReport a owl:ObjectProperty ;
    rdfs:domain naph:CaptureEvent ;
    rdfs:range foaf:Document ;
    rdfs:label "camera calibration report" ;
    rdfs:comment "Reference to the calibration certificate for the camera + lens combination used." .

naph:lensDistortionParameters a owl:DatatypeProperty ;
    rdfs:label "lens distortion parameters" ;
    rdfs:comment "Brown-Conrady or equivalent distortion coefficients (k1, k2, p1, p2 etc.)" .

naph:principalPoint a owl:DatatypeProperty ;
    rdfs:label "principal point" ;
    rdfs:comment "Coordinates of the camera's principal point (calibrated)." .
```

### G.2.3 Map product linkage

Photogrammetric collections are typically linked to derived map products:

```turtle
naph:hasDerivedMap a owl:ObjectProperty ;
    rdfs:domain naph:MappingSurvey ;
    rdfs:label "has derived map" ;
    rdfs:comment "Map product produced from this survey (linked to canonical map record)." .

naph:hasContourData a owl:ObjectProperty ;
    rdfs:label "has contour data" .

naph:hasOrthomosaic a owl:ObjectProperty ;
    rdfs:label "has orthomosaic" .
```

## G.3 Stereoscopic structure

Photogrammetric surveys produce stereo pairs explicitly. Each photograph SHOULD link to its stereo neighbours:

```turtle
ex:photo-survey-1947-L042-0125 a naph:AerialPhotograph ;
    naph:hasStereoPair ex:photo-survey-1947-L042-0124 ,
                       ex:photo-survey-1947-L042-0126 .
```

For Aspirational tier, every photograph in a mapping survey SHOULD have its forward and aft stereo pairs identified.

## G.4 Coordinate Reference System (CRS)

Photogrammetric work requires explicit CRS handling. Where the survey was processed in a non-WGS84 system (OSGB36 in UK, NAD83 in US, etc.), document both:

```turtle
ex:footprint-survey-1947-L042 a naph:GeographicFootprint ;
    naph:asWKT "POLYGON((-3.21 55.94, ...))"^^geo:wktLiteral ;
    naph:asWKT_OSGB "POLYGON((325000 670000, ...))"^^geo:wktLiteral ;
    naph:nativeCRS <http://www.opengis.net/def/crs/EPSG/0/27700> ;  # OSGB36
    rdfs:comment "WKT in WGS84 for federation; native CRS preserved for analytical reuse." .
```

## G.5 Worked example — OS aerial mapping frame

```turtle
@prefix naph: <https://w3id.org/naph/ontology#> .

ex:os-photo-001 a naph:AerialPhotograph ;
    dcterms:type dctype:StillImage ;
    rdfs:label "OS Aerial Survey 1947-50, Block 47, Flight Line 042, Frame 0125" ;
    naph:hasIdentifier "https://w3id.org/naph/photo/OS-AS-1947-50-B47-L042-0125" ;
    naph:partOfSortie ex:os-survey-1947-50-B47 ;
    naph:frameNumber 125 ;
    naph:belongsToCollection ex:NCAP-OS-mapping-collection ;
    naph:capturedOn "1948-06-12"^^xsd:date ;
    naph:coversArea ex:os-photo-001-footprint ;
    naph:captureOrientation "vertical" ;
    naph:filmStock "panchromatic" ;
    naph:groundSampleDistance 0.45 ;
    naph:hasRightsStatement ex:CrownCopyrightExpired ;
    naph:hasCaptureEvent ex:os-photo-001-capture ;
    naph:hasDigitalSurrogate ex:os-photo-001-master ;
    naph:hasProvenanceChain ex:os-photo-001-provenance ;
    naph:hasStereoPair ex:os-photo-002, ex:os-photo-000 ;
    naph:compliesWithTier naph:TierAspirational .

ex:os-survey-1947-50-B47 a naph:MappingSurvey ;
    naph:collectionCode "OS" ;
    naph:sortieReference "AS/1947-50/B47" ;
    naph:surveyProgramme "Ordnance Survey Aerial Survey 1947-50 (post-war re-mapping)" ;
    naph:surveyArea "Block 47 — Edinburgh and East Lothian" ;
    naph:flightLine "L042" ;
    naph:aircraft "Avro Lincoln B.II" ;
    naph:hasDerivedMap <https://canmore.org.uk/maps/os-1955-1-25k-edinburgh-and-east-lothian> .

ex:os-photo-001-capture a naph:CaptureEvent ;
    naph:flightAltitude 4572.0 ;
    naph:cameraType "Williamson F.49 Mk.II" ;
    naph:focalLength 152.4 ;
    naph:imageFormat "230x230" ;
    naph:cameraCalibrationReport <https://example.org/calibration/F49-Mk-II-1947-cert.pdf> .

ex:os-photo-001-footprint a naph:GeographicFootprint ;
    naph:asWKT "POLYGON((-3.21 55.94, -3.13 55.94, -3.13 55.99, -3.21 55.99, -3.21 55.94))"^^geo:wktLiteral ;
    naph:nativeCRS <http://www.opengis.net/def/crs/EPSG/0/27700> .
```

## G.6 Cross-references

- [Aerial Photography Profile](../aerial-photography.md) (parent)
- [Reconnaissance sub-profile](reconnaissance.md) (overlaps for wartime mapping work)
- [UAV sub-profile](uav-drone.md) (modern photogrammetric work increasingly UAV-based)
