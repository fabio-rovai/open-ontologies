"""Extract plain document text for the baseline retriever.

The comparison in the Studio puts classic chunk retrieval beside graph
retrieval. For that comparison to mean anything, the baseline must read the
SAME documents from the SAME source, with no help from the ontology. Feeding it
anything derived would rig the result and any competent reviewer would say so.

So this reads the .docx files directly and writes plain text. The baseline
chunks that text and retrieves over it; the graph path never touches this file.

Output: demo/derived/_corpus_text.json
        { "DOC-202": {"title": ..., "text": ...}, ... }
"""

from __future__ import annotations

import json
import pathlib
import re
import zipfile

CORPUS = pathlib.Path("demo/corpus/dcat-us")
OUT = pathlib.Path("demo/derived/_corpus_text.json")


def docx_text(path: pathlib.Path) -> str:
    """Paragraph-preserving text extraction.

    Paragraph boundaries matter: chunking on a single flattened line would
    produce chunks that straddle unrelated sections, which would make the
    baseline look worse than a real one does.
    """
    xml = zipfile.ZipFile(path).read("word/document.xml").decode("utf8", "ignore")
    xml = re.sub(r"</w:p>", "\n", xml)
    text = re.sub(r"<[^>]+>", "", xml)
    text = text.replace("&amp;", "&").replace("&lt;", "<").replace("&gt;", ">")
    lines = [re.sub(r"[ \t]+", " ", ln).strip() for ln in text.split("\n")]
    return "\n".join(ln for ln in lines if ln)


def plain_text(path: pathlib.Path) -> str:
    """Non-.docx corpus members are already text; read them as-is.

    Task 7's DCAT-US corpus is markdown, JSON Schema and SHACL Turtle, not
    .docx, so the .docx-only glob below previously matched nothing at all
    and the baseline had zero documents to retrieve over.
    """
    if path.suffix == ".docx":
        return docx_text(path)
    return path.read_text(encoding="utf-8", errors="ignore")


def main() -> None:
    docs: dict[str, dict[str, str]] = {}
    members = sorted(CORPUS.glob("*.docx")) or sorted(
        p for p in CORPUS.iterdir()
        if p.is_file() and p.name != "MANIFEST.json"
        and p.suffix in (".md", ".json", ".ttl"))
    for path in members:
        text = plain_text(path)
        m = re.search(r"Doc Id ([A-Z]+-\d+)", text.replace("\n", " "))
        doc_id = m.group(1) if m else path.stem
        title = ""
        t = re.search(r"Title (.+?)(?: Owner| Classification|$)", text.replace("\n", " "))
        if t:
            title = t.group(1).strip()
        docs[doc_id] = {"title": title, "text": text}

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(docs, indent=1))
    chars = sum(len(d["text"]) for d in docs.values())
    print(f"{len(docs)} documents, {chars:,} characters -> {OUT}")
    for k, v in docs.items():
        print(f"  {k:10} {len(v['text']):6,} chars  {v['title'][:50]}")


if __name__ == "__main__":
    main()
