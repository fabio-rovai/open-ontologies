# External Integrations — what to use instead of building from scratch

For each NAPH gap previously marked "stub" or "skipped (external API)", this document maps to the existing open-source implementation worth adopting rather than rebuilding. Status as of 2026-05.

> ⚠️ **Read [`external-integrations-tested.md`](external-integrations-tested.md) first.** This document lists candidates surveyed via search; the tested doc records what was actually run. Notably:
> - `qwikidata` 0.4.2 is **broken** against current Wikidata (User-Agent enforcement)
> - `Loris` IIIF server is **unmaintained** since 2021
> - `Apache Jena Fuseki` is **verified working** with NAPH data

## 1. USGS Earth Explorer scraping (declassified satellite imagery)

**Don't build from scratch.** Use one of the mature Python clients:

| Project | Notes |
|---|---|
| [`adehecq/usgs_explorer`](https://github.com/adehecq/usgs_explorer) (`usgsxplore`) | **Recommended.** Explicitly supports declassified imagery datasets (CORONA, KH-9). CLI + Python API. |
| [`Fergui/m2m-api`](https://github.com/Fergui/m2m-api) | Clean Python interface to login / dataset-search / scene-search endpoints. |
| [`castuofa/usgs-m2m-api`](https://github.com/castuofa/usgs-m2m-api) | Object-oriented wrapper. |
| [`yannforget/landsatxplore`](https://github.com/yannforget/landsatxplore) | Landsat-focused but uses M2M API. |

**NAPH integration approach:**

```python
# pseudo-code — wrap usgsxplore in a NAPH adapter
from usgsxplore.api import USGSExplorerAPI

api = USGSExplorerAPI(username=os.environ["USGS_USER"], token=os.environ["USGS_TOKEN"])
scenes = api.search(dataset="declass_3", bbox=(-3.5, 55.5, -3.0, 56.0))

for scene in scenes:
    # Map scene metadata → NAPH SatelliteAcquisition + AerialPhotograph
    # See pipeline/scrapers/usgs_earthexplorer.py — fill in the TODO sections
```

**Account required:** Free USGS ERS account at <https://ers.cr.usgs.gov/register> + access permission for declassified-imagery dataset.

**Estimated integration effort:** ~1 day to wrap one of these libraries with NAPH-emitting Turtle output.

---

## 2. IIIF Image API server (for serving image content)

**Don't build from scratch.** Multiple production-quality servers exist:

| Project | Language | Notes |
|---|---|---|
| [Cantaloupe](https://cantaloupe-project.github.io/) | Java | **Most widely deployed.** Full IIIF Image API 3.0 support. Used by major heritage institutions. |
| [`MITLibraries/docker-cantaloupe`](https://github.com/MITLibraries/docker-cantaloupe) | Docker | Production-tested Docker wrapper for Cantaloupe. |
| [`UCLALibrary/docker-cantaloupe`](https://github.com/UCLALibrary/docker-cantaloupe) | Docker | Alternative Docker wrapper. |
| [`ruven/iipsrv`](https://github.com/ruven/iipsrv) | C++ | High-performance IIPImage server. |
| [Loris](https://github.com/loris-imageserver/loris) | Python | Pure-Python option for institutions preferring Python. |
| [SIPI](https://github.com/dasch-swiss/sipi) | C++ | Used by DaSCH, supports IIIF + JPEG2000. |
| [`rsimon/tiny-iiif`](https://github.com/rsimon/tiny-iiif) | Mixed | Drag-and-drop folder → IIIF in a minute. Demo/dev only. |

**NAPH integration approach:**

1. Run Cantaloupe via Docker on your server
2. Configure source resolver to point at your TIFF preservation masters
3. Update [`pipeline/iiif-bridge.py`](../../pipeline/iiif-bridge.py) so the `service.id` URLs point at your Cantaloupe instance instead of placeholder URLs
4. Manifests now resolve to real images in Mirador / Universal Viewer

**Estimated integration effort:** ~1-2 days for institutional setup + testing.

---

## 3. Wikidata QID verification

**Don't build from scratch.** Several pure-Python libraries:

| Project | Notes |
|---|---|
| [`kensho-technologies/qwikidata`](https://github.com/kensho-technologies/qwikidata) | Pure-Python entity classes. No auth required for read-only operations. |
| [`SuLab/WikidataIntegrator`](https://github.com/SuLab/WikidataIntegrator) | Full read/write library with built-in conflict checks. Used in WikiCite. |
| [`suhasshrinivasan/wikidata-toolkit`](https://github.com/suhasshrinivasan/wikidata-toolkit) | High-level query/manipulation methods. |

**NAPH integration delivered:** [`pipeline/qid-verify.py`](../../pipeline/qid-verify.py) — implemented in this case study using direct SPARQL (no auth, no library dependency). Verifies that Wikidata QIDs in a NAPH dataset point to entities of the expected type and have not been deprecated.

---

## 4. Vision-language model classification (Aspirational tier subject classification)

**Don't build from scratch — partial.** Two distinct paths depending on whether you want general-purpose VLMs or remote-sensing-specific models.

### Remote-sensing-specific (Recommended for aerial heritage)

| Project | Notes |
|---|---|
| [`allenai/satlaspretrain_models`](https://github.com/allenai/satlaspretrain_models) | **Recommended for satellite imagery.** Pretrained models for remote sensing image understanding. ICCV 2023. |
| [`satellite-image-deep-learning/techniques`](https://github.com/satellite-image-deep-learning/techniques) | Curated index of techniques + datasets + tools. |
| [aitlas-arena](https://github.com/biasvariancelabs/aitlas-arena) | Benchmark framework for remote sensing classification. |
| [`luna983/stitch-aerial-photos`](https://github.com/luna983/stitch-aerial-photos) | Aerial photo co-registration (SURF + RANSAC + PyTorch). Useful for orthorectification of historic aerial photos. |

### General-purpose VLMs (require API access)

- Claude 3.5 Sonnet Vision (Anthropic) — paid API
- GPT-4 Vision (OpenAI) — paid API
- Gemini Vision (Google) — paid API
- LLaVA (open-source, runs locally on GPU) — free but needs hardware

**NAPH integration approach:**

The remote-sensing-specific models are pretrained for satellite/aerial imagery and don't need API calls. They're heavier to integrate (PyTorch + GPU recommended) but produce better results for aerial classification than general-purpose VLMs.

For institutions with GPU infrastructure: integrate satlaspretrain. For institutions without GPU but with API budget: use Claude Vision via the spec at [`vlm-pipeline-spec.md`](vlm-pipeline-spec.md).

**Estimated integration effort:** ~3-5 days for satlaspretrain integration with NAPH provenance recording.

---

## 5. SPARQL Federation Hub

**Don't build from scratch.** Multiple federation engines exist:

| Project | Notes |
|---|---|
| **FedX** (in [RDF4J](https://rdf4j.org/) and [GraphDB](https://graphdb.ontotext.com/)) | **Recommended for production.** Transparent federation across multiple SPARQL endpoints. Used in major linked-data projects. |
| [Apache Jena Fuseki](https://jena.apache.org/documentation/fuseki2/) with FedX | Open-source production option. |
| [`dbcls/sparql-proxy`](https://github.com/dbcls/sparql-proxy) | Adds caching, job control, query safety to any SPARQL endpoint. |
| [`zazuko/sparql-proxy`](https://github.com/zazuko/sparql-proxy) | Lightweight middleware proxy. |
| [`AKSW/HiBISCuS`](https://github.com/AKSW/HiBISCuS) | Academic-grade hypergraph-based source selection. |
| [`dice-group/CostFed`](https://github.com/dice-group/CostFed) | Cost-based query optimisation for federation. |
| [Comunica](https://github.com/comunica/comunica) | JS-based; runs in browser or Node. Used by [Linked Data Fragments](https://linkeddatafragments.org/). |

**NAPH federation approach:**

- For a national-archive-scale federation hub, deploy Apache Jena Fuseki with FedX
- Each adopting institution registers their endpoint in the hub's federation config
- A single query at the hub fans out to all participating endpoints
- See [`deliverables/06-knowledge-transfer/federation-playbook/README.md`](federation-playbook/README.md)

**Estimated infrastructure effort:** ~1 week for hub deployment + 1 day per participating institution.

---

## 6. NCAP Air Photo Finder scraping

**No existing implementation found.** The Angular SPA + `robots.txt` AI-bot blocking + lack of public API means no community implementation exists.

**Recommended path:** institutional partnership rather than scraping. Contact `ncap@hes.scot`.

If institutional access can be arranged, the integration is:

1. Get authenticated API access (institutional MOU)
2. Replace [`pipeline/scrapers/ncap_airphotofinder.py`](../../pipeline/scrapers/ncap_airphotofinder.py) stub with API client
3. Map response JSON → NAPH Turtle following the existing scraper pattern

If institutional access is impossible and Playwright-based scraping is acceptable to NCAP/HES under written permission:

- [`microsoft/playwright-python`](https://github.com/microsoft/playwright) for browser automation
- Capture network requests during page navigation
- Reverse-engineer the internal data API
- Apply rate limiting (1 request per 5 seconds minimum)
- Stop immediately if blocked

**Risk:** scraping without explicit permission may violate terms of service. Always preferred path is partnership.

---

## 7. Aerial photo georeferencing

**Don't build from scratch.** [`luna983/stitch-aerial-photos`](https://github.com/luna983/stitch-aerial-photos) handles co-registration of historic aerial photos using SURF + RANSAC + PyTorch autograd. Specifically designed for the historic-aerial use case.

**NAPH integration:** wrap the output as `prov:Activity` provenance metadata. Each georeferenced photo records the registration event with confidence and reference frames.

---

## Summary table

| Item | Status | Use this |
|---|---|---|
| USGS Earth Explorer scraper | Existing | [`adehecq/usgs_explorer`](https://github.com/adehecq/usgs_explorer) |
| IIIF Image API server | Existing | [Cantaloupe](https://cantaloupe-project.github.io/) (or Loris for Python) |
| Wikidata QID verification | Existing + integrated | [`qid-verify.py`](../../pipeline/qid-verify.py) (using SPARQL directly) |
| Aerial VLM classification | Existing | [`allenai/satlaspretrain_models`](https://github.com/allenai/satlaspretrain_models) (no API) |
| SPARQL federation hub | Existing | [Apache Jena Fuseki](https://jena.apache.org/documentation/fuseki2/) + FedX |
| NCAP scraper | No implementation | Institutional partnership required |
| Aerial photo georeferencing | Existing | [`luna983/stitch-aerial-photos`](https://github.com/luna983/stitch-aerial-photos) |

For each "Existing" entry, the engineering effort is integration (1-5 days) rather than building from scratch (weeks-months).

## Cross-references

- [VLM pipeline spec](vlm-pipeline-spec.md) — when external API VLMs are acceptable
- [Federation playbook](federation-playbook/README.md) — how to set up multi-institution federation
- [QID verifier](../../pipeline/qid-verify.py) — actual integration in this case study
