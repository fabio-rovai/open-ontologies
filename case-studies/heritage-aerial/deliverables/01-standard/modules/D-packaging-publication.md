# Module D — Packaging & Publication

**Status:** Normative · v1.0
**Applies to:** All NAPH-compliant collections, all tiers
**Defines:** how records are bundled, transferred, and exposed for downstream consumption

## D.1 Purpose

A digitised collection that cannot be efficiently bulk-downloaded, served via standard APIs, or imported into another system is **digitally published but not interoperable**. Module D specifies the packaging and publication layer so:

- Researchers can bulk-download collections via documented mechanisms
- Image viewers (Mirador, Universal Viewer, Annotorious) can consume records without bespoke integration
- Aggregators (Europeana, DPLA) can harvest the collection
- Migration between institutions or platforms is non-destructive

## D.2 Outcome requirements

### D.2.1 Baseline (D-baseline)

A Baseline-compliant collection MUST:

- **D.B.1** Provide a manifest listing all records in the collection (BagIt manifest, RO-Crate `ro-crate-metadata.json`, or DCAT 3 `dcat:Catalog`)
- **D.B.2** Each record MUST resolve via its `naph:hasIdentifier` to at least one machine-readable representation (Turtle, JSON-LD, or RDF/XML)
- **D.B.3** Each record's metadata MUST be downloadable as a discrete unit (one record = one resolvable URI = one downloadable representation)
- **D.B.4** A bulk download mechanism MUST be documented and accessible (DCAT distribution, sitemap, OAI-PMH endpoint, or static archive)

A Baseline-compliant collection SHOULD:

- **D.B.5** Provide a `sitemap.xml` listing all record URIs for crawlers
- **D.B.6** Use HTTP content negotiation so the same URI returns HTML for browsers and Turtle/JSON-LD for RDF clients

### D.2.2 Enhanced (D-enhanced)

An Enhanced-compliant collection MUST additionally:

- **D.E.1** Provide records bundled in standardised packaging (BagIt or RO-Crate) with manifests and checksums
- **D.E.2** Document the packaging format and version explicitly
- **D.E.3** Validate packaging integrity at publication time

An Enhanced-compliant collection SHOULD additionally:

- **D.E.4** Provide a SPARQL endpoint for the collection's RDF
- **D.E.5** Provide IIIF Presentation 3.0 manifests for any imagery

### D.2.3 Aspirational (D-aspirational)

An Aspirational-compliant collection MUST additionally:

- **D.A.1** Expose a IIIF Image API 3.0 service for each digital surrogate, OR document an alternative image-serving protocol that supports range requests, transformations, and metadata
- **D.A.2** Provide a federated SPARQL endpoint that can be queried in conjunction with national authorities (Wikidata, GeoNames, etc.)
- **D.A.3** Support OAI-PMH harvesting for legacy aggregators

## D.3 Manifest formats

### D.3.1 BagIt

[BagIt](https://datatracker.ietf.org/doc/html/rfc8493) is recommended for transfer/archival packaging because it is simple, well-supported, and has integrity verification built in.

A NAPH BagIt bag SHOULD include:

```
my-collection-bag/
├── bagit.txt
├── bag-info.txt
├── manifest-sha256.txt
├── tagmanifest-sha256.txt
└── data/
    ├── records/
    │   ├── photo-001.ttl
    │   ├── photo-002.ttl
    │   └── ...
    ├── ontology-version.txt
    └── README.md
```

`bag-info.txt` MUST include:

```
Bag-Software-Agent: NAPH Pipeline v1.0
Bagging-Date: 2024-04-30
Source-Organization: National Collection of Aerial Photography
External-Identifier: https://w3id.org/naph/example/collection-bag-2024-04
NAPH-Tier-Distribution: Baseline=42, Enhanced=18, Aspirational=5
NAPH-Spec-Version: 1.0
Payload-Oxum: 145890341.65
```

### D.3.2 RO-Crate

[RO-Crate](https://www.researchobject.org/ro-crate/) is recommended for research-oriented publication because it produces self-contained, JSON-LD-described data packages.

A NAPH RO-Crate `ro-crate-metadata.json` SHOULD declare:

```json
{
  "@context": ["https://w3id.org/ro/crate/1.1/context", "https://w3id.org/naph/ontology"],
  "@graph": [
    {
      "@id": "ro-crate-metadata.json",
      "@type": "CreativeWork",
      "conformsTo": [
        {"@id": "https://w3id.org/ro/crate/1.1"},
        {"@id": "https://w3id.org/naph/ontology"}
      ],
      "about": {"@id": "./"}
    },
    {
      "@id": "./",
      "@type": "Dataset",
      "naph:tierDistribution": {"baseline": 42, "enhanced": 18, "aspirational": 5}
    }
  ]
}
```

### D.3.3 DCAT 3 catalog

For a published web-facing catalogue:

```turtle
ex:NCAPCollection a dcat:Catalog ;
    dcterms:title "NCAP Holdings" ;
    dcat:dataset ex:photo-001, ex:photo-002, ... ;
    dcat:distribution ex:bagit-distribution-2024-04 .

ex:bagit-distribution-2024-04 a dcat:Distribution ;
    dcterms:format "application/zip+bagit" ;
    dcat:downloadURL <https://example.org/ncap-2024-04.bag.zip> ;
    dcat:byteSize 145890341 .
```

## D.4 IIIF Presentation 3.0 binding

### D.4.1 Manifest construction

For each record with a digital surrogate, an IIIF Presentation 3.0 Manifest MUST be derivable. The mapping between NAPH and IIIF is:

| NAPH | IIIF |
|---|---|
| `naph:AerialPhotograph` | `Manifest` |
| `rdfs:label` | `label` |
| `naph:hasRightsStatement / naph:rightsURI` | `rights` |
| `naph:hasRightsStatement / naph:rightsLabel` | `requiredStatement.value` |
| `naph:hasDigitalSurrogate` | `Canvas → Annotation → Image body + service` |
| `naph:hasIdentifier` | `Manifest.id` |
| Various NAPH metadata | `metadata` pairs |

The reference implementation is [`pipeline/iiif-bridge.py`](../../../pipeline/iiif-bridge.py).

### D.4.2 IIIF Image API service

An Aspirational-tier collection MUST run a IIIF Image API 3.0 service for each access surrogate. The service URL MUST be the `id` of the `ImageService3` declaration in the manifest. It MUST respond to:

- `{base-uri}/info.json` — service descriptor
- `{base-uri}/{region}/{size}/{rotation}/{quality}.{format}` — image requests

NAPH does not specify an implementation. Recommended servers: IIPImage, Cantaloupe, IIIF-Cloud, OpenSeadragon-Plus.

### D.4.3 Manifest collections

For collection-level browsing, expose a IIIF Collection that lists Manifests:

```json
{
  "@context": "http://iiif.io/api/presentation/3/context.json",
  "id": "https://example.org/ncap/collection",
  "type": "Collection",
  "label": {"en": ["NCAP Holdings"]},
  "items": [
    {"id": "https://example.org/ncap/photo-001/manifest", "type": "Manifest", "label": {"en": ["Berlin reconnaissance frame 4023"]}},
    ...
  ]
}
```

## D.5 SPARQL endpoint requirements (Enhanced+)

If a collection exposes a SPARQL endpoint:

- It MUST be at a stable, documented URL
- It MUST support SPARQL 1.1 SELECT, ASK, CONSTRUCT, DESCRIBE
- It SHOULD support [SPARQL Update](https://www.w3.org/TR/sparql11-update/) only over authenticated channels
- It SHOULD provide a [VOID](https://www.w3.org/TR/void/) descriptor at `{endpoint}/.well-known/void`
- It SHOULD support [SPARQL service descriptions](https://www.w3.org/TR/sparql11-service-description/) at `{endpoint}/.well-known/sd`
- For Aspirational tier, it MUST support [SPARQL 1.1 federated queries](https://www.w3.org/TR/sparql11-federated-query/) so users can join with Wikidata, GeoNames, and other authorities

## D.6 Bulk download mechanisms

For each tier, at least one of the following MUST be available:

| Mechanism | Use when |
|---|---|
| Direct ZIP archive download | Small-to-medium collections (<100GB) |
| BagIt over HTTPS | Standardised research transfer |
| OAI-PMH endpoint | Legacy aggregator compatibility |
| S3 / Object Store with public-read prefix | Cloud-hosted large collections |
| BitTorrent / IPFS | Bandwidth-constrained but cooperatively distributable |

The mechanism MUST be documented in the collection's primary catalogue page (HTML and DCAT distribution).

## D.7 Worked examples

### D.7.1 Baseline manifest with sitemap

```xml
<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>https://example.org/ncap/photo-001</loc>
    <lastmod>2024-04-30</lastmod>
  </url>
  <url>
    <loc>https://example.org/ncap/photo-002</loc>
    <lastmod>2024-04-30</lastmod>
  </url>
</urlset>
```

### D.7.2 Content negotiation example (HTTP request/response)

```http
GET /ncap/photo-001 HTTP/1.1
Accept: text/turtle

HTTP/1.1 200 OK
Content-Type: text/turtle

@prefix naph: <https://w3id.org/naph/ontology#> .
ex:photo-001 a naph:AerialPhotograph ;
    rdfs:label "Berlin reconnaissance frame 4023" ;
    ...
```

## D.8 Validation

A SHACL shape (`naph:CollectionShape`) checks:

- The collection has a recognised packaging declaration
- All records are reachable via the manifest
- Each record's `naph:hasIdentifier` resolves to RDF (HEAD request OK; full content negotiation tested by toolkit)

## D.9 Common errors

| Error | Why it matters | Remediation |
|---|---|---|
| Records only available via search interface, no bulk URL list | Computational pipelines cannot iterate | Expose a sitemap or DCAT catalogue |
| Identifier returns HTML only, no RDF | RDF clients cannot consume | Implement content negotiation |
| BagIt without checksum manifest | Integrity not verifiable | Generate `manifest-sha256.txt` |
| IIIF manifest with broken Image API service | Viewers fail | Run a real Image API server (Cantaloupe, IIPImage) |

## D.10 Cross-references

- [Module A — Capture & Imaging](A-capture-imaging.md)
- [Module B — Metadata & Data Structures](B-metadata-data-structures.md)
- [Module F — QA & Validation](F-qa-validation.md)
- [BagIt RFC 8493](https://datatracker.ietf.org/doc/html/rfc8493)
- [RO-Crate 1.1](https://www.researchobject.org/ro-crate/1.1/)
- [IIIF Presentation 3.0](https://iiif.io/api/presentation/3.0/)
- [IIIF Image 3.0](https://iiif.io/api/image/3.0/)
