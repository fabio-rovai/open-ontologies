# Tutorial 1 — Your First NAPH Record

A step-by-step walkthrough for creating a single NAPH-compliant aerial photograph record from scratch. By the end of this tutorial you will have:

- A valid Turtle file containing one Baseline-tier compliant aerial photograph record
- Validated it against the NAPH SHACL shapes
- Run a self-assessment to confirm compliance
- Understood the structural requirements for Baseline tier

**Estimated time:** 30-45 minutes
**Prerequisites:** A text editor; Open Ontologies CLI installed; Python 3.10+

## Step 0 — Set up

Install the Open Ontologies CLI if you haven't already:

```bash
curl -L https://github.com/fabio-rovai/open-ontologies/releases/latest/download/open-ontologies-aarch64-apple-darwin -o /usr/local/bin/open-ontologies
chmod +x /usr/local/bin/open-ontologies
```

(For Linux x86_64, replace `aarch64-apple-darwin` with `x86_64-unknown-linux-gnu`.)

Verify:

```bash
open-ontologies --help | head -3
```

You should see something like:

```
Terraform for Knowledge Graphs — AI-native ontology engine

Usage: open-ontologies [OPTIONS] <COMMAND>
```

## Step 1 — Pick a record to model

Use one of your real records or follow this worked example. We'll model a hypothetical aerial photograph:

- **What:** Single frame from RAF reconnaissance over Edinburgh
- **When:** 4 August 1946
- **Where:** Edinburgh Castle and Royal Mile
- **Who:** RAF 541 Squadron, Spitfire PR.XIX
- **Sortie reference:** RAF/541/EDI/1946-08
- **Frame number:** 2287
- **Rights:** Crown Copyright Expired (now public domain)

## Step 2 — Create the Turtle file

Create a new file `my-first-record.ttl`:

```turtle
@prefix rdf:     <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs:    <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd:     <http://www.w3.org/2001/XMLSchema#> .
@prefix dcterms: <http://purl.org/dc/terms/> .
@prefix dctype:  <http://purl.org/dc/dcmitype/> .
@prefix geo:     <http://www.opengis.net/ont/geosparql#> .
@prefix naph:    <https://w3id.org/naph/ontology#> .
@prefix ex:      <https://w3id.org/example/> .

# Custodial institution
ex:NCAP a naph:CustodialInstitution ;
    rdfs:label "National Collection of Aerial Photography" .

# Collection
ex:NCAPCollection a naph:Collection ;
    rdfs:label "NCAP Holdings" ;
    naph:custodian ex:NCAP .

# Rights statement (reusable across records with the same rights)
ex:CrownCopyrightExpired a naph:RightsStatement ;
    naph:rightsURI <http://rightsstatements.org/vocab/NoC-OKLR/1.0/> ;
    naph:rightsLabel "No Copyright — Other Known Legal Restrictions" .

# Sortie
ex:sortie-RAF-541-EDI-1946-08 a naph:Sortie ;
    rdfs:label "Sortie RAF/541/Edinburgh/1946" ;
    naph:collectionCode "RAF" ;
    naph:sortieReference "541/EDI/1946-08" .

# The photograph itself
ex:photo-001 a naph:AerialPhotograph ;
    dcterms:type dctype:StillImage ;
    rdfs:label "Edinburgh Castle and Royal Mile frame 2287" ;
    naph:hasIdentifier "https://w3id.org/example/photo/RAF-541-EDI-1946-08-2287" ;
    naph:partOfSortie ex:sortie-RAF-541-EDI-1946-08 ;
    naph:frameNumber 2287 ;
    naph:belongsToCollection ex:NCAPCollection ;
    naph:capturedOn "1946-08-04"^^xsd:date ;
    naph:coversArea ex:footprint-001 ;
    naph:hasRightsStatement ex:CrownCopyrightExpired ;
    naph:compliesWithTier naph:TierBaseline .

# Geographic footprint (WGS84 polygon, longitude first)
ex:footprint-001 a naph:GeographicFootprint ;
    naph:asWKT "POLYGON((-3.205 55.946, -3.185 55.946, -3.185 55.952, -3.205 55.952, -3.205 55.946))"^^geo:wktLiteral .
```

## Step 3 — Validate Turtle syntax

```bash
open-ontologies validate my-first-record.ttl
```

Expected output:

```json
{"ok":true,"triples":N}
```

Where N is the number of RDF triples in your file (should be ~20). If you see syntax errors, check:

- Every triple ends with a `.` or `;`
- Prefixes are declared at the top
- Strings use double quotes
- Class IRIs are namespaced (e.g. `naph:AerialPhotograph`, not `AerialPhotograph`)

## Step 4 — Validate against NAPH SHACL shapes

For SHACL validation you need:

1. The NAPH ontology (`naph-core.ttl`)
2. The NAPH shapes (`naph-shapes.ttl`)
3. Your record file

```bash
echo "clear
load /path/to/naph-core.ttl
load my-first-record.ttl
shacl /path/to/naph-shapes.ttl" | open-ontologies batch
```

Expected output:

```json
{"command":"shacl","result":{"conforms":true,"violation_count":0,"violations":[]},"seq":3}
```

If you see violations, common causes:

- "Photograph must have exactly one stable identifier" — missing `naph:hasIdentifier`
- "Capture date must be a single ISO 8601 date" — wrong date format or datatype
- "Photograph must reference a GeographicFootprint" — missing `naph:coversArea`
- "Photograph must have a machine-readable rights statement" — missing `naph:hasRightsStatement`

## Step 5 — Run self-assessment

```bash
python3 /path/to/pipeline/self-assessment.py my-first-record.ttl
```

You should see:

```
======================================================================
NAPH Self-Assessment Report
======================================================================
Spec version:    1.0
Assessed at:     2026-05-01T...
Data file:       my-first-record.ttl

Overall result:  PASS
  · All SHACL shapes conform

Total NAPH records: 1

Tier distribution:
  Baseline       1

SHACL conformance: CONFORMS (0 violations)
======================================================================
```

## Step 6 — Run a SPARQL query

Confirm the record is queryable:

```bash
echo "clear
load /path/to/naph-core.ttl
load my-first-record.ttl
query \"PREFIX naph: <https://w3id.org/naph/ontology#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
SELECT ?photo ?label ?date WHERE {
    ?photo a naph:AerialPhotograph ;
           rdfs:label ?label ;
           naph:capturedOn ?date .
}\"" | open-ontologies batch
```

Expected output: a single row with your photograph.

## What you've achieved

You have a Baseline NAPH-compliant record. It satisfies:

- ✅ Stable HTTP identifier
- ✅ ISO 8601 date
- ✅ WGS84 polygon footprint
- ✅ Machine-readable rights statement
- ✅ Sortie and Collection links
- ✅ DCMI Type assertion
- ✅ SHACL validation passes

A researcher with access to your collection's SPARQL endpoint can now query this record by date, location, sortie, rights, or collection. They can include it in federated queries with peer institutions. They can cite it via stable URI.

## Common pitfalls

### Wrong WKT coordinate order

GeoSPARQL WKT uses **longitude first, latitude second**:

✅ `POLYGON((-3.205 55.946, ...))` — longitude -3.205, latitude 55.946 (Edinburgh)
❌ `POLYGON((55.946 -3.205, ...))` — would place the record in the South Atlantic Ocean

### Polygon doesn't close

A WKT polygon's first and last coordinates must be identical:

✅ `POLYGON((-3.205 55.946, -3.185 55.946, -3.185 55.952, -3.205 55.952, -3.205 55.946))`
❌ `POLYGON((-3.205 55.946, -3.185 55.946, -3.185 55.952, -3.205 55.952))` — open polygon, validation may fail

### Date as plain string

Dates must have an XSD type annotation:

✅ `naph:capturedOn "1946-08-04"^^xsd:date`
❌ `naph:capturedOn "1946-08-04"` — plain string, won't be filterable as a date

### Free-text date

Even partial dates must use XSD types:

✅ `naph:capturedOn "1946"^^xsd:gYear`
✅ `naph:capturedOn "1946-08"^^xsd:gYearMonth`
❌ `naph:capturedOn "Summer 1946"` — free-text, not allowed

### Missing collection link

Every record must link to a collection:

✅ `ex:photo-001 naph:belongsToCollection ex:NCAPCollection`
❌ Missing this link → SHACL violation

### Wrong rights URI form

Use the canonical `vocab/` form, not the human-readable `page/` form:

✅ `naph:rightsURI <http://rightsstatements.org/vocab/NoC-OKLR/1.0/>`
❌ `naph:rightsURI <https://rightsstatements.org/page/NoC-OKLR/1.0/>` — human page, not RDF canonical

## Next steps

- **Tutorial 2** — Upgrade your record to Enhanced tier (digitisation provenance, capture context, provenance chain)
- **Tutorial 3** — Reach Aspirational tier (subject classification, place authorities, cross-collection links)
- **Tutorial 4** — Bulk ingest from CSV — applying the same transformation to thousands of records

## Cross-references

- [How to use this standard](../how-to-use-this-standard.md)
- [Validation checklists](../validation-checklists.md)
- [Module B — Metadata & Data Structures](../../01-standard/modules/B-metadata-data-structures.md)
- [Sample data](../../../data/sample-photographs.ttl) — 10 worked records, all 3 tiers
