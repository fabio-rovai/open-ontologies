#!/usr/bin/env python3
"""
Validate the Skills England Occupational Maps Ontology (SEOM) and write a
coverage report.

Two independent checks:
  1. SHACL (pyshacl): every occupation is fully placed in the map, every product
     and SOC concept is well-formed. Target: zero violations.
  2. SPARQL coverage (open-ontologies Oxigraph engine): counts and referential
     integrity across the whole graph (orphan references, coverage percentages).

Run:  python validate_ontology.py
Writes: ontology/coverage-report.md
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ONT = os.path.join(HERE, "ontology")
sys.path.insert(0, "/Users/fabio/projects/open-ontologies/python/src")

import rdflib
from pyshacl import validate as shacl_validate

VOCAB = os.path.join(ONT, "seom-vocabulary.ttl")
INST = os.path.join(ONT, "occupational-map.ttl")
SHAPES = os.path.join(ONT, "shapes.ttl")


def run_shacl():
    data = rdflib.Graph()
    data.parse(VOCAB, format="turtle")
    data.parse(INST, format="turtle")
    conforms, _, text = shacl_validate(
        data, shacl_graph=SHAPES, inference="none", advanced=True, meta_shacl=False
    )
    n_viol = 0 if conforms else text.count("Constraint Violation")
    return conforms, n_viol, text, len(data)


def run_coverage():
    from open_ontologies_lite import OntologyEngine

    eng = OntologyEngine()
    ttl = open(VOCAB).read() + "\n" + open(INST).read()
    eng.load(ttl, "turtle")
    PRE = (
        "PREFIX seom: <https://gov.tesseract.academy/ns/seom#>\n"
        "PREFIX skos: <http://www.w3.org/2004/02/skos/core#>\n"
        "PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\n"
    )

    def count(where):
        rows = eng.query(PRE + "SELECT (COUNT(*) AS ?n) WHERE {" + where + "}")["rows"]
        return int(rows[0]["n"].split("^")[0].strip('"'))

    stats = {
        "Occupational standards": count("?o a seom:Occupation"),
        "Routes": count("?r a seom:Route"),
        "Pathways": count("?p a seom:Pathway"),
        "Clusters": count("?c a seom:Cluster"),
        "SOC 2020 concepts": count("?s a seom:SOCConcept ; seom:socVersion '2020'"),
        "SOC 2010 concepts": count("?s a seom:SOCConcept ; seom:socVersion '2010'"),
        "Technical education products": count("?p a seom:Product"),
        "Green themes": count("?t a seom:GreenTheme"),
        "Progression edges": count("?a seom:progressesTo ?b"),
        "Occupations with a SOC 2020 mapping": count("?o a seom:Occupation ; seom:socMapping2020 ?s"),
        "Occupations in a green theme": count("?o a seom:Occupation ; seom:inGreenTheme ?t"),
        "Occupations delivered through a product": count("?o a seom:Occupation ; seom:deliveredThrough ?p"),
    }

    # Referential integrity: every progression target / route / SOC reference must
    # resolve to a typed resource in the graph.
    integrity = {
        "Occupations with no route": count(
            "?o a seom:Occupation FILTER NOT EXISTS { ?o seom:inRoute ?r }"
        ),
        "progressesTo targets that are not Occupations": count(
            "?a seom:progressesTo ?b FILTER NOT EXISTS { ?b a seom:Occupation }"
        ),
        "inGreenTheme targets that are not GreenThemes": count(
            "?o seom:inGreenTheme ?t FILTER NOT EXISTS { ?t a seom:GreenTheme }"
        ),
        "socMapping targets with no notation": count(
            "?o seom:socMapping2020|seom:socMapping2010 ?s FILTER NOT EXISTS { ?s skos:notation ?n }"
        ),
    }
    return stats, integrity, len(eng.query(PRE + "SELECT * WHERE { ?s ?p ?o }")["rows"])


def main():
    print("Running SHACL validation ...")
    conforms, n_viol, text, n_data = run_shacl()
    print(f"  conforms={conforms} violations={n_viol} (data triples: {n_data})")
    if not conforms:
        print(text[:3000])

    print("Running SPARQL coverage (Oxigraph) ...")
    stats, integrity, n_triples = run_coverage()

    lines = ["# SEOM coverage and validation report", ""]
    lines.append(f"Total triples loaded: **{n_triples:,}**  ")
    lines.append(f"SHACL: **{'CONFORMS, 0 violations' if conforms else str(n_viol)+' violations'}**")
    lines.append("")
    lines.append("## Graph coverage")
    lines.append("")
    lines.append("| Entity | Count |")
    lines.append("|---|---:|")
    for k, v in stats.items():
        lines.append(f"| {k} | {v:,} |")
    lines.append("")
    lines.append("## Referential integrity (all must be 0)")
    lines.append("")
    lines.append("| Check | Violations |")
    lines.append("|---|---:|")
    for k, v in integrity.items():
        lines.append(f"| {k} | {v} |")
    lines.append("")
    lines.append(
        "Data source: Skills England Occupational Maps Public API. Contains public "
        "sector information licensed under the Open Government Licence v3.0."
    )
    report = "\n".join(lines) + "\n"
    with open(os.path.join(ONT, "coverage-report.md"), "w") as f:
        f.write(report)
    print("\n" + report)

    bad = (not conforms) or any(v for v in integrity.values())
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
