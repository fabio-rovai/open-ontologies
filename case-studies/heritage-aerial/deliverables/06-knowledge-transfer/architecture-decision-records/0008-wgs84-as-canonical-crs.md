# ADR-0008: WGS84 as canonical CRS for geographic footprints

**Status:** Accepted
**Date:** 2026-04-30
**Decider:** Editorial team

## Context

Aerial photography heritage data has been captured and processed using many coordinate reference systems (CRS):

- **WGS84 (EPSG:4326)** — global GPS-default CRS
- **OSGB36 (EPSG:27700)** — UK Ordnance Survey
- **NAD83 (EPSG:4269)** — US national datum
- **ETRS89** — European Terrestrial Reference System
- Local national datums and projections

A standard must choose how to express geographic footprints. Options:

(a) Mandate a single CRS — typically WGS84 (simplest)
(b) Allow any CRS, but require explicit declaration
(c) Mandate WGS84 for federation, allow native CRS as additional information

## Decision

NAPH uses **WGS84 as the canonical CRS** for `naph:asWKT` literals. The native CRS may be preserved as an additional property.

```turtle
ex:footprint-001 a naph:GeographicFootprint ;
    naph:asWKT "POLYGON((-3.21 55.94, -3.16 55.94, -3.16 55.97, -3.21 55.97, -3.21 55.94))"^^geo:wktLiteral ;
    # Optional: native CRS preserved
    naph:asWKT_native "POLYGON((325000 670000, ...))"^^geo:wktLiteral ;
    naph:nativeCRS <http://www.opengis.net/def/crs/EPSG/0/27700> .
```

The `geo:wktLiteral` datatype implies WGS84 by GeoSPARQL specification (CRS84). A receiving system can rely on this.

## Consequences

### Positive

- **Simplest possible interoperability** — every NAPH-compliant collection's footprints can be combined without CRS reconciliation
- **GeoSPARQL spatial functions work** — `geof:sfIntersects` etc. assume CRS consistency; WGS84 universally satisfies this
- **Aggregator-friendly** — Europeana, DPLA, Wikidata, and all major geospatial tools default to WGS84
- **Tooling-friendly** — every modern GIS toolkit handles WGS84 natively
- **Federation-ready** — multi-institution queries don't require CRS transformation

### Negative

- **Lossy for some applications** — institutions doing photogrammetric work may need higher precision than WGS84 provides for their region
- **Re-projection cost** — institutions storing native OSGB36 (UK) or NAD83 (US) data must transform to WGS84 for the canonical footprint
- **Precision loss at high latitudes** — WGS84 is less precise near poles than purpose-built local CRSs

### Neutral

- The native CRS preservation (via `naph:asWKT_native` and `naph:nativeCRS`) means lossless round-trip: institutions can publish in WGS84 for federation while preserving their analytical data unchanged.

## Alternatives considered

### Alternative 1: Allow any CRS

Rejected because:

- Federation queries become hugely complex — you'd need to transform every footprint to a common CRS before any spatial operation
- Aggregators typically can't handle non-WGS84 data
- Adds significant complexity to the standard for marginal benefit

### Alternative 2: Mandate British National Grid (UK) / EPSG:27700

Considered briefly. Rejected because:

- The standard is intended for international adoption — UK-only CRS is parochial
- Requires re-projection for non-UK collections
- Doesn't solve the federation problem internationally

### Alternative 3: Use native CRS with mandatory CRS declaration

Considered. This is what the OGC GeoSPARQL standard supports natively.

Rejected for the canonical form (kept as optional via `naph:asWKT_native`) because:

- Most NAPH consumers (researchers, aggregators, tools) are not GeoSPARQL-aware
- A consistent, predictable WGS84 representation is more important for adoption than CRS flexibility
- Native CRS preservation as an additional property gets the best of both: simple federation + lossless analytical data

## CRS conversion

For institutions with native non-WGS84 data:

| Native | Conversion to WGS84 |
|---|---|
| OSGB36 (EPSG:27700) | OS-published transformation grids; OSTN15 for highest precision |
| NAD83 (EPSG:4269) | Effectively equivalent to WGS84 at heritage-data precision |
| Local UTM zones | Standard cartographic transformation |

Tooling: PROJ library, GDAL, Geopy, R (sf package), Python (pyproj).

## Precision and rounding

WGS84 footprints SHOULD be expressed with at least 5 decimal places (~1.1 metre precision at equator). For aerial photography heritage, 4-6 decimal places is typical.

Avoid spurious precision (e.g. don't write 13 decimal places for a footprint derived from a hand-drawn 1944 sortie plot — the original data wasn't that precise).

## Validation

The decision is validated by:

- All sample data uses WGS84 footprints; SHACL validation passes
- GeoSPARQL spatial queries work (where the engine supports them)
- The footprint-from-flight pipeline produces WGS84 polygons from altitude+focal-length+lat/lon inputs
- Comparing with comparable standards: DCAT, IIIF, Schema.org Place all default to WGS84

## Cross-references

- [Module B §B.6 — Geospatial structure](../../01-standard/modules/B-metadata-data-structures.md#b6-geospatial-structure)
- [Aerial Photography Profile §P.4 — Field-of-view derivation](../../01-standard/profiles/aerial-photography.md#p4-field-of-view-derivation)
- [Photogrammetric sub-profile §G.4 — CRS](../../01-standard/profiles/aerial-subprofiles/photogrammetric.md#g4-coordinate-reference-system-crs)
- [GeoSPARQL specification](https://www.ogc.org/standards/geosparql/)
- [PROJ — coordinate transformation library](https://proj.org/)
