#!/usr/bin/env python3
"""Build a SKOS crosswalk from Skills England occupational standards (SEOM) to
ESCO occupations, with conservative, labelled confidence.

For each SEOM occupation we compare its name against each cached ESCO candidate
title and its English alternative labels, and keep the best normalised
similarity. The match is then banded:

  exactMatch   sim >= 0.95   (label identity up to punctuation/case)
  closeMatch   sim >= 0.82
  relatedMatch sim >= 0.62
  (below 0.62) unmatched     - recorded as a gap, NOT forced to a wrong ESCO node

This is a lexical crosswalk, stated as such. It is a candidate-generation and
triage asset, not an authoritative equivalence: the unmatched set is the honest
output and is published alongside the matches.

SEOM names: Open Government Licence v3.0. ESCO: (c) European Union, CC BY 4.0.
"""
import json
import re
from collections import Counter
from difflib import SequenceMatcher
from pathlib import Path

from rdflib import Graph, Literal, Namespace, RDF, RDFS, URIRef
from rdflib.namespace import DCTERMS, SKOS, XSD

ROOT = Path(__file__).resolve().parent.parent
CAND = ROOT / "data" / "esco_candidates.jsonl"
CROSSWALK_JSON = ROOT / "data" / "crosswalk.json"
GRAPH_OUT = ROOT / "crosswalk.ttl"
METRICS_OUT = ROOT / "metrics.json"

SEOM = Namespace("https://tesseract.academy/id/seom/occupation/")
XW = Namespace("https://tesseract.academy/ns/seom-esco#")

BANDS = [(0.95, "exactMatch"), (0.82, "closeMatch"), (0.62, "relatedMatch")]


def norm(s: str) -> str:
    s = s.lower()
    s = re.sub(r"[^a-z0-9 ]", " ", s)
    return re.sub(r"\s+", " ", s).strip()


def sim(a: str, b: str) -> float:
    na, nb = norm(a), norm(b)
    seq = SequenceMatcher(None, na, nb).ratio()
    ta, tb = set(na.split()), set(nb.split())
    if ta and tb:
        inter = len(ta & tb)
        jac = inter / len(ta | tb)
        containment = inter / min(len(ta), len(tb))  # subset ("accountant" in "accountancy professional")
        token = max(jac, 0.9 * containment)
    else:
        token = 0.0
    return max(seq, token)


def best_match(name: str, candidates: list) -> tuple:
    best = (0.0, None, None)
    for c in candidates:
        labels = [c["title"]] + (c.get("altLabels") or [])
        s = max((sim(name, lab) for lab in labels if lab), default=0.0)
        if s > best[0]:
            best = (s, c["title"], c["uri"])
    return best


def band(score: float):
    for thr, name in BANDS:
        if score >= thr:
            return name
    return None


def main() -> None:
    # dedup cached candidates by stdCode (restarts appended duplicates)
    rows = {}
    for line in CAND.open():
        r = json.loads(line)
        rows[r["stdCode"]] = r
    rows = list(rows.values())

    results, band_counts = [], Counter()
    for r in rows:
        score, title, uri = best_match(r["name"], r["candidates"])
        b = band(score)
        band_counts[b or "unmatched"] += 1
        results.append({
            "stdCode": r["stdCode"], "seom_name": r["name"], "level": r.get("level"),
            "esco_title": title if b else None,
            "esco_uri": uri if b else None,
            "similarity": round(score, 3),
            "match": b or "unmatched",
        })

    g = Graph()
    g.bind("skos", SKOS)
    g.bind("dcterms", DCTERMS)
    g.bind("xw", XW)
    matched = 0
    for r in results:
        if not r["esco_uri"]:
            continue
        matched += 1
        s = URIRef(SEOM + r["stdCode"])
        o = URIRef(r["esco_uri"])
        pred = getattr(SKOS, r["match"])
        g.add((s, RDF.type, SKOS.Concept))
        g.add((s, SKOS.prefLabel, Literal(r["seom_name"], lang="en")))
        g.add((s, pred, o))
        g.add((s, XW.similarity, Literal(r["similarity"], datatype=XSD.decimal)))
        g.add((s, XW.matchMethod, Literal("lexical-difflib-v1")))

    CROSSWALK_JSON.write_text(json.dumps({
        "meta": {
            "seom_occupations": len(rows),
            "esco_source": "https://ec.europa.eu/esco/",
            "method": "lexical (difflib SequenceMatcher on normalised labels + ESCO altLabels)",
            "bands": {"exactMatch": ">=0.95", "closeMatch": ">=0.82", "relatedMatch": ">=0.62"},
            "licence_seom": "OGL v3.0", "licence_esco": "CC BY 4.0",
        },
        "band_counts": dict(band_counts),
        "crosswalk": results,
    }, indent=2))
    g.serialize(GRAPH_OUT, format="turtle")

    metrics = {
        "seom_occupations": len(rows),
        "matched": matched,
        "match_rate": round(matched / len(rows), 4),
        "band_counts": dict(band_counts),
    }
    METRICS_OUT.write_text(json.dumps(metrics, indent=2))
    print(f"{len(rows)} SEOM occupations")
    for b in ["exactMatch", "closeMatch", "relatedMatch", "unmatched"]:
        print(f"  {b}: {band_counts.get(b, 0)}")
    print(f"match rate (any band): {metrics['match_rate']*100:.1f}%")


if __name__ == "__main__":
    main()
