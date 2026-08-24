"""
Release gate for the MoDiP knowledge graph.

Three checks, all must pass:
  1. every RDF file parses;
  2. SHACL validation (pyshacl) reports conforms=True against modip-shapes.ttl,
     over the UNION of the instance graph and the taxonomies (so concept typing
     is in scope);
  3. closed-world vocabulary check: every mat:/proc:/dom: concept URI referenced
     by an object is actually DEFINED (carries a skos:prefLabel) in a taxonomy.
     Plain SHACL is open-world and would silently pass a dangling concept URI;
     this catch is the onto_vocab_check principle from the open-ontologies work.
"""
import glob, json, os, sys
import rdflib
from rdflib.namespace import SKOS
from pyshacl import validate

ROOT = os.path.dirname(os.path.dirname(__file__))
B = "https://ontology.tesseract.academy/modip/"


def load_union():
    g = rdflib.Graph()
    files = (sorted(glob.glob(os.path.join(ROOT, "ontology", "*.ttl"))) +
             [os.path.join(ROOT, "build", "modip-crm.ttl"),
              os.path.join(ROOT, "build", "dag_variants.ttl")])
    for f in files:
        n = len(g); g.parse(f, format="turtle")
        print(f"  parsed {os.path.relpath(f, ROOT)}: +{len(g)-n} triples")
    return g


def main():
    print("[1] parse check")
    g = load_union()
    print(f"  union graph: {len(g)} triples")

    print("[2] SHACL validation")
    shapes = rdflib.Graph().parse(os.path.join(ROOT, "shapes", "modip-shapes.ttl"), format="turtle")
    conforms, _, report = validate(g, shacl_graph=shapes, inference="none",
                                   abort_on_first=False, meta_shacl=False)
    print(f"  conforms: {conforms}")
    if not conforms:
        print(report[:3000])

    print("[3] closed-world vocabulary check")
    DCT_TITLE = rdflib.URIRef("http://purl.org/dc/terms/title")
    # a concept URI is 'defined' if it carries a prefLabel (skos:Concept) or is a
    # titled skos:ConceptScheme (the scheme nodes referenced via skos:inScheme).
    defined = set(str(s) for s in g.subjects(SKOS.prefLabel, None))
    defined |= set(str(s) for s in g.subjects(DCT_TITLE, None))
    referenced, dangling = set(), set()
    for pref in ("materials/", "processes/", "domains/"):
        pass
    for s, p, o in g:
        os_ = str(o)
        if any(os_.startswith(B + x) for x in ("materials/", "processes/", "domains/")):
            referenced.add(os_)
            if os_ not in defined:
                dangling.add(os_)
    print(f"  concept URIs referenced by data: {len(referenced)}")
    print(f"  dangling (referenced but undefined): {len(dangling)}")
    for d in sorted(dangling)[:20]:
        print(f"    DANGLING {d}")

    out = {
        "parse_ok": True,
        "union_triples": len(g),
        "shacl_conforms": bool(conforms),
        "concepts_referenced": len(referenced),
        "dangling_concepts": len(dangling),
    }
    json.dump(out, open(os.path.join(ROOT, "build", "validation_report.json"), "w"), indent=2)
    ok = conforms and not dangling
    print("\nRESULT:", "PASS" if ok else "FAIL")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
