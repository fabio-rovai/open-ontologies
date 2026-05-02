# NAPH SPARQL Query Library

A library of ready-to-use SPARQL queries for common research workflows on NAPH-compliant aerial photography heritage collections.

Each query file is self-contained, with standard prefixes, a description of the use case, and notes on which NAPH tier is required for the query to return useful results.

## Categories

- **[Discovery](discovery.sparql)** — find records by metadata criteria
- **[Spatial](spatial.sparql)** — geographic filtering and intersection
- **[Temporal](temporal.sparql)** — date-range and temporal aggregation
- **[Provenance](provenance.sparql)** — provenance audits and lineage tracing
- **[Rights](rights.sparql)** — rights-aware filtering for publication and reuse
- **[Aggregation](aggregation.sparql)** — collection-level statistics and summaries
- **[Federation](federation.sparql)** — federated queries across NAPH and external authorities
- **[Compliance](compliance.sparql)** — tier compliance and validation queries

## Standard prefixes

All queries assume the following prefixes:

```sparql
PREFIX naph:    <https://w3id.org/naph/ontology#>
PREFIX dcat:    <http://www.w3.org/ns/dcat#>
PREFIX dcterms: <http://purl.org/dc/terms/>
PREFIX dctype:  <http://purl.org/dc/dcmitype/>
PREFIX prov:    <http://www.w3.org/ns/prov#>
PREFIX skos:    <http://www.w3.org/2004/02/skos/core#>
PREFIX geo:     <http://www.opengis.net/ont/geosparql#>
PREFIX geof:    <http://www.opengis.net/def/function/geosparql/>
PREFIX foaf:    <http://xmlns.com/foaf/0.1/>
PREFIX rdfs:    <http://www.w3.org/2000/01/rdf-schema#>
PREFIX xsd:     <http://www.w3.org/2001/XMLSchema#>
```

## Tier requirement notation

Each query is annotated with the minimum NAPH tier required:

- **B** — Baseline tier sufficient
- **E** — Enhanced tier required
- **A** — Aspirational tier required

## Engine notes

- **GeoSPARQL queries** require an engine that implements GeoSPARQL spatial functions: Apache Jena Fuseki, GraphDB, Stardog. They DO NOT work in Oxigraph (the engine backing Open Ontologies).
- **Federated queries** require SPARQL 1.1 federation support (`SERVICE` keyword) and outbound network access from the SPARQL endpoint to external services.
- **GROUP_CONCAT and aggregations** are supported by all major engines.
