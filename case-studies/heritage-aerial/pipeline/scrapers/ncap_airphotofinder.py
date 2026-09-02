#!/usr/bin/env python3
"""
NCAP Air Photo Finder Adapter — LIVE public-API client.

Air Photo Finder (https://airphotofinder.ncap.org/) is an Angular SPA backed by
a public REST API at ``/api/v1``. The same endpoints the public website calls to
render its search and map views are reachable directly and return structured
JSON — including a machine-readable footprint polygon for every frame.

This adapter harvests a *small, stratified, metadata-only* sample of catalogue
records and lifts them to the NAPH Baseline tier. It fetches metadata only; no
image binaries are downloaded and no ordering/basket/account endpoints are
touched. It is deliberately rate-limited and identifies itself in the
User-Agent. It is intended as a good-faith interoperability demonstration, not a
bulk-extraction tool.

Scope and legal basis
---------------------
The sample is capped at ``MAX_RECORDS`` (500) records. The Air Photo Finder
catalogue holds on the order of 1.14 million records, so a full-cap run reads
well under 0.05% of it.

Regulation 19(1) of the Copyright and Rights in Databases Regulations 1997
entitles a lawful user of a database that has been made available to the public
to extract or re-utilise *insubstantial* parts of its contents for any purpose,
and regulation 19(2) makes void any term or condition purporting to prevent
that. This adapter is written to stay squarely inside that entitlement: an
insubstantial sample, metadata only, read-only public endpoints, rate-limited,
self-identifying, with no ordering, basket or account endpoint touched and no
image binary retrieved.

No harvested catalogue records are redistributed in this repository. The
published outputs are derived counts, proportions and NAPH-lifted exemplars.
Rights in the underlying images and in the catalogue remain with NCAP and
Historic Environment Scotland; anyone wanting a substantial extract, or any use
of the imagery itself, should approach NCAP directly and license it.

Key finding this adapter makes concrete
---------------------------------------
The ``details.image_coordinates`` field is a WKT ``POLYGON`` in EPSG:3857
(Web Mercator). NCAP therefore *already* holds a machine-readable footprint for
each frame — the computation-readiness gap for geometry is a reprojection and a
publication-format choice (WGS84 / GeoSPARQL), not missing data. ``details.date``
is already ISO 8601 and ``details.date_precision`` already encodes day / month /
year precision, which maps one-to-one onto the NAPH date-precision policy
(ADR-0009).

Endpoints used (all public, read-only)
--------------------------------------
- POST ``/api/v1/image_search?page_no=N``  body: {image_types?, min_date?, max_date?, ...}
- GET  ``/api/v1/place_name?name=...``      (place lookup, for provenance notes)

Usage
-----
    # Default: ~300 records stratified across several date windows
    python3 pipeline/scrapers/ncap_airphotofinder.py > pipeline/real-ncap-sample.ttl

    # Custom sample size and raw-JSON snapshot for reproducibility
    python3 pipeline/scrapers/ncap_airphotofinder.py \
        --limit 400 --raw-out pipeline/real-ncap-raw.json > out.ttl

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

API_BASE = "https://airphotofinder.ncap.org/api/v1"
USER_AGENT = (
    "NAPH-interop-demo/1.0 (Open Ontologies heritage-aerial case study; "
    "metadata-only; contact fabio@thetesseractacademy.com)"
)

# Stratified date windows so the sample spans the collection's real temporal
# spread (WWI-era RN sorties through Cold-War reconnaissance) rather than
# whatever the default sort returns first. Each window contributes records up to
# the per-window cap until the global --limit is reached.
#
# Hard ceiling on any single run. This is a deliberate design limit, not a
# tuning parameter: it keeps every harvest an insubstantial extract of the
# ~1.14M-record catalogue (see the module docstring on regulation 19).
MAX_RECORDS = 500
DATE_WINDOWS = [
    ("1918-01-01", "1930-12-31"),   # interwar / early naval
    ("1939-01-01", "1945-12-31"),   # WWII
    ("1946-01-01", "1955-12-31"),   # post-war surveys
    ("1956-01-01", "1975-12-31"),   # Cold-War reconnaissance
]

R_MAJOR = 6378137.0                 # EPSG:3857 sphere radius / WGS84 semi-major
ORIGIN_SHIFT = math.pi * R_MAJOR    # 20037508.342789244


# -----------------------------------------------------------------------------
# EPSG:3857 (Web Mercator) -> EPSG:4326 (WGS84) reprojection
# -----------------------------------------------------------------------------
def webmercator_to_wgs84(x: float, y: float) -> tuple[float, float]:
    """Return (lon, lat) in degrees for a Web-Mercator (x, y) in metres."""
    lon = (x / ORIGIN_SHIFT) * 180.0
    lat = (y / ORIGIN_SHIFT) * 180.0
    lat = 180.0 / math.pi * (2.0 * math.atan(math.exp(lat * math.pi / 180.0)) - math.pi / 2.0)
    return lon, lat


_POLY_RE = re.compile(r"POLYGON\s*\(\((.*?)\)\)", re.IGNORECASE | re.DOTALL)


def reproject_polygon_wkt(wkt_3857: str) -> str | None:
    """Convert a WKT POLYGON in EPSG:3857 to a WGS84 WKT POLYGON (lon lat)."""
    m = _POLY_RE.search(wkt_3857 or "")
    if not m:
        return None
    coords = []
    for pair in m.group(1).split(","):
        parts = pair.strip().split()
        if len(parts) != 2:
            return None
        try:
            x, y = float(parts[0]), float(parts[1])
        except ValueError:
            return None
        lon, lat = webmercator_to_wgs84(x, y)
        if not (-180 <= lon <= 180 and -90 <= lat <= 90):
            return None
        coords.append(f"{lon:.6f} {lat:.6f}")
    if len(coords) < 4:
        return None
    return "POLYGON((" + ", ".join(coords) + "))"


def polygon_centroid(wkt_wgs84: str) -> tuple[float, float] | None:
    m = _POLY_RE.search(wkt_wgs84 or "")
    if not m:
        return None
    pts = []
    for pair in m.group(1).split(","):
        parts = pair.strip().split()
        if len(parts) == 2:
            pts.append((float(parts[0]), float(parts[1])))
    if not pts:
        return None
    return (sum(p[0] for p in pts) / len(pts), sum(p[1] for p in pts) / len(pts))


# -----------------------------------------------------------------------------
# HTTP
# -----------------------------------------------------------------------------
def _post_json(path: str, body: dict, params: dict, timeout: int = 30) -> dict:
    url = f"{API_BASE}/{path}"
    if params:
        url += "?" + urllib.parse.urlencode(params)
    data = json.dumps(body).encode("utf-8")
    req = urllib.request.Request(url, data=data, method="POST")
    req.add_header("Content-Type", "application/json")
    req.add_header("Accept", "application/json")
    req.add_header("Origin", "https://airphotofinder.ncap.org")
    req.add_header("User-Agent", USER_AGENT)
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        return json.loads(resp.read().decode("utf-8"))


def image_search(page_no: int, min_date: str = "", max_date: str = "",
                 image_types: str = "") -> dict:
    body: dict = {}
    if min_date:
        body["min_date"] = min_date
    if max_date:
        body["max_date"] = max_date
    if image_types:
        body["image_types"] = image_types
    return _post_json("image_search", body, {"page_no": page_no})


# -----------------------------------------------------------------------------
# Turtle emission (NAPH Baseline+, matching pipeline/ingest.py conventions)
# -----------------------------------------------------------------------------
def safe_id(text: str) -> str:
    return re.sub(r"[^A-Za-z0-9]+", "-", (text or "").strip()).strip("-")


def esc(text: str) -> str:
    return (text or "").replace("\\", "\\\\").replace('"', '\\"')


DATE_PRECISION_XSD = {"day": "date", "month": "gYearMonth", "year": "gYear"}


def iso_for_precision(iso_date: str, precision: str) -> tuple[str, str]:
    """Trim an ISO date to its stated precision and pick the xsd type."""
    xsd = DATE_PRECISION_XSD.get((precision or "day").lower(), "date")
    if xsd == "gYear":
        return iso_date[:4], xsd
    if xsd == "gYearMonth":
        return iso_date[:7], xsd
    return iso_date, xsd


def emit_prologue(total: int, windows: list) -> str:
    win = "; ".join(f"{a}..{b}" for a, b in windows)
    return f"""@prefix rdf:     <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs:    <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd:     <http://www.w3.org/2001/XMLSchema#> .
@prefix dcterms: <http://purl.org/dc/terms/> .
@prefix dctype:  <http://purl.org/dc/dcmitype/> .
@prefix prov:    <http://www.w3.org/ns/prov#> .
@prefix geo:     <http://www.opengis.net/ont/geosparql#> .
@prefix naph:    <https://w3id.org/naph/ontology#> .
@prefix ex:      <https://w3id.org/naph/example/ncap-live/> .

# =============================================================================
# REAL NCAP records harvested from the public Air Photo Finder API (/api/v1).
# Metadata only. Footprints reprojected EPSG:3857 -> WGS84. Records lifted to
# NAPH Baseline tier. {total} records; date windows: {win}.
# Source: https://airphotofinder.ncap.org/  Rights: NCAP / see catalogue.
# =============================================================================

ex:NCAP a naph:CustodialInstitution ;
    rdfs:label "National Collection of Aerial Photography" ;
    rdfs:seeAlso <https://ncap.org.uk/> .

ex:NCAPCollection a naph:Collection ;
    rdfs:label "NCAP Holdings (Air Photo Finder)" ;
    naph:custodian ex:NCAP .

# DISCLOSURE: the Air Photo Finder API payload carries no rights field (the
# case study's central finding). So that the other five Baseline elements can
# be exercised by SHACL, the adapter attaches ONE adapter-supplied,
# collection-level interim rights statement to every record. It uses the
# rightsstatements.org Copyright Not Evaluated URI (the honest statement for a mixed collection whose determination is the institution's to make) and is
# NOT sourced from or asserted by NCAP; rights determination is NCAP's alone.
# Validation results are reported both with and without this attachment.
ex:rights-ncap a naph:RightsStatement ;
    naph:rightsURI <http://rightsstatements.org/vocab/CNE/1.0/> ;
    naph:rightsLabel "Copyright Not Evaluated - adapter-supplied interim statement, not asserted by NCAP" .

"""


def emit_record(rec: dict) -> str | None:
    details = rec.get("details") or {}
    meta = rec.get("image_metadata") or {}

    sortie = details.get("sortie") or ""
    frame = str(details.get("frame_id") or meta.get("Frame") or "").strip()
    fp_id = rec.get("footprint_id")
    if not sortie or fp_id is None:
        return None

    wkt = reproject_polygon_wkt(details.get("image_coordinates", ""))
    if not wkt:
        return None

    date = (details.get("date") or "").strip()
    if not re.match(r"^\d{4}-\d{2}-\d{2}$", date):
        return None
    iso_date, xsd_type = iso_for_precision(date, details.get("date_precision", "day"))

    pid = f"{safe_id(sortie)}-{safe_id(frame) or fp_id}"
    isad = meta.get("ISAD(G)") or ""
    uni = meta.get("UNI") or ""
    coll_ctx = details.get("collection_context") or ""
    img_type = details.get("image_type") or ""
    camera = meta.get("Camera") or ""
    apf_url = f"https://airphotofinder.ncap.org/image/{rec.get('id')}"

    lines = [f"# {sortie} frame {frame}  (footprint {fp_id}, ISAD(G) {isad})"]

    sortie_lines = [f"ex:sortie-{safe_id(sortie)} a naph:Sortie ;",
                    f'    naph:sortieReference "{esc(sortie)}" ;']
    if coll_ctx:
        sortie_lines.append(f'    rdfs:comment "Collection context: {esc(coll_ctx)}" ;')
    sortie_lines[-1] = sortie_lines[-1].rstrip(" ;") + " ."
    lines.append("\n".join(sortie_lines))

    photo = [
        f"ex:photo-{pid} a naph:AerialPhotograph ;",
        f"    dcterms:type dctype:StillImage ;",
        f'    rdfs:label "{esc(sortie)} frame {esc(frame)}" ;',
        f'    naph:hasIdentifier "https://w3id.org/naph/photo/{pid}" ;',
    ]
    if uni:
        photo.append(f'    dcterms:identifier "NCAP-UNI:{esc(uni)}" ;')
    if isad:
        photo.append(f'    dcterms:identifier "{esc(isad)}" ;')
    if frame.isdigit():
        photo.append(f"    naph:frameNumber {int(frame)} ;")
    photo += [
        f"    naph:partOfSortie ex:sortie-{safe_id(sortie)} ;",
        f"    naph:belongsToCollection ex:NCAPCollection ;",
        f'    naph:capturedOn "{iso_date}"^^xsd:{xsd_type} ;',
        f"    naph:coversArea ex:footprint-{pid} ;",
        f"    naph:hasRightsStatement ex:rights-ncap ;",
        f"    prov:hadPrimarySource <{apf_url}> ;",
    ]
    if img_type:
        photo.append(f'    naph:imagePerspective "{esc(img_type)}" ;')
    if camera:
        photo.append(f"    naph:hasCaptureEvent ex:capture-{pid} ;")
    photo.append(f"    naph:compliesWithTier naph:TierBaseline .")
    lines.append("\n".join(photo))

    fp = [f"ex:footprint-{pid} a naph:GeographicFootprint ;",
          f'    naph:asWKT "{wkt}"^^geo:wktLiteral ;',
          f'    rdfs:comment "Reprojected from NCAP EPSG:3857 image_coordinates to WGS84." .']
    lines.append("\n".join(fp))

    if camera:
        lines.append(f"ex:capture-{pid} a naph:CaptureEvent ;\n"
                     f'    naph:cameraType "{esc(camera)}" .')

    return "\n\n".join(lines) + "\n\n"


# -----------------------------------------------------------------------------
# Harvest
# -----------------------------------------------------------------------------
def harvest(limit: int, delay: float, raw_out: str | None) -> str:
    parts = [emit_prologue(limit, DATE_WINDOWS)]
    seen: set = set()
    raw_records: list = []
    per_window = max(1, math.ceil(limit / len(DATE_WINDOWS)))
    emitted = 0

    for (mn, mx) in DATE_WINDOWS:
        if emitted >= limit:
            break
        window_emitted = 0
        page = 1
        while window_emitted < per_window and emitted < limit:
            try:
                data = image_search(page, min_date=mn, max_date=mx)
            except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError) as e:
                print(f"# window {mn}..{mx} page {page}: request failed: {e}", file=sys.stderr)
                break
            images = data.get("images") or []
            if not images:
                break
            for rec in images:
                key = rec.get("footprint_id")
                if key in seen:
                    continue
                ttl = emit_record(rec)
                if ttl:
                    seen.add(key)
                    raw_records.append(rec)
                    parts.append(ttl)
                    emitted += 1
                    window_emitted += 1
                    if window_emitted >= per_window or emitted >= limit:
                        break
            print(f"# window {mn}..{mx} page {page}: {window_emitted} kept "
                  f"(total {emitted}/{limit}, {data.get('hits')} hits available)",
                  file=sys.stderr)
            page += 1
            time.sleep(delay)

    if raw_out:
        with open(raw_out, "w") as f:
            json.dump(raw_records, f, indent=1)
        print(f"# raw JSON snapshot written to {raw_out} ({len(raw_records)} records)",
              file=sys.stderr)

    print(f"# Harvest complete: {emitted} real NCAP records lifted to NAPH Baseline.",
          file=sys.stderr)
    return "".join(parts)


def main():
    p = argparse.ArgumentParser(description="Harvest a real NCAP Air Photo Finder sample into NAPH Turtle.")
    p.add_argument("--limit", type=int, default=300,
                   help=f"max records to harvest (default 300, hard cap {MAX_RECORDS})")
    p.add_argument("--delay", type=float, default=0.7, help="seconds between API requests (default 0.7)")
    p.add_argument("--raw-out", default=None, help="also write the raw API JSON here (for reproducibility)")
    args = p.parse_args()

    # Enforced, not advisory: keep every run an insubstantial extract.
    if args.limit > MAX_RECORDS:
        print(f"# --limit {args.limit} exceeds the hard cap of {MAX_RECORDS}; "
              f"harvesting {MAX_RECORDS}. This adapter deliberately does not "
              f"support bulk extraction (see module docstring).", file=sys.stderr)
        args.limit = MAX_RECORDS

    sys.stdout.write(harvest(args.limit, args.delay, args.raw_out))


if __name__ == "__main__":
    main()
