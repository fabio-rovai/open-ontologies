#!/usr/bin/env python3
"""Fetch ESCO occupation candidates for each Skills England occupational standard.

For every SEOM occupation name, query the public ESCO API (occupation type, en)
and cache the top candidates (title, uri, altLabels). No matching decisions are
made here; this is pure retrieval, cached to disk so the crosswalk build is
deterministic and re-runnable offline.

ESCO data is published by the European Commission under CC BY 4.0.
Skills England occupation names are used under the Open Government Licence v3.0.
"""
import json
import time
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SEOM = ROOT.parent / "skills-england-occupational-maps" / "data" / "occupations-list.json"
OUT = ROOT / "data" / "esco_candidates.jsonl"
API = "https://ec.europa.eu/esco/api/search"
UA = "skills-esco-crosswalk/0.1 (open research; fabio@thetesseractacademy.com)"


def query(text: str):
    params = urllib.parse.urlencode({
        "text": text, "type": "occupation", "language": "en", "limit": 5,
        "full": "false",
    })
    req = urllib.request.Request(API + "?" + params, headers={"User-Agent": UA})
    with urllib.request.urlopen(req, timeout=30) as r:
        d = json.load(r)
    out = []
    for res in d.get("_embedded", {}).get("results", []):
        out.append({
            "title": res.get("title"),
            "uri": res.get("uri"),
            "altLabels": res.get("alternativeLabel", {}).get("en", []) if isinstance(res.get("alternativeLabel"), dict) else [],
        })
    return out


def main():
    occs = json.loads(SEOM.read_text())
    done = set()
    if OUT.exists():
        done = {json.loads(l)["stdCode"] for l in OUT.open()}
    todo = [o for o in occs if o["stdCode"] not in done]
    print(f"{len(todo)} to fetch ({len(done)} cached)")
    out = OUT.open("a")
    for i, o in enumerate(todo):
        try:
            cands = query(o["name"])
        except Exception as e:
            cands = []
            print(f"{o['stdCode']} error {str(e)[:80]}")
        out.write(json.dumps({"stdCode": o["stdCode"], "name": o["name"], "level": o.get("level"), "candidates": cands}, ensure_ascii=False) + "\n")
        if i % 100 == 0:
            out.flush()
            print(f"{i}/{len(todo)}")
        time.sleep(0.15)
    out.close()
    print("done")


if __name__ == "__main__":
    main()
