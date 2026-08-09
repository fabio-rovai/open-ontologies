# The Open-World Hole: Why SHACL Cannot Catch a Hallucinated Ontology Term (and What Closes It)

*Tesseract Academy · Open Ontologies case study · all numbers reproducible from [onto-correctness-bench](../README.md)*

Ask a large language model to emit RDF and it will, most of the time, give you
syntactically perfect Turtle. It will also, some of the time, give you a term that does
not exist: a `schema:priceBracket` where it meant `schema:priceRange`, a
`schema:MerchandiseOffer` that was never a class, an OBO id off by a digit. The triple
parses. It reads correctly. And it is referentially fake.

The reflex is to reach for SHACL. SHACL is the W3C validation language for RDF; it is what
serious knowledge-graph teams run before they trust generated data. So here is the
uncomfortable result, measured on three real vocabularies: **SHACL does not catch this. Not
some of the time. None of the time.**

## The measurement

We took three real, public vocabularies of very different shapes: schema.org (the web's
vocabulary, 933 classes and 1,521 properties), IES4 (the UK government's Information
Exchange Standard, 510 classes and 204 properties), and a combination of PATO and RO from
the OBO Foundry (2,889 classes and 759 properties, from 270,126 triples). For each we
generated 100 clean record graphs using only real, declared terms, and 100 "hallucinated"
graphs that are identical except for one or two extra fabricated terms, each confirmed
absent from the vocabulary but named plausibly enough that a model would emit it. Then we
validated every graph two ways: with ordinary hand-authored SHACL shapes, and with a
closed-world vocabulary gate.

| Vocabulary | Fabricated terms | SHACL false-pass rate | Closed-world catch rate | CW false-positive |
|---|--:|--:|--:|--:|
| schema.org | 136 | **100%** | **100%** | 0% |
| IES4 | 134 | **100%** | **100%** | 0% |
| OBO (PATO+RO) | 148 | **100%** | **100%** | 0% |

Across 418 fabricated terms in 300 graphs, SHACL reported `conforms=true` on every single
graph that contained a fabricated term. The closed-world gate flagged one in all 300, and
raised zero false alarms on the 300 clean graphs. The full run is deterministic and
[reproducible in one command](../run-demo.sh).

## This is not a bug. It is the semantics.

It would be easy to read that 100% as a defect in some SHACL engine. It is not. It is the
open-world assumption working exactly as specified. SHACL, as Holger Knublauch and Dimitris
Kontokostas set out in the [W3C Recommendation](https://www.w3.org/TR/shacl/), validates a
data graph against *shapes*. A shape targets some nodes and constrains some of their
properties. If a triple uses a predicate no shape mentions, or an `rdf:type` no shape
targets, SHACL has nothing to say about it, and silence means conformance. The validator is
not asking "does this term exist?" It is asking "do the terms I was told to check satisfy
their constraints?" A fabricated term is, by construction, one nobody told it to check.

Consider one concrete record from the benchmark, a `schema:Offer` carrying two terms that
do not exist in schema.org:

```turtle
ex:offer1 a schema:Offer , schema:MerchandiseOffer ;   # fabricated class
    schema:priceCurrency "GBP" ;
    schema:price "49.00" ;
    schema:priceBracket "mid" .                          # fabricated predicate
```

Run it against a normal `schema:Offer` shape that requires a price and a currency, and it
conforms. Both real constraints are satisfied; the two fakes are simply extra triples the
shape never looks at. The closed-world gate, asked instead "is every schema.org term used
here actually declared in schema.org?", returns `MerchandiseOffer` and `priceBracket`. Same
data, two questions, and only one of them is the question you actually care about when a
model wrote the graph.

## "Just use sh:closed"

The informed objection is that SHACL has `sh:closed`, which rejects triples whose predicates
are not in an allowed list. It does, and it helps, but it is not the guarantee people think
it is. Closed shapes catch extra *predicates* only if you enumerate every permitted
predicate on every shape, in advance, and keep that list current. They do not police the
values of `rdf:type` against the vocabulary. And they break the moment a record legitimately
uses a term from an imported ontology you did not list, which in the OBO world (where PATO
imports RO imports BFO) is constant. `sh:closed` is per-shape bookkeeping. What generated
data needs is a vocabulary guarantee: not "did this node stay inside the property list I
wrote for it", but "does every term in this graph denote something the ontology actually
defines".

That is a different check, and it belongs at a different place in the pipeline. Frank van
Harmelen and colleagues, in their [boxology of hybrid learning-and-reasoning
systems](https://arxiv.org/abs/2102.11965), make the point that neuro-symbolic architectures
are assemblies of distinct components, and that being explicit about what each box does is
how you reason about the whole. Symbol grounding, the check that a generated symbol refers
to something real, is one such box. Most current LLM-to-KG pipelines simply do not have it.
They have generation, and they have SHACL, and they assume SHACL is the grounding box. It is
not. It never claimed to be.

## Why a closed-world gate is precise, not just strict

The obvious worry about a stricter checker is false positives: flag everything and you flag
nothing useful. The benchmark answers that directly. The gate polices only IRIs whose
namespace belongs to the ontology under test, plus namespaces you explicitly name. Standard
`rdf`, `rdfs`, `owl`, `xsd`, and `sh` vocabulary is never flagged, and your own instance
identifiers are never flagged, because they are not in a policed namespace. The result is a
0% false-positive rate on 300 clean graphs across all three vocabularies. It is strict about
exactly one thing (did you use a term the ontology does not define) and silent about
everything else.

This is the check that ships as a native Rust tool, `onto_vocab_check`, in the
[Open Ontologies](../../../README.md) MCP server. Barry Smith's OBO Foundry and Chris
Mungall's tooling around it have spent two decades insisting that biomedical terms mean one
thing and are declared in one place; a machine that authors against those ontologies should
be held to the same closed-world standard the human curators are. The benchmark is a way of
saying, with numbers, that open-world validation alone cannot hold it there.

## What this does and does not prove

Being honest about scope is the point of publishing the [BUILD_REPORT](../BUILD_REPORT.md)
alongside the numbers. Three caveats matter. First, the 100%s are a structural property, not
a leaderboard score: the gate is defined to catch undeclared terms and the fabricated terms
are undeclared, so the contribution is the measurement, at scale, on real vocabularies, that
ordinary SHACL practice catches none of them and the gate catches all of them cleanly.
Second, this checks term *existence*, not term *appropriateness*: a model that uses a real
term in a semantically impossible place is a harder problem, and the next gate (certified
denotation against a world model) is where that lives. Third, the corpus is generated, not
harvested; the natural follow-up is to replay real LLM output through the same gate, which
is exactly the regime under which our IES4 fine-tune moved term-confusion from 0% to 88.6%
and hallucination from 0.937 to 0.010.

The headline stands regardless. If you are letting a model write RDF and you are validating
it with SHACL alone, you have an open-world hole, and everything that falls through it looks
exactly like clean, conformant data.

---

**Tesseract Academy builds the correctness and assurance layer for AI-generated knowledge
graphs.** If you are deploying LLMs that author or extend ontologies, in science, defence,
health, or the enterprise, and you need to guarantee that every emitted term actually
exists, the closed-world gate above runs on your vocabulary today.

Reproduce the benchmark: [onto-correctness-bench](../README.md) · Talk to us:
[gov.tesseract.academy](https://gov.tesseract.academy) · fabio@thetesseractacademy.com
