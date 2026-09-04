# dcat-us-binding

**A national metadata standard that claims conformance to W3C DCAT, whose 115 published
examples expand to 76 triples, one predicate and zero DCAT. The engine was used to
cross-check the repair, and the same run exposed two defects in the engine itself.**

Code and full evidence live in a standalone repository:
**[fabio-rovai/dcat-us-binding](https://github.com/fabio-rovai/dcat-us-binding)**.
This page records what the case study contributes here.

## The situation

[DCAT-US 3.0](https://github.com/GSA/dcat-us) is the metadata profile every US federal
agency publishes against, maintained by GSA for data.gov. Its README states it is "an
implementation of the World Wide Web Consortium's (W3C) DCAT standard". DCAT v3 section
4 defines conformance in RDF terms.

The project inherited a website, a JSON Schema and a SHACL definition that were out of
sync, with nobody left who knew which was authoritative, and collapsed to the JSON
Schema as the single source of truth. The SHACL file was deleted in PR #120. The live
repository publishes no JSON-LD context, so the JSON it ships has no defined RDF
interpretation.

A context for the profile does exist, at
[DOI-DO/dcat-us](https://github.com/DOI-DO/dcat-us), 48,311 bytes alongside Turtle
serialisations of every example. That repository is marked deprecated in its own README,
which points to <https://resources.data.gov/resources/dcat-us3/>, and `GSA/dcat-us`
re-checked at `main` on 4 September 2026 still publishes no context, JSON-LD, SHACL or
Turtle artefact outside its `DEPRECATED/` tree. The measurement below is against the live
repository. The deprecated context is worth reading as the control case rather than a
counterexample: it is hand-maintained and it has drifted, declaring a namespace host that
does not resolve and minting the W3C Organization Ontology under `w3c.org` instead of
`w3.org`. A context generated from the schema cannot acquire either defect.

## The result

| Expansion of the same 115 unedited example files | Triples | Predicates | DCAT triples | Empty files |
| --- | --: | --: | --: | --: |
| As published | 76 | 1 | **0** | 38 |
| With the generated binding | **1,510** | **123** | **228** | 10 |

The binding was not written. 26 of 26 classes and 231 of 270 properties already record
the RDF term they stand for, in `_oldDocs` blocks retained when SHACL was retired. The
context and the shapes are generated from those records, which is the point: two
projections of one source cannot contradict each other, so the objection that killed
SHACL upstream ("two validations that do not agree") stops applying.

## What validation reach means here

| Run | `conforms` | Target classes matched | Focus nodes | Results |
| --- | --- | --- | --: | --: |
| Deleted shapes, corpus as published | `True` | 0 / 34 | 0 | 0 |
| Deleted shapes, corpus with the binding | `False` | 23 / 34 | 228 | 287 |
| Generated shapes, corpus with the binding | `False` | 24 / 24 | 191 | 316 |

The first row is the failure mode this repository exists to name. A SHACL run reports
`conforms` whether or not any shape matched anything, so a green result is ambiguous by
construction. Zero focus nodes and zero violations is not a pass, it is a miss, and
`onto_shacl` reports `focus_nodes` alongside the verdict for exactly this reason. The
shapes GSA deleted were not stale. They were pointed at a graph that did not exist.

## What it forced in this engine

Both cross-checks were run through `open-ontologies batch` (`load` then `shacl`) against
pySHACL on identical inputs. The two agree exactly on reach, 191 focus nodes each, and
disagree on findings:

| Measure | pySHACL | this engine |
| --- | --: | --: |
| `sh:class` results | 165 | **0** |
| `sh:nodeKind` results | 122 | **0** |
| `sh:maxCount` results | 27 | 27 |
| `sh:datatype` results | 2 | **8** |

Two defects, both ours:

1. **`sh:class` and `sh:nodeKind` are not evaluated.** 287 of pySHACL's 316 findings are
   invisible to the engine.
2. **`sh:datatype` rejects a correctly typed literal for derived numeric types.**
   `"1024"^^xsd:nonNegativeInteger` fails `sh:datatype xsd:nonNegativeInteger`, while
   `xsd:integer`, `xsd:decimal` and `xsd:string` all pass. Eight-line repro in the
   standalone repository's `tests/test_engine_findings.py`.

A third defect belongs to rdflib 7.6.0, which raises `UnboundLocalError` from
`plugins/parsers/jsonld.py:242` on a scalar JSON-LD document rather than producing the
empty graph the specification calls for.

## Reproducing

```sh
git clone https://github.com/fabio-rovai/dcat-us-binding
cd dcat-us-binding && pip install -e '.[dev]' && make build && make test
```

Then the engine leg:

```sh
printf 'load build/corpus.expanded.ttl\nshacl build/dcat-us-3.0-shapes.ttl\n' \
  | open-ontologies batch -
```
