#!/usr/bin/env python3
"""
Build shareable artefacts from the REAL NCAP sample, through the triple store.

Loads ontology + data/real-ncap-sample.ttl into Open Ontologies, runs ONE bulk
SPARQL query for every AerialPhotograph, and emits:

  reports/real-iiif-collection.json   IIIF Presentation 3.0 Collection (demo map)
  reports/real-footprints.geojson     GeoJSON FeatureCollection (GIS / STAC-adjacent)

Running through the store (not the raw JSON) proves the real records round-trip
through the NAPH ontology and are queryable as linked data — the whole point.

Usage:
    python3 pipeline/build-real-artefacts.py
"""

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CORE = ROOT / "ontology" / "naph-core.ttl"
DATA = ROOT / "data" / "real-ncap-sample.ttl"
NAPH = "https://w3id.org/naph/ontology#"

BULK_SPARQL = f"""PREFIX naph: <{NAPH}>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX dcterms: <http://purl.org/dc/terms/>
PREFIX prov: <http://www.w3.org/ns/prov#>
SELECT ?photo ?label ?date ?ident ?sortie ?wkt ?tier ?camera ?persp ?src ?sortComment
WHERE {{
  ?photo a naph:AerialPhotograph ;
         rdfs:label ?label ;
         naph:hasIdentifier ?ident ;
         naph:capturedOn ?date ;
         naph:partOfSortie ?s ;
         naph:coversArea ?fp ;
         naph:compliesWithTier ?tier .
  ?s naph:sortieReference ?sortie .
  ?fp naph:asWKT ?wkt .
  OPTIONAL {{ ?photo naph:hasCaptureEvent ?c . ?c naph:cameraType ?camera }}
  OPTIONAL {{ ?photo naph:imagePerspective ?persp }}
  OPTIONAL {{ ?photo prov:hadPrimarySource ?src }}
  OPTIONAL {{ ?s rdfs:comment ?sortComment }}
}} ORDER BY ?date"""


def strip(v: str) -> str:
    if not v:
        return ""
    if v.startswith("<") and v.endswith(">"):
        return v[1:-1]
    if v.startswith('"'):
        end = v.rfind('"')
        if end > 0:
            return v[1:end]
    return v


def run_bulk() -> list:
    one_line = " ".join(BULK_SPARQL.split())
    batch = f'clear\nload {CORE}\nload {DATA}\nquery "{one_line}"\n'
    proc = subprocess.run(
        ["open-ontologies", "batch", "--pretty"],
        input=batch, capture_output=True, text=True,
    )
    raw, objs, depth, start = proc.stdout, [], 0, 0
    for i, c in enumerate(raw):
        if c == "{":
            if depth == 0:
                start = i
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                try:
                    objs.append(json.loads(raw[start:i + 1]))
                except json.JSONDecodeError:
                    pass
    for o in objs:
        if o.get("command") == "query":
            rows, seen = [], set()
            for r in o["result"].get("results", []):
                row = {k: strip(v) for k, v in r.items()}
                if row.get("photo") in seen:
                    continue  # collapse SPARQL join multiplicities
                seen.add(row.get("photo"))
                rows.append(row)
            return rows
    print("ERROR: no query result. stderr:\n" + proc.stderr, file=sys.stderr)
    sys.exit(1)


def parse_polygon(wkt: str):
    import re
    m = re.search(r"POLYGON\s*\(\((.*?)\)\)", wkt or "")
    if not m:
        return None
    ring = []
    for pair in m.group(1).split(","):
        parts = pair.strip().split()
        if len(parts) == 2:
            ring.append([float(parts[0]), float(parts[1])])  # [lon, lat]
    return ring or None


def tier_local(t: str) -> str:
    return t.rsplit("#", 1)[-1].replace("Tier", "") if t else "Baseline"


def build_iiif(rows: list) -> dict:
    items = []
    for r in rows:
        ident = r["ident"].rstrip("/")
        coll = (r.get("sortComment") or "").replace("Collection context: ", "")
        pairs = []

        def add(lbl, val):
            if val:
                pairs.append({"label": {"en": [lbl]}, "value": {"en": [val]}})

        add("Date captured", r.get("date", ""))
        add("Sortie", r.get("sortie", ""))
        add("Perspective", r.get("persp", ""))
        add("Camera", r.get("camera", ""))
        add("Collection context", coll)
        add("Tier compliance", tier_local(r.get("tier", "")))
        add("Geographic footprint (WKT)", r.get("wkt", ""))
        add("Source record", r.get("src", ""))
        items.append({
            "id": ident + "/manifest",
            "type": "Manifest",
            "label": {"en": [r["label"]]},
            "metadata": pairs,
            "rights": "http://rightsstatements.org/vocab/CNE/1.0/",
            "requiredStatement": {
                "label": {"en": ["Attribution"]},
                "value": {"en": ["National Collection of Aerial Photography (NCAP), Historic Environment Scotland"]},
            },
            "seeAlso": [{"id": r["photo"], "type": "Dataset", "format": "text/turtle",
                         "profile": "https://w3id.org/naph/ontology",
                         "label": {"en": ["NAPH RDF representation"]}}],
        })
    return {
        "@context": "http://iiif.io/api/presentation/3/context.json",
        "id": "https://w3id.org/naph/example/ncap-live/collection/manifest",
        "type": "Collection",
        "label": {"en": [f"NAPH — real NCAP sample ({len(items)} records, Air Photo Finder API)"]},
        "items": items,
    }


def build_geojson(rows: list) -> dict:
    feats = []
    for r in rows:
        ring = parse_polygon(r.get("wkt", ""))
        if not ring:
            continue
        feats.append({
            "type": "Feature",
            "geometry": {"type": "Polygon", "coordinates": [ring]},
            "properties": {
                "label": r["label"], "sortie": r.get("sortie", ""),
                "date": r.get("date", ""), "perspective": r.get("persp", ""),
                "camera": r.get("camera", ""), "tier": tier_local(r.get("tier", "")),
                "identifier": r["ident"], "source": r.get("src", ""),
            },
        })
    return {"type": "FeatureCollection", "features": feats}


def main():
    rows = run_bulk()
    print(f"# bulk query returned {len(rows)} photographs from the store", file=sys.stderr)
    (ROOT / "reports" / "real-iiif-collection.json").write_text(
        json.dumps(build_iiif(rows), indent=2, ensure_ascii=False))
    (ROOT / "reports" / "real-footprints.geojson").write_text(
        json.dumps(build_geojson(rows), indent=2, ensure_ascii=False))
    print(f"# wrote reports/real-iiif-collection.json and reports/real-footprints.geojson", file=sys.stderr)


if __name__ == "__main__":
    main()
