# 0016 · A conclusion names the axioms responsible for it, and minimality is re-run

- **Status**: implemented · `src/justify.rs`, `src/provenance.rs`,
  `Reasoner::derivation_graph` · `onto_justify`, `onto_provenance` ·
  `tests/justify_test.rs` (17), `tests/provenance_test.rs` (18) · opt-in, and a run that does
  not ask for the DAG pays one branch per candidate triple
- **Written**: 2026-09-15
- **Related**: decision 0002 (an inference carries a certificate), decision 0007 (a slice
  preserves a conclusion or it does not); `docs/explanation.md`

## The problem

The engine could say what it derived, and since decision 0002 it could hand over a certificate a
proved-sound Lean checker verifies. Neither answers the question that actually gets asked when a
conclusion is wrong or an ontology contradicts itself, which is **which of my axioms did this**.

`derivations.tsv` is a derivation DAG and nothing read it back for explanation. That was the gap.

## Decisions

1. **Explanation does not read the certificate file.** `derivations.tsv` records ONE step per
   inferred triple, the first the fixpoint reached, which is correct for a checker that
   re-derives and wrong for an explainer. A triple derived two independent ways has two
   justifications and the file shows one. An explanation built from it would be a confident,
   specific, false statement about where a conclusion came from, which is worse than a vague one.
   `Reasoner::derivation_graph` captures EVERY applicable ground rule instance instead.
   `tests/justify_test.rs::the_derivation_dag_carries_both_instances_where_a_certificate_carries_one`
   measures the difference on one graph: two instances, one line.

2. **Minimality is the product, not a refinement of it.** A support set that is not minimal
   blames axioms that had nothing to do with the conclusion. Someone who deletes one of them and
   watches the conclusion survive learns to distrust the tool rather than the ontology, so a
   non-minimal answer is not a slightly worse answer.

3. **Minimality is established by re-running the fixpoint, and by nothing else.** For every
   element of every reported justification the engine is run again without it and the conclusion
   must be gone. No Lean theorem says a set is minimal, the payload does not imply one, and the
   word in it is `verified_by_re_running_the_fixpoint_without_each_element`.

4. **Sufficiency is the half a theorem can cover, so it is offered separately.** With
   `certificate_dir` each justification gets a certified run over exactly its own triples, and
   `lake exe oo-cert` verifies under `OOCert.certificate_sound` that the conclusion follows from
   that subset. The payload says what that establishes and what it does not, in those words.

5. **Completeness of the LIST is an algorithm's claim and is bounded out loud.** Reiter's
   hitting-set tree enumerates justifications over the same oracle. `max_justifications` and
   `max_oracle_calls` bound it, `truncated` fires and `truncation.bound` names which one. A
   truncated answer is incomplete and is never wrong: every justification in it was re-run.

6. **The one non-monotone corner of the engine is named rather than assumed away.** Reiter's
   construction is complete for a monotone oracle. No rule reads the absence of a triple, but the
   restriction and list vocabulary is read into maps keyed by the node, so a node carrying two
   values for a functional position contributes one, chosen by hash order. Removing a triple can
   change which value is read. Where that bites, a justification can be MISSED. None can be
   falsely reported, because each is re-run.

7. **Inconsistency gets the same machinery and keeps `onto_reason`'s word.** The target becomes
   "this engine finds a clash", the verdict explained is `clash_found_by_this_engine`, and the
   Lean checker's `unsatisfiable_under_disjointness` is never spoken here. Ten of the seventeen
   rules that conclude `false` are looked for, so no clash found is still not a consistency
   result.

8. **Provenance is a semiring evaluation over the same DAG, and recursion is reported rather than
   hidden.** Six semirings ship. Datalog is recursive, so the counting semiring diverges on a
   cycle. Rather than print a finite number and let a reader assume it is the count, round `k`
   computes the annotation over proof trees of height at most `k`, `depth_bound` rides beside the
   value, `value_is_exact` is false unless the iteration stabilised on its own, and
   `cycle_in_support` names a triple on the cycle. `tests/provenance_test.rs` pins two bounds
   giving two different numbers, which is what a bound has to do to be a bound.

9. **Convergence is never one word covering unrelated reasons.** `why`, `tropical` and `trust` converge
   because they are absorptive. `boolean` and `lineage` converge because their value lattices are
   finite and the iteration is monotone; they are NOT absorptive, and the payload says which
   reason applies to which semiring.

10. **A weight that breaks a semiring is refused.** Min-plus is absorptive only for non-negative
    weights. Max-min is a semiring on `[0, 1]` whose multiplicative identity is 1, so a weight
    above it makes a conjunction of premises come out smaller than the algebra says. Both are
    refused with the reason, rather than computed and quietly wrong. The second was found by a
    test that expected 5.0 and got 1.0.

11. **The truncation of why-provenance keeps the SMALLEST monomials.** A proper subset is strictly
    smaller, so a discarded monomial can never be a proper subset of a kept one and the survivors
    stay an antichain. Truncation still costs completeness, and it can cost minimality indirectly
    through an intermediate fact, so a truncated run sets `monomials_are_minimal_supports: false`
    and withdraws the claim.

## What this does not claim

Neither tool makes an ontology correct, and neither is a statement about OWL 2 RL. The rule table
is this engine's 29 of the profile's 78 rules, so a justification is a claim about what THIS engine
derives from the graph as loaded. A set that stops yielding the conclusion under this table may
still yield it under the full profile, and a conclusion this engine never reaches is not a
non-entailment result.

Nothing in `onto_provenance` is machine-checked. In `onto_justify` exactly one thing is, and only
when a certificate was written and the checker was actually run.

## Cost

One fixpoint per hitting-set-tree node, plus one per minimality check. On a large ontology that is
the entire cost of `onto_justify`, and it is why the bounds default low. The first justification is
free: it is a walk of the DAG with no re-run at all.
