# BUILD_REPORT: onto-correctness-bench

Scrupulously honest record of what was fetched, what was computed, how the corpus was
constructed, and the limits of the claim. Nothing here is hand-tuned; every number in
`results/` is produced by `src/bench.py` on a fixed seed.

## What was fetched (real, public)

| Source | URL | Local | Triples parsed |
|---|---|---|---|
| schema.org (current) | https://schema.org/version/latest/schemaorg-current-https.ttl | `data/schemaorg.ttl` | 17,949 |
| PATO (Phenotype And Trait Ontology) | http://purl.obolibrary.org/obo/pato.owl | `data/pato.owl` | 259,922 |
| RO (Relation Ontology) | http://purl.obolibrary.org/obo/ro.owl | `data/ro.owl` | 11,640 |
| IES4 (UK Information Exchange Standard) | bundled in parent repo | `../../benchmark/reference/ies4.ttl` | 3,976 |

Fetched 2026-07-20. PATO and RO are combined into one "OBO (PATO+RO)" configuration
because PATO is class-heavy and declares almost no object properties of its own (it
imports RO/BFO); records type instances as PATO qualities linked by RO relations, which
is how OBO instance data is actually shaped.

## What was computed

For each vocabulary:
1. **Vocabulary extraction**: declared classes (`owl:Class`/`rdfs:Class`) and properties
   (`owl:ObjectProperty`/`owl:DatatypeProperty`/`owl:AnnotationProperty`/`rdf:Property`)
   whose IRI falls in the ontology's own *policed* namespace(s). Counts in `results/`.
2. **Corpus**: 100 clean + 100 hallucinated small "record" graphs. Every record is an
   instance minted under `https://ex.tesseract.academy/inst/` (never policed), typed with
   a real declared class and given 3 triples on real declared properties.
3. **Hallucinated records** are the clean record **plus** 1–2 *extra* fabricated terms.
   The real properties are all still present, so every record satisfies the SHACL shapes;
   the fabricated term is an unconstrained extra.
4. **SHACL shapes**: authored per record in the ordinary, non-closed style: one
   `sh:NodeShape` with `sh:targetClass <realClass>` and a `sh:property [ sh:path <realProp>;
   sh:minCount 1 ]` for each real property used. Validated with pySHACL
   (`inference="none"`, `advanced=False`).
5. **Closed-world gate**: a direct reimplementation of the `onto_vocab_check` principle:
   collect every predicate and every `rdf:type` object used in the data; flag any whose IRI
   sits in a policed namespace but is not in the declared set.

## How fabricated terms were constructed (so you can judge fairness)

The fabricated terms are *plausible-but-nonexistent*, not random strings a typo-checker
would catch:

- **Readable-name vocabularies (schema.org, IES4):** a real local name is mutated into a
  natural synonym or affixed variant (`priceRange` → `priceBracket`, `Offer` →
  `MerchandiseOffer`, `<name>` → `<name>Spec`), then **confirmed absent** from the declared
  vocabulary before use. These are the terms an LLM invents by analogy.
- **OBO (opaque IDs):** a well-formed OBO id in a policed prefix (`PATO_…`, `RO_…`,
  `BFO_…`) that is **confirmed undeclared**. This is the "LLM cites a real-looking but wrong
  ontology id" failure mode.

Every hallucinated record is guaranteed to receive at least one fabricated term, so the
per-graph catch-rate denominator is honest (no empty "hallucinated" graphs).

## The result, stated precisely

- SHACL false-pass rate = P(`conforms=true` | graph contains ≥1 fabricated term) = **100%**
  on all three vocabularies (300/300 graphs). This is *structural*, not a tuning artifact:
  SHACL Core has no shape targeting the fabricated term, so it is never inspected.
- Closed-world catch rate = P(gate flags ≥1 fabricated term | graph has one) = **100%**
  (300/300), with **0%** false positives on the 300 clean graphs.
- Term-level recall: closed-world **100%** of 418 injected terms; SHACL **0%** (it never
  flags a term by IRI-existence, because that is not what it checks).

## Limits of the claim (do not overstate)

1. **This is a demonstration of a structural property, not a surprising ML finding.** The
   closed-world gate is *defined* to flag undeclared policed-namespace terms, and the fakes
   are exactly that. The contribution is measuring, on real vocabularies at scale, that
   (a) ordinary SHACL practice cannot catch this class of error at all, and (b) the gate is
   precise (0 FP). The 100%s are the point, not a leaderboard score to beat.
2. **`sh:closed` can catch extra *predicates*** if you enumerate every allowed predicate on
   every shape and accept the maintenance and import-breakage cost. It still does not police
   `rdf:type` class values. We benchmarked ordinary open shapes because that is what teams
   deploy; a `sh:closed` comparison is a fair next addition.
3. **Only vocabulary *existence* is checked here.** A model that uses a *real* term in a
   *semantically impossible* place (right symbol, wrong denotation) is not caught by this
   gate. That is the third stage in the Track-A design (certified denotation against a
   world model) and is not measured in this repo yet.
4. **Corpus is synthetic-but-grounded.** The vocabularies are real; the record graphs are
   generated, not harvested from real LLM output. A follow-up should replay actual
   LLM-generated RDF (e.g. the IES4 fine-tune's outputs) through the same gate, the shipped
   `fabsssss/qwen3-coder-30b-a3b-ies4` model reports term-confusion 0%→88.6% and
   hallucination 0.937→0.010 under exactly this kind of closed-world check.

## Reproducibility

`./run-demo.sh` fetches the vocabularies and reruns `src/bench.py`. Deterministic: the
corpus uses a SHA-256-based PRNG seeded on (vocabulary, record index, role), so results are
identical across machines. rdflib 7.6.0, pyshacl (current) at build time.
