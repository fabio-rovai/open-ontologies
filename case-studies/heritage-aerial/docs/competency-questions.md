# Competency Questions — what the standard lets you answer

A standard is only valuable if it lets researchers answer questions the previous representation couldn't. This document defines a set of **competency questions** — research questions that should be answerable via SPARQL against any NAPH-compliant dataset — and demonstrates each one against the sample dataset.

Run via:

```bash
cd /Users/fabio/projects/open-ontologies/case-studies/heritage-aerial
open-ontologies batch --pretty docs/competency-queries.batch.txt
```

## CQ1 — Temporal range

> "Which photographs were captured during the Second World War, with their dates?"

```sparql
PREFIX naph: <https://w3id.org/naph/ontology#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
SELECT ?photo ?label ?date
WHERE {
  ?photo a naph:AerialPhotograph ;
         rdfs:label ?label ;
         naph:capturedOn ?date .
  FILTER (?date >= "1939-09-01"^^xsd:date && ?date <= "1945-09-02"^^xsd:date)
}
ORDER BY ?date
```

**Why this matters:** the current Air Photo Finder lets you filter by date but not aggregate across the collection. A researcher writing a grant proposal needs to know how much material exists per year, by collection, with what rights status — without browsing 30 million records.

## CQ2 — Spatial coverage

> "Which photographs cover central Edinburgh?"

```sparql
PREFIX naph: <https://w3id.org/naph/ontology#>
PREFIX geo: <http://www.opengis.net/ont/geosparql#>
PREFIX geof: <http://www.opengis.net/def/function/geosparql/>
SELECT ?photo ?label ?date WHERE {
  ?photo a naph:AerialPhotograph ;
         rdfs:label ?label ;
         naph:capturedOn ?date ;
         naph:coversArea/naph:asWKT ?wkt .
  FILTER (geof:sfIntersects(?wkt, "POLYGON((-3.21 55.94, -3.16 55.94, -3.16 55.97, -3.21 55.97, -3.21 55.94))"^^geo:wktLiteral))
}
ORDER BY ?date
```

**Why this matters:** computational change detection over fixed locations across decades — a foundational use case for aerial photography research — requires bulk spatial query, not hand-clicking a map.

## CQ3 — Rights-aware access

> "Which photographs can I include in an open-access publication today?"

```sparql
PREFIX naph: <https://w3id.org/naph/ontology#>
SELECT ?photo ?label ?rightsLabel WHERE {
  ?photo a naph:AerialPhotograph ;
         rdfs:label ?label ;
         naph:hasRightsStatement ?rights .
  ?rights naph:rightsLabel ?rightsLabel ;
          naph:rightsURI ?rightsURI .
  FILTER (CONTAINS(STR(?rightsURI), "NoC") || CONTAINS(STR(?rightsURI), "publicdomain"))
}
```

**Why this matters:** today, rights status is implicit, requiring case-by-case clearance. With machine-readable rights statements, the entire collection becomes filterable for downstream uses.

## CQ4 — Cross-collection linking

> "Which photographs are linked to Wikidata historical event records?"

```sparql
PREFIX naph: <https://w3id.org/naph/ontology#>
PREFIX skos: <http://www.w3.org/2004/02/skos/core#>
SELECT ?photo ?label ?event ?wikidata WHERE {
  ?photo a naph:AerialPhotograph ;
         rdfs:label ?label ;
         naph:relatedToEvent ?event .
  ?event a naph:HistoricEvent ;
         skos:exactMatch ?wikidata .
  FILTER (CONTAINS(STR(?wikidata), "wikidata.org"))
}
```

**Why this matters:** linking photographs to Wikidata events makes them discoverable through every Wikidata-based research workflow, dramatically extending audience reach without any new digitisation.

## CQ5 — Provenance audit

> "Which photographs in the collection came via the NARA partnership?"

```sparql
PREFIX naph: <https://w3id.org/naph/ontology#>
SELECT ?photo ?label ?chainLabel WHERE {
  ?photo a naph:AerialPhotograph ;
         rdfs:label ?label ;
         naph:hasProvenanceChain ?chain .
  ?chain rdfs:label ?chainLabel .
  FILTER (CONTAINS(?chainLabel, "NARA"))
}
```

**Why this matters:** provenance audit is critical for repatriation requests, attribution accuracy, and rights clearance. Today this requires manual finding-aid review.

## CQ6 — Tier compliance distribution

> "How is the collection distributed across the three compliance tiers?"

```sparql
PREFIX naph: <https://w3id.org/naph/ontology#>
SELECT ?tierLabel (COUNT(?photo) AS ?count) WHERE {
  ?photo naph:compliesWithTier ?tier .
  ?tier rdfs:label ?tierLabel .
}
GROUP BY ?tierLabel
ORDER BY DESC(?count)
```

**Why this matters:** institutions need to know their starting position before they can plan upgrades. This query becomes the dashboard view for any institution adopting the standard.

## CQ7 — High-research-value subset

> "Which photographs have all three: post-1944 capture, urban subject linkage, and rights cleared for open use?"

```sparql
PREFIX naph: <https://w3id.org/naph/ontology#>
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>
SELECT DISTINCT ?photo ?label WHERE {
  ?photo a naph:AerialPhotograph ;
         rdfs:label ?label ;
         naph:capturedOn ?date ;
         naph:depicts ?place ;
         naph:hasRightsStatement ?rights .
  ?place a naph:Place .
  ?rights naph:rightsURI ?rightsURI .
  FILTER (?date >= "1944-01-01"^^xsd:date)
  FILTER (CONTAINS(STR(?rightsURI), "NoC") || CONTAINS(STR(?rightsURI), "publicdomain"))
}
```

**Why this matters:** combining temporal, semantic, and rights filters in a single query is impossible against current archive systems. With NAPH compliance, this becomes routine — a 5-line SPARQL query.

## CQ8 — Capture-context filtering

> "Which photographs were captured by a Spitfire above 6000m altitude?"

```sparql
PREFIX naph: <https://w3id.org/naph/ontology#>
SELECT ?photo ?label ?altitude WHERE {
  ?photo a naph:AerialPhotograph ;
         rdfs:label ?label ;
         naph:partOfSortie ?sortie ;
         naph:hasCaptureEvent ?capture .
  ?sortie naph:aircraft ?aircraft .
  ?capture naph:flightAltitude ?altitude .
  FILTER (CONTAINS(?aircraft, "Spitfire"))
  FILTER (?altitude >= 6000.0)
}
```

**Why this matters:** capture-context queries enable analysis of image quality factors (resolution implications of altitude, distortion from camera type) at scale. Currently impossible without manually cross-referencing flight logs.

## Verification

The companion file [`competency-queries.batch.txt`](competency-queries.batch.txt) contains all 8 queries as a runnable batch. Expected results against the v0.1 sample dataset:

| CQ | Expected | Verified ✅ |
|----|----------|-------------|
| CQ1 | 5 photographs from WWII period | records 1, 3, 4, 8, 10 |
| CQ2 | 2 photographs in central Edinburgh | records 2, 9 (requires GeoSPARQL) |
| CQ3 | 7 photographs with open rights | records 1, 3, 4, 6, 8, 9, 10 |
| CQ4 | 2 photographs linked to Wikidata events | records 8, 10 |
| CQ5 | 2 photographs from NARA partnership | records 6, 10 |
| CQ6 | tier distribution: Enhanced=4, Baseline=3, Aspirational=3 | confirmed |
| CQ7 | 2 photographs (Aspirational with Place + open rights post-1944) | records 9, 10 |
| CQ8 | 2 photographs (Spitfire ≥6000m) | records 4, 9 |
