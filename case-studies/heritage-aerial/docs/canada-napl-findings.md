# Empirical Findings — Canada's NAPL Open Data Is Already at Baseline

*A companion to [`empirical-api-findings.md`](empirical-api-findings.md) (United
Kingdom / NCAP). Measured against the open-data subset of Canada's **National Air
Photo Library** (NAPL, Natural Resources Canada), read from the Government of
Canada open-data portal (open.canada.ca) CKAN API on 2 July 2026. Metadata only;
no image binaries; read-only public endpoints; no authentication required. Data
released under the Open Government Licence - Canada. See
[`pipeline/scrapers/napl_opencanada.py`](../pipeline/scrapers/napl_opencanada.py).*

This is the second national collection tested against NAPH, after NCAP (UK). The
point of running it was to answer a direct question: **is the standard actually
source-agnostic, or is it quietly shaped around one archive?** The answer is that
the ontology, SHACL shapes and RiC-O × STAC crosswalk are unchanged — only the
thin harvester adapter differs. A US adapter pattern is documented at
[`pipeline/scrapers/usgs_earthexplorer.py`](../pipeline/scrapers/usgs_earthexplorer.py).

![NAPL Temporal Series regions across Canada, harvested live and lifted to NAPH
Baseline](../assets/real-demo-canada.png)

*8 NAPL Temporal Series regions (red) harvested live from the open Government of
Canada CKAN API and lifted to NAPH Baseline. The amber cluster is the US
WHAIFinder sample; teal is NCAP (UK) — same standard, three collections.*

## Headline

> Where NCAP was missing exactly one Baseline field (machine-readable rights,
> 0/300), every NAPL open-data record carries **all six**. Canada's open aerial
> collection is *already at NAPH Baseline* on every required field, including the
> one the UK collection lacked. The footprints are even delivered in **native
> WGS84** — no reprojection needed. The residual work is URI minting and format
> publication only. **40 dated orthophoto-mosaic slices across 8 regions
> (1932–2004) validate against the standard with 0 SHACL violations.**

## What we measured (8 NAPL Temporal Series datasets, 40 dated mosaics)

| Baseline field | Present | Notes |
|---|---:|---|
| Machine-readable footprint (CKAN `spatial`) | **100%** | GeoJSON `Polygon` in **EPSG:4326 (WGS84)** — already geographic, no reprojection (contrast NCAP's EPSG:3857). |
| Capture date (`gYear` per mosaic) | **100%** | Every per-year orthophoto-mosaic resource is year-stamped; maps to `xsd:gYear` under the same precision policy (ADR-0009). |
| Stable identifier (dataset UUID) | **100%** | e.g. `03ccfb5c-a06e-43e3-80fd-09d4f8f69703`, resolvable at open.canada.ca. |
| Machine-readable **rights** (`ca-ogl-lgo`) | **100%** | **The field NCAP lacked.** Open Government Licence - Canada, with a resolvable licence URI on every record. |
| Sortie / campaign linkage | **100%** | One `naph:Sortie` per regional temporal series. |
| Collection linkage | **100%** | All items linked to the NAPL open-data collection. |

Regions in the sample: Tuktoyaktuk (NWT), Markham (ON), Halifax (NS), Regina
(SK), Ring of Fire (ON), Victoria (BC), Ottawa River (ON/QC), Salish region (BC).
The mosaics span **1932–2004** — genuine cross-Canada, multi-decade reach in one
un-curated pull.

## The cross-national contrast is the interesting result

| | **NCAP (UK)** | **NAPL open data (Canada)** |
|---|---|---|
| Footprint | 100%, but **EPSG:3857** (needs reprojection) | 100%, **native WGS84** (no reprojection) |
| Date | 100% ISO-8601, day/year precision | 100%, year precision (`gYear`) |
| Identifier | 100% (`UNI`, not yet a URI) | 100% (dataset UUID, resolvable) |
| **Machine-readable rights** | **0% — the genuine gap** | **100% — already present (OGL-Canada)** |
| Granularity | **frame-level** (individual recon frames) | **collection-level** (regional orthophoto mosaics) |

The two collections are missing *opposite* things. The UK has frame-level
granularity but no machine-readable rights; Canada's open subset has machine-
readable rights and native WGS84 but publishes mosaics rather than individual
frames. This is exactly the kind of asymmetry a shared standard exists to
normalise: the same six-field Baseline test applied to both makes the gap in each
precise and automatable, instead of leaving each archive to describe itself in its
own vocabulary.

## Granularity, stated honestly

These NAPL open-data records are **regional orthophoto mosaics**, one per
acquisition year, not the ~6 million original single frames (those live behind
NRCan's EODMS service and its per-dataset STAC API — the natural Enhanced-tier
harvest). Each mosaic is modelled as a `naph:AerialPhotograph` at Baseline,
carrying the region footprint and that year's `gYear` date, and is labelled in the
data as a collection-level surrogate rather than an original frame. That is the
honest scope of this pull: it proves the standard travels and that the substrate
is present, not that every Canadian frame is already online.

## Reproduce it

```bash
# Harvest the live open-data sample (no auth, metadata only, rate-limited)
python3 pipeline/scrapers/napl_opencanada.py \
    --raw-out pipeline/real-napl-raw.json > data/real-napl-sample.ttl

# Validate real data against the standard
open-ontologies validate data/real-napl-sample.ttl
printf 'clear\nload ontology/naph-core.ttl\nload data/real-napl-sample.ttl\nshacl ontology/naph-shapes.ttl\n' \
    | open-ontologies batch          # -> conforms: true, violation_count: 0
```

Footprints as GeoJSON: [`reports/real-napl-footprints.geojson`](../reports/real-napl-footprints.geojson).

## Provenance and good-faith note

This harvest touches only the public CKAN API of the Government of Canada
open-data portal. It fetches dataset metadata only — no image binaries, no EODMS
ordering endpoints. It is rate-limited and identifies itself in the User-Agent.
The underlying data is published by Natural Resources Canada under the Open
Government Licence - Canada. This is a good-faith interoperability demonstration
of the NAPH standard's portability, in support of open aerial-heritage
computation-readiness.
