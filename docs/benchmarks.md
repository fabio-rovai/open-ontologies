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

The [IES standard](https://informationexchangestandard.org/) (canonical repo: [`IES-Org/ont-ies`](https://github.com/IES-Org/ont-ies); custodian: Department for Business and Trade since March 2025; the legacy [`dstl/IES4`](https://github.com/dstl/IES4) repo is archived, last public release was 4.3.1 under MIT) is the UK government's Information Exchange Standard for defence, intelligence, and increasingly built-environment / cross-sector use.

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

## OntoAxiom Benchmark — Tool Access vs Raw File vs Name Lists

[OntoAxiom](https://arxiv.org/abs/2512.05594) tests LLM axiom identification across five axiom types. The paper states 2,771 axioms; the released ground truth contains 3,042 ordered pairs over those types, or 2,514 if symmetric pairs are collapsed. We score the 3,042 and say so.

All conditions below are scored by **one** evaluator (`score_all_conditions.py`): shared normalizer, `domain`/`range`-only pair flipping, empty ground-truth cells skipped, both averages reported. Numbers are rescored from stored predictions, with no new inference.

| Model | Condition | Input | macro F1 | micro F1 |
| ----- | --------- | ----- | -------: | -------: |
| Claude Opus | A | Name lists only | 0.451 | 0.397 |
| Claude Opus | **D** | **Full raw OWL** | **0.768** | **0.686** |
| Qwen3-Coder-30B | A | Name lists only | 0.223 | 0.176 |
| Qwen3-Coder-30B | **D** | **Full raw OWL** | **0.673** | **0.667** |
| MCP + SPARQL extraction | C | Full OWL | 0.713 | 0.717 |

Two things follow, and neither is the flattering reading.

**Raw OWL helps.** The OntoAxiom paper's counterintuitive result, that an LLM given the full OWL file (0.323) scores below the same LLM given only name lists (0.431), does not survive a single shared scorer. Those two figures were produced by scripts that disagreed on three axes (camelCase normalisation, macro vs micro averaging, and which axiom types get pair-flipped), every one of which penalised the raw-OWL condition. Rescoring reverses the sign on **both** models under **both** averages. `score_condition_d.py --legacy` reproduces the 0.323 exactly, so the bug is demonstrated rather than asserted.

**Tools buy auditability, not accuracy.** Scored correctly, MCP extraction and raw-file reading are at parity: raw OWL wins macro (0.768 vs 0.713), extraction wins micro (0.717 vs 0.686). The remaining case for the tool layer is that every MCP pair traces to a SPARQL query against real triples, whereas a model reading a file can hallucinate a plausible pair and no F1 score will identify which one.

The OntoAxiom paper reports o1 at F1 = 0.197 as its best system. That figure is **not** used as a comparison baseline here, because the paper does not state which average it is, and mixing statistics is precisely the error being corrected above.

Results are **single-run** at temperature 0.2, not averaged over seeds. Several benchmark ontologies (Pizza, FOAF, OWL-Time) are widely published and likely present in pretraining data, so bare-LLM scores are contamination-inclusive baselines; the cross-model Qwen ablation exists to test whether an effect is a property of the tooling or of one vendor's model.

Full writeup, including the three-bug analysis and reproduction commands: [`benchmark/ontoaxiom/ONTOAXIOM_SHOWDOWN.md`](../benchmark/ontoaxiom/ONTOAXIOM_SHOWDOWN.md)

## OAEI Ontology Alignment

Anatomy 0.829 F1 (9th of 13 in the OAEI 2025 field), Conference 0.438 (below every participating system and both baselines). Full field, both baselines, and the stable-matching ablation: [`benchmark/oaei/README.md`](../benchmark/oaei/README.md)

## Claim Verification — Compiled Reasoning vs HermiT

The claim-verification benchmark measures the workload the `claimcheck` module
is built for: a fixed ontology, compiled once, against a stream of candidate
claims (small sets of type/relation assertions), each answered consistent /
rejected / undetermined.

### Methodology

- **Task-matched**: both engines answer the identical question on the identical
  file. The baseline (`ClaimConsistency.java`) asserts each claim as ABox
  axioms and asks HermiT 1.4.3.456 for full KB consistency, warm JVM, ontology
  loaded once and amortised across all claims.
- **Ground truth**: `DisjointnessMatrix.java` exhaustively tests every named
  class pair A ⊓ B for satisfiability with HermiT — the complete contradiction
  surface of the ontology, used to audit recall and soundness.
- **Adversarial claim generation**: `structural_parity.py` derives claims from
  the compiled structure itself — disjoint pairs pushed down to subclasses
  (contradictions reachable only by inference), sibling pairs (where
  incompleteness would show), and class-plus-own-superclass probes (which must
  never be rejected). Random pair sampling exercises a real contradiction only
  ~11% of the time; structural generation reaches ~60%.
- LUBM is deliberately not used: it is an ABox query benchmark with very simple
  schemas, and the literature (e.g. Lam et al., DMKG 2023) warns against using
  it to compare reasoners.

### Results (canonical pizza.owl, 1,944 triples, 99 classes; Apple M3 Max)

| Per-claim check | median | p95 | throughput |
| --- | --- | --- | --- |
| HermiT, warm | 4,936 µs | — | ~200 claims/s |
| compiled token-bitset check | 0.3 µs | 0.4 µs | 3.1M/s seq, 11.2M/s batched |

| Correctness | result |
| --- | --- |
| agreement with HermiT, 78,884 audited pairs (13 ontologies) | 100% |
| agreement on 793 structurally adversarial claims (8 ontologies) | 100%, 0 false negatives |
| contradiction recall, pizza.owl | 3,944/3,944 (100%) |
| contradiction recall, ore_ont_10230 | 232/232 (100%) |
| unsound rejections, all sweeps | 0 |

The compiled surface is sound but deliberately incomplete: pairs it cannot
settle return `Undetermined` and route to a reasoner-backed residual tier
(4.9% of adversarial claims in aggregate; per-ontology tier-1 coverage varies
with modelling style). The envelope and the propagation rules that close it
per idiom are documented in
[layer3-compiled-reasoning.md](layer3-compiled-reasoning.md).

### Offline compile cost

One classification pass plus a propagation fixpoint, per ontology:
pizza.owl ~120 ms; 659-class ore_ont_12925 ~270 ms for the compile step itself;
a 3,539-class ontology with 27k inferred subsumptions took 28.5 s to classify —
a build-time cost to budget for, not a query-time one.


## Running Benchmarks

```bash
make bench          # Run all benchmarks
make bench-pizza    # Just Pizza
make bench-ontoaxiom # Just OntoAxiom
make bench-reasoner # Just reasoner comparison
```

## Determinism and corrected results

Two subsystems in this repository were nondeterministic: tableaux classification
and alignment both let `HashMap` iteration order decide their output. Both are
fixed, and the Anatomy figure recorded before the fix (F1 0.832) was one draw
from a distribution whose deterministic value is 0.829. Full record, including
the five divergent alignment hashes and the reproduction commands:
[`docs/determinism.md`](determinism.md).

A speed claim against HermiT previously carried here and in `arXiv:2605.09184v1`
is **withdrawn**: it was measured on an empty store, and inverts when measured
correctly. See [`benchmark/reasoner/README.md`](../benchmark/reasoner/README.md).
