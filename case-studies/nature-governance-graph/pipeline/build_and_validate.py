#!/usr/bin/env python3
"""Build the UK Nature Governance Graph from sourced JSON and validate it with SHACL.

Reads data/entities.json (actors + sourced relationships), emits graph.ttl as a
provenance-first RDF graph in which every relationship is reified and carries a
prov:wasDerivedFrom link to a cited Source, then runs the SHACL shapes and
referential-integrity SPARQL checks and writes metrics.json.

No relationship is emitted without a citation URL; the SHACL SourceShape and
RelationshipShape make that a hard, machine-checked guarantee.
"""
import hashlib
import json
from pathlib import Path

from rdflib import Graph, Literal, Namespace, RDF, RDFS, URIRef
from rdflib.namespace import DCTERMS, XSD, PROV
import pyshacl

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "data" / "entities.json"
ONTO = ROOT / "ontology" / "ngg.ttl"
SHAPES = ROOT / "shapes" / "ngg-shapes.ttl"
GRAPH_OUT = ROOT / "graph.ttl"
METRICS_OUT = ROOT / "metrics.json"

NGG = Namespace("https://tesseract.academy/ns/ngg#")
BASE = "https://tesseract.academy/id/ngg/"

CLASS_MAP = {
    "statutory-agency": NGG.StatutoryAgency,
    "ngo": NGG.NGO,
    "data-body": NGG.DataBody,
    "funder": NGG.Funder,
    "sector-body": NGG.SectorBody,
    "partnership": NGG.Partnership,
    "international": NGG.International,
}


def source_uri(url: str) -> URIRef:
    h = hashlib.sha1(url.encode()).hexdigest()[:12]
    return URIRef(BASE + "source/" + h)


def build() -> tuple[Graph, dict]:
    data = json.loads(DATA.read_text())
    g = Graph()
    g.bind("ngg", NGG)
    g.bind("prov", PROV)
    g.bind("dcterms", DCTERMS)

    ids = set()
    for e in data["entities"]:
        uri = URIRef(BASE + "actor/" + e["id"])
        ids.add(e["id"])
        g.add((uri, RDF.type, NGG.Actor))
        g.add((uri, RDF.type, CLASS_MAP[e["class"]]))
        g.add((uri, RDFS.label, Literal(e["name"], datatype=XSD.string)))
        g.add((uri, NGG.actorClass, Literal(e["class"])))
        g.add((uri, NGG.role, Literal(e["role"], datatype=XSD.string)))
        if e.get("url"):
            g.add((uri, RDFS.seeAlso, URIRef(e["url"])))

    sources = {}
    dangling = []
    for i, r in enumerate(data["relationships"]):
        if r["from"] not in ids or r["to"] not in ids:
            dangling.append(r)
            continue
        rel = URIRef(BASE + f"rel/{i:03d}")
        g.add((rel, RDF.type, NGG.Relationship))
        g.add((rel, NGG.relFrom, URIRef(BASE + "actor/" + r["from"])))
        g.add((rel, NGG.relTo, URIRef(BASE + "actor/" + r["to"])))
        g.add((rel, NGG.relType, Literal(r["type"])))
        g.add((rel, NGG.basis, Literal(r["basis"], datatype=XSD.string)))
        suri = source_uri(r["citation_url"])
        if r["citation_url"] not in sources:
            g.add((suri, RDF.type, NGG.Source))
            g.add((suri, NGG.sourceUrl, Literal(r["citation_url"], datatype=XSD.anyURI)))
            sources[r["citation_url"]] = suri
        g.add((rel, PROV.wasDerivedFrom, suri))

    metrics = {
        "actors": len(ids),
        "relationships": len(data["relationships"]) - len(dangling),
        "sources": len(sources),
        "dangling_relationships": len(dangling),
        "actor_classes": {c: sum(1 for e in data["entities"] if e["class"] == c) for c in CLASS_MAP},
        "relationship_types": {},
    }
    for r in data["relationships"]:
        metrics["relationship_types"][r["type"]] = metrics["relationship_types"].get(r["type"], 0) + 1
    return g, metrics, dangling


REF_INTEGRITY = """
PREFIX ngg: <https://tesseract.academy/ns/ngg#>
SELECT (COUNT(*) AS ?bad) WHERE {
  ?r a ngg:Relationship ; ?p ?actor .
  FILTER(?p IN (ngg:relFrom, ngg:relTo))
  FILTER NOT EXISTS { ?actor a ngg:Actor }
}
"""


def main() -> None:
    g, metrics, dangling = build()
    # Merge ontology for SHACL class-based validation
    data_graph = Graph()
    data_graph.parse(ONTO, format="turtle")
    data_graph += g

    shapes_graph = Graph()
    shapes_graph.parse(SHAPES, format="turtle")

    conforms, _, report_text = pyshacl.validate(
        data_graph, shacl_graph=shapes_graph, inference="rdfs",
        abort_on_first=False, meta_shacl=False,
    )
    ref_bad = int(list(g.query(REF_INTEGRITY))[0][0])

    metrics["shacl_conforms"] = bool(conforms)
    metrics["referential_integrity_violations"] = ref_bad
    metrics["dangling_dropped"] = [f'{d["from"]}->{d["to"]}' for d in dangling]

    g.serialize(GRAPH_OUT, format="turtle")
    METRICS_OUT.write_text(json.dumps(metrics, indent=2))

    print(f"actors={metrics['actors']} relationships={metrics['relationships']} "
          f"sources={metrics['sources']}")
    print(f"SHACL conforms: {conforms}")
    print(f"referential-integrity violations: {ref_bad}")
    if not conforms:
        print(report_text[:3000])


if __name__ == "__main__":
    main()
