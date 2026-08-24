#!/usr/bin/env python3
"""
Generate the RAW material for compare.json: for each question, ask the same
local model twice -- once grounded in the derived ontology + its provenance
(GraphRAG-style retrieval against demo/derived/_store.ttl, mirroring
studio/src-tauri/sidecars/agent/graphrag.ts's retrieve()), once against
plain keyword-chunk retrieval over the raw corpus text with no ontology at
all (demo/derived/_corpus_text.json, built by demo/corpus_text.py).

This script does NOT decide the "divergence" field. It writes both answers
and the documents each cited, verbatim, to a scratch file for a human (or
the calling agent) to read and judge honestly, per Task 8 decision 4: a
comparison that always favours the grounded answer is worthless, so nothing
here is allowed to pre-judge that call.

Usage:
    python3 demo/build_compare.py --questions demo/compare_questions.json \
        --scratch /path/to/compare_raw.json
"""
import argparse
import json
import os
import pathlib
import re
import subprocess
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
BIN = ROOT / "target" / "release" / "open-ontologies"
STORE = ROOT / "demo" / "derived" / "_store.ttl"
CORPUS_TEXT = ROOT / "demo" / "derived" / "_corpus_text.json"
MANIFEST = ROOT / "demo" / "corpus" / "dcat-us" / "MANIFEST.json"

LLM_BASE = os.environ.get("ONTO_LLM_BASE_URL", "http://localhost:8081/v1").rstrip("/")
LLM_KEY = os.environ.get("ONTO_LLM_API_KEY", "not-needed")
LLM_MODEL = os.environ.get("ONTO_LLM_MODEL", "")

P = ("PREFIX : <https://w3id.org/dcat-us-demo#>\n"
     "PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>\n"
     "PREFIX owl: <http://www.w3.org/2002/07/owl#>\n"
     "PREFIX prov: <http://www.w3.org/ns/prov#>\n")

ALL_DOC_IDS = sorted(pathlib.Path(e["file"]).stem for e in json.loads(MANIFEST.read_text()))


def cited_docs(answer: str, retrieved: list[str]) -> list[str]:
    """The documents an answer actually cited: scanned against every corpus
    document id, not only the ones retrieval happened to surface. A model
    that names a document it saw in its context should get credit for that
    regardless of which fixed list built the context; falling back to
    `retrieved` only fires when the answer names none of them at all (a
    prompt-following failure, not evidence of zero grounding)."""
    named = sorted(d for d in ALL_DOC_IDS if d in answer)
    return named or sorted(set(retrieved))


STOP = set(("the a an of to in for is are be and or that this with by on at as it its what which "
           "who whom why how when where does do did can could would should may might will shall "
           "about any all some there their they them we our you your i me my show tell give list "
           "find explain").split())


def call_model(prompt: str, max_tokens: int = 700, temperature: float = 0.1) -> str:
    model = LLM_MODEL
    if not model:
        req = urllib.request.Request(LLM_BASE + "/models", headers={"Authorization": f"Bearer {LLM_KEY}"})
        model = json.load(urllib.request.urlopen(req, timeout=30))["data"][0]["id"]
    req = urllib.request.Request(
        LLM_BASE + "/chat/completions",
        data=json.dumps({"model": model, "messages": [{"role": "user", "content": prompt}],
                         "temperature": temperature, "max_tokens": max_tokens,
                         "chat_template_kwargs": {"enable_thinking": False}}).encode(),
        headers={"Content-Type": "application/json", "Authorization": f"Bearer {LLM_KEY}"})
    msg = json.load(urllib.request.urlopen(req, timeout=600))["choices"][0]["message"]
    return (msg.get("content") or "").strip()


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


def terms(question: str) -> list[str]:
    words = re.findall(r"[a-z0-9][a-z0-9\-_]{2,}", question.lower())
    uniq = list(dict.fromkeys(w for w in words if w not in STOP and len(w) >= 4))
    return uniq[:8]


# --------------------------------------------------------------------------
# GROUNDED path: GraphRAG retrieval against the derived ontology
# --------------------------------------------------------------------------

def retrieve(question: str) -> dict:
    ts = terms(question)
    if not ts:
        return {"anchors": [], "facts": [], "claims": [], "provenance": [], "conflicts": []}

    filt = " || ".join(f'CONTAINS(LCASE(?l), "{t}") || CONTAINS(LCASE(STR(?e)), "{t}")' for t in ts)
    anchor_rows = sparql(P + f"""SELECT DISTINCT ?e ?l WHERE {{
      ?e rdfs:label ?l . FILTER({filt})
    }} ORDER BY STRLEN(?l) LIMIT 16""")
    anchors = [{"iri": r["e"], "label": r["l"]} for r in anchor_rows]
    if not anchors:
        return {"anchors": [], "facts": [], "claims": [], "provenance": [], "conflicts": []}

    values = " ".join(f':{a["iri"]}' for a in anchors)

    fact_rows = sparql(P + f"""SELECT DISTINCT ?s ?sl ?p ?o ?ol WHERE {{
      VALUES ?anchor {{ {values} }}
      {{ ?anchor ?p ?o . BIND(?anchor AS ?s) }}
      UNION {{ ?s ?p ?anchor . BIND(?anchor AS ?o) }}
      OPTIONAL {{ ?s rdfs:label ?sl }} OPTIONAL {{ ?o rdfs:label ?ol }}
      FILTER(!isBlank(?o) && !isBlank(?s))
    }} LIMIT 120""")
    facts = []
    for r in fact_rows:
        s, o = r.get("sl") or r["s"], r.get("ol") or r["o"]
        if s and o and r.get("p"):
            facts.append(f"{s} --[{r['p']}]--> {o}")
    facts = sorted(set(facts))[:80]

    claim_rows = sparql(P + f"""SELECT DISTINCT ?text ?doc WHERE {{
      VALUES ?anchor {{ {values} }}
      ?c a :Claim ; :claimText ?text ; :aboutEntity ?anchor ; :statedIn ?target .
      ?target :docId ?doc .
    }}""")
    claims = [{"text": r["text"], "doc": r.get("doc", "unattributed")} for r in claim_rows]

    prov_rows = sparql(P + f"""SELECT ?thing ?doc WHERE {{
      VALUES ?thing {{ {values} }}
      ?thing prov:wasDerivedFrom ?s . ?s :docId ?doc .
    }}""")
    provenance: dict[str, set[str]] = {}
    for r in prov_rows:
        provenance.setdefault(r["thing"], set()).add(r["doc"])

    conflict_rows = sparql(P + f"""SELECT DISTINCT ?subject ?a ?b WHERE {{
      VALUES ?subject {{ {values} }}
      ?subject a ?a, ?b . FILTER(STR(?a) < STR(?b))
      ?a rdfs:subClassOf* ?da . ?b rdfs:subClassOf* ?db .
      {{ ?da owl:disjointWith ?db }} UNION {{ ?db owl:disjointWith ?da }}
    }}""")
    conflicts = [f"{r['subject']} is typed as both {r['a']} and {r['b']}, "
                f"which the ontology declares disjoint" for r in conflict_rows]

    return {"anchors": anchors, "facts": facts, "claims": claims,
            "provenance": [{"thing": k, "sources": sorted(v)} for k, v in provenance.items()],
            "conflicts": conflicts}


def grounded_answer(question: str) -> dict:
    r = retrieve(question)
    doc_ids = sorted({c["doc"] for c in r["claims"]} | {s for p in r["provenance"] for s in p["sources"]})
    context = (
        f"ANCHORS: {', '.join(a['label'] for a in r['anchors']) or '(none found)'}\n\n"
        f"GRAPH FACTS:\n" + "\n".join(f"- {f}" for f in r["facts"][:40]) + "\n\n"
        f"CLAIMS (with the document that asserted them):\n"
        + "\n".join(f"- [{c['doc']}] {c['text']}" for c in r["claims"]) + "\n\n"
        f"PROVENANCE:\n" + "\n".join(f"- {p['thing']}: {', '.join(p['sources'])}" for p in r["provenance"]) + "\n\n"
        f"STRUCTURAL CONFLICTS:\n" + ("\n".join(f"- {c}" for c in r["conflicts"]) or "(none)")
    )
    prompt = (
        "You are answering a question using ONLY the knowledge graph context below, "
        "retrieved from an ontology derived from a document corpus. Every claim is tagged "
        "with the document id that asserted it, in square brackets.\n\n"
        f"CONTEXT:\n{context}\n\n"
        f"QUESTION: {question}\n\n"
        "Answer in 2-5 sentences. Every factual claim you make must be traceable to the "
        "context above; when you use a claim, name the document id it came from in "
        "parentheses, e.g. (profile-readme). If the context does not contain enough to "
        "answer, say so plainly rather than guessing."
    )
    answer = call_model(prompt)
    cited = cited_docs(answer, doc_ids)
    return {"answer": answer, "citations": cited, "_retrieved": r}


# --------------------------------------------------------------------------
# BASELINE path: keyword chunk retrieval over the raw corpus text, no ontology
# --------------------------------------------------------------------------

def load_chunks() -> list[dict]:
    docs = json.loads(CORPUS_TEXT.read_text())
    chunks = []
    for doc_id, d in docs.items():
        text = d["text"]
        paras = [p.strip() for p in re.split(r"\n\s*\n", text) if p.strip()]
        if len(paras) < 3:  # JSON/TTL have no blank-line paragraphs; use line blocks
            lines = [ln for ln in text.split("\n") if ln.strip()]
            paras = ["\n".join(lines[i:i + 25]) for i in range(0, len(lines), 25)]
        for i, p in enumerate(paras):
            chunks.append({"doc": doc_id, "idx": i, "text": p})
    return chunks


def baseline_answer(question: str, chunks: list[dict], k: int = 6) -> dict:
    ts = set(terms(question))
    scored = []
    for c in chunks:
        low = c["text"].lower()
        score = sum(low.count(t) for t in ts)
        if score:
            scored.append((score, c))
    scored.sort(key=lambda x: -x[0])
    top = [c for _, c in scored[:k]]
    doc_ids = sorted({c["doc"] for c in top})
    context = "\n\n".join(f"[{c['doc']}#{c['idx']}]\n{c['text'][:1200]}" for c in top)
    prompt = (
        "You are answering a question using ONLY the document excerpts below, retrieved "
        "by plain keyword matching (no ontology, no knowledge graph). Each excerpt is "
        "tagged with the document id it came from.\n\n"
        f"EXCERPTS:\n{context or '(no excerpt matched)'}\n\n"
        f"QUESTION: {question}\n\n"
        "Answer in 2-5 sentences. Every factual claim you make must be traceable to the "
        "excerpts above; when you use one, name the document id in parentheses. If the "
        "excerpts do not contain enough to answer, say so plainly rather than guessing."
    )
    answer = call_model(prompt)
    cited = cited_docs(answer, doc_ids)
    return {"answer": answer, "citations": cited, "_retrieved_docs": doc_ids}


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--questions", required=True, type=pathlib.Path)
    ap.add_argument("--scratch", required=True, type=pathlib.Path)
    args = ap.parse_args()

    questions = json.loads(args.questions.read_text())
    chunks = load_chunks()
    out = {}
    for q in questions:
        print(f"grounded: {q}")
        g = grounded_answer(q)
        print(f"baseline: {q}")
        b = baseline_answer(q, chunks)
        out[q] = {"grounded": g, "baseline": b}
        print(f"  grounded cites {g['citations']}")
        print(f"  baseline cites {b['citations']}")

    args.scratch.write_text(json.dumps(out, indent=2, ensure_ascii=False))
    print(f"\nwrote raw comparison material for {len(out)} questions to {args.scratch}")


if __name__ == "__main__":
    main()
