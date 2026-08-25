#!/usr/bin/env python3
"""
Package the pipeline's derived output into the artifacts Task 8 owns:
corpus.json, graph.json, findings.json, chat.json. (compare.json is built
separately by build_compare.py because it needs a human read of the raw
grounded/baseline generations before the divergence field can be written
honestly.)

Shapes match studio/src/lib/demo-source.ts's DemoSource / ReplayFixtures
types exactly (committed in cb4a62e, after this script's first draft was
written against a guessed shape):

    corpus.json   -> Document[]            { id, title, text }
    graph.json    -> GraphView             { classes, properties, edges }
    findings.json -> Contradiction[]       { id, subject, kind, claims }
                     claims: Claim[]       { document, predicate, object }
    chat.json     -> Record<question, Chunk[]>   Chunk { type, value }

Consumes demo/corpus/dcat-us/ (for provenance) and demo/derived/ (written by
demo/ontology_from_docs.py, which must run first).

Usage:
    python3 demo/build_precomputed.py --out demo/precomputed
"""
import argparse
import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
BIN = ROOT / "target" / "release" / "open-ontologies"
CORPUS = ROOT / "demo" / "corpus" / "dcat-us"
DERIVED = ROOT / "demo" / "derived"
STORE = DERIVED / "_store.ttl"

P = ("PREFIX : <https://w3id.org/dcat-us-demo#>\n"
     "PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>\n"
     "PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>\n"
     "PREFIX owl: <http://www.w3.org/2002/07/owl#>\n"
     "PREFIX prov: <http://www.w3.org/ns/prov#>\n")


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


def sparql(query_str: str) -> list[dict]:
    """Query the STORE fresh every time: a clean clear+load per call, never
    whatever happens to be sitting in a long-lived engine process, so the
    same store on disk always yields the same rows."""
    lines = ["clear", f"load {STORE}", f"query {json.dumps(' '.join(query_str.split()))}"]
    out = subprocess.run([str(BIN), "batch", "-"], input="\n".join(lines) + "\n",
                         capture_output=True, text=True).stdout.strip()
    if not out:
        return []
    try:
        rows = json.loads(out.split("\n")[-1]).get("result", {}).get("results", [])
    except Exception:
        return []
    return [{k: short(v) for k, v in r.items()} for r in rows]


# --------------------------------------------------------------------------
# corpus.json -> Document[] { id, title, text }
# --------------------------------------------------------------------------

def guess_title(text: str, fname: str) -> str:
    m = re.search(r"^#\s+(.+)$", text, re.M)
    if m:
        return m.group(1).strip()
    m = re.search(r'"title"\s*:\s*"([^"]+)"', text)
    if m:
        return m.group(1)
    return fname


def build_corpus() -> list[dict]:
    manifest = json.loads((CORPUS / "MANIFEST.json").read_text())
    docs = []
    for entry in manifest:
        # MANIFEST.json also records the vendored jsonschema/ tree that
        # demo/dcat_conformance.py reads (role: "validator-input", a
        # directory, not a document). Task 7's seven markdown/JSON/Turtle
        # documents are the only entries this corpus is built from.
        if entry.get("role") == "validator-input":
            continue
        fname = entry["file"]
        path = CORPUS / fname
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8", errors="ignore")
        docs.append({
            "id": pathlib.Path(fname).stem,
            "title": guess_title(text, fname),
            "text": text,
            # Extra provenance beyond the Document{id,title,text} type. JSON
            # consumers that only read id/title/text ignore these; the ones
            # that want "everything cites its source" get it without a
            # second file to cross-reference.
            "file": fname,
            "source_url": entry["source_url"],
            "sha256": entry["sha256"],
            "retrieved": entry["retrieved"],
        })
    docs.sort(key=lambda d: d["id"])
    return docs


# --------------------------------------------------------------------------
# graph.json -> GraphView { classes, properties, edges }
# --------------------------------------------------------------------------

def _dedupe_by_iri(rows: list[dict], iri_key: str) -> list[dict]:
    by_iri: dict[str, set] = {}
    for r in rows:
        by_iri.setdefault(r[iri_key], set()).add(r.get("label") or "")
    out = []
    for iri, labels in by_iri.items():
        labels.discard("")
        entry = {"iri": iri}
        if labels:
            entry["label"] = sorted(labels)[0]
        out.append(entry)
    return sorted(out, key=lambda x: x["iri"])


def build_graph() -> dict:
    # A class or property can carry more than one rdfs:label -- independent
    # per-document derivation fragments each labelled it before the merge --
    # so this is grouped by IRI and only the alphabetically-first label kept,
    # rather than emitting one row per (iri, label) pair, which produced
    # duplicate IRIs in the array.
    classes_q = sparql(P + """SELECT ?c ?label WHERE {
      ?c a owl:Class . OPTIONAL { ?c rdfs:label ?label }
    }""")
    classes = _dedupe_by_iri(classes_q, "c")

    props_q = sparql(P + """SELECT ?p ?label WHERE {
      { ?p a owl:ObjectProperty } UNION { ?p a owl:DatatypeProperty }
      OPTIONAL { ?p rdfs:label ?label }
    }""")
    properties = _dedupe_by_iri(props_q, "p")

    # Edges: every real (non-literal-object) fact in the store, subject and
    # object both IRIs. This is the actual instance graph -- what the graph
    # view has to draw -- not merely the vocabulary's class/property list.
    edge_q = sparql(P + """SELECT DISTINCT ?s ?o WHERE {
      ?s ?p ?o . FILTER(isIRI(?o))
      FILTER(?p != rdf:type)
    }""")
    edges = sorted({(r["s"], r["o"]) for r in edge_q if r.get("s") and r.get("o")})
    edges = [{"source": s, "target": o} for s, o in edges]

    return {"classes": classes, "properties": properties, "edges": edges}


# --------------------------------------------------------------------------
# findings.json -> Contradiction[] { id, subject, kind, claims }
# --------------------------------------------------------------------------

def sources_of() -> dict[tuple[str, str], set[str]]:
    """Which document(s) asserted a given individual has a given type,
    read from the claim nodes Stage 7 of ontology_from_docs.py writes
    (:CLAIM_N :claimText "DOC types SUBJ as CLS"), keyed off the same
    provenance the live GraphRAG chat cites from."""
    rows = sparql(P + """SELECT ?text WHERE { ?c a :Claim ; :claimText ?text }""")
    out: dict[tuple[str, str], set[str]] = {}
    for r in rows:
        m = re.match(r"^(\S+)\s+types\s+(\S+)\s+as\s+(\S+)$", r["text"].strip())
        if not m:
            continue
        doc, subj, cls = m.groups()
        out.setdefault((subj, cls), set()).add(doc)
    return out


def build_findings() -> list[dict]:
    """Structural, disjointness-based contradictions: the SAME subject IRI
    typed into two classes the ontology declares mutually exclusive, by two
    DIFFERENT documents.

    This is the honest mechanism, not a padded one: it is exactly the query
    ontology_from_docs.py's own Stage 6 and verify.py's Check 6 already run
    against this exact store. Whatever it returns -- including nothing -- is
    what genuinely got extracted and reasoned over, not what would look good
    in a demo.
    """
    rows = sparql(P + """SELECT DISTINCT ?subject ?a ?b WHERE {
      ?subject a ?a, ?b . FILTER(STR(?a) < STR(?b))
      ?a rdfs:subClassOf* ?da . ?b rdfs:subClassOf* ?db .
      { ?da owl:disjointWith ?db } UNION { ?db owl:disjointWith ?da }
    }""")
    src = sources_of()
    findings = []
    for r in rows:
        subj, a, b = r["subject"], r["a"], r["b"]
        da, db = src.get((subj, a), set()), src.get((subj, b), set())
        # Only a genuine cross-document disagreement, not one document
        # over-typing the same individual twice.
        if not ((da - db) and (db - da)):
            continue
        claims = sorted(
            [{"document": d, "predicate": "type", "object": a} for d in da]
            + [{"document": d, "predicate": "type", "object": b} for d in db],
            key=lambda c: (c["document"], c["object"]))
        findings.append({
            "id": f"disjointness:{subj}:{a}:{b}",
            "subject": subj,
            "kind": "disjointness",
            "claims": claims,
        })
    findings.sort(key=lambda f: f["id"])
    return findings


# --------------------------------------------------------------------------
# chat.json -> Record<question, Chunk[]>, derived mechanically from
# compare.json's grounded half.
#
# No second round of model calls: compare.json already ran the same GraphRAG
# grounded generation build_compare.py implements, over the same store, for
# the same questions. Chat replay is that same generation reshaped as chat
# turns, so this stays a pure function of an artifact already on disk rather
# than a second, potentially-divergent code path to the model.
#
# Keyed by the LOWERCASED trimmed question, because
# studio/src/lib/replay-source.ts's ask() looks up
# `fixtures.chat[question.trim().toLowerCase()]` first.
# --------------------------------------------------------------------------

def build_chat(compare: dict) -> dict:
    chat = {}
    for question in sorted(compare.keys()):
        g = compare[question]["grounded"]
        key = question.strip().lower()
        chat[key] = [
            {"type": "tool_call",
             "value": f'graphrag_retrieve({{"question": {json.dumps(question)}}})'},
            {"type": "text", "value": g["answer"]},
        ]
    return chat


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True, type=pathlib.Path)
    args = ap.parse_args()

    if not STORE.exists():
        sys.exit(f"missing {STORE}; run demo/ontology_from_docs.py --corpus demo/corpus/dcat-us first")
    if not BIN.exists():
        sys.exit(f"engine binary not found: {BIN}")

    args.out.mkdir(parents=True, exist_ok=True)

    corpus = build_corpus()
    (args.out / "corpus.json").write_text(
        json.dumps(corpus, indent=2, sort_keys=True, ensure_ascii=False) + "\n")
    print(f"corpus.json: {len(corpus)} documents")

    graph = build_graph()
    (args.out / "graph.json").write_text(
        json.dumps(graph, indent=2, sort_keys=True, ensure_ascii=False) + "\n")
    print(f"graph.json: {len(graph['classes'])} classes, {len(graph['properties'])} properties, "
          f"{len(graph['edges'])} edges")

    findings = build_findings()
    (args.out / "findings.json").write_text(
        json.dumps(findings, indent=2, sort_keys=True, ensure_ascii=False) + "\n")
    multi = sum(1 for f in findings if len({c["document"] for c in f["claims"]}) >= 2)
    print(f"findings.json: {len(findings)} findings, {multi} cite >=2 distinct documents")

    compare_path = args.out / "compare.json"
    if compare_path.exists():
        compare = json.loads(compare_path.read_text())
        chat = build_chat(compare)
        (args.out / "chat.json").write_text(
            json.dumps(chat, indent=2, sort_keys=True, ensure_ascii=False) + "\n")
        print(f"chat.json: {len(chat)} scripted turns (derived from compare.json's grounded half)")
    else:
        print(f"chat.json: SKIPPED, {compare_path} does not exist yet "
              f"(run demo/build_compare.py and author compare.json first)")


if __name__ == "__main__":
    main()
