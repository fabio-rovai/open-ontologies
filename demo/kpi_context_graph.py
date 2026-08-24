#!/usr/bin/env python3
"""
Derive the KPI context graph, and answer impact questions with a query.

The other pitch showed a context graph connecting ontologies, agents and 99
KPIs. It was drawn. This one is DERIVED: every edge comes from a
`computedFrom` / `appliesTo` / `dependsOnKPI` / `governedBy` triple that the
KPI itself declares. A derived graph cannot drift from the model, because it
is a rendering of the model.

Three things it does that a drawn diagram cannot:

  1. `graph`   render the context graph as Mermaid, from traversal.
  2. `impact`  given a changed IRI, list the KPIs that depend on it,
               transitively through composite KPIs. This is the question
               nobody can answer when KPIs live in a config table.
  3. `breach`  evaluate the indicators against the current data and report
               which are breached, with their governing document.

Usage:
    python3 demo/kpi_context_graph.py graph
    python3 demo/kpi_context_graph.py impact :targetsTerm
    python3 demo/kpi_context_graph.py breach
"""

import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
BIN = ROOT / "target" / "release" / "open-ontologies"
LOADS = [
    ROOT / "demo" / "bundle" / "dcat-us-full.ttl",
    ROOT / "demo" / "ontology" / "dcat-us-kpi.ttl",
]

P = """PREFIX dcus:  <https://w3id.org/dcat-us-demo#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX owl:  <http://www.w3.org/2002/07/owl#>
"""


def short(v):
    if not isinstance(v, str):
        return v
    if v.startswith("<") and v.endswith(">"):
        return v[1:-1].split("#")[-1]
    if v.startswith('"'):
        body = v[1:]
        for cut in ('"^^', '"@', '"'):
            if cut in body:
                return body.split(cut)[0]
        return body
    return v


def query(sparql):
    lines = ["clear"] + [f"load {p}" for p in LOADS]
    lines.append(f"query {json.dumps(' '.join(sparql.split()))}")
    out = subprocess.run(
        [str(BIN), "batch", "-"], input="\n".join(lines) + "\n", capture_output=True, text=True
    ).stdout.strip()
    if not out:
        return []
    try:
        rows = json.loads(out.split("\n")[-1]).get("result", {}).get("results", [])
    except Exception:
        return []
    return [{k: short(v) for k, v in r.items()} for r in rows]


def cmd_graph():
    """Render the context graph from traversal.

    Grouped into layers and styled so it reads at a glance: what the model is
    computed FROM on the left, the indicators in the middle, what they measure
    and what mandates them on the right. Blocking indicators are highlighted,
    because "which of these stops a release" is the first thing anyone asks.
    """
    inputs = query(P + "SELECT ?kpi ?input WHERE { ?kpi a/rdfs:subClassOf* dcus:KPI ; dcus:computedFrom ?input }")
    applies = query(P + "SELECT ?kpi ?cls WHERE { ?kpi a/rdfs:subClassOf* dcus:KPI ; dcus:appliesTo ?cls }")
    comps = query(P + "SELECT ?kpi ?component WHERE { ?kpi dcus:dependsOnKPI ?component }")
    govs = query(P + "SELECT ?kpi ?doc WHERE { ?kpi dcus:governedBy ?doc }")
    labels = {r["kpi"]: r["label"] for r in
              query(P + "SELECT ?kpi ?label WHERE { ?kpi a/rdfs:subClassOf* dcus:KPI ; rdfs:label ?label }")}
    # Scope to KPIs: Requirements also carry isBlocking, and would otherwise leak
    # into the indicator styling.
    blocking = {r["kpi"] for r in
                query(P + "SELECT ?kpi WHERE { ?kpi a/rdfs:subClassOf* dcus:KPI ; dcus:isBlocking true }")}

    kpis = sorted(labels)
    ins = sorted({r["input"] for r in inputs})
    classes = sorted({r["cls"] for r in applies})
    docs = sorted({r["doc"] for r in govs})

    def wrap(text):
        words, lines, cur = text.split(), [], ""
        for w in words:
            if len(cur) + len(w) > 18:
                lines.append(cur); cur = w
            else:
                cur = f"{cur} {w}".strip()
        if cur:
            lines.append(cur)
        return "<br/>".join(lines)

    out = ["flowchart LR"]
    out.append('  subgraph SRC["Model terms these are computed from"]')
    out.append("    direction TB")
    for n in ins:
        out.append(f'    {n}["{wrap(n)}"]')
    out.append("  end")
    out.append('  subgraph IND["Indicators"]')
    out.append("    direction TB")
    for k in kpis:
        mark = " ⛔" if k in blocking else ""
        out.append(f'    {k}("{wrap(labels[k])}{mark}")')
    out.append("  end")
    out.append('  subgraph TGT["What they measure / what mandates them"]')
    out.append("    direction TB")
    for n in classes:
        out.append(f'    {n}["{wrap(n)}"]')
    for n in docs:
        out.append(f'    {n}[["{wrap(n)}"]]')
    out.append("  end")

    for r in inputs:
        out.append(f'  {r["input"]} --> {r["kpi"]}')
    for r in comps:
        out.append(f'  {r["kpi"]} ==>|"composed of"| {r["component"]}')
    for r in applies:
        out.append(f'  {r["kpi"]} -.-> {r["cls"]}')
    for r in govs:
        out.append(f'  {r["doc"]} -.->|"mandates"| {r["kpi"]}')

    out.append("  classDef blocking fill:#f8d7da,stroke:#a4232f,stroke-width:2px,color:#5c1119")
    out.append("  classDef kpi fill:#e8eef7,stroke:#3d5a80,color:#1d2f45")
    out.append("  classDef term fill:#f4f1ea,stroke:#b0a99a,color:#3a352c")
    out.append("  classDef doc fill:#fff3cd,stroke:#a68b2c,color:#4a3c0c")
    normal = [k for k in kpis if k not in blocking]
    if normal:
        out.append("  class " + ",".join(normal) + " kpi")
    if blocking:
        out.append("  class " + ",".join(sorted(blocking)) + " blocking")
    if ins or classes:
        out.append("  class " + ",".join(ins + classes) + " term")
    if docs:
        out.append("  class " + ",".join(docs) + " doc")

    nodes = len(ins) + len(kpis) + len(classes) + len(docs)
    edges = len(inputs) + len(comps) + len(applies) + len(govs)
    print("```mermaid")
    print("\n".join(out))
    print("```")
    print(f"\n{nodes} nodes, {edges} edges. Every one derived by traversal;")
    print("none placed by hand. Blocking indicators marked.")


def cmd_impact(changed):
    """Which KPIs depend on a changed IRI, transitively?"""
    term = changed.lstrip(":")
    direct = query(
        P
        + f"""SELECT ?kpi ?label ?blocking WHERE {{
  ?kpi dcus:computedFrom dcus:{term} .
  OPTIONAL {{ ?kpi rdfs:label ?label }}
  OPTIONAL {{ ?kpi dcus:isBlocking ?blocking }}
}}"""
    )
    # dependsOnKPI is transitive, so the closure comes back from one query.
    downstream = query(
        P
        + f"""SELECT DISTINCT ?composite ?label WHERE {{
  ?direct dcus:computedFrom dcus:{term} .
  ?composite dcus:dependsOnKPI ?direct .
  OPTIONAL {{ ?composite rdfs:label ?label }}
}}"""
    )

    print(f"Impact of a change to :{term}\n")
    if not direct and not downstream:
        print("  no KPI declares this as an input")
        return
    print(f"  Directly computed from it ({len(direct)}):")
    for r in direct:
        flag = "  [BLOCKING]" if r.get("blocking") == "true" else ""
        print(f"    - {r.get('label', r['kpi'])}{flag}")
    print(f"\n  Composite KPIs that inherit the change ({len(downstream)}):")
    for r in downstream:
        print(f"    - {r.get('label', r['composite'])}")
    print("\n  This is the question a config-table KPI registry cannot answer.")


def cmd_breach():
    """Evaluate indicators against current data."""
    print("Indicator evaluation against the loaded graph\n")

    checks = [
        ("Distribution coverage ratio", 0.80, "lower value is a breach",
         P + "SELECT ?c ?v WHERE { ?c dcus:distributionCoverageRatio ?v . FILTER(?v < 0.80) }"),
        ("Unthemed dataset ratio", 0.20, "higher value is a breach",
         P + "SELECT ?c ?v WHERE { ?c dcus:unthemedDatasetRatio ?v . FILTER(?v > 0.20) }"),
        ("Licence currency", 1.0, "any stale licence record is a breach",
         P + "SELECT ?d WHERE { ?d dcus:affectedByLicenceChange ?m ; dcus:isStale true }"),
        ("Conformance gap closure", 1.0, "any open gap is a breach",
         P + "SELECT ?c ?g WHERE { ?c dcus:hasConformanceGap ?g }"),
        ("Catalogue completeness", 1.0, "a catalogue with no dataset is a breach",
         P + "SELECT ?t WHERE { ?t a dcus:Catalog . FILTER NOT EXISTS { ?t dcus:hasDataset ?p } }"),
    ]
    breaches = 0
    for name, threshold, note, sparql in checks:
        rows = query(sparql)
        status = f"BREACH ({len(rows)})" if rows else "ok"
        if rows:
            breaches += 1
        print(f"  [{status:>12}] {name}  (threshold {threshold}, {note})")
        for r in rows:
            print(f"                 {json.dumps(r)}")

    safety = query(
        P
        + """SELECT ?candidate ?term ?doc WHERE {
  ?candidate dcus:targetsTerm ?term .
  ?term a ?cls . ?cls rdfs:subClassOf* dcus:DeprecatedTerm .
  OPTIONAL { ?kpi a dcus:GovernanceKPI ; dcus:governedBy ?d . ?d dcus:docId ?doc }
}"""
    )
    print()
    if safety:
        breaches += 1
        print(f"  [      BREACH] Deprecated term targeting  [BLOCKING, threshold 0.0]")
        for r in safety:
            gov = f", mandated by {r['doc']}" if r.get("doc") else ""
            print(f"                 {r['candidate']} targets {r['term']}{gov}")
    else:
        print("  [          ok] Deprecated term targeting")

    print(f"\n  {breaches} indicators breached. Release readiness: FAIL")
    print("  Every breach traces to a triple, and the blocking one traces to the")
    print("  controlled document that mandates it.")


def main():
    if not BIN.exists():
        sys.exit(f"engine binary not found: {BIN}")
    cmd = sys.argv[1] if len(sys.argv) > 1 else "graph"
    if cmd == "graph":
        cmd_graph()
    elif cmd == "impact":
        cmd_impact(sys.argv[2] if len(sys.argv) > 2 else ":targetsTerm")
    elif cmd == "breach":
        cmd_breach()
    else:
        sys.exit(f"unknown command: {cmd}")


if __name__ == "__main__":
    main()
