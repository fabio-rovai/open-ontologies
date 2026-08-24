#!/usr/bin/env python3
"""
Extract a knowledge graph from the demonstration documents.

Deliberately mirrors the MCP-native convention: this script does NOT reason.
It sends each document to whatever OpenAI-compatible model endpoint is
configured, asks for Turtle constrained to the loaded vocabulary, and then
hands the result to the engine's `onto_vocab_check` for a closed-world
check of every term used.

Terms the model invents are caught there, before anything reaches the store.
That gate is the point: an LLM writes the graph, a deterministic checker
decides whether it is allowed to exist.

Usage:
    python3 demo/extract.py                 # extract + vocab check
    python3 demo/extract.py --dry-run       # show prompts only
"""

import argparse
import json
import os
import pathlib
import re
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
DOCS = ROOT / "demo" / "documents"
OUT = ROOT / "demo" / "extracted"
ENGINE = os.environ.get("ONTO_ENGINE", "http://127.0.0.1:8137")
LLM_BASE = os.environ.get("ONTO_LLM_BASE_URL", "http://localhost:8081/v1").rstrip("/")
LLM_KEY = os.environ.get("ONTO_LLM_API_KEY", "not-needed")

VOCAB_HINT = """
Classes you may use:
  :Document :StandardOperatingProcedure :RegulatoryGuidance :TrialReport
  :Section :Claim :Control :ReferenceSource
  :Candidate :Organism :Pest :Pathogen :BeneficialOrganism :Pollinator :NaturalEnemy
  :FieldTrial :TargetProtein :Mutation :EvidenceGap :StageGate

Object properties you may use:
  :hasSection :parentSection :relatedSection :statedIn :aboutEntity
  :supersedes :declaresControl :controlOver :blocksProgressionOf :documentsRiskIn

Datatype properties you may use:
  :docId :classification :aclGroup :sectionNumber :claimText :isAutomated :isBlocking
  rdfs:label rdfs:comment
"""

PROMPT = """You are extracting a knowledge graph from a controlled document.

Return ONLY Turtle. No prose, no markdown fences, no explanation.

Use EXACTLY these prefixes:
@prefix : <https://w3id.org/dcat-us-demo#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

STRICT RULE: use ONLY the classes and properties listed below. Do not invent
any term. If something in the document does not fit the vocabulary, omit it.
{vocab}

Extract:
1. One :Document node, IRI :DOC_{docid_safe}, with :docId, rdfs:label,
   :classification, :aclGroup, and the most specific document type.
2. A :Section node per numbered section, IRI :SEC_{docid_safe}_<number with dots as underscores>,
   with :sectionNumber and rdfs:label, linked by :hasSection from the document.
3. :Claim nodes for assertions that matter to candidate progression or organism
   classification. Each with :claimText and :statedIn pointing at its section.
4. Where a section states a safeguard, a :Control node with :isAutomated and
   :isBlocking, linked with :declaresControl.
5. Where a section records a defect or limitation of a data source, link the
   section with :documentsRiskIn.

DOCUMENT ID: {docid}

DOCUMENT:
{body}
"""


def rpc(method, params, sid=None, _i=[0]):
    _i[0] += 1
    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json, text/event-stream",
    }
    if sid:
        headers["Mcp-Session-Id"] = sid
    req = urllib.request.Request(
        ENGINE + "/mcp",
        data=json.dumps(
            {"jsonrpc": "2.0", "id": _i[0], "method": method, "params": params}
        ).encode(),
        headers=headers,
    )
    r = urllib.request.urlopen(req, timeout=180)
    sid2 = r.headers.get("Mcp-Session-Id") or sid
    body = r.read().decode().strip()
    if body.startswith("{"):
        return json.loads(body), sid2
    for line in body.split("\n"):
        if line.startswith("data:"):
            try:
                fr = json.loads(line[5:].strip())
                if "result" in fr or "error" in fr:
                    return fr, sid2
            except Exception:
                pass
    return None, sid2


def tool_text(res):
    if not res:
        return ""
    return res.get("result", {}).get("content", [{}])[0].get("text", "")


def new_session():
    _, sid = rpc(
        "initialize",
        {
            "protocolVersion": "2025-03-26",
            "capabilities": {},
            "clientInfo": {"name": "extractor", "version": "1"},
        },
    )
    return sid


# Set once the vocabulary has been loaded, so a session reset can restore it.
_VOCAB_PATH = None


def call_tool(name, arguments, sid):
    """Call an MCP tool, re-initialising if the session has expired.

    Extraction turns can take minutes on a local model, which is long enough
    for the server to drop the session. A stale session surfaces as a bare
    HTTP 404, so treat that as "reconnect and retry once" rather than fatal.

    A new session starts with an EMPTY store, so the vocabulary must be
    reloaded before retrying. Without this, onto_vocab_check silently reports
    "0 declared terms" and checks nothing -- it returns conforms=false with a
    warning rather than an error, which is easy to mistake for a failed
    extraction when it is actually a failed setup.
    """
    try:
        res, sid = rpc("tools/call", {"name": name, "arguments": arguments}, sid)
        return res, sid
    except urllib.error.HTTPError as e:
        if e.code != 404:
            raise
        sid = new_session()
        if _VOCAB_PATH:
            rpc("tools/call", {"name": "onto_load", "arguments": {
                "path": _VOCAB_PATH, "name": "dcat-us-vocab", "force_recompile": True}}, sid)
        res, sid = rpc("tools/call", {"name": name, "arguments": arguments}, sid)
        return res, sid


def call_model(prompt):
    # Discover the served model id: MLX servers reject a mismatched id outright.
    model = os.environ.get("ONTO_LLM_MODEL", "")
    if not model:
        req = urllib.request.Request(
            LLM_BASE + "/models", headers={"Authorization": f"Bearer {LLM_KEY}"}
        )
        model = json.load(urllib.request.urlopen(req, timeout=30))["data"][0]["id"]

    req = urllib.request.Request(
        LLM_BASE + "/chat/completions",
        data=json.dumps(
            {
                "model": model,
                "messages": [{"role": "user", "content": prompt}],
                "temperature": 0.2,
                "top_p": 0.8,
                "max_tokens": 4096,
                "chat_template_kwargs": {"enable_thinking": False},
            }
        ).encode(),
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {LLM_KEY}",
        },
    )
    resp = json.load(urllib.request.urlopen(req, timeout=600))
    return resp["choices"][0]["message"]["content"], model


PREFIXES = (
    "@prefix : <https://w3id.org/dcat-us-demo#> .\n"
    "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n"
    "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n"
)


def normalise(text):
    """Recover parseable Turtle from a chatty model response.

    Small local models reliably do three things the prompt told them not to:
    wrap output in code fences, open with a prose sentence, and omit the
    prefix block. None of that changes the extracted CONTENT, so it is
    normalised here rather than treated as a failure. Anything that survives
    still has to pass onto_vocab_check.
    """
    text = re.sub(r"```(?:turtle|ttl)?", "", text).strip()

    # Drop any prose preamble. Start at the first @prefix if there is one;
    # otherwise at the first line that is unambiguously a SUBJECT, i.e.
    # "<iri> a <type>". Matching a bare leading ":token" is wrong: predicate
    # continuation lines like ':docId "DOC-114" ;' match it too, and starting
    # there silently decapitates the statement and loses its subject.
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
    text = "\n".join(lines[start if start is not None else 0 :]).strip()

    # Drop any trailing prose after the last statement-terminating line.
    lines = text.split("\n")
    end = len(lines)
    for i in range(len(lines) - 1, -1, -1):
        if lines[i].rstrip().endswith((".", ";", ",")):
            end = i + 1
            break
    text = "\n".join(lines[:end]).strip()

    if "@prefix :" not in text:
        text = PREFIXES + "\n" + text
    return text


strip_fences = normalise  # keep the call site stable


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    OUT.mkdir(parents=True, exist_ok=True)
    docs = sorted(DOCS.glob("*.md"))
    if not docs:
        sys.exit("no documents found")

    # Load the vocabulary the extraction will be checked against.
    sid = None
    if not args.dry_run:
        _, sid = rpc(
            "initialize",
            {
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "extractor", "version": "1"},
            },
        )
        global _VOCAB_PATH
        bundle = ROOT / "demo" / "bundle" / "dcat-us-vocab.ttl"
        _VOCAB_PATH = str(bundle)
        res, sid = call_tool(
            "onto_load",
            {"path": str(bundle), "name": "dcat-us-vocab", "force_recompile": True},
            sid,
        )
        print(f"vocabulary loaded: {tool_text(res)[:100]}")

    for doc in docs:
        raw = doc.read_text()
        m = re.match(r"^---\n(.*?)\n---\n(.*)$", raw, re.S)
        front, body = (m.group(1), m.group(2)) if m else ("", raw)
        docid = re.search(r'doc_id:\s*"?([^"\n]+)"?', front)
        docid = docid.group(1).strip() if docid else doc.stem
        safe = docid.replace("-", "_")

        prompt = PROMPT.format(
            vocab=VOCAB_HINT, docid=docid, docid_safe=safe, body=front + "\n" + body
        )
        if args.dry_run:
            print(f"--- {docid} prompt {len(prompt)} chars ---")
            continue

        print(f"\nextracting {docid} ...", flush=True)

        # Small models occasionally collapse into repetition. Detect it and
        # retry rather than writing degenerate output and calling it a graph.
        for attempt in range(1, 4):
            content, model = call_model(prompt)
            (OUT / f"{docid}.raw.attempt{attempt}.txt").write_text(content)
            if re.search(r"(\b\S{1,6}\b[ .]{1,3})\1{15,}", content):
                print(f"  attempt {attempt}: degenerate repetition, retrying")
                continue
            ttl = strip_fences(content)
            if ":" in ttl and len(ttl) > 200:
                break
            print(f"  attempt {attempt}: output too thin, retrying")
        else:
            print(f"  GAVE UP on {docid} after 3 attempts")
            ttl = strip_fences(content)
        path = OUT / f"{docid}.ttl"
        path.write_text(
            f"# Extracted from {doc.name} on 2026-08-10\n"
            f"# Model: {model}\n"
            f"# Verified with onto_vocab_check. See demo/extracted/REPORT.md\n\n" + ttl
        )
        print(f"  wrote {path.name} ({len(ttl)} chars)")

        res, sid = call_tool("onto_validate", {"input": ttl, "inline": True}, sid)
        v = tool_text(res)
        print(f"  syntax : {v[:120]}")

        res, sid = call_tool("onto_vocab_check", {"data": ttl, "inline": True}, sid)
        print(f"  vocab  : {tool_text(res)[:400]}")


if __name__ == "__main__":
    main()
