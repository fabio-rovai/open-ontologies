# Sub-profile: Aerial Archaeology

**Status:** Normative · v1.0
**Applies to:** aerial photography for archaeological survey
**Parent:** [Aerial Photography Profile](../aerial-photography.md)

## A.1 Scope

This sub-profile specialises NAPH for aerial photography taken for archaeological survey purposes — cropmark photography, soilmark photography, parchmark photography, and analytical reuse of historic reconnaissance for archaeological purposes.

The defining characteristics:

- Often **oblique** (low-angle) rather than vertical, to accentuate cropmark visibility
- Captured under specific lighting/seasonal conditions chosen for visibility
- Frequently linked to monument records (Canmore, Historic England, equivalent)
- Often associated with active research projects rather than long-term archives

Examples:

- Royal Commission on the Ancient and Historical Monuments of Scotland (RCAHMS) aerial survey programmes
- Historic England aerial reconnaissance archive
- Cambridge University Committee for Aerial Photography (CUCAP)
- O. G. S. Crawford / RCHME personal aerial archaeology archives

## A.2 Sub-profile additions

### A.2.1 Archaeological observation

```turtle
naph:ArchaeologicalObservation a owl:Class ;
    rdfs:label "Archaeological Observation" ;
    rdfs:comment "A research-grade interpretation of an aerial photograph identifying archaeological features." .

naph:hasArchaeologicalObservation a owl:ObjectProperty ;
    rdfs:domain naph:AerialPhotograph ;
    rdfs:range naph:ArchaeologicalObservation ;
    rdfs:label "has archaeological observation" .

naph:observedFeature a owl:DatatypeProperty ;
    rdfs:label "observed feature type" ;
    rdfs:comment "'cropmark' | 'soilmark' | 'parchmark' | 'shadow-mark' | 'standing-structure' | 'earthwork' | 'cut-feature' | 'enclosure' | 'fieldsystem' | 'industrial' | 'maritime' | etc." .

naph:observerName a owl:DatatypeProperty ;
    rdfs:label "observer name" .

naph:observationDate a owl:DatatypeProperty ;
    rdfs:label "observation date" ;
    rdfs:comment "Date of the archaeological interpretation, distinct from photograph capture date." .
```

### A.2.2 Capture conditions

Cropmark photography requires specific conditions:

```turtle
naph:cropMarkConditions a owl:DatatypeProperty ;
    rdfs:label "cropmark conditions" ;
    rdfs:comment "'optimal' | 'developing' | 'late' — recorded by the observer for cropmark visibility assessment" .

naph:lightingConditions a owl:DatatypeProperty ;
    rdfs:label "lighting conditions" ;
    rdfs:comment "Sun angle / direction notes when relevant to feature visibility." .

naph:seasonalContext a owl:DatatypeProperty ;
    rdfs:label "seasonal context" ;
    rdfs:comment "'late drought' | 'mid-summer' | 'spring' | etc. — reasons for capture timing" .
```

### A.2.3 Linked monument records

```turtle
naph:linkedMonumentRecord a owl:ObjectProperty ;
    rdfs:domain naph:AerialPhotograph ;
    rdfs:label "linked monument record" ;
    rdfs:comment "Canonical monument record (Canmore, Historic England Pastscape, equivalent)" .
```

For Aspirational tier, every aerial-archaeology record SHOULD link to at least one monument record.

## A.3 Reuse of historic reconnaissance for archaeology

WW2 reconnaissance frames (originally captured for military purposes) are now extensively used for archaeological research because they capture landscapes pre-modern agricultural intensification.

When archaeological observations are made on a historic reconnaissance frame, the record:

- Belongs primarily to the [reconnaissance sub-profile](reconnaissance.md)
- ALSO has archaeological observations attached
- Has dual context — military origin, archaeological research use

Example:

```turtle
ex:photo-RAF-541-EDI-1946-08-2287 a naph:AerialPhotograph ;
    rdfs:label "Edinburgh — pre-modern landscape reconnaissance" ;
    naph:partOfSortie ex:RAF-541-EDI-1946-08 ;
    # ... reconnaissance metadata ...
    naph:hasArchaeologicalObservation ex:obs-edinburgh-fieldsystems-2018 .

ex:obs-edinburgh-fieldsystems-2018 a naph:ArchaeologicalObservation ;
    naph:observedFeature "fieldsystem" ;
    naph:observerName "Dr A. B. Smith, RCAHMS" ;
    naph:observationDate "2018-04-12"^^xsd:date ;
    rdfs:comment "Pre-modern field-system boundaries visible as soilmarks; not visible in modern imagery due to subsequent ploughing." ;
    naph:linkedMonumentRecord <https://canmore.org.uk/site/300042> .
```

## A.4 Captures across visits

A single archaeological site may be photographed across multiple aerial visits over decades. Linking these is research-significant:

```turtle
naph:siteRevisit a owl:ObjectProperty ;
    rdfs:label "site revisit" ;
    rdfs:comment "Links a frame to other frames covering the same archaeological site at different times — supports change-detection over multi-decade timespans." ;
    a owl:SymmetricProperty .
```

For an archaeological research workflow, finding all photographs of a particular site across all sorties is a core competency question:

```sparql
PREFIX naph: <https://w3id.org/naph/ontology#>
SELECT ?photo ?date WHERE {
    ?photo a naph:AerialPhotograph ;
           naph:capturedOn ?date ;
           naph:siteRevisit*/naph:hasArchaeologicalObservation/naph:linkedMonumentRecord <https://canmore.org.uk/site/300042> .
}
ORDER BY ?date
```

## A.5 Worked example — RCAHMS oblique cropmark photography

```turtle
@prefix naph: <https://w3id.org/naph/ontology#> .

ex:rcahms-photo-2003-001 a naph:AerialPhotograph ;
    dcterms:type dctype:StillImage ;
    rdfs:label "Cropmark enclosure, East Lothian, RCAHMS aerial 2003-07-22" ;
    naph:hasIdentifier "https://w3id.org/naph/photo/RCAHMS-2003-07-22-001" ;
    naph:partOfSortie ex:rcahms-2003-07-22 ;
    naph:frameNumber 1 ;
    naph:belongsToCollection ex:RCAHMS-aerial-collection ;
    naph:capturedOn "2003-07-22"^^xsd:date ;
    naph:coversArea ex:rcahms-photo-2003-001-footprint ;
    naph:captureOrientation "low-oblique" ;
    naph:filmStock "colour" ;
    naph:hasRightsStatement ex:rcahms-rights ;
    naph:hasCaptureEvent ex:rcahms-photo-2003-001-capture ;
    naph:hasDigitalSurrogate ex:rcahms-photo-2003-001-master ;
    naph:hasArchaeologicalObservation ex:obs-eastlothian-enclosure-2003 ;
    naph:hasProvenanceChain ex:rcahms-photo-2003-001-provenance ;
    naph:compliesWithTier naph:TierAspirational .

ex:rcahms-2003-07-22 a naph:Sortie ;
    naph:collectionCode "RCAHMS" ;
    naph:sortieReference "2003-07-22-EAST-LOTHIAN" ;
    naph:missionType "archaeological-cropmark" ;
    naph:missionObjective "Cropmark survey, East Lothian, late drought conditions" ;
    naph:aircraft "Cessna 172" ;
    naph:cropMarkConditions "optimal" ;
    naph:seasonalContext "late drought, mid-summer" .

ex:obs-eastlothian-enclosure-2003 a naph:ArchaeologicalObservation ;
    naph:observedFeature "cropmark-enclosure" ;
    naph:observerName "Dr A. B. Smith, RCAHMS" ;
    naph:observationDate "2003-07-22"^^xsd:date ;
    naph:cropMarkConditions "optimal" ;
    rdfs:comment "Sub-rectangular ditched enclosure, ~80m × 60m, with internal divisions. Probably Iron Age." ;
    naph:linkedMonumentRecord <https://canmore.org.uk/site/300042> .

ex:rcahms-rights a naph:RightsStatement ;
    naph:rightsURI <https://www.nationalarchives.gov.uk/information-management/re-using-public-sector-information/uk-government-licensing-framework/crown-copyright/> ;
    naph:rightsLabel "Crown Copyright" ;
    naph:rightsHolder ex:HES ;
    naph:rightsNote "RCAHMS aerial photography, post-1992. Licensed under OGL v3.0 for re-use." ;
    dcterms:license <https://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/> .
```

## A.6 Cross-references

- [Aerial Photography Profile](../aerial-photography.md) (parent)
- [Reconnaissance sub-profile](reconnaissance.md) (historic reconnaissance often reused archaeologically)
- [UAV sub-profile](uav-drone.md) (modern aerial archaeology often UAV-based)
- [Canmore](https://canmore.org.uk/) — RCAHMS / HES national monument record
- [Historic England Pastscape](https://www.heritagegateway.org.uk/) — English equivalent
