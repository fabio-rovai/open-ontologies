#!/usr/bin/env python3
"""
OntoAxiom Benchmark: Tool-Augmented Extraction vs Bare LLMs

Runs the OntoAxiom benchmark (https://arxiv.org/abs/2512.05594) using
SPARQL extraction from loaded ontologies — the same approach Open Ontologies
uses via its MCP tools (onto_load + onto_query).

The paper asks LLMs to identify axioms from ontology descriptions.
Best bare LLM result: o1 with F1=0.197.

Our approach: load the actual ontology → extract axioms via SPARQL.
No hallucination. No prompt engineering. Just structured queries.
"""
import json
import os
import sys

try:
    from rdflib import Graph, RDF, RDFS, OWL, Namespace
except ImportError:
    print("ERROR: rdflib required. Install with: pip install rdflib")
    sys.exit(1)

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
DATA_DIR = os.path.join(SCRIPT_DIR, "data", "ontoaxiom")
ONTOLOGIES_DIR = os.path.join(DATA_DIR, "ontologies")


def normalize(s):
    """Normalize for soft matching (same approach as OntoAxiom evaluation)."""
    s = str(s).strip()
    # Extract local name from IRI
    if "#" in s:
        s = s.split("#")[-1]
    elif "/" in s:
        s = s.split("/")[-1]
    # Normalize spacing and casing
    return s.lower().replace("_", " ").replace("-", " ")


def load_ontology(ttl_path):
    """Load a Turtle ontology into an rdflib Graph."""
    g = Graph()
    g.parse(ttl_path, format="turtle")
    return g


def get_local_name(iri):
    """Extract local name from an IRI."""
    s = str(iri)
    if "#" in s:
        return s.split("#")[-1]
    return s.rsplit("/", 1)[-1] if "/" in s else s


def get_label_or_local(g, entity):
    """Get rdfs:label if available, otherwise local name."""
    for label in g.objects(entity, RDFS.label):
        return str(label)
    return get_local_name(entity)


def extract_subclassof(g):
    """Extract subClassOf axioms."""
    pairs = set()
    for sub, sup in g.subject_objects(RDFS.subClassOf):
        if sub == sup:
            continue
        # Skip blank nodes and W3C builtins
        from rdflib import BNode
        if isinstance(sub, BNode) or isinstance(sup, BNode):
            continue
        if str(sub).startswith("http://www.w3.org/") or str(sup).startswith("http://www.w3.org/"):
            continue
        pairs.add((normalize(get_label_or_local(g, sub)),
                    normalize(get_label_or_local(g, sup))))
    return pairs


def extract_disjoint(g):
    """Extract disjointWith axioms."""
    pairs = set()
    for c1, c2 in g.subject_objects(OWL.disjointWith):
        from rdflib import BNode
        if isinstance(c1, BNode) or isinstance(c2, BNode):
            continue
        a, b = normalize(get_label_or_local(g, c1)), normalize(get_label_or_local(g, c2))
        # Canonical order
        pairs.add((min(a, b), max(a, b)))

    # Also handle AllDisjointClasses
    for adc in g.subjects(RDF.type, OWL.AllDisjointClasses):
        members_list = list(g.objects(adc, OWL.members))
        if members_list:
            from rdflib.collection import Collection
            members = list(Collection(g, members_list[0]))
            for i, c1 in enumerate(members):
                for c2 in members[i+1:]:
                    from rdflib import BNode as BN
                    if isinstance(c1, BN) or isinstance(c2, BN):
                        continue
                    a = normalize(get_label_or_local(g, c1))
                    b = normalize(get_label_or_local(g, c2))
                    pairs.add((min(a, b), max(a, b)))
    return pairs


def extract_domain(g):
    """Extract domain axioms."""
    pairs = set()
    for prop in set(g.subjects(RDF.type, OWL.ObjectProperty)) | set(g.subjects(RDF.type, OWL.DatatypeProperty)):
        for dom in g.objects(prop, RDFS.domain):
            from rdflib import BNode
            if isinstance(dom, BNode):
                continue
            pairs.add((normalize(get_label_or_local(g, dom)),
                        normalize(get_label_or_local(g, prop))))
    return pairs


def extract_range(g):
    """Extract range axioms."""
    pairs = set()
    for prop in set(g.subjects(RDF.type, OWL.ObjectProperty)) | set(g.subjects(RDF.type, OWL.DatatypeProperty)):
        for ran in g.objects(prop, RDFS.range):
            from rdflib import BNode
            if isinstance(ran, BNode):
                continue
            pairs.add((normalize(get_label_or_local(g, prop)),
                        normalize(get_label_or_local(g, ran))))
    return pairs


def extract_subproperty(g):
    """Extract subPropertyOf axioms."""
    pairs = set()
    for sub, sup in g.subject_objects(RDFS.subPropertyOf):
        if sub == sup:
            continue
        if str(sub).startswith("http://www.w3.org/") or str(sup).startswith("http://www.w3.org/"):
            continue
        pairs.add((normalize(get_label_or_local(g, sub)),
                    normalize(get_label_or_local(g, sup))))
    return pairs


EXTRACTORS = {
    "subclassof": extract_subclassof,
    "disjoint": extract_disjoint,
    "domain": extract_domain,
    "range": extract_range,
    "subproperty": extract_subproperty,
}


def load_ground_truth(axiom_type, ontology_name):
    """Load ground truth pairs from OntoAxiom dataset."""
    path = os.path.join(DATA_DIR, axiom_type, f"{ontology_name}_{axiom_type}.json")
    if not os.path.exists(path):
        return set()
    with open(path) as f:
        data = json.load(f)
    gt = set()
    for pair in data:
        a, b = normalize(pair[0]), normalize(pair[1])
        if axiom_type == "disjoint":
            gt.add((min(a, b), max(a, b)))
        else:
            gt.add((a, b))
    return gt


def precision_recall_f1(tp, fp, fn):
    precision = tp / (tp + fp) if (tp + fp) > 0 else 0.0
    recall = tp / (tp + fn) if (tp + fn) > 0 else 0.0
    f1 = 2 * precision * recall / (precision + recall + 1e-9) if (precision + recall) > 0 else 0.0
    return precision, recall, f1


def main():
    ontologies = sorted([
        f.replace(".ttl", "")
        for f in os.listdir(ONTOLOGIES_DIR)
        if f.endswith(".ttl")
    ])

    print("=" * 70)
    print("OntoAxiom Benchmark: Tool-Augmented vs Bare LLMs")
    print("=" * 70)
    print(f"\nOntologies: {len(ontologies)}")
    print(f"Axiom types: {len(EXTRACTORS)}")
    print(f"Method: Load TTL + structured extraction (no hallucination)")
    print(f"Baseline: Best bare LLM (o1) F1 = 0.197")
    print()

    all_results = {}
    grand_tp = grand_fp = grand_fn = 0

    for axiom_type, extractor in EXTRACTORS.items():
        print(f"\n--- {axiom_type} ---")
        type_tp = type_fp = type_fn = 0

        for onto_name in ontologies:
            onto_path = os.path.join(ONTOLOGIES_DIR, f"{onto_name}.ttl")
            gt = load_ground_truth(axiom_type, onto_name)

            if not gt:
                continue

            g = load_ontology(onto_path)
            extracted = extractor(g)

            tp = len(extracted & gt)
            fp = len(extracted - gt)
            fn = len(gt - extracted)
            p, r, f1 = precision_recall_f1(tp, fp, fn)

            type_tp += tp
            type_fp += fp
            type_fn += fn

            status = "PERFECT" if fn == 0 and fp == 0 else ""
            print(f"  {onto_name:20s}  P={p:.3f}  R={r:.3f}  F1={f1:.3f}  "
                  f"(TP={tp}, FP={fp}, FN={fn})  {status}")

            all_results.setdefault(axiom_type, {})[onto_name] = {
                "precision": round(p, 4),
                "recall": round(r, 4),
                "f1": round(f1, 4),
                "tp": tp, "fp": fp, "fn": fn,
                "ground_truth": len(gt),
                "extracted": len(extracted),
            }

        p, r, f1 = precision_recall_f1(type_tp, type_fp, type_fn)
        grand_tp += type_tp
        grand_fp += type_fp
        grand_fn += type_fn
        print(f"  {'TOTAL':20s}  P={p:.3f}  R={r:.3f}  F1={f1:.3f}")
        all_results[axiom_type]["_total"] = {
            "precision": round(p, 4), "recall": round(r, 4), "f1": round(f1, 4),
            "tp": type_tp, "fp": type_fp, "fn": type_fn,
        }

    # Grand total
    gp, gr, gf1 = precision_recall_f1(grand_tp, grand_fp, grand_fn)
    all_results["_grand_total"] = {
        "precision": round(gp, 4), "recall": round(gr, 4), "f1": round(gf1, 4),
        "tp": grand_tp, "fp": grand_fp, "fn": grand_fn,
    }

    print()
    print("=" * 70)
    print("GRAND TOTAL")
    print("=" * 70)
    print(f"  Precision: {gp:.3f}")
    print(f"  Recall:    {gr:.3f}")
    print(f"  F1:        {gf1:.3f}")
    print()
    print(f"  Best bare LLM (o1):                F1 = 0.197")
    print(f"  Tool-augmented (Open Ontologies):   F1 = {gf1:.3f}")
    if gf1 > 0.197:
        improvement = ((gf1 / 0.197) - 1) * 100
        print(f"\n  >>> Tool-augmented outperforms best LLM by {improvement:.0f}% <<<")
    print()

    # Per-type comparison table
    print("Per-Axiom-Type Comparison:")
    print(f"  {'Type':15s}  {'OO F1':>8s}  {'o1 F1':>8s}  {'Winner'}")
    print(f"  {'-'*50}")
    # o1 per-type F1 from the paper
    o1_f1 = {
        "subclassof": 0.359,
        "disjoint": 0.095,
        "domain": 0.038,
        "range": 0.030,
        "subproperty": 0.106,
    }
    for at in EXTRACTORS:
        oo_f1 = all_results[at]["_total"]["f1"]
        baseline = o1_f1.get(at, 0)
        winner = "OO" if oo_f1 > baseline else "o1" if baseline > oo_f1 else "tie"
        print(f"  {at:15s}  {oo_f1:>8.3f}  {baseline:>8.3f}  {winner}")

    # Save results
    out_path = os.path.join(SCRIPT_DIR, "data", "results", "oo_ontoaxiom_results.json")
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, "w") as f:
        json.dump(all_results, f, indent=2)
    print(f"\nResults saved to {out_path}")


if __name__ == "__main__":
    main()
