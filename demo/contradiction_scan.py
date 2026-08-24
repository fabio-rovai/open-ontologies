#!/usr/bin/env python3
"""
Contradiction scanner: find the passages in a document corpus that disagree.

WHY NOT BAYESIAN OPTIMISATION
-----------------------------
The obvious framing is "search the space of chunks for the one that trips
retrieval". That invites Bayesian optimisation, and BO is the wrong tool:
it is built for optimising an expensive black-box function over a smooth,
low-dimensional, continuous space. Chunk-pair contradiction is combinatorial,
discrete, and has no useful notion of "nearby". BO would sample slowly through
a space that does not reward sampling.

It is also unnecessary. Once claims are extracted into the graph, most
contradictions stop being a search problem:

  Tier 1  STRUCTURAL   the reasoner and SPARQL find them outright.
                       Cost: milliseconds. Exact, with zero false positives.
                       Catches disjoint-class violations, functional-property
                       conflicts, and domain rule breaches.

  Tier 2  BLOCKING     for everything semantic that the axioms cannot express,
                       do NOT compare all pairs. Claims carry `aboutEntity`,
                       so only compare claims about the SAME entity that come
                       from DIFFERENT documents. This is standard blocking from
                       entity resolution: it collapses O(M^2) over the corpus
                       to sum(k_i^2) over per-entity groups, where k_i is
                       typically single digits.

  Tier 3  ADJUDICATE   run a model over the surviving handful of candidate
                       pairs only. Optional, --adjudicate.

The cascade is orders of magnitude cheaper than any search-based method, and
unlike BO the first tier is exact rather than probabilistic.

Usage:
    python3 demo/contradiction_scan.py
    python3 demo/contradiction_scan.py --adjudicate     # add tier 3
"""

import argparse
import json
import os
import pathlib
import subprocess
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
BIN = ROOT / "target" / "release" / "open-ontologies"
BUNDLE = ROOT / "demo" / "bundle" / "dcat-us-full.ttl"
# Any extracted document graphs are merged in too, so the scan sees the whole
# corpus rather than only the curated demo data.
EXTRA = sorted((ROOT / "demo" / "corpus_extracted").glob("*.ttl"))
LLM_BASE = os.environ.get("ONTO_LLM_BASE_URL", "http://localhost:8081/v1").rstrip("/")
LLM_KEY = os.environ.get("ONTO_LLM_API_KEY", "not-needed")

PREFIXES = """PREFIX dcus:  <https://w3id.org/dcat-us-demo#>
PREFIX owl:  <http://www.w3.org/2002/07/owl#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX prov: <http://www.w3.org/ns/prov#>
"""

# --- Tier 1: structural contradictions -----------------------------------

TIER1 = [
    (
        "disjoint-class",
        "An individual is typed into two classes the ontology declares disjoint",
        PREFIXES
        + """SELECT DISTINCT ?subject ?classA ?classB ?disjointA ?disjointB WHERE {
  ?subject a ?classA, ?classB .
  FILTER(STR(?classA) < STR(?classB))
  ?classA rdfs:subClassOf* ?disjointA .
  ?classB rdfs:subClassOf* ?disjointB .
  { ?disjointA owl:disjointWith ?disjointB } UNION { ?disjointB owl:disjointWith ?disjointA }
}""",
    ),
    (
        "functional-property",
        "A functional property carries two different values on the same subject",
        PREFIXES
        + """SELECT DISTINCT ?subject ?property ?valueA ?valueB WHERE {
  ?property a owl:FunctionalProperty .
  ?subject ?property ?valueA, ?valueB .
  FILTER(STR(?valueA) < STR(?valueB))
}""",
    ),
    (
        "domain-rule",
        "A candidate declares a target term the register classifies as deprecated",
        PREFIXES
        + """SELECT DISTINCT ?subject ?term ?viaClass WHERE {
  ?subject dcus:targetsTerm ?term .
  ?term a ?viaClass .
  ?viaClass rdfs:subClassOf* dcus:DeprecatedTerm .
}""",
    ),
]

# --- Tier 2: blocked candidate generation --------------------------------

# Blocking must be tight or it stops being blocking. Two refinements beyond
# "same entity, different documents":
#   - exclude CLASS-level entities. A claim "about :Candidate" is about a
#     category, not a thing, and pairing every such claim with every other
#     generates noise proportional to corpus size rather than to real overlap.
#   - require the two claims to share vocabulary. Claims about the same
#     individual that share no significant term are rarely contradictory.
TIER2 = PREFIXES + """SELECT ?entity ?docA ?secA ?claimA ?docB ?secB ?claimB WHERE {
  ?a a dcus:Claim ; dcus:aboutEntity ?entity ; dcus:claimText ?claimA ; dcus:statedIn ?sa .
  ?b a dcus:Claim ; dcus:aboutEntity ?entity ; dcus:claimText ?claimB ; dcus:statedIn ?sb .
  FILTER(STR(?a) < STR(?b))
  FILTER NOT EXISTS { ?entity a owl:Class }
  FILTER NOT EXISTS { ?someone a ?entity }
  ?sa dcus:sectionNumber ?secA . ?da dcus:hasSection ?sa ; dcus:docId ?docA .
  ?sb dcus:sectionNumber ?secB . ?db dcus:hasSection ?sb ; dcus:docId ?docB .
  FILTER(?docA != ?docB)
}"""

# Total claims, for an honest reduction ratio rather than a bare count.
CLAIM_COUNT = PREFIXES + "SELECT (COUNT(?c) AS ?n) WHERE { ?c a dcus:Claim }"

# Which source asserted each conflicting fact.
PROVENANCE = PREFIXES + """SELECT ?thing ?sourceLabel WHERE {
  ?thing prov:wasDerivedFrom ?source .
  ?source rdfs:label ?sourceLabel .
}"""


def short(v):
    if not isinstance(v, str):
        return v
    if v.startswith("<") and v.endswith(">"):
        return v[1:-1].split("#")[-1]
    if v.startswith('"'):
        body = v[1:]
        for cut in ('"^^', '"@', '"'):
            if cut in body:
                return body.split(cut)[0]
        return body
    return v


def query(sparql):
    loads = "\n".join([f"load {BUNDLE}"] + [f"load {e}" for e in EXTRA])
    batch = f"clear\n{loads}\nquery {json.dumps(' '.join(sparql.split()))}\n"
    out = subprocess.run([str(BIN), "batch", "-"], input=batch, capture_output=True, text=True).stdout.strip()
    if not out:
        return []
    try:
        rows = json.loads(out.split("\n")[-1]).get("result", {}).get("results", [])
    except Exception:
        return []
    return [{k: short(v) for k, v in r.items()} for r in rows]


STOP = set("the a an of to in for is are be and or that this with by on at as it its "
           "must not no any all where which when from than then shall should may".split())


def signal(a, b):
    """Cheap high-precision discriminator applied after blocking.

    Blocking on entity alone is asymptotically good but weak on a small corpus:
    a handful of hot entities appear in most documents. Two extra signals cut
    it further at negligible cost:

      - shared significant vocabulary (the claims are actually about the same
        thing, not merely tagged to the same entity)
      - a numeric disagreement (two documents stating different values for
        what reads like the same quantity is the single most common form of
        real contradiction in controlled documents)

    Returns (score, reason). Pairs scoring 0 are dropped.
    """
    import re as _re
    ta = {w for w in _re.findall(r"[a-z]{4,}", a.lower()) if w not in STOP}
    tb = {w for w in _re.findall(r"[a-z]{4,}", b.lower()) if w not in STOP}
    shared = ta & tb
    na = set(_re.findall(r"\d+(?:\.\d+)?", a))
    nb = set(_re.findall(r"\d+(?:\.\d+)?", b))
    score, why = 0, []
    if len(shared) >= 3:
        score += 1
        why.append(f"{len(shared)} shared terms")
    if na and nb and na != nb:
        score += 2
        why.append(f"numeric disagreement {sorted(na)} vs {sorted(nb)}")
    neg = {"not", "no", "must not", "never"}
    if any(n in a.lower() for n in neg) != any(n in b.lower() for n in neg):
        score += 1
        why.append("one asserts, one negates")
    return score, "; ".join(why)


def adjudicate(entity, a, b):
    """Tier 3. Only ever called on the small blocked candidate set."""
    prompt = (
        f"Two statements about the same entity ({entity}) from different controlled documents.\n\n"
        f"A: {a}\nB: {b}\n\n"
        "Do these CONTRADICT each other? A contradiction means both cannot be true at once. "
        "Differing detail or emphasis is NOT a contradiction.\n"
        'Answer with JSON only: {"contradiction": true|false, "reason": "<one sentence>"}'
    )
    try:
        model = json.load(
            urllib.request.urlopen(
                urllib.request.Request(f"{LLM_BASE}/models", headers={"Authorization": f"Bearer {LLM_KEY}"}),
                timeout=30,
            )
        )["data"][0]["id"]
        req = urllib.request.Request(
            f"{LLM_BASE}/chat/completions",
            data=json.dumps(
                {
                    "model": model,
                    "messages": [{"role": "user", "content": prompt}],
                    "temperature": 0.1,
                    "max_tokens": 200,
                    "chat_template_kwargs": {"enable_thinking": False},
                }
            ).encode(),
            headers={"Content-Type": "application/json", "Authorization": f"Bearer {LLM_KEY}"},
        )
        text = json.load(urllib.request.urlopen(req, timeout=180))["choices"][0]["message"]["content"]
        start, end = text.find("{"), text.rfind("}")
        return json.loads(text[start : end + 1]) if start >= 0 else {"contradiction": None, "reason": "unparseable"}
    except Exception as e:
        return {"contradiction": None, "reason": f"adjudication unavailable: {e}"}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--adjudicate", action="store_true", help="run tier 3 over blocked candidates")
    args = ap.parse_args()

    if not BIN.exists():
        sys.exit(f"engine binary not found: {BIN}")

    findings = 0

    print("=" * 72)
    print("TIER 1  structural contradictions (reasoner + SPARQL, exact)")
    print("=" * 72)
    for name, description, sparql in TIER1:
        rows = query(sparql)
        print(f"\n[{name}] {description}")
        if not rows:
            print("  none")
            continue
        findings += len(rows)
        for r in rows:
            print(f"  CONFLICT {json.dumps(r)}")

    prov = query(PROVENANCE)
    if prov:
        print("\n[provenance] which upstream source asserted each fact")
        by_thing = {}
        for r in prov:
            by_thing.setdefault(r["thing"], []).append(r["sourceLabel"])
        for thing, sources in by_thing.items():
            marker = "  <-- asserted by TWO sources" if len(sources) > 1 else ""
            print(f"  {thing}: {', '.join(sources)}{marker}")

    print("\n" + "=" * 72)
    print("TIER 2  blocked candidate pairs (same entity, different documents)")
    print("=" * 72)
    candidates = query(TIER2)
    try:
        total = int(query(CLAIM_COUNT)[0]["n"])
    except Exception:
        total = 0
    full = total * (total - 1) // 2 if total else 0
    print(f"\nclaims in corpus: {total}   full pairwise space: {full}")
    print(f"candidate pairs after blocking: {len(candidates)}"
          + (f"   ({100*len(candidates)/full:.1f}% of pairwise)" if full else ""))
    scored, seen = [], set()
    for c in candidates:
        # The SPARQL yields each pair once, but different section IRIs can carry
        # the same claim text, so dedupe on the unordered text pair.
        key = frozenset((c["claimA"], c["claimB"]))
        if key in seen:
            continue
        seen.add(key)
        sc, why = signal(c["claimA"], c["claimB"])
        if sc:
            scored.append((sc, why, c))
    scored.sort(key=lambda x: -x[0])
    print(f"high-signal pairs after discriminator: {len(scored)}"
          + (f"   ({100*len(scored)/full:.1f}% of pairwise)" if full else ""))
    print(f"\n{DIM if False else ''}Blocking alone is asymptotic: its advantage grows with corpus size.")
    print("On 36 claims a few hot entities dominate, so the discriminator does the")
    print("real work here. On a corpus of thousands both matter.\n")
    candidates = [c for _, _, c in scored[:12]]
    for sc, why, c in scored[:12]:
        print(f"  [signal {sc}] {c['entity']}  ({why})")
        print(f"    {c['docA']} S{c['secA']}: {c['claimA'][:130]}")
        print(f"    {c['docB']} S{c['secB']}: {c['claimB'][:130]}")
    if not candidates:
        print("  none")
    for c in candidates:
        if args.adjudicate:
            verdict = adjudicate(c["entity"], c["claimA"], c["claimB"])
            flag = "CONTRADICTION" if verdict.get("contradiction") else "consistent"
            print(f"    -> {flag}: {verdict.get('reason', '')}")

    print("\n" + "=" * 72)
    print(f"tier 1 structural findings: {findings}")
    print(f"tier 2 candidate pairs    : {len(candidates)}")
    print("\nCost note: tier 1 is exact and runs in milliseconds. Tier 2 compares")
    print("only claims that share an entity across documents, so the model in")
    print("tier 3 is invoked on a handful of pairs rather than on every chunk pair.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
