# Benchmarks

## Ontology Generation

### Pizza Ontology — Manchester Tutorial

The [Manchester Pizza Tutorial](https://www.michaeldebellis.com/post/new-protege-pizza-tutorial) is the most widely used OWL teaching material. Students build a Pizza ontology in Protege over ~4 hours.

**Input:** One sentence — "Build a Pizza ontology following the Manchester tutorial specification."

| Metric | Reference (Protege) | AI-Generated | Coverage |
| ------ | ------------------- | ------------ | -------- |
| Classes | 99 | 95 | **96%** |
| Properties | 8 | 8 | **100%** |
| Toppings | 49 | 49 | **100%** |
| Named Pizzas | 24 | 24 | **100%** |
| Time | ~4 hours | ~5 minutes | |

The 4 missing classes are teaching artifacts (e.g., `UnclosedPizza`) that exist only to demonstrate OWL syntax variants. Files: [`benchmark/`](../benchmark/)

### IES4 Building Domain — BORO/4D

The [IES4 standard](https://github.com/dstl/IES4) is the UK government's Information Exchange Standard for defence/intelligence.

| Metric | Value |
| ------ | ----- |
| Compliance checks | **86/86 passed (100%)** |
| Triples | 318 |
| Classes | 36 |
| Properties | 12 |
| Generation | One pass — valid Turtle directly |

## Ontology Extension — Pizza Menu Mapping

Given the Manchester Pizza OWL and a 13-row restaurant CSV, map the data into the ontology.

| Metric | Value |
| ------ | ----- |
| Topping coverage vs reference | **94%** (62/66 matched) |
| IRI accuracy (Claude-refined) | **94-100%** |
| Vegetarian classification | **92%** (100% with refined mapping) |

## Mushroom Classification — OWL Reasoning vs Expert Labels

**Dataset:** UCI Mushroom Dataset — 8,124 specimens classified by mycology experts.

| Metric | Value |
| ------ | ----- |
| Accuracy | **98.33%** |
| Recall (poisonous) | **100%** — zero toxic mushrooms missed |
| False positives | 136 (1.67%) — conservative by design |
| False negatives | **0** |
| Classification rules | 6 OWL axioms |

## Vision Benchmark — Image to Knowledge Graph

**Dataset:** 10 real photographs with manually annotated ground truth.

| Metric | Manual | Pure Claude | RDF Pipeline |
| ------ | ------ | ----------- | ------------ |
| Object Recall | 100% | 89% | **95%** |
| Total RDF Triples | 0 | 0 | **2,540** |
| SPARQL Queryable | No | No | **Yes** |

## OntoAxiom Benchmark — Three Approaches

[OntoAxiom](https://arxiv.org/abs/2512.05594) tests LLM axiom identification across 9 ontologies and 3,042 ground truth axioms.

| Approach | F1 | vs o1 |
| -------- | -- | ----- |
| o1 (paper's best) | 0.197 | — |
| **Bare Claude Opus** | **0.431** | **+119%** |
| **MCP extraction** | **0.717** | **+264%** |

Full writeup: [`benchmark/ontoaxiom/ONTOAXIOM_SHOWDOWN.md`](../benchmark/ontoaxiom/ONTOAXIOM_SHOWDOWN.md)

## Reasoning Performance — HermiT vs Open Ontologies

**Pizza Ontology (4,179 triples)**

| Tool | Time | Result |
| ---- | ---- | ------ |
| HermiT | 213ms | 312 subsumptions |
| Open Ontologies (OWL-RL) | 43ms | Load + rule-based inference |
| Open Ontologies (OWL-DL) | 19ms | Consistency check, SHOIQ tableaux |

**LUBM Scaling (load + reason cycle)**

| Axioms | Open Ontologies | HermiT | Speedup |
| ------ | --------------- | ------- | ------- |
| 1,000 | 15ms | 112ms | **7.5x** |
| 5,000 | 14ms | 410ms | **29x** |
| 10,000 | 14ms | 1,200ms | **86x** |
| 50,000 | 15ms | 24,490ms | **1,633x** |

Scripts and results: [`benchmark/reasoner/`](../benchmark/reasoner/)

## Running Benchmarks

```bash
make bench          # Run all benchmarks
make bench-pizza    # Just Pizza
make bench-ontoaxiom # Just OntoAxiom
make bench-reasoner # Just reasoner comparison
```
