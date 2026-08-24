# onto-correctness-bench

**The open-world hole, measured: why SHACL cannot catch a hallucinated ontology term, and what closes it.**

A reproducible benchmark showing that the field's default RDF validator, **SHACL, is
open-world** and therefore structurally blind to the most common failure mode of
LLM-generated knowledge graphs, a *plausible-but-nonexistent* class or property. It
measures the gap on **three real public vocabularies** and shows that a **closed-world
vocabulary gate** (the [`onto_vocab_check`](../../README.md) principle from Open
Ontologies) catches exactly what SHACL admits.

This is the correctness layer that Encode / ARIA Challengescape items
[#101](https://encode-challengescape.pillar.vc) ("AI systems built on the Semantic Web
lack formal guarantees of correctness and safety") and #108 ("shared representations
connecting domain concepts to model internals") are asking for.

## The result

Deterministic run of [`src/bench.py`](src/bench.py), 100 clean + 100 hallucinated
record graphs per vocabulary, no hand-entered numbers (full table in
[`results/SUMMARY.md`](results/SUMMARY.md), raw data in
[`results/results.json`](results/results.json)):

| Vocabulary | Ontology triples | Declared classes / props | Fabricated terms | **SHACL false-pass** | **Closed-world catch** | CW false-positive |
|---|--:|--:|--:|--:|--:|--:|
| schema.org | 17,949 | 933 / 1,521 | 136 | **100%** | **100%** | 0% |
| IES4 | 3,976 | 510 / 204 | 134 | **100%** | **100%** | 0% |
| OBO (PATO+RO) | 270,126 | 2,889 / 759 | 148 | **100%** | **100%** | 0% |

**Across 3 real vocabularies and 418 fabricated terms in 300 graphs, open-world SHACL
reported `conforms=true` on every single graph containing a fabricated term (300/300).
The closed-world gate flagged one in 300/300, with zero false positives on clean graphs.**

## See it on one concrete record

[`examples/hallucinated.ttl`](examples/hallucinated.ttl) is a valid-looking `schema:Offer`
with two terms that **do not exist** in schema.org, the class `schema:MerchandiseOffer`
and the predicate `schema:priceBracket`:

```
SHACL      conforms=True   flags: []                              <- admits both fakes
closed-world              flags: [MerchandiseOffer, priceBracket]  <- catches both
```

SHACL conforms because no shape *targets* those terms, so it never looks at them. That is
not a bug in pySHACL; it is the open-world semantics of SHACL Core. An LLM that emits a
fabricated term therefore sails through the exact validator teams rely on to trust it.

## Why this is the ownable gap

- **SHACL validates shapes, not vocabulary.** It reports violations only for triples a
  shape constrains. A predicate or `rdf:type` class with no shape is silently ignored.
- **`sh:closed` is not the fix.** Closed shapes catch *extra* predicates only if you
  enumerate every allowed predicate on *every* shape in advance, do not police `rdf:type`
  class values, and break the moment a record legitimately uses an imported vocabulary.
  It is per-shape bookkeeping, not a vocabulary guarantee.
- **The closed-world gate is one check:** every predicate and every `rdf:type` class whose
  IRI lives in a *policed* namespace must be **declared** in the loaded ontology. Standard
  `rdf`/`rdfs`/`owl`/`xsd`/`sh` vocabulary and your own instance IRIs are never policed, so
  the false-positive rate on clean data is 0%.

This is the primitive already shipped as a native Rust MCP tool, `onto_vocab_check`, in the
parent [Open Ontologies](../../README.md) server. This benchmark quantifies, on real
vocabularies, why it is not optional for any pipeline that lets a model author RDF.

## Reproduce

```bash
./run-demo.sh          # fetches the 3 vocabularies, runs the deterministic benchmark
```

Requires Python 3.10+. Downloads: schema.org (current Turtle), PATO and RO from the
OBO Foundry PURLs. IES4 is bundled in the parent repo at
[`benchmark/reference/ies4.ttl`](../../benchmark/reference/ies4.ttl).

## Honest scope

See [`BUILD_REPORT.md`](BUILD_REPORT.md) for exactly what was fetched and computed, the
construction of the fabricated terms, and the limits of the claim. In short: this
demonstrates a *structural* property (SHACL cannot see undeclared terms; a closed-world
gate can), measured at scale on real vocabularies. It is not a claim that closed-world
checking replaces SHACL, the two are complementary. It does not measure whether a *real*
value is used in a *semantically wrong* place (that is the next gate: certified denotation).

---

### Built by Tesseract Academy

We build the correctness and assurance layer for AI-generated knowledge graphs and
ontologies. If you are deploying LLMs that author or extend RDF/OWL, in science, defence,
health, or the enterprise, and need to guarantee that every emitted term actually exists,
we can help.

**Get the closed-world gate running on your ontology, or talk to us about an assurance
review:** [gov.tesseract.academy](https://gov.tesseract.academy) · fabio@thetesseractacademy.com

Part of the [Open Ontologies](../../README.md) project · MIT licensed · real data, real numbers.
