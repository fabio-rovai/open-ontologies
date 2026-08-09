# Reasoner regressions

Minimal reproductions of reasoner defects. Each file is the smallest input
found that triggers the behaviour.

## `nominals_blowup.ttl` — owl-dl does not terminate on nominals

**Found:** 2026-07-27, while repairing the LUBM benchmark harness.

`owl:hasValue` introduces a nominal (`∃R.{o}`), the "O" in SHOIQ. Nominals
break the tree-model property that ordinary subset blocking relies on, and the
current tableaux implementation appears to have no nominal-aware blocking. Cost
per additional nominal-bearing class is multiplicative, not additive:

| Nominal classes | Triples | `reason --profile owl-dl` |
| --------------- | ------- | ------------------------- |
| 1               | 11      | 0.030s                    |
| 2               | 19      | 0.029s                    |
| 3               | 27      | 0.047s                    |
| 4               | 35      | **> 20s (killed)**        |

Reproduce:

```bash
printf 'load benchmark/reasoner/regressions/nominals_blowup.ttl\nreason --profile owl-dl\n' \
  | ./target/release/open-ontologies batch -
```

This is why the LUBM benchmark never completes at any size: `generate_lubm.py`
emits one `owl:equivalentClass [ owl:hasValue ... ]` per department, so even the
1,000-axiom file carries dozens of nominals.

The Pizza ontology contains no `owl:hasValue` and classifies in ~0.04s, so the
defect is specific to nominals, not to scale.

**Status: contained, not fixed.**

The underlying blowup is still there. What changed on 2026-07-27 is that it can
no longer hang the process or corrupt the answer:

- `Tableau::expand` previously returned `false` on budget exhaustion, the same
  value it returns for a genuine clash. Callers therefore read "I ran out of
  room" as "this concept is impossible", which in classification becomes an
  asserted `owl:Nothing` subsumption. That was a **soundness** defect, not a
  performance one.
- Satisfiability is now three-valued (`Verdict::Satisfiable` /
  `Unsatisfiable` / `Unknown`). Only a completed, clash-terminated search yields
  `Unsatisfiable`. Exhaustion yields `Unknown` and the class lands in
  `undetermined_classes`.
- A per-test wall-clock deadline was added (`tableaux_test_timeout_ms`, default
  10s), because the node and depth budgets bound the size of the tableau but not
  the number of BRANCHES explored. Nominal blowup is branch explosion, so only a
  clock catches it.
- Reasoner output now carries `complete: true|false`. A caller can finally tell
  a proof from a run that gave up.

Current behaviour on this file: terminates in ~10s with

```json
{"complete": false,
 "undetermined_classes": ["...Department1", "...FacultyOf1", ...],
 "unsatisfiable_classes": [],
 "consistent": true}
```

Ten classes undetermined, zero fabricated unsatisfiability. That is the correct
answer to give when you cannot decide.

**Still open:** the reasoner cannot actually classify nominal-bearing
ontologies. The fix is the NI-rule of Motik, Shearer and Horrocks, *Optimizing
the Nominal Introduction Rule in (Hyper)Tableau Calculi*, which bounds root
individual creation at `n` per `≤n R⁻.A` restriction instead of generating them
freely. Separately, `update_blocking` implements ancestor **subset** blocking,
while SHIQ and SHOIQ require **pairwise** blocking once inverse roles and number
restrictions are present (Horrocks and Sattler). Both are prerequisites for any
published performance claim on a real corpus.
