# Explaining a conclusion: pinpointing and provenance

The reasoner has always been able to say WHAT it derived. Since
[decision 0002](decisions/0002-an-inference-carries-a-certificate.md) it can hand over a
certificate that a proved-sound Lean checker verifies, which says that each step is entailed.
Neither answers the question an engineer actually asks when a conclusion is wrong, which is
**which of my axioms did this**.

Two tools answer it, over one substrate.

- `onto_justify`, axiom pinpointing. The minimal sets of asserted triples responsible for a
  conclusion, or for a contradiction.
- `onto_provenance`, provenance semirings. The algebraic expression over the asserted triples
  that a derived triple carries.

## The substrate is the DAG, and it is not the certificate file

`derivations.tsv` records ONE step per inferred triple: the first the fixpoint reached. That is
exactly right for a certificate, because a checker re-derives and one derivation is all it needs.
It is exactly wrong for explanation. A triple derived two independent ways has two justifications
and the file shows one of them, so an explanation read off the file would be a confident, specific,
false statement about where a conclusion came from.

So `Reasoner::derivation_graph` runs the fixpoint and captures EVERY applicable ground rule
instance instead. The capture is opt-in and costs one branch per candidate triple on a run that did
not ask for it. The last round of the loop applies every rule over the complete closure and adds
nothing, so every instance applicable at the fixpoint fires at least once and is recorded;
deduplication is by (rule, conclusion, premises).

`tests/justify_test.rs::the_derivation_dag_carries_both_instances_where_a_certificate_carries_one`
is that difference, measured: two instances in the DAG, one line in the file, same graph.

## `onto_justify`

A justification (a MinA) for `T` is a subset `S` of the asserted graph with `T` in `closure(S)` and
`T` not in `closure(S')` for any proper subset. **The second half is the product.** A support set
that is not minimal blames axioms that had nothing to do with the conclusion, and someone who
deletes one of them and watches the conclusion survive learns to distrust the tool rather than the
ontology.

### Three claims, three words

| claim | how it is established | what stands behind it |
|---|---|---|
| sufficiency | the engine is run over exactly `S` and the conclusion appears | with `certificate_dir`, `OOCert.certificate_sound` via `lake exe oo-cert` |
| minimality | the engine is run again over `S \ {e}` for every `e`, and the conclusion must be gone | execution only. NO theorem |
| completeness of the list | Reiter's hitting-set tree over the same oracle | an algorithm, complete for a monotone oracle, bounded and flagged |

The certificate is worth spelling out. With `certificate_dir`, each justification gets its own
directory holding an `asserted.tsv` of exactly that justification's triples and a `derivations.tsv`
that concludes the target. `oo-cert` accepting it establishes that the conclusion really does
follow from that subset, under a machine-checked theorem. It establishes nothing about minimality
and nothing about the list being complete, and the payload says so in those words.

### Cost, and the bounds

The FIRST justification is nearly free: walk one derivation tree from the conclusion down to
asserted leaves, no re-run at all. It is not necessarily minimal, so it is shrunk by the oracle in
`|S|` re-runs, and it seeds the tree.

Everything after that is one full fixpoint per node. `max_justifications` (default 16) and
`max_oracle_calls` (default 400) bound it, `truncated` says when a bound fired and `truncation.bound`
says which. A truncated answer is not a wrong answer: every justification in it is still sufficient
and still minimal, both re-run. It is an incomplete one, and it says so.

### The monotonicity caveat, precisely

Reiter's construction is complete for a monotone oracle. The rule table is monotone, in that no rule reads
the ABSENCE of a triple, but two places in `src/reason.rs` are not. `owl:onProperty`,
`owl:someValuesFrom`, `owl:allValuesFrom`, `owl:hasValue`, `rdf:first` and `rdf:rest` are read into
maps keyed by the node, so a node carrying TWO values for one of them contributes one, chosen by
hash order. Removing a triple can therefore change which value is read rather than only removing
consequences.

Where that bites, the tree can MISS a justification. It cannot report a false one, because every
reported set is verified by re-running the engine over it and over each of its subsets.

### Inconsistency is the half that is worth more

An inconsistent ontology entails everything, so the useful question is never which conclusion to
doubt. It is which axioms to remove. `inconsistency: true` runs the same machinery with the target
being "this engine finds a clash".

The verdict word is `clash_found_by_this_engine`, exactly as `onto_reason` reports it, and never the
Lean checker's `unsatisfiable_under_disjointness`. Ten of the seventeen OWL 2 RL rules that conclude
`false` are looked for, so no clash found is not a consistency result and a justification here
explains a clash rather than unsatisfiability.

### Checking someone else's answer

Pass `candidate` and nothing is searched for. The set is checked, and the answer is one of

- `minimal_justification`: sufficient, and every element necessary. Both halves re-run.
- `not_a_justification_not_minimal`: with the removable triples named.
- `not_a_justification_target_not_reached`: a different failure, under a different word.

## `onto_provenance`

Green, Karvounarakis and Tannen's construction, applied to this engine's rule table rather than to
relational algebra: evaluate the fixpoint a second time with truth values replaced by elements of a
commutative semiring, and every derived triple carries an expression over the asserted ones.

| semiring | `⊕` | `⊗` | reads as |
|---|---|---|---|
| `boolean` | ∨ | ∧ | derivability |
| `why` | union then absorption | pairwise union | the leaf sets of the proof trees; the minimal ones are the justifications |
| `lineage` (`Which(X)`) | ∪ | ∪ | which triples contribute at all. NOT a justification |
| `counting` | + | × | the number of proof trees |
| `tropical` | min | + | the cheapest proof tree, summing leaf weights with multiplicity |
| `trust` | max | min | the confidence of the best derivation, which is that of its weakest premise |

`trust` is **max-min**, not min-plus, and the choice is named here because the two disagree about
which derivation is better and a reader is owed which one they are looking at. `tropical` is the
min-plus one and is offered beside it.

### Recursion, which is where provenance goes wrong quietly

Datalog is recursive. A triple whose support contains a cycle has arbitrarily many proof trees, so
its counting annotation DIVERGES and its how-provenance is an infinite series rather than a
polynomial. Producing a finite number anyway and not saying so is the failure this tool is built to
avoid.

- **The absorptive ones converge.** `why`, `tropical` and `trust` satisfy `a ⊕ (a ⊗ b) = a`, so a
  derivation that goes round a cycle is absorbed by the one that does not. The payload says
  `absorptive: true` and `stabilised: true`, and the round count is well under the bound.
- **`boolean` and `lineage` converge for a DIFFERENT reason**, which is that their value lattices
  are finite and the iteration is monotone in them. They are not absorptive, the payload says so,
  and neither reason is ever collapsed into the word "converges".
- **`counting` does not converge.** Round `k` counts proof trees of height at most `k`, so
  `depth_bound` (default 32) rides beside the number, `value_is_exact` is false unless the
  iteration stabilised on its own, `value_means` says the number is a LOWER BOUND, and
  `cycle_in_support` names a triple on the cycle. A 128-bit counter that overflows is reported as
  `saturated` rather than as a count.

`tests/provenance_test.rs::the_counting_semiring_reports_its_bound_instead_of_a_finite_lie` pins
two different bounds giving two different numbers on the same graph, which is what a bound has to
do to be a bound.

### The monomial cap, and why the survivors are still an antichain

Why-provenance is worst-case exponential, so `max_monomials` (default 64) caps it. The monomials
KEPT are the smallest. That is not a heuristic: a proper subset is strictly smaller, so a discarded
monomial can never be a proper subset of a kept one, and the survivors remain an antichain.

What truncation does cost is completeness, and it can cost minimality indirectly, because the
smaller monomial that would have absorbed a kept one may have been dropped at an INTERMEDIATE fact.
So a truncated run sets `monomials_are_minimal_supports: false` and withdraws the claim rather than
keeping it. Every monomial listed is still the leaf set of a real derivation.

### Weights are refused rather than bent

`tropical` is absorptive only for non-negative weights, since `min(a, a+b) = a` needs `b ≥ 0`; a
negative weight is refused. `trust` is a semiring on `[0, 1]` whose multiplicative identity is 1, so
a weight above 1 makes a conjunction of premises come out SMALLER than the algebra says; that is
refused too, and the message says to use `tropical` instead if the numbers are costs rather than
confidences.

## How the two relate

Feature 1 is feature 2 under the right semiring: the minimal elements of the why-provenance value
coincide with the MinAs, for an untruncated run that stabilised over a monotone rule table. The two
share the DAG index and the target parsing, and
`tests/provenance_test.rs::the_why_monomials_are_the_justifications_the_re_runs_verify`
asserts they return the same sets on the same graph.

They do not share the guarantee, and that is deliberate. `onto_provenance` does algebra over a DAG
and can truncate. `onto_justify` re-runs the engine, without each element of each answer, and
reports what those runs showed. When they disagree the re-runs win, which is why they exist.

## What neither tool claims

Neither makes an ontology correct, and neither is a statement about OWL 2 RL. The rule table is
this engine's 29 of the profile's 78 rules, so a justification is a claim about what THIS engine
derives from the graph as loaded. A set that stops yielding the conclusion under this table may
still yield it under the full profile, and a conclusion this engine never reaches is not a
non-entailment result.

Nothing in `onto_provenance` is machine-checked. In `onto_justify`, one thing is: that a
justification suffices, and only when a certificate was written and `oo-cert` was actually run.

A SUPPLIED Horn rule table is out of scope. `onto_reason` with `rules_file` runs a different
fixpoint (`Reasoner::run_horn`), the DAG capture is not threaded through it, and neither tool takes
a `rules_file`. Explaining a conclusion drawn under rules nobody has checked would also need a
fourth word, because the conclusion itself only ever earns `entailed_under_supplied_rules`.

`owl-dl` is refused by both, rather than answered with an empty DAG that would read as "nothing was
derived". The tableaux path records no rule applications.
