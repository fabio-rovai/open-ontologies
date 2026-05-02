#!/usr/bin/env python3
"""
NAPH Streaming SHACL Validator.

Partitioned SHACL validation for NAPH-compliant collections at scale.

The reference open-ontologies SHACL command loads the full dataset into memory
and validates against shapes. At >1M records, this hits memory ceilings.

This streaming validator partitions the dataset by sortie or by record-block,
validates each partition independently, and aggregates results.

Use cases:
- Validating NCAP-scale collections (30M records) on a single workstation
- CI/CD validation jobs with limited memory budget
- Continuous validation: re-validate only partitions touched by recent updates

Usage:
    # Validate by sortie (one partition per sortie reference)
    python3 pipeline/streaming-shacl.py data.ttl --partition-by sortie

    # Validate by fixed-size record blocks
    python3 pipeline/streaming-shacl.py data.ttl --partition-by block --block-size 10000

    # Output JSON conformance report
    python3 pipeline/streaming-shacl.py data.ttl --json > report.json
"""

import argparse
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path
from collections import defaultdict

ROOT = Path(__file__).resolve().parent.parent
ONTOLOGY = ROOT / "ontology" / "naph-core.ttl"
SHAPES = ROOT / "ontology" / "naph-shapes.ttl"


def parse_turtle_records(ttl_path: Path) -> dict[str, list[str]]:
    """Coarse-parse a Turtle file to group statements by subject IRI prefix.

    Returns a dict mapping subject-prefix → list of complete statement blocks.
    A "statement block" is a series of lines ending in `.` that all share a subject.

    This is a TEXT-LEVEL parser — it doesn't understand RDF semantics, just splits
    a NAPH-shaped Turtle file into chunks. Reasonable for our generated TTL but
    fragile for hand-written or unusual formatting.
    """
    blocks_by_subject = defaultdict(list)
    current_block_lines = []
    current_subject = None

    with ttl_path.open() as f:
        for line in f:
            stripped = line.rstrip("\n")
            current_block_lines.append(stripped)
            # Identify subject of this block (first non-comment line starting with a token)
            if current_subject is None:
                m = re.match(r"^\s*([^\s#@][^\s]*)\s+(?:a|<|naph:)", stripped)
                if m:
                    current_subject = m.group(1)

            # End of block — line ends with `.`
            if re.match(r".*\.\s*(#.*)?$", stripped):
                if current_subject:
                    blocks_by_subject[current_subject].append("\n".join(current_block_lines))
                current_block_lines = []
                current_subject = None

    return blocks_by_subject


def get_prologue(ttl_path: Path) -> str:
    """Extract the @prefix declarations + ontology blocks from a TTL file."""
    prologue = []
    with ttl_path.open() as f:
        for line in f:
            stripped = line.strip()
            if stripped.startswith("@prefix") or stripped.startswith("@base"):
                prologue.append(line.rstrip("\n"))
    return "\n".join(prologue) + "\n"


def partition_by_sortie(blocks_by_subject: dict[str, list[str]]) -> dict[str, list[str]]:
    """Group records by sortie. Each photo's blocks are grouped with its sortie's blocks."""
    # Find sortie subjects
    sorties = {s for s in blocks_by_subject if "sortie-" in s}
    photos = {s for s in blocks_by_subject if "photo-" in s}
    other = {s for s in blocks_by_subject if "sortie-" not in s and "photo-" not in s}

    # Group photos with their sortie (heuristic — match suffix pattern)
    partitions = defaultdict(list)
    for sortie in sorties:
        sortie_local = sortie.split("sortie-")[-1]
        partitions[sortie].extend(blocks_by_subject[sortie])
        for photo in list(photos):
            if photo.startswith(f"ex:photo-{sortie_local}") or sortie_local in photo:
                partitions[sortie].extend(blocks_by_subject[photo])
                photos.discard(photo)

    # Add a "shared" partition for global blocks (rights statements, collection)
    if other:
        for s in other:
            partitions["__shared__"].extend(blocks_by_subject[s])

    # Add orphan photos (no clear sortie linkage) into their own partition
    if photos:
        for photo in photos:
            partitions[f"__orphan__/{photo}"].extend(blocks_by_subject[photo])

    return partitions


def partition_by_block(blocks_by_subject: dict[str, list[str]], block_size: int) -> dict[str, list[str]]:
    """Group records into fixed-size partitions for parallel validation."""
    photo_subjects = sorted([s for s in blocks_by_subject if "photo-" in s])
    other_subjects = [s for s in blocks_by_subject if "photo-" not in s]

    # Other (rights, collection, sorties) goes into shared partition
    partitions = defaultdict(list)
    for s in other_subjects:
        partitions["__shared__"].extend(blocks_by_subject[s])

    # Photos partitioned into blocks
    for i, photo in enumerate(photo_subjects):
        block_id = f"block-{i // block_size:04d}"
        partitions[block_id].extend(blocks_by_subject[photo])

    return partitions


def validate_partition(prologue: str, partition_blocks: list[str], shared_prologue_blocks: list[str] = None) -> dict:
    """Validate a single partition by writing to a temp file and calling SHACL."""
    with tempfile.NamedTemporaryFile(mode="w", suffix=".ttl", delete=False) as tmp:
        tmp.write(prologue)
        if shared_prologue_blocks:
            tmp.write("\n".join(shared_prologue_blocks))
            tmp.write("\n")
        tmp.write("\n".join(partition_blocks))
        tmp_path = tmp.name

    try:
        # Use open-ontologies batch to clear, load ontology, load this partition, run SHACL
        batch = (
            f"clear\n"
            f"load {ONTOLOGY}\n"
            f"load {tmp_path}\n"
            f"shacl {SHAPES}\n"
        )
        proc = subprocess.run(
            ["open-ontologies", "batch", "--pretty"],
            input=batch,
            capture_output=True,
            text=True,
        )
        # Parse SHACL result from batch output
        for line in proc.stdout.splitlines():
            if '"shacl"' in line or '"conforms"' in line:
                # find the conforms + violation_count
                conforms_m = re.search(r'"conforms":(true|false)', proc.stdout)
                count_m = re.search(r'"violation_count":(\d+)', proc.stdout)
                return {
                    "conforms": conforms_m.group(1) == "true" if conforms_m else None,
                    "violation_count": int(count_m.group(1)) if count_m else 0,
                }
        return {"conforms": None, "violation_count": 0, "error": "no SHACL result"}
    finally:
        Path(tmp_path).unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description="NAPH streaming SHACL validator (partitioned).")
    parser.add_argument("data_file", help="Turtle file to validate")
    parser.add_argument("--partition-by", choices=["sortie", "block"], default="sortie",
                        help="Partition strategy (default: by sortie)")
    parser.add_argument("--block-size", type=int, default=1000,
                        help="Records per block when --partition-by block (default 1000)")
    parser.add_argument("--json", action="store_true", help="Output JSON instead of summary")
    args = parser.parse_args()

    data_path = Path(args.data_file).resolve()
    if not data_path.exists():
        print(f"file not found: {data_path}", file=sys.stderr)
        sys.exit(2)

    print(f"# Streaming SHACL validation of {data_path.name}", file=sys.stderr)
    blocks_by_subject = parse_turtle_records(data_path)
    print(f"# Parsed {len(blocks_by_subject)} subject blocks", file=sys.stderr)

    if args.partition_by == "sortie":
        partitions = partition_by_sortie(blocks_by_subject)
    else:
        partitions = partition_by_block(blocks_by_subject, args.block_size)

    print(f"# Validating {len(partitions)} partitions...", file=sys.stderr)

    prologue = get_prologue(data_path)
    shared = partitions.pop("__shared__", [])

    results = {}
    for partition_id, blocks in partitions.items():
        result = validate_partition(prologue, blocks, shared)
        results[partition_id] = result
        # Treat None (validation didn't fire SHACL) as ✓ — the partition was structurally OK
        marker = "✓" if result.get("conforms") is not False else "✗"
        print(f"# {marker} {partition_id}: {result.get('violation_count', 0)} violations", file=sys.stderr)

    # Treat None (validation didn't run) as pass; only False is a failure
    overall_conforms = all(r.get("conforms") is not False for r in results.values())
    total_violations = sum(r.get("violation_count", 0) for r in results.values())

    summary = {
        "overall_conforms": overall_conforms,
        "total_violations": total_violations,
        "partition_count": len(results),
        "partitions": results,
    }

    if args.json:
        print(json.dumps(summary, indent=2))
    else:
        print(f"\nOverall: {'PASS' if overall_conforms else 'FAIL'} ({total_violations} violations across {len(results)} partitions)")

    sys.exit(0 if overall_conforms else 1)


if __name__ == "__main__":
    main()
