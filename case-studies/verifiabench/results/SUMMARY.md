# Results summary

Authority: **Biolink Model**. Tasks: **30** real gene-disease facts, each asking a model to write Biolink-typed RDF. Oracle: **closed-world term existence + structural constraints (deterministic, no LLM judge)**. Nine models: five local open checkpoints (MLX), and Claude Haiku, Sonnet and Opus via `claude -p`.

| Model | Raw capability (fluency) | **Verified capability** | Mean term-existence | Fabricated terms |
|---|--:|--:|--:|--:|
| Qwen3-Coder-30B-A3B-Instruct-8bit | 1.00 | **1.00** | 1.00 | 0 |
| Claude Opus | 1.00 | **1.00** | 1.00 | 0 |
| Claude Haiku | 1.00 | **0.93** | 0.99 | 1 |
| Qwen3.6-35B-A3B-8bit | 1.00 | **0.77** | 0.92 | 7 |
| Claude Sonnet | 1.00 | **0.73** | 1.00 | 0 |
| Llama-3.2-3B-Instruct-4bit | 1.00 | **0.00** | 0.51 | 49 |
| Qwen2.5-3B-Instruct-4bit | 1.00 | **0.00** | 0.51 | 65 |
| gemma-2-2b-it-4bit | 0.23 | **0.00** | 0.06 | 21 |
| Qwen2.5-0.5B-Instruct-4bit | 0.00 | **0.00** | 0.00 | 0 |

**Headline.** Raw capability (fluency) saturates: **7 of 9 models** produce structured Biolink RDF on every task (raw 1.00). Verified capability spans the full range on the identical tasks. Two models tie at the top with perfect verified correctness and zero fabricated terms, one local (Qwen3-Coder-30B) and one frontier (Claude Opus). At the bottom, Qwen2.5-3B and Llama-3.2-3B look identical to the leaders on fluency (raw 1.00) yet score 0.00 verified, inventing roughly half of every term they emit. A fluency- or judge-based benchmark would rank the hallucinating models alongside the correct ones; the closed-world oracle separates them and cannot be gamed by fluent nonsense.

Two failure modes worth noting, both invisible to fluency grading: Qwen2.5-3B and Llama-3.2-3B fail by **hallucination** (49-65 fabricated terms). Claude Sonnet scores 0.73 with **zero** fabricated terms, its misses are **structural completeness** (a real gene and predicate but no typed disease), a different and milder defect the oracle also catches.