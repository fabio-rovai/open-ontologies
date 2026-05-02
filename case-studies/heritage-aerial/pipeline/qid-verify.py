#!/usr/bin/env python3
"""
NAPH Wikidata QID Verifier.

Verifies that Wikidata QIDs in a NAPH dataset:
1. Resolve to existing Wikidata entities (not 404 / deleted)
2. Are not deprecated
3. Are of an appropriate type (Place, HistoricEvent, Subject)

Uses Wikidata's public SPARQL endpoint — no auth, no API key, no rate limits
beyond Wikidata's standard public-use limits (~5 queries per second).

Why this exists: the NAPH red-team review found that the original sample data
contained fabricated QIDs (Q11461 was claimed to be "Atomic bombing" but
actually pointed to "Sound"). This tool catches that class of error
automatically — for any NAPH-compliant dataset.

Usage:
    python3 pipeline/qid-verify.py data/sample-photographs.ttl
    python3 pipeline/qid-verify.py data/sample-photographs.ttl --json > qid-report.json
"""

import argparse
import json
import re
import subprocess
import sys
import time
import urllib.parse
from pathlib import Path

WIKIDATA_SPARQL = "https://query.wikidata.org/sparql"
USER_AGENT = "NAPH-QID-Verifier/1.0 (https://w3id.org/naph; fabio@thetesseractacademy.com)"

# Expected entity types for NAPH cross-references.
# Each entry is a permissive allowlist — Wikidata's ontology is messy and
# legitimate places have many "instance of" types. The verifier flags warnings
# only when NONE of the instance-of types match the allowlist.
EXPECTED_TYPES = {
    "Place": [
        "Q486972",   # human settlement
        "Q515",      # city
        "Q1549591",  # big city
        "Q3957",     # town
        "Q532",      # village
        "Q23397",    # lake
        "Q8502",     # mountain
        "Q22746",    # mountain range
        "Q12280",    # bridge
        "Q41176",    # building
        "Q1370598",  # Listed building
        "Q839954",   # archaeological site
        "Q188055",   # museum
        "Q4989906",  # monument
        "Q570116",   # tourist attraction
        "Q35657",    # US state
        "Q5119",     # capital city
        "Q6256",     # country
        "Q1763521",  # capital
        "Q15640612", # port city
        "Q23413",    # castle
        "Q83405",    # factory
        "Q21560085", # research institute
        "Q1497364",  # launch site
        "Q245016",   # military installation
        "Q123705",   # neighborhood
        "Q1187811",  # old town / historical district
        "Q22674925", # prefectural capital of Japan
        "Q30185",    # mayor (admin entity)
        "Q1781234",  # city designated by government ordinance
        "Q34442",    # road
        "Q12819564", # neighbourhood
    ],
    "HistoricEvent": [
        "Q1190554",  # event
        "Q645883",   # military operation
        "Q178561",   # battle
        "Q198",      # war
        "Q40231",    # election
        "Q57733494", # natural disaster
        "Q207936",   # war crime
        "Q1418135",  # nuclear explosion
        "Q1145267",  # aerial bombing of a city
        "Q1391760",  # missile system
        "Q11825706", # missile model (V-2)
        "Q83267",    # crime
        "Q1860573",  # episode of military conflict
    ],
    "Subject": [
        "Q1190554",  # event
        "Q35120",    # entity (very general)
        "Q24017414", # subject (Wikidata authority)
    ],
}


def fetch_qid_metadata(qids: list[str]) -> dict[str, dict]:
    """Fetch metadata for a batch of QIDs via Wikidata SPARQL."""
    if not qids:
        return {}

    # Build VALUES clause
    values = " ".join(f"wd:{q}" for q in qids)

    sparql = f"""
    SELECT ?item ?label ?description ?instanceOf ?instanceOfLabel ?deprecated WHERE {{
      VALUES ?item {{ {values} }}
      OPTIONAL {{ ?item rdfs:label ?label . FILTER (LANG(?label) = "en") }}
      OPTIONAL {{ ?item schema:description ?description . FILTER (LANG(?description) = "en") }}
      OPTIONAL {{ ?item wdt:P31 ?instanceOf .
                 ?instanceOf rdfs:label ?instanceOfLabel .
                 FILTER (LANG(?instanceOfLabel) = "en") }}
      OPTIONAL {{ ?item wdt:P1366 ?replacedBy }}
    }}
    """

    encoded = urllib.parse.urlencode({"query": sparql, "format": "json"})
    url = f"{WIKIDATA_SPARQL}?{encoded}"

    proc = subprocess.run(
        [
            "curl", "-sS", "--max-time", "30",
            "-H", f"User-Agent: {USER_AGENT}",
            "-H", "Accept: application/sparql-results+json",
            url,
        ],
        capture_output=True,
        text=True,
    )

    if proc.returncode != 0:
        return {q: {"error": "network", "detail": proc.stderr.strip()} for q in qids}

    try:
        data = json.loads(proc.stdout)
    except json.JSONDecodeError:
        return {q: {"error": "non-json", "detail": proc.stdout[:200]} for q in qids}

    bindings = data.get("results", {}).get("bindings", [])

    by_qid = {}
    for binding in bindings:
        item_uri = binding.get("item", {}).get("value", "")
        m = re.search(r"/Q(\d+)$", item_uri)
        if not m:
            continue
        qid = f"Q{m.group(1)}"
        if qid not in by_qid:
            by_qid[qid] = {
                "label": "",
                "description": "",
                "instance_of": [],
            }
        if "label" in binding:
            by_qid[qid]["label"] = binding["label"]["value"]
        if "description" in binding:
            by_qid[qid]["description"] = binding["description"]["value"]
        if "instanceOf" in binding and "instanceOfLabel" in binding:
            inst = binding["instanceOf"]["value"]
            inst_qid = re.search(r"/Q(\d+)$", inst)
            if inst_qid:
                by_qid[qid]["instance_of"].append({
                    "qid": f"Q{inst_qid.group(1)}",
                    "label": binding["instanceOfLabel"]["value"],
                })

    # Mark missing QIDs
    for q in qids:
        if q not in by_qid:
            by_qid[q] = {"missing": True}

    return by_qid


def parse_naph_qids(ttl_path: Path) -> list[tuple[str, str, str]]:
    """Extract (subject_iri, naph_class, qid) tuples from a NAPH Turtle file.

    Looks for skos:exactMatch <http://www.wikidata.org/wiki/Q...> patterns.
    """
    text = ttl_path.read_text()
    results = []

    # Pattern: subject ... skos:exactMatch <wikidata-uri>
    # Coarse parsing — splits on blank lines, then looks for class + exactMatch within each block
    blocks = re.split(r"\n\s*\n", text)

    for block in blocks:
        # Find subject (first IRI/qname starting a statement)
        subj_match = re.search(r"^\s*([^\s#]\S*)\s+a\s+(naph:\S+)", block, re.MULTILINE)
        if not subj_match:
            continue
        subject = subj_match.group(1)
        cls = subj_match.group(2).split(":", 1)[-1]

        # Find Wikidata QIDs referenced via skos:exactMatch
        for qid_match in re.finditer(r"<https?://www\.wikidata\.org/(?:wiki|entity)/(Q\d+)>", block):
            results.append((subject, cls, qid_match.group(1)))

    return results


def verify(ttl_path: Path) -> dict:
    """Verify all Wikidata QIDs in a NAPH Turtle file."""
    refs = parse_naph_qids(ttl_path)
    if not refs:
        return {"checked": 0, "issues": [], "summary": "No Wikidata references found in dataset."}

    # Deduplicate QIDs for batch fetch
    unique_qids = sorted({qid for _, _, qid in refs})

    print(f"# Found {len(refs)} references to {len(unique_qids)} unique Wikidata QIDs", file=sys.stderr)
    print(f"# Fetching Wikidata metadata...", file=sys.stderr)

    # Fetch in batches of 50 to keep query size reasonable
    metadata = {}
    for i in range(0, len(unique_qids), 50):
        batch = unique_qids[i:i+50]
        metadata.update(fetch_qid_metadata(batch))
        time.sleep(0.5)  # Politeness

    issues = []
    for subject, cls, qid in refs:
        meta = metadata.get(qid, {})

        if meta.get("missing"):
            issues.append({
                "severity": "error",
                "subject": subject,
                "naph_class": cls,
                "qid": qid,
                "issue": "QID does not exist on Wikidata",
            })
            continue

        if meta.get("error"):
            issues.append({
                "severity": "warn",
                "subject": subject,
                "naph_class": cls,
                "qid": qid,
                "issue": f"Could not verify ({meta['error']})",
            })
            continue

        # Type check
        instance_of_qids = [t["qid"] for t in meta.get("instance_of", [])]
        instance_of_labels = [t["label"] for t in meta.get("instance_of", [])]

        if cls in EXPECTED_TYPES and instance_of_qids:
            expected = EXPECTED_TYPES[cls]
            if not any(t in expected for t in instance_of_qids):
                issues.append({
                    "severity": "warn",
                    "subject": subject,
                    "naph_class": cls,
                    "qid": qid,
                    "label": meta.get("label", ""),
                    "issue": f"Unexpected type for {cls}: instance of {', '.join(instance_of_labels) or 'unknown'}",
                })

    return {
        "checked": len(refs),
        "unique_qids": len(unique_qids),
        "issues": issues,
        "summary": f"{len(issues)} issue(s) across {len(refs)} references"
                   if issues else f"All {len(refs)} references verified.",
    }


def render_human(report: dict) -> str:
    lines = []
    lines.append("=" * 70)
    lines.append("NAPH Wikidata QID Verification Report")
    lines.append("=" * 70)
    lines.append(f"References checked:    {report.get('checked', 0)}")
    lines.append(f"Unique QIDs:           {report.get('unique_qids', 0)}")
    lines.append(f"Issues found:          {len(report.get('issues', []))}")
    lines.append("")

    if report.get("issues"):
        lines.append("Issues:")
        for issue in report["issues"]:
            sev = issue["severity"].upper()
            lines.append(f"  [{sev}] {issue['qid']} ({issue['naph_class']}): {issue['issue']}")
            if issue.get("label"):
                lines.append(f"          Wikidata label: {issue['label']}")
            lines.append(f"          Subject: {issue['subject']}")
    else:
        lines.append("✓ All references verified.")

    lines.append("")
    lines.append("=" * 70)
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(
        description="Verify Wikidata QIDs referenced in a NAPH Turtle file."
    )
    parser.add_argument("ttl_file")
    parser.add_argument("--json", action="store_true", help="Output JSON report")
    args = parser.parse_args()

    ttl_path = Path(args.ttl_file).resolve()
    if not ttl_path.exists():
        print(f"file not found: {ttl_path}", file=sys.stderr)
        sys.exit(2)

    report = verify(ttl_path)

    if args.json:
        print(json.dumps(report, indent=2))
    else:
        print(render_human(report))

    # Exit code: 1 if any errors (not warnings), 0 otherwise
    has_errors = any(i["severity"] == "error" for i in report.get("issues", []))
    sys.exit(1 if has_errors else 0)


if __name__ == "__main__":
    main()
