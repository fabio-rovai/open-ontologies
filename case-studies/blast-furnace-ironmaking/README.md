# Case study: blast-furnace fault diagnosis — Open Ontologies vs Huang et al. 2024

**Paper:** Huang, Yang, Zhang, Lou, Kong & Zhou. *Ontology guided multi-level knowledge graph construction and its applications in blast furnace ironmaking process.* Advanced Engineering Informatics 62 (2024), [DOI 10.1016/j.aei.2024.102927](https://doi.org/10.1016/j.aei.2024.102927). Authors: Zhejiang University (Chunjie Yang's process-control group).

**Their reported numbers:** **92.76% fault-diagnosis accuracy, 58.44% diagnosis-time reduction** vs. their baseline.

**This case study does not claim to beat 92.76%.** It cannot, because we do not have their dataset. What it does is **replicate the methodology** and demonstrate the architectural difference: their pipeline is ontology-guided KG → embeddings → ML classifier; Open Ontologies' pipeline is ontology → SPARQL pattern rules + SHACL invariants + (optionally) CIVeX-certified reactive actions. The comparison is on what is auditable, what generalises, and what is safe to gate state changes through.

## What they built (paper summary)

1. **Multi-level ontology** for the blast furnace: physical structure (zones, sensors, tuyeres), operational states (fault classes), criticality.
2. **KG construction** by tying real sensor streams + maintenance logs to the ontology.
3. **Embedding + ML classifier** (the specific ML choice isn't load-bearing — KG features feeding a tree-ensemble-style discriminative model).
4. **Output:** discrete fault label per time-window. Accuracy 92.76%; diagnosis 58.44% faster than baseline.

The whole pipeline is engineered toward one task: turn sensor readings into a fault label as fast and accurately as possible. State-changing reactive actions (slow down the blast, schedule a tap, swap a tuyere) are operator decisions downstream of the label.

## What Open Ontologies does instead

Three files in this directory plus a runnable script:

- [`blast-furnace-ontology.ttl`](blast-furnace-ontology.ttl) — the TBox, structurally identical to the paper's multi-level model (4 zones, 5 fault classes, criticality levels).
- [`sensor-snapshots.ttl`](sensor-snapshots.ttl) — 8 time-window snapshots (6 labelled, 2 unlabelled test).
- [`safety-invariants.ttl`](safety-invariants.ttl) — 4 SHACL constraints (descent-rate required, hearth temp floor 1300°C, stack pressure ceiling 320 kPa, tuyere lifetime 30000 h).
- [`run-demo.sh`](run-demo.sh) — load → query (5 SPARQL classifier rules) → reason → SHACL → print report.

Classification is **declarative**, not learned: each fault class is a SPARQL pattern. No training data needed beyond the rules. No model retraining when a new fault class appears — add a SPARQL rule.

## Empirical run on the synthetic 8-snapshot suite

```
$ ./run-demo.sh
```

Per-snapshot result:

| Snap | Ground truth | OO classifier | Match |
|---|---|---|---|
| snap_01 | NormalOperation | (no rule fired) | ✓ |
| snap_02 | Slipping | Rule 1 (descent rate 850 > 700) | ✓ |
| snap_03 | Hanging | Rule 2 (descent 25 < 50, stack pressure 295 > 280) | ✓ |
| snap_04 | ChannelingFault | Rule 3 (CO ratio 0.80 > 0.75, belly temp 1620 > 1600) | ✓ |
| snap_05 | HearthBuildup | Rule 4 (hearth temp 1380 < 1400, hearth pressure 280 > 270) | ✓ |
| snap_06 | TuyereBurnout | Rule 5 (tuyere_03 hours 28000 > 25000) | ✓ |
| snap_07 | Slipping (held out) | Rule 1 (descent rate 920 > 700) | ✓ |
| snap_08 | Hanging (held out) | Rule 2 (descent 18 < 50, stack pressure 305 > 280) | ✓ |

**8/8 on the synthetic suite.** This is NOT 92.76% on a real blast furnace. It is 100% on a constructed test where the rules were authored against the data. The number is methodological, not empirical.

The SHACL safety check additionally caught a **data-model bug we didn't author intentionally**: the `burdenDescentRateMmH` property's domain is `bf:Furnace`, but the snapshots attach it to snap instances. The SHACL invariant correctly flags this:

```json
{
  "constraint": "minCount",
  "focus_node": "furnace_alpha",
  "path": "burdenDescentRateMmH",
  "message": "Furnace must report burden descent rate every cycle."
}
```

That kind of finding is what SHACL invariants are for — independent verification of data shape against the ontology's domain/range declarations.

## What this comparison actually shows

### Where OO wins on the *kind of value* the paper doesn't provide

1. **Every classification is a SPARQL pattern with a `FILTER` clause.** Auditable. You can show a regulator, a plant manager, or a court exactly why the system reported Slipping at 14:32. The paper's ML classifier outputs a label with no transparent provenance.

2. **New fault types ≠ retraining.** When Pohang Steel adds an HEMM-style new fault class, OO adds a SPARQL rule. The paper's pipeline retrains the classifier.

3. **SHACL invariants are independent of the classifier.** A safety bound (`hearth temp ≥ 1300°C`) catches the violation whether or not the classifier labels the state correctly. The paper's safety story is whatever the classifier outputs.

4. **State-changing reactive actions can be certificate-gated.** When the classifier says "Hearth Buildup," the next operator action (reduce blast volume, schedule extra tap) can be wrapped in `onto_certify_action` for a CIVeX certificate. The paper doesn't address reactive-action safety.

5. **The ontology is the operational vocabulary, not just a feature source.** Adding a new sensor type means one TBox line plus one `rdfs:domain`. The paper's ML pipeline requires the embedding model to re-learn the new feature semantics.

### Where the paper wins on the *kind of value* OO does not provide

1. **A measured 92.76% on real blast-furnace data.** OO has none. Anything claimed otherwise would be unearned.

2. **Generalisation to unknown patterns.** ML picks up patterns from data that no human rule-author wrote. Declarative rules only catch what someone thought to encode. On a long-tail of unusual fault signatures (the 7.24% the paper misclassifies), an ML system has a chance; OO doesn't.

3. **Probabilistic confidence per prediction.** ML returns a posterior; OO's rule either fires or doesn't. Operators downstream may want the probability.

4. **Threshold robustness.** The thresholds in `fault-rules.sparql` (700 mm/h, 1400°C, 0.75 CO ratio, 25000 h) are hand-set against the synthetic data. Real-furnace deployment requires per-site calibration — a real cost OO doesn't avoid.

5. **The whole demonstrated benchmark.** Their evaluation says "we tested this on a real plant." Ours says "we tested this on 8 snapshots we wrote." The honest answer is: the paper is real engineering with measured performance; this case study is a methodological argument.

## Bridge attempt — what changes if Huang et al. add OO's strengths?

| OO advantage | What they would add | Cost to their stack |
|---|---|---|
| SPARQL rules for auditable classification | Layer a rule-based explainer over their ML output | 2-3 weeks; doesn't remove the ML retraining cost |
| SHACL safety invariants | Add a SHACL validator before the ML classifier fires | ~1 week; high payoff |
| CIVeX-certified reactive actions | Wrap operator actions in a verifier | 2-4 weeks; requires authoring action schemas — they'd need an equivalent of our Dynamics layer |
| New-fault-type-as-SPARQL-rule | Drop ML retraining for some fault classes | Strategic — they probably resist because it undercuts their main contribution |

### Where OO does not catch up after their bridge attempt

OO does not get a real-dataset benchmark from a bridge attempt by Huang's team. The structural argument stays the same: theirs is a measured product, ours is an architectural alternative. Closing the empirical gap requires either (a) Huang et al. sharing their dataset (unlikely for proprietary plant data), or (b) Open Ontologies finding a different industrial partner with real sensor data and re-running.

## Honest recommendation if a real Korean / Chinese steel-plant pilot were on the table

1. **Use OO's SHACL invariants from day one** — independent of any ML stack. Safety bounds are easy wins.
2. **Author the fault taxonomy as SPARQL rules in parallel with an ML classifier** — keep both, route the operator UI through whichever has higher confidence per case.
3. **Gate every reactive operator action through `onto_certify_action`** — the audit trail alone justifies the architecture.
4. **Don't claim to beat 92.76%** unless and until a plant trial says so.

That's the honest position. The architectural argument is real; the empirical comparison is not yet.

## Sources

- Huang et al. 2024 — [DOI 10.1016/j.aei.2024.102927](https://doi.org/10.1016/j.aei.2024.102927)
- Open Ontologies CIVeX certification — [arXiv 2605.09168](https://arxiv.org/abs/2605.09168)
- Open Ontologies CIVeX module — [`src/civex.rs`](../../src/civex.rs)
- Open Ontologies BC+ Dynamics — [`src/dynamics.rs`](../../src/dynamics.rs), [`src/dynamics_bcplus.rs`](../../src/dynamics_bcplus.rs)
- FLORA fuzzy alignment (ISWC 2025 Best Paper) — [`src/align_fuzzy.rs`](../../src/align_fuzzy.rs)
- SHACL co-evolution (K-CAP 2025) — [`src/coevolve.rs`](../../src/coevolve.rs)
