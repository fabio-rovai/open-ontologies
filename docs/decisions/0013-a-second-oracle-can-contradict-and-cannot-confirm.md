# 0013 · A second oracle can contradict, and cannot confirm

- **Status**: Implemented · cvc5 1.3.4 driven from `src/fol_solve.rs` as `Solver::Cvc5` over the
  SMT-LIB `src/tptp.rs` already emits, model ingestion in `src/fol_model.rs`'s `cvc5` module,
  differential in `tools/smt_differential.py`, gate in `tests/smt_second_oracle_test.rs`, installed
  by digest in the `lean` CI job
- **Written**: 2026-09-18
- **Related**: decision 0006 (a model is a certificate, and a refutation is not), whose line this
  decision does not move and exists to defend; decision 0005 (a prover is an oracle), whose
  disagreement discipline is reused verbatim; decision 0008, which is the same argument about two
  verified kernels rather than two unverified solvers

## The problem

Decision 0006 split the SAT and SMT family in half and was right to. A `sat` answer comes with a
finite structure, `lean/Fol/` checks it, and `Fol.satisfiable_of_check` turns the solver's opinion
into a proof of satisfiability. That half is defended by a machine-checked theorem, and it is
defended against a solver that is wrong, a solver that is lying and a solver that is broken in a way
nobody has thought of, because the checker re-evaluates the formulas against the structure and reads
nothing else.

The other half has no defence at all.

An `unsat` from Z3 is written into a report as `unsatisfiable_oracle` and there it stays. Nothing in
this repository can check it, by construction rather than by omission: replaying a CDCL(T) refutation
needs a verified theory solver for every theory involved plus a verified propositional core, which is
the research project decision 0005 declines. So the layer has one arm with a proof behind it and one
arm with a single vendor's word behind it, and until this decision the second arm had never been
questioned by anything.

That asymmetry is invisible in a green run, which is what makes it worth a decision record. Every
test of the SMT layer exercises the arm that is checked. A wrong `unsat` would pass all of them.

## The asymmetry, again, because it decides what a second solver is for

A second solver adds nothing to the `sat` arm. If Z3 says `sat` and the checker accepts the
structure, cvc5's opinion is of no interest whatsoever: the certificate does not rest on Z3 and would
not rest on cvc5. Running both and calling the agreement corroboration would be theatre.

A second solver is the **only** available evidence on the `unsat` arm. Not proof, and this record
says so as often as it can: two solvers agreeing that a theory has no model is two opinions. What it
can do is the one thing a single solver cannot do at any price, which is to be WRONG IN THE OPPOSITE
DIRECTION and say so. A contradiction between them is a fact, and it is a fact that implicates one of
them without saying which.

So the value of this work is entirely in the disagreement count, and a run that finds none has
established something much weaker than a reader will assume. The measurement section below is written
with that in mind.

## Decisions

1. **The verdict vocabulary does not grow, and this is the load-bearing decision.** Decision 0006
   fixes five words. Adding a second solver adds none. In particular there is deliberately no
   `unsatisfiable_corroborated`, no `unsat_by_two_solvers` and no confidence field, because every one
   of those would be a way of writing "more than an oracle said so" without having earned it. A
   report from a cvc5 run is the same five fields with the same five possible words, and the only
   thing that changes is which binary produced the output.

2. **A cvc5 `sat` reaches the certified word by exactly the route a Z3 `sat` does, and for a reason
   worth stating.** `Fol.satisfiable_of_check` is a theorem about a structure and a formula list. Its
   statement does not mention the producer, so a second producer needs no second soundness argument
   and gets no second word. `tests/smt_second_oracle_test.rs` runs the chain end to end and reads
   `"source":"cvc5"` out of the checker's own report.

3. **Both solvers are handed ONE file, written by ONE emitter.** `fol_solve::attempt` writes
   `problem_k{k}.smt2` with no solver in the name and `to_smtlib` is the only thing that writes it.
   If the two were ever given different bytes, a disagreement between them would be a fact about the
   emitter rather than about either solver, and that is the one thing a differential must not be able
   to measure.

4. **cvc5's one option is keyed on the ENCODING and never on the caller.** With default options cvc5
   answers `unknown` on the engine's quantified problems, because its default quantifier strategy is
   E-matching and E-matching is incomplete; `--finite-model-find` makes it answer. On the finite
   encoding that flag adds nothing, since `(declare-datatypes ((U 0)) ((e0) … ))` already fixes a
   finite carrier of known size and "look for a finite model" is precisely the question the file asks.
   On the unbounded encoding it would ask a different question, and the unbounded probe is the ONLY
   route to `unsatisfiable_oracle`. Passing it there would let a bounded search's answer be filed
   under the unbounded encoding's name, which is decision 0006 item 4's ten-minute mistake with a
   solver flag standing in for the cardinality constraint. `Solver::extra_args` therefore takes the
   encoding and not a preference, and a test asserts both halves.

5. **A model is ingested only on `sat`, and with cvc5 that is load-bearing rather than tidy.**
   Measured on cvc5 1.3.4: over a problem asserting `(forall ((X0 U)) (thing X0))` on a two-element
   datatype carrier, cvc5 answered `unknown` and then printed
   `(define-fun thing ((_arg_1 U)) Bool false)`, a structure that falsifies an asserted axiom. Z3 in
   the same position prints `(error "model is not available")`. A pipeline that read the block anyway
   would hand the checker something cvc5 never claimed was a model, the checker would reject it, and
   the run would report a `model_not_confirmed` STOP_THE_LINE against a solver that had done nothing
   wrong. A false stop-the-line costs as much credibility as a missed one.
   `cvc5_prints_a_structure_after_unknown_and_the_checker_rejects_it` runs cvc5 in exactly that
   configuration, ingests the block deliberately, and requires `oo-folmodel` to reject it.

6. **Provenance is rewritten, never inherited.** Both solvers print standard SMT-LIB `(get-model)`
   output, so one parser reads both; the differences measured are cosmetic, cvc5 naming its parameters
   `_arg_1` or `$x1` where Z3 uses `x!0`. What is NOT shared is the name. `FiniteModel::source` is
   `cvc5` on a cvc5 model and every `IngestError` arm is re-attributed by `IngestError::with_solver`,
   because a diagnostic reading "could not parse z3's model" over cvc5's bytes sends a reader to the
   wrong tool. Decision 0006 item 6 records `source` as untrusted and echoed; untrusted is not a
   licence to write the name of a solver that did not produce the file.

7. **The differential refuses to report a comparison it did not make.** `tools/smt_differential.py`
   exits 3, not 0, when no problem got a decisive answer from both solvers, and prints `NO_SIGNAL`. A
   run in which every row was `unknown` would otherwise print `CONTRADICTION 0` and read as a clean
   sweep. `ONE_SIDED` and `BOTH_UNKNOWN` are counted under their own names for the same reason and are
   never folded into agreement: one solver answering and the other giving up is a difference in
   completeness, not corroboration.

8. **The gate is known to be able to fail, because it has been made to.**
   `a_lying_solver_turns_the_differential_red` puts a shell script named `z3` on `PATH` that answers
   `unsat` to everything, and requires the differential to report a contradiction and exit non-zero.
   The stub says `unsat` rather than `sat` on purpose: a stub that said `sat` would be caught by the
   verified checker one step later anyway, whereas `unsat` is the answer nothing in this architecture
   can check, which makes it the exact failure a second solver is the only defence against. That is
   the argument for this whole decision in one test.

## What was measured

`tools/smt_differential.py --json` writes the run, `tools/smt_differential_report.py` turns it into
everything below the marker, and nothing below the marker was typed by hand. The corpus is every RDF
file under `case-studies/` that `git ls-files` reports, plus `tests/data/sample.ttl`, at
`finite(1)`, `finite(2)` and `unbounded`, with four goal triples per ontology taken from the
reasoner's own certificate.

<!-- MEASUREMENT -->

Measured on 65 ontologies producing **744 SMT-LIB problems**, each handed to both solvers at every encoding, with a 5 second limit per invocation.

* z3: `Z3 version 4.16.0 - 64 bit`
* cvc5: `cvc5 1.3.4 [git f3b21c4 on branch HEAD]`, pinned at 1.3.4

| outcome | problems | what it means |
|---|---|---|
| CONTRADICTION | 0 | one said `sat` and the other `unsat` on the same bytes. One of them is wrong |
| ERROR | 0 | a solver refused the file. A defect in the emitter until shown otherwise |
| ONE_SIDED | 70 | one answered and the other gave up. A difference in completeness, and NOT corroboration |
| BOTH_UNKNOWN | 2 | neither answered. No second opinion was obtained |
| AGREE | 672 | the same answer from both. Two opinions, and on the `unsat` rows that is all it will ever be |

The 672 agreements split 537 on `unsat` and 135 on `sat`. The first of those numbers is the whole point of the exercise: those 537 answers had, before this, been checked by nothing at all, and they still have not been PROVED by anything. What changed is that a second implementation was in a position to contradict them and did not.

The one-sided rows, by who answered:

* 68: z3 said sat, cvc5 said unknown
* 2: cvc5 said sat, z3 said unknown

By encoding:

| encoding | AGREE | ONE_SIDED | BOTH_UNKNOWN | ERROR | CONTRADICTION |
|---|---|---|---|---|---|
| finite(1) | 248 | 0 | 0 | 0 | 0 |
| finite(2) | 245 | 3 | 0 | 0 | 0 |
| unbounded | 179 | 67 | 2 | 0 | 0 |

`--finite-model-find` changed cvc5's answer on 137 of 744 problems: 137 from `unknown` to `sat`. It never turned one decisive answer into the other, which is the property that matters: the flag recovers completeness on problems cvc5 would otherwise abandon, and on this corpus it never changed what cvc5 concluded. That is a measurement and not a guarantee, and it is the reason the flag is still refused on the unbounded encoding, where no such measurement exists.

4 files produced no problems and are named rather than dropped:

* `case-studies/heritage-aerial/deliverables/07-templates/rights-statement-template.ttl`: the certificate named no triples, so there was nothing to ask
* `case-studies/heritage-aerial/registry/compliance-declaration-template.ttl`: the certificate named no triples, so there was nothing to ask
* `case-studies/heritage-aerial/registry/registry-shapes.ttl`: the certificate named no triples, so there was nothing to ask
* `case-studies/skills-england-occupational-maps/ontology/occupational-map.ttl`: 5.7 MB over the 4 MB cap

The run this section was generated from is committed at [`docs/measurements-smt-differential-2026-09-18.json`](../measurements-smt-differential-2026-09-18.json), and `tools/smt_differential_report.py` regenerates the section from it. Neither the table nor the sentences around it were typed by hand, which is the only way a record and the run behind it stay in step.

**Zero contradictions is a negative result and is reported as one.** It is not evidence that either solver is correct, and it is weaker evidence than it looks: the two are different programs, but they implement the same family of algorithms over the same standard, and on problems this exporter produces both are running an essentially mechanical search. The comparison that would be worth more is the one decision 0008 describes, between two independent formalisations of a specification, and it is a different kind of evidence rather than a larger amount of this one.

## The rows that looked like a capability gain, and were not

The table has rows where cvc5 decided a problem Z3 could not. That is the result in the run a reader
would naturally take as the practical argument for a second solver, so it was checked rather than
quoted, and it does not survive the check.

Both rows are the `finite(2)` encoding of `case-studies/foundry-owl-crosswalk/ontology/foundry-crosswalk.ttl`
and `case-studies/skills-england-occupational-maps/ontology/seom-vocabulary.ttl`, where Z3 hit the
five second limit and cvc5 answered `sat`. But the shipped pipeline never asks those problems. The
bounded ladder in `fol_solve::solve` starts at carrier 1 and STOPS at the first model, and on both
ontologies both solvers find one at carrier 1. Measured, running the real pipeline at a five second
and a thirty second limit, four runs per ontology:

| ontology | solver | verdict | encoding | seconds |
|---|---|---|---|---|
| foundry-crosswalk.ttl | z3 | `model_checked` | finite(1) | 0.09 |
| foundry-crosswalk.ttl | cvc5 | `model_checked` | finite(1) | 0.09 |
| seom-vocabulary.ttl | z3 | `model_checked` | finite(1) | 0.15 |
| seom-vocabulary.ttl | cvc5 | `model_checked` | finite(1) | 0.17 |

So the honest reading of the whole run is that cvc5 changed no verdict this engine would ever
produce, on any ontology in the corpus. It is a second opinion on the arm that had none, and on this
corpus that is the entirety of what it bought. The measurement is kept because the next corpus may
answer differently and because the number that matters is the one nobody has yet seen go non-zero.

## What this does not claim

1. **Not that the `unsat` answers are right.** Zero contradictions over this corpus means the two
   solvers were not caught disagreeing on it. It is not evidence that either is correct, and it is
   much weaker evidence than a reader's instinct suggests, for the reason in the next item.

2. **Not that agreement here is independent evidence.** Z3 and cvc5 are different programs with
   different authors, but they implement the same family of algorithms over the same standard, and on
   problems in the decidable fragment this engine emits they are both doing an essentially mechanical
   search. Two implementations of a well-understood decision procedure agreeing is much less
   surprising than two independent formalisations agreeing, which is the comparison decision 0008 is
   about and is a genuinely different kind of evidence.

3. **Nothing about unsatisfiability, in any direction, ever.** Unchanged from decision 0006 and
   restated here because a second solver is precisely the sort of addition that invites someone to
   relax it.

4. **Not that cvc5 is better or worse than Z3 at this.** The runs below use one option setting per
   encoding, one time limit, and one problem shape. A benchmark would need to vary all three and is
   not what this is.

5. **Not that the corpus is the interesting part of the space.** Every problem here comes from an
   OWL export, so they share a shape: one sort, unary and binary predicates, nullary constants, and
   quantifier nesting bounded by the translation. A disagreement, if one exists, is more likely to
   live in a shape this exporter cannot produce.

## Still not done

- **cvc5's proofs are not read.** cvc5 can emit proofs in several formats, including LFSC and Alethe,
  and unlike Z3's proof logs some of those are designed to be checked. That does not make them
  checkable HERE, because checking one still needs a mechanised calculus in core Lean, but it is a
  materially better starting point than anything the first-order provers offer and it is the obvious
  next question for the `unsat` arm. Nothing in this decision touches it.

- **The unbounded encoding's models are still not ingested, now for two solvers rather than one.**
  Z3 prints a universe block and cvc5 prints `(as @U_0 U)` qualified identifiers; neither is the
  enumeration the finite encoding declares. Decision 0006 listed this as open and it stays open.
  `an_ingestion_failure_names_the_solver_that_produced_the_output` pins the refusal so it stays a
  named `Unsupported` rather than becoming an approximation.

- **The differential is not run over the whole repository corpus in CI.** The `lean` job runs it over
  two ontologies, which takes seconds; a whole-corpus run takes hours on one machine, largely in the
  solvers rather than the engine. The run recorded above was done by hand. A scheduled workflow is
  the obvious home for the larger one and does not exist.

- **Mace4 is not in the differential.** It reads LADR rather than SMT-LIB, so it is not being handed
  the same bytes, and decision 0006's own rule is that the two solvers must read one file. A
  three-way comparison would need the comparison to be over the problem rather than over the file,
  which is a different tool with a different argument behind it.
