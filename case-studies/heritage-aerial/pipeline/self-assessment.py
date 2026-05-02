#!/usr/bin/env python3
"""
NAPH Self-Assessment Tool

Runs a NAPH-compliance self-assessment against a Turtle file, producing a
JSON conformance report and a human-readable summary.

This tool is designed for institutions to check their own data without needing
to learn SPARQL or SHACL. Outputs match the conformance report format
specified in Module F.

Usage:
    python3 self-assessment.py <data.ttl>
    python3 self-assessment.py <data.ttl> --tier baseline|enhanced|aspirational
    python3 self-assessment.py <data.ttl> --json > report.json
"""

import argparse
import json
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
ROOT = SCRIPT_DIR.parent
ONTOLOGY = ROOT / "ontology" / "naph-core.ttl"
SHAPES = ROOT / "ontology" / "naph-shapes.ttl"


def run_oo(*args: str, stdin: str = None) -> dict:
    """Run open-ontologies and parse JSON output. Returns dict or None on parse failure."""
    proc = subprocess.run(
        ["open-ontologies", *args],
        input=stdin,
        capture_output=True,
        text=True,
    )
    raw = proc.stdout
    try:
        return json.loads(raw)
    except json.JSONDecodeError:
        return {"raw": raw, "stderr": proc.stderr}


def run_batch(batch_text: str) -> list:
    """Run a batch of commands and return ordered list of result dicts."""
    proc = subprocess.run(
        ["open-ontologies", "batch", "--pretty"],
        input=batch_text,
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
    return objs


def assess(data_path: Path) -> dict:
    """Run the full assessment and return a structured result."""
    timestamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    result = {
        "spec_version": "1.0",
        "assessed_at": timestamp,
        "data_file": str(data_path),
        "checks": {},
        "tier_distribution": {},
        "violations": [],
        "summary": {"pass": True, "messages": []},
    }

    if not data_path.exists():
        result["summary"]["pass"] = False
        result["summary"]["messages"].append(f"File not found: {data_path}")
        return result

    # Step 1: syntactic validation
    syntax_result = run_oo("validate", str(data_path))
    result["checks"]["syntax_validation"] = syntax_result
    if not syntax_result.get("ok"):
        result["summary"]["pass"] = False
        result["summary"]["messages"].append(
            f"Turtle syntax invalid: {syntax_result}"
        )
        return result

    # Step 2: load ontology + data, run SHACL, gather stats
    tier_query = (
        'query "PREFIX naph: <https://w3id.org/naph/ontology#> '
        'PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#> '
        "SELECT ?tierLabel (COUNT(?photo) AS ?count) WHERE { "
        "?photo naph:compliesWithTier ?tier . ?tier rdfs:label ?tierLabel . "
        '} GROUP BY ?tierLabel"'
    )
    total_query = (
        'query "PREFIX naph: <https://w3id.org/naph/ontology#> '
        "SELECT (COUNT(?photo) AS ?total) WHERE { ?photo a naph:AerialPhotograph }"
        '"'
    )
    batch = (
        f"clear\n"
        f"load {ONTOLOGY}\n"
        f"load {data_path}\n"
        f"stats\n"
        f"shacl {SHAPES}\n"
        f"{tier_query}\n"
        f"{total_query}\n"
    )
    objs = run_batch(batch)

    # Find each result by command/seq
    stats = next((o["result"] for o in objs if o.get("command") == "stats"), {})
    shacl = next((o["result"] for o in objs if o.get("command") == "shacl"), {})
    queries = [o["result"] for o in objs if o.get("command") == "query"]

    result["checks"]["stats"] = stats
    result["checks"]["shacl_conformance"] = {
        "conforms": shacl.get("conforms"),
        "violation_count": shacl.get("violation_count", 0),
    }
    result["violations"] = shacl.get("violations", [])

    # Tier distribution
    if queries:
        for row in queries[0].get("results", []):
            label = strip_quote(row.get("tierLabel", ""))
            count_raw = row.get("count", "")
            count = int(re.match(r'"(\d+)"', count_raw).group(1)) if re.match(r'"(\d+)"', count_raw) else 0
            result["tier_distribution"][label] = count

    # Total photo count
    if len(queries) >= 2:
        for row in queries[1].get("results", []):
            total_raw = row.get("total", "")
            total = int(re.match(r'"(\d+)"', total_raw).group(1)) if re.match(r'"(\d+)"', total_raw) else 0
            result["checks"]["total_records"] = total

    # Determine overall pass
    if not shacl.get("conforms"):
        result["summary"]["pass"] = False
        result["summary"]["messages"].append(
            f"SHACL validation failed with {shacl.get('violation_count', 0)} violations"
        )
    else:
        result["summary"]["messages"].append("All SHACL shapes conform")

    return result


def strip_quote(s: str) -> str:
    """Remove SPARQL JSON literal/IRI wrapping."""
    if not s:
        return ""
    if s.startswith('"'):
        end = s.rfind('"')
        if end > 0:
            return s[1:end]
    if s.startswith("<") and s.endswith(">"):
        return s[1:-1]
    return s


def render_human_summary(report: dict) -> str:
    """Render a human-readable summary of the report."""
    lines = []
    lines.append("=" * 70)
    lines.append("NAPH Self-Assessment Report")
    lines.append("=" * 70)
    lines.append(f"Spec version:    {report.get('spec_version')}")
    lines.append(f"Assessed at:     {report.get('assessed_at')}")
    lines.append(f"Data file:       {report.get('data_file')}")
    lines.append("")

    # Overall pass/fail
    overall = "PASS" if report["summary"]["pass"] else "FAIL"
    lines.append(f"Overall result:  {overall}")
    for msg in report["summary"]["messages"]:
        lines.append(f"  · {msg}")
    lines.append("")

    # Stats
    stats = report["checks"].get("stats", {})
    lines.append("Graph statistics:")
    lines.append(f"  Triples:        {stats.get('triples', '—')}")
    lines.append(f"  Classes:        {stats.get('classes', '—')}")
    lines.append(f"  Properties:     {stats.get('properties', '—')}")
    lines.append(f"  Individuals:    {stats.get('individuals', '—')}")
    lines.append("")

    # Total records
    total = report["checks"].get("total_records", "—")
    lines.append(f"Total NAPH records: {total}")

    # Tier distribution
    tier_dist = report.get("tier_distribution", {})
    if tier_dist:
        lines.append("")
        lines.append("Tier distribution:")
        for tier in ("Baseline", "Enhanced", "Aspirational"):
            count = tier_dist.get(tier, 0)
            lines.append(f"  {tier:14s} {count}")

    # SHACL
    lines.append("")
    shacl = report["checks"].get("shacl_conformance", {})
    conforms = shacl.get("conforms")
    violation_count = shacl.get("violation_count", 0)
    lines.append(
        f"SHACL conformance: {'CONFORMS' if conforms else 'VIOLATIONS'} "
        f"({violation_count} violation{'s' if violation_count != 1 else ''})"
    )

    # Violations summary
    violations = report.get("violations", [])
    if violations:
        lines.append("")
        lines.append("Violations:")
        # Group by message
        msg_counts = {}
        for v in violations:
            msg = v.get("message", "—")
            msg_counts[msg] = msg_counts.get(msg, 0) + 1
        for msg, count in sorted(msg_counts.items(), key=lambda x: -x[1]):
            lines.append(f"  · [{count}x] {msg}")

    lines.append("")
    lines.append("=" * 70)
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(
        description="NAPH self-assessment for Turtle data files."
    )
    parser.add_argument("data_file", help="Path to Turtle data file to assess")
    parser.add_argument(
        "--json", action="store_true", help="Output JSON report instead of summary"
    )
    parser.add_argument(
        "--tier",
        choices=["baseline", "enhanced", "aspirational"],
        help="Filter assessment to a specific tier (informational; SHACL still runs all)",
    )
    args = parser.parse_args()

    data_path = Path(args.data_file).resolve()
    report = assess(data_path)

    if args.json:
        print(json.dumps(report, indent=2, ensure_ascii=False))
    else:
        print(render_human_summary(report))

    sys.exit(0 if report["summary"]["pass"] else 1)


if __name__ == "__main__":
    main()
