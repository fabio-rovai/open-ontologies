# Reasoner regressions

Minimal reproductions of reasoner defects. Each file is the smallest input
found that triggers the behaviour.

## `nominals_blowup.ttl` — owl-dl does not terminate on nominals

**Found:** 2026-07-27, while repairing the LUBM benchmark harness.

`owl:hasValue` introduces a nominal (`∃R.{o}`), the "O" in SHOIQ. This
reasoner has no nominal constructor: `Concept` in `src/tableaux.rs` has no such
variant, and `parse_restriction` approximates `owl:hasValue` as `∃R.A` where `A`
is an atomic concept named after the individual. Nominals
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

An earlier version of this file said the Pizza ontology contains no
`owl:hasValue` and classifies in about 0.04s, and concluded that the defect is
specific to nominals rather than to scale. That figure came from a stripped
566-triple Pizza fixture, not from the reference file. The reference
`benchmark/reference/pizza-reference.owl` carries six `owl:hasValue`
restrictions and one `owl:oneOf` enumeration of five individuals (`Country`),
counted as XML elements rather than lines, since `grep -c oneOf` also counts the
closing tag and reports two. It is itself a nominal-bearing ontology and cannot
separate nominals from scale.

Measured on 2026-10-13 with a development build, `reason --profile owl-dl`
over its 2,332 loaded triples did not complete: `complete: false`,
`budget_exhausted_in: ["satisfiability", "abox"]`, 143 of 144 named classes
undetermined, zero inferred subsumptions. Its run time is set by the phase
budgets, not by classification finishing, so it is not quoted here. The
conclusion that the blowup is specific to nominals rests on the table above
alone, not on Pizza.

**Status: fixed on 2026-10-13.** The file classifies in milliseconds with
HermiT's eight subsumptions and nothing else; `tests/tableaux_absorption_test.rs`
pins that. The cause was not nominals as such: each `C ≡ ∃worksFor.{d}` put the
disjunction `∀worksFor.¬{d} ⊔ C` on every node of every tableau, and four of
them multiplied. Absorbing `∃r.F ⊑ C` as `F ⊑ ∀r⁻.C` removes the disjunction.
Nominals are still approximated (`{d}` is read as an atom standing for it), so
the run reports `complete: false` with `owl:hasValue` under
`constructs_not_modelled`; the answer is exact here, and the flag says the
engine cannot promise that in general.

What follows is the history, kept because it explains the three-valued
verdicts every run still relies on.

**Status on 2026-07-27: contained, not fixed.**

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
freely. The second prerequisite was blocking: `update_blocking` implemented
ancestor **subset** blocking, while SHIQ and SHOIQ require **pairwise**
blocking once inverse roles and number restrictions are present (Horrocks and
Sattler). Pairwise blocking has since been implemented, see
`is_pairwise_blocked` in `src/tableaux.rs`. The NI-rule is still outstanding,
and it remains a prerequisite for any published performance claim on a real
corpus.
