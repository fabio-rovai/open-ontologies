"""
Reconcile the MoDiP materials/technique concepts against the Getty Art &
Architecture Thesaurus (AAT) via the public Getty SPARQL endpoint.

A match is accepted ONLY when an AAT concept's preferred or alternate label
matches one of our labels exactly (case-insensitive). No fuzzy matches are
kept, so every emitted skos:exactMatch is verifiable. Getty AAT is published
under the Open Data Commons Attribution License (ODC-By 1.0).

Output: data/getty_alignment.json  { concept_id: {"aat": uri, "matched_on": label} }
"""
import json, os, sys, time, urllib.parse, urllib.request

sys.path.insert(0, os.path.dirname(__file__))
from materials_taxonomy import CONCEPTS  # noqa: E402

ENDPOINT = "https://vocab.getty.edu/sparql.json"
ROOT = os.path.dirname(os.path.dirname(__file__))


def q(term):
    # Full-text find candidate AAT concepts, return all their english labels.
    esc = term.replace('"', '\\"')
    query = f'''
SELECT ?s ?lab WHERE {{
  ?s a skos:Concept ; luc:term "{esc}" ; skos:inScheme <http://vocab.getty.edu/aat/> .
  {{ ?s skos:prefLabel ?lab }} UNION {{ ?s skos:altLabel ?lab }}
  FILTER(lang(?lab)="en")
}} LIMIT 60'''
    url = ENDPOINT + "?" + urllib.parse.urlencode({"query": query})
    req = urllib.request.Request(url, headers={
        "Accept": "application/sparql-results+json",
        "User-Agent": "modip-kg/1.0 (fabio@thetesseractacademy.com)"})
    for attempt in range(3):
        try:
            with urllib.request.urlopen(req, timeout=40) as r:
                return json.load(r)["results"]["bindings"]
        except Exception as e:
            print(f"    retry {attempt}: {e}", flush=True)
            time.sleep(4)
    return []


def main():
    alignment = {}
    for cid, c in CONCEPTS.items():
        labels = {c["pref"].lower()} | {a.lower() for a in c["alt"]}
        # Query on the pref label (and a couple of alts if pref is an abbreviation-ish)
        seen_uri_labels = {}
        for term in [c["pref"]] + c["alt"][:2]:
            for b in q(term):
                uri = b["s"]["value"]
                lab = b["lab"]["value"]
                seen_uri_labels.setdefault(uri, set()).add(lab.lower())
            time.sleep(0.2)
        # accept the AAT uri whose label set intersects ours
        best = None
        for uri, labs in seen_uri_labels.items():
            inter = labs & labels
            if inter:
                best = (uri, sorted(inter)[0])
                break
        if best:
            alignment[cid] = {"aat": best[0], "matched_on": best[1]}
            print(f"  {cid:16s} -> {best[0].split('/')[-1]}  ({best[1]})", flush=True)
    out = os.path.join(ROOT, "data", "getty_alignment.json")
    json.dump(alignment, open(out, "w"), indent=2)
    print(f"\nmatched {len(alignment)}/{len(CONCEPTS)} concepts -> {out}")


if __name__ == "__main__":
    main()
