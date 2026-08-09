# verifiabench

**An un-game-able benchmark for scientific-workflow LLM reliability: grade the model with a
closed-world oracle, not a fluency check. Fluency is saturated; verified correctness is not.**

Most benchmarks for LLMs on scientific tasks grade the answer with string match or an LLM judge.
Both reward a confident, well-formed answer, so a model that emits a *plausible-but-nonexistent*
gene, ontology class or predicate can score as correct. verifiabench grades differently: every
term a model emits must **exist** in the authority (the real Biolink Model) and the output must
satisfy the task's **structural** constraints. Correctness is set-membership plus constraint
satisfaction, checked deterministically, so fluency cannot buy a point.

This is the reliable-evaluation layer Encode / ARIA Challengescape items on testing LLM reasoning
and reliability on realistic scientific workflows are asking for.

## The result

Deterministic run of [`src/verifiabench.py`](src/verifiabench.py): 30 real gene-disease facts, each
asking a model to write Biolink-typed RDF, graded by the closed-world oracle. Nine models, five
local open checkpoints (MLX) and Claude Haiku, Sonnet and Opus via `claude -p` (full table in
[`results/SUMMARY.md`](results/SUMMARY.md)):

| Model | Raw capability (fluency) | **Verified capability** | Mean term-existence | Fabricated terms |
|---|--:|--:|--:|--:|
| Qwen3-Coder-30B-A3B (8bit) | 1.00 | **1.00** | 1.00 | 0 |
| Claude Opus | 1.00 | **1.00** | 1.00 | 0 |
| Claude Haiku | 1.00 | **0.93** | 0.99 | 1 |
| Qwen3.6-35B-A3B (8bit) | 1.00 | **0.77** | 0.92 | 7 |
| Claude Sonnet | 1.00 | **0.73** | 1.00 | 0 |
| Llama-3.2-3B | 1.00 | **0.00** | 0.51 | 49 |
| Qwen2.5-3B | 1.00 | **0.00** | 0.51 | 65 |
| gemma-2-2b | 0.23 | **0.00** | 0.06 | 21 |
| Qwen2.5-0.5B | 0.00 | **0.00** | 0.00 | 0 |

**Raw capability** = produced structured Biolink-namespace RDF, what a fluency- or judge-based
benchmark would reward. **Verified capability** = a real `biolink:Gene`, a real `biolink:Disease`, a
real association predicate, and zero fabricated terms.

Seven of nine models produce structured Biolink RDF on every task (raw capability 1.00), so fluency
tells you almost nothing. Verified capability spans the full range. Two models tie at the top with
perfect verified correctness and zero fabricated terms, one local (Qwen3-Coder-30B) and one frontier
(Claude Opus). At the bottom, Qwen2.5-3B and Llama-3.2-3B look identical to the leaders on fluency
yet score 0.00 verified, inventing roughly half of every term they emit. Two distinct failure modes
the oracle exposes and fluency grading cannot: **hallucination** (the small local models, 49-65 fake
terms) and **structural incompleteness** (Claude Sonnet, 0.73 verified with zero fabricated terms,
its misses are a missing typed disease, not an invented term). A fluency- or judge-based benchmark
would rank the hallucinating models alongside the correct ones; the closed-world oracle cannot be
gamed by producing plausible but nonexistent terms.

## Expanded: two authorities, three task families

The single-family benchmark above is the easy case. We extended it to a second authority (the
**Gene Ontology**, 48,329 terms) and three task families, single-hop gene-disease (Biolink), GO
annotation (emit a real GO term), and cross-ontology multi-hop (Biolink and GO in one graph), then
re-ran six models (full table in [`results/SUMMARY_multi.md`](results/SUMMARY_multi.md)):

| Model | Overall verified | Biolink | GO annotation | Multi-hop | Fabricated terms |
|---|--:|--:|--:|--:|--:|
| Claude Opus | **1.00** | 1.00 | 1.00 | 1.00 | 0 |
| Claude Sonnet | **0.75** | 0.60 | 0.85 | 0.93 | 3 |
| Qwen3-Coder-30B-A3B | **0.48** | 1.00 | 0.00 | 0.07 | 34 |
| Claude Haiku | **0.43** | 0.77 | 0.20 | 0.07 | 28 |
| Llama-3.2-3B | 0.00 | 0.00 | 0.00 | 0.00 | 110 |
| Qwen2.5-3B | 0.00 | 0.00 | 0.00 | 0.00 | 167 |

On the single-authority task the local Qwen3-Coder-30B tied Claude Opus at 1.00. Add a second
ontology and multi-hop, and the tie breaks: **Opus scores 1.00 across all three families, the 30B
collapses to 0.48**, perfect on Biolink but 0.00 on GO annotation (it fluently emits fabricated
7-digit GO ids) and 0.07 on multi-hop. GO annotation is the discriminator; multi-hop is the hardest,
because it cannot be gamed by getting one authority right and inventing the other. The single family
called several models equal; the multi-domain version re-separates them and exposes cross-ontology
hallucination the single family missed.

## Why this is the ownable gap

- **Fluency and correctness are different axes.** Raw capability saturates at 1.00 for any competent
  model; verified capability ranges from 0.00 to 1.00 on the same tasks. Benchmarks that cannot see
  the difference are measuring the wrong thing.
- **The oracle is deterministic and closed-world.** A term is right if and only if it is in the
  authority and used in a valid structure. No string similarity, no LLM judge, no partial credit for
  confident nonsense. This is the [`onto_vocab_check`](../onto-correctness-bench/README.md) principle
  turned into an evaluation.
- **It is un-game-able the way that matters.** You cannot lift the score by writing more fluent RDF;
  you can only lift it by using terms that actually exist. That is exactly the property a scientific
  deployment needs from its model.
- **It validates in both directions.** A genuinely capable model (Qwen3-Coder-30B) scores 100%, so
  the oracle is not just failing everything; it is measuring a real capability the weak models lack.

## Reproduce

```bash
./run-demo.sh                       # runs the default model on a local MLX/OpenAI endpoint (:8080)
./run-demo.sh "model-a" "model-b"   # benchmark specific models
VB_API=https://your-endpoint/v1/chat/completions ./run-demo.sh "gpt-..."   # any OpenAI-compatible API
```

Requires Python 3.10+ and an OpenAI-compatible chat endpoint. The runs here used a local MLX server.

## Honest scope

See [`BUILD_REPORT.md`](BUILD_REPORT.md). This is one task family (Biolink gene-disease RDF authoring)
on one authority; the method extends unchanged to GO, ChEBI, Reactome and EDAM, which is the
documented next step toward a multi-domain suite. The 30 facts are well-established, not a held-back
secret split, so the honest use is relative comparison and a public, versioned oracle, not a secret
leaderboard. The oracle checks term existence and a coarse structure (a real Gene, a real Disease, a
real association predicate); deeper semantic correctness (right predicate for the specific relation)
is the certified-denotation direction, not measured here.

---

### Built by Tesseract Academy

We build the evaluation and assurance layer for AI that has to be right, not just fluent. If you are
choosing or deploying a model for scientific or ontology-grounded work, we can benchmark it on what
it actually gets correct.

[gov.tesseract.academy](https://gov.tesseract.academy) · fabio@thetesseractacademy.com
Part of [Open Ontologies](../../README.md) · MIT · real models, real numbers.
