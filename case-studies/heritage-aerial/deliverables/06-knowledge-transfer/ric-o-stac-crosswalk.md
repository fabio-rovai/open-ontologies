# NAPH Crosswalk: Records in Contexts (RiC-O) x STAC

**Binding the archival stack to the geospatial-imagery stack for historic aerial photography.**

Status: reference crosswalk, NAPH v0.1
Machine-readable form: [`ontology/naph-ric-o-crosswalk.ttl`](../../ontology/naph-ric-o-crosswalk.ttl)
Operational STAC output: [`reports/stac/catalog.json`](../../reports/stac/catalog.json) (built by [`pipeline/build-stac.py`](../../pipeline/build-stac.py))

---

## 1. Why this crosswalk exists: an unoccupied intersection

Historic aerial photography sits astride two mature but non-communicating standards stacks.

**The archival stack** describes *what a record is, who held it, and how it came to be here*. Its current apex is the **Records in Contexts Ontology (RiC-O) 1.1**, published by the International Council on Archives Expert Group on Archival Description (ICA/EGAD) in **May 2025**. RiC-O 1.1 is an OWL ontology of 107 classes formalising the RiC conceptual model; it supersedes the four legacy ICA standards (ISAD(G), ISAAR(CPF), ISDF, ISDIAH) with a single graph-based description model. RiC-O is strong exactly where imagery formats are weak: multi-provenance custody chains, corporate-body holders, instantiation hierarchies (the intellectual record vs. its physical and digital carriers), and the temporal association of records with agents, activities, and mandates. For declassified military reconnaissance imagery, which is most of NCAP, this custodial rigour is not optional; the transfer history from capturing air force, through defence intelligence bodies, to a public heritage custodian *is* the record's evidential value.

**The geospatial-imagery stack** describes *where and when a pixel array was captured, and how to compute over it*. Its lingua franca is the **SpatioTemporal Asset Catalog (STAC) 1.0.0**, which became an **OGC Community Standard** in 2024. STAC gives every image a bounding box, an RFC3339 datetime, a GeoJSON geometry, and typed asset links, so that the entire modern toolchain (pystac, stackstac, stac-browser, the QGIS STAC plugin, TiTiler) can index, search, and mosaic it. STAC is strong exactly where archival catalogues are weak: spatial querying, temporal-window search, and computability. But STAC has no concept of a fonds, a custodial holder, a mandate, or a provenance chain. It is a discovery-and-compute layer, not an evidential-description layer.

**No existing standard crosswalks historic aerial photography across both.** RiC-O has no spatial vocabulary beyond `rico:Coordinates` attached to a `rico:Place`; it cannot express a footprint polygon that a GIS can query, and it says nothing about how to make an image tile-computable. STAC has no archival provenance model whatsoever. Generic bridges (Dublin Core, DCAT, schema.org) collapse both sides into flat metadata and lose the two things that matter: the custody chain and the computable footprint. The RiC-O × STAC intersection is genuinely unoccupied.

Aerial photography is the specific domain that *forces* the join, because a single frame is simultaneously:

- an **archival record** with a provenance chain, a holder, and a legal status (the RiC-O view), and
- a **spatiotemporal raster asset** with a ground footprint and a capture instant (the STAC view).

Treat it as only one of these and you lose the other half of its value. NAPH's contribution is to be the ontology that pins the two views to the *same* entity: the NAPH `AerialPhotograph` is at once a `rico:Record` and a `stac:Item`, and the crosswalk below is what makes that dual identity machine-checkable. PROV-O sits underneath both as the shared provenance substrate (RiC-O's activities and STAC's processing history both reduce to `prov:Activity`/`prov:Entity`), and GeoSPARQL provides the geometry semantics that STAC's GeoJSON only implies.

---

## 2. The crosswalk table

Legend for the match column in the TTL: **=** `skos:exactMatch` (no scope difference); **~** `skos:closeMatch` (strong, not logically identical); **>** `rdfs:seeAlso` (relevant pointer, not an equivalence). STAC is a JSON spec, not an RDF ontology, so all STAC bindings are informative field anchors, never `skos:*Match`. All `rico:` terms were verified against the published `RiC-O_1-1.rdf` source; unverifiable candidates were dropped, not guessed.

### 2.1 Core artefacts

| NAPH term | RiC-O 1.1 | STAC 1.0.0 field | PROV-O | ISAD(G) legacy |
|---|---|---|---|---|
| `naph:AerialPhotograph` | ~ `rico:Record` (> `rico:RecordResource`) | Item | `prov:Entity` | 3.1.1 Reference code; item-level unit of description |
| `naph:Sortie` | ~ `rico:RecordSet` | Collection | (none) | Series / sub-series grouping |
| `naph:Collection` | ~ `rico:RecordSet` | Catalog | (none) | Fonds |
| `naph:CustodialInstitution` | ~ `rico:CorporateBody` (> `rico:Agent`) | (catalog provider) | `prov:Organization` | 3.2.1 Name of creator(s); holder has no ISAD(G) element (see ISDIAH) |
| `naph:Frame` | > `rico:RecordPart`, > `rico:Identifier` | Item `id` | (none) | Component of the reference code |
| `naph:DigitalSurrogate` | ~ `rico:Instantiation` | Asset | `prov:Entity` | 3.5.2 Existence and location of copies |

### 2.2 Spatial / temporal

| NAPH term | RiC-O 1.1 | STAC 1.0.0 field | PROV-O | ISAD(G) legacy |
|---|---|---|---|---|
| `naph:GeographicFootprint` | > `rico:Coordinates` | `geometry` | (none) | (none; ISAD(G) has no spatial model) |
| `naph:CaptureEvent` | > `rico:Activity`, > `rico:Event` | `properties.datetime` (instant) | ~ `prov:Activity` | 3.2.3 Archival history (capture) |
| `naph:capturedOn` | ~ `rico:hasCreationDate` | `properties.datetime` | (> `dcterms:created`) | 3.1.3 Date(s) |
| `naph:coversArea` | > `rico:isAssociatedWithPlace`, > `rico:hasOrHadLocation` | `bbox` + `geometry` | (none) | (none) |
| `naph:asWKT` | > `rico:hasOrHadCoordinates` | (implied by `geometry`) | (none) | (none) |

### 2.3 Provenance, custody, rights

| NAPH term | RiC-O 1.1 | STAC 1.0.0 field | PROV-O | ISAD(G) legacy |
|---|---|---|---|---|
| `naph:hasProvenanceChain` | ~ `rico:hasOrganicProvenance` | (none) | `prov:wasGeneratedBy` | 3.2.4 Immediate source of acquisition; 3.2.3 Archival history |
| `naph:ProvenanceChain` | > `rico:hasOrganicProvenance` | (none) | ~ `prov:Bundle` | 3.2.3 Archival history |
| `naph:custodian` | ~ `rico:hasOrHadHolder` | (catalog provider) | `prov:wasAttributedTo` | 3.2.1 Name of creator(s); holder has no ISAD(G) element |
| `naph:hasDigitalSurrogate` | ~ `rico:hasOrHadInstantiation` | Asset link | `prov:wasDerivedFrom` | 3.5.2 Existence and location of copies |
| `naph:hasCaptureEvent` | ~ `rico:isAssociatedWithEvent` | (none) | `prov:wasGeneratedBy` | 3.2.3 Archival history |
| `naph:RightsStatement` | > `rico:Mandate` (RiC-O 1.1 has no rights-statement class; IP rights are agent relations via rico:IntellectualPropertyRightsRelation) | `properties.license` | (none) | 3.4.2 Conditions governing reproduction |

### 2.4 Identifiers, subjects, form

| NAPH term | RiC-O 1.1 | STAC 1.0.0 field | PROV-O | ISAD(G) legacy |
|---|---|---|---|---|
| `naph:hasIdentifier` | ~ `rico:hasOrHadIdentifier` | Item `id` | (none) | 3.1.1 Reference code |
| `naph:sortieReference` | > `rico:hasOrHadIdentifier` | `properties.naph:sortie` | (none) | 3.1.1 Reference code (series segment) |
| `naph:partOfSortie` | ~ `rico:isOrWasIncludedIn` | `collection` link | (none) | Level-of-description linkage |
| `naph:belongsToCollection` | ~ `rico:isOrWasIncludedIn` | root/parent link | (none) | Fonds membership |
| `naph:cameraType` | > `rico:hasDocumentaryFormType`, > `rico:hasCarrierType` | `properties.naph:camera` | (none) | 3.1.5 Extent and medium |
| `naph:depicts` | ~ `rico:hasOrHadSubject` | (none) | (none) | 3.3.1 Scope and content |
| `naph:placeAuthorityURI` | > `rico:isAssociatedWithPlace` | (none) | (none) | 3.3.1 Scope and content |

### 2.5 The load-bearing hinge

The single most important row is `naph:DigitalSurrogate ~ rico:Instantiation`, aligned with STAC `Asset`. RiC-O's instantiation model, the same intellectual record can have multiple physical and digital carriers, is exactly the abstraction STAC needs but lacks: a STAC Item's `assets` (preservation TIFF, access JP2, thumbnail) are, archivally, the *instantiations* of one record. This is where the two stacks actually meet rather than merely coexist, and it is why the crosswalk is more than a lookup table.

---

## 3. How this maps to the real NCAP catalogue

The mappings above are grounded in the 292 **real** NCAP records harvested from the public Air Photo Finder API (`pipeline/real-ncap-raw.json`), not in invented examples. Take frame 0001 of sortie `PEGASUS/RN/H/0007`:

```json
"image_metadata": { "UNI": "000-000-357-415", "ISAD(G)": "GB 551 NCAP/20-1-2-2",
                    "Camera": "V", "Frame": "0001" },
"details": { "sortie": "PEGASUS/RN/H/0007", "frame_id": "0001",
             "date": "1924-11-12", "date_precision": "day",
             "image_type": "vertical", "collection_context": "Defence Geographic Centre",
             "image_coordinates": "POLYGON ((12705976.28 2554902.94, ...))" }
```

Each NCAP source field lands in a defined place across all three stacks:

| NCAP raw field | Value (this record) | NAPH | RiC-O 1.1 | STAC | ISAD(G) |
|---|---|---|---|---|---|
| `image_metadata."ISAD(G)"` | `GB 551 NCAP/20-1-2-2` | `naph:hasIdentifier` | `rico:hasOrHadIdentifier` | Item `id` / `properties` | **3.1.1 Reference code** (this field *is* the ISAD(G) reference, so the legacy column is native, not a mapping) |
| `image_metadata.UNI` | `000-000-357-415` | `dcterms:identifier` (`NCAP-UNI:...`) | `rico:hasOrHadIdentifier` | `properties.naph:identifier` | 3.1.1 |
| `details.sortie` | `PEGASUS/RN/H/0007` | `naph:sortieReference` on `naph:Sortie` | name of the `rico:RecordSet` | `properties.naph:sortie`; `collection` | series segment of 3.1.1 |
| `details.frame_id` | `0001` | `naph:frameNumber` on `naph:Frame` | `rico:RecordPart` / `rico:Identifier` | Item `id` suffix | frame segment of 3.1.1 |
| `details.date` (+ `date_precision`) | `1924-11-12` (day) | `naph:capturedOn` (xsd:date) | `rico:hasCreationDate` | `properties.datetime` = `1924-11-12T00:00:00Z` | **3.1.3 Date(s)** |
| `details.image_type` | `vertical` | `naph:imagePerspective` | `rico:hasDocumentaryFormType` | `properties.naph:perspective` | 3.1.5 Extent and medium |
| `image_metadata.Camera` | `V` | `naph:cameraType` on `naph:CaptureEvent` | `rico:hasCarrierType` | `properties.naph:camera` | 3.1.5 |
| `details.collection_context` | `Defence Geographic Centre` | custody note on `naph:ProvenanceChain` | `rico:hasOrganicProvenance` → `rico:CorporateBody` | (none) | **3.2.3 Archival history** |
| `details.image_coordinates` | `POLYGON ((...EPSG:3857...))` | `naph:GeographicFootprint` / `naph:asWKT` (reprojected to WGS84) | `rico:hasOrHadCoordinates` → `rico:Coordinates` | `geometry` + `bbox` | (none, spatial gap in ISAD(G)) |
| catalogue URL | `airphotofinder.ncap.org/image/797810` | `prov:hadPrimarySource` | (holder-provided access point) | Item `assets` | 3.5.2 Existence and location of copies |

Two structural observations fall straight out of the real data:

1. **`collection_context` is a provenance assertion, not a subject.** `Defence Geographic Centre` is the body that held the imagery before NCAP, it belongs on the custody chain (`rico:hasOrganicProvenance`), which is precisely the axis STAC cannot express and RiC-O exists to capture. A naive DCAT crosswalk would mislabel it as a keyword and destroy the evidential trail.

2. **`image_coordinates` is the axis RiC-O cannot compute over.** It arrives as an EPSG:3857 WKT polygon. RiC-O can only park it under `rico:Coordinates` as opaque literals; it is STAC's `geometry`/`bbox` (WGS84) that makes the footprint queryable by a GIS. NAPH holds both: the archival attachment via RiC-O and the computable footprint via GeoSPARQL/STAC, the same polygon, two computational affordances.

The net effect: an NCAP frame described once in NAPH can be served *simultaneously* into an archival discovery system speaking RiC-O and into a geospatial pipeline speaking STAC, with a shared PROV-O provenance spine and the ISAD(G) reference code preserved verbatim as the anchor identifier. That dual-serve, from a single computation-ready record, is the whitespace NAPH occupies.

---

## 4. Sources and verification notes

- **RiC-O 1.1**, ICA/EGAD, May 2025. Ontology IRI `https://www.ica.org/standards/RiC/ontology`; namespace prefix `rico:`. Every `rico:` term asserted as a `skos:closeMatch`/`skos:exactMatch` in the TTL was verified present as an `owl:Class`, `owl:ObjectProperty`, or `owl:DatatypeProperty` in the published `RiC-O_1-1.rdf`.
- **STAC 1.0.0**, SpatioTemporal Asset Catalog specification, an OGC Community Standard. `stac:` IRIs in the TTL are informative field anchors (`rdfs:seeAlso` only); the operational binding is the JSON emitted by `build-stac.py`.
- **PROV-O**, W3C Provenance Ontology (`http://www.w3.org/ns/prov#`).
- **GeoSPARQL**, OGC GeoSPARQL (`http://www.opengis.net/ont/geosparql#`).
- **ISAD(G)**, General International Standard Archival Description, 2nd ed., ICA. Legacy field references (3.1.1–3.4.2) are given for migration continuity; NCAP's own catalogue already emits an `ISAD(G)` reference code per record.

**RiC-O terms that could NOT be verified and were therefore NOT asserted as matches:** two candidate RecordSet-membership properties, `rico:isPartOfRecordSet` and `rico:isMemberOf`, could not be confirmed in `RiC-O_1-1.rdf`; both were removed rather than guessed, and `naph:partOfSortie` / `naph:belongsToCollection` map instead to the confirmed `rico:isOrWasIncludedIn`. Additionally, RiC-O 1.1 has **no** `Rights` class, rights/legal status is modelled through `rico:Mandate` and rights-holder relations, so `naph:RightsStatement` takes a `skos:closeMatch` to `dcterms:RightsStatement` (a genuine equivalent) and only a cautious `rdfs:seeAlso` to `rico:Mandate`.
