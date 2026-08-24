# OAEI Alignment Benchmark

Evaluates `onto_align` against the [Ontology Alignment Evaluation Initiative (OAEI)](http://oaei.ontologymatching.org/) standard tracks.

## Tracks

### Anatomy (Mouse-Human)

The OAEI Anatomy track aligns the Adult Mouse Anatomy ontology (2,744 classes, MGI) with the human anatomy fragment of the NCI Thesaurus (3,304 classes). The reference alignment contains 1,516 equivalence mappings.

This is the most widely reported OAEI track: nearly all alignment systems publish results on it, which makes it the best basis for comparison.

### Conference

The OAEI Conference track aligns 7 ontologies describing the conference organisation domain (ekaw, sigkdd, iasted, confOf, edas, cmt, conference). 21 pairwise alignments with reference mappings.

## Running

```bash
# Download OAEI data (one-time)
python3 download_oaei.py

# Run alignment benchmark
python3 run_oaei_benchmark.py

# Results appear in results/
```

## Results

Measured with `onto_align` at `min_confidence = 0.8`, stable 1-to-1 matching, label penalty on zero structural evidence. No external background knowledge (UMLS, BioPortal, LLM oracle) is used. Raw output in `results/oaei_results.json` and `results/conference_results.json`.

| Track | Precision | Recall | F1 | Predicted | Reference | Runtime |
|---|---:|---:|---:|---:|---:|---:|
| Anatomy | 0.960 | 0.730 | **0.829** | 1,152 | 1,516 | 47s |
| Conference (15 of 21 pairs, micro) | 0.693 | 0.320 | **0.438** | 101 | 219 | — |

## Comparison Systems

The full OAEI 2025 field, reproduced from Tables 9 and 10 of the official results paper ([CEUR Vol-4144, om2025-oaei-paper0](https://ceur-ws.org/Vol-4144/om2025-oaei-paper0.pdf)). Both baselines are included. No systems are omitted.

### Anatomy (Table 9, ordered by F-measure)

| System | Precision | F1 | Recall | Recall+ |
|---|---:|---:|---:|---:|
| Matcha | 0.951 | **0.941** | 0.931 | 0.820 |
| Agent-OM | 0.959 | **0.920** | 0.883 | 0.698 |
| ALIN | 0.942 | **0.912** | 0.884 | 0.700 |
| LogMapLLM | 0.964 | **0.899** | 0.842 | 0.588 |
| LogMap-Bio | 0.885 | **0.898** | 0.911 | 0.766 |
| MDMapper | 0.899 | **0.889** | 0.879 | 0.707 |
| LogMap | 0.917 | **0.881** | 0.848 | 0.602 |
| LogMapKG | 0.917 | **0.881** | 0.848 | 0.602 |
| **Open Ontologies** | **0.960** | **0.829** | **0.730** | — |
| DRAL-OA | 0.830 | **0.828** | 0.827 | 0.560 |
| LogMapLt | 0.962 | **0.828** | 0.728 | 0.288 |
| *StringEquiv (baseline)* | 0.997 | **0.766** | 0.622 | 0.000 |
| LSMatch | 0.952 | **0.761** | 0.634 | 0.037 |

Open Ontologies ranks **9th of 13**. Precision is third in the field; recall is second-from-bottom among non-baseline systems. F1 is level with LogMapLt, the lightweight lexical matcher, and **+0.063** over the `StringEquiv` string-equality baseline. Every system above it uses biomedical background knowledge, an LLM oracle, or both.

`Recall+` (recall excluding trivial same-normalised-label correspondences) is the metric that most exposes the gap and is not yet computed here; LogMapLt scores 0.288 on it against Matcha's 0.820.

### Conference (Table 10, `rar2` reference, classes and properties, F1-optimal threshold)

| System | Precision | F1 | Recall | Incoherent align. |
|---|---:|---:|---:|---:|
| ALIN | 0.62 | **0.65** | 0.68 | 7 |
| LogMap | 0.76 | **0.64** | 0.56 | 0 |
| Matcha | 0.77 | **0.63** | 0.53 | 9 |
| Agent-OM | 0.64 | **0.61** | 0.59 | 8 |
| MDMapper | 0.69 | **0.58** | 0.50 | 3 |
| *edna (baseline)* | 0.74 | **0.56** | 0.45 | — |
| LogMapLt | 0.68 | **0.56** | 0.47 | 3 |
| LSMatch | 0.83 | **0.55** | 0.41 | 0 |
| *StringEquiv (baseline)* | 0.76 | **0.53** | 0.41 | — |
| **Open Ontologies** | **0.693** | **0.438** | **0.320** | not measured |

Two caveats, both against this system. The OAEI rows cover all 21 pairs against `rar2` at each system's F1-optimal threshold; the Open Ontologies row covers 15 pairs at a fixed threshold, so the comparison is indicative rather than like-for-like. Even allowing for that, the result is **below every participating system and below both baselines**.

Per-pair spread (see `results/conference_results.json`): best `ekaw-iasted` 0.588 and `ekaw-sigkdd` 0.533; worst `edas-sigkdd` 0.211.

## Ablation: stable matching dominates

Removing stable 1-to-1 matching as the **only** variable, threshold held at 0.80
(`run_ablation_no_stable.py`, raw data in `results/ablation_no_stable.json`):

| Configuration | Precision | Recall | F1 | Candidates |
|---|---:|---:|---:|---:|
| With stable matching | 0.960 | 0.730 | **0.829** | 1,152 |
| Without stable matching | 0.711 | 0.746 | **0.728** | 1,590 |

Distinguish this from a second figure quoted elsewhere: the system *before* stable
matching, the label penalty and a raised pre-filter were introduced together scored
P 0.102 / R 0.846 / **F1 0.182** with 12,557 candidates. That is three simultaneous
changes and must not be attributed to the constraint alone.

### The weight ablation does not test what it appears to test

Five weight configurations with stable matching span F1 0.8303 to 0.8336, a spread of
0.0033; **without** stable matching all three configurations tested produce byte-identical
output. The obvious reading is "weights are irrelevant". The real reason is narrower.
`src/align.rs` branches:

```rust
if structural_sum == 0.0 { best_signal * 0.85 } else { signals·weights }
```

On Anatomy most class pairs share no properties, instances or restrictions, so
`structural_sum` is zero, the fallback fires, and **the weight vector is never read**. A
weight ablation on this track leaves most confidences untouched by construction. The
honest statement is that on this track weights are largely *not exercised*, and any
ablation reporting otherwise is measuring the fallback.

The conclusion about the constraint is unaffected, because it rests on the controlled
comparison above (0.101) rather than on the weight sweep (0.003). Raw data in
`results/ablation_signals.json` and `results/ablation_no_stable.json`.
