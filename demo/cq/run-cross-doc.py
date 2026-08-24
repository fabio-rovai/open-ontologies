#!/usr/bin/env python3
"""Run the cross-document competency questions against the full demo bundle.

These are the questions a vector retriever structurally cannot answer,
because no single passage contains the chain. Each one spans two to four
separate documents plus the structured graph.
"""

import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
BIN = ROOT / "target" / "release" / "open-ontologies"
BUNDLE = ROOT / "demo" / "bundle" / "dcat-us-full.ttl"

P = """PREFIX crd: <https://w3id.org/dcat-us-demo#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
"""

QUESTIONS = [
    (
        "XD-1",
        "Which controls should block SYN-4471, are they automated, and who declares them?",
        P
        + """SELECT ?docId ?control ?blocking ?automated WHERE {
  ?ctrl crd:blocksProgressionOf crd:SYN_4471 ;
        rdfs:label ?control ; crd:isBlocking ?blocking ; crd:isAutomated ?automated .
  ?doc crd:declaresControl ?ctrl ; crd:docId ?docId . }""",
    ),
    (
        "XD-2",
        "Root cause: every claim about the mis-mapped bucket or the hoverfly, with its source",
        P
        + """SELECT ?docId ?section ?claim WHERE {
  ?c a crd:Claim ; crd:claimText ?claim ; crd:statedIn ?sec ; crd:aboutEntity ?e .
  VALUES ?e { crd:AphidAssociatedArthropod crd:EpisyrphusBalteatus crd:SRC_PestScoutingFeed }
  ?sec crd:sectionNumber ?section .
  ?doc crd:hasSection ?sec ; crd:docId ?docId . }""",
    ),
    (
        "XD-3",
        "Which safeguards exist only on paper (blocking or not, but not automated)?",
        P
        + """SELECT ?docId ?control ?blocking WHERE {
  ?ctrl a crd:Control ; rdfs:label ?control ;
        crd:isAutomated false ; crd:isBlocking ?blocking .
  ?doc crd:declaresControl ?ctrl ; crd:docId ?docId . }""",
    ),
    (
        "XD-4",
        "Which reference source is superseded, and which section documents its defect?",
        P
        + """SELECT ?supersedingSource ?supersededSource ?documentingDoc ?section ?sectionLabel WHERE {
  ?winner crd:supersedes ?loser .
  BIND(REPLACE(STR(?winner), "^.*#", "") AS ?supersedingSource)
  BIND(REPLACE(STR(?loser),  "^.*#", "") AS ?supersededSource)
  ?sec crd:documentsRiskIn ?loser ; crd:sectionNumber ?section ; rdfs:label ?sectionLabel .
  ?doc crd:hasSection ?sec ; crd:docId ?documentingDoc . }""",
    ),
    (
        "XD-5",
        "Cross-document section links: which section of one document bears on another?",
        P
        + """SELECT ?fromDoc ?fromSec ?toDoc ?toSec WHERE {
  ?a crd:relatedSection ?b .
  ?da crd:hasSection ?a ; crd:docId ?fromDoc . ?a crd:sectionNumber ?fromSec .
  ?db crd:hasSection ?b ; crd:docId ?toDoc . ?b crd:sectionNumber ?toSec .
  FILTER(?fromDoc != ?toDoc) }""",
    ),
    (
        "XD-6",
        "Access control: which ACL groups would a reader need to see this whole chain?",
        P
        + """SELECT DISTINCT ?docId ?classification ?acl WHERE {
  ?doc crd:docId ?docId ; crd:classification ?classification ; crd:aclGroup ?acl . }""",
    ),
]


def clean(v):
    if not isinstance(v, str):
        return v
    if v.startswith("<") and v.endswith(">"):
        return v[1:-1].split("#")[-1]
    if v.startswith('"'):
        body = v[1:]
        for cut in ('"^^', '"@', '"'):
            if cut in body:
                body = body.split(cut)[0]
                break
        return body
    return v


def main():
    if not BIN.exists():
        sys.exit(f"engine binary not found: {BIN}. Run cargo build --release.")
    failures = 0
    for qid, title, sparql in QUESTIONS:
        batch = f"clear\nload {BUNDLE}\nquery {json.dumps(' '.join(sparql.split()))}\n"
        out = subprocess.run(
            [str(BIN), "batch", "-"], input=batch, capture_output=True, text=True
        ).stdout.strip()
        last = out.split("\n")[-1] if out else "{}"
        try:
            res = json.loads(last).get("result", {})
            rows = res.get("results", [])
        except Exception:
            print(f"\n{qid}  {title}\n  PARSE FAILURE: {last[:200]}")
            failures += 1
            continue

        print(f"\n{qid}  {title}")
        print(f"  rows: {len(rows)}")
        if not rows:
            failures += 1
            print("  NO RESULTS")
        for row in rows:
            print("   ", {k: clean(v) for k, v in row.items()})

    print(f"\n{'-' * 60}")
    print(f"{len(QUESTIONS) - failures}/{len(QUESTIONS)} cross-document questions answered")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
