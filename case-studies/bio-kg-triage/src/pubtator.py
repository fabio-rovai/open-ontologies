"""
Literature front end (#42, #48): turn real PubMed annotations into a validated,
Biolink-typed knowledge graph.

For each of the eight target genes, we query PubTator3 (the NLM's biomedical entity and
relation extractor), pull recent PMIDs, and export the machine-extracted gene-disease
relations. Each relation becomes a Biolink-typed edge, and the whole graph passes the same
closed-world vocabulary gate. This is the "fragmented literature -> knowledge graph" step,
grounded: nothing enters the graph unless its type and predicate are real Biolink terms.

Real source only: PubTator3 API (www.ncbi.nlm.nih.gov/research/pubtator3-api). No fabrication;
association assertions are PubTator3's, with its confidence scores.
"""
import json, os, time
import rdflib, requests
from rdflib import Graph, RDF, RDFS, URIRef, Literal
from rdflib.namespace import SH
from pyshacl import validate

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA, BUILD, RES = (os.path.join(ROOT, d) for d in ("data", "build", "results"))
BL = "https://w3id.org/biolink/vocab/"
PT = "https://www.ncbi.nlm.nih.gov/research/pubtator3-api"
NCBIGENE = "https://identifiers.org/ncbigene:"
MESH = "https://identifiers.org/mesh:"

TARGETS = ["EGFR", "TP53", "KRAS", "BRCA1", "PTEN", "BRAF", "ALK", "MYC"]
GENE_DISEASE_RELS = {"Association", "Positive_Correlation", "Negative_Correlation", "Cause"}

with open(os.path.join(DATA, "biolink_vocab.json")) as f:
    BV = json.load(f)
DECLARED = set(BV["declared"]); POLICED = BV["policed"]

def in_pol(i): return any(str(i).startswith(p) for p in POLICED)
def gate(g):
    f = set()
    for s, p, o in g:
        if in_pol(p) and str(p) not in DECLARED: f.add(str(p))
        if p == RDF.type and isinstance(o, URIRef) and in_pol(o) and str(o) not in DECLARED: f.add(str(o))
    return sorted(f)

def search_pmids(term, k=5):
    r = requests.get(f"{PT}/search/", params={"text": term, "page_size": k}, timeout=60).json()
    return [str(x["pmid"]) for x in r.get("results", [])][:k]

def export(pmids):
    r = requests.get(f"{PT}/publications/export/biocjson",
                     params={"pmids": ",".join(pmids)}, timeout=90)
    try:
        return r.json().get("PubTator3", [])
    except Exception:
        return []

def gene_disease_edges(docs):
    """Extract (gene_id, gene_name, disease_id, disease_name, reltype, score) from
    PubTator3 relations whose two roles are a Gene and a Disease."""
    edges = []
    for doc in docs:
        for rel in doc.get("relations", []):
            inf = rel.get("infons", {})
            r1, r2 = inf.get("role1", {}), inf.get("role2", {})
            pair = {r1.get("type"), r2.get("type")}
            if pair == {"Gene", "Disease"} and inf.get("type") in GENE_DISEASE_RELS:
                gene = r1 if r1["type"] == "Gene" else r2
                dis = r1 if r1["type"] == "Disease" else r2
                edges.append({
                    "gene_id": str(gene.get("normalized_id") or gene.get("identifier")),
                    "gene_name": gene.get("name") or gene.get("identifier"),
                    "disease_id": str(dis.get("identifier") or dis.get("normalized_id")),
                    "disease_name": dis.get("name") or dis.get("identifier"),
                    "reltype": inf.get("type"), "score": float(inf.get("score", 0) or 0),
                    "pmid": str(doc.get("pmid")),
                })
    return edges

def disease_iri(did):
    did = did.replace("MESH:", "").replace("mesh:", "")
    return MESH + did

def build_kg(edges, predicate):
    g = Graph()
    for e in edges:
        gene = URIRef(NCBIGENE + e["gene_id"]); dis = URIRef(disease_iri(e["disease_id"]))
        g.add((gene, RDF.type, URIRef(BL + "Gene"))); g.add((gene, RDFS.label, Literal(e["gene_name"])))
        g.add((dis, RDF.type, URIRef(BL + "Disease"))); g.add((dis, RDFS.label, Literal(e["disease_name"])))
        g.add((gene, URIRef(predicate), dis))
    return g

def shapes():
    s = Graph(); sh = URIRef("https://ex/shape")
    s.add((sh, RDF.type, SH.NodeShape)); s.add((sh, SH.targetClass, URIRef(BL + "Gene")))
    b = URIRef("https://ex/shape/l"); s.add((sh, SH.property, b)); s.add((b, SH.path, RDFS.label)); s.add((b, SH.minCount, Literal(1)))
    return s

def evaluate(name, g):
    c, _, _ = validate(g, shacl_graph=shapes(), inference="none")
    fl = gate(g)
    return {"label": name, "triples": len(g), "shacl_conforms": bool(c), "closed_world_violations": len(fl), "violations": fl}

def main():
    all_pmids, seen = [], set()
    for sym in TARGETS:
        for p in search_pmids(sym, k=5):
            if p not in seen:
                seen.add(p); all_pmids.append(p)
        time.sleep(0.3)
    print(f"[*] {len(all_pmids)} PMIDs from PubTator3 across {len(TARGETS)} targets")

    docs = []
    for i in range(0, len(all_pmids), 20):
        docs += export(all_pmids[i:i+20]); time.sleep(0.3)
    print(f"[*] exported {len(docs)} annotated documents")

    edges = gene_disease_edges(docs)
    # de-duplicate by (gene,disease)
    uniq = {}
    for e in edges:
        uniq[(e["gene_id"], e["disease_id"])] = e
    edges = list(uniq.values())
    print(f"[*] extracted {len(edges)} unique gene-disease relations from the literature")

    grounded = build_kg(edges, BL + "gene_associated_with_condition")
    ungrounded = build_kg(edges, BL + "associated_with_disease")
    grounded.serialize(os.path.join(BUILD, "lit-kg-grounded.ttl"), format="turtle")

    r_g = evaluate("grounded literature KG", grounded)
    r_u = evaluate("ungrounded literature KG", ungrounded)

    genes_in_lit = {e["gene_name"].upper() for e in edges}
    overlap = sorted(g for g in genes_in_lit if g in {t.upper() for t in TARGETS})

    out = {
        "source": "PubTator3 (NLM) machine-extracted gene-disease relations",
        "pmids": len(all_pmids), "documents": len(docs), "gene_disease_relations": len(edges),
        "grounded": r_g, "ungrounded": r_u,
        "target_genes_with_literature_edges": overlap,
        "headline": {
            "grounded_closed_world_violations": r_g["closed_world_violations"],
            "ungrounded_shacl_conforms": r_u["shacl_conforms"],
            "ungrounded_caught_by_gate": r_u["closed_world_violations"] > 0,
        },
        "sample_edges": edges[:10],
    }
    json.dump(out, open(os.path.join(RES, "results_literature.json"), "w"), indent=2)
    print(json.dumps(out["headline"], indent=2))
    print("grounded:", r_g); print("ungrounded:", r_u)
    print("target genes with literature edges:", overlap)

if __name__ == "__main__":
    main()
