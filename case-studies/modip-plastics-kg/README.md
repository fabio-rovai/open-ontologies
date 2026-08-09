# MoDiP Plastics Knowledge Graph

**A worked example of turning a small museum's raw collection records into a
standards-based knowledge graph — taxonomy, ontology mapping, and a variant
graph — using only open standards and the museum's own open data.**

The [Museum of Design in Plastics](https://www.modip.ac.uk/) (MoDiP, Arts
University Bournemouth) publishes **11,865** object records through the
[Museum Data Service](https://museumdata.uk/) under CC BY 4.0. Like most museum
data, they are published as *raw material*: flat, free-text, un-linked. The same
polymer appears as `PP`, `polypropylene`, `polythene` and `Perspex - trade name`
as unrelated strings; relationships between objects sit unread inside description
prose. This repository shows, end to end, what it takes to make that data
computable — and does it against the real records, not a toy sample.

Nothing here is bespoke where a standard exists. The instance graph is
**CIDOC-CRM** (ISO 21127, Linked Art compatible); the vocabularies are **SKOS**,
aligned by query to **Getty AAT**; the source records are Spectrum. The only URIs
minted locally are the concept and instance identifiers.

## What's in the box

| Artifact | File | Content |
|---|---|---|
| Raw-data profile | [`data/PROFILE.md`](data/PROFILE.md) | the "before" state, quantified |
| Materials taxonomy | [`ontology/materials.ttl`](ontology/materials.ttl) | 137 SKOS concepts (thermoplastic / thermoset / elastomer / biopolymer), abbreviations + trade names as `altLabel`, 55 Getty AAT `exactMatch` |
| Process taxonomy | [`ontology/processes.ttl`](ontology/processes.ttl) | 65 concepts (moulding, extrusion, forming, machining, textile processes) |
| Domain taxonomy | [`ontology/domains.ttl`](ontology/domains.ttl) | 29 concepts (MoDiP's own use-domain facet, made hierarchical) |
| Instance graph | [`build/modip-crm.ttl`](build/modip-crm.ttl) | 485,013 triples, 11,865 `crm:E22_Human-Made_Object`s |
| Variant DAG | [`build/dag_variants.ttl`](build/dag_variants.ttl) · [`.csv`](build/dag_variants.csv) | 289 same-mould / cross-reference edges recovered from description prose |
| Spectrum→CRM crosswalk | [`docs/spectrum-crm-mapping.md`](docs/spectrum-crm-mapping.md) | the reusable mapping, field by field |
| Honesty log | [`BUILD_REPORT.md`](BUILD_REPORT.md) | exactly what was fetched, computed, and could not be obtained |

## Headline results

- **99.9%** of 35,172 free-text material assertions resolved to a science-grounded
  concept (476 raw strings → 137 concepts).
- **100%** of technique and use-domain assertions resolved.
- **289** object-to-object variant relationships recovered from prose that no
  keyword search could traverse.
- **SHACL: `conforms=True`**, and a closed-world vocabulary check with **0**
  dangling concept references (`src/validate.py` is the release gate).

## The synonymy fix, concretely

Before, these were four unrelated strings; after, one concept a search actually
resolves:

```turtle
mat:pvc a skos:Concept, crm:E57_Material ;
    skos:prefLabel "polyvinyl chloride"@en ;
    skos:altLabel "PVC"@en, "plasticised polyvinyl chloride"@en, "uPVC"@en ;
    mod:thermalBehaviour "thermoplastic" ; mod:origin "synthetic" ;
    skos:exactMatch aat:300014513 .
```
(`aat:300014513` is the verified Getty AAT concept for polyvinyl chloride; the
`pmma` concept carries the abbreviations and trade names too, but is left without
an AAT link because no exact-label AAT match was found — see `BUILD_REPORT.md`.)

## Reproduce

```bash
pip install -r requirements.txt
# the raw records are committed, so you can skip the fetch:
python3 src/build_taxonomies.py
python3 src/build_graph.py
python3 src/validate.py        # must print RESULT: PASS
```
To re-fetch from source, get a free MDS API token from the "Get an API token"
form on any [MDS object-search page](https://museumdata.uk/object-search/) and run
`MDS_TOKEN="…" python3 src/fetch_modip.py`.

## Licence & attribution

- **Source records** © Museum of Design in Plastics / Arts University Bournemouth,
  published via the Museum Data Service under
  [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). Per-record rights are
  retained in the graph (`dct:license`, `dct:rightsHolder`).
- **This transformation** (taxonomies, mappings, code) is released under
  [CC BY 4.0](LICENSE) (data/vocabularies) and MIT (code) — see [`LICENSE`](LICENSE).
- **Getty AAT** alignments © J. Paul Getty Trust, under
  [ODC-By 1.0](https://opendatacommons.org/licenses/by/1-0/).

---

### Who made this, and why

Built by **[The Tesseract Academy](https://tesseract.academy)** as an open
demonstration for the GLAM (galleries, libraries, archives, museums) sector: this
is the pipeline that takes *any* museum's Spectrum/CSV export to a validated,
standards-based knowledge graph. If you run a collection — or fund one — and want
your data made this queryable (or want the method applied to your own catalogue),
we'd genuinely like to hear from you: **fabio@thetesseractacademy.com**.

This artifact is part of the [open-ontologies](https://github.com/fabio-rovai/open-ontologies)
programme of computation-ready reference vocabularies. The dataset is also
mirrored on Hugging Face:
[**datasets/fabsssss/modip-plastics-knowledge-graph**](https://huggingface.co/datasets/fabsssss/modip-plastics-knowledge-graph).
