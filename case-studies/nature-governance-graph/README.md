# UK Nature Governance Graph (NGG)

An open, computation-ready reference graph of the UK nature and environment
governance landscape: the statutory agencies, NGOs, data bodies, funders,
sector bodies, partnerships and international conventions that a public body
must navigate when it plans stakeholder engagement, and the **sourced**
relationships between them.

Part of [Open Ontologies](../../README.md). Released under CC BY 4.0.

## Why this exists

Stakeholder mapping for the environment sector is usually a slide: boxes and
arrows with no provenance, impossible to query, out of date the day it is
drawn. The relationships that matter (who sponsors whom, who advises
government internationally, whose data flows into the national biodiversity
record, which duties are statutory) are exactly the ones a diagram cannot
defend. This graph treats the landscape as data instead.

## What is in it

| | |
|---|---|
| Actors | **47** |
| Sourced relationships | **48** |
| Distinct cited sources | **42** |
| SHACL violations | **0** |
| Referential-integrity violations | **0** |
| Dangling relationships | **0** |

Seven actor classes (statutory-agency, ngo, data-body, funder, sector-body,
partnership, international) and seven controlled relationship predicates
(sponsors, advises, supplies-data-to, funds, partners-with, regulates,
designated-under).

## The design commitment: no unsourced edge

Every relationship is **reified** as its own node carrying a typed predicate,
a plain-language basis, and a `prov:wasDerivedFrom` link to a cited `Source`
with a resolvable URL. This is not a convention; it is enforced. The SHACL
`RelationshipShape` fails the build if any relationship lacks a source, and the
`SourceShape` fails if any source lacks a URL. A relationship you cannot cite
cannot enter the graph.

Where the tidy answer would be wrong, the graph records the accurate one: the
Forestry Commission is a non-ministerial department in the Defra group, not a
sponsored NDPB; NIEA is an executive agency inside DAERA; NatureScot, NRW and
SEPA are sponsored by the devolved governments, not Defra; statutory
biodiversity credits are sold by Natural England on behalf of Defra under
Schedule 7A of the Town and Country Planning Act 1990 (inserted by the
Environment Act 2021), and LNRS responsible authorities are appointed under
Environment Act 2021 s.104. Each of those is carried in the relationship's
`basis` and citation, not smoothed over.

## Files

- `data/entities.json` — the sourced source-of-truth (actors + relationships)
- `ontology/ngg.ttl` — the NGG vocabulary (reuses W3C ORG, PROV-O, SKOS, Dublin Core)
- `shapes/ngg-shapes.ttl` — SHACL shapes (labelling, controlled vocab, sourcing, referential integrity)
- `graph.ttl` / `graph.jsonld` — the built graph
- `queries/competency.rq` — the competency questions the graph answers
- `pipeline/build_and_validate.py` — reproducible build + validation
- `metrics.json` — machine-readable build metrics

## Reproduce

```bash
pip install rdflib pyshacl
python3 pipeline/build_and_validate.py
```

## Provenance and licence

Actor and relationship facts are drawn from official published sources (each
body's own site and legislation.gov.uk), cited per relationship. The graph is
released under CC BY 4.0. It is an independent, self-initiated reference built
entirely from public information; it represents no organisation's findings but
our own and is not endorsed by any body named within it.
