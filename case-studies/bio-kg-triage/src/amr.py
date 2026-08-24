"""
Antimicrobial-resistance layer (#60): a validated AMR knowledge graph from CARD's
Antibiotic Resistance Ontology (ARO).

Fragmented AMR evidence is exactly the kind of data that benefits from a closed-world gate:
a resistance determinant is only meaningful if its ARO identifier is real. We extract real
"confers resistance to" relationships from ARO (resistance gene/protein -> antibiotic or drug
class), build a KG, and run the same gate, now policing the ARO namespace instead of Biolink.
It generalises the correctness gate to a third biomedical ontology and the AMR domain.

Real source only: CARD Antibiotic Resistance Ontology (aro.obo, purl.obolibrary.org/obo/aro).
"""
import json, os, re
import rdflib
from rdflib import Graph, RDF, RDFS, URIRef, Literal, Namespace
from rdflib.namespace import SH
from pyshacl import validate

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA, BUILD, RES = (os.path.join(ROOT, d) for d in ("data", "build", "results"))
OBO = "http://purl.obolibrary.org/obo/"
ARO_NS = OBO + "ARO_"
EX = Namespace("https://ex.tesseract.academy/amr/")
REL_KINDS = ("confers_resistance_to_antibiotic", "confers_resistance_to_drug_class")
CAP = 800  # deterministic slice of the confers-resistance edges, for a tidy artifact

def iri(aro_id): return ARO_NS + aro_id.replace("ARO:", "")

def parse_aro():
    terms, names, edges = set(), {}, []
    cur = None
    for line in open(os.path.join(DATA, "aro.obo"), encoding="utf-8"):
        line = line.rstrip("\n")
        if line == "[Term]":
            cur = {"id": None}
        elif line.startswith("[") and line != "[Term]":
            cur = None
        elif cur is not None:
            if line.startswith("id: ARO:"):
                cur["id"] = line[4:].strip(); terms.add(cur["id"])
            elif line.startswith("name:") and cur.get("id"):
                names[cur["id"]] = line[5:].strip()
            elif line.startswith("relationship:") and cur.get("id"):
                m = re.match(r"relationship:\s+(\S+)\s+(ARO:\d+)", line)
                if m and m.group(1) in REL_KINDS:
                    edges.append((cur["id"], m.group(1), m.group(2)))
    return terms, names, edges

def build_kg(edges, names, fabricate=False):
    g = Graph()
    for i, (sub, rel, obj) in enumerate(edges):
        s = URIRef(iri(sub))
        o = URIRef(iri("ARO:9999999")) if (fabricate and i == 0) else URIRef(iri(obj))
        g.add((s, RDF.type, URIRef(ARO_NS + "3000000")))  # ARO root-ish class (real term used as type)
        if sub in names: g.add((s, RDFS.label, Literal(names[sub])))
        g.add((s, EX.confersResistanceTo, o))
        if not (fabricate and i == 0) and obj in names:
            g.add((o, RDFS.label, Literal(names[obj])))
    return g

def gate(g, declared):
    flags = set()
    for s, p, o in g:
        for node in (s, o):
            if isinstance(node, URIRef) and str(node).startswith(ARO_NS) and str(node) not in declared:
                flags.add(str(node))
        if p == RDF.type and isinstance(o, URIRef) and str(o).startswith(ARO_NS) and str(o) not in declared:
            flags.add(str(o))
    return sorted(flags)

def shapes():
    s = Graph(); sh = URIRef("https://ex/amr/shape")
    s.add((sh, RDF.type, SH.NodeShape)); s.add((sh, SH.targetClass, URIRef(ARO_NS + "3000000")))
    b = URIRef("https://ex/amr/shape/l"); s.add((sh, SH.property, b)); s.add((b, SH.path, RDFS.label)); s.add((b, SH.minCount, Literal(1)))
    return s

def evaluate(name, g, declared):
    c, _, _ = validate(g, shacl_graph=shapes(), inference="none")
    fl = gate(g, declared)
    return {"label": name, "triples": len(g), "shacl_conforms": bool(c), "closed_world_violations": len(fl), "violations": fl}

def main():
    terms, names, edges = parse_aro()
    declared = {iri(t) for t in terms}
    total = len(edges)
    edges = sorted(edges)[:CAP]
    print(f"[*] ARO: {len(terms)} declared terms, {total} confers-resistance relationships "
          f"(using a deterministic slice of {len(edges)} for the artifact)")

    grounded = build_kg(edges, names, fabricate=False)
    ungrounded = build_kg(edges, names, fabricate=True)   # first edge points at a fabricated ARO id
    grounded.serialize(os.path.join(BUILD, "amr-kg-grounded.ttl"), format="turtle")

    r_g = evaluate("grounded AMR KG", grounded, declared)
    r_u = evaluate("ungrounded AMR KG", ungrounded, declared)

    out = {
        "source": "CARD Antibiotic Resistance Ontology (aro.obo)",
        "aro_declared_terms": len(terms), "confers_resistance_relationships_total": total,
        "edges_used": len(edges), "grounded": r_g, "ungrounded": r_u,
        "headline": {
            "grounded_closed_world_violations": r_g["closed_world_violations"],
            "ungrounded_shacl_conforms": r_u["shacl_conforms"],
            "ungrounded_caught_by_gate": r_u["closed_world_violations"] > 0,
        },
        "sample_edges": [{"determinant": names.get(s, s), "confers_resistance_to": names.get(o, o)}
                          for s, _, o in edges[:10]],
    }
    json.dump(out, open(os.path.join(RES, "results_amr.json"), "w"), indent=2)
    print(json.dumps(out["headline"], indent=2))
    print("grounded:", r_g); print("ungrounded:", r_u)

if __name__ == "__main__":
    main()
