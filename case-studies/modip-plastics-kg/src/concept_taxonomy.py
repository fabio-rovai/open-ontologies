"""
MoDiP Domain-Concept Taxonomy — SKOS source of truth.

MoDiP records carry an 'associated concept' facet describing the use-domain of
each object. There are 22 such concepts, published as a flat list. This module
preserves each as an authoritative leaf concept and adds an explicit grouping
(the implicit structure MoDiP's facet already implies) so the domain facet
becomes navigable.
"""

DCONCEPTS = {}


def D(cid, pref, broader=None, note=None):
    DCONCEPTS[cid] = dict(pref=pref, broader=broader, note=note)


D("domain", "object use-domain")

# groups
D("domestic", "domestic life", "domain")
D("dress", "dress and personal", "domain")
D("workcomm", "work and communication", "domain")
D("leisure", "leisure and recreation", "domain")
D("industry", "materials, industry and environment", "domain")
D("collection_meta", "collection and reference material", "domain")

# leaves = the 22 published MoDiP associated concepts (prefLabel = verbatim term)
LEAVES = {
    "house and garden": "domestic",
    "health, care and grooming": "domestic",
    "smoking": "domestic",
    "animals and pets": "domestic",
    "fashion and costume": "dress",
    "textiles": "dress",
    "office and workplace": "workcomm",
    "telecommunications": "workcomm",
    "audio visual": "workcomm",
    "printed, written and drawn material": "workcomm",
    "photographic": "workcomm",
    "toys and games": "leisure",
    "sports, leisure and hobbies": "leisure",
    "travel and holiday": "leisure",
    "plastics samples and industry": "industry",
    "packaging and materials handling": "industry",
    "construction and building services": "industry",
    "plastics and the environment": "industry",
    "archival material": "collection_meta",
    "MoDiP reference library": "collection_meta",
    "promotional material": "collection_meta",
    "artist or designer work": "collection_meta",
}


def _slug(s):
    return "dc_" + "".join(ch if ch.isalnum() else "_" for ch in s.lower()).strip("_")[:40]


for term, grp in LEAVES.items():
    D(_slug(term), term, grp)

_BY_LABEL = {v["pref"].lower(): k for k, v in DCONCEPTS.items()}


def resolve(raw):
    return _BY_LABEL.get(raw.strip().lower())


if __name__ == "__main__":
    import sys
    print(f"domain concepts: {len(DCONCEPTS)}")
    if len(sys.argv) > 1:
        total = mapped = 0
        for line in open(sys.argv[1]):
            n, t = line.rstrip("\n").split("\t"); n = int(n)
            total += n
            if resolve(t):
                mapped += n
        print(f"concept assertions: {total}  mapped: {mapped} ({100*mapped/total:.1f}%)")
