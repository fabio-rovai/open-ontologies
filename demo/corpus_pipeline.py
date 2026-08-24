#!/usr/bin/env python3
"""
Read a folder of Word documents, build a knowledge graph, find the
contradictions between them.

This is the showcase end to end:

  1. READ      every .docx in demo/corpus/dcat-us (text via pandoc)
  2. EXTRACT   a knowledge graph per document, constrained to the ontology
  3. VERIFY    closed-world check that no term was invented
  4. LOAD      merge every document graph into one store
  5. SCAN      find the passages that contradict each other

Nothing leaves the machine: extraction runs against whatever model endpoint is
configured, which by default is a local one.

Usage:
    python3 demo/corpus_pipeline.py
    python3 demo/corpus_pipeline.py --limit 3      # quick run
    python3 demo/corpus_pipeline.py --cached       # skip extraction, reuse output
"""

import argparse
import functools
import concurrent.futures as cf
import json
import os
import pathlib
import re
import subprocess
import sys
import time
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
CORPUS = ROOT / "demo" / "corpus" / "dcat-us"
OUT = ROOT / "demo" / "corpus_extracted"
BIN = ROOT / "target" / "release" / "open-ontologies"
VOCAB = ROOT / "demo" / "bundle" / "dcat-us-vocab.ttl"
ENGINE = os.environ.get("ONTO_ENGINE", "http://127.0.0.1:8137")
LLM_BASE = os.environ.get("ONTO_LLM_BASE_URL", "http://localhost:8081/v1").rstrip("/")
LLM_KEY = os.environ.get("ONTO_LLM_API_KEY", "not-needed")

if os.environ.get("NO_COLOR"):
    BOLD = DIM = RED = GRN = YEL = OFF = ""
else:
    BOLD, DIM, RED, GRN, YEL, OFF = "\033[1m", "\033[2m", "\033[31m", "\033[32m", "\033[33m", "\033[0m"

VOCAB_HINT = """Classes: :Document :StandardOperatingProcedure :RegulatoryGuidance :TrialReport
  :Section :Claim :Control :ReferenceSource :Candidate :Organism :Pest :Pathogen
  :BeneficialOrganism :Pollinator :NaturalEnemy :FieldTrial :TargetProtein :Mutation
  :EvidenceGap :StageGate
Object properties: :hasSection :parentSection :relatedSection :statedIn :aboutEntity
  :supersedes :declaresControl :controlOver :blocksProgressionOf :documentsRiskIn
Datatype properties: :docId :classification :aclGroup :sectionNumber :claimText
  :isAutomated :isBlocking rdfs:label rdfs:comment"""

PROMPT = """Extract a knowledge graph from this controlled document.

Return ONLY Turtle. No prose, no fences.

Prefixes:
@prefix : <https://w3id.org/dcat-us-demo#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

Use ONLY these terms. Invent nothing:
{vocab}

Extract:
1. One :Document node :DOC_{safe} with :docId, rdfs:label, :classification, :aclGroup
   and its most specific document type.
2. A :Section per numbered section, IRI :SEC_{safe}_<number with dots as underscores>,
   with :sectionNumber and rdfs:label, linked by :hasSection.
3. :Claim nodes for assertions about candidate progression, organism classification,
   thresholds, or safeguards. Each with :claimText, :statedIn, and :aboutEntity where
   the subject is identifiable.
4. Where a section states a safeguard, a :Control with :isAutomated and :isBlocking.

DOCUMENT ID: {docid}

{body}
"""

PREFIXES = ("@prefix : <https://w3id.org/dcat-us-demo#> .\n"
            "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n"
            "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n")


def docx_text(path):
    """Word document to plain text."""
    r = subprocess.run(["pandoc", str(path), "-t", "plain", "--wrap=none"],
                       capture_output=True, text=True)
    return r.stdout


def normalise(text):
    text = re.sub(r"```(?:turtle|ttl)?", "", text).strip()
    lines = text.split("\n")
    start = None
    for i, line in enumerate(lines):
        if line.strip().startswith(("@prefix", "@base")):
            start = i
            break
    if start is None:
        for i, line in enumerate(lines):
            if re.match(r"^\s*:\S+\s+(a|rdf:type)\s", line):
                start = i
                break
    text = "\n".join(lines[start if start is not None else 0:]).strip()
    lines = text.split("\n")
    for i in range(len(lines) - 1, -1, -1):
        if lines[i].rstrip().endswith((".", ";", ",")):
            text = "\n".join(lines[: i + 1]).strip()
            break
    if "@prefix :" not in text:
        text = PREFIXES + "\n" + text
    return text


def call_model(prompt):
    model = os.environ.get("ONTO_LLM_MODEL", "")
    if not model:
        req = urllib.request.Request(LLM_BASE + "/models", headers={"Authorization": f"Bearer {LLM_KEY}"})
        model = json.load(urllib.request.urlopen(req, timeout=30))["data"][0]["id"]
    req = urllib.request.Request(
        LLM_BASE + "/chat/completions",
        data=json.dumps({"model": model, "messages": [{"role": "user", "content": prompt}],
                         "temperature": 0.2, "max_tokens": 3000,
                         "chat_template_kwargs": {"enable_thinking": False}}).encode(),
        headers={"Content-Type": "application/json", "Authorization": f"Bearer {LLM_KEY}"})
    return json.load(urllib.request.urlopen(req, timeout=900))["choices"][0]["message"]["content"]


def extract_one(path):
    raw = docx_text(path)
    m = re.match(r"([A-Z]+-[0-9][0-9A-Za-z\-]*?)(?=-[a-z])", path.stem)
    docid = m.group(1) if m else path.stem
    safe = docid.replace("-", "_")
    body = raw[:14000]
    for attempt in range(3):
        try:
            content = call_model(PROMPT.format(vocab=VOCAB_HINT, docid=docid, safe=safe, body=body))
        except Exception as e:
            return docid, None, f"model error: {e}"
        if re.search(r"(\b\S{1,6}\b[ .]{1,3})\1{15,}", content):
            continue
        ttl = normalise(content)
        if len(ttl) > 200:
            out = OUT / f"{docid}.ttl"
            out.write_text(f"# Extracted from {path.name} on 2026-08-10\n\n{ttl}")
            return docid, out, None
    return docid, None, "degenerate output after 3 attempts"


def mcp(method, params, sid=[None]):
    h = {"Content-Type": "application/json", "Accept": "application/json, text/event-stream"}
    if sid[0]:
        h["Mcp-Session-Id"] = sid[0]
    req = urllib.request.Request(ENGINE + "/mcp", headers=h, data=json.dumps(
        {"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode())
    r = urllib.request.urlopen(req, timeout=120)
    sid[0] = r.headers.get("Mcp-Session-Id") or sid[0]
    body = r.read().decode().strip()
    if body.startswith("{"):
        return json.loads(body)
    for line in body.split("\n"):
        if line.startswith("data:"):
            try:
                fr = json.loads(line[5:].strip())
                if "result" in fr or "error" in fr:
                    return fr
            except Exception:
                pass
    return {}


def vocab_check(ttl):
    """Closed-world term check. The vocabulary must be loaded in the SAME
    session, so reload it here rather than assuming it survived."""
    try:
        mcp("initialize", {"protocolVersion": "2025-03-26", "capabilities": {},
                           "clientInfo": {"name": "pipeline", "version": "1"}})
        mcp("tools/call", {"name": "onto_load", "arguments":
            {"path": str(VOCAB), "name": "dcat-us-vocab", "force_recompile": True}})
        res = mcp("tools/call", {"name": "onto_vocab_check",
                                 "arguments": {"data": ttl, "inline": True}})
        return json.loads(res.get("result", {}).get("content", [{}])[0].get("text", "{}"))
    except Exception as e:
        return {"conforms": None, "error": str(e)}


def engine(cmds):
    return subprocess.run([str(BIN), "batch", "-"], input="\n".join(cmds) + "\n",
                          capture_output=True, text=True).stdout.strip()


print = functools.partial(__builtins__.print if not isinstance(__builtins__, dict) else __builtins__["print"], flush=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--cached", action="store_true")
    ap.add_argument("--workers", type=int, default=4)
    ap.add_argument("--corpus", default=str(CORPUS),
                    help="folder of documents to read")
    args = ap.parse_args()

    OUT.mkdir(exist_ok=True)
    corpus_dir = pathlib.Path(args.corpus)
    docs = sorted(corpus_dir.glob("*.docx")) or sorted(corpus_dir.glob("*.md"))
    if args.limit:
        docs = docs[: args.limit]
    if not docs:
        sys.exit(f"no .docx or .md found in {corpus_dir}")

    t0 = time.time()
    print(f"\n{BOLD}STAGE 1  READ{OFF}  {len(docs)} documents from {corpus_dir.name}/")
    for d in docs:
        print(f"  {DIM}{d.name}{OFF}")

    extracted = []
    if args.cached:
        extracted = sorted(OUT.glob("*.ttl"))
        print(f"\n{BOLD}STAGE 2  EXTRACT{OFF}  {DIM}cached, {len(extracted)} graphs reused{OFF}")
    else:
        print(f"\n{BOLD}STAGE 2  EXTRACT{OFF}  knowledge graph per document")
        print(f"  {DIM}model endpoint {LLM_BASE} ({args.workers} in parallel){OFF}")
        with cf.ThreadPoolExecutor(max_workers=args.workers) as pool:
            for docid, out, err in pool.map(extract_one, docs):
                if err:
                    print(f"  {RED}FAIL{OFF} {docid:14s} {err}")
                else:
                    print(f"  {GRN}ok{OFF}   {docid:14s} {out.name}")
                    extracted.append(out)

    print(f"\n{BOLD}STAGE 3  VERIFY{OFF}  closed-world check that no term was invented")
    ok = bad = 0
    for f in extracted:
        d = vocab_check(f.read_text())
        conforms, halluc = d.get("conforms"), d.get("hallucinated_terms", [])
        if conforms:
            ok += 1
            print(f"  {GRN}conforms{OFF}  {f.stem}")
        else:
            bad += 1
            print(f"  {YEL}review  {OFF}  {f.stem}  {halluc if halluc else ''}")

    print(f"\n{BOLD}STAGE 4  LOAD{OFF}  merge every document graph into the live store")
    # Load through the RUNNING engine over MCP, not a throwaway CLI process.
    # Two things depend on this: the graph view renders the store that was
    # just built, and every load is recorded in the lineage trail.
    try:
        mcp("initialize", {"protocolVersion": "2025-03-26", "capabilities": {},
                           "clientInfo": {"name": "pipeline", "version": "1"}})
        mcp("tools/call", {"name": "onto_clear", "arguments": {}})
        base = ROOT / "demo" / "bundle" / "dcat-us-full.ttl"
        mcp("tools/call", {"name": "onto_load", "arguments":
            {"path": str(base), "name": "corpus", "force_recompile": True}})
        merged = base.read_text().rstrip() + "\n"
        for f in extracted:
            body = "\n".join(l for l in f.read_text().splitlines()
                              if not l.lstrip().startswith(("@prefix", "@base", "#"))).strip()
            if not body:
                continue
            # A block that does not terminate its last statement will glue onto
            # the next file and produce a parse error hundreds of lines away.
            if not body.endswith("."):
                body += " ."
            merged += "\n" + body + "\n"
        staged = ROOT / "demo" / "corpus_extracted" / "_merged.ttl"
        staged.write_text(merged)
        mcp("tools/call", {"name": "onto_load", "arguments":
            {"path": str(staged), "name": "corpus", "force_recompile": True}})
        # onto_load does not write lineage; a save does. Persist the merged
        # store so the ingestion appears in the lineage trail.
        mcp("tools/call", {"name": "onto_save", "arguments":
            {"path": str(ROOT / "demo" / "corpus_extracted" / "_live.ttl")}})
        res = mcp("tools/call", {"name": "onto_stats", "arguments": {}})
        st = json.loads(res.get("result", {}).get("content", [{}])[0].get("text", "{}"))
        print(f"  {st.get('triples','?')} triples, {st.get('classes','?')} classes, "
              f"{st.get('individuals','?')} individuals  {DIM}(loaded into the live engine){OFF}")
    except Exception as e:
        print(f"  {YEL}live engine unavailable ({e}); falling back to a local load{OFF}")
        res = engine(["clear", f"load {ROOT/'demo'/'bundle'/'dcat-us-full.ttl'}"]
                     + [f"load {f}" for f in extracted] + ["stats"])
        try:
            st = json.loads(res.split("\n")[-1])["result"]
            print(f"  {st['triples']} triples, {st['classes']} classes, {st['individuals']} individuals")
        except Exception:
            print(f"  {DIM}{res[-200:]}{OFF}")

    print(f"\n{BOLD}STAGE 5  SCAN{OFF}  find passages that contradict each other")
    scan = subprocess.run([sys.executable, str(ROOT / "demo" / "contradiction_scan.py")],
                          capture_output=True, text=True, cwd=ROOT).stdout
    for line in scan.splitlines():
        if "CONFLICT" in line:
            print(f"  {RED}{line.strip()}{OFF}")
        elif "TWO sources" in line or "candidate pairs after blocking" in line:
            print(f"  {YEL}{line.strip()}{OFF}")

    print(f"\n{BOLD}{len(docs)} documents processed in {time.time()-t0:.0f}s."
          f"  {ok} graphs conformed, {bad} flagged for review.{OFF}")
    print(f"{DIM}Every fact carries the document and section it came from, and inherits"
          f"\nthat document's access group.{OFF}\n")


if __name__ == "__main__":
    main()
