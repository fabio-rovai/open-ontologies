# Tutorial 3 — Reaching Aspirational Tier

You have an Enhanced-compliant record (from [Tutorial 2](02-upgrading-to-enhanced.md)). Now upgrade it to Aspirational tier — semantic discovery and cross-collection knowledge graph integration.

**Estimated time:** 60-90 minutes
**Prerequisites:** Tutorials 1-2

## What changes from Enhanced to Aspirational

Aspirational adds three semantic layers:

1. **Subject classification** (`naph:depicts`) — what the photograph shows
2. **Place authority linkage** — connection to GeoNames / Wikidata
3. **Cross-collection links** (`naph:linkedRecord`) — connection to peer institutions' records

## Step 1 — Identify subjects

What does this photograph depict?

For our Berlin reconnaissance frame:

- **Place:** Berlin (specifically central industrial district)
- **Historic event:** WWII reconnaissance aerial photography over Berlin (or more specifically: Allied bombing damage assessment)

For a Hiroshima frame:

- **Place:** Hiroshima
- **Historic event:** Atomic bombing of Hiroshima

For aerial archaeology:

- **Place:** specific archaeological site
- **Observed feature:** "cropmark-enclosure", "earthwork-mound", etc.

## Step 2 — Add place subjects

Define each place as a `naph:Place`:

```turtle
ex:place-Berlin a naph:Place, skos:Concept ;
    rdfs:label "Berlin" ;
    naph:placeAuthorityURI <https://www.geonames.org/2950159/berlin.html> ;
    skos:exactMatch <https://www.wikidata.org/wiki/Q64> .
```

The key fields:

- `naph:placeAuthorityURI` — link to GeoNames (or equivalent gazetteer)
- `skos:exactMatch` — link to Wikidata

For more specific locations:

```turtle
ex:place-Berlin-Industrial-District a naph:Place, skos:Concept ;
    rdfs:label "Berlin industrial district" ;
    naph:placeAuthorityURI <https://www.geonames.org/2950157/treptow-koepenick.html> ;
    skos:broader ex:place-Berlin .
```

`skos:broader` indicates that the industrial district is contained within Berlin.

## Step 3 — Add historic event subject (where applicable)

For records linked to historic events:

```turtle
ex:event-WWII-Reconnaissance a naph:HistoricEvent, skos:Concept ;
    rdfs:label "WWII Allied aerial reconnaissance" ;
    skos:exactMatch <https://www.wikidata.org/wiki/Q...> .
```

Choose Wikidata QIDs carefully — verify they actually point to the intended entity. (See [red-team report](../../../docs/red-team-report.md) — fabricating QIDs is the worst-case credibility failure.)

## Step 4 — Update the photograph to reference subjects

```turtle
ex:photo-001
    naph:depicts ex:place-Berlin, ex:place-Berlin-Industrial-District ;
    naph:relatedToEvent ex:event-WWII-Reconnaissance .
```

A photograph can depict multiple places; comma-separate them.

## Step 5 — Find cross-collection links

What other records exist that relate to this photograph?

For our Berlin frame, related records might include:

- IWM holdings of related sortie reports
- NARA holdings of complementary US-side reconnaissance
- A Wikidata article on the specific operation

Use `naph:linkedRecord` to express these:

```turtle
ex:photo-001 naph:linkedRecord
    <https://www.iwm.org.uk/collections/item/object/IWM-FLM-3001> ,  # IWM sortie report
    <https://catalog.archives.gov/id/12345> ,                       # NARA complementary record
    <https://www.wikidata.org/wiki/Q....> .                          # Wikidata article
```

For Aspirational tier, at least one cross-collection link is required.

## Step 6 — Update tier compliance

```turtle
ex:photo-001 naph:compliesWithTier naph:TierAspirational .
```

## Step 7 — Validate

```bash
echo "clear
load /path/to/naph-core.ttl
load my-aspirational-record.ttl
shacl /path/to/naph-shapes.ttl" | open-ontologies batch
```

Common Aspirational violations:

- "Aspirational tier requires subject classification" — missing `naph:depicts`
- "Aspirational tier requires at least one cross-collection linked record" — missing `naph:linkedRecord`
- "Place must have an external authority URI" — `naph:Place` without `naph:placeAuthorityURI`

## Step 8 — Self-assess

```bash
python3 /path/to/pipeline/self-assessment.py my-aspirational-record.ttl
```

Output:

```
Tier distribution:
  Aspirational   1
```

## Step 9 — Run an Aspirational-tier query

The point of Aspirational is to enable queries that don't work at lower tiers. Try:

```bash
echo "clear
load /path/to/naph-core.ttl
load my-aspirational-record.ttl
query \"PREFIX naph: <https://w3id.org/naph/ontology#>
PREFIX skos: <http://www.w3.org/2004/02/skos/core#>
SELECT ?photo ?wikidata WHERE {
    ?photo naph:depicts ?place .
    ?place skos:exactMatch ?wikidata .
    FILTER (CONTAINS(STR(?wikidata), 'wikidata.org'))
}\"" | open-ontologies batch
```

This query — "find all photographs that depict a Wikidata-identifiable place" — only returns useful results when records have `naph:depicts` linkages, which only Aspirational records have.

## What you've gained

At Aspirational tier, your collection participates in the wider linked-data ecosystem:

- **Federated queries** — researchers can join your collection with Wikidata, GeoNames, Pleiades, Canmore, IWM
- **Multilingual labels** — Wikidata provides labels in many languages, automatically queryable via federation
- **Knowledge-graph navigation** — clicking "Berlin" in your data leads to all records that mention Berlin across the linked-data web
- **AI-friendly enrichment** — vision-language models can validate or extend the subject classifications, bootstrapping further enrichment

## Tips for scaling

Doing all of this by hand for every record is impractical. For Aspirational tier at scale:

1. **Use vision-language models** to propose subject classifications (with confidence scores)
2. **Use entity-linking tools** to resolve text mentions to Wikidata / GeoNames automatically
3. **Sample-validate** — human-validate ~5-10% of automated classifications
4. **Track provenance** — every AI-derived field MUST include `naph:placeDerivedBy` and `naph:placeConfidence`

See [Module E.6 — AI-derived fields](../../01-standard/modules/E-paradata-workflow.md#e6-ai-derived-fields) for the structural requirement and [`pipeline/vision-classify.py`](../../../pipeline/vision-classify.py) (planned v0.4) for a reference implementation.

## Common pitfalls

### Picking wrong Wikidata QIDs

The biggest red-team finding was fabricated QIDs (Q11461 for "atomic bombing" actually pointed to "Sound"). **Always verify** Wikidata QIDs by visiting the page or running a SPARQL query against Wikidata.

### Over-broad subject classification

A photograph showing Berlin during WWII is `naph:depicts ex:place-Berlin`. It is NOT `naph:depicts ex:event-WWII` (an event isn't a place). Don't conflate places, events, themes — these have separate classes.

### Missing place authority URIs

Aspirational requires every `naph:Place` to have `naph:placeAuthorityURI`. If a place isn't in any gazetteer, that's a problem — either find a gazetteer entry or downgrade the record to Enhanced.

### Stale cross-collection links

URLs at peer institutions move. Aspirational tier requires `naph:linkedRecord` URIs to resolve. Run periodic link-health checks (see [Module F.A.3](../../01-standard/modules/F-qa-validation.md#f23-aspirational-f-aspirational)).

## Cross-references

- [Module B — Metadata & Data Structures](../../01-standard/modules/B-metadata-data-structures.md)
- [Module E — Paradata & Workflow](../../01-standard/modules/E-paradata-workflow.md) (AI-derived fields)
- [Module F — QA & Validation](../../01-standard/modules/F-qa-validation.md)
- [Sample data](../../../data/sample-photographs.ttl) — records 8-10 are Aspirational
- [SPARQL library — Federation](../sparql-library/federation.sparql)
