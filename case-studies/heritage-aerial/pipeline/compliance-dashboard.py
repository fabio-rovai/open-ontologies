#!/usr/bin/env python3
"""
NAPH Compliance Dashboard.

Generates a self-contained HTML dashboard summarising the compliance status of
a NAPH-compliant collection. Used as the institution-facing artefact published
alongside collection metadata.

Pulls data from a running Oxigraph server (started via oxigraph_server.py) and
runs a series of dashboard queries to produce:

- Tier compliance distribution
- Date coverage histogram
- Geographic coverage summary (by country if linked)
- Collection code distribution
- Rights status distribution
- Recent validation history
- Common SHACL violation patterns (if any)

Usage:
    python3 pipeline/oxigraph_server.py start
    python3 pipeline/oxigraph_server.py reload-data
    python3 pipeline/compliance-dashboard.py > reports/compliance-dashboard.html
"""

import datetime
import json
import sys
import urllib.parse
import urllib.request


ENDPOINT = "http://localhost:7878"


def query(sparql: str) -> list[dict]:
    """Run a SPARQL query and return result rows."""
    body = json.dumps({"query": sparql}).encode()
    req = urllib.request.Request(
        f"{ENDPOINT}/api/query",
        data=body,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = json.load(resp)
            if data is None:
                return []
            return data.get("results", []) or []
    except urllib.error.HTTPError as e:
        print(f"# Query failed (HTTP {e.code}): {e.reason}", file=sys.stderr)
        return []
    except (urllib.error.URLError, ConnectionError) as e:
        print(f"# Cannot reach Oxigraph at {ENDPOINT}: {e}", file=sys.stderr)
        print(f"# Start the server with: python3 pipeline/oxigraph_server.py start", file=sys.stderr)
        return []


def strip(v: str) -> str:
    if not v:
        return ""
    if v.startswith("<") and v.endswith(">"):
        # Extract last path segment for IRI display
        return v.rstrip(">").rsplit("/", 1)[-1].rsplit("#", 1)[-1]
    if v.startswith('"'):
        end = v.rfind('"')
        if end > 0:
            return v[1:end]
    return v


def get_int(row: dict, key: str) -> int:
    raw = row.get(key, "")
    import re
    m = re.match(r'"(\d+)"', raw)
    return int(m.group(1)) if m else 0


def render_dashboard() -> str:
    timestamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d %H:%M UTC")

    # Tier distribution
    tier_rows = query("""
        PREFIX naph: <https://w3id.org/naph/ontology#>
        PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
        SELECT ?tierLabel (COUNT(?p) AS ?n)
        WHERE {
            ?p naph:compliesWithTier ?t .
            ?t rdfs:label ?tierLabel .
        }
        GROUP BY ?tierLabel
    """)
    tier_data = {strip(r["tierLabel"]): get_int(r, "n") for r in tier_rows}

    # Total records
    total_rows = query("""
        PREFIX naph: <https://w3id.org/naph/ontology#>
        SELECT (COUNT(?p) AS ?n) WHERE { ?p a naph:AerialPhotograph }
    """)
    total = get_int(total_rows[0], "n") if total_rows else 0

    # Year distribution
    year_rows = query("""
        PREFIX naph: <https://w3id.org/naph/ontology#>
        SELECT (SUBSTR(STR(?date), 1, 4) AS ?year) (COUNT(?p) AS ?n)
        WHERE {
            ?p a naph:AerialPhotograph ;
               naph:capturedOn ?date .
        }
        GROUP BY (SUBSTR(STR(?date), 1, 4))
        ORDER BY ?year
    """)
    year_data = [(strip(r["year"]), get_int(r, "n")) for r in year_rows]

    # Collection code distribution
    coll_rows = query("""
        PREFIX naph: <https://w3id.org/naph/ontology#>
        SELECT ?collectionCode (COUNT(?p) AS ?n)
        WHERE {
            ?p a naph:AerialPhotograph ;
               naph:partOfSortie ?s .
            ?s naph:collectionCode ?collectionCode .
        }
        GROUP BY ?collectionCode
        ORDER BY DESC(?n)
    """)
    coll_data = [(strip(r["collectionCode"]), get_int(r, "n")) for r in coll_rows]

    # Rights distribution
    rights_rows = query("""
        PREFIX naph: <https://w3id.org/naph/ontology#>
        SELECT ?rightsLabel (COUNT(?p) AS ?n)
        WHERE {
            ?p a naph:AerialPhotograph ;
               naph:hasRightsStatement ?r .
            ?r naph:rightsLabel ?rightsLabel .
        }
        GROUP BY ?rightsLabel
        ORDER BY DESC(?n)
    """)
    rights_data = [(strip(r["rightsLabel"]), get_int(r, "n")) for r in rights_rows]

    # Aircraft distribution (Enhanced+)
    aircraft_rows = query("""
        PREFIX naph: <https://w3id.org/naph/ontology#>
        SELECT ?aircraft (COUNT(?p) AS ?n)
        WHERE {
            ?p a naph:AerialPhotograph ;
               naph:partOfSortie ?s .
            ?s naph:aircraft ?aircraft .
        }
        GROUP BY ?aircraft
        ORDER BY DESC(?n)
    """)
    aircraft_data = [(strip(r["aircraft"]), get_int(r, "n")) for r in aircraft_rows]

    # Compute basic stats
    baseline = tier_data.get("Baseline", 0)
    enhanced = tier_data.get("Enhanced", 0)
    aspirational = tier_data.get("Aspirational", 0)
    pct = lambda n: f"{(n / total * 100):.0f}%" if total else "—"

    def render_bars(data: list[tuple], max_value: int = None) -> str:
        if not data:
            return "<p class='empty'>No data.</p>"
        if max_value is None:
            max_value = max(n for _, n in data)
        rows = []
        for label, n in data:
            width = int((n / max_value) * 100) if max_value else 0
            rows.append(
                f"<div class='bar-row'>"
                f"<span class='bar-label'>{label}</span>"
                f"<div class='bar-track'><div class='bar-fill' style='width:{width}%'>{n}</div></div>"
                f"</div>"
            )
        return "\n".join(rows)

    html = f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>NAPH Compliance Dashboard — {timestamp}</title>
<style>
  body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif;
         max-width: 1100px; margin: 0 auto; padding: 1.5em; color: #1a1a2a; line-height: 1.5;
         background: #f6f7fa; }}
  header {{ background: #1a2a3a; color: white; padding: 1.5em 2em; margin: -1.5em -1.5em 1.5em -1.5em; }}
  header h1 {{ margin: 0 0 0.4em 0; font-size: 1.6em; }}
  header p {{ margin: 0; opacity: 0.85; font-size: 0.9em; }}
  .grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 1em; }}
  .card {{ background: white; border-radius: 0.5em; padding: 1.2em; box-shadow: 0 1px 3px rgba(0,0,0,0.05); }}
  .card h2 {{ margin: 0 0 0.6em 0; font-size: 1em; color: #444; text-transform: uppercase; letter-spacing: 0.04em; }}
  .stat-large {{ font-size: 2.4em; font-weight: 600; color: #1a2a3a; line-height: 1; }}
  .stat-label {{ color: #666; font-size: 0.85em; margin-top: 0.3em; }}
  .tier-box {{ display: flex; gap: 0.8em; }}
  .tier-cell {{ flex: 1; text-align: center; padding: 0.8em 0.4em; border-radius: 0.4em;
               background: #fafafa; }}
  .tier-num {{ font-size: 1.6em; font-weight: 600; }}
  .tier-pct {{ color: #666; font-size: 0.8em; }}
  .tier-baseline .tier-num {{ color: #94a3b8; }}
  .tier-enhanced .tier-num {{ color: #4a90c2; }}
  .tier-aspirational .tier-num {{ color: #d97757; }}
  .bar-row {{ display: flex; align-items: center; gap: 0.6em; margin-bottom: 0.3em; }}
  .bar-label {{ flex: 0 0 130px; font-size: 0.8em; color: #555; text-align: right; }}
  .bar-track {{ flex: 1; background: #f0f1f4; border-radius: 0.2em; overflow: hidden; }}
  .bar-fill {{ background: #4a90c2; color: white; padding: 0.25em 0.4em; font-size: 0.75em;
              white-space: nowrap; min-width: 1.5em; text-align: right; }}
  .empty {{ color: #aaa; font-style: italic; font-size: 0.85em; }}
  footer {{ margin-top: 2em; padding-top: 1em; border-top: 1px solid #ddd;
            font-size: 0.85em; color: #888; text-align: center; }}
  footer a {{ color: #4a90c2; text-decoration: none; }}
</style>
</head>
<body>

<header>
  <h1>NAPH Compliance Dashboard</h1>
  <p>Generated <strong>{timestamp}</strong> · Data fetched from <code>{ENDPOINT}</code> · NAPH spec v1.0</p>
</header>

<div class="grid">

  <div class="card">
    <h2>Total Records</h2>
    <div class="stat-large">{total:,}</div>
    <div class="stat-label">NAPH-compliant aerial photographs</div>
  </div>

  <div class="card">
    <h2>Tier Distribution</h2>
    <div class="tier-box">
      <div class="tier-cell tier-baseline">
        <div class="tier-num">{baseline}</div>
        <div class="stat-label">Baseline</div>
        <div class="tier-pct">{pct(baseline)}</div>
      </div>
      <div class="tier-cell tier-enhanced">
        <div class="tier-num">{enhanced}</div>
        <div class="stat-label">Enhanced</div>
        <div class="tier-pct">{pct(enhanced)}</div>
      </div>
      <div class="tier-cell tier-aspirational">
        <div class="tier-num">{aspirational}</div>
        <div class="stat-label">Aspirational</div>
        <div class="tier-pct">{pct(aspirational)}</div>
      </div>
    </div>
  </div>

  <div class="card">
    <h2>Year Distribution</h2>
    {render_bars(year_data)}
  </div>

  <div class="card">
    <h2>Collection Code</h2>
    {render_bars(coll_data)}
  </div>

  <div class="card">
    <h2>Rights Distribution</h2>
    {render_bars(rights_data)}
  </div>

  <div class="card">
    <h2>Aircraft (Enhanced+)</h2>
    {render_bars(aircraft_data)}
  </div>

</div>

<footer>
  Generated by <code>pipeline/compliance-dashboard.py</code> ·
  Source: <a href="{ENDPOINT}">{ENDPOINT}</a> ·
  <a href="https://w3id.org/naph/ontology">NAPH ontology</a>
</footer>

</body>
</html>
"""

    return html


def main():
    html = render_dashboard()
    sys.stdout.write(html)


if __name__ == "__main__":
    main()
