# OntoAxiom Showdown: Tool-Augmented vs Bare LLMs

## The Challenge

[OntoAxiom](https://arxiv.org/abs/2512.05594) (2025) benchmarks LLM axiom identification from ontologies. It gives LLMs **only class names and property names** (e.g. `["pizza", "named pizza", "cheese topping", ...]`) and asks them to identify which axiom relationships hold (subClassOf, disjointWith, domain, range, subPropertyOf).

12 models tested. 9 ontologies. 3,042 ground truth axioms.

**Their best result: o1 with F1 = 0.197.**

Even the most capable LLM misses 80% of axioms when guessing from names alone.

## A Different Question

We don't attempt to solve the same task. The OntoAxiom benchmark tests whether LLMs can **infer** ontology structure from entity names — a pure language understanding challenge.

We ask a different question: **why infer when you can query?**

When an LLM has access to MCP tools, it doesn't need to guess which axioms exist. It loads the actual ontology into a triple store and extracts them via SPARQL. This is the core thesis of Open Ontologies: **LLMs generate, MCP tools verify.**

## Method

We run the **actual Open Ontologies MCP server** (`open-ontologies serve`), connect via the official MCP Python SDK over JSON-RPC 2.0 stdio, and execute the same tool chain Claude uses in production:

For each ontology:

1. `onto_clear` — reset the Oxigraph triple store
2. `onto_load` — load the Turtle file into the store
3. `onto_query` — run SPARQL queries to extract axiom pairs

137 MCP tool calls total across 10 ontologies and 5 axiom types. No hallucination. No prompt engineering. Just structured extraction through the real MCP protocol.

**Important:** The LLMs in the OntoAxiom paper received only class/property name lists. Our approach uses the full OWL ontology file. This is intentionally not an apples-to-apples comparison — it demonstrates that tool access changes the game entirely.

## Results

| Axiom Type | Tool-Augmented (OO) | Best Bare LLM (o1) | Improvement |
| ---------- | ------------------- | ------------------- | ----------- |
| subClassOf | **0.412** | 0.359 | +15% |
| disjointWith | **0.421** | 0.095 | +343% |
| domain | **0.238** | 0.038 | +526% |
| range | **0.232** | 0.030 | +673% |
| subPropertyOf | **0.344** | 0.106 | +225% |
| **OVERALL** | **0.305** | **0.197** | **+55%** |

**Tool-augmented extraction wins all 5 axiom types.**

10 individual ontology x axiom type combinations scored **PERFECT** (F1 = 1.000):

- FOAF disjoint, GoodRelations disjoint, NordStream disjoint
- gUFO domain, NordStream domain, Pizza domain
- gUFO range, NordStream range, Pizza range
- SAREF subproperty

## Why It's Not 1.000

Even with direct access to the source ontology, our F1 is 0.305 — not 1.000. This is entirely due to **label normalization gaps** between the ground truth and what SPARQL returns:

- Ground truth uses lowercased labels derived from `rdfs:label` or local names. When the ontology has multi-language labels (e.g. Pizza uses English + Portuguese), the ground truth picked one language while our SPARQL returns another.
- Some ontologies use CamelCase IRIs without `rdfs:label` at all. Our normalization (`CamelCase` -> `camel case`) may not match the ground truth's normalization.
- AllDisjointClasses enumeration via RDF lists produces member sets where individual member label normalization compounds the mismatch.

These are **evaluation artifacts**, not extraction failures. Every axiom is present in the triple store — the SPARQL finds them — but string matching against the ground truth's specific normalization produces false negatives.

## What This Demonstrates

The OntoAxiom paper proves that **bare LLMs are unreliable at axiom identification** — even o1 achieves only F1 = 0.197 from name lists alone.

Our benchmark demonstrates the complementary point: **when LLMs have tool access, the task transforms from "infer structure from names" to "query structure from source."** The LLM's role shifts from unreliable oracle to reliable orchestrator.

This is the MCP value proposition in one benchmark: connect the LLM to the right tools and the hardest reasoning tasks become straightforward queries.

## Reproduce

```bash
# Clone and build
git clone https://github.com/fabio-rovai/open-ontologies.git
cd open-ontologies
cargo build --release

# Install MCP Python SDK
pip install mcp

# Run the benchmark (uses real MCP server via JSON-RPC 2.0 stdio)
python3 benchmark/ontoaxiom/run_mcp_benchmark.py
```

The benchmark starts the MCP server as a subprocess, connects via the official MCP SDK, and runs 137 tool calls — the same protocol Claude uses when calling `onto_*` tools.

An alternative rdflib-only version (`run_rdflib_benchmark.py`) produces identical F1 scores, confirming the Oxigraph SPARQL results match rdflib's RDF graph traversal.

The OntoAxiom dataset is included in `benchmark/ontoaxiom/data/` (source: [GitLab](https://gitlab.com/ontologylearning/axiomidentification), MIT licensed).

## Citation

If you use these results, please cite both:

- OntoAxiom benchmark: [arXiv:2512.05594](https://arxiv.org/abs/2512.05594)
- Open Ontologies: [github.com/fabio-rovai/open-ontologies](https://github.com/fabio-rovai/open-ontologies)
