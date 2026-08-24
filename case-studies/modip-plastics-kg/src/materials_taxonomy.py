"""
MoDiP Materials Taxonomy — single source of truth.

A SKOS concept scheme for the materials of design-in-plastics collections,
grounded in polymer science. Two facets are modelled explicitly because they
cross-cut the chemical-family tree:

  * thermal behaviour: thermoplastic | thermoset | elastomer | thermoplastic-elastomer
  * origin:            synthetic | semi-synthetic | bio-based | natural

The chemical family is the primary skos:broader tree. Abbreviations and
commercial trade names are folded in as skos:altLabel on the generic polymer
concept, which is what lets a search for "polycarbonate" also return records
tagged "PC" or "Lexan".

Nothing here is invented: every abbreviation, trade name and family assignment
below reflects standard polymer nomenclature. External alignments (Getty AAT,
Wikidata) are asserted ONLY where verified and are otherwise left for manual
review (see BUILD_REPORT.md) rather than guessed.

CONCEPTS[id] = dict(
    pref      = preferred label (generic chemical / material name),
    alt       = [alternate labels: abbreviations, trade names, spellings],
    broader   = parent concept id (None => top concept of the scheme),
    thermal   = one of {thermoplastic, thermoset, elastomer, tpe} or None,
    origin    = one of {synthetic, semisynthetic, biobased, natural} or None,
    note      = optional scope note,
)
"""

CONCEPTS = {}


def C(cid, pref, broader=None, alt=None, thermal=None, origin=None, note=None):
    CONCEPTS[cid] = dict(pref=pref, broader=broader, alt=alt or [],
                         thermal=thermal, origin=origin, note=note)


# ---------------------------------------------------------------- top concepts
C("material", "material", note="Top concept for all physical materials in the collection.")
C("polymer", "polymer", "material",
  alt=["plastic"],
  note="Macromolecular material. 'plastic' is used in the collection as a broad, "
       "unspecified tag and is treated as an alt label of this top concept.")
C("nonplastic", "non-plastic material", "material",
  note="Materials that are not polymers (co-materials of objects, or reference items).")

# ------------------------------------------------------------- polymer by origin
C("synthetic", "synthetic polymer", "polymer", origin="synthetic")
C("semisynthetic", "semi-synthetic polymer", "polymer", origin="semisynthetic",
  note="Polymers made by chemical modification of a natural macromolecule "
       "(e.g. cellulose, casein).")
C("biopolymer", "biopolymer / bioplastic", "polymer", origin="biobased",
  alt=["biopolymer", "plastic (natural)"],
  note="Bio-based and/or biodegradable polymers.")
C("naturalpolymer", "natural polymeric material", "polymer", origin="natural",
  note="Naturally occurring polymeric materials used as plastics before/alongside synthetics.")

# ------------------------------------------------------------------- polyolefins
C("polyolefin", "polyolefin", "synthetic", thermal="thermoplastic", origin="synthetic",
  alt=["olefin"])
C("pe", "polyethylene", "polyolefin", thermal="thermoplastic", origin="synthetic",
  alt=["PE", "polythene", "Alkathene"])
C("hdpe", "high-density polyethylene", "pe", thermal="thermoplastic", origin="synthetic",
  alt=["HDPE", "high density polyethylene"])
C("ldpe", "low-density polyethylene", "pe", thermal="thermoplastic", origin="synthetic",
  alt=["LDPE", "low density polyethylene"])
C("lldpe", "linear low-density polyethylene", "pe", thermal="thermoplastic", origin="synthetic",
  alt=["LLDPE", "linear low density polyethylene"])
C("mdpe", "medium-density polyethylene", "pe", thermal="thermoplastic", origin="synthetic",
  alt=["MDPE", "medium density polyethylene"])
C("uhmwpe", "ultra-high-molecular-weight polyethylene", "pe", thermal="thermoplastic",
  origin="synthetic", alt=["UHMWPE", "Dyneema", "Zyex"])
C("epe", "expanded polyethylene", "pe", thermal="thermoplastic", origin="synthetic", alt=["EPE"])
C("pp", "polypropylene", "polyolefin", thermal="thermoplastic", origin="synthetic",
  alt=["PP", "high density polypropylene", "HDPP"])
C("epp", "expanded polypropylene", "pp", thermal="thermoplastic", origin="synthetic", alt=["EPP"])
C("poe", "polyolefin elastomer", "polyolefin", thermal="tpe", origin="synthetic", alt=["POE"])

# -------------------------------------------------------------------- styrenics
C("styrenic", "styrenic polymer", "synthetic", thermal="thermoplastic", origin="synthetic")
C("ps", "polystyrene", "styrenic", thermal="thermoplastic", origin="synthetic", alt=["PS"])
C("hips", "high-impact polystyrene", "ps", thermal="thermoplastic", origin="synthetic",
  alt=["HIPS", "high impact polystyrene", "Lustrex"])
C("eps", "expanded polystyrene", "ps", thermal="thermoplastic", origin="synthetic",
  alt=["EPS", "Styrofoam"])
C("xps", "extruded polystyrene", "ps", thermal="thermoplastic", origin="synthetic", alt=["XPS"])
C("abs", "acrylonitrile butadiene styrene", "styrenic", thermal="thermoplastic", origin="synthetic",
  alt=["ABS"])
C("san", "styrene acrylonitrile", "styrenic", thermal="thermoplastic", origin="synthetic", alt=["SAN"])
C("asa", "acrylonitrile styrene acrylate", "styrenic", thermal="thermoplastic", origin="synthetic",
  alt=["ASA", "Luran-S"])
C("smma", "styrene methyl methacrylate", "styrenic", thermal="thermoplastic", origin="synthetic",
  alt=["SMMA", "Zylar", "styrene acrylic copolymer"])
C("mbs", "methyl methacrylate butadiene styrene", "styrenic", thermal="thermoplastic",
  origin="synthetic", alt=["MBS", "methylmethacrylate butadiene styrene"])

# ----------------------------------------------------------------- vinyl polymers
C("vinyl", "vinyl polymer", "synthetic", thermal="thermoplastic", origin="synthetic", alt=["vinyl"])
C("pvc", "polyvinyl chloride", "vinyl", thermal="thermoplastic", origin="synthetic",
  alt=["PVC", "plasticised polyvinyl chloride", "unplasticised polyvinyl chloride",
       "uPVC", "linear low density polyvinyl", "LLDPV"])
C("pvdc", "polyvinylidene chloride", "vinyl", thermal="thermoplastic", origin="synthetic", alt=["PVDC"])
C("pvoh", "polyvinyl alcohol", "vinyl", thermal="thermoplastic", origin="synthetic", alt=["PVOH"])
C("pvb", "polyvinyl butyral", "vinyl", thermal="thermoplastic", origin="synthetic", alt=["PVB"])
C("pvca", "polyvinyl chloride acetate", "vinyl", thermal="thermoplastic", origin="synthetic", alt=["PVCA"])
C("eva", "ethylene vinyl acetate", "vinyl", thermal="thermoplastic", origin="synthetic", alt=["EVA"])
C("peva", "polyethylene vinyl acetate", "vinyl", thermal="thermoplastic", origin="synthetic", alt=["PEVA"])
C("evoh", "ethylene vinyl alcohol", "vinyl", thermal="thermoplastic", origin="synthetic",
  alt=["ethylene vinyl alcohol polymer", "EHOV", "EVOH"])

# ------------------------------------------------------------------- acrylics
C("acrylicfam", "acrylic polymer", "synthetic", thermal="thermoplastic", origin="synthetic",
  alt=["acrylic"])
C("pmma", "polymethyl methacrylate", "acrylicfam", thermal="thermoplastic", origin="synthetic",
  alt=["PMMA", "acrylic", "Perspex", "Plexiglass", "Plexiglas", "Lucite", "Diakon", "Oroglas"])
C("pan", "polyacrylonitrile", "acrylicfam", thermal="thermoplastic", origin="synthetic",
  alt=["PAN", "acrylic (fibres)", "Courtelle"])

# ----------------------------------------------------------------- polyamides
C("polyamide", "polyamide", "synthetic", thermal="thermoplastic", origin="synthetic",
  alt=["PA", "nylon", "Bri-nylon", "Enkalon", "Celon", "Zytel", "brushed nylon",
       "spinnaker nylon", "rip-stop nylon", "Loopamid"])
C("aramid", "aramid", "polyamide", thermal="thermoplastic", origin="synthetic",
  alt=["Kevlar", "Nomex", "Twaron", "polymetaphenylene isophthalamide",
       "poly p-phenylene terephthalamide", "poly-p-phenylene terephthalamide"])

# -------------------------------------------------------- thermoplastic polyesters
C("tp_polyester", "thermoplastic polyester", "synthetic", thermal="thermoplastic", origin="synthetic",
  alt=["polyester"])
C("pet", "polyethylene terephthalate", "tp_polyester", thermal="thermoplastic", origin="synthetic",
  alt=["PET", "Terylene", "Dacron", "Mylar", "Melinex", "Diolen", "Tetoron", "Trevira",
       "Crimplene", "Tersuisse", "Diolen"])
C("rpet", "recycled polyethylene terephthalate", "pet", thermal="thermoplastic", origin="synthetic",
  alt=["rPET", "Q-Nova"])
C("mpet", "metallised polyethylene terephthalate", "pet", thermal="thermoplastic", origin="synthetic",
  alt=["metalized polyethylene terephthalate", "metallised polyethylene terephthalate",
       "MPET", "MPT"])
C("petg", "polyethylene terephthalate glycol", "tp_polyester", thermal="thermoplastic",
  origin="synthetic", alt=["PETG", "polyester terephthalate glycol"])
C("pbt", "polybutylene terephthalate", "tp_polyester", thermal="thermoplastic", origin="synthetic",
  alt=["PBT"])
C("copolyester", "copolyester", "tp_polyester", thermal="thermoplastic", origin="synthetic",
  alt=["Tritan", "Provista", "plastic copolymer"])

# --------------------------------------------------------- other engineering TPs
C("pc", "polycarbonate", "synthetic", thermal="thermoplastic", origin="synthetic",
  alt=["PC", "Lexan", "Makrolon"])
C("pom", "polyoxymethylene", "synthetic", thermal="thermoplastic", origin="synthetic",
  alt=["POM", "acetal", "polyacetal", "Delrin", "Kematal", "Hostaform"])
C("pmp", "high-performance thermoplastic", "synthetic", thermal="thermoplastic", origin="synthetic")
C("peek", "polyetheretherketone", "pmp", thermal="thermoplastic", origin="synthetic", alt=["PEEK"])
C("psu", "polysulfone", "pmp", thermal="thermoplastic", origin="synthetic", alt=["PSU"])
C("pps", "polyphenylene sulphide", "pmp", thermal="thermoplastic", origin="synthetic", alt=["PPS"])
C("ppo", "polyphenylene oxide/ether", "pmp", thermal="thermoplastic", origin="synthetic",
  alt=["PPO", "PPE", "polyphenylene ether", "polyphenylene oxide", "Noryl", "Xylex"])
C("lcp", "liquid crystal polymer", "pmp", thermal="thermoplastic", origin="synthetic",
  alt=["Vectran"])

# ------------------------------------------------------------------ fluoropolymers
C("fluoropolymer", "fluoropolymer", "synthetic", thermal="thermoplastic", origin="synthetic")
C("ptfe", "polytetrafluoroethylene", "fluoropolymer", thermal="thermoplastic", origin="synthetic",
  alt=["PTFE", "polytetrafluorethylene", "Teflon"])
C("eptfe", "expanded polytetrafluoroethylene", "ptfe", thermal="thermoplastic", origin="synthetic",
  alt=["ePTFE", "Gore-Tex", "eVent"])

# ------------------------------------------------------------------- polyurethanes
C("pu", "polyurethane", "synthetic", origin="synthetic",
  alt=["PU", "PUR", "urethane", "cellular urethane", "Impranil"],
  note="Polyurethanes span thermoset (rigid/flexible foam) and thermoplastic (TPU) forms.")
C("purigid", "rigid polyurethane", "pu", thermal="thermoset", origin="synthetic",
  alt=["Baydur 110", "Baydur", "epoxy foam"])
C("pufoam", "polyurethane foam", "pu", thermal="thermoset", origin="synthetic",
  alt=["viscoelastic foam", "memory foam", "Poron", "plastazote", "MicroGel",
       "Iso-Zorb", "closed cell foam"])

# ------------------------------------------------------------- thermoset resins
C("thermoset", "thermosetting resin", "synthetic", thermal="thermoset", origin="synthetic")
C("phenolic", "phenol formaldehyde", "thermoset", thermal="thermoset", origin="synthetic",
  alt=["PF", "phenolic resin", "bakelite - generic term", "Bakelite", "Catalin", "Birmite",
       "Aramith", "Karophite Black"])
C("aminoplast", "aminoplast", "thermoset", thermal="thermoset", origin="synthetic")
C("uf", "urea formaldehyde", "aminoplast", thermal="thermoset", origin="synthetic",
  alt=["UF", "Beatl", "Beatle", "Beetle", "Bandalasta", "Scarab", "Jaxonite",
       "Beatl", "Roanoid"])
C("mf", "melamine formaldehyde", "aminoplast", thermal="thermoset", origin="synthetic",
  alt=["MF", "Melmex", "Melflex", "Warerite", "Formica", "Melamine", "Melaware"])
C("epoxy", "epoxy resin", "thermoset", thermal="thermoset", origin="synthetic",
  alt=["polyepoxide", "epoxide"])
C("ts_polyester", "unsaturated polyester resin", "thermoset", thermal="thermoset", origin="synthetic",
  alt=["polyester resin"])
C("boisdurci", "bois durci", "thermoset", thermal="thermoset", origin="semisynthetic",
  note="Composite of fine wood/other flour bound with albumen, moulded under heat and pressure.")

# ------------------------------------------------------- semi-synthetic (cellulosic)
C("cellulosic", "cellulosic plastic", "semisynthetic", origin="semisynthetic",
  alt=["cellulose"])
C("cn", "cellulose nitrate", "cellulosic", thermal="thermoplastic", origin="semisynthetic",
  alt=["CN", "celluloid - generic term", "celluloid", "Parkesine", "Xylonite", "Ivorine",
       "French Ivory", "Viscoloid", "Rhodoid"])
C("ca", "cellulose acetate", "cellulosic", thermal="thermoplastic", origin="semisynthetic",
  alt=["CA", "acetate", "Dicel", "Celanese", "Rhodoid", "Tenite"])
C("cta", "cellulose triacetate", "cellulosic", thermal="thermoplastic", origin="semisynthetic",
  alt=["CTA", "Tricel", "Tricelon"])
C("cap", "cellulose acetate propionate", "cellulosic", thermal="thermoplastic", origin="semisynthetic",
  alt=["CAP"])
C("rayon", "regenerated cellulose (rayon)", "cellulosic", origin="semisynthetic",
  alt=["rayon", "viscose", "viscose rayon", "acetate rayon", "Evlan", "Sarille",
       "Dicel", "Xsilite"])
C("lyocell", "lyocell", "rayon", origin="semisynthetic", alt=["Tencel", "SeaCell", "Kareline"])
C("modal", "modal", "rayon", origin="semisynthetic", alt=["Modal"])
C("cellophane", "regenerated cellulose film (cellophane)", "cellulosic", origin="semisynthetic",
  alt=["Cellophane"])
C("vulcfibre", "vulcanised fibre", "cellulosic", thermal="thermoset", origin="semisynthetic")

# --------------------------------------------------------- semi-synthetic (protein)
C("casein", "casein formaldehyde", "semisynthetic", thermal="thermoset", origin="semisynthetic",
  alt=["CF", "Erinoid", "Galalith", "Lactoid", "casein fibre"])

# ------------------------------------------------------------------- elastomers
C("elastomer", "elastomer", "polymer", thermal="elastomer",
  alt=["elastomer", "elastic"])
C("naturalrubber", "natural rubber", "elastomer", thermal="elastomer", origin="natural",
  alt=["rubber", "latex", "India rubber"])
C("guttapercha", "gutta percha", "elastomer", thermal="elastomer", origin="natural")
C("hardrubber", "hard rubber", "elastomer", thermal="thermoset", origin="natural",
  alt=["vulcanite", "ebonite"])
C("syntheticrubber", "synthetic rubber", "elastomer", thermal="elastomer", origin="synthetic",
  alt=["synthetic rubber"])
C("polychloroprene", "polychloroprene", "syntheticrubber", thermal="elastomer", origin="synthetic",
  alt=["Neoprene"])
C("nbr", "nitrile butadiene rubber", "syntheticrubber", thermal="elastomer", origin="synthetic",
  alt=["NBR"])
C("sbr", "styrene butadiene rubber", "syntheticrubber", thermal="elastomer", origin="synthetic",
  alt=["SBR", "styrene butadiene", "styrene butadiene copolymer"])
C("silicone", "silicone", "elastomer", thermal="elastomer", origin="synthetic",
  alt=["silicone", "fluorosilicone acrylate", "silicone acrylate"])
C("tpe", "thermoplastic elastomer", "elastomer", thermal="tpe", origin="synthetic",
  alt=["TPE", "thermoplastic elastomer", "thermoplastic rubber", "TPR", "thermoplastic technopolymer",
       "Thermolast TPE", "Monoprene", "Sofprene", "Mediprene", "Melflex", "Durapren", "Duraflex"])
C("tpu", "thermoplastic polyurethane", "tpe", thermal="tpe", origin="synthetic",
  alt=["TPU", "Pebax", "polyether block amides"])
C("tps", "thermoplastic styrenic elastomer", "tpe", thermal="tpe", origin="synthetic",
  alt=["TPS", "SEBS", "styrene ethylene butylene styrene", "SBS", "styrene butadiene styrene",
       "Thermolast"])
C("elastane", "elastane", "tpe", thermal="tpe", origin="synthetic",
  alt=["elastane", "spandex", "Spandex", "Lycra", "Lastex", "Roica Eco-Smart", "elastomultiester"])

# ------------------------------------------------------------------- biopolymers
C("pla", "polylactic acid", "biopolymer", thermal="thermoplastic", origin="biobased",
  alt=["PLA", "Ingeo", "Biobu", "Bio-Flex"])
C("cpla", "crystallised polylactic acid", "pla", thermal="thermoplastic", origin="biobased",
  alt=["CPLA", "crystallized ploylactic acid"])
C("pha", "polyhydroxyalkanoate", "biopolymer", thermal="thermoplastic", origin="biobased",
  alt=["PHA", "PHB"])
C("starchbio", "starch-based bioplastic", "biopolymer", origin="biobased",
  alt=["cornstarch", "potato starch", "cassava", "Plantic", "Plantic", "Gum-Tec", "bagasse",
       "sugarcane", "limestone compound", "Biograde 300A", "Ecodear", "CQuestBio", "EcoCore",
       "Biobu", "Minerale"])
C("biope", "bio-based polyethylene", "biopolymer", thermal="thermoplastic", origin="biobased",
  alt=["Bio-PE", "BLOOM", "ecothylene", "Ecodear"])
C("psu_bio", "next-generation bio-based leather alternative", "biopolymer", origin="biobased",
  alt=["Mylo", "mycelium", "Vegea", "grape", "Pinatex", "pineapple leaf fibre", "PALF",
       "AppleSkin", "Orange Fiber", "Lyka skin", "algae", "Solidwool", "woodlastic",
       "Pyratex Active 1"])
C("pdo", "polytrimethylene terephthalate / bio-PDO", "biopolymer", thermal="thermoplastic",
  origin="biobased", alt=["PDO", "propanediol", "Sorona"])

# ------------------------------------------------------- natural polymeric materials
C("shellac", "shellac", "naturalpolymer", thermal="thermoset", origin="natural",
  alt=["shellac", "composition"])
C("amber", "amber", "naturalpolymer", origin="natural")
C("horn", "horn", "naturalpolymer", origin="natural", alt=["oxhorn", "ram", "buffalo horn"])
C("tortoiseshell", "tortoiseshell", "naturalpolymer", origin="natural")
C("wax", "wax", "naturalpolymer", origin="natural")

# ---------------------------------------------------- generic / unspecified plastic
C("resin_generic", "resin (unspecified)", "polymer", alt=["resin", "thermoplastic resin"])
C("plastic_unidentified", "unidentified plastic", "polymer", alt=["unidentified"])
C("composite_generic", "fibre-reinforced plastic", "synthetic", thermal=None, origin="synthetic",
  alt=["FRP", "fibre-reinforced plastic", "GRP", "glass-reinforced plastic",
       "glass-reinforced polypropylene", "glass-reinforced nylon", "glass-reinforced polyester",
       "glass-reinforced polyethylene", "fibre glass", "fibreglass", "fiberglass",
       "carbon fibre composite", "carbon fibre", "carbon steel composite", "aramid"],
  note="Composite of a polymer matrix with reinforcing fibres.")

# ============================================================ non-plastic materials
C("metal", "metal", "nonplastic")
for cid, pref, al in [
    ("steel", "steel", ["carbon steel"]), ("stainlesssteel", "stainless steel", []),
    ("aluminium", "aluminium", ["anodised aluminium", "aluminium foil"]),
    ("brass", "brass", []), ("copper", "copper", []), ("silver", "silver", ["silver plate"]),
    ("tin", "tin", ["tin plate", "tinplate"]), ("chrome", "chrome", ["chromium"]),
    ("zinc", "zinc alloy", ["mazak", "mazak"]), ("nickel", "nickel", ["nickel silver", "EPNS",
     "electroplated nickel silver"]), ("titanium", "titanium", []),
    ("tungstencarbide", "tungsten carbide", []), ("lead", "lead", []), ("iron", "iron", []),
    ("graphite", "graphite", []), ("wire", "metal wire", ["wire", "foil"]),
]:
    C(cid, pref, "metal", alt=al, origin="natural")

C("glassfam", "glass", "nonplastic", alt=["glass", "Pyrex", "vitreous enamel", "enamel"], origin="natural")
C("ceramicfam", "ceramic", "nonplastic", alt=["ceramic", "porcelain", "Aramith"], origin="natural")
C("paperfam", "paper and board", "nonplastic",
  alt=["paper", "card", "board", "cardboard", "Tyvek", "Correx"], origin="natural")
C("woodfam", "wood and plant material", "nonplastic",
  alt=["wood", "bamboo", "cork", "rattan", "pine fibre", "M.D.F.", "MDF", "plant", "canvas"],
  origin="natural")
C("textilefam", "textile and natural fibre", "nonplastic",
  alt=["textile", "cotton", "wool", "silk", "linen", "velvet", "felt", "mohair", "taffeta",
       "chlorofibre", "bristle", "Lurex", "Coolmax", "Supplex", "X-Static", "Dri-lex",
       "Dri-Release", "Comtex", "Microfibra Techpro", "modal"], origin="natural")
C("leatherfam", "leather and skin", "nonplastic",
  alt=["leather", "suede", "patent leather", "synthetic leather", "Lorica", "skin",
       "crocodile skin", "cowhide", "buffalo", "mole skin", "fur", "rabbit", "hair", "feather",
       "Vegea"], origin="natural")
C("animalfam", "animal / organic material", "nonplastic",
  alt=["animal", "bone", "shell", "pearl", "ivory", "diamond", "mother of pearl", "cork"],
  origin="natural")
C("stonefam", "stone and mineral", "nonplastic",
  alt=["stone", "limestone", "plaster", "porcelain", "graphite", "diamond", "Aerogel",
       "Nanopreme"], origin="natural")
C("plasticine", "plasticine (modelling material)", "nonplastic")


# ------------------------------------------------------------------ raw -> concept
# Explicit overrides where a raw string is ambiguous or needs a specific target.
# Trade-name strings carry a " - trade name" / " - tradename" suffix in the data;
# those are resolved by stripping the suffix and matching an alt label (see resolver).
RAW_OVERRIDES = {
    "plastic": "polymer",
    "biopolymer": "biopolymer",
    "plastic (natural)": "biopolymer",
    "unidentified": "plastic_unidentified",
    "resin": "resin_generic",
    "thermoplastic resin": "resin_generic",
    "thermoplastic technopolymer": "tpe",
    "recycled": "polymer",            # 'recycled' alone is a process attribute, not a polymer
    "acrylic": "pmma",
    "acetate": "ca",
    "acetal": "pom",
    "polyacetal": "pom",
    "nylon": "polyamide",
    "polythene": "pe",
    "vinyl": "pvc",
    "olefin": "polyolefin",
    "polyester": "tp_polyester",
    "polyester resin": "ts_polyester",
    "rubber": "naturalrubber",
    "hard rubber": "hardrubber",
    "synthetic rubber": "syntheticrubber",
    "silicone": "silicone",
    "cellulose": "cellulosic",
    "casein fibre": "casein",
    "shellac": "shellac",
    "vulcanite": "hardrubber",
    "ebonite": "hardrubber",
    "gutta percha": "guttapercha",
    "elastic": "elastomer",
    "elastomer": "elastomer",
    "fibre glass": "composite_generic",
    "carbon fibre": "composite_generic",
    "carbon fibre composite": "composite_generic",
    "wire": "wire",
    "foil": "wire",
    "aluminium foil": "aluminium",
    "enamel": "glassfam",
    "vitreous enamel": "glassfam",
    "stove enamelled alloy": "metal",
    "composition": "shellac",
    "PP TV": "pp",
    "talcum-reinforced polypropylene": "pp",
    "M49": "pmma",
    "CR39": "padc",
    "woodlastic": "psu_bio",
    "Duraflex": "tpe",
    "PPK": "pmp",
    "Econyl - trade name": "polyamide",      # regenerated nylon
    "Crystalate - trade name": "phenolic",   # phenolic/shellac record material
    "Synchilla - trade name": "pet",         # recycled-PET fleece
    "Torayca - trade name": "composite_generic",  # carbon-fibre
    "plasticine": "plasticine",
    "ebony": "woodfam",
    "synthetic fur": "textilefam",
    "Mepal - trade name": "polymer",         # tableware brand, mixed polymers
    "butyl stearate": "polymer",             # plasticiser additive
}


def _norm(s):
    return s.strip().lower()


def build_alt_index():
    """Map every alt label (and pref) -> concept id, case-insensitively."""
    idx = {}
    for cid, c in CONCEPTS.items():
        idx.setdefault(_norm(c["pref"]), cid)
        for a in c["alt"]:
            idx.setdefault(_norm(a), cid)
    return idx


_ALT = build_alt_index()

_TRADE_SUFFIXES = (" - trade name", " - tradename", " -trade name", " - trade  name")
_GENERIC_SUFFIXES = (" - generic term",)


def resolve(raw):
    """Resolve a raw material string from the collection to a concept id, or None."""
    if raw in RAW_OVERRIDES:
        return RAW_OVERRIDES[raw]
    n = _norm(raw)
    if n in _ALT:
        return _ALT[n]
    # strip trade-name / generic-term suffixes then retry
    base = raw
    for suf in _TRADE_SUFFIXES + _GENERIC_SUFFIXES:
        if base.lower().endswith(suf):
            base = base[: -len(suf)]
            break
    bn = _norm(base)
    if bn in _ALT:
        return _ALT[bn]
    if base in RAW_OVERRIDES:
        return RAW_OVERRIDES[base]
    return None


# add a couple of concepts referenced above but not yet defined
C("padc", "polyallyl diglycol carbonate", "thermoset", thermal="thermoset", origin="synthetic",
  alt=["PADC", "CR39", "CR-39", "M49"])
_ALT = build_alt_index()  # rebuild to include late additions


if __name__ == "__main__":
    import sys, collections
    # self-test: report coverage against a vocab TSV passed as argv[1]
    path = sys.argv[1] if len(sys.argv) > 1 else None
    print(f"concepts defined: {len(CONCEPTS)}")
    if path:
        total = mapped = 0
        unmapped = []
        for line in open(path):
            n, term = line.rstrip("\n").split("\t")
            n = int(n)
            total += n
            if resolve(term):
                mapped += n
            else:
                unmapped.append((n, term))
        print(f"material assertions: {total}  mapped: {mapped} "
              f"({100*mapped/total:.1f}%)  unmapped terms: {len(unmapped)}")
        for n, t in sorted(unmapped, reverse=True)[:40]:
            print(f"  UNMAPPED {n:5d}  {t}")
