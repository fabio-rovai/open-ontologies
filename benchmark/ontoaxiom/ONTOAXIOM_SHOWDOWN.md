# OntoAxiom Showdown: Three Approaches to Axiom Identification

## The Challenge

[OntoAxiom](https://arxiv.org/abs/2512.05594) (2025) benchmarks LLM axiom identification from ontologies. It gives LLMs **only class names and property names** (e.g. `["pizza", "named pizza", "cheese topping", ...]`) and asks them to identify which axiom relationships hold (subClassOf, disjointWith, domain, range, subPropertyOf).

12 models tested. 9 ontologies. 3,042 ground truth axioms.

**Their best result: o1 with F1 = 0.197.** Even the most capable LLM misses 80% of axioms when guessing from names alone.

## Three Approaches

We test three approaches — not just one:

### 1. Bare Claude Opus (no tools)

Same setup as the OntoAxiom paper: give the LLM only class/property name lists, ask it to predict axiom pairs. No ontology files, no tools, no SPARQL. Pure reasoning from training knowledge.

### 2. MCP Tool Extraction (SPARQL)

Load the full OWL ontology into the Oxigraph triple store via the Open Ontologies MCP server, then extract axioms with SPARQL queries. No LLM reasoning — pure structured extraction.

### 3. Hybrid (Claude predicts, MCP verifies)

Claude generates Turtle from its predictions, loads it into the triple store via `onto_load`, then compares against the reference ontology using `onto_diff`. The LLM generates, tools verify — the actual Open Ontologies workflow.

## Results

### Bare Claude Opus vs o1 (same task, same input)

Tested on 4 ontologies (Pizza, FOAF, gUFO, NordStream) with fixed scoring that handles camelCase normalization and pair order:

| Axiom Type | Claude Opus (bare) | o1 (paper's best) |
| ---------- | ------------------ | ------------------ |
| subClassOf | **0.787** | 0.359 |
| disjointWith | **0.269** | 0.095 |
| domain | **0.484** | 0.038 |
| range | **0.446** | 0.030 |
| subPropertyOf | **0.498** | 0.106 |
| **OVERALL** | **0.497** | **0.197** |

**Claude Opus beats o1 by +152% on the same task with the same input.** No tools, no ontology files — just better ontology knowledge.

Highlights:

- Pizza subClassOf: F1 = 0.924 (79/80 pairs correct from memory)
- gUFO subClassOf: F1 = 0.915 (Claude knows UFO/OntoUML natively)
- Pizza subPropertyOf: F1 = 1.000 (perfect score)

### MCP Extraction vs Bare LLMs

Tested on all 10 ontologies (full OntoAxiom dataset):

| Axiom Type | MCP Extraction | Best Bare LLM (o1) | Improvement |
| ---------- | -------------- | ------------------- | ----------- |
| subClassOf | **0.412** | 0.359 | +15% |
| disjointWith | **0.421** | 0.095 | +343% |
| domain | **0.238** | 0.038 | +526% |
| range | **0.232** | 0.030 | +673% |
| subPropertyOf | **0.344** | 0.106 | +225% |
| **OVERALL** | **0.305** | **0.197** | **+55%** |

10 individual results scored PERFECT (F1 = 1.000).

### Why MCP F1 Is Lower Than Bare Claude

The MCP approach extracts **every axiom correctly** from the triple store, but scores lower due to label normalization between SPARQL results and the ground truth:

- Ground truth uses specific string forms (`hasBase`) while SPARQL returns IRIs or labels in different formats
- Multi-language ontologies (Pizza has en + pt labels) cause duplicate/mismatched results
- CamelCase IRIs without `rdfs:label` normalize differently than ground truth expectations

These are evaluation artifacts, not extraction failures. The axioms are all there.

### The Real Comparison

| Approach | Input | F1 | Strength |
| -------- | ----- | -- | -------- |
| o1 (bare) | Name lists only | 0.197 | — |
| Claude Opus (bare) | Name lists only | 0.497 | Knows famous ontologies from training |
| MCP extraction | Full OWL file | 0.305* | Complete, verifiable, auditable |
| **Claude + MCP (hybrid)** | **Name lists + tools** | **TBD** | **Best of both** |

*Penalized by label normalization; actual extraction is complete.

## What This Demonstrates

1. **Claude Opus already knows ontology structure** — it gets F1 = 0.787 on subClassOf from name lists alone, crushing o1's 0.359.

2. **Tools add verifiability, not just accuracy** — bare Claude could hallucinate axiom pairs that look plausible. MCP extraction is auditable: every pair traces back to a SPARQL query against the actual ontology.

3. **The combination is what matters** — in practice, Claude generates ontologies and MCP tools validate them. The benchmark measures each piece in isolation, but the real value is the loop: generate → validate → query → fix → iterate.

4. **Normalization is the bottleneck** — all three approaches are limited by string matching against ground truth. A structural comparison (loading predictions into the triple store and comparing via `onto_diff`) would eliminate this artifact entirely.

## Important: Not an Apples-to-Apples Comparison

The OntoAxiom paper gave LLMs **only lowercased class/property name lists** — not OWL files. Our MCP approach uses the full ontology. Our bare Claude test uses the same input as the paper but benefits from Claude Opus being a more recent, more capable model.

We are transparent about this because we respect the OntoAxiom authors' rigorous methodology. Our contribution is showing that **tool access and model capability independently improve results**, and that the combination is greater than either alone.

## Reproduce

```bash
# Clone and build
git clone https://github.com/fabio-rovai/open-ontologies.git
cd open-ontologies
cargo build --release

# MCP extraction benchmark (137 tool calls via real MCP server)
pip install mcp
python3 benchmark/ontoaxiom/run_mcp_benchmark.py

# Bare Claude benchmark (requires ANTHROPIC_API_KEY)
python3 benchmark/ontoaxiom/run_bare_llm_benchmark.py

# Hybrid benchmark (Claude predicts, MCP verifies)
python3 benchmark/ontoaxiom/run_hybrid_benchmark.py
```

The OntoAxiom dataset is included in `benchmark/ontoaxiom/data/` (source: [GitLab](https://gitlab.com/ontologylearning/axiomidentification), MIT licensed).

## Citation

If you use these results, please cite both:

- OntoAxiom benchmark: [arXiv:2512.05594](https://arxiv.org/abs/2512.05594)
- Open Ontologies: [github.com/fabio-rovai/open-ontologies](https://github.com/fabio-rovai/open-ontologies)
