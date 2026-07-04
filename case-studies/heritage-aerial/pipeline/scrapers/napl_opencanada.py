#!/usr/bin/env python3
"""
NAPL (Canada) Open-Government Adapter — LIVE public-API client.

Canada's National Air Photo Library (NAPL, Natural Resources Canada) holds
~6 million aerial photographs. The frame-level catalogue lives behind EODMS, but
NRCan additionally publishes the **NAPL Temporal Series** — regional orthophoto
mosaics assembled from those frames — as fully open data on the Government of
Canada open-data portal (open.canada.ca), served by a standard **CKAN** API that
needs no authentication.

This adapter harvests that open subset, metadata only, and lifts each dated
temporal slice to the NAPH Baseline tier. It is a companion to
``ncap_airphotofinder.py`` (United Kingdom) and ``usgs_earthexplorer.py`` (United
States), demonstrating that the NAPH standard is source-agnostic: the ontology,
SHACL shapes and RiC-O x STAC crosswalk are unchanged across all three national
collections — only the thin harvester adapter differs.

Key finding this adapter makes concrete
---------------------------------------
Where the NCAP payload was missing exactly one Baseline field (machine-readable
rights, 0/300), every NAPL open-data record carries all six:

- **Footprint**  — CKAN ``spatial`` field, a GeoJSON Polygon already in **WGS84**
  (EPSG:4326). No reprojection needed, unlike NCAP's EPSG:3857.
- **Date**       — ``time_period_coverage_start/end`` plus a year stamped on every
  per-year orthophoto-mosaic resource, mapping onto ``xsd:gYear``.
- **Identifier** — a stable dataset UUID, resolvable at open.canada.ca.
- **Rights**     — ``license_id = ca-ogl-lgo`` with a resolvable licence URI
  (Open Government Licence - Canada). This is the field NCAP lacked.

So the Canadian open collection is *already at Baseline on all six required
fields*, including the one the UK collection was missing. The residual work is
URI minting and format publication only.

Granularity note (stated honestly)
----------------------------------
These are collection-level **orthophoto mosaics**, one per acquisition year per
region, not the original individual frames (those sit in EODMS). Each mosaic is
modelled as a ``naph:AerialPhotograph`` at Baseline with the region's footprint
and that year's ``gYear`` date. Frame-level harvest via EODMS is the Enhanced-tier
upgrade path; NRCan already exposes a per-dataset STAC API for it.

Endpoints used (all public, read-only, no auth)
-----------------------------------------------
- GET ``/data/api/3/action/package_search?q=...``   (discover NAPL datasets)
- GET ``/data/api/3/action/package_show?id=...``     (dataset + resources)

Usage
-----
    python3 pipeline/scrapers/napl_opencanada.py \
        --raw-out pipeline/real-napl-raw.json > data/real-napl-sample.ttl

Output: NAPH-compliant Turtle to stdout; progress + provenance to stderr.
"""

import argparse
import json
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

CKAN_BASE = "https://open.canada.ca/data/api/3/action"
USER_AGENT = (
    "NAPH-interop-demo/1.0 (Open Ontologies heritage-aerial case study; "
    "metadata-only; contact fabio@thetesseractacademy.com)"
)

# The Open Government Licence - Canada is a single machine-readable rights URI
# that applies to every NAPL open-data record.
OGL_CANADA_URI = "https://open.canada.ca/en/open-government-licence-canada"

_NAPL_TITLE = "temporal series of the national air photo library"
_YEAR_RE = re.compile(r"\b(1[89]\d\d|20\d\d)\b")
_RES_RE = re.compile(r"\((\d+)\s*cm\)", re.IGNORECASE)


# -----------------------------------------------------------------------------
# HTTP
# -----------------------------------------------------------------------------
def _get_json(action: str, params: dict, timeout: int = 30) -> dict:
    url = f"{CKAN_BASE}/{action}?" + urllib.parse.urlencode(params)
    req = urllib.request.Request(url, method="GET")
    req.add_header("Accept", "application/json")
    req.add_header("User-Agent", USER_AGENT)
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        payload = json.loads(resp.read().decode("utf-8"))
    if not payload.get("success"):
        raise RuntimeError(f"CKAN action {action} returned success=false")
    return payload["result"]


def discover_napl_datasets(rows: int = 30) -> list:
    """Return the NAPL Temporal Series dataset stubs (regional series only)."""
    res = _get_json("package_search",
                    {"q": "Temporal Series National Air Photo Library", "rows": rows})
    out = []
    for r in res.get("results", []):
        hay = (str(r.get("title", "")) + json.dumps(r.get("title_translated", {}))).lower()
        if _NAPL_TITLE in hay and r.get("spatial"):
            out.append(r)
    return out


def dataset_detail(dataset_id: str) -> dict:
    return _get_json("package_show", {"id": dataset_id})


# -----------------------------------------------------------------------------
# Field extraction
# -----------------------------------------------------------------------------
def _label_str(value) -> str:
    """CKAN fields are sometimes {'en':..,'fr':..} dicts; prefer English text."""
    if isinstance(value, dict):
        return value.get("en") or value.get("fr") or ""
    return value or ""


def geojson_polygon_to_wkt(spatial_json: str):
    """Convert a CKAN ``spatial`` GeoJSON Polygon (WGS84) to a WKT POLYGON.

    NAPL footprints are already in EPSG:4326 (lon, lat), so this is a pure
    format change with no reprojection — the contrast with NCAP's EPSG:3857.
    """
    try:
        geom = json.loads(spatial_json)
    except (TypeError, ValueError):
        return None
    if geom.get("type") != "Polygon" or not geom.get("coordinates"):
        return None
    ring = geom["coordinates"][0]
    pts = []
    for pair in ring:
        if len(pair) < 2:
            return None
        lon, lat = float(pair[0]), float(pair[1])
        if not (-180 <= lon <= 180 and -90 <= lat <= 90):
            return None
        pts.append(f"{lon:.6f} {lat:.6f}")
    if len(pts) < 4:
        return None
    if pts[0] != pts[-1]:
        pts.append(pts[0])
    return "POLYGON((" + ", ".join(pts) + "))"


def geotif_year_slices(detail: dict) -> list:
    """Yield (year, resolution_cm, resource_name) for each per-year mosaic resource."""
    slices = []
    for res in detail.get("resources", []):
        fmt = (res.get("format") or "").upper()
        if "TIF" not in fmt:            # GeoTIF / GeoTIFF only; skip WMS/WCS/HTML/STAC
            continue
        name = _label_str(res.get("name"))
        ym = _YEAR_RE.search(name)
        if not ym:
            continue
        rm = _RES_RE.search(name)
        slices.append((ym.group(1), rm.group(1) if rm else None, name))
    return slices


# -----------------------------------------------------------------------------
# Turtle emission (NAPH Baseline, matching ncap_airphotofinder.py conventions)
# -----------------------------------------------------------------------------
def safe_id(text: str) -> str:
    return re.sub(r"[^A-Za-z0-9]+", "-", (text or "").strip()).strip("-")


def esc(text: str) -> str:
    return (text or "").replace("\\", "\\\\").replace('"', '\\"')


def region_from_title(title: str) -> str:
    """'... (NAPL) - Regina, Saskatchewan (1947-1967)' -> 'Regina, Saskatchewan'."""
    t = _label_str(title)
    m = re.search(r"\(NAPL\)\s*[-–]\s*(.+?)\s*\(\d", t)
    return (m.group(1) if m else t).strip()


def emit_prologue(n_datasets: int, n_items: int) -> str:
    return f"""@prefix rdf:     <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs:    <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd:     <http://www.w3.org/2001/XMLSchema#> .
@prefix dcterms: <http://purl.org/dc/terms/> .
@prefix dctype:  <http://purl.org/dc/dcmitype/> .
@prefix prov:    <http://www.w3.org/ns/prov#> .
@prefix geo:     <http://www.opengis.net/ont/geosparql#> .
@prefix naph:    <https://w3id.org/naph/ontology#> .
@prefix ex:      <https://w3id.org/naph/example/napl-live/> .

# =============================================================================
# REAL NAPL records harvested from the Government of Canada open-data portal
# (open.canada.ca CKAN API). Metadata only. Footprints are native WGS84 (no
# reprojection). {n_items} dated orthophoto-mosaic slices across {n_datasets}
# NAPL Temporal Series regions, lifted to NAPH Baseline.
# Rights: Open Government Licence - Canada (machine-readable, ca-ogl-lgo).
# =============================================================================

ex:NAPL a naph:CustodialInstitution ;
    rdfs:label "National Air Photo Library (Natural Resources Canada)" ;
    rdfs:seeAlso <https://natural-resources.canada.ca/maps-tools-publications/satellite-elevation-air-photos/air-photos-library> .

ex:NAPLCollection a naph:Collection ;
    rdfs:label "NAPL Temporal Series (open data)" ;
    naph:custodian ex:NAPL .

ex:rights-ogl a naph:RightsStatement ;
    naph:rightsURI <{OGL_CANADA_URI}> ;
    naph:rightsLabel "Open Government Licence - Canada" .

"""


def emit_dataset(detail: dict) -> tuple:
    """Return (ttl_string, n_items) for one NAPL Temporal Series dataset."""
    ds_id = detail.get("id")
    title = detail.get("title") or detail.get("title_translated")
    region = region_from_title(title)
    wkt = geojson_polygon_to_wkt(detail.get("spatial", ""))
    if not wkt or not ds_id:
        return "", 0
    slices = geotif_year_slices(detail)
    if not slices:
        return "", 0

    short = safe_id(region) or ds_id[:8]
    sortie = f"ex:sortie-{short}"
    portal = f"https://open.canada.ca/data/en/dataset/{ds_id}"

    lines = [f"# NAPL Temporal Series — {region}  (dataset {ds_id})"]
    lines.append(
        f"{sortie} a naph:Sortie ;\n"
        f'    naph:sortieReference "NAPL-TS/{esc(short)}" ;\n'
        f'    rdfs:label "NAPL Temporal Series over {esc(region)}" ;\n'
        f"    prov:hadPrimarySource <{portal}> ."
    )

    n = 0
    for i, (year, res_cm, res_name) in enumerate(slices):
        pid = f"{short}-{year}-{i}"
        label = esc(res_name) or f"{esc(region)} orthophoto mosaic {year}"
        photo = [
            f"ex:photo-{pid} a naph:AerialPhotograph ;",
            f"    dcterms:type dctype:StillImage ;",
            f'    rdfs:label "{label}" ;',
            f'    rdfs:comment "Orthophoto mosaic assembled from NAPL frames; collection-level Baseline surrogate, not an original single frame." ;',
            f'    naph:hasIdentifier "https://w3id.org/naph/photo/napl-{pid}" ;',
            f'    dcterms:identifier "NAPL-OpenData:{ds_id}" ;',
            f'    naph:capturedOn "{year}"^^xsd:gYear ;',
            f"    naph:partOfSortie {sortie} ;",
            f"    naph:belongsToCollection ex:NAPLCollection ;",
            f"    naph:coversArea ex:footprint-{pid} ;",
            f"    naph:hasRightsStatement ex:rights-ogl ;",
            f"    prov:hadPrimarySource <{portal}> ;",
            f'    naph:imagePerspective "vertical" ;',
            f"    naph:compliesWithTier naph:TierBaseline .",
        ]
        lines.append("\n".join(photo))
        lines.append(
            f"ex:footprint-{pid} a naph:GeographicFootprint ;\n"
            f'    naph:asWKT "{wkt}"^^geo:wktLiteral ;\n'
            f'    rdfs:comment "Native WGS84 footprint from CKAN spatial field (no reprojection)." .'
        )
        n += 1

    return "\n\n".join(lines) + "\n\n", n


# -----------------------------------------------------------------------------
# Harvest
# -----------------------------------------------------------------------------
def harvest(delay: float, raw_out):
    stubs = discover_napl_datasets()
    print(f"# discovered {len(stubs)} NAPL Temporal Series datasets", file=sys.stderr)

    bodies = []
    raw = []
    total_items = 0
    used_datasets = 0
    for stub in stubs:
        try:
            detail = dataset_detail(stub["id"])
        except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError, RuntimeError) as e:
            print(f"# {stub['id']}: fetch failed: {e}", file=sys.stderr)
            continue
        ttl, n = emit_dataset(detail)
        if n:
            bodies.append(ttl)
            raw.append(detail)
            used_datasets += 1
            total_items += n
            print(f"# {region_from_title(detail.get('title'))}: {n} dated mosaics kept",
                  file=sys.stderr)
        time.sleep(delay)

    out = emit_prologue(used_datasets, total_items) + "".join(bodies)

    if raw_out:
        with open(raw_out, "w") as f:
            json.dump(raw, f, indent=1)
        print(f"# raw JSON snapshot written to {raw_out} ({len(raw)} datasets)", file=sys.stderr)

    print(f"# Harvest complete: {total_items} real NAPL mosaics across "
          f"{used_datasets} regions lifted to NAPH Baseline.", file=sys.stderr)
    return out


def main():
    p = argparse.ArgumentParser(description="Harvest a real NAPL (Canada) open-data sample into NAPH Turtle.")
    p.add_argument("--delay", type=float, default=0.7, help="seconds between API requests (default 0.7)")
    p.add_argument("--raw-out", default=None, help="also write the raw CKAN JSON here (for reproducibility)")
    args = p.parse_args()
    sys.stdout.write(harvest(args.delay, args.raw_out))


if __name__ == "__main__":
    main()
