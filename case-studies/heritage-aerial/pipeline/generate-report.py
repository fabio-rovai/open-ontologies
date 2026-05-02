#!/usr/bin/env python3
"""
NAPH Validation Report Generator

Runs the full validation pipeline (load → SHACL → competency queries) and
emits a self-contained HTML report. Used as a reference output that
institutions can publish alongside their data to demonstrate compliance.

Usage:
    python3 generate-report.py > ../reports/validation-report.html
"""

import json
import subprocess
import sys
import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BATCH_FILE = ROOT / "docs" / "competency-queries.batch.txt"
SHACL_BATCH = """clear
load {core}
load {data}
shacl {shapes}
""".format(
    core=ROOT / "ontology" / "naph-core.ttl",
    data=ROOT / "data" / "sample-photographs.ttl",
    shapes=ROOT / "ontology" / "naph-shapes.ttl",
)


def run_batch(batch_text: str):
    """Pipe batch_text into open-ontologies batch and return parsed results."""
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


def get_shacl_result(objs):
    for o in objs:
        if o.get("command") == "shacl":
            return o["result"]
    return None


def get_query_results(objs):
    return [o["result"] for o in objs if o.get("command") == "query"]


CQ_TITLES = [
    ("CQ1", "WWII temporal range — photos captured 1939-09 to 1945-09"),
    ("CQ3", "Open rights — photos available for open-access publication"),
    ("CQ4", "Wikidata event linkage — photos linked to historic events"),
    ("CQ5", "NARA partnership audit — provenance via US National Archives"),
    ("CQ6", "Tier compliance distribution"),
    ("CQ8", "Capture-context filter — Spitfire above 6000m"),
    ("CQ7", "High-research-value subset — post-1944 + Place + open rights"),
]


def html_table(rows, headers):
    out = ['<table><thead><tr>']
    for h in headers:
        out.append(f"<th>{h}</th>")
    out.append("</tr></thead><tbody>")
    for r in rows:
        out.append("<tr>")
        for h in headers:
            v = r.get(h, "")
            v = str(v).replace("<", "&lt;").replace(">", "&gt;")
            out.append(f"<td>{v}</td>")
        out.append("</tr>")
    out.append("</tbody></table>")
    return "".join(out)


def main():
    timestamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d %H:%M UTC")

    shacl_objs = run_batch(SHACL_BATCH)
    shacl_result = get_shacl_result(shacl_objs)

    query_batch_text = BATCH_FILE.read_text()
    query_objs = run_batch(query_batch_text)
    query_results = get_query_results(query_objs)

    cq_sections = []
    for (cq_id, cq_title), result in zip(CQ_TITLES, query_results):
        rows = result.get("results", [])
        headers = result.get("variables", [])
        cq_sections.append(
            f'<section class="cq"><h3>{cq_id}: {cq_title}</h3>'
            f'<p class="meta">{len(rows)} result{"s" if len(rows) != 1 else ""}</p>'
            f"{html_table(rows, headers)}</section>"
        )

    shacl_status_class = "pass" if shacl_result and shacl_result.get("conforms") else "fail"
    shacl_status_text = "CONFORMS ✓" if shacl_result and shacl_result.get("conforms") else "VIOLATIONS"
    violation_count = shacl_result.get("violation_count", 0) if shacl_result else "—"

    html = f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>NAPH Validation Report — {timestamp}</title>
<style>
  body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
          max-width: 980px; margin: 2em auto; padding: 0 1em; color: #1a1a1a; line-height: 1.5; }}
  h1 {{ border-bottom: 2px solid #333; padding-bottom: 0.3em; }}
  h2 {{ margin-top: 2em; border-bottom: 1px solid #ccc; padding-bottom: 0.2em; }}
  h3 {{ margin-top: 1.5em; color: #444; }}
  .meta {{ color: #666; font-size: 0.9em; margin: 0.2em 0; }}
  .status {{ display: inline-block; padding: 0.2em 0.8em; border-radius: 0.3em;
             font-weight: 600; }}
  .pass {{ background: #d4f4dd; color: #1a5f2a; }}
  .fail {{ background: #fcd7d7; color: #7a1f1f; }}
  table {{ border-collapse: collapse; width: 100%; margin-top: 0.5em; }}
  th, td {{ text-align: left; padding: 0.4em 0.6em; border-bottom: 1px solid #ddd;
            font-size: 0.9em; }}
  th {{ background: #f4f4f6; font-weight: 600; }}
  td {{ word-break: break-word; }}
  .cq {{ margin-bottom: 1.5em; padding: 1em; background: #fafafa; border-left: 3px solid #4a90c2; }}
  footer {{ margin-top: 3em; color: #888; font-size: 0.85em; border-top: 1px solid #eee; padding-top: 1em; }}
</style>
</head>
<body>

<h1>NAPH Validation Report</h1>
<p class="meta">Generated <strong>{timestamp}</strong> · National Aerial Photography Heritage Ontology · v0.1</p>

<h2>Summary</h2>
<ul>
  <li>Ontology: <code>ontology/naph-core.ttl</code></li>
  <li>Shapes: <code>ontology/naph-shapes.ttl</code></li>
  <li>Sample data: <code>data/sample-photographs.ttl</code> (10 records)</li>
  <li>SHACL conformance: <span class="status {shacl_status_class}">{shacl_status_text}</span> (violations: <strong>{violation_count}</strong>)</li>
</ul>

<h2>SHACL Validation</h2>
<p class="meta">Each photograph validated against tier-specific shapes (Baseline / Enhanced / Aspirational), DigitalSurrogate quality requirements, and Place authority shape.</p>
{f'<p class="status pass">All records conform to declared tier compliance.</p>' if shacl_status_class == 'pass' else '<p class="status fail">Violations present — see details below.</p>'}

<h2>Competency Question Verification</h2>
<p class="meta">Each question is a research workflow that the standard enables. Results below are run live against the sample dataset.</p>

{''.join(cq_sections)}

<footer>
  <p><strong>NAPH</strong> — National Aerial Photography Heritage Ontology · case study under the Towards a National Collection / N-RICH Prototype framework.</p>
  <p>Built with <a href="https://github.com/fabio-rovai/open-ontologies">Open Ontologies</a>. Data illustrative; modeled on publicly known NCAP collection structure.</p>
  <p>© Kampakis and Co Ltd, trading as The Tesseract Academy. Ontology released under <a href="https://creativecommons.org/licenses/by/4.0/">CC BY 4.0</a>.</p>
</footer>

</body>
</html>
"""
    sys.stdout.write(html)


if __name__ == "__main__":
    main()
