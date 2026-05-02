# Sub-profile: Wartime Reconnaissance

**Status:** Normative · v1.0
**Applies to:** military aerial reconnaissance photography (WW1, WW2, Korea, Cold War, Falklands, etc.)
**Parent:** [Aerial Photography Profile](../aerial-photography.md)

## R.1 Scope

This sub-profile specialises NAPH for collections of military aerial reconnaissance photography. The defining characteristics:

- **Originally classified material** — declassification provenance is critical
- **Operational sortie context** — squadron, aircraft, mission type are research-relevant
- **Crown Copyright (UK) or Federal Public Domain (US) typical rights regime**
- **Stereo pairs intended for photo-interpretation** — adjacency relationships matter
- **Frame numbers within sortie sequence** — operational ordering preserves intelligence context

Examples: RAF/106G, RAF/541, USAAF/3PRS, USAF/91SRS, Luftwaffe (German Federal Archive), Soviet (selectively released).

## R.2 Sub-profile additions

### R.2.1 Mission classification

```turtle
naph:missionType a owl:DatatypeProperty ;
    rdfs:domain naph:Sortie ;
    rdfs:range xsd:string ;
    rdfs:label "mission type" ;
    rdfs:comment "Operational classification: 'damage assessment' | 'pre-strike' | 'post-strike' | 'mapping' | 'order-of-battle' | 'troop-movement' | 'scientific'" .

naph:targetPriority a owl:DatatypeProperty ;
    rdfs:domain naph:Sortie ;
    rdfs:label "target priority" ;
    rdfs:comment "Where recorded: '1' (highest) through '4' (low) operational priority." .
```

### R.2.2 Declassification

```turtle
naph:declassificationEvent a owl:Class ;
    rdfs:subClassOf prov:Activity ;
    rdfs:label "Declassification Event" ;
    rdfs:comment "The administrative or executive event that lifted classification restrictions on a record." .

naph:declassificationOrder a owl:DatatypeProperty ;
    rdfs:domain naph:DeclassificationEvent ;
    rdfs:label "declassification order" ;
    rdfs:comment "Reference to the EO, statute, or administrative ruling that authorised declassification." .

naph:originalClassification a owl:DatatypeProperty ;
    rdfs:label "original classification level" ;
    rdfs:comment "'TOP SECRET' | 'SECRET' | 'CONFIDENTIAL' | 'RESTRICTED' or equivalent national level." .
```

### R.2.3 Photo-interpretation lineage

Reconnaissance photography typically passed through PI (photo-interpretation) cells which produced annotated derivative material. The lineage is research-significant:

```turtle
naph:hasInterpretationReport a owl:ObjectProperty ;
    rdfs:domain naph:AerialPhotograph ;
    rdfs:range naph:InterpretationReport ;
    rdfs:label "has interpretation report" ;
    rdfs:comment "Link to a contemporaneous photo-interpretation report produced from the photograph." .

naph:InterpretationReport a owl:Class ;
    rdfs:subClassOf prov:Entity ;
    rdfs:label "Interpretation Report" .
```

## R.3 Identifier scheme — wartime conventions

Wartime reconnaissance identifiers commonly use:

```
{operator}/{wing-or-squadron}/{theatre}/{sortie-number}
```

Examples:

- `RAF/106G/UK/1655` — RAF 106 Group, UK area, sortie 1655
- `USAAF/3PRS/EU/2287` — USAAF 3rd Photo Reconnaissance Squadron, European theatre
- `RAF/541/HAM/1943-07-26` — RAF 541 Sqn, Hamburg, dated sortie

Frame numbers within the sortie are sequential and indicate temporal/spatial ordering of exposures.

## R.4 Rights determination — wartime specifics

Use the [rights decision tree](../../04-adoption-guidance/decision-trees/rights-decision-tree.md) with these reconnaissance-specific notes:

### R.4.1 UK Crown Copyright reconnaissance

- WW1 RAF and predecessor units (RFC, RNAS): Crown Copyright. Most expired (>100 years).
- WW2 RAF: Crown Copyright. Unpublished works expire 125 years post-creation OR 50 years post-publication, whichever shorter. WW2-era unpublished material is now expiring (2020-2030 window).
- Cold War material (post-1945): Crown Copyright still applies for non-published material. Declassification status separate.

### R.4.2 US Federal Public Domain

- USAAF, USAF, USN, NARA-held material: public domain in US under federal-employee work doctrine.
- Note: status outside US not automatically asserted — use `NoC-US/1.0/` rather than CC0.

### R.4.3 Captured / seized enemy material

Reconnaissance archives often include captured material:

- Luftwaffe / Wehrmacht material captured 1944-1945: now held in German Federal Archive or Allied repositories. Rights regime varies by repository.
- Soviet material declassified post-1991: subject to Russian (and successor state) law plus repository agreements.
- Document the seizure event in provenance:

```turtle
ex:photo-X naph:hasProvenanceChain ex:provenance-X .
ex:provenance-X a naph:ProvenanceChain ;
    rdfs:label "Luftwaffe → captured 1945 → US Strategic Bombing Survey → NARA → NCAP digitisation 2019" ;
    prov:hadMember ex:capture-event-Luftwaffe, ex:seizure-event-1945, ex:transfer-USSBS, ex:transfer-NARA, ex:digitisation-NCAP .
```

## R.5 Cross-references with companion materials

Reconnaissance archives are typically deeply embedded with companion materials:

- **Sortie plotting reports** (where the sortie flew, what was photographed)
- **Mission summary reports** (operational outcomes)
- **PI reports** (what the photographs revealed)
- **Order-of-battle assessments** (compiled from multiple PI reports)

The standard supports linking to these via `naph:linkedRecord`. Aspirational tier requires at least one such link where companions exist in the same institution's holdings.

```turtle
ex:photo-Hamburg-001 naph:linkedRecord
    <https://www.iwm.org.uk/collections/item/object/IWM-FLM-3001> ,  # Sortie plot
    <https://www.iwm.org.uk/collections/item/object/IWM-PI-1287>     # PI report
.
```

## R.6 Aerial-archaeology overlap

WW1 and WW2 reconnaissance frames are increasingly used for archaeological landscape research (e.g. cropmark analysis pre-modern agricultural intensification). Where reconnaissance archives are heavily used by archaeologists, consider also applying the [Aerial Archaeology sub-profile](aerial-archaeology.md) for relevant frames.

## R.7 Worked example — Mosquito reconnaissance frame, Berlin 1944

```turtle
@prefix naph: <https://w3id.org/naph/ontology#> .
@prefix dcterms: <http://purl.org/dc/terms/> .
@prefix dctype: <http://purl.org/dc/dcmitype/> .
@prefix prov: <http://www.w3.org/ns/prov#> .

ex:photo-001 a naph:AerialPhotograph ;
    dcterms:type dctype:StillImage ;
    rdfs:label "Berlin reconnaissance — 540 Squadron, 1944-03-28, frame 4023" ;
    naph:hasIdentifier "https://w3id.org/naph/photo/RAF-106G-UK-1655-4023" ;
    naph:partOfSortie ex:sortie-RAF-106G-UK-1655 ;
    naph:frameNumber 4023 ;
    naph:belongsToCollection ex:NCAP-RAF-collection ;
    naph:capturedOn "1944-03-28"^^xsd:date ;
    naph:coversArea ex:footprint-001 ;
    naph:captureOrientation "vertical" ;
    naph:filmStock "panchromatic" ;
    naph:hasRightsStatement ex:CrownCopyrightExpired ;
    naph:hasCaptureEvent ex:capture-001 ;
    naph:hasDigitalSurrogate ex:photo-001-master, ex:photo-001-access ;
    naph:hasProvenanceChain ex:provenance-001 ;
    naph:compliesWithTier naph:TierEnhanced .

ex:sortie-RAF-106G-UK-1655 a naph:Sortie ;
    naph:collectionCode "RAF" ;
    naph:sortieReference "106G/UK/1655" ;
    naph:squadron "540 Squadron" ;
    naph:aircraft "de Havilland Mosquito PR.IX" ;
    naph:missionType "damage assessment" ;
    naph:targetPriority 1 ;
    naph:missionObjective "Strategic photographic reconnaissance — Berlin industrial district" .

ex:provenance-001 a naph:ProvenanceChain ;
    rdfs:label "RAF 1944 → Air Ministry → MoD → declassified 1972 → NCAP transfer 2008" ;
    prov:hadMember ex:capture-1944, ex:declassify-1972, ex:transfer-NCAP-2008 .

ex:declassify-1972 a naph:DeclassificationEvent ;
    rdfs:label "Declassification under UK 30-Year Rule (1968 Public Records Act)" ;
    prov:atTime "1972-01-01T00:00:00Z"^^xsd:dateTime ;
    naph:declassificationOrder "Public Records Act 1968 — 30-year automatic declassification" ;
    naph:originalClassification "SECRET" .
```

## R.8 Cross-references

- [Aerial Photography Profile](../aerial-photography.md) (parent)
- [Module C — Rights, Licensing & Ethics](../../modules/C-rights-licensing-ethics.md)
- [Module E — Paradata & Workflow](../../modules/E-paradata-workflow.md)
- [Aerial Archaeology sub-profile](aerial-archaeology.md)
- [Satellite Imagery sub-profile](satellite.md)
