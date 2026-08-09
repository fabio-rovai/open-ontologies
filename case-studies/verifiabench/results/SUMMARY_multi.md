# Expanded results: two authorities, three task families

Authorities: **Biolink Model** and **Gene Ontology** (65 tasks). Families: single-hop gene-disease (Biolink), GO annotation (a real GO term), and cross-ontology multi-hop (Biolink + GO at once). Oracle: closed-world term existence across both authorities + structure (deterministic, no LLM judge).

| Model | Overall verified | Biolink gene-disease | GO annotation | Multi-hop | Fabricated terms |
|---|--:|--:|--:|--:|--:|
| Claude Opus | **1.00** | 1.00 | 1.00 | 1.00 | 0 |
| Claude Sonnet | **0.75** | 0.60 | 0.85 | 0.93 | 3 |
| Qwen3-Coder-30B-A3B-Instruct-8bit | **0.48** | 1.00 | 0.00 | 0.07 | 34 |
| Claude Haiku | **0.43** | 0.77 | 0.20 | 0.07 | 28 |
| Llama-3.2-3B-Instruct-4bit | **0.00** | 0.00 | 0.00 | 0.00 | 110 |
| Qwen2.5-3B-Instruct-4bit | **0.00** | 0.00 | 0.00 | 0.00 | 167 |

**Headline.** On the single-authority benchmark a local Qwen3-Coder-30B tied Claude Opus at 1.00. Adding a second ontology and multi-hop tasks breaks the tie: **Claude Opus scores 1.00 on all three families**, while the 30B collapses to 0.48, perfect on Biolink but **0.00 on GO annotation** (it fluently emits fabricated 7-digit GO ids) and 0.07 on multi-hop. GO annotation is the discriminator, only frontier models emit real GO terms; multi-hop is the hardest, because it cannot be gamed by getting one authority right and inventing the other. A single-family benchmark called several models equal; the multi-domain, multi-hop version re-separates them and exposes cross-ontology hallucination the single family missed.