#!/usr/bin/env python3
"""
Stereo Pair Detector

Identifies overlapping stereo pairs in a NAPH-compliant aerial photograph
collection by comparing geographic footprints. Outputs RDF triples linking
each photograph to its detected stereo neighbours.

Stereo pairs in aerial reconnaissance are typically:
- Adjacent frames within the same sortie
- With ~60% along-track overlap
- Captured within seconds of each other
- Useful for stereoscopic 3D analysis and photogrammetric processing

Usage:
    python3 stereo-pair-detector.py <data.ttl> > stereo-pairs.ttl
    open-ontologies validate stereo-pairs.ttl
"""

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import NamedTuple


class FrameInfo(NamedTuple):
    """A photograph with frame number, footprint corners, and sortie reference."""
    iri: str
    frame_number: int | None
    sortie_iri: str
    polygon_corners: list[tuple[float, float]]


def run_query(data_path: Path, ontology_path: Path, sparql: str) -> list[dict]:
    """Run a SPARQL query against ontology + data, return result rows."""
    batch = (
        f"clear\n"
        f"load {ontology_path}\n"
        f"load {data_path}\n"
        f'query "{sparql}"\n'
    )
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


def strip_value(s: str) -> str:
    """Strip SPARQL JSON IRI/literal wrapping."""
    if not s:
        return ""
    if s.startswith("<") and s.endswith(">"):
        return s[1:-1]
    if s.startswith('"'):
        end = s.rfind('"')
        if end > 0:
            return s[1:end]
    return s


def parse_wkt(wkt: str) -> list[tuple[float, float]] | None:
    """Parse a POLYGON WKT into list of (lon, lat) corners."""
    match = re.search(r"POLYGON\s*\(\(([^)]+)\)\)", wkt)
    if not match:
        return None
    return [
        tuple(map(float, p.strip().split()))
        for p in match.group(1).split(",")
    ]


def polygon_overlap_fraction(
    p1: list[tuple[float, float]],
    p2: list[tuple[float, float]],
) -> float:
    """
    Compute the fractional overlap of two polygons by bounding-box approximation.

    For accurate overlap, use a proper geometry library (Shapely, GEOS).
    This is a fast approximation suitable for stereo-pair detection.

    Returns 0.0 (no overlap) to 1.0 (complete overlap).
    """
    def bbox(poly: list[tuple[float, float]]) -> tuple[float, float, float, float]:
        lons, lats = zip(*poly)
        return min(lons), min(lats), max(lons), max(lats)

    b1 = bbox(p1)
    b2 = bbox(p2)

    # Intersection bbox
    inter_minx = max(b1[0], b2[0])
    inter_miny = max(b1[1], b2[1])
    inter_maxx = min(b1[2], b2[2])
    inter_maxy = min(b1[3], b2[3])

    if inter_minx >= inter_maxx or inter_miny >= inter_maxy:
        return 0.0

    inter_area = (inter_maxx - inter_minx) * (inter_maxy - inter_miny)
    p1_area = (b1[2] - b1[0]) * (b1[3] - b1[1])

    if p1_area == 0:
        return 0.0

    return inter_area / p1_area


def fetch_frames(data_path: Path, ontology_path: Path) -> list[FrameInfo]:
    """Fetch all photograph + footprint + sortie + frame number from the data."""
    sparql = (
        "PREFIX naph: <https://w3id.org/naph/ontology#> "
        "SELECT ?photo ?frameNumber ?sortie ?wkt WHERE { "
        "?photo a naph:AerialPhotograph ; "
        "naph:partOfSortie ?sortie ; "
        "naph:coversArea ?fp . "
        "?fp naph:asWKT ?wkt . "
        "OPTIONAL { ?photo naph:frameNumber ?frameNumber } "
        "}"
    )
    rows = run_query(data_path, ontology_path, sparql)

    frames = []
    for row in rows:
        iri = strip_value(row.get("photo", ""))
        sortie_iri = strip_value(row.get("sortie", ""))
        wkt = strip_value(row.get("wkt", ""))

        frame_str = row.get("frameNumber", "")
        # Match an integer literal
        fn_match = re.match(r'"(\d+)"', frame_str)
        frame_number = int(fn_match.group(1)) if fn_match else None

        polygon = parse_wkt(wkt)
        if polygon:
            frames.append(FrameInfo(
                iri=iri,
                frame_number=frame_number,
                sortie_iri=sortie_iri,
                polygon_corners=polygon,
            ))
    return frames


def detect_stereo_pairs(
    frames: list[FrameInfo],
    min_overlap: float = 0.3,
    max_overlap: float = 0.95,
    same_sortie_only: bool = True,
) -> list[tuple[str, str, float]]:
    """
    Detect candidate stereo pairs.

    Args:
        frames: list of FrameInfo from the collection
        min_overlap: minimum overlap fraction to consider a stereo pair (default 0.3 = 30%)
        max_overlap: maximum (above this is considered too-similar/duplicate, default 0.95)
        same_sortie_only: only consider pairs from the same sortie (default True)

    Returns:
        List of (photo1_iri, photo2_iri, overlap_fraction) tuples.
    """
    pairs = []

    # Group by sortie if same_sortie_only
    if same_sortie_only:
        by_sortie: dict[str, list[FrameInfo]] = {}
        for f in frames:
            by_sortie.setdefault(f.sortie_iri, []).append(f)
        groups = list(by_sortie.values())
    else:
        groups = [frames]

    for group in groups:
        # Sort by frame number where available
        group = sorted(group, key=lambda f: (f.frame_number or 0, f.iri))
        for i, frame_i in enumerate(group):
            for frame_j in group[i+1:]:
                # Skip same frame
                if frame_i.iri == frame_j.iri:
                    continue
                # Compute overlap (one direction)
                overlap = polygon_overlap_fraction(
                    frame_i.polygon_corners,
                    frame_j.polygon_corners,
                )
                if min_overlap <= overlap <= max_overlap:
                    pairs.append((frame_i.iri, frame_j.iri, overlap))

    return pairs


def emit_turtle(pairs: list[tuple[str, str, float]]) -> str:
    """Emit RDF triples linking detected stereo pairs."""
    lines = [
        "@prefix naph: <https://w3id.org/naph/ontology#> .",
        "",
        "# Stereo pairs detected by automated overlap analysis.",
        "# Each pair represents two photographs with ~30-95% geographic overlap,",
        "# likely captured as stereo neighbours within the same sortie.",
        "",
    ]

    for photo1, photo2, overlap in pairs:
        lines.append(
            f"<{photo1}> naph:hasStereoPair <{photo2}> ;\n"
            f"    naph:stereoOverlapFraction \"{overlap:.3f}\"^^<http://www.w3.org/2001/XMLSchema#decimal> ."
        )
        lines.append("")

    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(
        description="Detect candidate stereo pairs in NAPH-compliant aerial photography."
    )
    parser.add_argument("data_file", help="Path to NAPH-compliant Turtle data file")
    parser.add_argument("--ontology", default=None,
                        help="Path to NAPH ontology (default: ../ontology/naph-core.ttl)")
    parser.add_argument("--min-overlap", type=float, default=0.3,
                        help="Minimum overlap fraction (default 0.3)")
    parser.add_argument("--max-overlap", type=float, default=0.95,
                        help="Maximum overlap fraction (default 0.95)")
    parser.add_argument("--cross-sortie", action="store_true",
                        help="Allow pairs from different sorties (default: same sortie only)")
    parser.add_argument("--report-only", action="store_true",
                        help="Print summary report instead of Turtle")
    args = parser.parse_args()

    data_path = Path(args.data_file).resolve()
    ontology_path = Path(args.ontology) if args.ontology else \
        Path(__file__).resolve().parent.parent / "ontology" / "naph-core.ttl"

    if not data_path.exists():
        print(f"data file not found: {data_path}", file=sys.stderr)
        sys.exit(2)
    if not ontology_path.exists():
        print(f"ontology file not found: {ontology_path}", file=sys.stderr)
        sys.exit(2)

    frames = fetch_frames(data_path, ontology_path)
    print(f"# Found {len(frames)} photographs with footprints", file=sys.stderr)

    pairs = detect_stereo_pairs(
        frames,
        min_overlap=args.min_overlap,
        max_overlap=args.max_overlap,
        same_sortie_only=not args.cross_sortie,
    )
    print(f"# Detected {len(pairs)} candidate stereo pairs", file=sys.stderr)

    if args.report_only:
        for p1, p2, overlap in pairs:
            print(f"{p1}\t{p2}\t{overlap:.3f}")
    else:
        print(emit_turtle(pairs))


if __name__ == "__main__":
    main()
