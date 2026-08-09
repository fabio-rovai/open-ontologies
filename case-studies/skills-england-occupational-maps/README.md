# Skills England Occupational Maps Ontology (SEOM)

An open, machine-readable ontology of the **Skills England occupational maps**: every occupational standard, its place in the Route to Pathway to Cluster hierarchy, its ONS Standard Occupational Classification (SOC 2010 and 2020) mappings, its apprenticeship and technical education products, its green-jobs classification, and the progression relationships between occupations.

Built from a static snapshot of the [Skills England Occupational Maps Public API](https://occupational-maps.skillsengland.education.gov.uk/public-api/), harvested on 6 July 2026.

## What is in the graph

| Entity | Count |
|---|---:|
| Occupational standards | 1,269 |
| Routes | 15 |
| Pathways | 35 |
| Clusters | 172 |
| SOC 2020 concepts (crosswalk) | 278 |
| SOC 2010 concepts (crosswalk) | 246 |
| Technical education products | 1,313 |
| Green themes | 8 |
| Progression edges | 2,717 |
| **Total triples** | **51,355** |

SHACL validation: **conforms, 0 violations**. See [ontology/coverage-report.md](ontology/coverage-report.md).

## Layout

```
data/                          static snapshot of the Public API (the reproducible source)
  reference.json               routes, green themes, statuses, technical levels, product types, API version
  occupations-list.json        all 1,269 standards (summary records)
  occupation-details.json      full detail per standard (SOC, map hierarchy, products, job titles, ...)
  progression.json             occupational progression maps
  occupation-green-themes.json occupation -> green theme id map
ontology/
  seom-vocabulary.ttl/.jsonld  TBox: classes, properties, SKOS classification schemes, SOC crosswalk
  occupational-map.ttl/.jsonld ABox: the 1,269 standards as instances with all relations
  shapes.ttl                   SHACL shapes
  coverage-report.md           validation + coverage output
build_ontology.py              rebuild the ontology from data/
validate_ontology.py           SHACL + SPARQL coverage validation
```

## Reproduce

```bash
python -m venv .venv && . .venv/bin/activate
pip install rdflib pyshacl pyoxigraph
python build_ontology.py       # writes ontology/*.ttl and *.jsonld
python validate_ontology.py    # SHACL + SPARQL coverage -> ontology/coverage-report.md
```

To refresh the snapshot you need a Skills England API key (request one via their public-api page); the harvester passes it as the `X-API-KEY` header. The key is never stored in this repository.

## Namespaces

- Vocabulary: `https://gov.tesseract.academy/ns/seom#`
- Instances: `https://gov.tesseract.academy/id/seom/`

## Licence

The Skills England data is Crown copyright, licensed under the [Open Government Licence v3.0](https://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/). This ontology contains public sector information from Skills England used under that licence. This is an independent Tesseract Academy work and is not endorsed by Skills England.
