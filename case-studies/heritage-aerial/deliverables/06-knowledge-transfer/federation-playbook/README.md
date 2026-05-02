# Cross-Collection Federation Playbook

How to run federated SPARQL queries across NAPH-compliant aerial photography collections at multiple institutions. The playbook covers technical setup, query patterns, and operational considerations.

## What federation enables

Federation lets a researcher query "all aerial coverage of [location] across [date range] from any NAPH-compliant institution" — in one query. Instead of:

1. Searching NCAP's catalogue
2. Searching IWM's catalogue  
3. Searching NARA's catalogue
4. Manually deduplicating results
5. Comparing rights statuses across systems

A federated query returns combined results in seconds.

## Prerequisites

For federation to work:

1. **Each participating institution publishes a SPARQL endpoint** that exposes their NAPH-compliant collection
2. **Endpoints support SPARQL 1.1 federation** (`SERVICE` keyword) — true for Apache Jena Fuseki, GraphDB, Stardog, Virtuoso
3. **Endpoints are publicly accessible** (no authentication required for read access)
4. **Endpoints have stable URLs** registered in the [NAPH compliance registry](../../../registry/)

If even one institution doesn't have a SPARQL endpoint, they can't participate in federation. (They can still publish NAPH-compliant data via bulk download — federation just isn't possible for them.)

## Endpoint requirements

A federation-ready SPARQL endpoint MUST:

- Be at a stable, documented URL
- Support SPARQL 1.1 (SELECT, ASK, CONSTRUCT, DESCRIBE)
- Support SPARQL Federation (`SERVICE`)
- Provide a [VOID](https://www.w3.org/TR/void/) descriptor at `/.well-known/void`
- Support content negotiation (return Turtle or JSON-LD on request)
- Have CORS enabled (so browser-based clients can query)

A federation-ready SPARQL endpoint SHOULD:

- Support GeoSPARQL spatial functions (for spatial federation)
- Have query timeouts that allow complex federated queries (typically 30-120 seconds)
- Document its dataset descriptor (which graphs, what the data is)

## Query patterns

### Pattern 1 — Multi-collection count

"How many records exist sector-wide?"

```sparql
PREFIX naph: <https://w3id.org/naph/ontology#>

SELECT (SUM(?count) AS ?total)
WHERE {
    {
        SELECT (COUNT(?photo) AS ?count) WHERE {
            SERVICE <https://sparql.ncap.org/> {
                ?photo a naph:AerialPhotograph
            }
        }
    } UNION {
        SELECT (COUNT(?photo) AS ?count) WHERE {
            SERVICE <https://sparql.iwm.org.uk/> {
                ?photo a naph:AerialPhotograph
            }
        }
    } UNION {
        SELECT (COUNT(?photo) AS ?count) WHERE {
            SERVICE <https://sparql.nara.archives.gov/> {
                ?photo a naph:AerialPhotograph
            }
        }
    }
}
```

### Pattern 2 — Multi-collection geographic query

"Find all aerial coverage of central London 1939-1945 across NAPH institutions."

```sparql
PREFIX naph: <https://w3id.org/naph/ontology#>
PREFIX geo: <http://www.opengis.net/ont/geosparql#>
PREFIX geof: <http://www.opengis.net/def/function/geosparql/>
PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>

SELECT ?endpoint ?photo ?date ?label
WHERE {
    VALUES ?endpoint {
        <https://sparql.ncap.org/>
        <https://sparql.iwm.org.uk/>
        <https://sparql.nara.archives.gov/>
    }
    SERVICE ?endpoint {
        ?photo a naph:AerialPhotograph ;
               rdfs:label ?label ;
               naph:capturedOn ?date ;
               naph:coversArea/naph:asWKT ?wkt .
        FILTER (?date >= "1939-09-01"^^xsd:date && ?date <= "1945-09-02"^^xsd:date)
        FILTER (geof:sfIntersects(?wkt,
            "POLYGON((-0.20 51.48, -0.05 51.48, -0.05 51.55, -0.20 51.55, -0.20 51.48))"^^geo:wktLiteral
        ))
    }
}
ORDER BY ?date
```

### Pattern 3 — Cross-institution stereo pair detection

"Find pairs of frames where one is in NCAP and the other in IWM, with overlapping coverage."

```sparql
PREFIX naph: <https://w3id.org/naph/ontology#>
PREFIX geo: <http://www.opengis.net/ont/geosparql#>
PREFIX geof: <http://www.opengis.net/def/function/geosparql/>

SELECT ?ncapPhoto ?iwmPhoto ?overlap
WHERE {
    SERVICE <https://sparql.ncap.org/> {
        ?ncapPhoto a naph:AerialPhotograph ;
                   naph:coversArea/naph:asWKT ?ncapWkt ;
                   naph:capturedOn ?ncapDate .
    }
    SERVICE <https://sparql.iwm.org.uk/> {
        ?iwmPhoto a naph:AerialPhotograph ;
                  naph:coversArea/naph:asWKT ?iwmWkt ;
                  naph:capturedOn ?iwmDate .
    }
    FILTER (geof:sfIntersects(?ncapWkt, ?iwmWkt))
    FILTER (?ncapDate = ?iwmDate)
    BIND (geof:area(geof:intersection(?ncapWkt, ?iwmWkt)) AS ?overlap)
    FILTER (?overlap > 0)
}
```

### Pattern 4 — Wikidata-enriched federation

"For aerial photographs depicting specific Wikidata historic events, list with multilingual labels."

```sparql
PREFIX naph: <https://w3id.org/naph/ontology#>
PREFIX skos: <http://www.w3.org/2004/02/skos/core#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>

SELECT ?photo ?eventLabel_en ?eventLabel_ja ?eventLabel_de
WHERE {
    SERVICE <https://sparql.ncap.org/> {
        ?photo a naph:AerialPhotograph ;
               naph:relatedToEvent ?event .
        ?event skos:exactMatch ?wikidataIRI .
    }
    SERVICE <https://query.wikidata.org/sparql> {
        OPTIONAL { ?wikidataIRI rdfs:label ?eventLabel_en . FILTER (LANG(?eventLabel_en) = "en") }
        OPTIONAL { ?wikidataIRI rdfs:label ?eventLabel_ja . FILTER (LANG(?eventLabel_ja) = "ja") }
        OPTIONAL { ?wikidataIRI rdfs:label ?eventLabel_de . FILTER (LANG(?eventLabel_de) = "de") }
    }
}
```

## Operational considerations

### Endpoint failures

What happens when one of the federated endpoints is down?

- Default behaviour: SPARQL Federation propagates the failure — your query fails
- Workaround: use `SERVICE SILENT` to ignore failures from individual endpoints

```sparql
SERVICE SILENT <https://sparql.iwm.org.uk/> {
    ?photo a naph:AerialPhotograph .
}
```

This produces partial results when an endpoint is down rather than total failure. Document which endpoints failed in any results.

### Performance

Federated queries can be slow. Optimisation strategies:

- **Reduce per-endpoint result size** — use FILTER and LIMIT inside each SERVICE block
- **Use ASK queries** when you only need existence, not retrieval
- **Cache aggressively** — federation is read-only; results can be cached
- **Run during off-peak hours** for batch federation

### Query timeouts

Most public SPARQL endpoints have 30-60 second query timeouts. Federated queries to multiple slow endpoints can exceed this.

If timeouts are a problem:

1. Decompose the federated query into per-endpoint queries
2. Run each query separately, store results
3. Combine in your application code

### Authentication and rate limiting

Some endpoints rate-limit anonymous queries (e.g. "100 queries per IP per hour"). For sustained programmatic use, contact the institution to negotiate access.

### Trust and verification

Federation results combine data from multiple institutions, each of which controls their own data quality. NAPH compliance + SHACL validation is the institutional guarantee, but cross-institution verification (e.g. Wikidata QID consistency) is the researcher's responsibility.

For critical work, validate cross-references — e.g. a Wikidata QID in NCAP should match the same QID's referent in NARA.

## Setting up a federation-ready endpoint

If your institution wants to be federation-ready:

### Step 1 — Choose a triple store

Recommended for production:

- **Apache Jena Fuseki** — most widely used, open source, full GeoSPARQL support
- **GraphDB** (free version) — excellent SPARQL Federation, GUI
- **Stardog** — good federation, requires licensing
- **Virtuoso Open Source** — established, full feature support
- **Oxigraph** — lightweight, but limited GeoSPARQL support — not recommended for primary endpoint

### Step 2 — Load the data

Load:

- The NAPH ontology (`naph-core.ttl`)
- The NAPH shapes (`naph-shapes.ttl`)
- Your collection data

### Step 3 — Configure CORS

Browser-based federation clients require CORS. Add to your endpoint config:

```
Access-Control-Allow-Origin: *
Access-Control-Allow-Methods: GET, POST, OPTIONS
Access-Control-Allow-Headers: Content-Type, Authorization
```

### Step 4 — Publish endpoint URL

Update the [NAPH compliance registry](../../../registry/) with your endpoint URL.

Add a VOID descriptor at `[endpoint]/.well-known/void`:

```turtle
@prefix void: <http://rdfs.org/ns/void#> .
@prefix dcterms: <http://purl.org/dc/terms/> .

<#dataset> a void:Dataset ;
    dcterms:title "[Your collection name]" ;
    void:sparqlEndpoint <[your endpoint URL]> ;
    void:vocabulary <https://w3id.org/naph/ontology> ;
    void:triples [N] ;
    dcterms:license <[your data licence]> .
```

### Step 5 — Test

Run a self-federation query to verify everything works:

```sparql
SELECT (COUNT(?photo) AS ?count)
WHERE {
    SERVICE <[your endpoint URL]> {
        ?photo a naph:AerialPhotograph .
    }
}
```

## When federation isn't enough

Federation is powerful but not suitable for:

- **Bulk research downloads** — use bulk download URLs instead
- **Citation-grade reproducibility** — federated queries can fail in interesting ways. For published research, document a snapshot of results.
- **Real-time UI** — federation queries can be slow; cache results in your application layer

## The N-RICH role

A future N-RICH service could provide:

- A canonical federation hub that queries all NAPH-compliant endpoints
- Caching to amortise federation cost
- A registry-driven query builder that automatically includes new institutions
- Result reconciliation across institutions

This is the natural sector-shared infrastructure layer that NAPH adoption unlocks.

## Cross-references

- [NAPH compliance registry](../../../registry/)
- [SPARQL library — Federation queries](../../04-adoption-guidance/sparql-library/federation.sparql)
- [Module D — Packaging & Publication](../../01-standard/modules/D-packaging-publication.md)
- [Apache Jena Fuseki](https://jena.apache.org/documentation/fuseki2/)
- [SPARQL 1.1 Federated Query](https://www.w3.org/TR/sparql11-federated-query/)
