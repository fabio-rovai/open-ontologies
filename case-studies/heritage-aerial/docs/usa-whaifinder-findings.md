# Empirical Findings — Reaching Baseline for a US Collection Without the Token-Gated API

*Third national-scale collection tested against NAPH, after NCAP (UK) and NAPL
(Canada). Measured against the **Wisconsin Historic Aerial Imagery Finder
(WHAIFinder)** — ~201,000 frame-level records held by the UW-Madison Robinson Map
Library and the Wisconsin State Cartographer's Office, drawn largely from
public-domain USDA survey photography — read from its public **ArcGIS
FeatureServer** on 2 July 2026. Metadata only; read-only; no authentication. See
[`pipeline/scrapers/whaifinder_arcgis.py`](../pipeline/scrapers/whaifinder_arcgis.py).*

![225 real WHAIFinder frames across Wisconsin, footprints reconstructed from
centerpoint and scale](../assets/real-demo-usa.png)

*225 real Wisconsin Historic Aerial Imagery frames (1937–1967, public-domain USDA),
harvested live from the open ArcGIS FeatureServer. Each footprint reconstructed
closed-form from the published centerpoint plus map scale.*

## Why not USGS

The obvious US target is the USGS EROS "Aerial Photo Single Frames" archive (6.5M
frames). Its Machine-to-Machine API is real but **token-gated** — it needs a free
USGS account and per-dataset access grants (documented, unused, in
[`pipeline/scrapers/usgs_earthexplorer.py`](../pipeline/scrapers/usgs_earthexplorer.py)).
Rather than gate the demonstration behind credentials, this adapter uses an
**openly queryable** US holding instead: US aerial indexes are widely published as
ArcGIS Feature Services, whose REST/JSON query API needs no auth. WHAIFinder is one
such service; UCSB's national FrameFinder and many state indexes share the pattern.

## Headline

> WHAIFinder publishes, for every frame, a **stable UUID**, a full **ISO date**, a
> **map scale**, a **public-domain rights** provenance (USDA/USGS federal work) —
> and a **point centerpoint** rather than a polygon footprint. The Baseline
> footprint is a **closed-form reconstruction** from centerpoint + scale (standard
> 9x9-inch frame): ground side = 0.2286 m x scale. **225 real frames (1937–1967)
> reconstructed and lifted to Baseline validate at 0 SHACL violations**, against
> the unchanged ontology, shapes and crosswalk.

## What we measured (n = 225, all with a usable map scale)

| Baseline field | Present | Notes |
|---|---:|---|
| Stable identifier (`uuid`) | **100%** | Plus roll/frame and a `collection_identifier`. |
| Capture date (`acquisition_date`) | **100%** | Full `YYYY-MM-DD` (`xsd:date`); all 225 day-precision in this pull. |
| Geometry | **100% — but a *centerpoint*** | `esriGeometryPoint` in WGS84, not a polygon. **This is the US-specific gap.** |
| Map scale (`map_scale_denom`) | **100%** | e.g. 1:20000 — the key that makes the footprint reconstructable. |
| Machine-readable **rights** | **100%** | USDA/USGS survey photography is US-Government work → public domain. |
| Sortie / collection linkage | **100%** | One `naph:Sortie` per roll; all frames linked to the WHAIFinder collection. |

Provenance in the sample: **222/225 USDA**, 3 USGS — all federal, public domain.
Also present for the Enhanced tier: scan resolution, ground resolution, exposure
type, and direct TIFF/JPEG download URLs.

## The reconstruction transform

The US index gives a centerpoint, not an area — so, unlike NCAP (polygon in the
wrong CRS) or NAPL (native-WGS84 polygon), the Baseline geometry has to be
*derived*. It is still a closed-form transform, not a guess:

- Standard US aerial survey frame = **9 x 9 inches = 0.2286 m** film format.
- Ground side length = `0.2286 m x map_scale_denom` (e.g. at 1:20000 → 4572 m).
- Build an axis-aligned square centred on the published centerpoint, in WGS84.

This is a **nominal** footprint: it ignores terrain relief and camera tilt, which
is exactly what the Enhanced tier refines using altitude / ground-sample-distance.
At Baseline it is a faithful, automatable reconstruction of where the frame looks —
the substrate (centerpoint + scale) was already in the data.

## Three collections, three different transforms — same standard

![Three national aerial collections — NCAP (UK), NAPL (Canada), WHAIFinder (USA) —
on one map, all lifted to the same NAPH Baseline](../assets/real-demo-three-country.png)

*NCAP (teal, UK), NAPL (red, Canada) and WHAIFinder (amber, USA), harvested live
from public endpoints and lifted to the same NAPH Baseline with 0 SHACL violations.
Same ontology, shapes and crosswalk throughout; only the harvester differs.*

| | **NCAP (UK)** | **NAPL (Canada)** | **WHAIFinder (USA)** |
|---|---|---|---|
| Geometry | polygon, **EPSG:3857** | polygon, **native WGS84** | **point centerpoint** |
| Transform to Baseline geometry | **reproject** | **none (already there)** | **reconstruct from scale** |
| Date | ISO, day/year | year (`gYear`) | ISO day (`xsd:date`) |
| Identifier | `UNI` (→ URI) | dataset UUID | frame `uuid` |
| Machine-readable rights | **0% (the gap)** | **100% (OGL)** | **100% (public domain)** |
| Granularity | frame-level | collection (mosaic) | frame-level |
| Real frames validated | 292 | 40 | 225 |

The point of running three national collections was to see whether NAPH is
genuinely source-agnostic or quietly shaped around NCAP. It survived the test:
each archive was missing a *different* Baseline piece — rights (UK), frame
granularity (Canada), footprint geometry (US) — and each gap was closed by a
*different* single automatable transform, with **no change to the ontology, SHACL
shapes or RiC-O × STAC crosswalk**. That is the strongest evidence so far that the
standard is a real interoperability layer and not a one-archive artefact.

## Reproduce it

```bash
python3 pipeline/scrapers/whaifinder_arcgis.py --limit 300 \
    --raw-out pipeline/real-whai-raw.json > data/real-whai-sample.ttl

open-ontologies validate data/real-whai-sample.ttl
printf 'clear\nload ontology/naph-core.ttl\nload data/real-whai-sample.ttl\nshacl ontology/naph-shapes.ttl\n' \
    | open-ontologies batch          # -> conforms: true, violation_count: 0
```

Reconstructed footprints as GeoJSON:
[`reports/real-whai-footprints.geojson`](../reports/real-whai-footprints.geojson).

## Provenance and good-faith note

This harvest touches only the public ArcGIS FeatureServer query endpoint of the
Wisconsin Historic Aerial Imagery service. It fetches attribute + geometry
metadata only — no image binaries. It is rate-limited and identifies itself in the
User-Agent. The underlying photography is public-domain US-Government work. This is
a good-faith interoperability demonstration of the NAPH standard's portability.
