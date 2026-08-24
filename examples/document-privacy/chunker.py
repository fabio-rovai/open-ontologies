#!/usr/bin/env python3
"""
Semantic chunking with context and a rejoin scheme.

Three decisions, all of which matter, and none of which the usual splitters do:

1. SEMANTIC BOUNDARIES, NOT FIXED WIDTH.
   Chunks are whole paragraphs inside whole sections. Small paragraphs are
   merged upward toward a target size rather than emitted alone. The point is
   not tidiness: fixed-width splitting explodes the number of chunks, and the
   cost of a pipeline like this is dominated by how many times a model gets
   called. Fewer, more meaningful chunks is both cheaper and better.

2. EVERY CHUNK CARRIES ITS CONTEXT.
   A chunk lifted out of a fifty-page report is uninterpretable on its own.
   Each record stores a `context` sentence saying which document it came from,
   what that document is about, which section it sits in, and where in that
   section it falls. This is what lets a model reason about a fragment without
   re-reading the source.

3. IDS ENCODE STRUCTURE SO CHUNKS CAN BE REJOINED.
   Ids look like DOC-042.S3.P2, and every record carries parent, prev and next.
   Retrieval returns a fragment; answering often needs the whole section. With
   a join scheme the consumer can ask for the parent or walk the siblings
   instead of hallucinating the surrounding text or over-retrieving everything.

Emitted as JSON so the records can be stored, indexed and joined by anything.
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass, asdict
from typing import Iterator

# Target chunk size in characters. Paragraphs below MIN are merged forward;
# paragraphs above MAX are split at sentence boundaries.
TARGET = 1200
MIN = 350
MAX = 2400

HEADING = re.compile(
    r"^\s*(?:#{1,6}\s+(?P<md>.+?)|(?P<num>\d+(?:\.\d+)*)\.?\s+(?P<numtitle>[A-Z].{2,80}))\s*$"
)
SENTENCE = re.compile(r"(?<=[.!?])\s+(?=[A-Z(])")


@dataclass
class Chunk:
    id: str
    doc_id: str
    section: str
    section_title: str
    text: str
    context: str
    parent: str | None
    prev: str | None
    next: str | None

    def to_json(self) -> dict:
        return asdict(self)


def _split_long(text: str) -> list[str]:
    """Split an oversized paragraph at sentence boundaries, not mid-sentence."""
    if len(text) <= MAX:
        return [text]
    out, cur = [], ""
    for sentence in SENTENCE.split(text):
        if cur and len(cur) + len(sentence) > TARGET:
            out.append(cur.strip())
            cur = sentence
        else:
            cur = f"{cur} {sentence}".strip()
    if cur.strip():
        out.append(cur.strip())
    return out


def _sections(text: str) -> Iterator[tuple[str, str, list[str]]]:
    """Yield (section_number, section_title, paragraphs)."""
    lines = text.splitlines()
    num, title, buf = "0", "Preamble", []
    for line in lines:
        m = HEADING.match(line)
        if m:
            if buf:
                yield num, title, _paragraphs("\n".join(buf))
                buf = []
            if m.group("md"):
                heading = m.group("md").strip()
                lead = re.match(r"^(\d+(?:\.\d+)*)\.?\s+(.*)$", heading)
                num, title = (lead.group(1), lead.group(2)) if lead else (num, heading)
            else:
                num, title = m.group("num"), m.group("numtitle").strip()
        else:
            buf.append(line)
    if buf:
        yield num, title, _paragraphs("\n".join(buf))


def _paragraphs(block: str) -> list[str]:
    """Paragraphs, with short ones merged forward toward the target size."""
    raw = [p.strip() for p in re.split(r"\n\s*\n", block) if p.strip()]
    merged: list[str] = []
    for p in raw:
        if merged and len(merged[-1]) < MIN:
            merged[-1] = f"{merged[-1]}\n\n{p}"
        else:
            merged.append(p)
    out: list[str] = []
    for p in merged:
        out.extend(_split_long(p))
    return [p for p in out if len(p.strip()) > 40]


# Front-matter noise that says nothing about what a document is ABOUT.
_BOILERPLATE = re.compile(
    r"^\s*(?:\||#|\*\*Document control|SYNTHETIC|\*\*SYNTHETIC|-{3,}|\d+\.\s*(?:Purpose|Scope)\b)",
    re.I,
)


def _summarise(text: str, limit: int = 160) -> str:
    """A one-line gist of the document, used in every chunk's context.

    Skips document-control tables, headings and demo disclaimers: they are
    present in every document and say nothing about what this one covers, so
    including them would make every chunk's context identical and useless.
    """
    lines = [l.strip() for l in text.splitlines()]
    prose = [l for l in lines if l and not _BOILERPLATE.match(l) and len(l) > 40]
    body = re.sub(r"\s+", " ", " ".join(prose[:4])).strip()
    body = re.sub(r"^\d+(?:\.\d+)*\s+", "", body)
    gist = " ".join(SENTENCE.split(body)[:2]).strip()
    return (gist[: limit - 1] + "…") if len(gist) > limit else (gist or "an internal controlled document")


def chunk_document(text: str, doc_id: str, title: str | None = None) -> list[Chunk]:
    """Chunk one document into context-carrying, rejoinable records."""
    doc_title = title or (re.search(r"^#\s+(.+)$", text, flags=re.M).group(1)
                          if re.search(r"^#\s+(.+)$", text, flags=re.M) else doc_id)
    gist = _summarise(text)

    chunks: list[Chunk] = []
    for num, sec_title, paras in _sections(text):
        total = len(paras)
        for i, para in enumerate(paras, start=1):
            cid = f"{doc_id}.S{num}.P{i}"
            context = (
                f"From {doc_id} ({doc_title}), a document about {gist} "
                f"Section {num}, {sec_title}. Paragraph {i} of {total} in this section."
            )
            chunks.append(Chunk(
                id=cid, doc_id=doc_id, section=num, section_title=sec_title,
                text=para, context=context, parent=f"{doc_id}.S{num}",
                prev=None, next=None,
            ))

    # Link siblings after the fact, so prev/next span section boundaries in
    # reading order rather than stopping at each heading.
    for i, c in enumerate(chunks):
        c.prev = chunks[i - 1].id if i > 0 else None
        c.next = chunks[i + 1].id if i < len(chunks) - 1 else None
    return chunks


def pack(chunks: list[Chunk], budget: int = 9000) -> list[list[Chunk]]:
    """Group chunks into as few model calls as possible.

    This is the decision that controls cost. One call per chunk is what makes
    these pipelines absurdly expensive on long documents; packing to a context
    budget, while keeping each chunk's context header attached, gets the same
    information across in a fraction of the calls.
    """
    groups: list[list[Chunk]] = []
    cur: list[Chunk] = []
    size = 0
    for c in chunks:
        cost = len(c.text) + len(c.context) + 40
        if cur and size + cost > budget:
            groups.append(cur)
            cur, size = [], 0
        cur.append(c)
        size += cost
    if cur:
        groups.append(cur)
    return groups


def render(group: list[Chunk]) -> str:
    """Render a packed group for a prompt, each chunk labelled and contextualised."""
    parts = []
    for c in group:
        parts.append(f"[{c.id}]\n({c.context})\n{c.text}")
    return "\n\n".join(parts)


def to_jsonl(chunks: list[Chunk]) -> str:
    return "\n".join(json.dumps(c.to_json(), ensure_ascii=False) for c in chunks)


if __name__ == "__main__":
    import pathlib
    import sys

    if len(sys.argv) < 2:
        sys.exit("usage: chunker.py <file.md> [--json]")
    path = pathlib.Path(sys.argv[1])
    doc_id = re.match(r"([A-Z]+-[0-9][0-9A-Za-z\-]*?)(?=-[a-z])", path.stem)
    cs = chunk_document(path.read_text(), doc_id.group(1) if doc_id else path.stem)
    if "--json" in sys.argv:
        print(to_jsonl(cs))
    else:
        groups = pack(cs)
        print(f"{len(cs)} chunks in {len(groups)} model calls "
              f"(a per-chunk pipeline would make {len(cs)})")
        for c in cs[:3]:
            print(f"\n[{c.id}] parent={c.parent} prev={c.prev} next={c.next}")
            print(f"  context: {c.context}")
            print(f"  text:    {c.text[:110]}...")
