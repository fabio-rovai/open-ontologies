---
title: Onto-Correctness Checker
emoji: 🧬
colorFrom: blue
colorTo: indigo
sdk: static
pinned: false
license: mit
---

# Onto-Correctness Checker

A live, in-browser demo of the open-world hole. SHACL, the default RDF validator, is
open-world: it silently passes any ontology term it has no shape for, exactly the failure
mode of a language model authoring RDF. A closed-world vocabulary gate catches it.

Pick a vocabulary (schema.org, IES4, OBO PATO+RO), keep the preloaded hallucinated example
or paste your own Turtle, and press Run. Real `pyshacl` and `rdflib` run entirely in your
browser via Pyodide, no server.

Full benchmark and code: https://github.com/fabio-rovai/open-ontologies/tree/main/case-studies/onto-correctness-bench
Built by [Tesseract Academy](https://gov.tesseract.academy/research/ontology-correctness-benchmark).
