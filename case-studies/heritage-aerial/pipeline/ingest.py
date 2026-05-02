#!/usr/bin/env python3
"""
NAPH Ingest Pipeline — legacy NCAP-style CSV → computation-ready NAPH Turtle

Demonstrates the transformations required to lift current-state heritage
metadata (free-text dates, ambiguous rights, lat/lon points, mixed formats)
into the NAPH Baseline tier — the minimum bar for computation-readiness.

This pipeline is the operational equivalent of "going from digitised to
computable" — the central insight of the Towards a National Collection / N-RICH Prototype.

Usage:
    python3 ingest.py legacy-ncap-style.csv > generated.ttl
    open-ontologies validate /dev/stdin < generated.ttl

Features:
- Partial-date support (xsd:date / gYearMonth / gYear) with circa annotations
- Field-of-view footprint derivation when altitude + focal length present
- Three-component identifier parsing (Collection / Sortie / Frame)
- WGS84 coordinate range validation
- Rights text → canonical URI mapping
"""

import csv
import math
import re
import sys
from datetime import datetime
from pathlib import Path


# -----------------------------------------------------------------------------
# Rights text → rightsstatements.org URI mapping
# -----------------------------------------------------------------------------
RIGHTS_MAPPING = {
    "Crown Copyright Expired": (
        "http://rightsstatements.org/vocab/NoC-OKLR/1.0/",
        "No Copyright — Other Known Legal Restrictions",
        "CrownCopyrightExpired",
    ),
    "Crown Copyright": (
        "https://www.nationalarchives.gov.uk/information-management/re-using-public-sector-information/uk-government-licensing-framework/crown-copyright/",
        "Crown Copyright",
        "CrownCopyright",
    ),
    "Public Domain (US)": (
        "http://rightsstatements.org/vocab/NoC-US/1.0/",
        "No Copyright — United States",
        "NARAPublicDomain",
    ),
}


# -----------------------------------------------------------------------------
# Date normalisation — handles 8+ formats including partial dates and circa
# -----------------------------------------------------------------------------
DATE_PATTERNS_FULL = [
    "%d %B %Y",        # 28 March 1944
    "%d-%b-%Y",        # 15-Jun-1947
    "%B %d %Y",        # August 12 1944
    "%d/%m/%Y",        # 30/07/1943 (UK convention)
    "%d %B %Y",        # 22 September 1948
    "%d-%b-%y",        # 23-Jun-43
    "%Y-%m-%d",        # 2019-08-12
]
DATE_PATTERNS_MONTH = [
    "%B %Y",           # March 1944
    "%b %Y",           # Mar 1944
    "%m/%Y",           # 03/1944
    "%Y-%m",           # 1944-03
]
DATE_PATTERNS_YEAR = [
    "%Y",              # 1944
]
CIRCA_RE = re.compile(r"^(c\.?|circa|ca\.?)\s+", re.IGNORECASE)


def normalise_date(raw: str) -> tuple[str, str, str | None]:
    """Convert messy date text to (iso_value, xsd_type_localname, uncertainty).

    xsd_type_localname is one of: 'date', 'gYearMonth', 'gYear'.
    uncertainty is 'approximate' for circa-prefixed dates, else None.
    """
    raw = raw.strip()
    uncertainty = None
    circa_match = CIRCA_RE.match(raw)
    if circa_match:
        uncertainty = "approximate"
        raw = raw[circa_match.end():].strip()

    # Day precision
    for pattern in DATE_PATTERNS_FULL:
        try:
            dt = datetime.strptime(raw, pattern)
            if dt.year < 1900:
                dt = dt.replace(year=dt.year + 1900)
            return dt.strftime("%Y-%m-%d"), "date", uncertainty
        except ValueError:
            continue

    # Month precision
    for pattern in DATE_PATTERNS_MONTH:
        try:
            dt = datetime.strptime(raw, pattern)
            return dt.strftime("%Y-%m"), "gYearMonth", uncertainty
        except ValueError:
            continue

    # Year precision
    for pattern in DATE_PATTERNS_YEAR:
        try:
            dt = datetime.strptime(raw, pattern)
            return dt.strftime("%Y"), "gYear", uncertainty
        except ValueError:
            continue

    raise ValueError(f"Could not parse date: {raw!r}")


def feet_to_metres(feet_str: str) -> float:
    return round(float(feet_str) * 0.3048, 1)


def validate_coords(lat: float, lon: float) -> None:
    if not -90 <= lat <= 90:
        raise ValueError(f"latitude {lat} out of valid range (-90, 90)")
    if not -180 <= lon <= 180:
        raise ValueError(f"longitude {lon} out of valid range (-180, 180)")


def safe_id(text: str) -> str:
    return re.sub(r"[^a-zA-Z0-9]+", "-", text.strip()).strip("-")


def point_to_footprint_wkt(lat: float, lon: float, half_size: float = 0.025) -> str:
    """Fallback footprint when altitude/focal-length unknown."""
    minx, maxx = lon - half_size, lon + half_size
    miny, maxy = lat - half_size, lat + half_size
    return (
        f"POLYGON(({minx:.4f} {miny:.4f}, {maxx:.4f} {miny:.4f}, "
        f"{maxx:.4f} {maxy:.4f}, {minx:.4f} {maxy:.4f}, {minx:.4f} {miny:.4f}))"
    )


def derive_fov_footprint(
    lat: float,
    lon: float,
    altitude_m: float,
    focal_length_mm: float,
    image_edge_mm: float = 230.0,
) -> str:
    """Vertical-photography footprint from camera geometry.

    Formula: ground_distance_per_edge = (image_edge_mm × altitude_m) / focal_length_mm
    """
    ground_edge_m = (image_edge_mm * altitude_m) / focal_length_mm
    half_edge_m = ground_edge_m / 2.0
    deg_per_m_lat = 1.0 / 111_111.0
    deg_per_m_lon = 1.0 / (111_111.0 * math.cos(math.radians(lat)))
    half_lat = half_edge_m * deg_per_m_lat
    half_lon = half_edge_m * deg_per_m_lon
    return (
        f"POLYGON(({lon-half_lon:.6f} {lat-half_lat:.6f}, "
        f"{lon+half_lon:.6f} {lat-half_lat:.6f}, "
        f"{lon+half_lon:.6f} {lat+half_lat:.6f}, "
        f"{lon-half_lon:.6f} {lat+half_lat:.6f}, "
        f"{lon-half_lon:.6f} {lat-half_lat:.6f}))"
    )


def parse_collection_code(sortie_ref: str) -> tuple[str, str]:
    """Split sortie ref into (collection_code, sortie_local).

    Returns ("", sortie_ref) if the prefix isn't a known collection.
    """
    KNOWN = {"RAF", "NARA", "USAAF", "USAF", "USN", "DOS", "JARIC", "OS",
             "HEXAGON", "CORONA", "GAMBIT", "ZENIT", "NCAP", "Luftwaffe", "BCM"}
    parts = sortie_ref.split("/", 1)
    if len(parts) == 2 and parts[0] in KNOWN:
        return parts[0], parts[1]
    return "", sortie_ref


def derive_focal_length_mm(camera_str: str, csv_focal: str = "") -> float | None:
    """Derive focal length from camera description if not in CSV."""
    if csv_focal.strip():
        try:
            return float(csv_focal)
        except ValueError:
            pass
    if not camera_str.strip():
        return None
    m = re.search(r"(\d+)[-\s]?inch", camera_str)
    if m:
        return float(m.group(1)) * 25.4
    m = re.search(r"\bF\.?(\d+)\b", camera_str)
    if m:
        cam_focal = {"49": 152.4, "52": 914.4, "63": 304.8}
        return cam_focal.get(m.group(1))
    return None


def emit_prologue() -> str:
    return """@prefix rdf:     <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs:    <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd:     <http://www.w3.org/2001/XMLSchema#> .
@prefix dcterms: <http://purl.org/dc/terms/> .
@prefix dctype:  <http://purl.org/dc/dcmitype/> .
@prefix geo:     <http://www.opengis.net/ont/geosparql#> .
@prefix naph:    <https://w3id.org/naph/ontology#> .
@prefix ex:      <https://w3id.org/naph/example/ingest/> .

# =============================================================================
# Generated by NAPH ingest pipeline from legacy NCAP-style CSV.
# All photographs lifted to Baseline tier compliance.
# =============================================================================

ex:NCAP a naph:CustodialInstitution ;
    rdfs:label "National Collection of Aerial Photography" .

ex:NCAPCollection a naph:Collection ;
    rdfs:label "NCAP Holdings" ;
    naph:custodian ex:NCAP .

"""


def emit_rights() -> str:
    out = []
    for key, (uri, label, slug) in RIGHTS_MAPPING.items():
        out.append(
            f"ex:rights-{slug} a naph:RightsStatement ;\n"
            f'    naph:rightsURI <{uri}> ;\n'
            f'    naph:rightsLabel "{label}" .\n'
        )
    return "\n".join(out) + "\n"


def emit_record(row: dict, errors: list) -> str:
    sortie_id = safe_id(row["sortie_ref"])
    frame_no = row["frame_no"].strip()
    photo_id = f"{sortie_id}-{frame_no}"

    try:
        iso_date, date_type, date_uncertainty = normalise_date(row["date_text"])
    except ValueError as e:
        errors.append(f"{photo_id}: {e}")
        return ""

    try:
        altitude_m = feet_to_metres(row["altitude_ft"])
    except (ValueError, KeyError):
        altitude_m = None

    focal_length_mm = derive_focal_length_mm(
        row.get("camera", ""), row.get("focal_length_mm", "")
    )

    rights_text = row["rights_text"].strip()
    if rights_text not in RIGHTS_MAPPING:
        errors.append(f"{photo_id}: unmapped rights text {rights_text!r}")
        return ""
    rights_slug = RIGHTS_MAPPING[rights_text][2]

    try:
        lat = float(row["lat"])
        lon = float(row["lon"])
        validate_coords(lat, lon)
    except ValueError as e:
        errors.append(f"{photo_id}: {e}")
        return ""

    fov_used = False
    if altitude_m is not None and focal_length_mm is not None and altitude_m > 100:
        wkt = derive_fov_footprint(lat, lon, altitude_m, focal_length_mm)
        fov_used = True
    else:
        wkt = point_to_footprint_wkt(lat, lon)

    collection_code, sortie_local = parse_collection_code(row["sortie_ref"])
    location_label = row["location"].strip().strip('"')

    out = []
    out.append(f"# Sortie {row['sortie_ref']} — frame {frame_no}")

    sortie_lines = [f"ex:sortie-{sortie_id} a naph:Sortie ;"]
    sortie_lines.append(f'    naph:sortieReference "{row["sortie_ref"]}" ;')
    if collection_code:
        sortie_lines.append(f'    naph:collectionCode "{collection_code}" ;')
    if row.get("squadron", "").strip() and row["squadron"].strip() != "—":
        sortie_lines.append(f'    naph:squadron "{row["squadron"].strip()}" ;')
    if row.get("aircraft", "").strip():
        sortie_lines.append(f'    naph:aircraft "{row["aircraft"].strip()}" ;')
    sortie_lines[-1] = sortie_lines[-1].rstrip(" ;") + " ."
    out.append("\n".join(sortie_lines))

    photo_lines = [
        f"ex:photo-{photo_id} a naph:AerialPhotograph ;",
        f"    dcterms:type dctype:StillImage ;",
        f'    rdfs:label "{location_label} frame {frame_no}" ;',
        f'    naph:hasIdentifier "https://w3id.org/naph/photo/{photo_id}" ;',
        f"    naph:frameNumber {frame_no} ;",
        f"    naph:partOfSortie ex:sortie-{sortie_id} ;",
        f"    naph:belongsToCollection ex:NCAPCollection ;",
        f'    naph:capturedOn "{iso_date}"^^xsd:{date_type} ;',
    ]
    if date_uncertainty:
        photo_lines.append(f'    naph:dateUncertainty "{date_uncertainty}" ;')
    photo_lines.extend([
        f"    naph:coversArea ex:footprint-{photo_id} ;",
        f"    naph:hasRightsStatement ex:rights-{rights_slug} ;",
        f"    naph:hasCaptureEvent ex:capture-{photo_id} ;",
        f"    naph:hasDigitalSurrogate ex:surrogate-{photo_id} ;",
        f"    naph:compliesWithTier naph:TierBaseline .",
    ])
    out.append("\n".join(photo_lines))

    fp_lines = [f"ex:footprint-{photo_id} a naph:GeographicFootprint ;"]
    fp_lines.append(f'    naph:asWKT "{wkt}"^^geo:wktLiteral ;')
    if fov_used:
        fp_lines.append(
            f'    rdfs:comment "Derived from FOV: '
            f'altitude={altitude_m:.0f}m, focal_length={focal_length_mm:.0f}mm" .'
        )
    else:
        fp_lines.append(
            f'    rdfs:comment "Point-with-buffer fallback (FOV inputs unavailable)" .'
        )
    out.append("\n".join(fp_lines))

    capture_lines = [f"ex:capture-{photo_id} a naph:CaptureEvent ;"]
    if altitude_m is not None:
        capture_lines.append(f"    naph:flightAltitude {altitude_m} ;")
    if focal_length_mm is not None:
        capture_lines.append(f"    naph:focalLength {focal_length_mm} ;")
    if row.get("camera", "").strip():
        capture_lines.append(f'    naph:cameraType "{row["camera"].strip()}" ;')
    capture_lines[-1] = capture_lines[-1].rstrip(" ;") + " ."
    out.append("\n".join(capture_lines))

    try:
        scan_iso, scan_type, _ = normalise_date(row["scan_date"])
    except (ValueError, KeyError):
        scan_iso, scan_type = None, None

    surrogate_lines = [f"ex:surrogate-{photo_id} a naph:DigitalSurrogate ;"]
    if scan_iso:
        surrogate_lines.append(
            f'    naph:digitisedOn "{scan_iso}"^^xsd:{scan_type} ;'
        )
    if row.get("scan_dpi", "").strip():
        surrogate_lines.append(f'    naph:scanResolution {row["scan_dpi"].strip()} ;')
    if row.get("scan_format", "").strip():
        fmt_map = {"TIFF": "image/tiff", "JPEG": "image/jpeg", "JP2": "image/jp2"}
        fmt = fmt_map.get(row["scan_format"].strip().upper(), row["scan_format"].strip())
        surrogate_lines.append(f'    naph:fileFormat "{fmt}" ;')
    surrogate_lines.append("    naph:digitisedBy ex:NCAP .")
    out.append("\n".join(surrogate_lines))

    return "\n\n".join(out) + "\n\n"


def main():
    if len(sys.argv) != 2:
        print("usage: ingest.py <input.csv>", file=sys.stderr)
        sys.exit(2)

    csv_path = Path(sys.argv[1])
    if not csv_path.exists():
        print(f"file not found: {csv_path}", file=sys.stderr)
        sys.exit(2)

    errors = []
    parts = [emit_prologue(), emit_rights()]

    with csv_path.open(newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            parts.append(emit_record(row, errors))

    sys.stdout.write("".join(parts))

    if errors:
        print(f"\n# {len(errors)} errors during ingest:", file=sys.stderr)
        for e in errors:
            print(f"#   {e}", file=sys.stderr)
        sys.exit(1)
    else:
        with csv_path.open() as f:
            count = sum(1 for _ in csv.DictReader(f))
        print(f"# Ingest complete: {count} records transformed.", file=sys.stderr)


if __name__ == "__main__":
    main()
