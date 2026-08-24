#!/usr/bin/env python3
"""
Ontology out of text: derive the ontology FROM a document corpus, then
populate it, with PII tokenised before anything reaches a model.

This is domain-agnostic. Nothing about any particular subject is hardcoded;
the vocabulary is whatever the documents turn out to be about.

  STAGE 1  READ       every document in the folder
  STAGE 2  TOKENISE   detect and replace sensitive values with stable tokens
  STAGE 3  DERIVE     propose classes, properties and disjointness per
                      document, then merge into ONE candidate ontology
  STAGE 4  POPULATE   extract instances, constrained to the derived ontology
  STAGE 5  VERIFY     closed-world check that nothing was invented
  STAGE 6  SCAN       reason, and report contradictions

The two-phase split is the point. A single pass either invents terms freely,
which gives an unusable model, or is pinned to a vocabulary written in advance,
which cannot describe documents nobody has read yet. Deriving first and
constraining second gives a vocabulary that came from the corpus AND a
closed-world check with teeth.

Tokenisation happens BEFORE derivation, so no model ever sees a real value.
Tokens are deterministic, which means the same value produces the same token in
every document: the token doubles as a join key for exact-match entity
resolution, achieved without any component handling the raw value.

Usage:
    python3 demo/ontology_from_docs.py --corpus demo/corpus/dcat-us
    python3 demo/ontology_from_docs.py --corpus demo/corpus/dcat-us --cached
"""

import argparse
import concurrent.futures as cf
import hashlib
import hmac
import json
import os
import pathlib
import re
import subprocess
import sys
import time
import urllib.error
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
BIN = ROOT / "target" / "release" / "open-ontologies"
ENGINE = os.environ.get("ONTO_ENGINE", "http://127.0.0.1:8137")
LLM_BASE = os.environ.get("ONTO_LLM_BASE_URL", "http://localhost:8081/v1").rstrip("/")
LLM_KEY = os.environ.get("ONTO_LLM_API_KEY", "not-needed")
NS = "https://w3id.org/dcat-us-demo#"

# Samples voted over when proposing disjointness. Odd, so a majority exists.
SAMPLES = int(os.environ.get("ONTO_DISJOINT_SAMPLES", "3"))

# Repair attempts per non-conforming graph before it goes to human review.
REPAIR_ROUNDS = int(os.environ.get("ONTO_REPAIR_ROUNDS", "3"))

# Every automated resolution decision goes on the record: what changed, in
# which document, by which rule. The Studio's resolution panel renders this
# ledger and the judgement calls in it can be reverted.
LEDGER = {"path": None}


def record(kind, **kw):
    if LEDGER["path"]:
        with open(LEDGER["path"], "a") as fh:
            fh.write(json.dumps({"kind": kind, **kw}) + "\n")

if os.environ.get("NO_COLOR"):
    BOLD = DIM = RED = GRN = YEL = OFF = ""
else:
    BOLD, DIM, RED, GRN, YEL, OFF = "\033[1m", "\033[2m", "\033[31m", "\033[32m", "\033[33m", "\033[0m"

print = __builtins__.print if not isinstance(__builtins__, dict) else __builtins__["print"]


# --------------------------------------------------------------------------
# Tokenisation: delegated to the swappable detector + vault module
# --------------------------------------------------------------------------

sys.path.insert(0, str(ROOT / "demo"))
from tokenisation import build as build_tokeniser  # noqa: E402


# --------------------------------------------------------------------------
# Model + engine plumbing
# --------------------------------------------------------------------------

def call_model(prompt: str, max_tokens: int = 3000, attempts: int = 4,
               temperature: float = 0.2, timeout: int = 900) -> str:
    """Call the model, backing off on overload.

    A local inference server under parallel load answers 503 or 504 rather
    than queueing. Treating that as a permanent failure loses most of a run,
    which is exactly what happened before this was added.
    """
    last = None
    for i in range(attempts):
        try:
            return _call_model_once(prompt, max_tokens, temperature, timeout)
        except urllib.error.HTTPError as e:
            if e.code not in (429, 500, 502, 503, 504):
                raise
            last = e
        except urllib.error.URLError as e:
            last = e
        time.sleep(2 ** i)
    raise RuntimeError(f"model unavailable after {attempts} attempts: {last}")


def _call_model_once(prompt: str, max_tokens: int = 3000,
                     temperature: float = 0.2, timeout: int = 900) -> str:
    model = os.environ.get("ONTO_LLM_MODEL", "")
    if not model:
        req = urllib.request.Request(LLM_BASE + "/models", headers={"Authorization": f"Bearer {LLM_KEY}"})
        model = json.load(urllib.request.urlopen(req, timeout=30))["data"][0]["id"]
    req = urllib.request.Request(
        LLM_BASE + "/chat/completions",
        data=json.dumps({"model": model, "messages": [{"role": "user", "content": prompt}],
                         "temperature": temperature, "max_tokens": max_tokens,
                         "chat_template_kwargs": {"enable_thinking": False}}).encode(),
        headers={"Content-Type": "application/json", "Authorization": f"Bearer {LLM_KEY}"})
    msg = json.load(urllib.request.urlopen(req, timeout=900))["choices"][0]["message"]
    # A null content field is a valid response shape, not an error: the model
    # produced no visible text. Callers regex over this, so hand back a string.
    return msg.get("content") or ""


def mcp(method, params, sid=[None]):
    h = {"Content-Type": "application/json", "Accept": "application/json, text/event-stream"}
    if sid[0]:
        h["Mcp-Session-Id"] = sid[0]
    req = urllib.request.Request(ENGINE + "/mcp", headers=h, data=json.dumps(
        {"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode())
    r = urllib.request.urlopen(req, timeout=180)
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


def tool_text(res):
    return res.get("result", {}).get("content", [{}])[0].get("text", "") if res else ""


def engine(cmds):
    return subprocess.run([str(BIN), "batch", "-"], input="\n".join(cmds) + "\n",
                          capture_output=True, text=True).stdout.strip()


# Declare every prefix a model might plausibly use. One undeclared prefix
# (rdf: was the culprit) aborts the parse at first use, so a 2700-line
# ontology silently loaded as 195 triples: all the classes, none of the
# properties, because classes happen to come first in the file.
PREFIXES = (f"@prefix : <{NS}> .\n"
            "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n"
            "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n"
            "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n"
            "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n"
            "@prefix skos: <http://www.w3.org/2004/02/skos/core#> .\n"
            "@prefix dcterms: <http://purl.org/dc/terms/> .\n")


def is_valid_turtle(ttl: str) -> tuple[bool, str]:
    """Parse-check a fragment on its own."""
    import tempfile
    with tempfile.NamedTemporaryFile("w", suffix=".ttl", delete=False) as fh:
        fh.write(ttl)
        path = fh.name
    try:
        out = subprocess.run([str(BIN), "validate", path], capture_output=True, text=True).stdout
        return ('"ok":true' in out), out.strip()[:160]
    finally:
        pathlib.Path(path).unlink(missing_ok=True)


# Structural vocabulary the chunker imposes on every corpus. The extractor
# reaches for it in every document because the documents really are divided
# into sections; no per-document derivation proposes it, because it is a
# property of how documents are read rather than of what they say.
STRUCTURAL = """
:Section a owl:Class ;
    rdfs:label "Section" ;
    rdfs:comment "A numbered division of a document, as identified by the chunker." .

:hasSection a owl:ObjectProperty ;
    rdfs:label "has section" ;
    rdfs:domain :Document ;
    rdfs:range :Section .

:hasSectionNumber a owl:DatatypeProperty ;
    rdfs:label "has section number" ;
    rdfs:domain :Section ;
    rdfs:range xsd:string .
"""


def body_of(frag: str) -> str:
    """The statements of a fragment, with any prefix header removed."""
    return "\n".join(l for l in frag.splitlines()
                     if not l.lstrip().startswith(("@prefix", "@base"))).strip()


def mend(body: str) -> str:
    """Repair the syntax slips a generator makes at the end of a statement.

    A predicate list that ends `; .` or a property block whose last predicate
    keeps its separator are the two failures seen in practice. Both are
    unambiguous, so fixing them costs nothing and recovers a whole fragment
    that would otherwise be discarded along with every term it declares.
    """
    body = re.sub(r";\s*\.", " .", body)          # dangling separator before the stop
    body = re.sub(r";\s*\n\s*(?=[:\w]+\s+a\s)", " .\n", body)  # missing stop between subjects
    body = re.sub(r",\s*\.", " .", body)

    # A bare `disjointWith`. Turtle has no default predicate namespace, so an
    # unprefixed name is a parse error, not a lookup failure.
    body = re.sub(r"(?<![:\w])disjointWith\b", "owl:disjointWith", body)

    # A standard prefix pasted after the default one, giving `:rdfs:label`.
    # This parses, which is what makes it dangerous: it silently mints a term
    # in the local namespace that looks like the standard one, and the
    # closed-world check then correctly reports a term nobody declared.
    body = re.sub(r"(?<![\w:]):(rdf|rdfs|owl|xsd|skos|dcterms):", r"\1:", body)

    # A type assertion whose object was lost, leaving the keyword exposed.
    body = re.sub(r"\ba\s+a\b", " a", body)

    # An IRI the generator split with a space before a token:
    # ":TimePoint TOK_DATE_x a :TimePoint". Rejoin with an underscore; a bare
    # token in predicate position truncates every file it is merged into.
    body = re.sub(r"^(:\w+)[ \t]+(TOK_\w+)\b", r"\1_\2", body, flags=re.M)

    # A statement whose subject was dropped, leaving the predicate in subject
    # position. Nothing can recover the intended subject, so the statement goes
    # rather than the fragment. Only an unindented occurrence starts a
    # statement: an indented one is a continuation of a valid subject above it,
    # and removing those is what breaks an otherwise sound block. The whole
    # orphan runs to its terminating dot.
    kept, skipping = [], False
    for line in body.splitlines():
        if re.match(r"^(?:owl:)?:?disjointWith\s", line):
            skipping = True
        if skipping:
            if line.rstrip().endswith("."):
                skipping = False
            continue
        kept.append(line)
    body = "\n".join(kept)

    # A statement left unterminated before the next subject begins. The blank
    # line and the following `:Term a` are together unambiguous.
    body = re.sub(r"(?<=[\"\w>])\s*\n(\s*\n\s*)(?=:\S+\s+(?:a|rdf:type)\s)",
                  r" .\n\1", body)
    return body


def rehead(frag: str) -> str:
    """A fragment under the canonical prefix header.

    The generator writes its own header and routinely omits a prefix it then
    uses (`rdf:` above all). Validating the fragment as written therefore
    rejects it for a declaration problem rather than a modelling one. Strip
    whatever header it wrote, impose ours, and judge the statements.
    """
    return PREFIXES + "\n" + mend(body_of(frag))


def normalise(text: str) -> str:
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
    # The same mechanical repairs the vocabulary fragments get. Extracted data
    # comes from the same generator and fails in the same ways; leaving them
    # out here is why a whole graph was being discarded for one missing dot.
    body = mend(body_of(text))
    if body and not body.endswith("."):
        body += " ."
    return PREFIXES + "\n" + body


def docx_text(path: pathlib.Path) -> str:
    if path.suffix == ".docx":
        return subprocess.run(["pandoc", str(path), "-t", "plain", "--wrap=none"],
                              capture_output=True, text=True).stdout
    # .md, .json, .ttl (and anything else pandoc has no business touching)
    # are already text; reading them directly is what lets a real corpus of
    # mixed document types (Task 7's DCAT-US corpus is markdown + JSON
    # Schema + SHACL Turtle) reach the model at all.
    return path.read_text()


# --------------------------------------------------------------------------
# Stage 3: derive the vocabulary
# --------------------------------------------------------------------------

DERIVE_PROMPT = """Read this document and propose the ONTOLOGY it implies. You are
defining the vocabulary, not describing the specific facts.

Return ONLY Turtle. No prose, no code fences.

@prefix : <{ns}> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

Produce:
1. owl:Class declarations for every KIND of thing the document is about. Use
   UpperCamelCase local names. Give every class rdfs:label and rdfs:comment.
2. rdfs:subClassOf where one kind is clearly a kind of another.
3. owl:ObjectProperty for relationships between things, each with rdfs:domain,
   rdfs:range and rdfs:label.
4. owl:DatatypeProperty for attributes, each with rdfs:domain, rdfs:range
   (an xsd type) and rdfs:label.
5. owl:disjointWith between any two classes that CANNOT both describe the same
   thing at the same time. Be deliberate: this is what lets a reasoner detect
   that two documents contradict each other.

Do NOT create individuals. Classes and properties only.

DOCUMENT:
{body}
"""

POPULATE_PROMPT = """Extract the FACTS from this document as instances of a
given ontology.

Return ONLY Turtle. No prose, no code fences.

@prefix : <{ns}> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

Use ONLY the classes and properties in this vocabulary. Invent nothing:
{vocab}

Every class and property you write must appear VERBATIM in the list above.
Before writing any term, find it in the list and copy it exactly.

Do NOT build a property name by joining a class name to a field name. Writing
:hasChangeLogEntryChangeApplied because the subject is a :ChangeLogEntry and the
field is "applied" produces a term that does not exist. Use the property the
vocabulary already provides, even when its name is shorter or more general than
the wording in the document.

If the document asserts something the vocabulary genuinely cannot express,
OMIT it. A smaller correct graph is the goal; an invented term is a defect,
not a partial success.

Produce:
1. One :Document individual, IRI :DOC_{safe}, with rdfs:label and any
   document-control values present.
2. A :Section individual per numbered section where the vocabulary has a
   suitable class, IRI :SEC_{safe}_<number with dots as underscores>.
3. Individuals for the things the document states facts about, typed with the
   most specific class available, each with rdfs:label.

   When the document says a thing IS a kind of something, and the vocabulary
   has a class for that kind, TYPE the thing with that class. Never create an
   individual to stand for the kind itself.

     "Dataset07 is catalogued as a published dataset"
       correct:   :Dataset07 a :PublishedDataset ; rdfs:label "Dataset07" .
       WRONG:     :Dataset07 a :Dataset .
                  :PUBLISHED_DATASET a :Dataset ; rdfs:label "Published dataset" .

   The wrong form loses the fact entirely: it records that something called
   "published dataset" exists, not that Dataset07 is one.

   Give each individual EXACTLY ONE type: the single most specific class that
   fits. A registry row listing a dataset's licence, update frequency and
   access level describes ONE dataset; it does not make that dataset a
   licence or an update frequency. Attach those as properties, never as extra
   types.

     correct:   :Dataset07 a :PublishedDataset ;
                    :hasLicence :LICENCE_CC_BY ;
                    :hasUpdateFrequency "monthly" .
     WRONG:     :Dataset07 a :Dataset, :Licence, :UpdateFrequency .

4. Name each individual after the thing itself and nothing else. The IRI for
   Dataset07 is :Dataset07 in every document that mentions it. Do not prefix it
   with the document id, and do not qualify it with the context it appeared in
   (:DATASET07_STR, :DATASET_CATALOGUE_DATASET07_REGISTRY). The same real thing
   must get the same IRI everywhere, because that is what lets facts from
   different documents meet.
5. The relationships and attribute values the document asserts.

Values that look like TOK_ followed by capitals and a hex string are tokens
standing in for redacted content. Use them verbatim as identifiers or literals.
Do not attempt to interpret or expand them.

DOCUMENT:
{body}
"""


def derive_one(args):
    path, tokenised, docid = args
    try:
        raw = call_model(DERIVE_PROMPT.format(ns=NS, body=tokenised[:12000]))
    except Exception as e:
        return docid, None, f"model error: {e}"
    if re.search(r"(\b\S{1,6}\b[ .]{1,3})\1{15,}", raw):
        return docid, None, "degenerate output"
    return docid, normalise(raw), None


def snap_to_vocab(body: str, declared: set[str]) -> str:
    """Snap a term to a declared term it differs from only in case.

    The generator writes `:HasPublisher` where the ontology declares
    `:hasPublisher`. Closed-world checking is right to reject it, but the intent
    is unambiguous and there is exactly one candidate, so resolving it here is
    sound rather than charitable. Anything that does not match a declared term
    under case folding is left alone: that is real invention and it must stay
    visible to the check.
    """
    fold = {}
    for d in declared:
        fold.setdefault(d.lower(), d)

    def swap(m: re.Match) -> str:
        term = m.group(1)
        target = fold.get(term.lower())
        return f":{target}" if target and target != term else m.group(0)

    return re.sub(r":(\w+)", swap, body)


def populate_one(args):
    path, tokenised, docid, vocab = args
    safe = docid.replace("-", "_")
    try:
        raw = call_model(POPULATE_PROMPT.format(ns=NS, vocab=vocab, safe=safe, body=tokenised[:12000]))
    except Exception as e:
        return docid, None, f"model error: {e}"
    if re.search(r"(\b\S{1,6}\b[ .]{1,3})\1{15,}", raw):
        return docid, None, "degenerate output"
    declared = set(re.findall(r":(\w+)", vocab))
    return docid, snap_to_vocab(normalise(raw), declared), None


DISJOINT_PROMPT = """Here is a list of classes from an ontology derived from a
document corpus.

Identify pairs that are MUTUALLY EXCLUSIVE: a single thing cannot be both at
the same time. Examples of the kind of pair that qualifies: a dataset that is
published versus one that is a draft; a check that passed versus failed; a
record that is authenticated versus unauthenticated.

Return ONLY Turtle, one statement per line, no prose and no fences:

:ClassA owl:disjointWith :ClassB .

Only assert a pair when you are confident. An incorrect disjointness makes the
ontology inconsistent, which is worse than a missing one. Return nothing at all
if no pair qualifies.

CLASSES:
{classes}
"""


def _sibling_groups(classes: list[str], cap: int = 60) -> list[list[str]]:
    """Batch the class list into groups worth asking a disjointness question about.

    Asking about all 272 classes in one call returns nothing at all: the model
    answers 20 classes with 3700 characters, 150 with 4700, and 272 with an
    empty string. That failure is silent, so the run reports zero axioms and
    looks merely unlucky.

    Grouping is by head noun rather than arbitrary slicing, because that is
    where mutual exclusivity actually lives. `PublishedDataset` and
    `DraftDataset` are candidates precisely because they are both kinds
    of `Dataset`; two classes with nothing in common almost never need an axiom
    between them. Small groups are pooled so a group of one is not a call.
    """
    def head(name: str) -> str:
        parts = re.findall(r"[A-Z][a-z0-9]*", name)
        return parts[-1] if parts else name

    by_head: dict[str, list[str]] = {}
    for c in classes:
        by_head.setdefault(head(c), []).append(c)

    groups, pool = [], []
    for _, members in sorted(by_head.items()):
        if len(members) >= 2:
            for i in range(0, len(members), cap):
                groups.append(sorted(members[i:i + cap]))
        else:
            pool.extend(members)
    for i in range(0, len(pool), cap):
        chunk = sorted(pool[i:i + cap])
        if len(chunk) >= 2:
            groups.append(chunk)
    return groups


def add_disjointness(merged_path: pathlib.Path) -> int:
    """Propose disjointness across the merged vocabulary.

    Deliberately a SEPARATE pass over the merged class list rather than part of
    per-document derivation. Disjointness is a cross-document property: whether
    "published" and "draft" are mutually exclusive is not visible from
    inside one document that only mentions one of them. Asking per document
    reliably produced none at all.

    Stability matters more here than coverage. These axioms decide what the
    reasoner is even capable of detecting, so an axiom set that changes run to
    run makes every downstream contradiction count unreproducible. Sampling
    once gave 57, 68, 93, 114 and 150 pairs on an identical class list. So:
    sample SAMPLES times, keep only pairs a majority agrees on, and cache the
    result against a hash of the class list. Re-running over the same
    vocabulary is then exactly reproducible, and a changed vocabulary
    invalidates the cache by construction.
    """
    ttl = merged_path.read_text()
    classes = sorted(set(re.findall(r":(\w+)\s+(?:a|rdf:type)\s+owl:Class", ttl)))
    if len(classes) < 2:
        return 0

    key = hashlib.sha256("\n".join(classes).encode()).hexdigest()[:16]
    cache = merged_path.parent / f"_disjoint.{key}.ttl"
    known = set(classes)

    if cache.exists():
        block = cache.read_text().strip()
        n = block.count("owl:disjointWith")
    else:
        votes: dict[tuple[str, str], int] = {}
        rounds = 0
        for group in _sibling_groups(classes):
            prompt = DISJOINT_PROMPT.format(classes="\n".join(":" + c for c in group))
            got = 0
            for _ in range(SAMPLES):
                try:
                    raw = call_model(prompt, 1200, temperature=0.0)
                except Exception:
                    continue
                if not raw.strip():
                    continue
                got += 1
                # Within one sample a pair counts once, however often it is
                # repeated, and direction is irrelevant: disjointness is
                # symmetric.
                seen = set()
                for a, b in re.findall(r"(:\w+)\s+owl:disjointWith\s+(:\w+)", raw):
                    a, b = a.lstrip(":"), b.lstrip(":")
                    if a == b or a not in known or b not in known:
                        continue
                    seen.add(tuple(sorted((a, b))))
                for pair in seen:
                    votes[pair] = votes.get(pair, 0) + 1
            rounds = max(rounds, got)
        if not rounds:
            return 0
        need = rounds // 2 + 1
        valid = sorted(p for p, v in votes.items() if v >= need)
        if not valid:
            return 0
        block = "\n".join(f":{a} owl:disjointWith :{b} ." for a, b in valid)
        cache.write_text(block + "\n")
        n = len(valid)

    ttl = re.split(r"\n# Disjointness, [^\n]*\n", ttl)[0].rstrip()
    merged_path.write_text(
        ttl + "\n\n# Disjointness, agreed by a majority of samples over the merged vocabulary\n"
        + block + "\n")
    return n


# Suffixes that name a KIND of the parent, which is exactly what a subclass
# partition already expresses. Status/State are deliberately absent: a status
# ("in routine use", "retired") is a property of a thing over time, not a kind
# of thing, and collapsing it into the class hierarchy would be wrong.
KIND_SUFFIXES = ("Type", "Kind", "Category", "Mode", "Variant")


def remove_subject(ttl: str, name: str) -> str:
    """Remove every statement whose subject is :name."""
    out, skipping = [], False
    for line in ttl.splitlines():
        if re.match(rf"^:{re.escape(name)}\b", line):
            skipping = True
        if skipping:
            if line.rstrip().endswith("."):
                skipping = False
            continue
        out.append(line)
    return "\n".join(out)


def _reconcile_ttl(merged_path: pathlib.Path) -> list[str]:
    """Collapse competing modelling patterns left behind by the merge.

    Per-document derivation is independent, so one document models a mode
    as a subclass partition (:PublishedDataset, :DraftDataset) while another
    models the same notion as an attribute class (:DatasetType). Concatenating
    fragments keeps both, and the extractor then legitimately picks whichever
    it likes. It reliably picks the attribute class, which is the one form
    disjointness cannot reach: :Dataset07 a :DatasetType says nothing the
    reasoner can contradict.

    The rule: where a parent has a subclass partition (two or more
    subclasses) AND the vocabulary also declares parent+KIND_SUFFIX, the
    attribute class is redundant with the partition and loses. Remove it,
    and remove any property whose domain or range points at it, so the only
    way left to express the distinction is the one that supports reasoning.

    None of the engine's enforce packs (generic, hierarchy, value_partition)
    catches this today; it is a merge problem, not a single-ontology problem.

    Operates on the Turtle text on disk, mirroring the file-based fragment
    merge the rest of this pipeline stage uses.
    """
    ttl = merged_path.read_text()
    classes = set(re.findall(r":(\w+)\s+(?:a|rdf:type)\s+owl:Class", ttl))
    children: dict[str, set[str]] = {}
    for m in re.finditer(r":(\w+)\s+rdfs:subClassOf\s+:(\w+)", ttl):
        children.setdefault(m.group(2), set()).add(m.group(1))

    doomed = sorted({parent + suf
                     for parent, kids in children.items() if len(kids) >= 2
                     for suf in KIND_SUFFIXES if parent + suf in classes})
    if not doomed:
        return []

    for name in doomed:
        ttl = remove_subject(ttl, name)
        # Properties anchored to the removed class go with it. A property
        # whose range no longer exists invites the extractor to mint an
        # untyped individual to fill it, which recreates the pattern that
        # was just removed.
        for prop in set(re.findall(
                rf":(\w+)\s+(?:a|rdf:type)\s+owl:(?:Object|Datatype)Property\b"
                rf"[^.]*?rdfs:(?:domain|range)\s+:{re.escape(name)}\b", ttl)):
            ttl = remove_subject(ttl, prop)
        ttl = re.sub(rf"^:{re.escape(name)}\s+owl:disjointWith[^\n]*\n?", "",
                     ttl, flags=re.M)
        ttl = re.sub(rf"^:\w+\s+owl:disjointWith\s+:{re.escape(name)}[^\n]*\n?", "",
                     ttl, flags=re.M)
    merged_path.write_text(ttl)
    return doomed


def _name_tokens(name: str) -> tuple[str, ...]:
    return tuple(sorted(t.lower() for t in re.findall(r"[A-Z][a-z0-9]*|^[a-z]+[0-9]*", name)
                        if t.lower() != "has"))


def mech_resolve(body: str, bad: list[str], onto_ttl: str, doc: str = "") -> str:
    """Resolve undeclared terms that map onto declared vocabulary mechanically.

    The three graphs that stall in LLM repair all fail on terms whose intent
    is recoverable without a model:

      A  token equality      hasConcentrationValue -> ConcentrationValue
      B  unique superset     hasContainmentLevel   -> ContainmentLevelValue
      C  has{Class} inversion  S :hasDistributionFormat O, where
         DistributionFormat is a declared CLASS: the generator manufactured a
         forward property where the vocabulary models the inverse. Rewritten
         as O a :DistributionFormat ; :hasDistribution S, which is the declared
         pattern every other document uses.

    Anything ambiguous (hasName with five plausible targets) is left alone
    for the LLM repair rounds and, failing those, the review queue.
    """
    classes = set(re.findall(r":(\w+)\s+(?:a|rdf:type)\s+owl:Class", onto_ttl))
    props = set(re.findall(r":(\w+)\s+(?:a|rdf:type)\s+owl:(?:Object|Datatype)Property", onto_ttl))
    parent = dict(re.findall(r":(\w+)\s+rdfs:subClassOf\s+:(\w+)", onto_ttl))

    by_toks: dict[tuple, list[str]] = {}
    for p in props:
        by_toks.setdefault(_name_tokens(p), []).append(p)

    renames, inversions = {}, set()
    for term in {b.split("#")[-1] for b in bad}:
        if not re.fullmatch(r"\w+", term) or term in props or term in classes:
            continue
        tt = _name_tokens(term)
        if tt in by_toks and len(by_toks[tt]) == 1:                      # rule A
            renames[term] = by_toks[tt][0]
            continue
        sups = [p for p in props if set(tt) < set(_name_tokens(p))]
        if sups:                                                          # rule B
            m = min(len(_name_tokens(p)) for p in sups)
            best = [p for p in sups if len(_name_tokens(p)) == m]
            if len(best) == 1 and m - len(tt) <= 2:
                renames[term] = best[0]
                continue
        if term[:3].lower() == "has" and term[3:] in classes:             # rule C
            inversions.add(term)

    for old, new in renames.items():
        body = re.sub(rf":{re.escape(old)}\b", f":{new}", body)
        record("renamed", doc=doc, **{"from": old, "to": new},
               how="token match against declared vocabulary")

    if inversions:
        types = {}
        subj = None
        for line in body.splitlines():
            m0 = re.match(r"^:(\w+)\b", line)
            if m0:
                subj = m0.group(1)
            mt = re.search(r"(?:^|\s)a\s+:(\w+)", line)
            if mt and subj and subj not in types:
                types[subj] = mt.group(1)

        out, extra, subj = [], [], None
        for line in body.splitlines():
            m0 = re.match(r"^:(\w+)\b", line)
            if m0:
                subj = m0.group(1)
            m = re.match(r"^(\s*):(\w+)\s+:(\w+)\s*([;.])\s*$", line)
            if m and m.group(2) in inversions and subj:
                prop, obj, term = m.group(2), m.group(3), m.group(4)
                inv, c, seen = None, types.get(subj), set()
                while c and c not in seen:                # walk up to a declared inverse
                    seen.add(c)
                    inv = next((x for x in (f"has{c}", f"Has{c}") if x in props), None)
                    if inv:
                        break
                    c = parent.get(c)
                stmt = f":{obj} a :{prop[3:]}"
                if inv:
                    stmt += f" ;\n    :{inv} :{subj}"
                extra.append(stmt + " .")
                record("inverted", doc=doc, predicate=prop, subject=subj, obj=obj,
                       how=f"has{{Class}} inversion via declared :{inv}" if inv
                           else "has{Class} inversion, no declared inverse")
                if term == ".":                           # re-terminate the statement
                    for i in range(len(out) - 1, -1, -1):
                        if out[i].strip():
                            out[i] = re.sub(r"[;,]\s*$", " .", out[i])
                            break
                continue
            out.append(line)
        body = "\n".join(out + [""] + extra)
    return body


REFINE_PROMPT = """One document contains these excerpts about "{label}":

{snippets}

Which ONE of these categories does the document assign to "{label}"?
{options}

Reply with EXACTLY one category name from the list and nothing else.
If "{label}" is not any kind of {parent} at all, reply exactly: other
If the excerpts do not say, reply exactly: unstated
"""

GATE_PROMPT = """One document contains these excerpts mentioning "{label}":

{snippets}

Question: does "{label}" name a {parent}?

The excerpts may be registry rows, change logs, or procedures ABOUT the thing;
those still count: a registry row about a dataset is describing a dataset.
Answer no only when "{label}" names a different KIND of thing, such as a
licence code, a file format, a document, or a person.

Reply exactly yes or no.
"""


def partitions_of(ttl: str) -> dict[str, list[str]]:
    """Parents whose subclasses form a disjoint partition worth asking about."""
    kids: dict[str, set[str]] = {}
    for m in re.finditer(r":(\w+)\s+rdfs:subClassOf\s+:(\w+)", ttl):
        kids.setdefault(m.group(2), set()).add(m.group(1))
    disjoint = set()
    for m in re.finditer(r":(\w+)\s+owl:disjointWith\s+:(\w+)", ttl):
        disjoint.add(frozenset(m.groups()))
    return {p: sorted(k) for p, k in kids.items()
            if len(k) >= 2 and any(frozenset((a, b)) in disjoint
                                   for a in k for b in k if a != b)}


def drop_clone_spam(body: str, threshold: int = 8) -> tuple[str, int]:
    """Remove runs of numbered clone individuals (STEM_1 ... STEM_78).

    A partially degenerate generation emits dozens of individuals differing
    only in a counter. The repetition guard misses it because every line is
    unique, and the vocabulary check passes it because every term is
    declared. The counter is the tell: no real document describes 78 things
    named identically but for an integer.
    """
    stems: dict[str, int] = {}
    for m in re.finditer(r"^:(\w+?)_\d+\b", body, re.M):
        stems[m.group(1)] = stems.get(m.group(1), 0) + 1
    doomed = {s for s, c in stems.items() if c >= threshold}
    if not doomed:
        return body, 0
    kept, skipping, dropped = [], False, 0
    for line in body.splitlines():
        m = re.match(r"^:(\w+?)_\d+\b", line)
        if m and m.group(1) in doomed:
            skipping = True
            dropped += 1
        if skipping:
            if line.rstrip().endswith("."):
                skipping = False
            continue
        kept.append(line)
    return "\n".join(kept), dropped


def refine(merged_path: pathlib.Path, instances: list[pathlib.Path],
           texts: dict[str, str]) -> int:
    """Assign partition subclasses with one closed-choice question each.

    Six rounds of instruction in the main extraction prompt failed to make
    the generator type individuals with partition subclasses; it wrote the
    safe parent class every time. So the decision is taken out of the big
    prompt entirely. For each individual typed with a bare partition parent,
    the excerpts mentioning it are shown back with the subclass names as a
    closed choice. A model that cannot follow "use the most specific class"
    inside a 16k-character extraction task answers "published or draft
    or unstated" correctly, because there is nothing else to do.

    This is the LLM-Modulo shape end to end: generation is decomposed until
    each answer is small enough to verify, and the reasoner adjudicates what
    the answers jointly imply.
    """
    onto_ttl = merged_path.read_text()
    parts = partitions_of(onto_ttl)
    if not parts:
        return 0
    all_classes = sorted(set(re.findall(r":(\w+)\s+(?:a|rdf:type)\s+owl:Class", onto_ttl)))
    (merged_path.parent / "_review.jsonl").unlink(missing_ok=True)
    LEDGER["path"] = merged_path.parent / "_decisions.jsonl"
    LEDGER["path"].unlink(missing_ok=True)

    # Verdicts persist across runs. A queued individual stays typed with the
    # parent class, so every cached replay re-found it and re-asked the model
    # the same gate question: seconds of model time per build, answering
    # something already answered. The memo is keyed by subject and asserted
    # type; reverting a typing decision changes the type and naturally
    # re-opens the question.
    memo_path = merged_path.parent / "_refine_memo.json"
    try:
        memo = json.loads(memo_path.read_text())
    except Exception:
        memo = {}
    n = 0
    for f in instances:
        doc = f.stem.replace(".data", "")
        text = texts.get(doc, "")
        g = f.read_text()
        g, spam = drop_clone_spam(g)
        if spam:
            record("spam", doc=doc, dropped=spam,
                   how="numbered clone individuals removed")
            print(f"  {YEL}spam{OFF}     {doc:14s} dropped {spam} numbered clone individuals")
        if text:
            for parent, kids in parts.items():
                seen_subjects: set[str] = set()
                for m in list(re.finditer(rf":(\w+)\s+a\s+:{parent}\b(?!\w)", g)):
                    subj = m.group(1)
                    # One subject can carry several statement blocks when a
                    # document mentions it in several rows; asking (and
                    # queueing) once per block produced duplicate ledger
                    # entries for the same decision.
                    if subj in seen_subjects:
                        continue
                    seen_subjects.add(subj)
                    if subj in all_classes:
                        # An INDIVIDUAL whose IRI is a declared class (a source
                        # document emitted a literal :Dataset individual). Retyping it
                        # would rewrite what looks like vocabulary. Humans only.
                        with open(merged_path.parent / "_review.jsonl", "a") as rq:
                            rq.write(json.dumps({
                                "doc": doc, "subject": subj,
                                "reason": "individual IRI collides with a declared class"}) + "\n")
                        print(f"  {YEL}review{OFF}   {doc:14s} :{subj} "
                              f"{DIM}IRI collides with a class; queued{OFF}")
                        n += 1
                        continue
                    lm = re.search(rf":{re.escape(subj)}\b[^.]*?rdfs:label\s+\"([^\"]+)\"",
                                   g, re.S)
                    label = lm.group(1) if lm else subj
                    lines = [l.strip() for l in text.splitlines()
                             if label.lower() in l.lower() and len(l.strip()) > len(label)]
                    if not lines:
                        continue
                    mkey = f"{doc}|{subj}|{parent}"
                    if memo.get(mkey) == "queued":
                        entry = {"doc": doc, "subject": subj, "label": label,
                                 "asserted": parent,
                                 "reason": f"not a {parent} per gate question (memoised)"}
                        with open(merged_path.parent / "_review.jsonl", "a") as rq:
                            rq.write(json.dumps(entry) + "\n")
                        record("queued", **entry)
                        print(f"  {YEL}review{OFF}   {doc:14s} :{subj} "
                              f"{DIM}not a {parent}; memoised verdict{OFF}")
                        n += 1
                        continue

                    snippets = "\n".join(f"- {l[:220]}" for l in lines[:6])
                    # The gate is its own binary question. Offered only as an
                    # extra option inside the partition question, "other" loses
                    # to pattern-matching: a licence code mentioned alongside
                    # published datasets came back "PublishedDataset". Asked
                    # alone, "is CC-BY-4.0 itself a dataset?" gets the right no.
                    try:
                        gate = call_model(GATE_PROMPT.format(
                            label=label, parent=parent, snippets=snippets),
                            16, attempts=2, temperature=0.0, timeout=60).strip().lower()
                    except Exception:
                        continue
                    # An empty or off-script reply is NO VERDICT. Routing it
                    # to the queue turned one transient null response into a
                    # permanent, memoised wrong answer about the corpus's main
                    # entity.
                    if not gate or not (gate.startswith("yes") or gate.startswith("no")):
                        continue
                    anskey, pick = "", None
                    if gate.startswith("yes"):
                        try:
                            ans = call_model(REFINE_PROMPT.format(
                                label=label, parent=parent, snippets=snippets,
                                options="\n".join(kids)), 30, attempts=2,
                                temperature=0.0, timeout=60).strip()
                        except Exception:
                            continue
                        anskey = ans.strip().strip(":").lower()
                        pick = next((k for k in kids if k.lower() == anskey), None)
                    else:
                        anskey = "other"
                    if pick:
                        g = re.sub(rf"(:{re.escape(subj)}\b\s+a\s+):{parent}\b(?!\w)",
                                   rf"\g<1>:{pick}", g)
                        n += 1
                        record("typed", doc=doc, subject=subj,
                               **{"from": parent, "to": pick},
                               how="closed-choice question over the partition")
                        print(f"  {GRN}typed{OFF}    {doc:14s} :{subj} -> :{pick}")
                    elif anskey == "other":
                        # The extractor mistyped it upstream. An automatic
                        # second guess was tried here and reintroduced the
                        # failure it was meant to fix (a licence code became
                        # :DatasetUpdateFrequencyComparison with :LicenceCode
                        # sitting in the candidate list). Wrong-type-with-
                        # confidence is worse than wrong-type-in-a-queue: the
                        # type stays as asserted and a human decides.
                        entry = {"doc": doc, "subject": subj, "label": label,
                                 "asserted": parent,
                                 "reason": f"not a {parent} per gate question"}
                        with open(merged_path.parent / "_review.jsonl", "a") as rq:
                            rq.write(json.dumps(entry) + "\n")
                        record("queued", **entry)
                        memo[f"{doc}|{subj}|{parent}"] = "queued"
                        memo_path.write_text(json.dumps(memo))
                        print(f"  {YEL}review{OFF}   {doc:14s} :{subj} "
                              f"{DIM}not a {parent}; queued for a human{OFF}")
                        n += 1
        f.write_text(g)
    return n


def split_statements(ttl: str) -> list[tuple[str, str]]:
    """(subject, statement) pairs, line-based, prefix header excluded."""
    out, subj, buf = [], None, []
    for line in ttl.splitlines():
        if not buf:
            m = re.match(r"^:(\w+)\b", line)
            if not m:
                continue
            subj = m.group(1)
        buf.append(line)
        if line.rstrip().endswith("."):
            out.append((subj, "\n".join(buf)))
            subj, buf = None, []
    return out


def prune_to_used(onto_ttl: str, data: str) -> tuple[str, dict]:
    """The vocabulary the data actually exercises, nothing else.

    Derivation over-proposes by an order of magnitude: 10 documents produced
    346 classes, most of them compositional one-offs
    (DatasetDistributionFormatConformanceAssessmentOutcome) that no
    instance ever uses. Rendering all of that buries the dozen entities the
    corpus is actually about. The FULL vocabulary stays on disk and keeps
    constraining extraction; the store the graph view and chat read carries
    only: classes with instances, their ancestors, the domain and range of
    every property the data uses, and disjointness among what remains.
    """
    # A class is anything the ontology treats as one, not only what carries an
    # explicit `a owl:Class`. The partition members are declared through
    # rdfs:subClassOf alone, and dropping them silently removed the exact
    # disjointness axiom the contradiction demo runs on.
    decl_cls = set(re.findall(r":(\w+)\s+(?:a|rdf:type)\s+owl:Class", onto_ttl))
    decl_cls |= set(re.findall(r":(\w+)\s+rdfs:subClassOf\s+:", onto_ttl))
    for a, b in re.findall(r":(\w+)\s+owl:disjointWith\s+:(\w+)", onto_ttl):
        decl_cls.update((a, b))
    decl_props = set(re.findall(r":(\w+)\s+(?:a|rdf:type)\s+owl:(?:Object|Datatype)Property", onto_ttl))
    counts: dict[str, int] = {}
    for c in re.findall(r"\ba\s+:(\w+)", data):
        counts[c] = counts.get(c, 0) + 1
    # A class instantiated once is a description of one node, not a category.
    # The graph view draws the CLASS diagram, so every singleton is a box that
    # says nothing. Recurring classes are what the corpus is actually about;
    # singletons keep their individuals in the store but not their box.
    used_cls = {c for c, n in counts.items() if n >= 2 and c in decl_cls}
    used_props = decl_props & set(re.findall(r":(\w+)", data))

    stmts = split_statements(onto_ttl)
    by_subj = {}
    for s, t in stmts:
        by_subj.setdefault(s, t)

    keep = set(used_cls)
    for pr in used_props:                      # domain/range classes stay: edges render
        for dr in re.findall(r"rdfs:(?:domain|range)\s+:(\w+)", by_subj.get(pr, "")):
            if dr in decl_cls:
                keep.add(dr)

    parents: dict[str, set[str]] = {}
    for m in re.finditer(r":(\w+)\s+rdfs:subClassOf\s+:(\w+)", onto_ttl):
        parents.setdefault(m.group(1), set()).add(m.group(2))
    frontier = list(keep)
    while frontier:                            # ancestors close the hierarchy
        for p in parents.get(frontier.pop(), ()):
            if p in decl_cls and p not in keep:
                keep.add(p)
                frontier.append(p)

    # View collapse. Two shapes of noise survive the instance-count filter:
    # specialisation chains (DatasetDistributionFormatConformance...
    # extending DatasetDistributionFormat token by token) and families of near
    # duplicate siblings (fourteen ChangeLogEntryChange* classes). A box per
    # variant hides the dozen entities the corpus is about. Chains collapse
    # into their shortest kept prefix; families keep their two most
    # instantiated members. Instance data is never touched, and partition
    # members are exempt: they are what the reasoner argues about.
    # Exempt only true partitions: disjoint SIBLINGS under a common parent.
    # The disjointness pass asserts pairs across the whole vocabulary, so
    # exempting every pair member exempts nearly everything.
    all_parents: dict[str, set[str]] = {}
    for m in re.finditer(r":(\w+)\s+rdfs:subClassOf\s+:(\w+)", onto_ttl):
        all_parents.setdefault(m.group(1), set()).add(m.group(2))
    exempt = {c for a, b in re.findall(r":(\w+)\s+owl:disjointWith\s+:(\w+)", onto_ttl)
              if all_parents.get(a, set()) & all_parents.get(b, set())
              for c in (a, b)}
    ctok = {c: tuple(re.findall(r"[A-Z][a-z0-9]*", c)) for c in keep if c in decl_cls}
    hide = set()
    for a, ta in ctok.items():
        for b, tb in ctok.items():
            if a != b and len(tb) > len(ta) and tb[:len(ta)] == ta and b not in exempt:
                hide.add(b)
    families: dict[tuple, list[str]] = {}
    for c, tc in ctok.items():
        if c not in hide and len(tc) >= 3:
            families.setdefault(tc[:3], []).append(c)
    for group in families.values():
        if len(group) >= 5:
            group.sort(key=lambda c: -counts.get(c, 0))
            hide.update(c for c in group[2:] if c not in exempt)
    keep -= hide

    # Case twins and noun duplicates of the same property (hasOwner/HasOwner,
    # Title/hasTitle) render as distinct edges saying the same thing. One
    # survives in the view; the data keeps using whichever it used.
    used_props -= {p for p in used_props
                   if p.startswith("Has") and "has" + p[3:] in used_props}
    used_props -= {p for p in used_props
                   if not p.startswith(("has", "Has"))
                   and ("has" + p in used_props or "Has" + p in used_props
                        or "has" + p[0].upper() + p[1:] in used_props)}

    kept = []
    for s, t in stmts:
        if re.match(r"^:\w+\s+owl:disjointWith", t):
            other = re.search(r"owl:disjointWith\s+:(\w+)", t)
            if s in keep and other and other.group(1) in keep:
                kept.append(t)
        elif s in keep or s in used_props:
            kept.append(t)

    stats = {"classes": (len(decl_cls), len(keep & decl_cls)),
             "properties": (len(decl_props), len(used_props))}
    return PREFIXES + "\n" + "\n\n".join(kept), stats


def sources_of(instances: list[pathlib.Path]) -> dict[tuple[str, str], set[str]]:
    """Which document asserted that a given individual has a given type.

    Provenance is what separates the two things a disjointness violation can
    mean. If one document types Dataset07 as both a dataset and an update
    frequency, that is a broken extraction. If two documents disagree about which
    of two mutually exclusive classes it belongs to, that is the corpus
    contradicting itself, and it is the only kind worth showing anyone.

    Reported together they are indistinguishable, and the noisy kind buries
    the interesting kind: one over-typed registry produced 603 violations and
    not one of them was a disagreement.
    """
    out: dict[tuple[str, str], set[str]] = {}
    for f in instances:
        doc = f.stem.replace(".data", "")
        text = f.read_text()
        for m in re.finditer(r":(\w+)\s+(?:a|rdf:type)\s+([^;.]+)", text):
            for t in re.findall(r":(\w+)", m.group(2)):
                out.setdefault((m.group(1), t), set()).add(doc)
    return out


def vocab_summary(ttl: str) -> str:
    """Vocabulary for the populate prompt, WITH definitions.

    A bare list of names invites the model to invent near-misses: given
    :Dataset it will happily write :DatasetType or :DatasetArchived
    because they sound plausible. Including each term's label and comment makes
    the existing vocabulary concrete enough to reuse instead of extend, which
    is what the closed-world check is measuring.
    """
    def described(kind: str) -> list[str]:
        out = []
        for name in sorted(set(re.findall(rf":(\w+)\s+a\s+owl:{kind}", ttl))):
            m = re.search(rf":{name}\s+a\s+owl:{kind}\b.*?rdfs:label\s+\"([^\"]+)\"", ttl, re.S)
            out.append(f"  :{name}" + (f"  ({m.group(1)})" if m else ""))
        return out

    return ("CLASSES (use these exact names, do not invent variants):\n"
            + "\n".join(described("Class"))
            + "\n\nOBJECT PROPERTIES:\n" + "\n".join(described("ObjectProperty"))
            + "\n\nDATATYPE PROPERTIES:\n" + "\n".join(described("DatatypeProperty")))


REPAIR_PROMPT = """This Turtle uses terms that do not exist in the ontology. Every
term must come from the vocabulary below.

INVALID TERMS USED: {bad}

VOCABULARY:
{vocab}

Rewrite the Turtle so that every class and property is one of the vocabulary
terms above. Where an invalid term has a clear equivalent, map it. Where it does
not, DROP those statements rather than inventing a substitute. Keep everything
that was already valid unchanged.

Return ONLY Turtle, no prose and no fences.

TURTLE TO REPAIR:
{ttl}
"""


def repair(ttl: str, bad: list[str], vocab: str) -> str | None:
    """One repair attempt against the named invalid terms.

    A closed-world check that only reports is half a control: it tells you the
    output is unusable without making it usable. Naming the specific offending
    terms and asking for a rewrite is cheap, and it converts a rejected graph
    into an accepted one without a human in the loop for the routine cases.
    """
    names = sorted({t.split("#")[-1] for t in bad})[:25]
    try:
        raw = call_model(REPAIR_PROMPT.format(bad=", ".join(names), vocab=vocab, ttl=ttl[:9000]), 3000)
    except Exception:
        return None
    out = normalise(raw)
    return out if len(out) > 200 else None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--corpus", default=str(ROOT / "demo" / "corpus" / "dcat-us"))
    ap.add_argument("--workers", type=int, default=2,
                    help="parallel model calls; local servers overload above 2")
    ap.add_argument("--cached", action="store_true")
    ap.add_argument("--reuse-derive", action="store_true",
                    help="re-merge existing vocab fragments instead of re-deriving")
    ap.add_argument("--no-tokenise", action="store_true")
    args = ap.parse_args()

    corpus = pathlib.Path(args.corpus)
    out = ROOT / "demo" / "derived"
    out.mkdir(exist_ok=True)
    # A real corpus is not one file type. Task 7's DCAT-US corpus is
    # markdown, JSON Schema and SHACL Turtle side by side (a schema-only
    # profile, its recovered RDF binding, and the W3C text that defines what
    # binding it would need); a glob for .docx-or-.md alone silently drops
    # four of the seven documents and, with them, the corpus's central
    # disagreement. MANIFEST.json is provenance metadata, not a document.
    docs = sorted(p for p in corpus.iterdir()
                  if p.is_file() and p.name != "MANIFEST.json"
                  and p.suffix in (".docx", ".md", ".json", ".ttl"))
    if not docs:
        sys.exit(f"no documents in {corpus}")

    t0 = time.time()
    print(f"\n{BOLD}STAGE 1  READ{OFF}  {len(docs)} documents from {corpus.name}/")

    # ---- Stage 2: tokenise -------------------------------------------------
    os.environ.setdefault("ONTO_VAULT_PATH", str(out / "_vault.json"))
    # Reading and tokenising dominate the wall clock of a cached build:
    # pandoc is a subprocess per document and the Presidio detector loads a
    # spaCy model on import, which costs seconds before any work happens.
    # Both are pure functions of the file, so cache by mtime and only build
    # the detector when at least one document actually changed.
    doc_cache_path = out / "_docs_cache.json"
    try:
        doc_cache = json.loads(doc_cache_path.read_text())
    except Exception:
        doc_cache = {}
    tokeniser = None
    prepared, total_tokens = [], 0
    dirty = False
    for d in docs:
        m = re.match(r"([A-Z]+-[0-9][0-9A-Za-z\-]*?)(?=-[a-z])", d.stem)
        docid = m.group(1) if m else d.stem
        key, mtime = str(d), d.stat().st_mtime
        hit = doc_cache.get(key)
        if not args.no_tokenise and hit and hit.get("mtime") == mtime:
            total_tokens += hit["count"]
            prepared.append((d, hit["text"], docid))
            continue
        text = docx_text(d)
        if args.no_tokenise:
            prepared.append((d, text, docid))
            continue
        if tokeniser is None:
            tokeniser = build_tokeniser()
        tok, n = tokeniser.tokenise(text)
        total_tokens += n
        prepared.append((d, tok, docid))
        doc_cache[key] = {"mtime": mtime, "text": tok, "count": n}
        dirty = True
    if tokeniser is not None:
        tokeniser.save()
    if dirty:
        doc_cache_path.write_text(json.dumps(doc_cache))
    if args.no_tokenise:
        print(f"\n{BOLD}STAGE 2  TOKENISE{OFF}  {YEL}skipped (--no-tokenise){OFF}")
    else:
        print(f"\n{BOLD}STAGE 2  TOKENISE{OFF}  {total_tokens} values replaced with stable tokens")
        desc = tokeniser.description if tokeniser is not None else \
            "cached detection results (documents unchanged)"
        print(f"  {DIM}{desc}. No model sees a real value from here on.{OFF}")

    # ---- Stage 3: derive ---------------------------------------------------
    merged_path = out / "_ontology.ttl"
    if args.cached and merged_path.exists():
        print(f"\n{BOLD}STAGE 3  DERIVE{OFF}  {DIM}cached{OFF}")
    elif args.reuse_derive and list(out.glob("*.vocab.ttl")):
        # Derivation is the expensive, stable half. Re-merging existing
        # fragments exercises the merge and everything after it without
        # paying for the model calls again.
        # Persist fragments in mended form: the align tools parse these
        # files as written, and a missing prefix makes a fragment invisible
        # to schema resolution even though the merge recovered it.
        for fp in sorted(out.glob("*.vocab.ttl")):
            fixed = rehead(fp.read_text())
            if is_valid_turtle(fixed)[0]:
                fp.write_text(fixed)
        fragments = [f.read_text() for f in sorted(out.glob("*.vocab.ttl"))]
        print(f"\n{BOLD}STAGE 3  DERIVE{OFF}  reusing {len(fragments)} existing fragments")
        good, dropped = [], []
        for frag in fragments:
            ok, why = is_valid_turtle(rehead(frag))
            if ok:
                good.append(mend(body_of(frag)))
            else:
                dropped.append(why)
        if dropped:
            print(f"  {YEL}{len(dropped)} fragment(s) dropped as unparseable{OFF}")
            for why in dropped[:4]:
                print(f"    {DIM}{why}{OFF}")
        merged_path.write_text(PREFIXES + "\n" + STRUCTURAL + "\n" + "\n\n".join(b for b in good if b))
        ok, why = is_valid_turtle(merged_path.read_text())
        print(f"  merged {len(good)} fragments, parses: {'yes' if ok else 'NO: ' + why}")
    else:
        print(f"\n{BOLD}STAGE 3  DERIVE{OFF}  propose the vocabulary the documents imply")
        fragments = []
        with cf.ThreadPoolExecutor(max_workers=args.workers) as pool:
            for docid, ttl, err in pool.map(derive_one, prepared):
                if err:
                    print(f"  {RED}fail{OFF} {docid:14s} {err}")
                else:
                    mended_ttl = rehead(ttl)
                    (out / f"{docid}.vocab.ttl").write_text(
                        mended_ttl if is_valid_turtle(mended_ttl)[0] else ttl)
                    fragments.append(ttl)
                    n = len(re.findall(r"a\s+owl:Class", ttl))
                    print(f"  {GRN}ok{OFF}   {docid:14s} {n} classes proposed")
        # Merge: prefixes once, bodies concatenated. Duplicate declarations are
        # harmless in RDF; the store deduplicates identical triples.
        # Validate each fragment alone. One malformed fragment otherwise
        # truncates the merged file at its first bad statement, discarding
        # every valid fragment that follows it.
        good, dropped = [], []
        for frag in fragments:
            ok, why = is_valid_turtle(rehead(frag))
            if ok:
                good.append(mend(body_of(frag)))
            else:
                dropped.append(why)
        if dropped:
            print(f"  {YEL}{len(dropped)} fragment(s) dropped as unparseable:{OFF}")
            for why in dropped[:3]:
                print(f"    {DIM}{why}{OFF}")
        merged_path.write_text(PREFIXES + "\n" + STRUCTURAL + "\n" + "\n\n".join(b for b in good if b))
        ok, why = is_valid_turtle(merged_path.read_text())
        if not ok:
            print(f"  {RED}merged ontology still does not parse: {why}{OFF}")

    doomed = _reconcile_ttl(merged_path)
    if doomed:
        print(f"  {BOLD}reconciled:{OFF} removed {len(doomed)} attribute class(es) that "
              f"compete with a subclass partition: {', '.join(':' + d for d in doomed)}")

    check = engine(["clear", f"load {merged_path}", "stats"])
    try:
        st = json.loads(check.split("\n")[-1])["result"]
        print(f"  {BOLD}derived ontology: {st['classes']} classes, {st['properties']} properties, "
              f"{st['triples']} triples{OFF}")
    except Exception:
        print(f"  {YEL}derived ontology did not parse cleanly{OFF}")
        print(f"  {DIM}{check[-200:]}{OFF}")

    n_disjoint = add_disjointness(merged_path)
    print(f"  {BOLD}disjointness: {n_disjoint} mutually exclusive pairs asserted{OFF}"
          if n_disjoint else f"  {YEL}disjointness: none asserted (nothing for the reasoner to catch){OFF}")

    vocab = vocab_summary(merged_path.read_text())

    # ---- Stage 4: populate -------------------------------------------------
    print(f"\n{BOLD}STAGE 4  POPULATE{OFF}  extract facts, constrained to that vocabulary")
    instances = []
    if args.cached:
        instances = sorted(out.glob("*.data.ttl"))
        if not instances:
            print(f"  {YEL}no cached graphs to replay. Tick 'Re-run extraction' to build them.{OFF}")
            sys.exit(2)
        print(f"  {DIM}cached, {len(instances)} graphs{OFF}")
    else:
        jobs = [(d, t, i, vocab) for (d, t, i) in prepared]
        with cf.ThreadPoolExecutor(max_workers=args.workers) as pool:
            for docid, ttl, err in pool.map(populate_one, jobs):
                if err:
                    print(f"  {RED}fail{OFF} {docid:14s} {err}")
                else:
                    f = out / f"{docid}.data.ttl"
                    f.write_text(ttl)
                    instances.append(f)
                    print(f"  {GRN}ok{OFF}   {docid:14s} {f.name}")

    # ---- Stage 4b: refine --------------------------------------------------
    print(f"\n{BOLD}STAGE 4b REFINE{OFF}  partition typing as closed-choice questions")
    texts = {docid: tokenised for (_p, tokenised, docid) in prepared}
    refined = refine(merged_path, instances, texts)
    if not refined:
        print(f"  {DIM}nothing to refine{OFF}")

    # ---- Stage 5: verify ---------------------------------------------------
    print(f"\n{BOLD}STAGE 5  VERIFY{OFF}  closed-world check against the DERIVED ontology")
    ok = bad = 0
    try:
        mcp("initialize", {"protocolVersion": "2025-03-26", "capabilities": {},
                           "clientInfo": {"name": "otd", "version": "1"}})
        mcp("tools/call", {"name": "onto_load", "arguments":
            {"path": str(merged_path), "name": "derived", "force_recompile": True}})
        declared = set(re.findall(r":(\w+)\s+a\s+owl:", merged_path.read_text()))

        def check(ttl: str) -> dict:
            return json.loads(tool_text(mcp("tools/call", {
                "name": "onto_vocab_check",
                "arguments": {"data": ttl, "inline": True}})) or "{}")

        for f in instances:
            current = f.read_text()
            d = check(current)
            if d.get("conforms"):
                ok += 1
                print(f"  {GRN}conforms{OFF}  {f.stem}")
                continue
            if "error" in d:
                # A graph that does not parse is a different and worse failure
                # than one that uses the wrong terms. Reporting both as
                # "review" hid a broken graph behind an empty term list.
                bad += 1
                print(f"  {RED}unparsed{OFF}  {f.stem}  {DIM}{d['error'][:90]}{OFF}")
                continue

            # Mechanical resolution first: token-equality renames, unique
            # superset renames, and has{Class} inversions need no model and
            # cannot hallucinate. The LLM repair rounds then only see what is
            # genuinely ambiguous.
            h = d.get("hallucinated_terms", [])
            start = len(h)
            mech = mech_resolve(current, h, merged_path.read_text(),
                                doc=f.stem.replace(".data", ""))
            if mech != current and is_valid_turtle(mech)[0]:
                d = check(mech)
                if "error" not in d:
                    current = mech
                    f.write_text(current)
                    h = d.get("hallucinated_terms", [])
                    if d.get("conforms"):
                        ok += 1
                        print(f"  {GRN}repaired{OFF}  {f.stem}  "
                              f"{DIM}({start} terms resolved mechanically){OFF}")
                        continue
            for _ in range(REPAIR_ROUNDS):
                fixed = repair(current, h, vocab)
                if not fixed:
                    break
                fixed = snap_to_vocab(fixed, declared)
                d = check(fixed)
                if d.get("conforms"):
                    f.write_text(fixed)
                    current, h = fixed, []
                    break
                nh = d.get("hallucinated_terms", h)
                if "error" in d or len(nh) >= len(h):
                    break               # not converging, stop paying for it
                current, h = fixed, nh
            if not h:
                ok += 1
                print(f"  {GRN}repaired{OFF}  {f.stem}  {DIM}({start} invalid terms resolved){OFF}")
            else:
                f.write_text(current)   # keep the partial improvement
                bad += 1
                print(f"  {YEL}review  {OFF}  {f.stem}  {DIM}{start} -> {len(h)}{OFF}  "
                      f"{[t.split('#')[-1] for t in h][:5]}")
    except Exception as e:
        print(f"  {YEL}verification unavailable: {e}{OFF}")

    # ---- Stage 6: scan -----------------------------------------------------
    print(f"\n{BOLD}STAGE 6  SCAN{OFF}  reason over the derived ontology and its instances")
    loads = ["clear", f"load {merged_path}"] + [f"load {f}" for f in instances]
    res = engine(loads + ["stats"])
    try:
        st = json.loads(res.split("\n")[-1])["result"]
        print(f"  merged: {st['triples']} triples, {st['classes']} classes, {st['individuals']} individuals")
    except Exception:
        pass

    q = ("PREFIX owl: <http://www.w3.org/2002/07/owl#> "
         "PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#> "
         "SELECT DISTINCT ?subject ?a ?b WHERE { ?subject a ?a, ?b . "
         "FILTER(STR(?a) < STR(?b)) ?a rdfs:subClassOf* ?da . ?b rdfs:subClassOf* ?db . "
         "{ ?da owl:disjointWith ?db } UNION { ?db owl:disjointWith ?da } }")
    res = engine(loads + [f"query {json.dumps(q)}"])
    try:
        rows = json.loads(res.split("\n")[-1])["result"]["results"]
    except Exception:
        rows = []
    src = sources_of(instances)
    disagreements, defects = [], []
    for r in rows:
        v = {k: x.split("#")[-1].rstrip(">") for k, x in r.items()}
        da = src.get((v["subject"], v["a"]), set())
        db = src.get((v["subject"], v["b"]), set())
        # A disagreement needs a document that asserts one side and does not
        # assert the other. Both types coming from the same single document is
        # one extractor being wrong, not two sources in conflict.
        (disagreements if (da - db) and (db - da) else defects).append((v, da, db))

    for v, da, db in disagreements:
        print(f"  {RED}CONTRADICTION{OFF} {v['subject']} is {v['a']} per "
              f"{','.join(sorted(da - db))} but {v['b']} per {','.join(sorted(db - da))}")
    if defects:
        by_doc: dict[str, int] = {}
        for _, da, db in defects:
            for d in da | db:
                by_doc[d] = by_doc.get(d, 0) + 1
        worst = sorted(by_doc.items(), key=lambda x: -x[1])[:3]
        print(f"  {YEL}{len(defects)} violations within a single document{OFF} "
              f"{DIM}(over-typing, not disagreement: "
              f"{', '.join(f'{d} {n}' for d, n in worst)}){OFF}")
    if not rows:
        print(f"  {DIM}no disjointness violations{OFF}")

    # ---- Stage 7: load -----------------------------------------------------
    # Everything above ran against throwaway CLI stores. The Studio graph
    # view and the chat's GraphRAG read the LIVE engine, so the session ends
    # by loading the merged ontology plus every instance graph there. Without
    # this the app renders vocabulary with zero individuals, which looks like
    # extraction never happened.
    print(f"\n{BOLD}STAGE 7  LOAD{OFF}  push the result into the live engine "
          f"for the graph view and chat")
    store = merged_path.parent / "_store.ttl"
    data = "\n\n".join(body_of(f.read_text()) for f in instances)
    pruned, pstats = prune_to_used(merged_path.read_text(), data)
    c0, c1 = pstats["classes"]
    p0, p1 = pstats["properties"]
    print(f"  pruned to the vocabulary the data uses: "
          f"{c0} -> {c1} classes, {p0} -> {p1} properties "
          f"{DIM}(full vocabulary stays on disk for extraction){OFF}")
    # Typing claims with provenance, in the shape GraphRAG retrieves: the
    # chat's own suggested question is "which documents disagree", and
    # without these the store knows THAT a conflict exists but not who says
    # what. One claim node per specific typing per document.
    claims, ci = [], 0
    for f in instances:
        doc = f.stem.replace(".data", "")
        for m in re.finditer(r":(\w+)\s+a\s+:(\w+)", f.read_text()):
            subj, cls = m.groups()
            if cls in ("Document", "Section"):
                continue
            ci += 1
            claims.append(
                f":CLAIM_{ci} a :Claim ;\n"
                f"    :claimText \"{doc} types {subj} as {cls}\" ;\n"
                f"    :aboutEntity :{subj} ;\n"
                f"    :statedIn :DOC_{doc.replace('-', '_')} .\n"
                f":{subj} <http://www.w3.org/ns/prov#wasDerivedFrom> :DOC_{doc.replace('-', '_')} .\n"
                # graphrag.ts's claim query resolves a document-shaped statedIn
                # target via `?target dcus:docId ?doc`, not its label. Without
                # this triple every claim in the live store retrieves with
                # doc == 'unattributed' and the chat cannot cite anything.
                f":DOC_{doc.replace('-', '_')} rdfs:label \"{doc}\" ; :docId \"{doc}\" .")
    store.write_text(pruned + "\n" + data + "\n" + "\n".join(claims))
    store_ok, why = is_valid_turtle(store.read_text())
    if not store_ok:
        # A parse error truncates the load at the bad line and the engine
        # reports the partial store without complaint: the graph view then
        # quietly shows a fraction of the data.
        print(f"  {YEL}store has a syntax defect; the view may be partial: {why}{OFF}")
    try:
        mcp("tools/call", {"name": "onto_load", "arguments":
            {"path": str(store), "name": "derived", "force_recompile": True}})
        st = json.loads(tool_text(mcp("tools/call",
                                      {"name": "onto_stats", "arguments": {}})) or "{}")
        print(f"  live store: {st.get('triples', '?')} triples, "
              f"{st.get('classes', '?')} classes, {st.get('individuals', '?')} individuals")
    except Exception as e:
        print(f"  {YEL}live engine not reachable ({e}); graph view will be stale{OFF}")

    print(f"\n{BOLD}{len(docs)} documents in {time.time()-t0:.0f}s. "
          f"{ok} conformed, {bad} for review, "
          f"{len(disagreements)} contradictions across documents.{OFF}")
    if not args.no_tokenise:
        print(f"{DIM}Every value the model saw was a token. Detokenisation requires the "
              f"vault and happens only at render time.{OFF}\n")


if __name__ == "__main__":
    try:
        main()
    except SystemExit:
        raise
    except Exception:
        # The Studio pane only shows "finished with errors" plus whatever was
        # printed. A crash with the traceback on stdout is diagnosable from
        # the pane; a bare non-zero exit is not.
        import traceback
        print("PIPELINE CRASH:")
        traceback.print_exc(file=sys.stdout)
        sys.exit(1)
