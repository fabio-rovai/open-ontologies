#!/usr/bin/env python3
"""
Build the Skills England Occupational Maps Ontology (SEOM) from a static
snapshot of the Skills England Occupational Maps Public API.

Inputs (in ./data, harvested once from the Public API under an OGL v3.0 licence):
  reference.json                routes, green themes, statuses, technical levels, product types, API version
  occupations-list.json         all occupational standards (summary records)
  occupation-details.json       full detail per standard (expanded: soc, mapHierarchy, products, job titles, ...)
  progression.json              occupational progression maps (directed occupation -> occupation edges)
  occupation-green-themes.json  occupation stdCode -> [green theme ids]

Outputs (in ./ontology):
  seom-vocabulary.ttl / .jsonld   TBox: classes, properties, and the SKOS classification schemes
                                  (Route > Pathway > Cluster hierarchy, green themes, SOC 2010/2020 crosswalk)
  occupational-map.ttl / .jsonld  ABox: the 1,200+ occupational standards as instances with all relations
  shapes.ttl                      SHACL shapes constraining the instance data

Data source: Skills England Occupational Maps Public API. Contains public sector
information licensed under the Open Government Licence v3.0.
"""
import json
import os
import re
from urllib.parse import quote
from rdflib import Graph, Namespace, Literal, URIRef, BNode
from rdflib.namespace import RDF, RDFS, SKOS, DCTERMS, OWL, XSD, PROV

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.join(HERE, "data")
OUT = os.path.join(HERE, "ontology")
os.makedirs(OUT, exist_ok=True)

BASE = "https://gov.tesseract.academy/ns/seom#"
SEOM = Namespace(BASE)
# Instance IRIs live under a resource namespace.
RES = Namespace("https://gov.tesseract.academy/id/seom/")
API_LICENCE = URIRef("https://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/")
API_HOME = URIRef("https://occupational-maps.skillsengland.education.gov.uk/public-api/")
SE_ORG = URIRef("https://www.gov.uk/government/organisations/skills-england")

HARVEST_DATE = "2026-07-06"


def load(name):
    with open(os.path.join(DATA, name)) as f:
        return json.load(f)


def strip_html(s):
    if not s:
        return None
    s = re.sub(r"</p>", "\n", s, flags=re.I)
    s = re.sub(r"<br\s*/?>", "\n", s, flags=re.I)
    s = re.sub(r"<[^>]+>", "", s)
    s = (s.replace("&nbsp;", " ").replace("&amp;", "&").replace("&lt;", "<")
           .replace("&gt;", ">").replace("&#39;", "'").replace("&rsquo;", "'")
           .replace("&ldquo;", '"').replace("&rdquo;", '"').replace("&quot;", '"'))
    s = re.sub(r"[ \t]+", " ", s)
    s = re.sub(r"\n\s*\n+", "\n", s).strip()
    return s or None


# Resource IRI helpers. Codes may carry stray whitespace in the source data, so
# every code-derived IRI segment is trimmed and percent-encoded.
def _seg(x):          return quote(str(x).strip(), safe="")
def occ(code):        return RES[f"occupation/{_seg(code)}"]
def route(i):         return RES[f"route/{i}"]
def pathway(i):       return RES[f"pathway/{i}"]
def cluster(i):       return RES[f"cluster/{i}"]
def techlevel(i):     return RES[f"technical-level/{i}"]
def status(i):        return RES[f"status/{i}"]
def product(code):    return RES[f"product/{_seg(code)}"]
def producttype(i):   return RES[f"product-type/{i}"]
def soc(ver, code):   return RES[f"soc/{ver}/{_seg(code)}"]
def greentheme(i):    return RES[f"green-theme/{i}"]
def greensub(i):      return RES[f"green-subtheme/{i}"]


# ---------------------------------------------------------------------------
# 1. VOCABULARY (TBox) + classification schemes
# ---------------------------------------------------------------------------
def build_vocabulary(ref, details):
    g = Graph()
    for p, ns in [("seom", SEOM), ("skos", SKOS), ("dct", DCTERMS), ("owl", OWL),
                  ("rdfs", RDFS), ("prov", PROV), ("res", RES)]:
        g.bind(p, ns)

    onto = URIRef(BASE.rstrip("#"))
    g.add((onto, RDF.type, OWL.Ontology))
    g.add((onto, DCTERMS.title, Literal("Skills England Occupational Maps Ontology (SEOM)")))
    g.add((onto, DCTERMS.description, Literal(
        "An open, machine-readable ontology of the Skills England occupational maps: "
        "occupational standards and their routes, pathways and clusters, their Standard "
        "Occupational Classification (SOC 2010 and 2020) mappings, their apprenticeship and "
        "technical education products, their green-jobs classification, and the progression "
        "relationships between them. Built from a static snapshot of the Skills England "
        "Occupational Maps Public API.")))
    g.add((onto, DCTERMS.creator, URIRef("https://gov.tesseract.academy/#organization")))
    g.add((onto, DCTERMS.source, API_HOME))
    g.add((onto, DCTERMS.license, API_LICENCE))
    g.add((onto, DCTERMS.created, Literal(HARVEST_DATE, datatype=XSD.date)))
    g.add((onto, RDFS.comment, Literal(
        "Contains public sector information from Skills England licensed under the Open "
        "Government Licence v3.0.")))
    info = ref.get("info")
    if isinstance(info, str):
        g.add((onto, PROV.wasDerivedFrom, Literal(info.strip())))

    # ---- Classes ----
    classes = {
        "Occupation": "An occupational standard published by Skills England (a defined occupation with a standard code, e.g. OCC0118).",
        "Route": "A top-level occupational route (one of 15), the broadest grouping of the occupational map.",
        "Pathway": "A pathway within an occupational route.",
        "Cluster": "A cluster of related occupations within a pathway.",
        "TechnicalLevel": "A technical level of an occupational standard (Technical, Higher Technical, Professional).",
        "OccupationStatus": "The lifecycle status of an occupational standard (e.g. Approved occupation).",
        "Product": "A technical education product through which an occupation is delivered (e.g. an apprenticeship, HTQ or T Level).",
        "ProductType": "The type of a technical education product (Apprenticeship, HTQ, TLevel, TQ, ...).",
        "SOCConcept": "A UK Standard Occupational Classification (ONS SOC) code that an occupation maps to.",
        "GreenTheme": "A Skills England green-jobs theme (occupations underpinning the net-zero transition).",
    }
    for name, desc in classes.items():
        c = SEOM[name]
        g.add((c, RDF.type, OWL.Class))
        g.add((c, RDFS.label, Literal(name)))
        g.add((c, RDFS.comment, Literal(desc)))
    for sub in ("Route", "Pathway", "Cluster", "SOCConcept", "GreenTheme"):
        g.add((SEOM[sub], RDFS.subClassOf, SKOS.Concept))

    # ---- Object properties ----
    objprops = {
        "inRoute": ("Occupation", "Route", "The route this occupational standard belongs to."),
        "inPathway": ("Occupation", "Pathway", "The pathway this occupational standard belongs to."),
        "inCluster": ("Occupation", "Cluster", "The cluster this occupational standard belongs to."),
        "atTechnicalLevel": ("Occupation", "TechnicalLevel", "The technical level of this occupational standard."),
        "hasStatus": ("Occupation", "OccupationStatus", "The lifecycle status of this occupational standard."),
        "socMapping2020": ("Occupation", "SOCConcept", "The SOC 2020 code this occupation maps to."),
        "socMapping2010": ("Occupation", "SOCConcept", "The SOC 2010 code this occupation maps to."),
        "deliveredThrough": ("Occupation", "Product", "A technical education product delivering this occupation."),
        "progressesTo": ("Occupation", "Occupation", "An occupation a holder of this occupation can progress to."),
        "inGreenTheme": ("Occupation", "GreenTheme", "A green-jobs theme this occupation contributes to."),
        "hasProductType": ("Product", "ProductType", "The type of this technical education product."),
    }
    for name, (dom, rng, desc) in objprops.items():
        p = SEOM[name]
        g.add((p, RDF.type, OWL.ObjectProperty))
        g.add((p, RDFS.label, Literal(name)))
        g.add((p, RDFS.domain, SEOM[dom]))
        g.add((p, RDFS.range, SEOM[rng]))
        g.add((p, RDFS.comment, Literal(desc)))
    g.add((SEOM.progressesTo, RDF.type, OWL.TransitiveProperty))

    # ---- Datatype properties ----
    dataprops = {
        "stdCode": "The Skills England standard code (e.g. OCC0118).",
        "level": "The level of the occupational standard.",
        "versionNo": "The version of the occupational standard.",
        "overview": "The occupation overview text.",
        "involvedEmployers": "Employers involved in defining the standard.",
        "keyword": "A keyword associated with the occupation.",
        "typicalJobTitle": "A typical job title for the occupation.",
        "greenJobTitle": "A typical job title flagged as a green job.",
        "statusLastUpdated": "The date the standard's status was last updated.",
        "productCode": "The product code of a technical education product (e.g. ST0118).",
        "socVersion": "The SOC edition of a SOC concept (2010 or 2020).",
        "medianAnnualSalaryGBP": "Median annual salary in GBP, where published.",
    }
    for name, desc in dataprops.items():
        p = SEOM[name]
        g.add((p, RDF.type, OWL.DatatypeProperty))
        g.add((p, RDFS.label, Literal(name)))
        g.add((p, RDFS.comment, Literal(desc)))

    # ---- Reference individuals ----
    for s in ref.get("occupationStates", []):
        n = status(s["id"])
        g.add((n, RDF.type, SEOM.OccupationStatus))
        g.add((n, RDFS.label, Literal(s["name"])))
    for t in ref.get("technicalLevels", []):
        n = techlevel(t["id"])
        g.add((n, RDF.type, SEOM.TechnicalLevel))
        g.add((n, RDFS.label, Literal(t["name"])))
    for pt in ref.get("productTypes", []):
        n = producttype(pt["id"])
        g.add((n, RDF.type, SEOM.ProductType))
        g.add((n, RDFS.label, Literal(pt["name"])))

    # ---- SKOS scheme: Route > Pathway > Cluster ----
    om_scheme = RES["scheme/occupational-map"]
    g.add((om_scheme, RDF.type, SKOS.ConceptScheme))
    g.add((om_scheme, DCTERMS.title, Literal("Skills England occupational map classification")))
    g.add((om_scheme, DCTERMS.source, API_HOME))

    # Routes from reference; pathway/cluster relationships from occupation mapHierarchy.
    routes_seen, pathways, clusters = {}, {}, {}
    for r in ref.get("routes", []):
        routes_seen[r["routeId"]] = r["name"]
    for d in details:
        if not isinstance(d, dict):
            continue
        mh = d.get("mapHierarchy") or {}
        if mh.get("routeId") is not None:
            routes_seen.setdefault(mh["routeId"], mh.get("routeName"))
        if mh.get("pathwayId") is not None:
            pathways[mh["pathwayId"]] = (mh.get("pathwayName"), mh.get("routeId"))
        if mh.get("clusterId") is not None:
            clusters[mh["clusterId"]] = (mh.get("clusterName"), mh.get("pathwayId"))

    for rid, rname in routes_seen.items():
        n = route(rid)
        g.add((n, RDF.type, SEOM.Route))
        g.add((n, SKOS.prefLabel, Literal(rname)))
        g.add((n, SKOS.inScheme, om_scheme))
        g.add((n, SKOS.topConceptOf, om_scheme))
        g.add((om_scheme, SKOS.hasTopConcept, n))
    for pid, (pname, rid) in pathways.items():
        n = pathway(pid)
        g.add((n, RDF.type, SEOM.Pathway))
        if pname:
            g.add((n, SKOS.prefLabel, Literal(pname)))
        g.add((n, SKOS.inScheme, om_scheme))
        if rid is not None and rid in routes_seen:
            g.add((n, SKOS.broader, route(rid)))
            g.add((route(rid), SKOS.narrower, n))
    for cid, (cname, pid) in clusters.items():
        n = cluster(cid)
        g.add((n, RDF.type, SEOM.Cluster))
        if cname:
            g.add((n, SKOS.prefLabel, Literal(cname)))
        g.add((n, SKOS.inScheme, om_scheme))
        if pid is not None and pid in pathways:
            g.add((n, SKOS.broader, pathway(pid)))
            g.add((pathway(pid), SKOS.narrower, cluster(cid)))

    # ---- SKOS scheme: green themes ----
    green_scheme = RES["scheme/green-themes"]
    g.add((green_scheme, RDF.type, SKOS.ConceptScheme))
    g.add((green_scheme, DCTERMS.title, Literal("Skills England green occupation themes")))
    for t in ref.get("greenThemes", []):
        n = greentheme(t["themeId"])
        g.add((n, RDF.type, SEOM.GreenTheme))
        g.add((n, SKOS.prefLabel, Literal(t["themeName"])))
        g.add((n, SKOS.inScheme, green_scheme))
        g.add((n, SKOS.topConceptOf, green_scheme))
        g.add((green_scheme, SKOS.hasTopConcept, n))
        for sub in t.get("subThemes", []) or []:
            sn = greensub(sub["themeId"])
            g.add((sn, RDF.type, SKOS.Concept))
            g.add((sn, SKOS.prefLabel, Literal(sub["themeName"])))
            g.add((sn, SKOS.inScheme, green_scheme))
            g.add((sn, SKOS.broader, n))
            g.add((n, SKOS.narrower, sn))

    # ---- SOC crosswalk schemes (concepts collected from occupation detail) ----
    soc_2020 = RES["scheme/soc-2020"]
    soc_2010 = RES["scheme/soc-2010"]
    g.add((soc_2020, RDF.type, SKOS.ConceptScheme))
    g.add((soc_2020, DCTERMS.title, Literal("ONS SOC 2020 codes referenced by the occupational maps")))
    g.add((soc_2010, RDF.type, SKOS.ConceptScheme))
    g.add((soc_2010, DCTERMS.title, Literal("ONS SOC 2010 codes referenced by the occupational maps")))
    soc_seen = set()
    for d in details:
        if not isinstance(d, dict):
            continue
        s = d.get("soc") or {}
        for ver, code_k, desc_k, scheme in [("2020", "soc2020Code", "soc2020Description", soc_2020),
                                            ("2010", "soc2010Code", "soc2010Description", soc_2010)]:
            code = s.get(code_k)
            if code is None or code == 0:
                continue
            key = (ver, code)
            if key in soc_seen:
                continue
            soc_seen.add(key)
            n = soc(ver, code)
            g.add((n, RDF.type, SEOM.SOCConcept))
            g.add((n, SKOS.notation, Literal(str(code))))
            g.add((n, SEOM.socVersion, Literal(ver)))
            g.add((n, SKOS.inScheme, scheme))
            if s.get(desc_k):
                g.add((n, SKOS.prefLabel, Literal(s[desc_k].strip())))
    return g


# ---------------------------------------------------------------------------
# 2. INSTANCE DATA (ABox): occupations, products, progression, green links
# ---------------------------------------------------------------------------
def build_instances(details, progression, green_map):
    g = Graph()
    for p, ns in [("seom", SEOM), ("skos", SKOS), ("dct", DCTERMS), ("rdfs", RDFS), ("res", RES)]:
        g.bind(p, ns)

    products_seen = set()
    for d in details:
        if not isinstance(d, dict) or not d.get("stdCode"):
            continue
        code = d["stdCode"]
        n = occ(code)
        g.add((n, RDF.type, SEOM.Occupation))
        g.add((n, RDFS.label, Literal(d.get("name", code))))
        g.add((n, SKOS.prefLabel, Literal(d.get("name", code))))
        g.add((n, SEOM.stdCode, Literal(code)))
        if d.get("level") is not None:
            g.add((n, SEOM.level, Literal(int(d["level"]), datatype=XSD.integer)))
        if d.get("versionNo"):
            g.add((n, SEOM.versionNo, Literal(str(d["versionNo"]))))
        if d.get("status") is not None:
            g.add((n, SEOM.hasStatus, status(d["status"])))
        if d.get("statusLastUpdated"):
            g.add((n, SEOM.statusLastUpdated, Literal(d["statusLastUpdated"], datatype=XSD.date)))
        summ = strip_html(d.get("summary"))
        if summ:
            g.add((n, DCTERMS.description, Literal(summ)))
        ov = strip_html(d.get("overview"))
        if ov:
            g.add((n, SEOM.overview, Literal(ov)))
        emp = strip_html(d.get("involvedEmployers"))
        if emp:
            g.add((n, SEOM.involvedEmployers, Literal(emp)))
        if d.get("medianAnnualSalaryinGBP"):
            g.add((n, SEOM.medianAnnualSalaryGBP, Literal(int(d["medianAnnualSalaryinGBP"]), datatype=XSD.integer)))
        for kw in d.get("keywords") or []:
            if kw:
                g.add((n, SEOM.keyword, Literal(kw)))
        for jt in d.get("typicalJobTitles") or []:
            title = jt.get("name")
            if not title:
                continue
            g.add((n, SEOM.typicalJobTitle, Literal(title)))
            if jt.get("isGreen"):
                g.add((n, SEOM.greenJobTitle, Literal(title)))

        mh = d.get("mapHierarchy") or {}
        if mh.get("routeId") is not None:
            g.add((n, SEOM.inRoute, route(mh["routeId"])))
        if mh.get("pathwayId") is not None:
            g.add((n, SEOM.inPathway, pathway(mh["pathwayId"])))
        if mh.get("clusterId") is not None:
            g.add((n, SEOM.inCluster, cluster(mh["clusterId"])))
        if mh.get("technicalLevel") is not None:
            g.add((n, SEOM.atTechnicalLevel, techlevel(mh["technicalLevel"])))

        s = d.get("soc") or {}
        if s.get("soc2020Code"):
            g.add((n, SEOM.socMapping2020, soc("2020", s["soc2020Code"])))
        if s.get("soc2010Code"):
            g.add((n, SEOM.socMapping2010, soc("2010", s["soc2010Code"])))

        for p in d.get("products") or []:
            pcode = p.get("productCode")
            if not pcode:
                continue
            pn = product(pcode)
            g.add((n, SEOM.deliveredThrough, pn))
            if pcode not in products_seen:
                products_seen.add(pcode)
                g.add((pn, RDF.type, SEOM.Product))
                g.add((pn, RDFS.label, Literal(p.get("name", pcode))))
                g.add((pn, SEOM.productCode, Literal(pcode)))
                if p.get("level") is not None:
                    g.add((pn, SEOM.level, Literal(int(p["level"]), datatype=XSD.integer)))
                if p.get("type") is not None:
                    g.add((pn, SEOM.hasProductType, producttype(p["type"])))
                if p.get("statusName"):
                    g.add((pn, DCTERMS.description, Literal(p["statusName"])))

    # Green theme links
    for code, theme_ids in green_map.items():
        n = occ(code)
        for tid in theme_ids:
            g.add((n, SEOM.inGreenTheme, greentheme(tid)))

    # Progression edges (directed)
    edges = 0
    for pmap in progression:
        if not isinstance(pmap, dict):
            continue
        for e in pmap.get("progressions") or []:
            fr, to = e.get("stdCodeFrom"), e.get("stdCodeTo")
            if fr and to:
                g.add((occ(fr), SEOM.progressesTo, occ(to)))
                edges += 1
    return g, edges


def main():
    ref = load("reference.json")
    details = load("occupation-details.json")
    progression = load("progression.json")
    green_map = load("occupation-green-themes.json")

    print("Building vocabulary (TBox) ...")
    vocab = build_vocabulary(ref, details)
    vocab.serialize(os.path.join(OUT, "seom-vocabulary.ttl"), format="turtle")
    vocab.serialize(os.path.join(OUT, "seom-vocabulary.jsonld"), format="json-ld", auto_compact=True)
    print(f"  vocabulary triples: {len(vocab)}")

    print("Building instance data (ABox) ...")
    inst, edges = build_instances(details, progression, green_map)
    inst.serialize(os.path.join(OUT, "occupational-map.ttl"), format="turtle")
    inst.serialize(os.path.join(OUT, "occupational-map.jsonld"), format="json-ld", auto_compact=True)
    print(f"  instance triples: {len(inst)} (progression edges: {edges})")
    print("Done.")


if __name__ == "__main__":
    main()
