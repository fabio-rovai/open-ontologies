"""
AMR pathogen linkage (#60, completing the layer): link resistance genes to the pathogens
they occur in, grounded in three real vocabularies at once.

Source: CARD's card.json gives, for each resistance determinant (ARO id), the organisms its
reference sequences come from (NCBI taxonomy id). We build gene -> in_taxon -> organism edges
and police THREE namespaces against their real authorities:
  - ARO (the resistance determinant)   against ARO's declared terms (aro.obo);
  - Biolink (the types and predicate)  against the Biolink Model;
  - NCBITaxon (the organism)           against the CURRENT NCBI taxonomy (nodes.dmp, 2.87M taxids).

Two things fall out. A fabricated taxon id injected into the ungrounded twin is caught, as
before. And, run against the CURRENT taxonomy, the gate also flags organism ids in CARD that
are no longer current (retired and merged in NCBI's merged.dmp), a data-freshness signal that
open-world SHACL and a naive "is it a number" check both miss.
"""
import json, os
import rdflib
from rdflib import Graph, RDF, RDFS, URIRef, Literal
from rdflib.namespace import SH
from pyshacl import validate

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA, BUILD, RES = (os.path.join(ROOT, d) for d in ("data", "build", "results"))
BL = "https://w3id.org/biolink/vocab/"
OBO = "http://purl.obolibrary.org/obo/"
ARO_NS = OBO + "ARO_"; TAX_NS = OBO + "NCBITaxon_"
POLICED = [ARO_NS, TAX_NS, BL]

def load_declared():
    d = set()
    # ARO terms
    for line in open(os.path.join(DATA, "aro.obo"), encoding="utf-8"):
        if line.startswith("id: ARO:"):
            d.add(ARO_NS + line[8:].strip())
    # Biolink terms
    d |= set(json.load(open(os.path.join(DATA, "biolink_vocab.json")))["declared"])
    # current NCBI taxonomy
    for line in open(os.path.join(DATA, "nodes.dmp"), encoding="utf-8"):
        d.add(TAX_NS + line.split("\t", 1)[0].strip())
    return d

def load_merged():
    m = set()
    p = os.path.join(DATA, "merged.dmp")
    if os.path.exists(p):
        for line in open(p, encoding="utf-8"):
            m.add(TAX_NS + line.split("\t", 1)[0].strip())
    return m

def card_pairs():
    d = json.load(open(os.path.join(DATA, "card", "card.json")))
    pairs = {}
    for k, v in d.items():
        if not (isinstance(v, dict) and v.get("ARO_accession")):
            continue
        aro = v["ARO_accession"]; gname = v.get("ARO_name", aro)
        for sid, seq in v.get("model_sequences", {}).get("sequence", {}).items():
            tax = seq.get("NCBI_taxonomy", {})
            tid = tax.get("NCBI_taxonomy_id")
            if tid:
                pairs[(aro, str(tid))] = (gname, tax.get("NCBI_taxonomy_name", str(tid)))
    return [(a, t, n[0], n[1]) for (a, t), n in pairs.items()]

def build(pairs, fabricate=False):
    g = Graph()
    for i, (aro, tid, gname, oname) in enumerate(pairs):
        gene = URIRef(ARO_NS + aro.replace("ARO:", ""))
        org = URIRef(TAX_NS + ("99999999" if (fabricate and i == 0) else tid))
        g.add((gene, RDF.type, URIRef(BL + "Gene"))); g.add((gene, RDFS.label, Literal(gname)))
        g.add((org, RDF.type, URIRef(BL + "OrganismTaxon"))); g.add((org, RDFS.label, Literal(oname)))
        g.add((gene, URIRef(BL + "in_taxon"), org))
    return g

def in_pol(i): return any(str(i).startswith(p) for p in POLICED)
def gate(g, declared):
    flags = set()
    for s, p, o in g:
        if in_pol(p) and str(p) not in declared: flags.add(str(p))
        for node in (s, o):
            if isinstance(node, URIRef) and str(node).startswith((ARO_NS, TAX_NS)) and str(node) not in declared:
                flags.add(str(node))
        if p == RDF.type and isinstance(o, URIRef) and in_pol(o) and str(o) not in declared:
            flags.add(str(o))
    return sorted(flags)

def shapes():
    s = Graph(); sh = URIRef("https://ex/path/shape")
    s.add((sh, RDF.type, SH.NodeShape)); s.add((sh, SH.targetClass, URIRef(BL + "Gene")))
    b = URIRef("https://ex/path/shape/l"); s.add((sh, SH.property, b)); s.add((b, SH.path, RDFS.label)); s.add((b, SH.minCount, Literal(1)))
    return s

def main():
    declared = load_declared(); merged = load_merged()
    pairs = card_pairs()
    organisms = {t for _, t, _, _ in pairs}
    print(f"[*] CARD gene-organism links: {len(pairs)} edges over {len(organisms)} distinct organisms")

    grounded = build(pairs, fabricate=False)
    ungrounded = build(pairs, fabricate=True)
    grounded.serialize(os.path.join(BUILD, "pathogen-kg-grounded.ttl"), format="turtle")

    c, _, _ = validate(grounded, shacl_graph=shapes(), inference="none")
    flags = gate(grounded, declared)
    tax_flags = [f for f in flags if f.startswith(TAX_NS)]
    retired = [f for f in tax_flags if f in merged]     # retired-and-merged (data freshness)
    fabricated = [f for f in tax_flags if f not in merged]

    cu, _, _ = validate(ungrounded, shacl_graph=shapes(), inference="none")
    ung_flags = gate(ungrounded, declared)
    injected_caught = (TAX_NS + "99999999") in ung_flags

    out = {
        "source": "CARD card.json (gene-organism) + current NCBI taxonomy (nodes.dmp)",
        "edges": len(pairs), "distinct_organisms": len(organisms),
        "policed_namespaces": ["ARO", "Biolink", "NCBITaxon"],
        "ncbi_current_taxids": sum(1 for _ in open(os.path.join(DATA, "nodes.dmp"), encoding="utf-8")),
        "grounded_shacl_conforms": bool(c),
        "organism_ids_flagged_not_current": len(tax_flags),
        "of_which_retired_and_merged": len(retired),
        "of_which_unexplained": len(fabricated),
        "retired_examples": [f.split("_")[-1] for f in retired[:10]],
        "ungrounded_shacl_conforms": bool(cu),
        "injected_fabricated_taxid_caught": injected_caught,
    }
    json.dump(out, open(os.path.join(RES, "results_pathogen.json"), "w"), indent=2)
    print(json.dumps(out, indent=2))

if __name__ == "__main__":
    main()
