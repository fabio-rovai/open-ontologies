---
title: "The Skills England occupational maps as an ontology"
summary: "An open, machine-readable ontology of the entire Skills England occupational map: 1,269 occupational standards placed in the Route to Pathway to Cluster hierarchy, crosswalked to ONS SOC 2010 and 2020, linked to their apprenticeship and technical education products, their green-jobs themes, and to each other through progression pathways. Built from a static snapshot of the Skills England Public API, published as Turtle and JSON-LD, and validated at zero SHACL violations."
date: 2026-07-06
audience: "Skills analysts, curriculum and qualifications teams, labour-market researchers, and anyone integrating occupational-standards data."
---

# The Skills England occupational maps as an ontology

## The finding, first

Skills England publishes the national occupational maps through a public API: 1,269 occupational standards, arranged into 15 routes, 35 pathways and 172 clusters, each carrying a Standard Occupational Classification mapping, apprenticeship and technical education products, and a green-jobs classification, with progression relationships between occupations. That is a graph. Delivered as JSON over REST endpoints, it reads as documents; consumed the usual way, a slice of it ends up flattened into a spreadsheet where the relationships that make it valuable are lost.

We turned the whole thing into what it already is: a single, connected, machine-readable graph. Every occupational standard is a node; its route, pathway, cluster, technical level, status, SOC codes, products, green themes and progression targets are typed edges. The result is **51,355 triples** that validate against a formal SHACL schema with **zero violations**, published openly as Turtle and JSON-LD.

## Challenge

Occupational standards sit at the join between three systems that rarely share a data model: the education system (apprenticeships, T Levels, Higher Technical Qualifications), the labour market (ONS Standard Occupational Classification, used for every official statistic on jobs and pay), and skills policy (routes, clusters, green-jobs themes). The Skills England occupational maps are the object that connects them, which is exactly why they are hard to use well.

Three practical obstacles. First, the data is relational but the delivery is document-shaped: to answer "which occupations in the Digital route map to SOC 2020 code 2433, and what can they progress to?" you must fetch, expand and join across several endpoints by hand. Second, there is no published schema for the connected object, so every consumer re-invents one, usually implicitly and inconsistently. Third, the SOC crosswalk, the single most useful bridge to official labour-market statistics, is buried inside each occupation record rather than exposed as a first-class, queryable mapping.

## Intervention

We harvested a complete static snapshot of the Public API (all 1,269 standards with full detail, all routes, green themes, reference lists, and the progression map for every occupation) and modelled it as an ontology, SEOM.

Three design choices make it trustworthy and reusable.

1. **The map is modelled as it is defined, not as it is delivered.** The Route to Pathway to Cluster hierarchy is a SKOS concept scheme (`skos:broader` / `skos:narrower`), so it can be browsed, reasoned over and aligned like any other controlled vocabulary. Occupations are typed instances linked into that hierarchy with explicit object properties (`seom:inRoute`, `seom:inPathway`, `seom:inCluster`, `seom:atTechnicalLevel`).

2. **The SOC crosswalk is a first-class citizen.** Every distinct SOC 2010 and 2020 code referenced by the maps becomes a `seom:SOCConcept` with its notation and label, in its own concept scheme, and each occupation carries an explicit `seom:socMapping2020` / `seom:socMapping2010` edge. That makes the bridge from occupational standards to ONS labour-market and pay statistics a single SPARQL hop instead of a bespoke join.

3. **The graph is constraint-checked, not just serialised.** A SHACL shapes file states what a well-formed occupation, product and SOC concept must look like (every standard fully placed in the map, every progression edge pointing at a real occupation, every SOC reference carrying a notation). The graph is validated against it, and against SPARQL referential-integrity checks, on every build.

The whole pipeline is reproducible: `build_ontology.py` regenerates the graph from the snapshot, `validate_ontology.py` re-runs SHACL and the coverage report.

## Outcome

A single open graph of the national occupational map, published as Turtle and JSON-LD.

**Coverage.** 1,269 occupational standards; 15 routes, 35 pathways, 172 clusters; 278 SOC 2020 and 246 SOC 2010 concepts in the crosswalk; 1,313 technical education products across six product types (apprenticeship, HTQ, T Level, TQ, foundation apprenticeship, apprenticeship unit); 8 green themes with 231 occupation-to-theme links; and 2,717 progression edges forming a directed career-progression graph. Every one of the 1,269 standards is fully placed in the map (route, pathway, cluster and technical level all present); 1,079 (85%) carry a SOC 2020 mapping.

**Assurance.** The graph conforms to the SHACL shapes with zero violations across all 51,355 triples, and passes four referential-integrity checks (no occupation without a route, no progression edge to a non-occupation, no green-theme link to a non-theme, no SOC reference without a notation). See [ontology/coverage-report.md](ontology/coverage-report.md).

**Why it matters.** Once the maps are a graph with an explicit SOC crosswalk, questions that used to need custom code become single queries: the progression neighbourhood of any occupation; every standard that feeds a given SOC group; the apprenticeship and HTQ offer under any green theme; the occupations that sit on more than one route. It is also the operational companion to our open research on [skills and social mobility in England](https://github.com/fabio-rovai/open-ontologies/blob/main/case-studies/skills-mobility/case-study.md): PIAAC measures what adults can do, the occupational maps show where those skills lead, and SOC is the join between them.

## Reusable assets

- The SEOM vocabulary and the full instance graph (Turtle and JSON-LD), openly published.
- The SOC 2010 and 2020 crosswalk as SKOS concept schemes, reusable on its own to bridge occupational standards to ONS statistics.
- The SHACL shapes and the reproducible build and validation scripts, re-pointable to any future snapshot of the maps.

## Licence

The Skills England data is Crown copyright, published under the Open Government Licence v3.0. This work contains public sector information from Skills England used under that licence. It is an independent Tesseract Academy demonstration and is not endorsed by Skills England.
