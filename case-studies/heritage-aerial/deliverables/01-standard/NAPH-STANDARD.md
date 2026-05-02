# NAPH — Computation-Ready Digitisation Standard for Aerial Photography Heritage

**Version:** 1.0
**Status:** Draft for partner review
**Editors:** Kampakis and Co Ltd, trading as The Tesseract Academy
**Licence:** [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)
**Scope:** Aerial photography heritage collections (vertical, high-oblique, low-oblique, satellite, UAV)

---

## Abstract

This document specifies a digitisation standard for **aerial photography heritage collections** that supports computational research, not just human browsing. It defines an extensible, tiered, openly licensed framework that institutions of varying scale, capacity, and resource can adopt incrementally. The standard is operationalised through (a) an OWL ontology, (b) SHACL validation shapes, (c) a transformation pipeline from legacy metadata, and (d) a domain-specific aerial-photography profile.

NAPH is deliberately a **narrow vertical** standard — it covers aerial photography heritage and only aerial photography heritage. This depth-over-breadth choice is documented in [ADR-0001](../06-knowledge-transfer/architecture-decision-records/0001-narrow-vertical.md). Generic GLAM-wide digitisation standards exist; this is the standard for the aerial vertical specifically.

The standard's central premise: most aerial photography heritage data is **digitised but not computable**. Closing that gap is structural rather than technical — the work is consistency in identifiers, dates, rights, packaging, and provenance, not new digitisation.

## Status of this document

This is **v1.0** — the first complete iteration suitable for partner application and refinement under a structured testing programme. It is **not** final. v1.x updates are expected following each round of partner application during partner adoption.

A future v2.0 may make breaking changes informed by accumulated partner experience. v1.x changes are guaranteed to be backwards-compatible.

## 1. Conformance language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** in this document are to be interpreted as described in [BCP 14, RFC 2119](https://datatracker.ietf.org/doc/html/rfc2119) when, and only when, they appear in all capitals.

## 2. Scope and out-of-scope

### In scope

- Defining what a digitised aerial photograph needs to expose to be **computation-ready**
- Specifying outcome requirements for capture, metadata, rights, packaging, provenance, and validation
- Providing a tier model that allows incremental adoption
- Aligning with established W3C and OGC standards rather than reinventing them
- Specifying a governance model so the standard remains maintained beyond NAPH
- Aerial-specific domain features: stereo pair structure, ground sample distance, sortie metadata, declassification provenance

### Out of scope

- Non-aerial heritage (manuscripts, audio, video, 3D objects, ground-level photography) — these need their own focused standards
- Mandating specific digitisation hardware, scanning equipment, or imaging workflows
- Replacing existing archival cataloguing standards (ISAD-G, AACR2, RDA) — NAPH augments these for computational access
- Specifying user interfaces, search portals, or presentation systems
- Curatorial decisions about what to digitise or in what order
- Long-term digital preservation strategy beyond format and provenance documentation

### Domain coverage

The [Aerial Photography Profile](profiles/aerial-photography.md) is the single normative profile in v1.0. It covers:

- WW1 / WW2 / Cold War aerial reconnaissance
- Post-war civil aerial survey (mapping, urban, agricultural)
- Directorate of Overseas Surveys (DOS) imagery
- Photogrammetric survey collections
- Declassified satellite imagery archives (CORONA, KH-9, Hexagon, etc.)
- Drone / UAV imagery (born-digital aerial heritage)
- Aerial archaeological survey

Future major versions may add adjacent profiles (e.g. terrestrial photogrammetric, LiDAR), but v1.x will not expand scope beyond aerial photography.

## 3. The three tiers

NAPH defines three nested compliance tiers. Each is a superset of the previous — a record that conforms to a higher tier also conforms to all lower tiers.

### 3.1 Baseline — minimum computation-readiness

A Baseline-compliant record provides the minimum metadata structure necessary for a computational research workflow to find, identify, locate, and use the record without manual interpretation.

A record claiming Baseline conformance MUST satisfy [Module A](modules/A-capture-imaging.md), [Module B-baseline](modules/B-metadata-data-structures.md), [Module C-baseline](modules/C-rights-licensing-ethics.md), and [Module D-baseline](modules/D-packaging-publication.md).

Baseline tier is the **non-negotiable bar** for any record published as part of a NAPH-compliant collection.

### 3.2 Enhanced — supports research workflows

An Enhanced-compliant record provides the additional metadata and provenance documentation needed to support reproducible computational research at scale: documented digitisation provenance, capture context, full transformation chain, and multiple surrogate formats with quality classifications.

A record claiming Enhanced conformance MUST satisfy all Baseline requirements plus the additional requirements in [Module B-enhanced](modules/B-metadata-data-structures.md), [Module D-enhanced](modules/D-packaging-publication.md), and [Module E](modules/E-paradata-workflow.md).

### 3.3 Aspirational — supports semantic discovery

An Aspirational-compliant record provides semantic enrichment and cross-collection linkage that supports knowledge-graph queries, federated search, and content-based retrieval across institutional boundaries.

A record claiming Aspirational conformance MUST satisfy all Enhanced requirements plus the additional requirements in [Module B-aspirational](modules/B-metadata-data-structures.md) and [Module F](modules/F-qa-validation.md).

### 3.4 Mixed-tier collections

A NAPH-compliant collection MAY contain records at different tiers. Each record SHOULD declare its tier explicitly via the `naph:compliesWithTier` property.

Collections SHOULD report their tier distribution as part of public metadata (using the [tier-distribution profile](../04-adoption-guidance/validation-checklists.md)).

## 4. Six functional modules

The standard is decomposed into six modules. Each module has a Baseline / Enhanced / Aspirational specification. Modules are independent but cross-reference each other where alignment is necessary. All modules are interpreted in the aerial-photography vertical context — the [Aerial Photography Profile](profiles/aerial-photography.md) defines the domain-specific application.

| Module | Topic | Spec |
|---|---|---|
| A | Capture & Imaging | [modules/A-capture-imaging.md](modules/A-capture-imaging.md) |
| B | Metadata & Data Structures | [modules/B-metadata-data-structures.md](modules/B-metadata-data-structures.md) |
| C | Rights, Licensing & Ethics | [modules/C-rights-licensing-ethics.md](modules/C-rights-licensing-ethics.md) |
| D | Packaging & Publication | [modules/D-packaging-publication.md](modules/D-packaging-publication.md) |
| E | Paradata & Workflow Documentation | [modules/E-paradata-workflow.md](modules/E-paradata-workflow.md) |
| F | QA & Validation | [modules/F-qa-validation.md](modules/F-qa-validation.md) |

Each module specifies **outcome requirements** rather than prescriptive workflows. Institutions are free to use any tooling, vendor, or in-house process that meets the requirements.

## 5. FAIR and CARE principles

NAPH is structurally aligned with FAIR (Findable, Accessible, Interoperable, Reusable) and CARE (Collective benefit, Authority to control, Responsibility, Ethics) principles.

### FAIR mappings

| Principle | NAPH instrument |
|---|---|
| **F**indable | Stable identifier ([Module B](modules/B-metadata-data-structures.md), Baseline). Geographic footprint ([Module B](modules/B-metadata-data-structures.md), Baseline). |
| **A**ccessible | Machine-readable rights statement ([Module C](modules/C-rights-licensing-ethics.md), Baseline). HTTP-resolvable URIs ([Module B](modules/B-metadata-data-structures.md), Baseline). |
| **I**nteroperable | Subclass alignment to W3C/OGC standards ([Module B](modules/B-metadata-data-structures.md)). IIIF Presentation API mapping ([Module D](modules/D-packaging-publication.md)). |
| **R**eusable | Provenance chain ([Module E](modules/E-paradata-workflow.md), Enhanced). Rights URIs ([Module C](modules/C-rights-licensing-ethics.md), Baseline). Documented digitisation transformations ([Module E](modules/E-paradata-workflow.md), Enhanced). |

### CARE mappings

| Principle | NAPH instrument |
|---|---|
| **C**ollective benefit | Open licensing of standard (CC BY 4.0). Community input via RFC process ([governance](../05-governance/governance-proposal.md)). |
| **A**uthority to control | Institution-asserted rights statements ([Module C](modules/C-rights-licensing-ethics.md)). Indigenous and community labels supported as extension ([Module C](modules/C-rights-licensing-ethics.md), Aspirational). |
| **R**esponsibility | Provenance chain MUST document transfer history including any rights or ethical complications ([Module E](modules/E-paradata-workflow.md), Enhanced). |
| **E**thics | Ethics statement RECOMMENDED for sensitive or contested material ([Module C](modules/C-rights-licensing-ethics.md), Enhanced). |

## 6. Standards alignment

NAPH defers to and subsumes the following established standards. A NAPH-compliant record automatically becomes a valid instance of these parent standards through subclass inference.

| Standard | Body | Used for |
|---|---|---|
| [PROV-O](https://www.w3.org/TR/prov-o/) | W3C | Provenance and digitisation events |
| [GeoSPARQL](https://www.ogc.org/standards/geosparql/) | OGC | Geographic footprints |
| [SKOS](https://www.w3.org/2004/02/skos/) | W3C | Subjects, places, controlled vocabularies |
| [DCAT 3](https://www.w3.org/TR/vocab-dcat-3/) | W3C | Collections and resource discovery |
| [DCMI Type vocabulary](https://www.dublincore.org/specifications/dublin-core/dcmi-terms/#http://purl.org/dc/dcmitype/) | DCMI | Resource type (StillImage, Text, etc.) |
| [Dublin Core Terms](https://www.dublincore.org/specifications/dublin-core/dcmi-terms/) | DCMI | Descriptive metadata |
| [FOAF](http://xmlns.com/foaf/0.1/) | FOAF Project | Custodial institutions and agents |
| [IIIF Presentation 3.0](https://iiif.io/api/presentation/3.0/) | IIIF Consortium | Image presentation manifests |
| [rightsstatements.org](https://rightsstatements.org/) | Europeana / DPLA | Rights statement URIs |
| [BagIt](https://datatracker.ietf.org/doc/html/rfc8493) | IETF | Packaging |
| [PREMIS](https://www.loc.gov/standards/premis/) | Library of Congress | Preservation metadata (Aspirational) |

The **architectural principle** is synthesis over invention. NAPH does not redefine concepts that authoritative standards already cover.

## 7. Naming, namespaces, and identifiers

### 7.1 Ontology namespace

The NAPH ontology namespace is:

```
https://w3id.org/naph/ontology#
```

This URI MUST resolve to the canonical Turtle representation of the ontology. The `w3id.org` permanent URL service guarantees long-term stability independent of hosting.

### 7.2 Identifier minting

Institutions MUST mint persistent identifiers for individual records under their own namespace. NAPH RECOMMENDS the [w3id.org permanent URI service](https://w3id.org/) for institutions that lack their own permanent URL infrastructure.

Identifiers MUST be HTTP(S) URIs. They MUST resolve to a representation of the record (HTML, RDF, JSON-LD). They MUST NOT be reused if the record is deleted or replaced.

The recommended pattern is:

```
https://<institution-namespace>/<collection-code>/<sortie-or-accession>/<frame-or-item>
```

The standard does NOT mandate a specific identifier syntax beyond persistence and resolvability requirements. Institutions adopt this pattern OR maintain their existing scheme provided it meets the persistence and resolvability requirements.

### 7.3 Compatibility with existing identifier schemes

Where institutions have existing identifier schemes (e.g. NCAP's Collection / Sortie / Frame composite), NAPH provides distinct properties for each component (`naph:collectionCode`, `naph:sortieReference`, `naph:frameNumber`) so existing schemes can be modelled without information loss.

## 8. Versioning and change management

### 8.1 Semantic versioning

NAPH follows [semantic versioning 2.0.0](https://semver.org/):

- **MAJOR** — backwards-incompatible changes (e.g. removing required properties)
- **MINOR** — backwards-compatible additions (e.g. new optional properties, new modules)
- **PATCH** — clarifications, errata, doc fixes

A record valid against v1.x will remain valid against any future v1.y. v2.0 will be a deliberate breaking-change point requiring migration tooling.

### 8.2 Change governance

Substantive changes follow the [RFC process](../05-governance/rfc-process.md). Editorial changes (typos, clarifications) follow a lightweight review process documented in the [governance proposal](../05-governance/governance-proposal.md).

### 8.3 Deprecation policy

A property or class slated for removal in a future MAJOR version MUST be marked with `owl:deprecated true` for at least one MINOR cycle before removal. Migration guidance MUST be included in the release notes.

## 9. Conformance assessment

A NAPH-compliant record is one that:

1. Is expressed in valid RDF (Turtle, JSON-LD, RDF/XML, or N-Triples)
2. Conforms to the SHACL shapes in [`shapes/naph-shapes.ttl`](../../ontology/naph-shapes.ttl) for its declared tier
3. Resolves all referenced URIs that the standard requires to resolve
4. Includes a `naph:compliesWithTier` declaration

A NAPH-compliant collection is one composed entirely of NAPH-compliant records, with each record declaring its tier.

The [validation toolkit](../04-adoption-guidance/validation-checklists.md) provides an automated SHACL-based assessment that produces a conformance report suitable for institutional self-certification.

## 10. Acknowledgements

This standard was developed under the **Towards a National Collection / N-RICH Prototype** (Towards a National Collection / AHRC / UKRI), with reference to the published Pilot specification.

Standards we build on, in order of architectural significance: **W3C** (PROV-O, DCAT, SKOS), **OGC** (GeoSPARQL), **IIIF Consortium** (Presentation API), **rightsstatements.org** (Europeana / DPLA), **DCMI**, **IETF** (BagIt), **Library of Congress** (PREMIS).

## Appendix A: Document conventions

- Property names use the `naph:` prefix; standards-aligned classes use their authoritative prefixes
- All examples are valid Turtle unless explicitly stated otherwise
- Dates in examples follow ISO 8601 unless illustrating a non-conformant input

## Appendix B: References

- [Towards a National Collection — N-RICH Prototype](https://www.nationalcollection.org.uk/n-rich-prototype)
- [Towards a National Collection / N-RICH Prototype](https://www.nationalcollection.org.uk/n-rich-prototype)
- [W3C PROV-O](https://www.w3.org/TR/prov-o/)
- [W3C DCAT 3](https://www.w3.org/TR/vocab-dcat-3/)
- [W3C SKOS](https://www.w3.org/2004/02/skos/)
- [OGC GeoSPARQL](https://www.ogc.org/standards/geosparql/)
- [IIIF Presentation 3.0](https://iiif.io/api/presentation/3.0/)
- [FAIR principles](https://www.go-fair.org/fair-principles/)
- [CARE principles](https://www.gida-global.org/care)
