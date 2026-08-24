# SEOM technical note

Companion to [case-study.md](case-study.md). Documents the data model, the IRI scheme, and worked SPARQL queries.

## Source

Static snapshot of the Skills England Occupational Maps Public API (`https://occupational-maps-api.skillsengland.education.gov.uk/api/v1`), harvested 6 July 2026 with the `X-API-KEY` header. Occupation detail was retrieved with the `expand` set `occupation.summary,occupation.overview,occupation.involvedemployers,occupation.keywords,occupation.typicaljobtitles,occupation.soc,occupation.maphierarchy,occupation.products,occupation.links`. Progression was retrieved per occupation from `OccupationalProgression/{stdCode}` (327 of 1,269 standards return no progression map; that is expected). Green-theme membership was reconstructed from the `GreenTheme/{themeId}` occupation trees.

## Model

Namespaces: vocabulary `seom: https://gov.tesseract.academy/ns/seom#`, instances `https://gov.tesseract.academy/id/seom/`.

### Classes

| Class | Meaning |
|---|---|
| `seom:Occupation` | An occupational standard (e.g. OCC0118). |
| `seom:Route`, `seom:Pathway`, `seom:Cluster` | The three levels of the occupational-map hierarchy (subclasses of `skos:Concept`). |
| `seom:TechnicalLevel` | Technical, Higher Technical, Professional. |
| `seom:OccupationStatus` | Approved occupation, standard in development, etc. |
| `seom:Product`, `seom:ProductType` | A technical education product and its type. |
| `seom:SOCConcept` | An ONS SOC 2010 or 2020 code (subclass of `skos:Concept`). |
| `seom:GreenTheme` | A green-jobs theme (subclass of `skos:Concept`). |

### Key properties

| Property | Domain to range |
|---|---|
| `seom:inRoute` / `seom:inPathway` / `seom:inCluster` | Occupation to hierarchy concept |
| `seom:atTechnicalLevel` | Occupation to TechnicalLevel |
| `seom:hasStatus` | Occupation to OccupationStatus |
| `seom:socMapping2020` / `seom:socMapping2010` | Occupation to SOCConcept |
| `seom:deliveredThrough` | Occupation to Product |
| `seom:progressesTo` | Occupation to Occupation (transitive) |
| `seom:inGreenTheme` | Occupation to GreenTheme |
| `seom:stdCode`, `seom:level`, `seom:versionNo`, `dct:description` (summary), `seom:overview`, `seom:keyword`, `seom:typicalJobTitle`, `seom:greenJobTitle` | Occupation literals |

The Route to Pathway to Cluster hierarchy is expressed with `skos:broader` / `skos:narrower`; SOC 2010, SOC 2020 and the green themes are each their own `skos:ConceptScheme`.

## Notes on the data

- **Progression edges** are de-duplicated by RDF set semantics: 3,293 raw `stdCodeFrom -> stdCodeTo` pairs across the progression maps collapse to 2,717 distinct directed edges.
- **Products** are keyed by product code; 1,990 occupation-to-product links resolve to 1,313 distinct products (a product can serve several occupations).
- **SOC code 0** in the source means "no mapping" and is dropped, so `seom:SOCConcept` nodes are real codes only.
- **Sector Subject Area (`ssa`)** was null across the snapshot and is not modelled.

## Worked SPARQL

Occupations in the Digital route that map to a given SOC 2020 group:

```sparql
PREFIX seom: <https://gov.tesseract.academy/ns/seom#>
PREFIX skos: <http://www.w3.org/2004/02/skos/core#>
SELECT ?occ ?name ?soc WHERE {
  ?r skos:prefLabel "Digital" .
  ?o seom:inRoute ?r ; skos:prefLabel ?name ;
     seom:socMapping2020 ?s .
  ?s skos:notation ?soc .
}
```

Full progression neighbourhood of Data analyst (transitive):

```sparql
PREFIX seom: <https://gov.tesseract.academy/ns/seom#>
SELECT DISTINCT ?name WHERE {
  <https://gov.tesseract.academy/id/seom/occupation/OCC0118> seom:progressesTo+ ?o .
  ?o <http://www.w3.org/2000/01/rdf-schema#label> ?name .
}
```

Apprenticeship and HTQ offer under a green theme:

```sparql
PREFIX seom: <https://gov.tesseract.academy/ns/seom#>
PREFIX skos: <http://www.w3.org/2004/02/skos/core#>
SELECT ?theme ?occName ?prodName WHERE {
  ?o seom:inGreenTheme ?t ; skos:prefLabel ?occName ;
     seom:deliveredThrough ?p .
  ?t skos:prefLabel ?theme .
  ?p <http://www.w3.org/2000/01/rdf-schema#label> ?prodName .
}
```

## Validation

`validate_ontology.py` runs two independent checks: SHACL (pyshacl) against [ontology/shapes.ttl](ontology/shapes.ttl), and SPARQL coverage plus referential integrity via the open-ontologies Oxigraph engine. Current result: conforms, 0 violations, 51,355 triples; all four integrity checks 0. See [ontology/coverage-report.md](ontology/coverage-report.md).
