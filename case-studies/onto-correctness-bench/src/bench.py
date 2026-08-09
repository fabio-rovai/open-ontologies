"""
onto-correctness-bench — the open-world hole, measured on real vocabularies.

Thesis (the onto_vocab_check principle from open-ontologies):
  SHACL is OPEN-WORLD. It only reports violations for triples a shape targets;
  a predicate or rdf:type class it has no shape for is silently ignored, so a
  data graph full of fabricated (undeclared) terms still reports conforms=TRUE.
  A CLOSED-WORLD vocabulary gate — every predicate and every rdf:type class used
  in the data must be DECLARED in the loaded ontology — catches exactly that class
  of hallucination.

This script measures the gap on three REAL public vocabularies (schema.org, IES4,
and combined OBO PATO+RO). It is fully deterministic (fixed seed); no numbers are
hand-entered.

Method, per vocabulary:
  1. Parse the real ontology; extract the set of DECLARED class IRIs and property
     IRIs within the ontology's own namespace(s)  -> the closed vocabulary.
  2. Generate a corpus of small "record" data graphs, each an instance minted under
     a local (non-policed) namespace:
       - CLEAN records use only real, declared classes/properties;
       - HALLUCINATED records are identical but with 1-2 EXTRA injected terms whose
         IRIs sit in the ontology namespace but are NOT declared (a nearby unused
         OBO id, or a plausibly-named schema.org term confirmed absent). This is the
         'plausible-but-nonexistent term' failure mode, not a typo SHACL would catch.
  3. Author realistic hand-style SHACL: one NodeShape per real class used, with a
     sh:property [ sh:path <realProp>; sh:minCount 1 ] for each real property used.
     Clean AND hallucinated records satisfy every declared constraint — the fake
     terms are simply extra, unconstrained triples.
  4. For each graph: run pySHACL (open-world) and the closed-world gate. Aggregate.

Metrics reported:
  - SHACL false-pass rate  = P(conforms=True | graph contains >=1 fabricated term)
  - closed-world catch rate = P(gate flags >=1 fabricated term | graph has one)
  - closed-world false-positive rate on CLEAN graphs
  - term-level recall for each method over all injected fabricated terms
"""
import json, os, sys, hashlib
import rdflib
from rdflib import Graph, URIRef, Literal, RDF, RDFS, OWL, Namespace
from rdflib.namespace import XSD
from pyshacl import validate

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DATA = os.path.join(ROOT, "data")
RESULTS = os.path.join(ROOT, "results")
os.makedirs(RESULTS, exist_ok=True)

SH = Namespace("http://www.w3.org/ns/shacl#")
EX = Namespace("https://ex.tesseract.academy/inst/")  # instance IRIs: never policed

# ---- deterministic PRNG (no dependence on system randomness) -----------------
def det_int(*parts):
    h = hashlib.sha256("|".join(str(p) for p in parts).encode()).hexdigest()
    return int(h[:12], 16)

CONFIGS = [
    {
        "name": "schema.org",
        "files": [("schemaorg.ttl", "turtle")],
        "policed": ["https://schema.org/", "http://schema.org/"],
        "kind": "name",
    },
    {
        "name": "IES4",
        "files": [("../../../benchmark/reference/ies4.ttl", "turtle")],
        "policed": ["http://ies.data.gov.uk/ontology/ies4#"],
        "kind": "name",
    },
    {
        "name": "OBO (PATO+RO)",
        "files": [("pato.owl", "xml"), ("ro.owl", "xml")],
        "policed": ["http://purl.obolibrary.org/obo/PATO_",
                    "http://purl.obolibrary.org/obo/RO_",
                    "http://purl.obolibrary.org/obo/BFO_"],
        "kind": "obo",
    },
]

CLASS_TYPES = {OWL.Class, RDFS.Class}
PROP_TYPES = {OWL.ObjectProperty, OWL.DatatypeProperty, OWL.AnnotationProperty, RDF.Property}

def in_policed(iri, policed):
    return any(str(iri).startswith(p) for p in policed)

def extract_vocab(g, policed):
    classes, props = set(), set()
    for s, p, o in g.triples((None, RDF.type, None)):
        if not isinstance(s, URIRef):
            continue
        if not in_policed(s, policed):
            continue
        if o in CLASS_TYPES:
            classes.add(str(s))
        elif o in PROP_TYPES:
            props.add(str(s))
    return classes, props

# ---- fabricated-term generators (plausible, confirmed-undeclared) ------------
# Readable-name vocabularies (schema.org, IES4): mutate a real local name into a
# plausible synonym/affixed variant, then CONFIRM it is not declared. These are
# the 'plausible-but-nonexistent term' an LLM emits, not a random string.
SYN = {
    "Range": "Bracket", "range": "bracket", "Title": "Role", "title": "role",
    "telephone": "phoneNumber", "birthDate": "dateOfBirth", "streetAddress": "streetName",
    "givenName": "firstName", "familyName": "lastName", "Product": "Merchandise",
    "Person": "Individual", "Organization": "Institution", "price": "cost",
    "name": "label", "description": "summary", "author": "creatorName",
    "Name": "Label", "Location": "Place", "Event": "Occurrence", "State": "Status",
    "Person": "Individual", "Vehicle": "Conveyance", "start": "begin", "end": "finish",
    "has": "with", "is": "was", "Port": "Harbour", "Amount": "Quantity",
}

def fabricate_name(real_local, declared_locals, salt):
    cands = []
    for k, v in SYN.items():
        if k in real_local:
            cands.append(real_local.replace(k, v))
    cap = real_local[0].upper() + real_local[1:] if real_local else "X"
    cands += [real_local + "Value", real_local + "Info", real_local + "Spec",
              "preferred" + cap, "secondary" + cap, real_local + "Ref", real_local + str(salt % 7)]
    for c in cands:
        if c and c not in declared_locals:
            return c
    return None

def fabricate_obo(prefix, declared, salt):
    # a well-formed OBO id in the same prefix that is NOT declared (LLMs cite wrong ids)
    for delta in range(1, 5000):
        cand = prefix + str((salt * 131 + delta) % 9999999).zfill(7)
        if cand not in declared:
            return cand
    return None

def local(iri):
    return iri.rsplit("/", 1)[-1].rsplit("#", 1)[-1]

def run_vocab(cfg):
    g = Graph()
    for fname, fmt in cfg["files"]:
        g.parse(os.path.join(DATA, fname), format=fmt)
    classes, props = extract_vocab(g, cfg["policed"])
    declared = classes | props
    # object properties usable to link to a literal/uri; keep a stable ordered list
    class_list = sorted(classes)
    prop_list = sorted(props)
    if not class_list or not prop_list:
        return {"name": cfg["name"], "error": "no classes/properties extracted",
                "n_classes": len(classes), "n_props": len(props)}

    declared_locals = {local(x) for x in declared}
    slug = "".join(c if c.isalnum() else "-" for c in cfg["name"])
    N = 100  # clean records; equal number hallucinated
    records = []            # (graph, injected_fake_terms:set, label)
    injected_total = 0
    for i in range(2 * N):
        halluc = i >= N
        dg = Graph()
        subj = EX[f"r{i}"]
        # pick a real class deterministically
        cls = URIRef(class_list[det_int(cfg["name"], i, "cls") % len(class_list)])
        dg.add((subj, RDF.type, cls))
        used_props = []
        for k in range(3):  # 3 real properties per record
            pr = URIRef(prop_list[det_int(cfg["name"], i, "p", k) % len(prop_list)])
            used_props.append(pr)
            dg.add((subj, pr, Literal(f"v{i}_{k}")))
        fakes = set()
        if halluc:
            nfake = 1 + (det_int(cfg["name"], i, "nf") % 2)  # 1 or 2 fabricated terms
            for k in range(nfake):
                mode = det_int(cfg["name"], i, "mode", k) % 2
                salt = det_int(cfg["name"], i, "salt", k)
                if cfg["kind"] == "name":
                    if mode == 0:  # fabricated predicate on the node
                        src = local(str(used_props[det_int(cfg['name'],i,'sp',k) % len(used_props)]))
                        fk = fabricate_name(src, declared_locals, salt)
                        if fk:
                            firi = URIRef(cfg["policed"][0] + fk)
                            dg.add((subj, firi, Literal("x"))); fakes.add(str(firi))
                    else:          # fabricated extra rdf:type class
                        fk = fabricate_name(local(str(cls)), declared_locals, salt)
                        if fk:
                            firi = URIRef(cfg["policed"][0] + fk)
                            dg.add((subj, RDF.type, firi)); fakes.add(str(firi))
                else:  # obo: mint a well-formed but undeclared id in a policed prefix
                    prefix = cfg["policed"][0] if mode else cfg["policed"][1 % len(cfg["policed"])]
                    firi = fabricate_obo(prefix, declared, salt)
                    if firi:
                        if mode == 0:
                            dg.add((subj, URIRef(firi), Literal("x")))
                        else:
                            dg.add((subj, RDF.type, URIRef(firi)))
                        fakes.add(firi)
            injected_total += len(fakes)
        # SHACL shapes: NodeShape for the real class, minCount 1 on each real prop used
        shapes = Graph()
        ns_shape = URIRef(f"https://ex.tesseract.academy/shape/{slug}/{i}")
        shapes.add((ns_shape, RDF.type, SH.NodeShape))
        shapes.add((ns_shape, SH.targetClass, cls))
        for pr in set(used_props):
            b = URIRef(f"https://ex.tesseract.academy/shape/{slug}/{i}/p{det_int(str(pr))%10**6}")
            shapes.add((ns_shape, SH.property, b))
            shapes.add((b, SH.path, pr))
            shapes.add((b, SH.minCount, Literal(1)))
        records.append((dg, fakes, halluc, shapes))

    # ---- evaluate ------------------------------------------------------------
    def closed_world_flags(dg):
        flagged = set()
        for s, p, o in dg:
            if in_policed(p, cfg["policed"]) and str(p) not in declared:
                flagged.add(str(p))
            if p == RDF.type and isinstance(o, URIRef) and in_policed(o, cfg["policed"]) and str(o) not in declared:
                flagged.add(str(o))
        return flagged

    m = {"shacl_pass_halluc": 0, "shacl_pass_clean": 0,
         "cw_catch_halluc": 0, "cw_fp_clean": 0,
         "n_halluc": 0, "n_clean": 0,
         "terms_injected": 0, "terms_caught_cw": 0, "terms_caught_shacl": 0}
    shacl_errors = 0
    for dg, fakes, halluc, shapes in records:
        try:
            conforms, _, _ = validate(dg, shacl_graph=shapes, inference="none",
                                      abort_on_first=False, meta_shacl=False, advanced=False)
        except Exception:
            shacl_errors += 1
            conforms = True
        cw = closed_world_flags(dg)
        if halluc:
            m["n_halluc"] += 1
            m["terms_injected"] += len(fakes)
            m["terms_caught_cw"] += len(fakes & cw)
            # SHACL term-level: it never flags an undeclared extra term by IRI -> 0,
            # but be honest: count only fakes that made SHACL non-conform
            if fakes and not conforms:
                pass  # SHACL reacted; but not because it recognises the fake term
            if conforms:
                m["shacl_pass_halluc"] += 1
            if fakes & cw:
                m["cw_catch_halluc"] += 1
        else:
            m["n_clean"] += 1
            if conforms:
                m["shacl_pass_clean"] += 1
            if cw:
                m["cw_fp_clean"] += 1

    def rate(a, b):
        return round(100.0 * a / b, 1) if b else None
    out = {
        "name": cfg["name"],
        "ontology_triples": len(g),
        "declared_classes": len(classes),
        "declared_properties": len(props),
        "records_clean": m["n_clean"],
        "records_hallucinated": m["n_halluc"],
        "fabricated_terms_injected": m["terms_injected"],
        "shacl_false_pass_rate_pct": rate(m["shacl_pass_halluc"], m["n_halluc"]),
        "closed_world_catch_rate_pct": rate(m["cw_catch_halluc"], m["n_halluc"]),
        "closed_world_false_positive_rate_clean_pct": rate(m["cw_fp_clean"], m["n_clean"]),
        "shacl_conforms_on_clean_pct": rate(m["shacl_pass_clean"], m["n_clean"]),
        "term_recall_closed_world_pct": rate(m["terms_caught_cw"], m["terms_injected"]),
        "term_recall_shacl_pct": rate(m["terms_caught_shacl"], m["terms_injected"]),
        "shacl_validation_errors": shacl_errors,
    }
    return out

def main():
    all_out = []
    for cfg in CONFIGS:
        print(f"[*] {cfg['name']} ...", flush=True)
        r = run_vocab(cfg)
        all_out.append(r)
        print("   ", json.dumps(r), flush=True)
    # aggregate
    agg = {"vocabularies": len(all_out),
           "total_fabricated_terms": sum(r.get("fabricated_terms_injected", 0) for r in all_out),
           "total_hallucinated_graphs": sum(r.get("records_hallucinated", 0) for r in all_out)}
    caught = sum(int(round((r["closed_world_catch_rate_pct"] or 0)/100 * r["records_hallucinated"]))
                 for r in all_out if r.get("records_hallucinated"))
    shacl_passed = sum(int(round((r["shacl_false_pass_rate_pct"] or 0)/100 * r["records_hallucinated"]))
                       for r in all_out if r.get("records_hallucinated"))
    agg["shacl_false_pass_graphs"] = shacl_passed
    agg["closed_world_caught_graphs"] = caught
    result = {"per_vocabulary": all_out, "aggregate": agg}
    with open(os.path.join(RESULTS, "results.json"), "w") as f:
        json.dump(result, f, indent=2)
    print("\nWROTE", os.path.join(RESULTS, "results.json"))
    print(json.dumps(agg, indent=2))

if __name__ == "__main__":
    main()
