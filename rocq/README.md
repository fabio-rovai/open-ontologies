# `rocq/`: an independent third formalisation of the Horn certificate checker

This directory is a third formalisation of the Horn certificate checker and its model
theory, in Rocq 9.2 (the proof assistant formerly called Coq). It exists for the reason
`isabelle/` exists: a machine-checked proof rules out a bad argument and says nothing
about a bad definition, and a bad definition is invisible from inside a single
formalisation by construction. A second reading catches a definitional mistake only if the
second reading is genuinely a second one.

It is smaller than either of the other two and it says so throughout. What is covered and
what is not is in "Scope" below, and the boundary is stated in the negative, because a
third formalisation that overclaims is worse than no third formalisation.

## Build

Needs Rocq 9.2 and an OCaml compiler. Both come from `brew install rocq` on macOS
(`rocq` 9.2.0 with OCaml 5.5.0 as of this writing) and from `apt-get install rocq-prover`
or the `rocq-prover/rocq` Docker image elsewhere.

    rocq/build.sh

That regenerates the fixture data, compiles every theory, checks the axiom footprint and
links the extracted checker into `rocq/build/oo-horn-rocq`. It fails if any audited
theorem is not closed under the global context.

    rocq/build.sh --prove-the-gate-can-fail

That copies the tree, replaces one `Qed` with `Admitted`, runs the same build, and fails
if the build SUCCEEDS. A gate nobody has watched fail is decoration.

## Run the checker

    rocq/build/oo-horn-rocq check RULES.tsv ASSERTED.tsv HORN.tsv

Exit 0 accepted, 1 rejected, 2 unreadable or unparseable. One JSON object on stdout. The
CLI shape and the exit codes are `lean/HMain.lean`'s deliberately, so the differential
compares two checkers rather than two ways of being invoked.

## How independent is it, exactly

Less independent than `isabelle/`, and the difference matters enough to be the second
section of this file rather than a footnote.

**Read while this was written:** `docs/lean-certificates.md`, decision records 0002, 0003
and 0008, `docs/reasoning-systems-inventory.md`, `isabelle/README.md`, the committed
fixture bytes under `tests/fixtures/horn/`, and `lean/HMain.lean` for the command line,
the exit codes and the two verdict words.

**Not opened:** every file under `lean/OOCert/`, including `Horn.lean`, `HornBuiltin.lean`,
`HornParse.lean` and `Parse.lean`, and every `.thy` file under `isabelle/`. No definition,
no proof and no rule lemma from either was read. Both directories were LISTED, which is how
this paragraph can name the files it did not open, and a file name is not a definition.

**Known leakage, stated because it is unavoidable and because a reader should discount
for it:** decision 0008 quotes Lean's `substOf` and Isabelle's `distinct (map fst b)` in
its own text, so the shape of the two well-formedness conjuncts was known before
`Checker.v` was written, and `Determinacy.v` re-derives a property those records describe
rather than discovering it. That part of this development corroborates; it does not
discover. Everything else, the model theory, the premise discipline, the parser's
decisions and all twenty-seven built-in arms, was designed from the specifications and the
fixture data.

Nothing here should ever be edited to agree with `lean/` or with `isabelle/`. If the three
disagree, that is a finding and it belongs in a report before it belongs in a patch.

## Scope: what is covered

* **The rule-relative soundness theorem, end to end from the bytes.** `run_is_sound` in
  `Run.v` takes three file contents, and says that if the tool accepts then every
  conclusion of the parsed certificate is true in every world that satisfies the parsed
  graph and every parsed rule.
* **The absolute theorem for the built-in table.** `entails_of_builtin_horn` in
  `Builtin.v`: a certificate over exactly the built-in twenty-seven rows has conclusions
  true in every model of the asserted graph, with no rule assumed. Each of the
  twenty-seven arms is derived from the semantic conditions of `Interp.v` and each takes
  exactly the conditions it uses as explicit hypotheses.
* **What the two well-formedness conjuncts buy, and what they do not.**
  `horn_certificate_sound_without_wellformedness` proves the soundness theorem with both
  conjuncts deleted, so neither carries soundness content, and
  `wellformed_determines_instantiation` proves what they do carry. Both halves are refuted
  with their own hypothesis dropped.
* **Non-vacuity.** `Witness.v` exhibits a model of the conditions, exhibits a triple that
  is not absolutely entailed, and exhibits a one-line user rule table under which that
  same triple IS relatively entailed. The last of those is decision 0003's point as a
  theorem: the two verdicts are different claims.
* **The transcribed table is the committed one.** `Fixtures.v` parses
  `tests/fixtures/horn/builtin_rules.tsv` with this development's own parser inside the
  kernel and requires the result to equal the table `Builtin.v` proves things about.
* **Every shipped fixture's verdict, computed by the kernel.** Also in `Fixtures.v`, which
  is the only handle this development has on the extraction step.

## Scope: what is NOT covered

* **The four non-Horn rules.** `cls-int1`, `cls-int2`, `cls-uni` and `cls-oo` read an RDF
  list off the graph, so their premise count is data rather than fixed by the rule. They
  are outside the Horn family and outside this directory, as they are outside the Lean's
  Horn layer.
* **Everything else in `lean/`.** The mixed certificate layer, refutations, the
  description-logic model certificates, the first-order model checker, the SHACL
  evaluator, the institution and comorphism material: none of it is formalised here and
  none of it is even referred to.
* **That the conditions in `Interp.v` are a subset of the Recommendation's.** They are
  written from the RDF 1.1 and OWL 2 RDF-Based Semantics conditions and each carries the
  clause it is meant to be, but the correspondence is read cell by cell by a human and is
  not a theorem. That a subset is the safe direction is the reason it is allowed to be
  prose; a superset would not be.
* **The biconditional reading is a strengthening, and fourteen arms depend on it.**
  R-INT-5 in `Interp.v` is the warning. RDFS states its conditions as implications, and no
  implication in that direction licenses a rule that CONCLUDES a `subClassOf`,
  `subPropertyOf`, `domain` or `range` triple. The OWL 2 RDF-Based Semantics states the
  same conditions as biconditionals, which is stronger, which makes the model class
  smaller and the absolute claim weaker than it looks. The fourteen arms that consume a
  backward condition are named in `Builtin.v` and each one carries the hypothesis in its
  own statement, so the count is checked against the source text by
  `the_fourteen_arms_that_need_a_backward_condition_are_the_fourteen_named` rather than
  counted by hand. `R-BLT-3` carries the qualification that count needs: six other
  conditions are biconditionals too, and arms consume their reverse direction, but those
  are biconditional because that is what `someValuesFrom` and `sameAs` MEAN rather than
  because this development strengthened them. The fourteen is the count of arms resting on
  a reading stronger than the specification they come from, which is the number that
  matters, and it is not the count of arms that use an `iff`.
* **A total denotation.** R-INT-2: interpretations in which some literal fails to denote
  are outside the claim.
* **Extraction.** `Extract.v` uses no `Extract Inductive` and no `Extract Constant`, so
  the trusted glue is as small as it gets, but the extraction mechanism itself is trusted
  and is not verified. `Fixtures.v` computes the shipped verdicts inside the kernel and
  the differential runs the extracted binary over the same files, so a divergence between
  the two is visible; that is a check, not a proof.
* **The driver.** `driver/main.ml` is untrusted, as `isabelle/driver/` is and as
  `lean/HMain.lean` is. It opens files, converts at the boundary, prints JSON and returns
  an exit code.
* **Anything about the engine.** The theorem is conditional on the asserted graph being
  what the file says it is. `docs/trusted-computing-base.md` is where that boundary lives.

## Files

| file | what it holds |
|---|---|
| `theories/Syntax.v` | terms, triples, patterns, rules, steps. No semantics, no checking. |
| `theories/Semantics.v` | worlds, satisfaction, the rule-relative entailment relation. |
| `theories/Checker.v` | the executable checker. No semantics. |
| `theories/Determinacy.v` | what distinct keys and coverage buy, and refutations without each. |
| `theories/Sound.v` | `horn_certificate_sound`, and the same theorem with both conjuncts deleted. |
| `theories/Interp.v` | RDF-based interpretations and the seventeen semantic conditions. |
| `theories/Builtin.v` | the table as data, the twenty-seven arms, `entails_of_builtin_horn`. |
| `theories/Parse.v` | TSV bytes to datatypes, and the rule-table comparison. |
| `theories/Run.v` | the outcome type and `run_is_sound`, the end-to-end statement. |
| `theories/Witness.v` | non-vacuity, and the gap between the two verdicts. |
| `theories/Fixtures.v` | the checker run on the committed bytes, inside the kernel. |
| `theories/Audit.v` | `Print Assumptions` on every advertised theorem. |
| `theories/Extract.v` | extraction to OCaml, with no mapping of any inductive. |
| `theories/FixtureData.v` | GENERATED and gitignored. `gen_fixture_data.sh` writes it. |
| `driver/main.ml` | the untrusted driver. |
| `build.sh` | build, audit, link, and the mutation that proves the audit can fail. |

## Trust boundary

Inside: the Rocq kernel and the theories above.

Outside: the extraction mechanism, the OCaml compiler, file IO, splitting bytes,
JSON, exit codes, and the claim that `asserted.tsv` is the store it says it is.

## The differential

`tests/rocq_kernel_differential_test.rs` runs this checker and the Lean one over one
corpus built from `tests/fixtures/horn/` and systematic mutations of it, and classifies
every row. THE NUMBERS ARE NOT WRITTEN HERE. `cargo test --test
rocq_kernel_differential_test -- --nocapture` prints them and is the only place they
should be read from. What the comparison found, and what it means, is in
`docs/decisions/0015-a-blank-line-is-a-line-or-it-is-not.md`.
