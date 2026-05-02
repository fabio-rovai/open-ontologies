# Sub-profile: UAV / Drone Imagery

**Status:** Normative · v1.0
**Applies to:** unmanned aerial vehicle imagery (born-digital)
**Parent:** [Aerial Photography Profile](../aerial-photography.md)

## U.1 Scope

This sub-profile specialises NAPH for born-digital aerial imagery captured by UAVs (drones). The defining characteristics:

- Born-digital — no scanning step
- High-resolution and georeferenced via on-board GNSS
- Frequently part of structured surveys (orthomosaic generation, photogrammetric input)
- Modern rights regime (operator-licensed rather than crown/federal)
- Often associated with active research projects rather than archival deposits

Examples: archaeological survey UAV imagery, environmental monitoring datasets, drone-based heritage documentation projects, post-disaster aerial assessment.

## U.2 Sub-profile additions

### U.2.1 Drone flight class

```turtle
naph:DroneFlight rdfs:subClassOf naph:Sortie ;
    rdfs:label "Drone / UAV Flight" ;
    rdfs:comment "A UAV mission producing a structured survey of images." .

naph:droneModel a owl:DatatypeProperty ;
    rdfs:domain naph:DroneFlight ;
    rdfs:label "drone model" ;
    rdfs:comment "Manufacturer + model (e.g. 'DJI Phantom 4 RTK')." .

naph:operatorLicence a owl:DatatypeProperty ;
    rdfs:domain naph:DroneFlight ;
    rdfs:label "operator licence" ;
    rdfs:comment "Operator licence number (CAA in UK, FAA Part 107 in US)." .

naph:airspaceAuthorisation a owl:DatatypeProperty ;
    rdfs:label "airspace authorisation" ;
    rdfs:comment "Reference to authorisation under which the flight was conducted." .
```

### U.2.2 Per-frame georeferencing

Born-digital UAV imagery typically has per-frame GNSS-recorded position. NAPH supports both per-frame and per-flight footprint:

```turtle
naph:gnssLatitude a owl:DatatypeProperty ;
    rdfs:domain naph:CaptureEvent ;
    rdfs:range xsd:decimal ;
    rdfs:label "GNSS-recorded latitude" .

naph:gnssLongitude a owl:DatatypeProperty ;
    rdfs:domain naph:CaptureEvent ;
    rdfs:range xsd:decimal ;
    rdfs:label "GNSS-recorded longitude" .

naph:gnssAltitudeAGL a owl:DatatypeProperty ;
    rdfs:domain naph:CaptureEvent ;
    rdfs:label "GNSS altitude above ground level (m)" .

naph:gnssAccuracy a owl:DatatypeProperty ;
    rdfs:label "GNSS accuracy (m)" ;
    rdfs:comment "Reported horizontal accuracy at time of capture." .
```

### U.2.3 Photogrammetric processing chain

Drone imagery is typically the input to photogrammetric processing producing orthomosaics, DEMs, 3D models. Document the processing chain:

```turtle
naph:hasOrthomosaicOutput a owl:ObjectProperty ;
    rdfs:domain naph:DroneFlight ;
    rdfs:range naph:DigitalSurrogate ;
    rdfs:label "has orthomosaic output" .

naph:hasDigitalElevationModel a owl:ObjectProperty ;
    rdfs:domain naph:DroneFlight ;
    rdfs:range naph:DigitalSurrogate ;
    rdfs:label "has DEM output" .

naph:photogrammetrySoftware a owl:DatatypeProperty ;
    rdfs:label "photogrammetry software" ;
    rdfs:comment "'Agisoft Metashape' | 'Pix4D' | 'ContextCapture' | 'OpenDroneMap' | etc." .

naph:processingSettings a owl:DatatypeProperty ;
    rdfs:label "photogrammetry processing settings" ;
    rdfs:comment "Brief description of processing settings — accuracy, point density, GCPs used" .
```

### U.2.4 Ground control points

Where ground control points (GCPs) were used:

```turtle
naph:hasGroundControlPoints a owl:ObjectProperty ;
    rdfs:domain naph:DroneFlight ;
    rdfs:label "has ground control points" .

naph:GroundControlPoint a owl:Class ;
    rdfs:label "Ground Control Point" .

naph:gcpCoordinates a owl:DatatypeProperty ;
    rdfs:domain naph:GroundControlPoint ;
    rdfs:label "GCP coordinates" ;
    rdfs:comment "Survey-grade coordinates for the control point." .

naph:gcpRMSE a owl:DatatypeProperty ;
    rdfs:label "GCP RMSE" ;
    rdfs:comment "Root mean square error in survey terms — accuracy estimate." .
```

## U.3 Capture metadata expectations

For UAV imagery, EXIF metadata typically contains:

- Camera make/model + lens
- ISO, exposure, aperture
- Timestamp (with timezone — important)
- GNSS lat/lon/altitude
- Drone telemetry (sometimes)

NAPH MUST preserve this metadata. The `pipeline/uav-ingest.py` (planned v0.3) will:

1. Read EXIF + drone-telemetry CSV
2. Map EXIF tags to NAPH properties
3. Construct per-frame footprint from GNSS + flight parameters
4. Document any geometric processing (deskew, orthorectify) as `prov:Activity`

## U.4 Rights — operator-licensed regime

For UAV imagery, rights typically lie with:

- **Operator** (the organisation that flew the drone) — typically asserts copyright
- **Project sponsor** (where the project paid for the flight) — may have licensing terms
- **Subjects** (people or property visible in imagery) — privacy / data protection concerns
- **Permission-granting authority** (landowner / airspace authority) — may impose terms

For modern UAV imagery, the typical rights statement is a Creative Commons licence:

```turtle
ex:uav-rights a naph:RightsStatement ;
    naph:rightsURI <https://creativecommons.org/licenses/by/4.0/> ;
    naph:rightsLabel "Creative Commons Attribution 4.0" ;
    naph:rightsHolder ex:operator-X ;
    naph:rightsNote "Survey conducted under research grant. Attribution required: 'University X UAV Heritage Programme'" .
```

For commercial UAV operators, more restrictive licences are typical:

```turtle
ex:uav-commercial-rights a naph:RightsStatement ;
    naph:rightsURI <http://rightsstatements.org/vocab/InC/1.0/> ;
    naph:rightsLabel "In Copyright" ;
    naph:rightsHolder ex:operator-Y ;
    naph:rightsNote "Commercial UAV survey. Reuse requires written permission from operator." .
```

### U.4.1 Privacy considerations

UAV imagery routinely captures identifiable individuals, vehicles, residential property. Even where rights are clear, privacy / data-protection concerns may apply:

- GDPR (EU/UK): high-resolution imagery containing identifiable persons may require consent
- Data Protection Act 2018 (UK)
- Specific airspace / sensitive-location restrictions

Use `naph:ethicsStatement` for material with privacy concerns even when legally clear.

## U.5 Worked example — UAV survey of a heritage site

```turtle
@prefix naph: <https://w3id.org/naph/ontology#> .

ex:uav-001 a naph:AerialPhotograph ;
    dcterms:type dctype:StillImage ;
    rdfs:label "UAV survey, Site X, frame 0042 of survey 2024-08-15" ;
    naph:hasIdentifier "https://example.org/uav/2024-08-15-site-X/0042" ;
    naph:partOfSortie ex:drone-flight-2024-08-15 ;
    naph:frameNumber 42 ;
    naph:belongsToCollection ex:UAV-heritage-survey-collection ;
    naph:capturedOn "2024-08-15"^^xsd:date ;
    naph:coversArea ex:uav-001-footprint ;
    naph:captureOrientation "vertical" ;
    naph:filmStock "born-digital" ;
    naph:groundSampleDistance 0.02 ;
    naph:hasRightsStatement ex:uav-rights ;
    naph:hasCaptureEvent ex:uav-001-capture ;
    naph:hasDigitalSurrogate ex:uav-001-original ;
    naph:hasProvenanceChain ex:uav-001-provenance ;
    naph:compliesWithTier naph:TierAspirational .

ex:drone-flight-2024-08-15 a naph:DroneFlight ;
    naph:collectionCode "EX" ;
    naph:sortieReference "2024-08-15-site-X" ;
    naph:droneModel "DJI Phantom 4 RTK" ;
    naph:operatorLicence "CAA-OPER-12345" ;
    naph:airspaceAuthorisation "CAA Article 16 OA — Site X heritage research" ;
    naph:photogrammetrySoftware "Agisoft Metashape Professional 2.1" ;
    naph:processingSettings "high-density point cloud, 12 GCPs, RMSE 1.2 cm" ;
    naph:hasOrthomosaicOutput ex:orthomosaic-2024-08-15 .

ex:uav-001-capture a naph:CaptureEvent ;
    naph:flightAltitude 80.0 ;
    naph:cameraType "DJI FC6310 (1-inch CMOS)" ;
    naph:focalLength 8.8 ;
    naph:gnssLatitude 56.123456 ;
    naph:gnssLongitude -3.654321 ;
    naph:gnssAltitudeAGL 80.0 ;
    naph:gnssAccuracy 0.02 .

ex:uav-001-original a naph:DigitalSurrogate ;
    naph:digitisedOn "2024-08-15"^^xsd:date ;
    naph:fileFormat "image/x-adobe-dng" ;
    naph:digitisedBy ex:operator-X ;
    naph:hasChecksum "sha256:..." .
```

## U.6 Cross-references

- [Aerial Photography Profile](../aerial-photography.md) (parent)
- [Aerial Archaeology sub-profile](aerial-archaeology.md) (frequent application domain)
- [Photogrammetric Survey sub-profile](photogrammetric.md) (overlaps for UAV photogrammetric work)
