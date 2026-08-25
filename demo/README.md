# Document-to-ontology demo pipeline

This directory holds the pipeline that turns a corpus of plain documents into
an ontology, populates it, verifies it, and scans the result for
contradictions that split along document provenance. The code here is
domain-agnostic: nothing about any particular subject is hardcoded, the
vocabulary is whatever the documents themselves turn out to be about.

The corpus that ships with this repository is public DCAT-US material, and
its provenance is recorded alongside it so every derived claim can be traced
back to the document and section it came from.

## Pipeline stages

1. **Read.** Every document in the corpus folder is loaded (`corpus_pipeline.py`,
   `ontology_from_docs.py`).
2. **Tokenise.** Sensitive values are detected and replaced with stable,
   deterministic tokens before anything reaches a model (`tokenisation.py`).
   The same value produces the same token in every document, so a token
   doubles as a join key for entity resolution without any component ever
   handling the raw value.
3. **Chunk.** Text is split on semantic boundaries, not fixed width, with
   each chunk carrying enough context to be interpreted on its own and an id
   scheme that lets a fragment be rejoined to its section (`chunker.py`).
4. **Derive per-document fragments.** Each document proposes its own classes,
   properties and disjointness axioms, independently of every other document
   (`ontology_from_docs.py`, stage DERIVE).
5. **Merge.** The per-document fragments are concatenated into one candidate
   ontology.
6. **Reconcile.** Independent derivation leaves behind competing modelling
   patterns: one document may model a distinction as a subclass partition
   while another models the same notion as a separate attribute class.
   Where both exist for the same parent, the attribute class is removed
   because it is redundant with the partition and is the one form a
   reasoner cannot check for contradictions. Status and State are exempt,
   because a status is a property of a thing over time, not a kind of
   thing (`ontology_from_docs.py`, function `_reconcile_ttl`).
7. **Populate.** Instances are extracted from the documents, constrained to
   the derived and reconciled ontology so nothing gets invented
   (`ontology_from_docs.py`, stage POPULATE; `extract.py`).
8. **Verify.** A closed-world check confirms every term used in the
   extracted data was declared in the ontology, and a standalone pass
   re-checks parsing, stats, lint, enforce, vocabulary and reasoning against
   whatever is currently on disk (`ontology_from_docs.py` stage VERIFY;
   `verify.py`).
9. **Scan for contradictions.** Once claims are extracted into the graph,
   contradictions are found in three cascading tiers: structural checks the
   reasoner and SPARQL catch outright, blocked candidate pairs that compare
   only claims about the same entity from different documents, and an
   optional model adjudication pass over the small surviving set
   (`contradiction_scan.py`). Findings that split along document provenance
   are the ones worth showing anyone: two documents disagreeing about the
   same entity is a genuine contradiction in the corpus, not a broken
   extraction.

Two supporting tools sit alongside the main pipeline:

- `corpus_text.py` extracts plain text directly from the source documents,
  with no help from the ontology, so a classic chunk-retrieval baseline can
  be compared fairly against the graph-based retrieval path.
- `kpi_context_graph.py` derives a KPI context graph from the `computedFrom`,
  `appliesTo`, `dependsOnKPI` and `governedBy` triples the ontology itself
  declares, so the graph cannot drift from the model it describes. It
  answers impact questions (what depends on a changed term) and evaluates
  indicators against current data.
- `cq/run-cross-doc.py` runs a fixed set of competency questions against the
  merged store, as a repeatable check that the ontology can answer the
  questions it was built to answer.

## Derived artifacts

Everything the pipeline produces is written under `demo/derived/` and
`demo/corpus_extracted/`, and is regenerated from the corpus on every run.
None of it is checked in:

- `demo/derived/_ontology.ttl`: the merged, reconciled ontology.
- `demo/derived/_corpus_text.json`: the plain-text baseline extraction.
- `demo/corpus_extracted/*.ttl`: one knowledge graph fragment per document.
- `demo/corpus_extracted/_merged.ttl` and `_live.ttl`: the merged store
  loaded into the running engine.
- `demo/_review.jsonl`: individuals the pipeline could not type with
  confidence, queued for a human decision rather than guessed at.

## Running it

```bash
python -m pip install -r demo/requirements.txt
make demo
```

`make demo` runs the pipeline end to end against the corpus in
`demo/corpus/dcat-us`: read, tokenise, derive, merge, reconcile, populate,
verify, and scan for contradictions. Each stage can also be run on its own,
for example:

```bash
python3 demo/ontology_from_docs.py --corpus demo/corpus/dcat-us
python3 demo/contradiction_scan.py
python3 demo/kpi_context_graph.py graph
```

Run the tests with:

```bash
python -m pytest demo/tests/
```

## The conformance finding

`demo/precomputed/findings.json` is not produced by the pipeline above. The
pipeline's contradiction scanner detects provenance-split typing conflicts
(two documents typing the same individual incompatibly), and this corpus's
disagreement is not that shape: it is a README's conformance claim
contradicted by the artifacts published alongside it. That finding is
established by a validator, not the model, and lives in
`demo/dcat_conformance.py`. `make demo-verify` runs it (and its tests) as
part of the verify target, so the command a sceptic runs re-derives the
figures below rather than only checking that committed bytes have not moved.

The script reads only files committed under `demo/corpus/dcat-us/`: the full
GSA/dcat-us `jsonschema/definitions/` and `jsonschema/examples/` tree
(`demo/corpus/dcat-us/jsonschema/`, vendored separately from the seven
documents the pipeline above reads), `recovered-shapes.ttl`, and
`recovered-context.jsonld`. Pull request 120 deleted two files, not one:
the SHACL shapes and the profile's only published JSON-LD `@context`
(`context/dcat-us-3.0.jsonld`). Both are recovered here, from the pull
request's base commit, and vendored unmodified; see
`demo/corpus/dcat-us/pr-120-record.md` and `MANIFEST.json`.

The script measures two independent things against the same 115 examples and
the same unmodified `recovered-shapes.ttl`, and reports both rather than
picking one:

- **A schema-derived reconstruction.** A JSON-LD context built from the RDF
  terms the JSON Schema's own `_oldDocs` blocks still carry (`declared`, and
  `observed`, which also relaxes terms the corpus itself publishes as prose
  rather than IRIs).
- **The real recovered context**, `recovered-context.jsonld`, injected
  verbatim (which binds almost nothing against today's examples, because its
  class bindings are keyed by CURIE and every example's own `@type` is the
  schema's bare title) and with each example's `@type` mechanically rewritten
  to the CURIE the schema's own `_oldDocs.rdfClass` already names for it
  (`typed`, exercising what the real context actually binds).

```bash
python3 demo/dcat_conformance.py
python3 -m pytest demo/tests/test_dcat_conformance.py
```

It writes `demo/corpus/dcat-us/jsonschema/generated-context.jsonld` (and an
`.observed.jsonld` variant), `demo/dcat_conformance_measurements.json` (every
figure this script measures, both ways), and, by hand from those
measurements, `demo/precomputed/findings.json`. No network call and no model
call happen anywhere in this path.

**No single SHACL violation count is defensible as "the" figure.** The
schema-derived reconstruction's own two variants disagree with each other
(178 declared vs. 272 observed). The real recovered context, exercised
against the same shapes, gives a third number again (147), lower than either
reconstruction because it scopes every property binding strictly to its
owning class with no cross-class default, unlike the reconstruction. And the
two recovered files disagree internally about what the `org:` prefix
expands to (`recovered-context.jsonld` line 18 vs. `recovered-shapes.ttl`),
which alone moves the real-context count by several more violations if
corrected. `demo/tests/test_dcat_conformance.py::
test_shacl_violation_counts_disagree_across_methods` pins the disagreement,
not a winner.
