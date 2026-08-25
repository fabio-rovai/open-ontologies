# Document privacy and chunking seams

Two standalone modules for document-ingestion pipelines that feed an
ontology or knowledge graph. No dependency on the engine; plain Python.

## tokenisation.py: detection and custody as separate, swappable seams

Sensitive values are detected and replaced with deterministic keyed tokens
BEFORE any model sees a document. Two independent interfaces:

- **Detector**: what counts as sensitive. `RegexDetector` (patterns),
  `PresidioDetector` (Microsoft Presidio, local, no egress), or the
  composite of both. Select with `ONTO_DETECTOR=regex|presidio|both`;
  the composite degrades loudly, never silently, when Presidio is absent.
- **VaultBackend**: who holds the real values. `LocalVault` (demonstration
  only), `SkyflowVault`, `DatabunkerVault`. Select with
  `ONTO_VAULT=local|skyflow|databunker`.

Tokens are `TOK_{KIND}_{hmac_sha256(key, value)[:12]}`: deterministic, so
the same value produces the same token in every document and every question,
which makes the token itself an exact-match join key for entity resolution
across documents, achieved without any component handling the raw value.
Detokenisation requires the vault and happens only at render time.

```python
from tokenisation import build
tokeniser = build()                    # honours ONTO_DETECTOR / ONTO_VAULT
text, n = tokeniser.tokenise(raw_text) # n values replaced
```

The same scheme is trivially mirrored in other languages (an HMAC and four
regexes), so a chat front end can tokenise questions at its own door and the
tokens still join to the graph.

## chunker.py: semantic chunks that carry context and rejoin

Three decisions most splitters get wrong, implemented directly:

1. Semantic boundaries (whole paragraphs in whole sections), merged toward a
   target size: fewer, more meaningful chunks, fewer model calls.
2. Every chunk carries a `context` field: which document, what it is about,
   which section, where in the section. A fragment of a fifty-page report is
   uninterpretable without it.
3. IDs encode structure (`DOC-042.S3.P2` with parent/prev/next), so a
   consumer can rejoin the surrounding section instead of hallucinating it.

```python
from chunker import chunk_document, pack, render
chunks = chunk_document(text, doc_id="DOC-042")
for group in pack(chunks):            # packs to a context budget
    prompt_part = render(group)
```

## provenance.py: light up the Studio's 3D document map

The Studio's 3D view has two projections. With no provenance in the store it
draws the ontology itself (classes, hierarchy, instances). With
`?entity prov:wasDerivedFrom ?document` triples present it draws the far more
interesting one: documents as hubs, entities as bridges between them, and any
entity typed into disjoint classes by different documents as a red knot
between the documents that disagree.

```python
from provenance import emit_provenance
tail = emit_provenance({"DOC-201": ttl_1, "DOC-601": ttl_2})
# append to the merged store before loading
```
