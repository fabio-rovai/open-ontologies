---
license: cc-by-4.0
language:
  - en
pretty_name: MoDiP Plastics Knowledge Graph
tags:
  - collections-as-data
  - cultural-heritage
  - GLAM
  - CIDOC-CRM
  - SKOS
  - getty-aat
  - knowledge-graph
  - plastics
  - ontology
  - linked-data
size_categories:
  - 100K<n<1M
task_categories:
  - graph-ml
  - other
source_datasets:
  - Museum Data Service (MoDiP)
configs:
  - config_name: default
    data_files:
      - split: objects
        path: data/raw/modip_records.json.gz
---

# MoDiP Plastics Knowledge Graph

A standards-based knowledge graph built from the full open catalogue of the
**Museum of Design in Plastics** (MoDiP, Arts University Bournemouth) — the UK's
only accredited museum devoted to plastics in design — as a worked example of
turning a small museum's raw "collections as data" into something computable.

- **11,865** object records (the complete MoDiP set), retrieved from the
  [Museum Data Service](https://museumdata.uk/) under **CC BY 4.0**.
- **485,013-triple** CIDOC-CRM (Linked Art compatible) instance graph.
- **137-concept** SKOS materials taxonomy grounded in polymer science
  (thermoplastic / thermoset / elastomer / biopolymer), resolving **99.9%** of
  35,172 free-text material assertions and unifying abbreviation / full-name /
  trade-name synonyms (PP = polypropylene; Perspex → PMMA; Bakelite → phenol
  formaldehyde).
- **55** verified `skos:exactMatch` links to the **Getty Art & Architecture
  Thesaurus** (query-reconciled, exact-label only).
- **289** object-to-object variant / same-mould edges recovered from accession
  numbers written into free-text descriptions.
- Release gate: **SHACL `conforms=True`** + closed-world vocabulary check with
  **0** dangling concept references.

## Files

| Path | Content |
|---|---|
| `data/raw/modip_records.json.gz` | the raw MoDiP records as fetched (gzipped) |
| `ontology/materials.ttl` | SKOS materials taxonomy (137 concepts, AAT-aligned) |
| `ontology/processes.ttl`, `ontology/domains.ttl` | process & use-domain taxonomies |
| `build/modip-crm.ttl.gz` | the CIDOC-CRM instance graph (gzipped Turtle) |
| `build/dag_variants.ttl` / `.csv` | the variant graph |
| `docs/spectrum-crm-mapping.md` | the Spectrum → CIDOC-CRM crosswalk |
| `BUILD_REPORT.md` | exactly what was fetched, computed and could not be obtained |

## Standards & modelling

No bespoke class ontology is minted. Instances use **CIDOC-CRM** (ISO 21127);
vocabularies use **SKOS**; materials align to **Getty AAT**; source records are
**Spectrum**. Only concept and instance URIs are local, plus two clearly-marked
polymer-science facet properties (`thermalBehaviour`, `origin`). See the
[Spectrum→CRM mapping](docs/spectrum-crm-mapping.md).

## Licence & attribution

- **Source records** © Museum of Design in Plastics / Arts University Bournemouth,
  published via the Museum Data Service under
  [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). Per-record rights are
  retained in the graph (`dct:license`, `dct:rightsHolder`).
- **This transformation** (taxonomies, mappings, graph) is released under CC BY 4.0.
- **Getty AAT** alignments © J. Paul Getty Trust (ODC-By 1.0).
- Endorsed by neither MoDiP nor the Museum Data Service.

## Provenance & reproduction

Full pipeline and honesty log in the companion repository:
[github.com/fabio-rovai/open-ontologies → case-studies/modip-plastics-kg](https://github.com/fabio-rovai/open-ontologies/tree/main/case-studies/modip-plastics-kg).
`run_all.sh` regenerates and revalidates the whole graph from the committed data.

Built by [The Tesseract Academy](https://tesseract.academy). If you run or fund a
collection and want your catalogue made this queryable:
**fabio@thetesseractacademy.com**.
