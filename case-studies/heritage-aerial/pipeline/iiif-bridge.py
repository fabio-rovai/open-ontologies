#!/usr/bin/env python3
"""
IIIF Presentation API 3.0 Bridge for NAPH

Generates IIIF Presentation Manifests from NAPH-compliant aerial photograph
records. Each photograph becomes a IIIF Manifest with one Canvas, with NAPH
metadata exposed as IIIF metadata pairs.

Why this matters: IIIF is the de facto interoperability layer for cultural
heritage images. Any NAPH-compliant collection can be consumed by IIIF
viewers (Mirador, Universe Viewer), annotation tools, and computational
pipelines that expect IIIF — without lock-in to NCAP-specific tooling.

Usage:
    python3 iiif-bridge.py <photo-iri> > manifest.json
    python3 iiif-bridge.py --all > manifests-bundle.json
"""

import json
import sys
import argparse
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CORE = ROOT / "ontology" / "naph-core.ttl"
DATA = ROOT / "data" / "sample-photographs.ttl"

NAPH = "https://w3id.org/naph/ontology#"


def query_store(sparql: str) -> list:
    """Run SPARQL through open-ontologies batch and return result rows."""
    batch = f"clear\nload {CORE}\nload {DATA}\nquery \"{sparql}\"\n"
    proc = subprocess.run(
        ["open-ontologies", "batch", "--pretty"],
        input=batch,
        capture_output=True,
        text=True,
    )
    raw = proc.stdout
    objs = []
    depth = 0
    start = 0
    for i, c in enumerate(raw):
        if c == "{":
            if depth == 0:
                start = i
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                try:
                    objs.append(json.loads(raw[start : i + 1]))
                except json.JSONDecodeError:
                    pass
    for o in objs:
        if o.get("command") == "query":
            return o["result"].get("results", [])
    return []


def strip(v: str) -> str:
    """Strip SPARQL JSON literal/IRI wrapping."""
    if not v:
        return ""
    if v.startswith("<") and v.endswith(">"):
        return v[1:-1]
    if v.startswith('"'):
        end = v.rfind('"')
        if end > 0:
            return v[1:end]
    return v


def fetch_photo_metadata(iri: str) -> dict:
    """Fetch all metadata for one photo IRI as a flat dict."""
    sparql = (
        f"PREFIX naph: <{NAPH}> "
        f"PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#> "
        f"SELECT ?label ?date ?identifier ?rightsLabel ?rightsURI ?wkt ?altitude ?camera ?aircraft ?squadron ?sortie ?tier "
        f"WHERE {{ "
        f"<{iri}> rdfs:label ?label ; naph:hasIdentifier ?identifier ; naph:capturedOn ?date ; "
        f"naph:hasRightsStatement ?r ; naph:partOfSortie ?s ; naph:coversArea ?fp ; naph:compliesWithTier ?tier . "
        f"?r naph:rightsLabel ?rightsLabel ; naph:rightsURI ?rightsURI . "
        f"?fp naph:asWKT ?wkt . "
        f"?s naph:sortieReference ?sortie . "
        f"OPTIONAL {{ ?s naph:aircraft ?aircraft }} "
        f"OPTIONAL {{ ?s naph:squadron ?squadron }} "
        f"OPTIONAL {{ <{iri}> naph:hasCaptureEvent ?c . ?c naph:flightAltitude ?altitude ; naph:cameraType ?camera }} "
        f"}}"
    )
    rows = query_store(sparql)
    if not rows:
        return {}
    r = rows[0]
    return {k: strip(v) for k, v in r.items()}


def list_all_photos() -> list:
    sparql = (
        f"PREFIX naph: <{NAPH}> "
        f"PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#> "
        f"SELECT ?photo ?label WHERE {{ ?photo a naph:AerialPhotograph ; rdfs:label ?label }} ORDER BY ?photo"
    )
    return [{"iri": strip(r["photo"]), "label": strip(r["label"])} for r in query_store(sparql)]


def build_manifest(iri: str) -> dict:
    """Build a IIIF Presentation 3.0 Manifest from a NAPH photograph."""
    m = fetch_photo_metadata(iri)
    if not m:
        raise ValueError(f"No metadata for {iri}")

    # Manifest IRI based on the photograph's stable identifier
    manifest_iri = m["identifier"].rstrip("/") + "/manifest"
    canvas_iri = m["identifier"].rstrip("/") + "/canvas/1"
    annotation_iri = m["identifier"].rstrip("/") + "/anno/1"

    metadata_pairs = []

    def add(label: str, value: str):
        if value:
            metadata_pairs.append(
                {"label": {"en": [label]}, "value": {"en": [value]}}
            )

    add("Identifier", m.get("identifier", ""))
    add("Date captured", m.get("date", ""))
    add("Sortie", m.get("sortie", ""))
    add("Squadron", m.get("squadron", ""))
    add("Aircraft", m.get("aircraft", ""))
    add("Camera", m.get("camera", ""))
    if m.get("altitude"):
        add("Flight altitude", f"{m['altitude']} m")
    add("Tier compliance", m.get("tier", "").rsplit("#", 1)[-1] if "#" in m.get("tier", "") else m.get("tier", ""))
    add("Geographic footprint (WKT)", m.get("wkt", ""))

    return {
        "@context": "http://iiif.io/api/presentation/3/context.json",
        "id": manifest_iri,
        "type": "Manifest",
        "label": {"en": [m["label"]]},
        "metadata": metadata_pairs,
        "summary": {
            "en": [
                f"Aerial photograph captured {m['date']}. "
                f"NAPH tier compliance: {m.get('tier', '').rsplit('#', 1)[-1]}."
            ]
        },
        "rights": m.get("rightsURI", ""),
        "requiredStatement": {
            "label": {"en": ["Attribution"]},
            "value": {"en": [f"{m.get('rightsLabel', '')} — National Collection of Aerial Photography"]},
        },
        "items": [
            {
                "id": canvas_iri,
                "type": "Canvas",
                "label": {"en": [m["label"]]},
                "height": 4096,
                "width": 4096,
                "items": [
                    {
                        "id": canvas_iri + "/page",
                        "type": "AnnotationPage",
                        "items": [
                            {
                                "id": annotation_iri,
                                "type": "Annotation",
                                "motivation": "painting",
                                "target": canvas_iri,
                                "body": {
                                    "id": m["identifier"] + "/full/full/0/default.jpg",
                                    "type": "Image",
                                    "format": "image/jpeg",
                                    "service": [
                                        {
                                            "id": m["identifier"],
                                            "type": "ImageService3",
                                            "profile": "level2",
                                        }
                                    ],
                                },
                            }
                        ],
                    }
                ],
            }
        ],
        "seeAlso": [
            {
                "id": iri,
                "type": "Dataset",
                "format": "text/turtle",
                "profile": "https://w3id.org/naph/ontology",
                "label": {"en": ["NAPH RDF representation"]},
            }
        ],
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("photo_iri", nargs="?", help="Photograph IRI to convert")
    parser.add_argument("--all", action="store_true", help="Emit all photos as a bundle")
    parser.add_argument("--list", action="store_true", help="List photo IRIs")
    args = parser.parse_args()

    if args.list:
        for p in list_all_photos():
            print(f"{p['iri']}\t{p['label']}")
        return

    if args.all:
        manifests = []
        for p in list_all_photos():
            try:
                manifests.append(build_manifest(p["iri"]))
            except ValueError as e:
                print(f"# skip {p['iri']}: {e}", file=sys.stderr)
        json.dump({"@context": "http://iiif.io/api/presentation/3/context.json", "type": "Collection", "id": "https://w3id.org/naph/example/collection/manifest", "label": {"en": ["NAPH sample collection"]}, "items": manifests}, sys.stdout, indent=2, ensure_ascii=False)
        sys.stdout.write("\n")
        return

    if not args.photo_iri:
        parser.print_help()
        sys.exit(1)

    manifest = build_manifest(args.photo_iri)
    json.dump(manifest, sys.stdout, indent=2, ensure_ascii=False)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
