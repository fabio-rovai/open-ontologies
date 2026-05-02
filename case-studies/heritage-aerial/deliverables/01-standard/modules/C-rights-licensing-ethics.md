# Module C — Rights, Licensing & Ethics

**Status:** Normative · v1.0
**Applies to:** All NAPH-compliant records, all tiers
**Defines:** machine-readable rights statements and the ethical framework for sensitive material

## C.1 Purpose

A digital surrogate is computationally useless if a researcher cannot determine its legal and ethical status programmatically. Module C specifies the structure of rights and ethics declarations so:

- An institution can mass-classify their collection by reuse status without case-by-case clearance
- A researcher can write a SPARQL query that filters the collection by reuse permission
- Sensitive or contested material can be flagged for ethical attention separately from legal status
- Indigenous, community, and contributor rights can be expressed alongside legal copyright (CARE alignment)

## C.2 Outcome requirements

### C.2.1 Baseline (C-baseline)

A Baseline-compliant record MUST have:

- **C.B.1** A `naph:hasRightsStatement` link to a `naph:RightsStatement` instance
- **C.B.2** The `naph:RightsStatement` MUST have `naph:rightsURI` pointing to a registered authority (rightsstatements.org, Creative Commons, UK Government Licensing Framework, or equivalent national authority)
- **C.B.3** The `naph:RightsStatement` MUST have `naph:rightsLabel` providing the canonical human-readable label

A Baseline-compliant record SHOULD have:

- **C.B.4** `naph:rightsHolder` linking to a `prov:Agent` (the entity asserting the rights)
- **C.B.5** `dcterms:license` for records under Creative Commons or other open licences

### C.2.2 Enhanced (C-enhanced)

An Enhanced-compliant record MUST additionally have:

- **C.E.1** A documented basis for the rights determination (legal review, default policy, donor agreement)
- **C.E.2** A review date (`naph:rightsReviewedOn`) — rights determinations are not permanent; copyright laws and donor agreements change

An Enhanced-compliant record SHOULD additionally have:

- **C.E.3** `naph:ethicsStatement` for material that is sensitive even when legally clear (e.g. records of victims, contested colonial material, depictions of identifiable individuals)

### C.2.3 Aspirational (C-aspirational)

An Aspirational-compliant record MUST additionally have:

- **C.A.1** Where culturally appropriate, [Traditional Knowledge (TK) Labels](https://localcontexts.org/labels/traditional-knowledge-labels/) or [Biocultural (BC) Labels](https://localcontexts.org/labels/biocultural-labels/) from Local Contexts
- **C.A.2** Where the record involves identifiable living individuals, an explicit `naph:dataSubject` link with appropriate access conditions

## C.3 Authoritative rights vocabularies

### C.3.1 rightsstatements.org

The primary vocabulary for rights statements on heritage material. NAPH uses canonical `vocab/` URIs:

| Statement | Canonical URI | Use when |
|---|---|---|
| In Copyright | `http://rightsstatements.org/vocab/InC/1.0/` | The institution has determined the item is under copyright |
| In Copyright — EU Out of Copyright | `http://rightsstatements.org/vocab/InC-OW-EU/1.0/` | Public domain in EU but possibly under copyright elsewhere |
| In Copyright — Educational Use Permitted | `http://rightsstatements.org/vocab/InC-EDU/1.0/` | Copyright but with educational use permission |
| No Copyright — Other Known Legal Restrictions | `http://rightsstatements.org/vocab/NoC-OKLR/1.0/` | Out of copyright but other legal restrictions apply |
| No Copyright — Non-Commercial Use Only | `http://rightsstatements.org/vocab/NoC-NC/1.0/` | Public domain but reuse restricted to non-commercial |
| No Copyright — United States | `http://rightsstatements.org/vocab/NoC-US/1.0/` | Public domain in US, status elsewhere unverified |
| Copyright Undetermined | `http://rightsstatements.org/vocab/CNE/1.0/` | Status not yet evaluated — temporary use only |

### C.3.2 Creative Commons

For born-digital material or material released by the institution under an open licence:

- `https://creativecommons.org/publicdomain/zero/1.0/` (CC0 1.0)
- `https://creativecommons.org/licenses/by/4.0/` (CC BY 4.0)
- `https://creativecommons.org/licenses/by-sa/4.0/` (CC BY-SA 4.0)
- `https://creativecommons.org/licenses/by-nc/4.0/` (CC BY-NC 4.0)

### C.3.3 UK Government

For Crown Copyright material:

- `https://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/` (OGL v3.0)
- `https://www.nationalarchives.gov.uk/information-management/re-using-public-sector-information/uk-government-licensing-framework/crown-copyright/` (Crown Copyright statement)

### C.3.4 Local Contexts (Aspirational)

For material relating to Indigenous knowledge, contested heritage, or community-controlled material:

- [Traditional Knowledge Labels](https://localcontexts.org/labels/traditional-knowledge-labels/)
- [Biocultural Labels](https://localcontexts.org/labels/biocultural-labels/)

These labels MUST coexist with the legal rights statement, not replace it.

## C.4 Decision tree

For each record, determine the rights statement using [`decision-trees/rights-decision-tree.md`](../../04-adoption-guidance/decision-trees/rights-decision-tree.md). The decision tree resolves to a single canonical URI per record.

## C.5 Worked examples

### C.5.1 Baseline — Crown Copyright (UK government source)

```turtle
ex:crownRights a naph:RightsStatement ;
    naph:rightsURI <https://www.nationalarchives.gov.uk/information-management/re-using-public-sector-information/uk-government-licensing-framework/crown-copyright/> ;
    naph:rightsLabel "Crown Copyright" .

ex:photo-001 naph:hasRightsStatement ex:crownRights .
```

### C.5.2 Baseline — Out-of-copyright UK material

```turtle
ex:expiredCrown a naph:RightsStatement ;
    naph:rightsURI <http://rightsstatements.org/vocab/NoC-OKLR/1.0/> ;
    naph:rightsLabel "No Copyright — Other Known Legal Restrictions" ;
    naph:rightsNote "Crown Copyright expired (50 years post-creation for unpublished Crown works pre-1957)" .
```

### C.5.3 Enhanced — full review documentation

```turtle
ex:expiredCrown a naph:RightsStatement ;
    naph:rightsURI <http://rightsstatements.org/vocab/NoC-OKLR/1.0/> ;
    naph:rightsLabel "No Copyright — Other Known Legal Restrictions" ;
    naph:rightsHolder ex:HMG ;
    naph:rightsReviewedOn "2024-01-15"^^xsd:date ;
    naph:rightsReviewedBy ex:reviewer-A ;
    naph:rightsReviewBasis "Crown Copyright expiry — 50 years post-creation per CDPA 1988 s.163(3) for material created pre-1989 unpublished" .
```

### C.5.4 Enhanced + Ethics — sensitive material

```turtle
ex:photo-Hiroshima naph:hasRightsStatement ex:naraPublicDomain ;
    naph:ethicsStatement [
        rdfs:label "Records depicting civilian victims" ;
        rdfs:comment "Material documents civilian casualties and infrastructure destruction. Use respectfully; consider impact on descendants and survivor communities. See institutional access policy." ;
        naph:ethicsCategory "civilian-conflict-imagery"
    ] .
```

### C.5.5 Aspirational — TK Labels alongside legal rights

```turtle
ex:photo-Indigenous-X naph:hasRightsStatement ex:crownExpired ;
    naph:culturalRights [
        a localcontexts:TraditionalKnowledgeLabel ;
        rdfs:label "TK Attribution" ;
        rdfs:comment "Cultural attribution to [community] required for reuse. Contact [steward]." ;
        naph:tkLabelURI <https://localcontexts.org/label/tk-a/>
    ] .
```

## C.6 What this module does NOT do

Module C does NOT:

- Provide legal advice
- Adjudicate rights disputes
- Override existing institutional rights policies
- Assert that a rights determination is correct (only that one has been recorded)

Institutions remain fully responsible for the legal accuracy of rights statements they assert. Module C provides the structural framework for expressing those determinations machine-readably.

## C.7 Validation

A SHACL shape (`naph:RightsStatementShape`) checks:

- `naph:rightsURI` is present and is a valid URI
- `naph:rightsLabel` is present
- For Aspirational tier: cultural rights and ethics statements are properly typed if present

## C.8 Common errors

| Error | Why it matters | Remediation |
|---|---|---|
| Free-text rights ("public domain") | Cannot be queried; ambiguous | Map to canonical URI per [decision tree](../../04-adoption-guidance/decision-trees/rights-decision-tree.md) |
| Using `/page/` URI form | Wrong canonical form for RDF | Use `/vocab/` form for rightsstatements.org |
| Missing review date in Enhanced tier | Rights determinations age | Add `naph:rightsReviewedOn` |
| Treating legal rights as ethics | Conflates separate concerns | Use `naph:ethicsStatement` separately from `naph:hasRightsStatement` |

## C.9 Cross-references

- [Module B — Metadata & Data Structures](B-metadata-data-structures.md)
- [Rights decision tree](../../04-adoption-guidance/decision-trees/rights-decision-tree.md)
- [rightsstatements.org documentation](https://rightsstatements.org/page/1.0/)
- [Local Contexts](https://localcontexts.org/)
