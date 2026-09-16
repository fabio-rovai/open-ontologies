# 0006 · A model is a certificate, and a refutation is not

- **Status**: Implemented, both halves · `lean/Fol/` with `Fol.satisfiable_of_check`,
  `Fol.check_complete` and `Fol.not_entails_of_check` machine-checked, no `sorry`, no
  `native_decide`, no Mathlib · checker `oo-folmodel`, both targets in `defaultTargets` so the CI
  job's bare `lake build` compiles them · Rust half in `src/tptp.rs` (`sym`, `smtlib`, `ladr`,
  `checkfmt`), `src/fol_model.rs` (ingestion, two front ends) and `src/fol_solve.rs` (the
  certifying pipeline), reachable as `fol --format smtlib|ladr`, `fol-model` and `onto_fol_model`,
  with three CI legs over z3 and mace4
- **Written**: 2026-09-14
- **Related**: decision 0005 (a prover is an oracle, and a translation is a theorem), which this
  decision is the mirror of; decision 0003 (a rule is data), whose verdict discipline and whose
  sentence about digests are reused verbatim; decision 0002 (an inference carries a certificate)

## The problem

Five automated-reasoning families were the goal. Logic programming is `lean/OOCert/`, description
logics are `lean/Dl/` and `src/tableaux.rs`, first-order theorem proving is `src/tptp.rs` and
decision 0005. The SAT and SMT family was entirely absent: no SMT-LIB anywhere in the repository
and no solver called.

Decision 0005 settled the hard half of that family before it arrived. A superposition or CDCL(T)
refutation cannot be replayed in core Lean, so an automated prover's verdict is an oracle opinion
and never a certificate. Taken at face value that rules the whole family out, and taking it at face
value would have been wrong.

## The asymmetry, which is the whole reason this layer exists

A refutation and a model are not two answers of the same kind.

A refutation is a proof object in a calculus this repository does not implement. Checking one needs
verified unification, a verified ordering, verified redundancy criteria and verified theory
lemmas. That is a research project, and decision 0005 is right that nothing short of it earns the
word "certified".

A model is a finite object. Checking whether a formula holds in a finite structure is decidable,
the decision procedure is a recursive `Bool`-valued function, and proving it agrees with the
`Prop`-valued satisfaction relation is an induction over eleven constructors. `lean/Fol/Check.lean`
is 130 lines of definitions and proof, and `Fol.eval_iff` is the whole of it.

So one direction of the SAT/SMT family can be CERTIFIED while the other stays an ORACLE, and the
same is true of every model finder. That includes a dead toolchain's live half: Prover9 is
unmaintained and its refutations are uncheckable, but Mace4 ships in the same distribution, is a
finite model finder, and prints exactly the kind of object this layer can certify.

## Decisions

1. **The satisfiability direction is certified, the refutation direction is not, and they never
   share a word.** `Fol.satisfiable_of_check` turns an accepted structure into `Satisfiable Γ`.
   Nothing in `lean/Fol/` says anything about unsatisfiability in any direction, and the driver's
   own docstring says so in the negative space, in the shape `lean/DlMain.lean` uses.

2. **The headline is a NON-ENTAILMENT, because that is the sentence no prover can produce.**
   `Fol.not_entails_of_check` says that a checked model of `¬φ :: Γ` is a machine-checked proof of
   `¬ Entails Γ φ`, under the same `Entails` that `OwlLean.adequacy` is stated about. A prover
   answering "not a theorem" is guessing at worst and reporting a failed search at best. This is
   the one place where the model direction is strictly STRONGER than what an ATP can offer, rather
   than merely different, and it is why the layer is worth building at all.

3. **The producer is untrusted by construction.** The checker re-evaluates the original formulas
   against the structure and reads nothing else. It never trusts the solver's clausification, its
   Skolemisation, its preprocessing or its choice of encoding. Mace4 clausifies and Skolemises, so
   its models interpret symbols like `f1(_)` and `c1` that `Fol.FinModel` has no slot for; dropping
   them on the way in is taking the reduct of its structure and needs no lemma here, because the
   checker re-evaluates formulas that cannot mention a symbol the translation never emitted. The
   driver reports the dropped names and their count so the reduct is visible rather than silent.

4. **Four verdict words, and the fourth is the one nobody reaches for.** A report carries five
   fields and they are never collapsed:

   ```
   "solver_verdict":  "sat" | "unsat" | "unknown"          what the oracle said
   "encoding":        "unbounded" | "finite(k)"            what it was asked
   "checker_exit":    0 | 1 | 2 | null                     null means never run
   "verdict":         "model_checked"           CERTIFIED. names Fol.satisfiable_of_check
                    | "satisfiable_oracle"      sat, no checkable model
                    | "no_model_up_to_size_k"   bounded search exhausted. NOT unsatisfiability
                    | "unsatisfiable_oracle"    unsat on the UNBOUNDED encoding. Never more
                    | "unknown_oracle"          timeout, incomplete, gave up
   "owl_reading":     "not_entailed_under_unproved_translation" | null
   ```

   Three rules are mechanical. `model_checked` requires `checker_exit == 0`. `unsatisfiable_oracle`
   may be produced ONLY by a run whose emitted problem carried no cardinality constraint of any
   kind; any bounded run's `unsat` is `no_model_up_to_size_k`. `owl_reading` is non-null only when
   the checker itself reported `goal_negated_present` AND the verdict is `model_checked`, computed
   from what the checker read back rather than from the Rust side's intention.

   The first and third are no longer rules a reviewer has to enforce. `verdict` is a
   `verdict::FolVerdict` and `owl_reading` is a `verdict::OwlReading`, and the certified variant
   of each carries a `verdict::Certified` whose field is private to `src/verdict.rs`. The only
   function that returns one is `CheckerRun::accepted`, the only constructor of a `CheckerRun`
   spawns the checker, and `accepted` returns `None` on any non-zero exit. So a code path that
   has not run `oo-folmodel` cannot write either word: it is E0451, a privacy error, and
   `compile_fail` doctests in `src/verdict.rs` fail if that stops being true. The second rule is
   still enforced by reading, because "which question was asked" is a property of the SMT-LIB
   this side emitted and not of anything the checker hands back.

   `no_model_up_to_size_k` exists because the obvious wiring is wrong and takes ten minutes. A
   finite-domain encoding that returns `unsat` has established that no model of that size exists,
   which is not unsatisfiability and is often not even evidence of it. The formula
   `∀x∃y (r(x,y) ∧ x≠y)` is `unsat` at carrier 1 and `sat` at carrier 2; a driver that mapped the
   first to `unsatisfiable_oracle` would be reporting a falsehood from correct solver output.

5. **A solver's `sat` whose model the checker REJECTS is not a statement about the ontology.**
   Z3 can answer `sat` on quantified UF through model-based instantiation with a candidate model
   that `get-value` reports incompletely. The verdict for that is `satisfiable_oracle` with
   `checker_exit: 1` and a `model_not_confirmed` note. It is never `rejected` as though the
   ontology were at fault, and it is never `model_checked`.

6. **The coverage gate is about ATTRIBUTION and says so.** `Fol.covers` checks that every symbol
   the problem uses is declared by the model file. Soundness holds without it: `FinModel`'s three
   fields are total, an undeclared symbol is still interpreted, and `check` validates the resulting
   structure from scratch. What the gate buys is that the structure certified is the structure the
   solver described rather than the solver's completed with defaults. It is reported at exit 1 with
   `"reason":"undeclared_symbol"` and `"gate":"attribution"`, it is kept out of `Satisfiable`, out
   of every theorem, and out of the soundness statement. Dressing it up as soundness would be the
   laundering move this repository keeps finding in other people's work and once in its own.

7. **The digest binds the two files, and it identifies rather than commits.** `model.tsv` carries a
   digest of the problem the solver was given and the Lean recomputes it from `problem.tsv`. It is
   taken over the canonical re-serialisation of the PARSED formula list, not over the file bytes,
   so labels and spacing cannot forge a match and a relabelled `goal_negated` cannot digest the
   same. FNV-1a 64 is specified in `lean/Fol/Parse.lean` rather than taken from a library, because
   `String.hash` is an opaque extern with no specification a Rust writer could target. It is not a
   cryptographic hash; it exists so two implementations can be compared, not so one can be defended
   against someone who controls the file. That is decision 0003 item 5, in the same words, for the
   same reason.

8. **An empty problem is exit 2.** Every structure satisfies the empty formula list, so a
   certificate over one would report `model_checked` and say nothing. An exporter bug that dropped
   every formula would otherwise produce a clean green run, which is the exact shape decision 0005
   item 7 records happening inside this repository already, when the differential exported from a
   reasoned store and every conjecture became trivially entailed.

9. **The footprint is pinned at the MEASURED value, not at the habitual one.** Every result in
   `lean/Fol/` is `[propext, Quot.sound]`, and several are `[propext]` or free of axioms entirely.
   `Classical.choice` appears nowhere in the layer. Pinning the `[propext, Classical.choice,
   Quot.sound]` triple that the older layers carry would be looser than the truth and would stop
   catching anything. `lean/Dl/Check.lean` already pins `checkWF_iff` tighter than its neighbours,
   so this is the house habit rather than a new one.

## Why `Fol` does not reuse `Dl/Check.lean`

This looks like duplication and it is not, so the reasons are written down here rather than left
for someone to re-derive in six months and get wrong.

1. `Dl.Sat : Interp → Concept → Name → Prop` evaluates at ONE free variable. First-order
   satisfaction needs an environment `Nat → Dom`. Every clause of `Sat`, `sat` and `sat_iff`
   changes signature.
2. `Dl.Sat`'s `.ex r c` quantifies over `I.rext r x`, a successor LIST, not over the carrier.
   Different quantification domain, different recursion, different well-formedness obligation.
3. `Dl.Concept` has no equality constructor. `src/tptp.rs` emits `Form::Eq` for `SameAs`,
   `DifferentFrom`, `OneOf` and `HasVal`.
4. `Dl.Axiom` is a flat thirteen-constructor enumeration. `tptp::Form` is recursive with `Imp` and
   arbitrary binder nesting, and `Translation::background()` emits shapes with no `Dl.Axiom` at all.
5. **Copying `Dl.WellFormed` would be a defect rather than a shortcut.** `Dl` carries `cextInDom`
   and `rextInDom` because `∃R.C` ENUMERATES `rext`, so an out-of-carrier successor would really be
   evaluated. Here every quantifier ranges over the carrier TYPE and atoms are only tested, so
   importing those clauses would make `Fol.check_complete` reject genuine models. The
   well-formedness predicate is not shared; it is different mathematics.
6. `Dl.holds_iff` is thirteen axiom cases. The first-order analogue is eleven formula cases with an
   environment threaded through. No case transfers.

Against that, `lean/Dl/Count.lean`, the 119-line hand-rolled pigeonhole that is the hardest file in
that directory, has no first-order counterpart at all, because first-order logic has no counting
constructor. The new library is smaller than the one it does not reuse. What IS reused is the
shape: the `Interp`-style record, the hash-tables-behind-closures parser idiom, the driver's
exit-code convention and its explicitly unverified diagnostic pass, the witness file's
accept/reject/non-triviality triad including its short-bare-names trick, and the `#guard_msgs` plus
`#print axioms` pinning. `lean/Dl/` is the template, not the library.

## What this does not claim

1. **Nothing about unsatisfiability, in any direction, ever.**
2. **The absence of a finite model implies nothing.** SHIQ lacks the finite model property, so a
   satisfiable ontology can have only infinite models and will never receive a certificate here.
   That is a limitation of the method rather than a defect, and it is the honest counterweight to
   the asymmetry the first section is built on. A bounded search that finds nothing has established
   `no_model_up_to_size_k` and that is all.
3. **The OWL-level reading rides on two things this layer does not prove.** `OwlLean.adequacy`
   lives in the sibling project, and the Rust-to-Lean translation correspondence that decision 0005
   item 2 states is pinned by tests and NOT proved. The certified verdict is about the formulas in
   `problem.tsv`; the OWL sentence is strictly weaker and carries its own word,
   `not_entailed_under_unproved_translation`, which must never be shortened to "not entailed".
4. **That `Fol.Form`, `Fol.Struc` and `Fol.Entails` are `OwlLean.FOL`'s is a transcription, pinned
   by tests.** It is a far smaller claim than `src/tptp.rs`'s, being about twenty lines of
   inductive types and their semantics rather than a translation, and the two should not be allowed
   to sound equally load-bearing.
5. **The parser and the file formats are outside the theorem**, exactly as in `lean/Dl/Parse.lean`.
   A parse error is exit 2 and never a verdict in either direction.
6. **`source` and `cardinality_search` are untrusted.** They are what the file claims about its own
   provenance, echoed for a report. Nothing checks them.

## Two traps that are already measured, so that nobody has to find them twice

**A LADR variable letter in the symbol table.** LADR's default convention is that a name beginning
with `u`, `v`, `w`, `x`, `y` or `z` is a VARIABLE. A mangler that turns an IRI into `w0` therefore
turns a constant into a universally quantified variable. Measured on this machine with
LADR 2009-11A: the input `p0(w0). -p0(k0).` is echoed by Mace4 in its own `CLAUSES FOR SEARCH`
block as

```
formulas(mace4_clauses).
p0(x).
-p0(k0).
end_of_list.
```

and the search then runs until killed, because the mangled problem is unsatisfiable while the real
one is not. A time limit would report `unknown`, so the failure is silent. The same file with `c0`
in place of `w0` is echoed unchanged and returns a two-element model at once. The mangler must
therefore never emit an initial letter in `{u,v,w,x,y,z}`: `p0…` for unary, `r0…` for binary,
`c0…` for constants, and bound variables as `x0, x1, …` so that they still are variables under the
same convention. The echoed clause block should be parsed and compared, not assumed.

**Whitespace in a symbol.** `bare()` in `src/tptp.rs` strips the angle brackets before an IRI
reaches `Term::Const` or `P1::Cls`, so the symbols are raw IRIs and the whitespace refusal in the
checker-format writer is load-bearing rather than belt and braces. A symbol with a space in it
re-parses as a different formula and one with a tab in it re-parses as a different set of fields.
The 13 September 2026 audit found exactly this truncation class silently disabling any SHACL
constraint that mentioned a typed literal. The writer must return an error rather than emit such a
line, and the digest is the second line of defence: a file that splits differently on read-back
canonicalises differently and the digest mismatches.

## What the Lean half found while being built

`free` was written with `List.erase` on the binder cases, copied from the design prototype.
`List.erase` removes the FIRST occurrence only, so `free (∀x0. thing(x0) ∧ lit(x0))` came back as
`[0]` and every formula using a bound variable twice was classified as open. Nothing became
unsound, because a larger `free` only strengthens the hypothesis of `holds_congr`. What would have
happened is that `check_complete_closed` would have stopped applying to essentially every real
problem, and `oo-folmodel` would have reported `closed: false` and quoted the weaker rejection
sentence for ever, with no test failing. It was caught by a `by decide` in `lean/Fol/Witness.lean`
asserting that the witness problem is a set of sentences, and the fix is `filter` rather than
`erase`.

## What the Rust half found when it was first run end to end

**A defect in this repository's own export reporting, of the exact shape this project exists to
catch.** Running `fol-model --goals` over
`case-studies/blast-furnace-ironmaking/blast-furnace-ontology.ttl` returned a MACHINE-CHECKED
countermodel for five of the nine triples the OWL-RL reasoner derives there. `bf:Hanging` is
declared `owl:Class` and also carries `bf:hasSeverity bf:HighSeverity`; `bf:hasSeverity` has domain
`bf:ProcessState`; `rdfs2` fires and the engine derives `bf:Hanging rdf:type bf:ProcessState`. The
exporter, reading `bf:Hanging` as a class, dropped the assertion — correctly, because
`OwlLean/Syntax.lean` gives each entity kind its own type and `Sig.Ind` is disjoint from `Sig.Cls`,
so the punning is outside the fragment. What was wrong is that it dropped it SILENTLY:
`count_out_of_fragment` knew six named OWL constructs and nothing about punning, so the report said
`exports_a_weaker_axiom_set: false` over an export that was weaker than the graph. Fixed by
counting it as `assertion on a punned entity`, with the reason; the reading itself is unchanged, so
no expected string in `fol_translation_correspondence_test.rs` moved. Five punned assertions
reported, five certified non-entailments: the loop closes exactly.

**And one in the ingestion, caught by the pipeline's own stop-the-line block on its first run.**
Z3's `(define-fun NAME ((args)) SORT BODY)` has the RETURN SORT at index 3 and the body at 4. The
first parser took index 3 for the body and every ingestion failed with `z3's model uses "Bool"`.
Nothing was mis-certified, because the disagreement block is what fires when a model cannot be
turned into a structure, and the verdict was `satisfiable_oracle` with nothing checked. It is
recorded here because the fix added a sort check whose only job is to make that message name the
symbol.

**And a third, in the exit code.** `fol-model` exited non-zero on a stop-the-line only on the
direct CLI path. `src/batch.rs` decided the exit code from the presence of an `"error"` key, and a
stop-the-line is not an error — the command ran and answered — so `batch` exited 0 over it. Batch
is the mode every tool in `tools/` uses, so in the one place the gate has to bite it did not. Fixed
by reading the `stop_the_line` count as well, keyed on the field rather than on the command name so
a later command reporting the same thing inherits it. Measured after the fix: the same ontology
with a deliberately lying `z3` on `PATH` exits 1, and with the real `z3` exits 0.

## Still not done

- **The digest still identifies rather than commits.** FNV-1a is the right tool for comparing two
  implementations and the wrong tool for defending against someone who controls the file. It is now
  written twice, in `lean/Fol/Parse.lean` and in `tptp::checkfmt`, and the two agree on
  `4403d8aaa0c422f7` by test on one side and `#guard` on the other. Neither is a commitment.
- **The unbounded encoding's models are not ingested.** A `sat` on `declare-sort` is
  `satisfiable_oracle` with `checker_exit: null`, because Z3 answers there through model-based
  instantiation and prints a universe block rather than an enumeration. The bounded ladder is the
  route to a certificate and the unbounded probe is the route to `unsatisfiable_oracle`; joining
  them would need a reader for the universe form and a way to tell a complete model from a
  candidate one.
- **The ladder starts at 1 and stops at the first model, so the certificates are the SMALLEST
  models.** Measured over the shipped case studies, every one of them is satisfied at carrier 1,
  because none exports an axiom forcing two distinct elements. The certificate is real and the
  property is weak. A `--min-domain` would produce more interesting structures and has not been
  built.
- **The emitted file asks for `(get-model)` unconditionally**, so an `unsat` run leaves
  `(error "model is not available")` in the solver's captured output after the status line. It is
  harmless — `read_sat` is line-oriented and finds `unsat` on its own line first, and the exit code
  is not consulted for Z3 — but a reader of `z3_unbounded.out` sees an error next to a correct
  answer. A second file without the `get-model` line, or `(set-option :produce-models false)` on
  the probe, would remove it.
- **Three things are quadratic or repeated and none of them matters yet.** `Fol.covers` is
  O(|Γ| × |declarations|) in the Lean; the Rust `FolProblem::vocabulary()` is rebuilt on every call;
  and `fol_model::mace4::parse_entries` materialises the remaining block as a `String` once per
  entry, so reading a model with `e` entries copies O(e × block) characters. All three are fine at
  the measured sizes — the largest shipped fixture is 2441 formulas and the whole pipeline runs in
  0.47 s — and all three would want fixing an order of magnitude up.
- **`lean/Dl/Parse.lean` has an attribution bug of its own**, found while reading it for this work
  and NOT fixed here. `Tables.toInterp` sets `ind := fun a => t.imap.getD a ""` and its docstring
  claims the empty name "is in no domain". That is not guaranteed: `parseModel` accepts a `domain`
  line with an empty element name, at which point `"" ∈ I.dom`, `indInDom` passes, and an
  individual with no `ind` line silently denotes a real carrier element. It is a provenance defect
  and not an unsoundness, since `Dl.Satisfiable` is existential over `Interp` and the interpretation
  produced is a genuine one, so `satisfiable_of_checkModel` still holds. The fix is to refuse an
  empty element name in `parseModel`, and it needs its own test.
- **Decision 0002's reference to a missing record is resolved.** It cited a "decision 0004" that did not exist when this was written. One was written independently, under that number and with this title, by an agent working from the same base and unable to see this file. The two stated the same decision; this one is the accurate one and is cited from `src/`, `lean/` and the tests, so 0002 was repointed here and the duplicate removed rather than renumbering a dozen call sites.
