---
title: Onto-Correctness Checker
emoji: 🧬
colorFrom: blue
colorTo: indigo
sdk: gradio
sdk_version: 4.44.0
app_file: app.py
pinned: false
license: mit
---

# Onto-Correctness Checker

A live demo of the open-world hole. SHACL, the default RDF validator, is open-world:
it silently passes any ontology term it has no shape for, which is exactly the failure
mode of a language model authoring RDF. A closed-world vocabulary gate catches it.

Pick a vocabulary (schema.org, IES4, or OBO PATO+RO), keep the preloaded hallucinated
example or paste your own Turtle, and press **Run** to see SHACL (open-world) report
`conforms=true` while the closed-world gate flags the fabricated terms.

Full benchmark, method and reproducible code:
[github.com/fabio-rovai/open-ontologies](https://github.com/fabio-rovai/open-ontologies/tree/main/case-studies/onto-correctness-bench)

Built by [Tesseract Academy](https://gov.tesseract.academy/research/ontology-correctness-benchmark).
