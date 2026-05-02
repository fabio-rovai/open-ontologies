# External Integrations — Verified Test Results

Companion to [`external-integrations.md`](external-integrations.md). The earlier doc lists candidates from web search; this doc records what I actually **ran and tested** vs what I only **inspected**.

Tested: 2026-05-01, on macOS 14, Python 3.11, Java 25 (Homebrew OpenJDK).

## Status table

| Tool | Verdict | Evidence |
|---|---|---|
| **Apache Jena Fuseki 5.1.0** (plain SPARQL) | ✅ **Verified working** | Booted, loaded NAPH ontology (198 triples) + sample data (313 triples), ran SPARQL tier-distribution query — returned correct counts (Aspirational=3, Enhanced=4, Baseline=3). |
| **`tiny-iiif`** (rsimon) | ✅ Recently maintained, looks credible | Repo cloned. Last commit 2026-03-24 (v0.2.0). Docker-compose based. Did not run-test (Docker pull cost). |
| **`stitch-aerial-photos`** (luna983) | ✅ Real, published research, but Linux-only | Repo cloned. Has Zenodo DOI, Docker image. SURF requires custom OpenCV build (Linux x86_64 wheel only). Did not run-test on macOS. |
| **`pipeline/qid-verify.py`** (this case study) | ✅ Verified working | Tested earlier on sample data: 6 QIDs resolved correctly. Uses curl with explicit User-Agent. |
| **Apache Jena Fuseki + GeoSPARQL** | ⚠️ Requires extra setup | The standard Fuseki download does NOT bundle GeoSPARQL. Need separate `jena-fuseki-geosparql` module from Maven. Did not test the geosparql variant. |
| **`qwikidata` 0.4.2** | ❌ **Broken against current Wikidata** | Installed via pip. Both `linked_data_interface` and `sparql` modules fail with 403 because they don't set User-Agent header. Wikidata enforces UA policy since ~2025. Library appears unmaintained (last release 2024). |
| **Loris IIIF server** (loris-imageserver/loris) | ❌ **Unmaintained** | Git repo last commit 2021-06-23 (5 years). Dependencies pinned to 2018-2020 versions. `pip install` fails with `legacy-install-failure` on modern Python. **Do not adopt.** |
| **`adehecq/usgs_explorer`** | ⏭ Not run-tested | Requires USGS ERS account registration. Repo inspection only — looks clean and maintained. |
| **Cantaloupe (Java IIIF server)** | ⏭ Not run-tested | Heavy install (Java + multi-GB Docker image). Skipped due to time + resource cost. Production-grade based on documentation; no runtime verification. |
| **`allenai/satlaspretrain_models`** | ⏭ Not run-tested | Needs PyTorch + GPU + model download (~GB). Skipped due to resource cost. Published at ICCV 2023; assumed credible but unverified. |
| **`dbcls/sparql-proxy`** | ⏭ Not run-tested | Skipped — Fuseki test already covers the federation hub need. |

## Detailed test commands (reproducible)

### Apache Jena Fuseki — verified working

```bash
# Setup
brew install openjdk@25
export JAVA_HOME=/opt/homebrew/opt/openjdk@25
export PATH=$JAVA_HOME/bin:$PATH
curl -LO https://archive.apache.org/dist/jena/binaries/apache-jena-fuseki-5.1.0.zip
unzip apache-jena-fuseki-5.1.0.zip
cd apache-jena-fuseki-5.1.0

# Start with in-memory dataset, allow updates
./fuseki-server --mem --update /naph &

# Load NAPH ontology
curl -X POST -H "Content-Type: text/turtle" \
  --data-binary @ontology/naph-core.ttl \
  "http://localhost:3030/naph/data?default"
# → {"count": 198, "tripleCount": 198}

# Load sample data
curl -X POST -H "Content-Type: text/turtle" \
  --data-binary @data/sample-photographs.ttl \
  "http://localhost:3030/naph/data?default"
# → {"count": 313, "tripleCount": 313}

# Tier distribution query
curl -G --data-urlencode "query=PREFIX naph: <https://w3id.org/naph/ontology#>
  SELECT ?tier (COUNT(?p) AS ?n)
  WHERE { ?p naph:compliesWithTier ?tier }
  GROUP BY ?tier" \
  -H "Accept: application/sparql-results+json" \
  "http://localhost:3030/naph/sparql"
# → Aspirational=3, Enhanced=4, Baseline=3 ✓
```

This is a **drop-in replacement** for Oxigraph for any institution that wants standard SPARQL with persistence. Add the `jena-fuseki-geosparql` jar to also handle the spatial queries Oxigraph can't.

### qwikidata — broken

```bash
pip install qwikidata
python3 -c "
from qwikidata.linked_data_interface import get_entity_dict_from_api
data = get_entity_dict_from_api('Q212065')
"
# → LdiResponseNotOk: response.status_code: 403
#    "Please set a user-agent and respect our robot policy"
```

The library issues plain `requests.get()` without a User-Agent header. Wikidata's UA enforcement (since ~2025) blocks all such requests.

**Workaround:** monkey-patch the requests session, or use `pipeline/qid-verify.py` instead (which sets UA explicitly via curl).

### Loris — unmaintained

```bash
pip install loris
# → error: legacy-install-failure
```

`pip install` fails on Python 3.11+ because Loris uses the deprecated `setup.py install` flow with pinned 2018-vintage dependencies. The git repo's last commit is 2021-06-23.

For Python-based IIIF, Loris cannot be recommended. Use Cantaloupe (Java) or tiny-iiif (Node/Docker) instead.

### tiny-iiif — credible without run-test

Repo state inspected:
- Last commit 2026-03-24 (v0.2.0) — actively maintained
- Docker-compose with Cantaloupe backend
- Clear README, screenshots, hosted demo at tiny-iiif.org
- License: not inspected (assumed open per GitHub display)

Recommended for institutions wanting a quick IIIF setup. Run-test would require docker pull + 5min config; skipped here.

### stitch-aerial-photos — credible without run-test

Repo state inspected:
- Has Zenodo DOI (proper research artefact)
- README references academic publication
- Custom OpenCV wheel for SURF: Linux x86_64 only (excludes macOS arm64 dev)
- Docker image available: `luna983/stitch-aerial-photos:latest`

Credible; run-test would need Docker pull (~GB). Skipped.

## What this changes about my earlier recommendations

The earlier [`external-integrations.md`](external-integrations.md) lists 7 tools as "use this." After actual testing:

| Was recommended as | Now verified as |
|---|---|
| `qwikidata` — Python pkg, no auth | ❌ Broken; use `pipeline/qid-verify.py` instead |
| `Loris` (Python IIIF) | ❌ Abandoned; use Cantaloupe or tiny-iiif |
| **Apache Jena Fuseki** | ✅ Verified working — actually tested with NAPH data |
| Other 5 tools | Repo-inspected, plausible but not run-tested |

## Honest engineering note

Recommending tools without testing is a category of red-team failure. The first sweep of [`external-integrations.md`](external-integrations.md) listed `qwikidata` and `Loris` as if they worked; both don't (in 2026). Adopters who follow that doc would waste hours debugging.

Going forward, every "recommended" tool in NAPH documentation should either:

1. Be marked as "verified" with a runnable test command
2. Be marked as "candidate, not tested" with explicit warning

Test commands for verified tools live in this file and are reproducible.

## Cross-references

- [`external-integrations.md`](external-integrations.md) — the broader landscape view (now needs caveats)
- [`pipeline/qid-verify.py`](../../pipeline/qid-verify.py) — verified working
- [`pipeline/oxigraph_server.py`](../../pipeline/oxigraph_server.py) — Oxigraph-based local dev; substitute Fuseki for production GeoSPARQL needs
