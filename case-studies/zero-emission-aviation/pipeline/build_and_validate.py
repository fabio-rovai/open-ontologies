#!/usr/bin/env python3
"""Build the UK Zero-Emission Flight ecosystem graph (v0.2) from ecosystem.json,
validate it against the SHACL shapes (must be 0 violations), prove the
referential-integrity shape catches a planted dangling edge, RUN the competency
questions in queries/competency.rq, and export graph.ttl, graph.json (demo),
competency-results.md, metrics.json and a network PNG.

The v0.2 model is provenance-first: technology maturity is reified as a dated,
sourced TRL assessment, and every quantity carries a unit and a source.

Run: python3 pipeline/build_and_validate.py
"""
import json, os, sys, re
from rdflib import Graph, Namespace, RDF, RDFS, Literal, URIRef, XSD
from pyshacl import validate

HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ZEF = Namespace("https://tesseract.academy/ns/zef#")
PROV = Namespace("http://www.w3.org/ns/prov#")
DCT = Namespace("http://purl.org/dc/terms/")
BASE = "https://tesseract.academy/id/zef/"

TYPE_CLASS = {
    "Organisation": ZEF.Organisation, "Airport": ZEF.Airport, "Programme": ZEF.Programme,
    "Project": ZEF.Project, "Funder": ZEF.Funder, "Body": ZEF.Body,
    "Alliance": ZEF.Alliance, "Technology": ZEF.Technology, "Standard": ZEF.Standard,
}
REL = {p: ZEF[p] for p in [
    "funds", "develops", "partnerOf", "memberOf", "basedAt", "demonstratesAt",
    "usesTechnology", "leads", "regulates", "coordinates", "governedBy",
    "realisesStage", "feedsInto"]}

def uri(local):
    return URIRef(BASE + local.replace(":", "/"))

def build_graph(data):
    g = Graph()
    g.bind("zef", ZEF); g.bind("prov", PROV); g.bind("dcterms", DCT)

    for e in data["entities"]:
        s = uri(e["id"])
        g.add((s, RDF.type, ZEF.Entity))
        g.add((s, RDF.type, TYPE_CLASS[e["type"]]))
        g.add((s, RDFS.label, Literal(e["label"], datatype=XSD.string)))
        if e.get("note"): g.add((s, RDFS.comment, Literal(e["note"])))
        if e.get("source"): g.add((s, DCT.source, Literal(e["source"], datatype=XSD.string)))
        if e.get("maturityBand"): g.add((s, ZEF.maturityBand, Literal(e["maturityBand"])))
        if e.get("realisesStage"): g.add((s, ZEF.realisesStage, uri(e["realisesStage"])))
        for st in e.get("governedBy", []): g.add((s, ZEF.governedBy, uri(st)))

    for c in data.get("chainStages", []):
        s = uri(c["id"])
        g.add((s, RDF.type, ZEF.ChainStage)); g.add((s, RDFS.label, Literal(c["label"], datatype=XSD.string)))
        g.add((s, ZEF.stageOrder, Literal(int(c["order"]), datatype=XSD.integer)))
        if c.get("input"): g.add((s, ZEF.hasInput, uri(c["input"])))
        if c.get("output"): g.add((s, ZEF.hasOutput, uri(c["output"])))
    for h in data.get("hydrogenForms", []):
        s = uri(h["id"]); g.add((s, RDF.type, ZEF.HydrogenForm)); g.add((s, RDFS.label, Literal(h["label"], datatype=XSD.string)))
    for sc in data.get("scenarios", []):
        s = uri(sc["id"]); g.add((s, RDF.type, ZEF.Scenario)); g.add((s, RDFS.label, Literal(sc["label"], datatype=XSD.string)))

    for src in data.get("sources", []):
        s = uri(src["id"])
        g.add((s, RDF.type, ZEF.Source)); g.add((s, RDF.type, PROV.Entity))
        g.add((s, DCT.title, Literal(src["title"], datatype=XSD.string)))
        g.add((s, DCT.source, Literal(src["url"], datatype=XSD.string)))
        if src.get("creator"): g.add((s, DCT.creator, Literal(src["creator"])))
        if src.get("date"): g.add((s, DCT.date, Literal(src["date"])))

    for a in data.get("trlAssessments", []):
        s = uri(a["id"])
        g.add((s, RDF.type, ZEF.TRLAssessment)); g.add((s, RDF.type, PROV.Entity))
        g.add((uri(a["tech"]), ZEF.hasTRLAssessment, s))
        g.add((s, ZEF.trlValue, Literal(int(a["trl"]), datatype=XSD.integer)))
        g.add((s, ZEF.assessedOn, Literal(a["date"], datatype=XSD.date)))
        if a.get("scale"): g.add((s, ZEF.trlScale, Literal(a["scale"], datatype=XSD.string)))
        g.add((s, PROV.wasDerivedFrom, uri(a["source"])))

    for q in data.get("quantities", []):
        s = uri(q["id"])
        g.add((s, RDF.type, ZEF.Quantity)); g.add((s, RDF.type, PROV.Entity))
        g.add((s, ZEF.aboutMetric, Literal(q["metric"], datatype=XSD.string)))
        g.add((s, ZEF.numericValue, Literal(str(q["value"]), datatype=XSD.decimal)))
        g.add((s, ZEF.unit, Literal(q["unit"], datatype=XSD.string)))
        if q.get("year"): g.add((s, ZEF.forYear, Literal(str(q["year"]), datatype=XSD.gYear)))
        if q.get("scenario"): g.add((s, ZEF.underScenario, uri(q["scenario"])))
        if q.get("concerns"): g.add((uri(q["concerns"]), ZEF.hasQuantity, s))  # entity has quantity
        g.add((s, PROV.wasDerivedFrom, uri(q["source"])))

    for r in data["relations"]:
        g.add((uri(r["s"]), REL[r["p"]], uri(r["o"])))
    return g

def load_shapes_onto():
    shapes = Graph().parse(os.path.join(HERE, "shapes", "zef-shapes.ttl"), format="turtle")
    onto = Graph().parse(os.path.join(HERE, "ontology", "zef.ttl"), format="turtle")
    return shapes, onto

def run_validation(g):
    shapes, onto = load_shapes_onto()
    conforms, _, text = validate(g, shacl_graph=shapes, ont_graph=onto,
                                 inference="rdfs", abort_on_first=False)
    return conforms, text

PREFIXES = """
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
PREFIX zef: <https://tesseract.academy/ns/zef#>
PREFIX prov: <http://www.w3.org/ns/prov#>
PREFIX dcterms: <http://purl.org/dc/terms/>
"""

def run_competency(g):
    raw = open(os.path.join(HERE, "queries", "competency.rq")).read()
    blocks = re.split(r"(?m)^### ", raw)
    out = []
    for b in blocks:
        b = b.strip()
        if not b.startswith("CQ"):
            continue
        title, _, body = b.partition("\n")
        # strip any inline PREFIX lines from the block; we inject a canonical set
        q = "\n".join(l for l in body.splitlines()
                      if not l.strip().startswith("###") and not l.strip().upper().startswith("PREFIX"))
        q = PREFIXES + "\n" + q
        try:
            res = g.query(q)
            vars_ = [str(v) for v in res.vars]
            rows = [[str(x) if x is not None else "" for x in row] for row in res]
            out.append((title.strip(), vars_, rows))
        except Exception as ex:
            out.append((title.strip(), ["error"], [[str(ex)[:200]]]))
    return out

TYPE_COL = {
    "Organisation": "#2e75b6", "Airport": "#c0642a", "Programme": "#1f3864",
    "Project": "#8e44ad", "Funder": "#2f9e6f", "Body": "#555555",
    "Alliance": "#c0392b", "Technology": "#0b8a8a", "Standard": "#b8860b",
}

def export_demo_json(data, path):
    trl = {a["tech"]: a["trl"] for a in data.get("trlAssessments", [])}
    nodes = [{"id": e["id"], "label": e["label"], "type": e["type"],
              "color": TYPE_COL[e["type"]], "note": e.get("note", ""),
              "maturity": e.get("maturityBand", ""), "trl": trl.get(e["id"], "")}
             for e in data["entities"]]
    edges = [{"from": r["s"], "to": r["o"], "label": r["p"]} for r in data["relations"]]
    json.dump({"nodes": nodes, "edges": edges}, open(path, "w"), indent=1)

def render_png(data, path):
    """Clean columnar 'stakeholder map': nodes grouped into role columns with
    evenly spaced, halo-backed labels and light curved edges. Far more legible
    than a force-directed hairball at this node count."""
    import matplotlib; matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    from matplotlib.patches import Patch, FancyArrowPatch
    # role -> column index; columns are ordered left (upstream policy/funding) to right
    COL = {"Funder": 0, "Body": 0, "Programme": 1, "Project": 2,
           "Organisation": 3, "Airport": 3, "Alliance": 3, "Technology": 4, "Standard": 5}
    HEAD = {0: "Funders & bodies", 1: "Programmes", 2: "Projects",
            3: "Delivery actors", 4: "Technologies", 5: "Standards"}
    XGAP = 6.0
    ents = {e["id"]: e for e in data["entities"]}
    cols = {}
    for e in data["entities"]:
        cols.setdefault(COL[e["type"]], []).append(e)
    pos, node_col = {}, {}
    for c, items in cols.items():
        items.sort(key=lambda e: (e["type"], e["label"]))
        n = len(items)
        for i, e in enumerate(items):
            y = (n - 1) / 2.0 - i               # centre the column vertically
            pos[e["id"]] = (c * XGAP, y * 1.15)
            node_col[e["id"]] = c

    fig, ax = plt.subplots(figsize=(19, 13)); ax.axis("off")
    # edges as light curved arrows
    for r in data["relations"]:
        if r["s"] in pos and r["o"] in pos:
            x1, y1 = pos[r["s"]]; x2, y2 = pos[r["o"]]
            rad = 0.12 if node_col[r["s"]] != node_col[r["o"]] else 0.35
            ax.add_patch(FancyArrowPatch((x1, y1), (x2, y2), connectionstyle=f"arc3,rad={rad}",
                         arrowstyle="-|>", mutation_scale=8, lw=0.6, color="#c4ccd4", alpha=0.55, zorder=1))
    # nodes + labels
    for e in data["entities"]:
        x, y = pos[e["id"]]; col = TYPE_COL[e["type"]]
        ax.scatter([x], [y], s=430, c=col, edgecolors="white", linewidths=1.3, zorder=3)
        ax.text(x + 0.28, y, e["label"], va="center", ha="left", fontsize=8.2, color="#16202c", zorder=4,
                bbox=dict(facecolor="white", edgecolor="none", alpha=0.72, pad=0.6, boxstyle="round,pad=0.15"))
    # reserve headroom so the title sits clearly above the column headers
    ys = [p[1] for p in pos.values()]; ymin_n, ymax_n = min(ys), max(ys)
    ax.set_ylim(ymin_n - 1.2, ymax_n + 3.0)
    ax.set_xlim(-1.0, 5 * XGAP + 5.0)
    for c, title in HEAD.items():
        ax.text(c * XGAP - 0.3, ymax_n + 1.1, title, ha="left", va="bottom",
                fontsize=11, fontweight="bold", color="#1f3864")
    ax.text(-0.3, ymax_n + 2.4, "UK Zero-Emission Flight Ecosystem   Open, SHACL-validated reference graph   The Tesseract Academy",
            ha="left", va="bottom", fontsize=14, fontweight="bold", color="#16202c")
    ax.legend(handles=[Patch(color=cc, label=t) for t, cc in TYPE_COL.items()],
              loc="lower left", fontsize=9.5, frameon=False, ncol=3, bbox_to_anchor=(0, -0.02))
    plt.savefig(path, dpi=150, bbox_inches="tight"); plt.close()

def main():
    data = json.load(open(os.path.join(HERE, "data", "ecosystem.json")))
    g = build_graph(data)
    conforms, text = run_validation(g)
    print(f"Triples: {len(g)}  SHACL conforms (clean data): {conforms}")
    if not conforms:
        print(text); sys.exit("FAIL: clean data did not conform.")

    g2 = Graph(); [g2.add(t) for t in g]
    g2.add((uri("org:zeroavia"), ZEF.partnerOf, uri("org:does-not-exist")))
    neg_conforms, _ = run_validation(g2)
    print(f"Negative test (planted dangling edge caught): {not neg_conforms}")
    if neg_conforms: sys.exit("FAIL: validator missed the dangling edge.")

    # Competency questions
    cqs = run_competency(g)
    md = ["# Competency question results (auto-generated)\n",
          "Run by `pipeline/build_and_validate.py` against the built graph.\n"]
    for title, vars_, rows in cqs:
        md.append(f"\n## {title}\n")
        if vars_:
            md.append("| " + " | ".join(vars_) + " |")
            md.append("|" + "|".join(["---"] * len(vars_)) + "|")
            for r in rows[:40]:
                md.append("| " + " | ".join(c.split("/")[-1].replace("#", "") if c.startswith("http") else c for c in r) + " |")
            md.append(f"\n_{len(rows)} row(s)._\n")
    open(os.path.join(HERE, "competency-results.md"), "w").write("\n".join(md))
    print(f"Competency questions run: {len(cqs)} (results in competency-results.md)")

    g.serialize(os.path.join(HERE, "graph.ttl"), format="turtle")
    export_demo_json(data, os.path.join(HERE, "demo", "graph.json"))

    from collections import Counter
    by_type = Counter(e["type"] for e in data["entities"])
    metrics = {"entities": len(data["entities"]), "relations": len(data["relations"]),
               "triples": len(g), "shacl_violations": 0, "dangling_edge_caught": True,
               "trl_assessments": len(data.get("trlAssessments", [])),
               "quantities": len(data.get("quantities", [])),
               "sources": len(data.get("sources", [])),
               "chain_stages": len(data.get("chainStages", [])),
               "competency_questions": len(cqs), "by_type": dict(by_type)}
    json.dump(metrics, open(os.path.join(HERE, "metrics.json"), "w"), indent=2)
    print("Metrics:", json.dumps(metrics))
    render_png(data, os.path.join(HERE, "assets", "ecosystem-graph.png"))
    print("Exports written.")

if __name__ == "__main__":
    main()
