#!/usr/bin/env python3
"""
build-stac.py: NAPH STAC 1.0.0 static catalog generator.

Reads reports/real-footprints.geojson (a FeatureCollection of real NCAP
records) and emits a valid STAC 1.0.0 static catalog:

    reports/stac/catalog.json          (root Catalog)
    reports/stac/items/<id>.json       (one STAC Item per record)

Pure Python standard library only, no rdflib, no pystac, no external deps.
This is deliberate: the NAPH standard must be reproducible on any archival
workstation with a bare Python install, not a data-science environment.

STAC (SpatioTemporal Asset Catalog) is an OGC Community Standard. Binding the
NCAP archival records to STAC makes the historic-aerial footprints computable
by the entire modern geospatial-imagery toolchain (stac-browser, pystac,
stackstac, QGIS STAC plugin) without abandoning their archival provenance,
which is carried in the NAPH ontology and the RiC-O crosswalk.

Usage:
    python3 pipeline/build-stac.py
"""

import json
import os
import re
import sys

STAC_VERSION = "1.0.0"
COLLECTION_ID = "ncap-aerial"
CAP = 292

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
GEOJSON = os.path.join(ROOT, "reports", "real-footprints.geojson")
OUT_DIR = os.path.join(ROOT, "reports", "stac")
ITEMS_DIR = os.path.join(OUT_DIR, "items")

# Relative links inside the static catalog (catalog.json lives in OUT_DIR,
# items live in OUT_DIR/items).
CATALOG_HREF = "../../catalog.json"   # from an item -> root/parent catalog
SELF_ITEM_HREF = "./items/{id}.json"  # from catalog -> item
ITEM_SELF_HREF = "./{id}.json"        # an item's own self link (dir-relative)


def slugify(value):
    """Make a filesystem- and STAC-id-safe slug from an arbitrary label."""
    value = value.strip()
    value = re.sub(r"[^A-Za-z0-9]+", "-", value)
    value = re.sub(r"-+", "-", value).strip("-")
    return value or "item"


def item_id_for(props, index):
    """Derive a stable STAC Item id from the NAPH identifier or the label."""
    ident = props.get("identifier") or ""
    # https://w3id.org/naph/photo/PEGASUS-RN-H-0007-0028B -> tail slug
    if ident:
        tail = ident.rstrip("/").split("/")[-1]
        slug = slugify(tail)
        if slug and slug != "item":
            return slug
    label = props.get("label") or f"record-{index}"
    return slugify(label)


def temporal_props(date_str):
    """
    Convert an ISO date (YYYY-MM-DD, YYYY-MM, or YYYY) into STAC temporal
    properties WITHOUT manufacturing precision (ADR-0009). A full date maps to
    properties.datetime; a month- or year-precision date maps to datetime null
    with an explicit start_datetime/end_datetime interval spanning the period,
    which is STAC's own way of stating a range rather than an instant.
    """
    if not date_str:
        return None
    date_str = str(date_str).strip()
    m = re.match(r"^(\d{4})(?:-(\d{2}))?(?:-(\d{2}))?$", date_str)
    if not m:
        return None
    year, month, day = m.group(1), m.group(2), m.group(3)
    if day:
        return {"datetime": f"{year}-{month}-{day}T00:00:00Z"}
    if month:
        import calendar
        last = calendar.monthrange(int(year), int(month))[1]
        return {"datetime": None,
                "start_datetime": f"{year}-{month}-01T00:00:00Z",
                "end_datetime": f"{year}-{month}-{last:02d}T23:59:59Z"}
    return {"datetime": None,
            "start_datetime": f"{year}-01-01T00:00:00Z",
            "end_datetime": f"{year}-12-31T23:59:59Z"}


def bbox_of(coordinates):
    """Compute [minlon, minlat, maxlon, maxlat] from a GeoJSON Polygon."""
    xs = []
    ys = []
    for ring in coordinates:
        for lon, lat in ring:
            xs.append(lon)
            ys.append(lat)
    return [min(xs), min(ys), max(xs), max(ys)]


def build_item(feature, index):
    props = feature.get("properties", {}) or {}
    geometry = feature.get("geometry")
    if geometry is None or geometry.get("type") != "Polygon":
        return None, "missing or non-polygon geometry"

    iid = item_id_for(props, index)
    bbox = bbox_of(geometry["coordinates"])
    tprops = temporal_props(props.get("date"))
    if tprops is None:
        return None, "unparseable date"

    source_url = props.get("source") or ""

    stac_props = {
        **tprops,
        # NAPH-native fields, namespaced so they survive a round-trip through
        # generic STAC tooling and remain machine-addressable.
        "naph:tier": props.get("tier"),
        "naph:sortie": props.get("sortie"),
        "naph:perspective": props.get("perspective"),
        "naph:camera": props.get("camera"),
        "naph:identifier": props.get("identifier"),
        "naph:source": source_url,
        "title": props.get("label"),
        "license": "other",  # NCAP catalogue rights, see per-item licensing
    }
    # Drop keys that are None so items stay clean.
    # Drop absent optional fields, but KEEP a null datetime: STAC expresses a
    # partial-precision date as datetime null plus start/end interval bounds.
    stac_props = {k: v for k, v in stac_props.items()
                  if v is not None or k == "datetime"}

    assets = {}
    if source_url:
        assets["catalogue"] = {
            "href": source_url,
            "title": "Air Photo Finder catalogue record",
            "type": "text/html",
            "roles": ["overview", "metadata"],
        }
    # An always-present placeholder for the primary image asset (the scan is
    # access-controlled behind the NCAP catalogue; href points at the record).
    assets["image"] = {
        "href": source_url or "https://airphotofinder.ncap.org/",
        "title": "Digital surrogate (NCAP access copy)",
        "type": "image/tiff",
        "roles": ["data"],
    }

    item = {
        "type": "Feature",
        "stac_version": STAC_VERSION,
        "id": iid,
        "collection": COLLECTION_ID,
        "geometry": geometry,
        "bbox": bbox,
        "properties": stac_props,
        "links": [
            {"rel": "root", "href": CATALOG_HREF, "type": "application/json"},
            {"rel": "parent", "href": CATALOG_HREF, "type": "application/json"},
            {"rel": "self", "href": ITEM_SELF_HREF.format(id=iid),
             "type": "application/geo+json"},
            {"rel": "collection", "href": CATALOG_HREF,
             "type": "application/json"},
        ],
        "assets": assets,
    }
    return item, None


REQUIRED_ITEM_KEYS = (
    "id", "type", "stac_version", "geometry", "bbox", "properties",
    "links", "assets",
)


def validate_item(item):
    """Structural validation: fail loud on any STAC Item invariant break."""
    for k in REQUIRED_ITEM_KEYS:
        if k not in item:
            return f"missing key '{k}'"
    if item["type"] != "Feature":
        return "type != Feature"
    if item["stac_version"] != STAC_VERSION:
        return "wrong stac_version"
    if "datetime" not in item["properties"]:
        return "missing properties.datetime"
    if not isinstance(item["bbox"], list) or len(item["bbox"]) != 4:
        return "bbox not length-4"
    if not item["links"]:
        return "empty links"
    rels = {l["rel"] for l in item["links"]}
    for needed in ("root", "parent", "self"):
        if needed not in rels:
            return f"missing link rel={needed}"
    return None


def main():
    with open(GEOJSON, "r", encoding="utf-8") as fh:
        fc = json.load(fh)
    features = fc.get("features", [])[:CAP]

    os.makedirs(ITEMS_DIR, exist_ok=True)

    written = 0
    skipped = 0
    seen_ids = {}
    item_links = []

    for i, feat in enumerate(features):
        item, err = build_item(feat, i)
        if item is None:
            skipped += 1
            sys.stderr.write(f"skip[{i}]: {err}\n")
            continue

        # De-duplicate ids deterministically.
        base = item["id"]
        if base in seen_ids:
            seen_ids[base] += 1
            item["id"] = f"{base}-{seen_ids[base]}"
            for l in item["links"]:
                if l["rel"] == "self":
                    l["href"] = ITEM_SELF_HREF.format(id=item["id"])
        else:
            seen_ids[base] = 0

        verr = validate_item(item)
        if verr is not None:
            skipped += 1
            sys.stderr.write(f"invalid[{item['id']}]: {verr}\n")
            continue

        path = os.path.join(ITEMS_DIR, f"{item['id']}.json")
        with open(path, "w", encoding="utf-8") as out:
            json.dump(item, out, indent=2, ensure_ascii=False)
        written += 1
        item_links.append({
            "rel": "item",
            "href": SELF_ITEM_HREF.format(id=item["id"]),
            "type": "application/geo+json",
            "title": item["properties"].get("title", item["id"]),
        })

    catalog = {
        "type": "Catalog",
        "stac_version": STAC_VERSION,
        "id": COLLECTION_ID,
        "title": "NCAP Aerial Photography (NAPH STAC)",
        "description": (
            "Static STAC 1.0.0 catalog of real National Collection of Aerial "
            "Photography (NCAP) records, harvested from the public Air Photo "
            "Finder API and bound to the NAPH computation-ready heritage "
            "standard. Each Item carries its archival identifiers (naph:*) so "
            "the spatiotemporal-imagery view stays joined to the Records in "
            "Contexts (RiC-O) archival description via the NAPH crosswalk."
        ),
        "links": [
            {"rel": "root", "href": "./catalog.json",
             "type": "application/json"},
            {"rel": "self", "href": "./catalog.json",
             "type": "application/json"},
        ] + item_links,
    }

    catalog_path = os.path.join(OUT_DIR, "catalog.json")
    with open(catalog_path, "w", encoding="utf-8") as out:
        json.dump(catalog, out, indent=2, ensure_ascii=False)

    sys.stderr.write(
        f"STAC catalog written: {written} items "
        f"(skipped {skipped}) -> {os.path.relpath(OUT_DIR, ROOT)}\n"
    )


if __name__ == "__main__":
    main()
