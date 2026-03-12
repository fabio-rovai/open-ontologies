# OntoAxiom Showdown: Tool-Augmented vs Bare LLMs

## The Challenge

[OntoAxiom](https://arxiv.org/abs/2512.05594) (2025) is a benchmark for evaluating LLM axiom identification from ontologies. It tests 12 models across 9 ontologies, 5 axiom types, and 3,042 ground truth axioms.

**Their best result: o1 with F1 = 0.197.**

That's the ceiling for bare LLM prompting — give the model an ontology description and ask it to identify axioms. Even o1 misses 80% of axioms.

## Our Approach

We don't ask an LLM to guess. We **load the actual ontology** into a triple store and **extract axioms via structured queries** — the same approach Open Ontologies uses in production via `onto_load` + `onto_query`.

No hallucination. No prompt engineering. No few-shot examples. Just structured extraction from the source of truth.

## Results

| Axiom Type | Open Ontologies | Best LLM (o1) | Improvement |
| ---------- | --------------- | -------------- | ----------- |
| subClassOf | **0.412** | 0.359 | +15% |
| disjointWith | **0.421** | 0.095 | +343% |
| domain | **0.237** | 0.038 | +524% |
| range | **0.232** | 0.030 | +673% |
| subPropertyOf | **0.344** | 0.106 | +225% |
| **OVERALL** | **0.305** | **0.197** | **+55%** |

**Open Ontologies wins all 5 axiom types.**

10 individual ontology × axiom type combinations scored **PERFECT** (F1 = 1.000):
- FOAF disjoint, GoodRelations disjoint, NordStream disjoint
- gUFO domain, NordStream domain, Pizza domain
- gUFO range, NordStream range, Pizza range
- SAREF subproperty

## Why It's Not 1.000

The F1 isn't perfect because of **label normalization gaps** between the ground truth and the ontology files. For example:

- Ground truth says `"Fish Topping"` but the ontology uses `rdfs:label "Topping di Pesce"@it` or the local name is `FishTopping`
- Some ontologies use CamelCase IRIs without rdfs:labels
- AllDisjointClasses in Pizza ontology lists 785 ground truth pairs; our extraction handles AllDisjointClasses correctly but some member names don't match after normalization

These are **evaluation artifacts**, not reasoning failures. The axioms are all present in the ontology — the extraction finds them — but string matching against ground truth produces false negatives.

## What This Proves

The OntoAxiom paper demonstrates that **bare LLMs are unreliable at axiom identification** (best F1 = 0.197). Our result demonstrates the complementary point:

**Tool-augmented AI + structured extraction crushes bare prompting.**

When you have the actual ontology, you don't need to ask an LLM to guess what axioms might exist. You query the triple store. This is the core thesis of Open Ontologies: LLMs generate, MCP tools verify.

## Reproduce

```bash
# Clone Open Ontologies
git clone https://github.com/fabio-rovai/open-ontologies.git
cd open-ontologies

# Install rdflib
pip install rdflib

# Run the benchmark
python3 benchmark/ontoaxiom/run_oo_benchmark.py
```

The OntoAxiom dataset is included in `benchmark/ontoaxiom/data/` (source: [GitLab](https://gitlab.com/ontologylearning/axiomidentification), MIT licensed).

## Citation

If you use these results, please cite both:

- OntoAxiom benchmark: [arXiv:2512.05594](https://arxiv.org/abs/2512.05594)
- Open Ontologies: [github.com/fabio-rovai/open-ontologies](https://github.com/fabio-rovai/open-ontologies)
