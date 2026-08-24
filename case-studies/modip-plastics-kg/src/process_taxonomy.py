"""
MoDiP Manufacturing-Process (Technique) Taxonomy — SKOS source of truth.

Groups the 82 free-text technique strings used across the collection into a
hierarchy of polymer-processing and general fabrication methods. The raw string
is preserved as the leaf prefLabel; the broader groups are the added structure.
"""

PROCESSES = {}


def P(pid, pref, broader=None, alt=None, note=None):
    PROCESSES[pid] = dict(pref=pref, broader=broader, alt=alt or [], note=note)


P("process", "manufacturing process")

# --- moulding -------------------------------------------------------------
P("moulding", "moulding", "process", alt=["moulded"])
P("injection_moulding", "injection moulding", "moulding", alt=["injection moulded"])
P("insert_moulding", "insert moulding", "injection_moulding", alt=["insert moulded"])
P("gas_injection", "gas-assisted injection moulding", "injection_moulding",
  alt=["gas assisted injection moulded"])
P("rim", "reaction injection moulding", "injection_moulding", alt=["reaction injection moulded"])
P("blow_moulding", "blow moulding", "moulding", alt=["blow moulded"])
P("ext_blow", "extrusion blow moulding", "blow_moulding", alt=["extrusion blow moulded"])
P("inj_blow", "injection blow moulding", "blow_moulding", alt=["injection blow moulded"])
P("inj_stretch_blow", "injection stretch blow moulding", "blow_moulding",
  alt=["injection stretch blow moulded"])
P("dip_moulding", "dip moulding", "moulding", alt=["dip moulded"])
P("compression_moulding", "compression moulding", "moulding", alt=["compression moulded"])
P("rotational_moulding", "rotational moulding", "moulding",
  alt=["rotational moulded", "rotocast", "rotationally moulded"])
P("rtm", "resin transfer moulding", "moulding", alt=["resin transfer moulded"])

# --- extrusion & film -----------------------------------------------------
P("extrusion", "extrusion", "process", alt=["extruded"])
P("blown_film", "blown film extrusion", "extrusion", alt=["blown film extruded"])
P("melt_blown", "melt blowing", "extrusion", alt=["melt blown"])
P("pultrusion", "pultrusion", "extrusion", alt=["pultruded"])

# --- forming --------------------------------------------------------------
P("forming", "forming", "process")
P("thermoforming", "thermoforming", "forming", alt=["thermoformed"])
P("vacuum_forming", "vacuum forming", "thermoforming", alt=["vacuum formed"])
P("calendering", "calendering", "forming", alt=["calendered"])
P("pressing", "pressing", "forming", alt=["pressed", "stamped"])
P("folding", "folding", "forming", alt=["folded"])
P("drawing_form", "drawing", "forming", alt=["drawn"])

# --- casting & foaming ----------------------------------------------------
P("casting", "casting", "process", alt=["cast"])
P("die_casting", "die casting", "casting", alt=["die cast"])
P("foaming", "foaming", "process", alt=["foamed"])

# --- additive -------------------------------------------------------------
P("additive", "additive manufacturing", "process", alt=["3D printed", "3d printed"])

# --- machining / subtractive ---------------------------------------------
P("machining", "machining and cutting", "process", alt=["machined"])
P("cutting", "cutting", "machining", alt=["cut", "machine cut", "chipped", "shredded"])
P("laser_cut", "laser cutting", "cutting", alt=["laser cut"])
P("lathe_cut", "lathe cutting", "cutting", alt=["lathe cut"])
P("turning", "turning", "machining", alt=["turned"])
P("drilling", "drilling", "machining", alt=["drilled"])
P("routing", "routing", "machining", alt=["routed"])
P("grinding", "grinding", "machining", alt=["ground", "shaved"])
P("carving", "carving", "machining", alt=["carved", "hollowed out"])

# --- surface & decoration -------------------------------------------------
P("surface", "surface treatment and decoration", "process")
P("printing", "printing", "surface", alt=["printed", "photocopied"])
P("painting", "painting", "surface", alt=["painted"])
P("plating", "plating", "surface", alt=["plated", "vapour deposition", "metallised"])
P("polishing", "polishing", "surface", alt=["polished"])
P("flocking", "flocking", "surface", alt=["flocked", "napped"])
P("etching", "etching", "surface", alt=["etched", "laser etched"])
P("engraving", "engraving", "surface", alt=["engraved"])
P("dyeing", "dyeing", "surface", alt=["batch dyed"])

# --- joining & assembly ---------------------------------------------------
P("joining", "joining and assembly", "process", alt=["fabricated", "assembled"])
P("welding", "welding", "joining", alt=["welded"])
P("heat_welding", "heat welding", "welding", alt=["heat welded", "thermal bonded"])
P("hf_welding", "high-frequency welding", "welding",
  alt=["high frequency welded", "radio frequency welded"])
P("bonding", "bonding", "joining", alt=["bonded"])
P("lamination", "lamination", "joining", alt=["laminated"])

# --- textile processes ----------------------------------------------------
P("textileproc", "textile process", "process")
P("weaving", "weaving", "textileproc", alt=["woven", "tricot"])
P("knitting", "knitting", "textileproc", alt=["knitted", "hand knitted"])
P("crochet", "crochet", "textileproc", alt=["crocheted"])
P("stitching", "stitching", "textileproc", alt=["stitched"])
P("spinning", "spinning", "textileproc", alt=["spun"])
P("braiding", "braiding", "textileproc", alt=["braided", "knotted"])
P("tufting", "tufting", "textileproc", alt=["tufted", "needle punched"])

# --- hand & documentary ---------------------------------------------------
P("handwork", "hand production", "process",
  alt=["handmade", "hand made", "home made", "hand finished", "hand laid"])
P("documentary", "documentary production", "process",
  alt=["typed", "handwritten"])
P("heating", "heating", "process", alt=["heated"])
P("unknown_proc", "unknown process", "process", alt=["unknown"])


def _norm(s):
    return s.strip().lower()


def _alt_index():
    idx = {}
    for pid, p in PROCESSES.items():
        idx.setdefault(_norm(p["pref"]), pid)
        for a in p["alt"]:
            idx.setdefault(_norm(a), pid)
    return idx


_IDX = _alt_index()


def resolve(raw):
    return _IDX.get(_norm(raw))


if __name__ == "__main__":
    import sys
    print(f"processes: {len(PROCESSES)}")
    if len(sys.argv) > 1:
        total = mapped = 0
        un = []
        for line in open(sys.argv[1]):
            n, t = line.rstrip("\n").split("\t"); n = int(n)
            total += n
            if resolve(t):
                mapped += n
            else:
                un.append((n, t))
        print(f"technique assertions: {total}  mapped: {mapped} ({100*mapped/total:.1f}%)  unmapped: {len(un)}")
        for n, t in sorted(un, reverse=True)[:20]:
            print(f"  UNMAPPED {n:5d}  {t}")
