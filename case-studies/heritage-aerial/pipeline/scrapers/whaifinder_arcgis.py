#!/usr/bin/env python3
"""
WHAIFinder (USA) Adapter — LIVE open ArcGIS FeatureServer client.

The United States has no single national aerial-heritage catalogue with an open
API. The frame-level USGS EROS archive is real but its M2M API is token-gated
(see ``usgs_earthexplorer.py``). Instead, the richest *openly queryable* US
holdings are published as **ArcGIS Feature Services**, which expose a public
REST/JSON query API with no authentication.

This adapter targets the **Wisconsin Historic Aerial Imagery Finder (WHAIFinder)**
— ~201,000 frame-level records, held by the UW-Madison Robinson Map Library and
the Wisconsin State Cartographer's Office, drawn largely from public-domain USDA
survey photography (1930s onward). It is a companion to ``ncap_airphotofinder.py``
(UK) and ``napl_opencanada.py`` (Canada). The same ArcGIS-FeatureServer pattern
also fits UCSB's national FrameFinder service and many US state indexes; only the
service URL and field map change.

The interesting US-specific finding
-----------------------------------
WHAIFinder publishes a **point centerpoint**, not a polygon footprint — so unlike
NCAP (which had a polygon in the wrong CRS) or NAPL (native-WGS84 polygon), the US
index is missing the *geometry area* at Baseline. But it also publishes
``map_scale_denom``. A footprint is therefore a **closed-form reconstruction**:
for the standard 9x9-inch (0.2286 m) aerial frame, ground side = 0.2286 x scale.
That is one automatable transform to reach Baseline, exactly parallel to NCAP's
reprojection — the substrate (centerpoint + scale) is already in the data.

Each collection needed a *different* single transform to reach Baseline:
reproject (UK), publish-as-is / already-there (Canada), reconstruct-from-scale
(US). Same ontology, shapes and crosswalk throughout.

Endpoints used (public, read-only, no auth)
-------------------------------------------
- GET ``<FeatureServer>/0/query?where=...&outFields=...&f=geojson``

Usage
-----
    python3 pipeline/scrapers/whaifinder_arcgis.py \
        --limit 300 --raw-out pipeline/real-whai-raw.json > data/real-whai-sample.ttl

Output: NAPH-compliant Turtle to stdout; progress + provenance to stderr.
"""

import argparse
import json
import math
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timedelta, timezone

LAYER = ("https://services.arcgis.com/HRPe58bUyBqyyiCt/arcgis/rest/services/"
         "Wisconsin_Historic_Aerial_Imagery/FeatureServer/0")
USER_AGENT = (
    "NAPH-interop-demo/1.0 (Open Ontologies heritage-aerial case study; "
    "metadata-only; contact fabio@thetesseractacademy.com)"
)

# Standard US aerial survey frame: 9 x 9 inches = 0.2286 m film format.
FRAME_SIDE_M = 0.2286

# Temporal windows so the sample spans the collection, not just its first page.
YEAR_WINDOWS = [(1930, 1945), (1946, 1960), (1961, 1975), (1976, 2005)]

OUTFIELDS = ",".join([
    "uuid", "acquisition_date", "acquisition_year", "photo_common_name",
    "held_by", "exposure_type", "roll_number", "frame_number",
    "map_scale_denom", "collection_identifier", "source_organization",
])


# -----------------------------------------------------------------------------
# HTTP (ArcGIS REST query)
# -----------------------------------------------------------------------------
def query(where: str, offset: int, count: int = 200, timeout: int = 40) -> list:
    params = {
        "where": where,
        "outFields": OUTFIELDS,
        "orderByFields": "uuid ASC",
        "resultOffset": offset,
        "resultRecordCount": count,
        "f": "geojson",
    }
    url = f"{LAYER}/query?" + urllib.parse.urlencode(params)
    req = urllib.request.Request(url, method="GET")
    req.add_header("Accept", "application/json")
    req.add_header("User-Agent", USER_AGENT)
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        payload = json.loads(resp.read().decode("utf-8"))
    return payload.get("features", [])


# -----------------------------------------------------------------------------
# Footprint reconstruction: centerpoint + map scale -> nominal WGS84 polygon
# -----------------------------------------------------------------------------
def reconstruct_footprint(lon: float, lat: float, scale_denom: float):
    """Closed-form nominal footprint for a standard 9x9-inch frame at ``scale``.

    Ground side = frame_side (m) * scale_denominator. The square is centred on the
    published centerpoint and axis-aligned; terrain and camera tilt are ignored
    (a Baseline nominal footprint, refined at Enhanced from altitude/GSD).
    """
    if not scale_denom or scale_denom <= 0:
        return None
    half_m = (FRAME_SIDE_M * scale_denom) / 2.0
    dlat = half_m / 111_320.0
    dlon = half_m / (111_320.0 * max(math.cos(math.radians(lat)), 1e-6))
    ring = [
        (lon - dlon, lat - dlat), (lon + dlon, lat - dlat),
        (lon + dlon, lat + dlat), (lon - dlon, lat + dlat),
        (lon - dlon, lat - dlat),
    ]
    return "POLYGON((" + ", ".join(f"{x:.6f} {y:.6f}" for x, y in ring) + "))"


def epoch_ms_to_iso(ms) -> str:
    """ArcGIS epoch-ms (UTC) -> 'YYYY-MM-DD', or '' if unusable."""
    if ms is None:
        return ""
    try:
        dt = datetime(1970, 1, 1, tzinfo=timezone.utc) + timedelta(milliseconds=int(ms))
    except (ValueError, OverflowError, OSError):
        return ""
    if not (1900 <= dt.year <= 2030):
        return ""
    return dt.strftime("%Y-%m-%d")


# -----------------------------------------------------------------------------
# Turtle emission (NAPH Baseline, matching the UK/Canada adapter conventions)
# -----------------------------------------------------------------------------
def safe_id(text) -> str:
    return re.sub(r"[^A-Za-z0-9]+", "-", (str(text) or "").strip()).strip("-")


def esc(text) -> str:
    return (str(text) if text is not None else "").replace("\\", "\\\\").replace('"', '\\"')


def is_usda(source: str) -> bool:
    s = (source or "").lower()
    return "agriculture" in s or "usda" in s or "united states department" in s


def emit_prologue(n: int) -> str:
    return f"""@prefix rdf:     <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs:    <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd:     <http://www.w3.org/2001/XMLSchema#> .
@prefix dcterms: <http://purl.org/dc/terms/> .
@prefix dctype:  <http://purl.org/dc/dcmitype/> .
@prefix prov:    <http://www.w3.org/ns/prov#> .
@prefix geo:     <http://www.opengis.net/ont/geosparql#> .
@prefix naph:    <https://w3id.org/naph/ontology#> .
@prefix ex:      <https://w3id.org/naph/example/whai-live/> .

# =============================================================================
# REAL WHAIFinder records harvested from the public Wisconsin Historic Aerial
# Imagery ArcGIS FeatureServer (open REST, no auth). Metadata only. Geometry
# published as centerpoints; footprints reconstructed closed-form from centerpoint
# + map scale (standard 9x9-inch frame). {n} frames lifted to NAPH Baseline.
# =============================================================================

ex:WHAI a naph:CustodialInstitution ;
    rdfs:label "Wisconsin Historic Aerial Imagery (UW-Madison Robinson Map Library / WI State Cartographer)" ;
    rdfs:seeAlso <https://maps.sco.wisc.edu/whaifinder/> .

ex:WHAICollection a naph:Collection ;
    rdfs:label "Wisconsin Historic Aerial Imagery Finder (WHAIFinder)" ;
    naph:custodian ex:WHAI .

# USDA survey photography is US-Government work: public domain.
ex:rights-usda-pd a naph:RightsStatement ;
    naph:rightsURI <https://creativecommons.org/publicdomain/mark/1.0/> ;
    naph:rightsLabel "Public Domain (US Government work, USDA aerial survey)" .

ex:rights-whai a naph:RightsStatement ;
    naph:rightsURI <https://maps.sco.wisc.edu/whaifinder/> ;
    naph:rightsLabel "See holding institution terms (WHAIFinder)" .

"""


def emit_record(feat: dict, seen: set):
    geom = feat.get("geometry") or {}
    props = feat.get("properties") or {}
    if geom.get("type") != "Point":
        return None
    uuid = props.get("uuid")
    if not uuid or uuid in seen:
        return None
    lon, lat = geom.get("coordinates", [None, None])[:2]
    if lon is None or lat is None:
        return None

    scale = props.get("map_scale_denom")
    wkt = reconstruct_footprint(float(lon), float(lat), float(scale) if scale else 0)
    if not wkt:
        return None

    iso = epoch_ms_to_iso(props.get("acquisition_date"))
    if iso:
        date_line = f'    naph:capturedOn "{iso}"^^xsd:date ;'
    elif props.get("acquisition_year"):
        date_line = f'    naph:capturedOn "{int(props["acquisition_year"])}"^^xsd:gYear ;'
    else:
        return None

    seen.add(uuid)
    pid = safe_id(uuid)
    roll = props.get("roll_number") or "0"
    frame = props.get("frame_number") or ""
    coll = props.get("collection_identifier") or "WHAI"
    name = (props.get("photo_common_name") or "").strip().replace("\n", " ")
    source = props.get("source_organization") or ""
    rights = "ex:rights-usda-pd" if is_usda(source) else "ex:rights-whai"
    sortie = f"ex:sortie-{safe_id(coll)}-{safe_id(roll)}"

    lines = [f"# {esc(coll)} roll {esc(roll)} frame {esc(frame)}  (uuid {uuid})"]
    lines.append(
        f"{sortie} a naph:Sortie ;\n"
        f'    naph:sortieReference "{esc(coll)}/roll-{esc(roll)}" ;\n'
        f'    rdfs:label "WHAI {esc(coll)} roll {esc(roll)}" .'
    )

    photo = [
        f"ex:photo-{pid} a naph:AerialPhotograph ;",
        f"    dcterms:type dctype:StillImage ;",
        f'    rdfs:label "{esc(name) or (esc(coll) + " roll " + esc(roll) + " frame " + esc(frame))}" ;',
        f'    naph:hasIdentifier "https://w3id.org/naph/photo/whai-{pid}" ;',
        f'    dcterms:identifier "WHAIFinder:{esc(uuid)}" ;',
    ]
    if str(frame).isdigit():
        photo.append(f"    naph:frameNumber {int(frame)} ;")
    photo += [
        date_line,
        f"    naph:partOfSortie {sortie} ;",
        f"    naph:belongsToCollection ex:WHAICollection ;",
        f"    naph:coversArea ex:footprint-{pid} ;",
        f"    naph:hasRightsStatement {rights} ;",
        f"    prov:hadPrimarySource <https://maps.sco.wisc.edu/whaifinder/> ;",
        f'    naph:imagePerspective "vertical" ;',
        f"    naph:compliesWithTier naph:TierBaseline .",
    ]
    lines.append("\n".join(photo))
    lines.append(
        f"ex:footprint-{pid} a naph:GeographicFootprint ;\n"
        f'    naph:asWKT "{wkt}"^^geo:wktLiteral ;\n'
        f'    rdfs:comment "Nominal footprint reconstructed from WHAIFinder centerpoint + map scale 1:{int(float(scale))} '
        f'(standard 9x9-inch frame); axis-aligned, terrain/tilt ignored." .'
    )
    return "\n\n".join(lines) + "\n\n"


# -----------------------------------------------------------------------------
# Harvest
# -----------------------------------------------------------------------------
def harvest(limit: int, delay: float, raw_out):
    parts = [emit_prologue(limit)]
    seen: set = set()
    raw: list = []
    per_window = max(1, math.ceil(limit / len(YEAR_WINDOWS)))
    emitted = 0

    for (y0, y1) in YEAR_WINDOWS:
        if emitted >= limit:
            break
        where = f"map_scale_denom>0 AND acquisition_year>={y0} AND acquisition_year<={y1}"
        offset = 0
        window_emitted = 0
        while window_emitted < per_window and emitted < limit:
            try:
                feats = query(where, offset)
            except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError) as e:
                print(f"# window {y0}-{y1} offset {offset}: request failed: {e}", file=sys.stderr)
                break
            if not feats:
                break
            for feat in feats:
                ttl = emit_record(feat, seen)
                if ttl:
                    parts.append(ttl)
                    raw.append(feat)
                    emitted += 1
                    window_emitted += 1
                    if window_emitted >= per_window or emitted >= limit:
                        break
            print(f"# window {y0}-{y1} offset {offset}: {window_emitted} kept "
                  f"(total {emitted}/{limit})", file=sys.stderr)
            offset += len(feats)
            time.sleep(delay)

    if raw_out:
        with open(raw_out, "w") as f:
            json.dump(raw, f, indent=1)
        print(f"# raw JSON snapshot written to {raw_out} ({len(raw)} records)", file=sys.stderr)

    print(f"# Harvest complete: {emitted} real WHAIFinder frames lifted to NAPH Baseline.",
          file=sys.stderr)
    return "".join(parts)


def main():
    p = argparse.ArgumentParser(description="Harvest a real WHAIFinder (USA) sample into NAPH Turtle.")
    p.add_argument("--limit", type=int, default=300, help="max records to harvest (default 300)")
    p.add_argument("--delay", type=float, default=0.5, help="seconds between API requests (default 0.5)")
    p.add_argument("--raw-out", default=None, help="also write the raw ArcGIS GeoJSON here")
    args = p.parse_args()
    sys.stdout.write(harvest(args.limit, args.delay, args.raw_out))


if __name__ == "__main__":
    main()
