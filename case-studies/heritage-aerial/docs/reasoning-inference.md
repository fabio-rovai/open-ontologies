# Reasoning & Inference — what the standard gives you for free

NAPH is built by **subsuming existing standards** rather than reinventing them. The key consequence: applying RDFS reasoning to a NAPH-compliant collection automatically infers membership in PROV-O, DCAT, SKOS, and GeoSPARQL — meaning every NAPH record is simultaneously discoverable through every linked-data tool that speaks those standards.

This document shows what reasoning produces, with live numbers from the sample dataset.

## Run it yourself

```bash
echo "clear
load ontology/naph-core.ttl
load data/sample-photographs.ttl
stats
reason rdfs
stats" | open-ontologies batch
```

## Results — before and after RDFS reasoning

| Metric | Before reasoning | After RDFS reasoning | Inferred |
|---|---|---|---|
| Total triples | 496 | 565 | **+69** |
| Total individuals | 66 | 74 | **+8** |

The inferred triples come from RDFS subclass propagation: NAPH classes are subclasses of standards classes, so every NAPH instance gains membership in the parent standard.

## What gets inferred — and why it matters

### NAPH photographs become DCAT Datasets (×11)

```sparql
PREFIX dcat: <http://www.w3.org/ns/dcat#>
SELECT (COUNT(?s) AS ?datasets) WHERE { ?s a dcat:Dataset }
# → 11 (10 AerialPhotograph + 1 Collection)
```

**Why this matters:** every DCAT-aware tool — CKAN portals, data.gov.uk, the European Data Portal, Schema.org/Dataset structured data on the web — can now find these aerial photographs as datasets, without any extra modelling. NAPH publishes once and is discoverable everywhere DCAT is supported.

### Capture events and digitisation events become PROV Activities (×7)

```sparql
PREFIX prov: <http://www.w3.org/ns/prov#>
SELECT (COUNT(?s) AS ?activities) WHERE { ?s a prov:Activity }
# → 7 (CaptureEvents + DigitisationEvents from Enhanced/Aspirational records)
```

**Why this matters:** PROV-O is the W3C standard for provenance. Any tool that audits provenance chains, traces data lineage, or visualises data history — Linked Open Data trackers, scholarly publishing pipelines, FAIR-data audit tools — can read the NAPH provenance graph natively.

### Places and historic events become SKOS Concepts (×6)

```sparql
PREFIX skos: <http://www.w3.org/2004/02/skos/core#>
SELECT (COUNT(?s) AS ?concepts) WHERE { ?s a skos:Concept }
# → 6 (Places + HistoricEvents in the Aspirational tier)
```

**Why this matters:** SKOS is the standard for thesauri and controlled vocabularies. Heritage authority files (AAT, ULAN, TGN, GeoNames) all expose data as SKOS. NAPH places auto-align with this ecosystem and benefit from SKOS hierarchical navigation, multilingual labels, and concept matching.

### Geographic footprints become GeoSPARQL Geometries

```sparql
PREFIX geo: <http://www.opengis.net/ont/geosparql#>
SELECT (COUNT(?s) AS ?geometries) WHERE { ?s a geo:Geometry }
# → 10 (one per photograph, inferred from naph:GeographicFootprint subclass)
```

**Why this matters:** GeoSPARQL is the OGC standard for spatial RDF. Inheriting from `geo:Geometry` means NAPH footprints work directly with GeoSPARQL spatial functions (`sfIntersects`, `sfWithin`, distance queries) supported by Apache Jena, Stardog, and Oxigraph itself.

### Provenance chains become PROV Bundles

```sparql
PREFIX prov: <http://www.w3.org/ns/prov#>
SELECT (COUNT(?s) AS ?bundles) WHERE { ?s a prov:Bundle }
# → 7 (one per Enhanced/Aspirational provenance chain)
```

**Why this matters:** PROV Bundles are reusable provenance documents that can be shared, signed, and published independently. NAPH provenance chains immediately become signable provenance documents conforming to PROV — useful for declassification audit, repatriation evidence, and rights clearance.

## The architectural insight

This is what "synthesis over invention" buys you. The NAPH ontology defines:

```turtle
naph:AerialPhotograph rdfs:subClassOf dcat:Dataset .
naph:CaptureEvent rdfs:subClassOf prov:Activity .
naph:DigitisationEvent rdfs:subClassOf prov:Activity .
naph:DigitalSurrogate rdfs:subClassOf prov:Entity .
naph:ProvenanceChain rdfs:subClassOf prov:Bundle .
naph:GeographicFootprint rdfs:subClassOf geo:Geometry .
naph:Place rdfs:subClassOf skos:Concept .
naph:HistoricEvent rdfs:subClassOf skos:Concept .
naph:Subject rdfs:subClassOf skos:Concept .
naph:Collection rdfs:subClassOf dcat:Catalog .
naph:CustodialInstitution rdfs:subClassOf foaf:Organization .
```

Eleven `rdfs:subClassOf` declarations. That's the entire integration surface with the existing linked-data heritage ecosystem.

A standard that re-invents these classes would force institutions to dual-publish, maintain crosswalks, and stay in sync as parent standards evolve. NAPH inherits, defers to authority, and benefits from every tool built for the parent standards — for free.

## The compounding effect

Every new tool that joins the PROV/DCAT/SKOS/GeoSPARQL ecosystem automatically becomes a tool that works with NAPH-compliant collections. Every Wikidata entity that gets a SKOS exact-match becomes a NAPH-linkable concept. Every CKAN portal that adds DCAT support gains aerial photography records.

This is why the choice of design philosophy — synthesis over invention, FAIR through inheritance, compliance through subclass — matters more than any individual modelling decision in the standard.

A standard that doesn't subsume existing authorities will become legacy within five years. NAPH is designed to be one extension of an evolving open-standards ecosystem, not a competing centre.
