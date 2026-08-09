# Determinism

Two subsystems in this repository returned different answers on identical input.
Both are fixed. This page records what happened, because a flaky test is the
symptom and the disease is worth naming: **if output depends on hash iteration
order, the system is not a function of its input.**

Rust seeds `HashMap` and `HashSet` iteration order per process. Any traversal
derived from one, collected into a `Vec` and then walked, is a per-run coin
flip. That is harmless if the consumer is order-insensitive. Neither of these
was.

## 1. Tableaux classification

**Symptom.** `w3c_pizza_vegetarian_classification` in
`tests/owl2_conformance_test.rs` failed intermittently. On a fixed commit and a
fixed machine it failed on **2 runs in 6**. The assertion is that
`Margherita ⊑ VegetarianPizza` is inferred; on failing runs the classification
returned only `Pizza`.

**Cause.** `named_classes` is a `HashSet`, and `src/tableaux.rs` collected
`self.nodes.keys()` into a `Vec` at five expansion sites: the merge redirect,
the expansion loop, the max-cardinality rule, the disjunction rule and the
blocking update. Class order fixed candidate-pair order, which fixed which
subsumption checks were reached before a finite node or depth budget was spent.
Within a single check it fixed the order nodes were visited in.

The conformance tests do **not** disable the node and depth budgets. On an
unlucky ordering a budget was exhausted, `Tableau::decide` correctly returned
`Verdict::Unknown`, `is_subsumed` correctly declined to assert, and the
entailment was simply absent from the output. Nothing false was ever claimed;
something true was silently omitted, and omission is indistinguishable from a
correct negative without a complete oracle.

**Fix.** Sort both traversals by identifier. Node ids are assigned sequentially,
so sorting by id is creation order, the natural deterministic traversal.

| | Before | After |
| --- | --- | --- |
| Consecutive full runs of the conformance binary passing | 4 / 6 | **12 / 12** |
| Wall time, that binary | 1.17-2.57 s | **0.01 s** |

The two-order-of-magnitude speed-up says the previous flakiness was budget
exhaustion under bad orderings rather than genuine hardness. This is a
determinism fix, not a completeness fix; the nominal blowup in
[`benchmark/reasoner/regressions/`](../benchmark/reasoner/regressions/) is
untouched and still open.

## 2. Alignment

**Symptom.** The OAEI Anatomy candidate count moved between runs of the same
binary on the same input. Five runs, hashing the sorted correspondence set:

| Run | Correspondences | Set hash |
| --- | ---: | --- |
| 1 | 1151 | `c23fd913a17a` |
| 2 | 1153 | `77db7f7c5f79` |
| 3 | 1151 | `05db643ab8dc` |
| 4 | 1152 | `56f1e1cb5774` |
| 5 | 1154 | `b4f525ec6056` |

No two output sets matched.

**Cause.** Two links. `extract_classes` in `src/align.rs` ended in
`class_map.into_values().collect()`, so the class list arrived in hash order and
set the order candidate pairs were generated in. The candidate comparator then
sorted on confidence alone, which is **not a total order** here: the
zero-structural-signal branch assigns very many pairs the identical
`label_sim * 0.85`. `sort_by` is stable, so tied candidates kept generation
order, and the greedy 1-to-1 `retain` that follows is first-come-first-served.

**Fix.** Sort `extract_classes` output by IRI, and break confidence ties on
`(source_iri, target_iri)` so the comparator is total. Either alone would suffice
today; both together mean a future upstream change cannot reintroduce it.

| | Correspondences | Set hash |
| --- | ---: | --- |
| 5 consecutive runs after the fix | **1152** | **`469e0e04cb58`** (all five) |

**Consequence for the recorded result.** The Anatomy figure previously recorded
in this repository, P 0.963 / R 0.733 / F1 0.832, was one draw from the
distribution above. The deterministic value is:

| | P | R | F1 | Correspondences |
| --- | ---: | ---: | ---: | ---: |
| Deterministic | 0.960 | 0.730 | **0.829** | 1,152 |

Rank in the OAEI 2025 field is unchanged at 9th of 13. Three decimal places had
been quoted for a quantity whose third decimal was not a property of the system.

## Reproducing

```bash
# Tableaux: run the conformance binary repeatedly; all runs must agree.
for i in $(seq 1 12); do cargo test --test owl2_conformance_test 2>&1 | grep "^test result"; done

# Alignment: five runs must produce one hash.
for i in 1 2 3 4 5; do
  ./target/release/open-ontologies align \
    benchmark/oaei/data/anatomy/mouse.owl \
    benchmark/oaei/data/anatomy/human.owl \
    --min-confidence 0.80 --dry-run \
  | python3 -c "import json,sys,hashlib; c=json.load(sys.stdin)['candidates']; \
p=[(x['source_iri'],x['target_iri']) for x in c]; \
print(len(c), hashlib.sha256(repr(sorted(p)).encode()).hexdigest()[:12])"
done
```

## The general rule

Sort every hash-derived traversal, and make every comparator a total order.
Determinism is not a performance concern or a testing nicety. Both defects here
were found only because something downstream wobbled by one, and in both cases
the wobble was hiding a result that was not reproducible.
