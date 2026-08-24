"""
Transform the raw MoDiP records into a CIDOC-CRM (Linked Art compatible) graph.

No bespoke class ontology: objects, production events, actors, places,
identifiers, titles, inscriptions and dimensions are modelled with standard
CIDOC-CRM classes/properties. Materials, techniques and use-domains are linked
to the SKOS concept schemes (which are themselves typed crm:E57_Material /
crm:E55_Type and aligned to Getty AAT).

Also computes the variant/same-mould DAG by resolving the accession-number
cross-references that MoDiP curators wrote into free-text descriptions
("AIBDC 007661.2 is the same box but a different colourway") into typed
object-to-object edges (crm:P130_shows_features_of).

Outputs (build/):
  modip-crm.ttl        the instance graph
  objectnames.ttl      E55 object-name types (controlled list)
  colours.ttl          E55 colour types
  dag_variants.ttl     object-to-object variant edges
  dag_variants.csv     source,target,relation  (for graph tooling)
  build_stats.json     counts used by BUILD_REPORT and the article
"""
import json, os, re, sys, collections
import rdflib
from rdflib import Graph, URIRef, Literal, RDF, RDFS
from rdflib.namespace import XSD, DCTERMS, SKOS

sys.path.insert(0, os.path.dirname(__file__))
import materials_taxonomy as M
import process_taxonomy as P
import concept_taxonomy as D

ROOT = os.path.dirname(os.path.dirname(__file__))
CRM = rdflib.Namespace("http://www.cidoc-crm.org/cidoc-crm/")
B = "https://ontology.tesseract.academy/modip/"
OBJ = rdflib.Namespace(B + "object/")
PROD = rdflib.Namespace(B + "production/")
ACT = rdflib.Namespace(B + "actor/")
PLACE = rdflib.Namespace(B + "place/")
IDN = rdflib.Namespace(B + "id/")
INSC = rdflib.Namespace(B + "inscription/")
DIM = rdflib.Namespace(B + "dimension/")
TS = rdflib.Namespace(B + "timespan/")
MAT = rdflib.Namespace(B + "materials/")
PROC = rdflib.Namespace(B + "processes/")
DOM = rdflib.Namespace(B + "domains/")
COL = rdflib.Namespace(B + "colours/")
ONM = rdflib.Namespace(B + "objectnames/")
AAT = rdflib.Namespace("http://vocab.getty.edu/aat/")

MODIP = ACT["modip"]


def slug(s, n=48):
    return re.sub(r"[^a-z0-9]+", "-", s.lower()).strip("-")[:n] or "x"


def flatten(units):
    """Yield (type, value, [child units]) for every unit, recursively at top level;
    children are returned attached so callers can read qualifiers."""
    for u in units:
        yield u
        # note: we walk children explicitly where needed


def first(units, typ):
    for u in units:
        if u.get("type") == typ:
            return u.get("value")
    return None


def collect(units, typ):
    return [u.get("value") for u in units if u.get("type") == typ and u.get("value") is not None]


def child(u, typ):
    for c in u.get("units", []):
        if c.get("type") == typ:
            return c.get("value")
    return None


YEAR = re.compile(r"\b(1[6-9]\d\d|20\d\d)\b")
DIM_RE = re.compile(r"^\s*([a-z ]+?)\s+([\d.]+)\s*([a-zA-Z²³%]+)\s*$")
ACC_IN_TEXT = re.compile(r"AIBDC[\s:]*([0-9O][0-9O_.\-]*)", re.I)


def norm_acc(s):
    return re.sub(r"[^0-9a-z.]", "", s.lower().replace("aibdc", "").replace("o", "0"))


def load_raw():
    import gzip
    p = os.path.join(ROOT, "data", "raw", "modip_records.json")
    if os.path.exists(p):
        return json.load(open(p))
    return json.load(gzip.open(p + ".gz", "rt"))


def main():
    recs = load_raw()
    g = Graph()
    g.bind("crm", CRM); g.bind("skos", SKOS); g.bind("dct", DCTERMS)
    g.bind("rdfs", RDFS); g.bind("aat", AAT)
    for p, n in [("obj", OBJ), ("prod", PROD), ("actor", ACT), ("place", PLACE),
                 ("mat", MAT), ("proc", PROC), ("dom", DOM), ("col", COL), ("onm", ONM)]:
        g.bind(p, n)

    # keeper / owner
    g.add((MODIP, RDF.type, CRM.E74_Group))
    g.add((MODIP, RDFS.label, Literal(
        "Museum of Design in Plastics (MoDiP), Arts University Bournemouth", lang="en")))

    colours, objnames = set(), {}
    stats = collections.Counter()
    unreconciled_mat = collections.Counter()
    acc_index = {}      # normalised accession -> object uri
    obj_desc = {}       # object uri -> description text (for DAG pass 2)
    obj_own_acc = {}

    # ---- pass 1: objects ----
    for r in recs:
        units = r["@document"]["units"]
        adm = r.get("@admin", {})
        uuid = adm.get("uuid") or slug(first(units, "spectrum/object_number") or str(stats["obj"]))
        o = OBJ[uuid]
        stats["obj"] += 1
        g.add((o, RDF.type, CRM["E22_Human-Made_Object"]))

        title = first(units, "spectrum/title")
        if title:
            g.add((o, RDFS.label, Literal(title, lang="en")))
            t = URIRef(str(o) + "/title")
            g.add((o, CRM.P102_has_title, t))
            g.add((t, RDF.type, CRM.E35_Title))
            g.add((t, CRM.P190_has_symbolic_content, Literal(title, lang="en")))

        objnum = first(units, "spectrum/object_number")
        if objnum:
            idn = IDN[uuid]
            g.add((o, CRM.P1_is_identified_by, idn))
            g.add((idn, RDF.type, CRM.E42_Identifier))
            g.add((idn, CRM.P190_has_symbolic_content, Literal(objnum)))
            na = norm_acc(objnum)
            acc_index[na] = o
            obj_own_acc[str(o)] = na

        desc = first(units, "spectrum/brief_description")
        if desc:
            g.add((o, CRM.P3_has_note, Literal(desc, lang="en")))
            obj_desc[str(o)] = desc

        # materials -> concepts
        for mv in collect(units, "spectrum/material"):
            cid = M.resolve(mv)
            if cid:
                g.add((o, CRM.P45_consists_of, MAT[cid]))
                stats["mat_linked"] += 1
            else:
                unreconciled_mat[mv] += 1
                g.add((o, CRM.P45_consists_of, MAT["plastic_unidentified"]))

        # domain / associated concept -> P2_has_type
        for dv in collect(units, "spectrum/associated_concept"):
            did = D.resolve(dv)
            if did:
                g.add((o, CRM.P2_has_type, DOM[did]))

        # object name(s) -> E55 controlled type
        for onv in collect(units, "spectrum/object_name"):
            s = slug(onv)
            objnames[s] = onv
            g.add((o, CRM.P2_has_type, ONM[s]))

        # colour -> E55 colour type
        for cv in collect(units, "spectrum/colour"):
            s = slug(cv)
            colours.add((s, cv))
            g.add((o, CRM.P2_has_type, COL[s]))

        # dimensions
        for i, dv in enumerate(collect(units, "spectrum/dimension")):
            m = DIM_RE.match(dv)
            dnode = DIM[f"{uuid}-{i}"]
            g.add((o, CRM.P43_has_dimension, dnode))
            g.add((dnode, RDF.type, CRM.E54_Dimension))
            if m:
                dtype, val, unit = m.group(1).strip(), m.group(2), m.group(3)
                g.add((dnode, CRM.P2_has_type, Literal(dtype)))
                try:
                    g.add((dnode, CRM.P90_has_value, Literal(val, datatype=XSD.decimal)))
                except Exception:
                    pass
                g.add((dnode, CRM.P91_has_unit, Literal(unit)))
                stats["dim_parsed"] += 1
            else:
                g.add((dnode, RDFS.label, Literal(dv)))

        # inscriptions
        for i, u in enumerate([u for u in units if u.get("type") == "spectrum/inscription_content"]):
            content = u.get("value")
            if not content:
                continue
            ins = INSC[f"{uuid}-{i}"]
            g.add((o, CRM.P128_carries, ins))
            g.add((ins, RDF.type, CRM.E34_Inscription))
            g.add((ins, CRM.P190_has_symbolic_content, Literal(content, lang="en")))
            pos = child(u, "spectrum/inscription_position")
            if pos:
                g.add((ins, CRM.P3_has_note, Literal(f"position: {pos}")))
            meth = child(u, "spectrum/inscription_method")
            if meth:
                pid = P.resolve(meth)
                if pid:
                    g.add((ins, CRM.P32_used_general_technique, PROC[pid]))
            stats["inscription"] += 1

        # production event
        makers = [(u.get("value"), child(u, "spectrum/organisations_association"), "org")
                  for u in units if u.get("type") == "spectrum/object_production_organisation"]
        makers += [(u.get("value"), child(u, "spectrum/persons_association"), "person")
                   for u in units if u.get("type") == "spectrum/object_production_person"]
        techniques = collect(units, "spectrum/technique")
        places = collect(units, "spectrum/object_production_place")
        date = first(units, "spectrum/object_production_date")
        if makers or techniques or places or (date and date != "NULL"):
            pr = PROD[uuid]
            g.add((o, CRM.P108i_was_produced_by, pr))
            g.add((pr, RDF.type, CRM.E12_Production))
            for name, role, kind in makers:
                if not name or name.lower() in ("unknown", "null"):
                    continue
                a = ACT[slug(name)]
                g.add((pr, CRM.P14_carried_out_by, a))
                g.add((a, RDF.type, CRM.E21_Person if kind == "person" else CRM.E74_Group))
                g.add((a, RDFS.label, Literal(name)))
                if role:
                    g.add((pr, CRM.P3_has_note, Literal(f"{role}: {name}")))
                stats["maker_link"] += 1
            for tv in techniques:
                pid = P.resolve(tv)
                if pid:
                    g.add((pr, CRM.P32_used_general_technique, PROC[pid]))
                    stats["tech_linked"] += 1
            for pv in places:
                if pv and pv.lower() != "null":
                    pl = PLACE[slug(pv)]
                    g.add((pr, CRM.P7_took_place_at, pl))
                    g.add((pl, RDF.type, CRM.E53_Place))
                    g.add((pl, RDFS.label, Literal(pv)))
            if date and date.lower() != "null":
                ts = TS[uuid]
                g.add((pr, CRM["P4_has_time-span"], ts))
                g.add((ts, RDF.type, CRM["E52_Time-Span"]))
                g.add((ts, RDFS.label, Literal(date)))
                ym = YEAR.search(date)
                if ym:
                    g.add((ts, CRM.P82_at_some_time_within,
                           Literal(ym.group(1), datatype=XSD.gYear)))

        # keeper / owner / rights
        g.add((o, CRM.P50_has_current_keeper, MODIP))
        g.add((o, CRM.P52_has_current_owner, MODIP))
        lic = first(units, "ciim/license_url")
        if lic:
            g.add((o, DCTERMS.license, URIRef(lic)))
        g.add((o, DCTERMS.rightsHolder, MODIP))
        g.add((o, RDFS.seeAlso, URIRef(f"https://museumdata.uk/objects/{uuid}")))

    import gzip as _gz
    crm_path = os.path.join(ROOT, "build", "modip-crm.ttl")
    g.serialize(crm_path, format="turtle")
    with open(crm_path, "rb") as _f, _gz.open(crm_path + ".gz", "wb") as _o:
        _o.writelines(_f)

    # object-name & colour E55 controlled type lists -> standalone vocabularies
    onm_g = Graph(); onm_g.bind("crm", CRM); onm_g.bind("onm", ONM)
    for s, lab in sorted(objnames.items()):
        onm_g.add((ONM[s], RDF.type, CRM.E55_Type)); onm_g.add((ONM[s], RDFS.label, Literal(lab, lang="en")))
    onm_g.serialize(os.path.join(ROOT, "ontology", "objectnames.ttl"), format="turtle")
    col_g = Graph(); col_g.bind("crm", CRM); col_g.bind("col", COL)
    for s, lab in sorted(colours):
        col_g.add((COL[s], RDF.type, CRM.E55_Type)); col_g.add((COL[s], RDFS.label, Literal(lab, lang="en")))
    col_g.serialize(os.path.join(ROOT, "ontology", "colours.ttl"), format="turtle")

    # ---- pass 2: variant DAG from description cross-references ----
    dag = Graph(); dag.bind("crm", CRM); dag.bind("obj", OBJ)
    edges = []
    for src, desc in obj_desc.items():
        own = obj_own_acc.get(src)
        for ref in ACC_IN_TEXT.findall(desc):
            na = norm_acc("aibdc" + ref)
            tgt = acc_index.get(na)
            if tgt is None or str(tgt) == src or na == own:
                continue
            variant = bool(re.search(r"same|colourway|colorway|different colour|version|variant", desc, re.I))
            rel = "P130_shows_features_of" if variant else "P67_refers_to"
            dag.add((URIRef(src), CRM[rel], tgt))
            edges.append((src, str(tgt), rel))
    dag.serialize(os.path.join(ROOT, "build", "dag_variants.ttl"), format="turtle")
    with open(os.path.join(ROOT, "build", "dag_variants.csv"), "w") as f:
        f.write("source,target,relation\n")
        for s, t, r in edges:
            f.write(f"{s},{t},{r}\n")

    # connected components over the variant edges (undirected)
    adj = collections.defaultdict(set)
    for s, t, r in edges:
        adj[s].add(t); adj[t].add(s)
    seen, comps = set(), []
    for n in adj:
        if n in seen:
            continue
        stack, comp = [n], []
        while stack:
            x = stack.pop()
            if x in seen:
                continue
            seen.add(x); comp.append(x); stack.extend(adj[x] - seen)
        comps.append(len(comp))

    total_triples = len(g)
    stats_out = {
        "records": len(recs),
        "instance_triples": total_triples,
        "objects": stats["obj"],
        "material_concept_links": stats["mat_linked"],
        "unreconciled_material_assertions": sum(unreconciled_mat.values()),
        "technique_links": stats["tech_linked"],
        "maker_links": stats["maker_link"],
        "inscriptions": stats["inscription"],
        "dimensions_parsed": stats["dim_parsed"],
        "object_name_types": len(objnames),
        "colour_types": len(colours),
        "dag_edges": len(edges),
        "dag_components": len(comps),
        "dag_largest_component": max(comps) if comps else 0,
        "dag_nodes": len(adj),
        "top_unreconciled_materials": unreconciled_mat.most_common(25),
    }
    json.dump(stats_out, open(os.path.join(ROOT, "build", "build_stats.json"), "w"), indent=2)
    for k, v in stats_out.items():
        if k != "top_unreconciled_materials":
            print(f"  {k}: {v}")


if __name__ == "__main__":
    main()
