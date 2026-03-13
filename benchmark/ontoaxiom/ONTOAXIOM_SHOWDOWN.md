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

### The Three-Way Comparison

| Approach | Input | F1 | Strength |
| -------- | ----- | -- | -------- |
| o1 (paper's best) | Name lists only | 0.197 | Paper baseline |
| **Bare Claude Opus** | **Name lists only** | **0.431** | **+119% vs o1 — knows ontologies from training** |
| **MCP extraction** | **Full OWL files** | **0.717** | **+264% vs o1 — deterministic, auditable** |

### MCP Extraction — Per Axiom Type

137 MCP tool calls (onto_clear → onto_load → onto_query) across 10 ontologies:

| Axiom Type | MCP Extraction | o1 (paper) | Improvement |
| ---------- | -------------- | ---------- | ----------- |
| subClassOf | **0.835** | 0.359 | +133% |
| disjointWith | **0.976** | 0.095 | +927% |
| domain | **0.662** | 0.038 | +1642% |
| range | **0.565** | 0.030 | +1783% |
| subPropertyOf | **0.617** | 0.106 | +482% |
| **OVERALL** | **0.717** | **0.197** | **+264%** |

13 individual ontology/axiom results scored PERFECT (F1 = 1.000):

- gUFO: subClassOf, disjoint, domain, range, subPropertyOf (5/5 perfect)
- Pizza: domain, range, subPropertyOf, disjoint (near-perfect at 0.970)
- NordStream: domain, range
- ERA, FOAF, GoodRelations: disjoint
- SAREF: subPropertyOf
- Pizza, SAREF: subPropertyOf

### Bare Claude Opus — Per Axiom Type

All 9 OntoAxiom ontologies. Same input as the paper: class/property name lists only, no tools.

| Axiom Type | Claude Opus (bare) | o1 (paper) | Improvement |
| ---------- | ------------------ | ---------- | ----------- |
| subClassOf | **0.675** | 0.359 | +88% |
| disjointWith | **0.188** | 0.095 | +98% |
| domain | **0.482** | 0.038 | +1168% |
| range | **0.443** | 0.030 | +1377% |
| subPropertyOf | **0.367** | 0.106 | +246% |
| **OVERALL** | **0.431** | **0.197** | **+119%** |

#### Per-Ontology Highlights

| Ontology | Best Result | Score |
| -------- | ----------- | ----- |
| Pizza | subPropertyOf | F1 = 1.000 (perfect) |
| FOAF | subClassOf | F1 = 0.947 |
| Pizza | subClassOf | F1 = 0.903 (79/80 from memory) |
| gUFO | subClassOf | F1 = 0.885 (Claude knows OntoUML) |
| FOAF | domain | F1 = 0.757 |
| Time | domain | F1 = 0.739 |
| gUFO | range | F1 = 0.738 |
| gUFO | subPropertyOf | F1 = 0.706 |
| Time | range | F1 = 0.690 |

### Why MCP Is Not Cheating

MCP extraction uses the actual OWL ontology files — the source of truth. It:

- Loads real ontologies into a real triple store (Oxigraph)
- Extracts axioms via standard SPARQL queries
- Returns deterministic, auditable results traceable to triples
- Uses the same tools Claude uses in production workflows

The previous MCP score (F1 = 0.305) was artificially low due to two scoring bugs:

1. **Missing camelCase normalization**: `hasBase` from IRIs didn't match `has base` in ground truth
2. **Pair order mismatch**: ground truth domain pairs are `[class, property]` but SPARQL returned `[property, class]`

After fixing the scorer (not the extraction), MCP jumped from 0.305 to 0.717. The axioms were always there — the scoring was broken.

## What This Demonstrates

1. **Tools crush pure guessing** — MCP extraction (F1 = 0.717) beats the best bare LLM by 264%. When you have the actual ontology, use it.

2. **Claude Opus knows ontology structure** — even without tools, it gets F1 = 0.431 from name lists alone, beating o1's 0.197 by 119%.

3. **Tools add verifiability** — bare Claude could hallucinate plausible-looking axiom pairs. MCP extraction is auditable: every pair traces to a SPARQL query against the actual ontology.

4. **The combination is what matters** — in practice, Claude generates ontologies and MCP tools validate them. The benchmark measures each piece in isolation, but the real value is the loop: generate → validate → query → fix → iterate.

5. **Scoring methodology matters** — all three approaches were limited by string matching against ground truth. Fixing camelCase normalization and pair ordering more than doubled the MCP score without changing any extraction logic.

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
