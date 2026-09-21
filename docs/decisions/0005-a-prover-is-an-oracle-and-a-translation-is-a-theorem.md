# 0005 · A prover is an oracle, and a translation is a theorem

- **Status**: implemented · `src/tptp.rs`, `fol --out DIR --format tptp|clif`, `onto_fol_export` ·
  correspondence pinned by `tests/fol_translation_correspondence_test.rs` (31 tests) ·
  differential in `tools/fol_differential.py`, run against E 3.2.5 · the translation mirrors
  `OwlLean/Translation.lean` in the sibling `owl-lean` project, whose `OwlLean.adequacy` is
  machine-checked with axioms `propext`, `Classical.choice`, `Quot.sound` · **the Rust-to-Lean
  correspondence is pinned by tests and is NOT proved**
- **Amended**: 2026-09-20 · the resolution half of the missing calculus now exists in core Lean
  (`lean/Fo`, `Fo.unsat_of_check`), a prover's derivation is translated into it
  (`tstp::to_fo_certificate`), and `fol --format cnf` keeps the prover from clausifying, so on
  the clausal fragment a refutation is CHECKED BY A THEOREM. Outside that fragment, and for
  equality, everything below still holds. See the second addendum.
- **Written**: 2026-09-14
- **Related**: decision 0002 (an inference carries a certificate), which this decision is the
  boundary of; `tools/shacl_differential.py`, whose treatment of pyshacl this copies exactly

## The problem

There is a problem on each side of this work, and they pull in opposite directions.

An OWL-to-first-order exporter is easy to write and hard to justify. FOWL
validated its mapping empirically over 168 consistent ChEBI modules. Hets proves its OWL-to-CASL
comorphism on paper, through the institution satisfaction condition. LATIN's `OWL2toFOL.elf` is
type-checked by Twelf, which is no longer maintained, and has its cardinality constructors and
`objectPropertyChain` commented out. So an exporter can emit a plausible file and nobody, including
its author, can say what the file means.

The opposite temptation arrives with the output. Once an ontology is in TPTP, a prover will answer questions
about it, and the answers look like proofs. They are not. Decision 0002 lets this engine say an
inference is *checked*, because `lean/` holds a checker whose soundness is a machine-checked
theorem and the certificate is small enough to re-derive. A superposition refutation is not like
that. Checking one needs a verified first-order calculus with unification, which does not exist in
core Lean. Treating a prover's `SZS status Theorem` as a certificate would undo the distinction
decision 0002 exists to draw, inside the same codebase, one layer up.

## Decisions

1. **The exporter emits the translation a machine-checked theorem is about.** The sibling project
   `owl-lean` proves `OwlLean.adequacy`: for an ontology `O` and an axiom `a`,
   `Entails O a ↔ FOL.Entails (background ++ indAxioms inds ++ O.map trAx) (trAx a)`, no `sorry`,
   no Mathlib, axioms `[propext, Classical.choice, Quot.sound]`. `src/tptp.rs` transcribes `tr`,
   `trAx`, `background` and `indAxioms` function for function. Nothing is normalised, simplified or
   optimised on the way out: `⊤` fillers still emit `$true` conjuncts, because the file has to
   carry the formula the theorem is about rather than one equivalent to it.
2. **The correspondence is pinned by tests and is not proved, and the output says so.**
   Nothing mechanically checks that `Translation::axiom` is `OwlLean.trAx`. What exists is
   `tests/fol_translation_correspondence_test.rs`, whose expected strings are computed BY HAND from
   `OwlLean/Translation.lean` rather than recorded from a run: a golden file captured from the
   emitter would agree with the emitter by construction and would catch nothing. The claim appears
   in the module docs, in the JSON report, and in the header of every emitted file, because a limit
   stated only in a README gets read without one.
3. **What the theorem needs is handled, not assumed.** `tr_bridge` holds only under
   `Fresh n x`, and `OwlLean.Refutations.tr_bridge_needs_freshness` is a machine-checked
   countermodel where the existential `tr` allocates captures its own subject variable. The
   exporter never chooses a counter: every entry goes through `Translation::concept_fresh`, which
   refuses unless `x < c`, and the call sites reproduce `trAx`'s own `tr c 0 2`, `tr c 1 2`,
   `tr c 0 1`. Separately, `background` alone does not force a constant to denote an object, and
   `OwlLean.Refutations.adequacy_needs_ind_axioms` refutes adequacy outright with the empty
   ontology and `⊤(a)`; `indAxioms`, i.e. `thing(a)` for every individual name, is the fix and is
   emitted. Both were real defects found while proving the theorem.
4. **What is not exported is named in the output.** `exports_a_weaker_axiom_set` and
   `constructs_not_exported` carry every construct outside the fragment with its count and the
   reason, the same shape as `certifies_a_weaker_axiom_set` on the description-logic layer. A
   construct rewritten into the fragment before translation, such as `owl:AllDisjointClasses` or
   `owl:equivalentProperty`, is listed under `reduced_to_fragment` rather than passed off as
   native. Annotations are counted separately, because an annotation carries no Direct Semantics
   content and folding 206 `rdfs:label` triples into the drop list would bury the sixteen
   constructs that do weaken the axiom set.
5. **An ATP verdict is an oracle opinion and is labelled as one everywhere it appears.**
   `tools/fol_differential.py` runs the engine's claimed entailments past E or Vampire and reports
   `AGREE`, `CLAIMED_NOT_ENTAILED`, `WEAKER_EXPORT`, `UNDETERMINED`, `ATP_ERROR` or `NOT_ASKED`. A
   disagreement is a bug in one of the two and the tool's job is to say so, not to adjudicate.
   `UNDETERMINED` is never collapsed into agreement: the translated theory is not decidable in
   general and a non-entailment often has only infinite countermodels. The word "proved" appears
   nowhere next to a prover's answer.
6. **A disagreement caused by the fragment is separated from a disagreement caused by a bug, and
   the separation is computed rather than guessed.** OWL TIME derives a class assertion from a
   data-property assertion, and GoodRelations derives a range from a data-property hierarchy;
   neither has a constructor in `OwlLean/Syntax.lean`, so the exported theory genuinely does not
   entail those conclusions and the prover is right to say so. Calling that a bug would send a
   reader hunting a defect that was a stated limitation. Every triple in the certificate is put to
   the exporter as a goal of its own, so the exporter's own judgement decides what is expressible;
   a conclusion is reachable when it is an expressible asserted triple or some derivation of it has
   every premise reachable; and a conclusion with no surviving derivation is reported as
   `WEAKER_EXPORT` with the blocking triple named. The verdict is only as good as that
   expressibility judgement, so the triple is printed rather than summarised.
7. **The differential exports from an unreasoned store, and checks that it did.** `reason`
   materialises into the default graph, so exporting from the same store puts the conclusion into
   the axioms and every conjecture becomes trivially entailed. That was this tool's first shape,
   and a deliberately broken exporter still scored a clean run under it. It now runs the reasoner
   and the exporter against separate stores, loads the certificate's own `asserted.tsv` for the
   export so the two see the same graph down to the blank node labels, and aborts if the triple
   counts differ rather than reporting a differential that cannot fail.
8. **Two serialisers, one translation.** `Form` is computed once; TPTP FOF and CLIF are folds over
   it and contain no OWL-specific logic. The correspondence test reads the CLIF back with an
   S-expression reader and requires the identical `Form`, so a drift in either writer is caught
   rather than argued about.
9. **The CLIF is restricted to the first-order-equivalent fragment, and says so in the file.**
   Common Logic is not plain first-order logic: it has sequence markers, arity-free predicates, and
   a universe in which relations are themselves individuals. ISO/IEC 24707 clause 6.5 is explicit
   that sequence markers make the logic non-compact and therefore not first-order. The adequacy
   theorem is about plain first-order logic, so the emitted text uses no sequence markers, fixed
   arity everywhere, and no quantification into a predicate position. `Form` cannot express any of
   the three; `clif_uses_only_fol_fragment` checks that the writer still does not.

## Why CLIF at all, given TPTP exists

Because they are not competing. TPTP is the execution format and CLIF is the interchange format,
and the toolchain confirms the split: Macleod translates CLIF to TPTP and to LADR and then calls
Vampire, Prover9, Paradox or Mace4 (`src/macleod/Commands.py` has exactly those four; E is not
among them). The consumer that matters here is standards-side, and it is a requirement rather than
a preference. **ISO/IEC 21838-1:2021 clause 4.3 requires that a top-level ontology be available
through an axiomatisation in a language conforming to ISO/IEC 24707**, its note naming CLIF, CGIF
and XCL as the qualifying dialects. Two qualifications travel with that and must not be dropped:
the English clause text was NOT read here, being paywalled, so the "shall be available" modality
comes from the Russian identical adoption of the same standard; and CLIF specifically is not
mandatory, since CGIF or XCL would satisfy clause 4.3 equally. CLIF is named for Basic Formal
Ontology in particular by ISO/IEC 21838-2 clause 4.4.1(a), which WAS read verbatim, and the ISO
maintenance portal ships `21838-2/common-logic/*.cl` alongside a Prover9 rendering and an OWL
approximation. BFO's own documentation states that the OWL version is an approximation to the CLIF
one and that an OWL axiom counts as valid just in case it is provable from the stronger
implementation. An ontology engine that can emit only TPTP cannot speak to that audience in its own
notation.

## CLIF was got wrong three times, and each time by assertion rather than measurement

Every correction below came from reading the standard or running a parser, and each replaced
something this project had simply assumed.

**Vertical bars.** Names were first written as `|...|`. That is Common Lisp's and KIF's convention.
A.2.2.2 sets `namequote = '"'`, and in CLIF the vertical bar is an ordinary name character, so
`|x|` lexes as a bare name containing two pipes and protects nothing. Names are now enclosed names
in double quotes, which A.2.2.4 recommends for IRIs.

**Double-quoted comment strings.** Comments were then written with double quotes, justified by
matching the CLIF files ISO hosts for ISO/IEC 21838-2. That corpus has been withdrawn by its own
maintainers: BFO's release notes of 7 December 2025 say "Comment texts are surrounded by single,
not double quotes". Counted, the ISO-hosted files carry 369 double-quoted `cl:comment` forms and
BFO master carries 356 single-quoted and none double-quoted. Worse, quote style had been bound to
operator spelling, so **no combination of flags could emit conforming CLIF**. The two are now
independent and comment strings are single-quoted in both dialects.

**Sentences wrapped in comments.** Every sentence was emitted as `(cl:comment '...' SENTENCE)`, the
shape BFO uses. Measured, both CLIF parsers that exist return an EMPTY theory from such a file:
py-typedlogic treats the form as discardable and Macleod has no production for it, and both do the
same to BFO's own files. A file that is formally valid and practically empty is the
assurance-laundering shape this project exists to attack, found inside this project's own output.
The default is now a standalone `(cl:comment '...')` phrase followed by a bare sentence, which
py-typedlogic reads back at exactly the sentence count the exporter reports; `--clif-comments
wrapped` keeps the old shape and the docs say what it costs. The text is also named now, because
all 227 COLORE texts are named and an unnamed one is refused by Macleod and misnamed by
py-typedlogic.

## Two dialects, because the ecosystem has two

`cl:text` and `cl:comment` are ISO/IEC 24707's reserved tokens and are what ISO publishes BFO in.
The COLORE repository is written with `cl-text` and `cl-comment`, and the Macleod parser
(`src/macleod/parsing/parser.py`) maps only the hyphen forms, with the colon forms present and
commented out. `--clif-dialect` therefore takes `iso` (the default) or `colore`. It is a spelling
and not a second translation: the correspondence test reads both dialects back to the identical
`Form`. It is **not** true that a file in one spelling fails in the other's tools, which this
record claimed until it was measured: py-typedlogic maps both spellings to identical results with
identical sentence counts. Macleod reads neither, because it cannot lex an IRI in a symbol
position at all.

## The subdialect claim is gated, not asserted

ISO/IEC 24707 first edition A.4.2: "The subdialect of CLIF which does not use numerals or quoted
strings is exactly semantically conformant". Staying inside it makes CLIF entailment and Common
Logic entailment coincide, so the adequacy theorem needs no qualification at the CLIF end. The
scope is stated exactly, in the header and in the docs: no decimal numerals and no quoted strings
IN SENTENCE POSITIONS, with comment annotations as the named exception, since a comment's text is a
quoted string by definition. `clif_stays_in_the_exactly_conformant_subdialect` walks every emitted
sentence and fails on either violation, and both failures have been demonstrated.

Nothing is claimed about the second edition's Annex A.3, which this project has not read, and
nothing anywhere says Common Logic requires infinite universes: abstract Common Logic requires only
non-emptiness, and the infinite-universe requirement is CLIF-specific, sits in the withdrawn first
edition, and sits awkwardly beside A.4.2 in that same edition.

## The quoting trap points opposite ways

TPTP takes single-quoted atoms for IRIs and CLIF takes double-quoted enclosed names, for the same
IRIs, for opposite reasons. In TPTP a double-quoted string is a *distinct object*, pairwise unequal
to every other; using it for IRIs would silently assert that all individuals are pairwise distinct,
which contradicts OWL's lack of a unique name assumption and would make every `owl:sameAs` export
unsound while still parsing and still proving things. In CLIF a single-quoted string is an
*interpreted name* that denotes itself, which is why A.2.2.4 recommends enclosed names for IRIs and
why using single quotes there would leave the exactly-conformant subdialect. Both traps are real
and they point opposite ways, so both are written down rather than left to a reader's intuition.

## What this does not claim

- **No ATP result is certified, in either direction.** `AGREE` means a second implementation
  reached the same conclusion. It is evidence, like pyshacl's agreement, and it is not a proof.
- **The Rust is not verified against the Lean.** Decision 0002's checker earns its verdict from a
  theorem. This layer earns its verdict from a test suite, and the difference is stated rather than
  blurred.
- **The front end is unclaimed on both sides.** `owl-lean`'s README lists OWL-file-to-abstract-syntax
  as not started: IRI resolution, the imports closure, the OWL 2 datatype map and facets, punning,
  blank node scoping. The reader in `src/tptp.rs` is that front end, and it is the part of this work
  that no theorem touches. Everything it cannot read, it names.
- **`hinds` is satisfied by construction, not weakened.** The theorem's hypothesis is
  `∀ a : S.Ind, a ∈ inds`, and the export's signature is the individual vocabulary occurring in the
  axioms and the goal, so the hypothesis holds for the signature the export defines. `owl-lean`
  lists weakening it to the occurring vocabulary as open; the exporter sits on the side of the gap
  where it is satisfied.

## Still not done

- The reader covers the common constructs and names the rest; it is not an OWL 2 conformance front
  end and does not claim to be.
- Cardinalities above 25 are dropped and named rather than expanded, because `minCard n` emits
  `n(n-1)/2` distinctness literals.
- Nothing runs the differential in CI. It needs a prover on `PATH`, skips loudly without one, and
  turns that skip into a failure under `FOL_DIFF_REQUIRE_ATP=1`; wiring a CI leg that installs E is
  not done. Nor is a CI leg that runs a CLIF parser over the export, which is what caught the
  empty-file defect.
- Nobody here has read ISO/IEC 24707:2018 or either part of ISO/IEC 21838. All three are priced at
  CHF 0, 70 pages for 24707, but downloading them needs a free ISO account, which is an owner
  action and not one to take on someone's behalf. The old ITTF free-standards site closed in 2025;
  the catalogue pages are reachable at `committee.iso.org` when `www.iso.org` returns 403. Until
  that is done, every clause cited here is scoped to the edition and the text actually read.

## Addendum, 15 September 2026 · The oracle's answer is now an object, and it is still an oracle

Nothing above is retracted. Item 5 stands word for word: an ATP verdict is an oracle opinion and
is labelled as one everywhere it appears. What has changed is that the opinion is no longer a
single word. Both installed provers will print the derivation they found, a derivation is a finite
object, and `src/tstp.rs` reads one back and re-checks what can honestly be re-checked. The
capability is `fol-prove`, `onto_fol_prove` and a second column in `tools/fol_differential.py`, and
the rest of this addendum is the boundary between what it now establishes and what it still
assumes.

### What is now evidence

1. **The prover refuted OUR problem.** Every leaf of the derivation is matched against the problem
   file the exporter wrote: by the name the prover's own `file(…,NAME)` annotation gives, by the
   PARSED formula, and by the ROLE. All three matter and none is a formality. The name catches a
   prover pointed at a stale or different file. The formula catches a file that has since been
   edited under the same names. The role catches a conjecture presented as an axiom, which would
   make the refutation say nothing about entailment while looking perfect. This is the cheapest of
   the three things below and by a distance the most valuable, because until it existed an `AGREE`
   row in the differential simply assumed it.
2. **The derivation is a well-founded DAG ending in the empty clause.** Every parent reference
   resolves to a node that exists, no name is used twice, the parent relation is acyclic, and the
   node nothing else cites is `$false`. Nodes the empty clause does not depend on are counted and
   excluded from every tally, because counting them would report work the refutation does not rest
   on.
3. **Some steps are recomputed.** `resolution`, `subsumption_resolution` and its forward and
   backward spellings, `factoring`, `duplicate_literal_removal`, `flattening`,
   `trivial_inequality_removal`, `equality_resolution`, and `negated_conjecture` /
   `assume_negation`. Each is replayed from its premises with a syntactic unifier with an occurs
   check, and the conclusion must agree up to a bijective renaming of variables.
4. **Whether the conjecture was used at all.** `what_was_refuted` is
   `axioms_and_the_negated_conjecture` or `axioms_alone`, and the second is not a success: it means
   the exported theory is inconsistent on its own, which is the `ContradictoryAxioms` case item 5's
   differential already had to separate by hand.

### What remains an oracle, in full

- **The calculus.** Even a derivation whose every step is recomputed is a derivation in a calculus
  whose soundness is written down nowhere in `lean/`. `Fol.satisfiable_of_check` has a theorem
  behind it; this has a Rust program behind it, and the difference is the same one decision 0006
  item 1 draws between the two directions.
- **The replayer itself.** `src/tstp.rs` is ordinary unverified Rust with ordinary tests. A defect
  in the unifier makes a wrong step look right. The tests below are what stands between that and a
  false green, and they are tests rather than a proof.
- **Clausification, and everything downstream of it.** `cnf_transformation`,
  `ennf_transformation`, `nnf_transformation`, `skolemize`, `distribute`,
  `true_and_false_elimination`, `variable_rename`, `shift_quantors`, `split_conjunct`, every
  AVATAR rule, and every SAT-solver step are NOT checked. They are named, counted and given a
  reason in `steps_unchecked`, and on a real proof they are the majority.
- **Introduced definitions.** AVATAR invents propositional symbols and prints them as
  `introduced(definition, …)` leaves. They are not from the problem and are not derived from it.
  Nothing here checks that the extension is conservative, so an introduced leaf is counted as
  unchecked and keeps the strongest word out of reach.
- **E's nested inference records.** E writes an inference record inline in a parent position, and
  such a record carries a rule and parents but NO FORMULA. Neither it nor the step it feeds can be
  replayed, and both are reported with that reason rather than with the generic one.
- **The SZS status line.** Echoed into `szs_status`, labelled untrusted in the report's own JSON,
  and used to decide nothing.

### The verdict vocabulary, and the rule it turns on

Eight words, and the ladder is the point:

```
problem_unparsed                    the TPTP problem could not be read
derivation_unparsed                 the output is not a TSTP derivation. Usually: no proof option
no_refutation_offered               no empty clause. A satisfiable problem, a timeout, a give-up
derivation_rejected                 THE CHECKER SAID NO. Dangling parent, cycle, or a leaf that is
                                    not a formula of the problem
refutation_step_not_reconstructed   a step whose rule IS implemented did not reconstruct
refutation_structure_checked        structure holds, NO step replayed
refutation_partially_replayed       structure holds, SOME steps replayed. The normal outcome
refutation_fully_replayed           structure holds, EVERY step replayed. STILL NOT unsatisfiable
```

**An unchecked step prevents the strongest word, and so does an introduced leaf.** That rule is
mechanical, it is the reason the ladder has three rungs instead of a boolean, and
`tests/tstp_derivation_test.rs::one_unchecked_step_prevents_the_strongest_word` takes a fully
replayed derivation, renames exactly one step's rule to `cnf_transformation`, changes nothing else,
and requires the verdict to drop. If that test ever passes without the drop, the vocabulary has
stopped meaning anything.

### A failed reconstruction gets its own word, and that is not a hedge

A step whose rule is implemented, whose premises and conclusion are clause-shaped, and which still
does not reconstruct is EITHER a defect in the derivation OR a gap in this checker. Reporting it as
"unchecked" would let a forged step hide behind a rule name. Reporting it as "rejected" would
accuse someone else's prover on this module's word alone. So it is
`refutation_step_not_reconstructed`, it names the step, it prints what was expected and what was
found, and it exits non-zero. That is item 5's own discipline, one layer down: the tool reports the
disagreement and does not adjudicate it.

### Subsumption resolution is checked as binary resolution, and that is exact rather than lax

Vampire prints `forward_subsumption_resolution` for a step whose premises are `C ∨ L` and
`D ∨ ¬L'` and whose conclusion is `C`, under `D'σ ⊆ C`. The binary resolvent of those premises is
`C ∪ D'σ`, and `D'σ ⊆ C` makes that exactly `C`. One check therefore covers both rules and neither
is weakened by it. The same identity is why the check still passes on AVATAR's printed clauses,
where the side premise's assertion literals are carried into the conclusion and the classical side
condition does not hold of the printed form.

### Measured, on the day it was written

FOAF, `owl-rl-ext`, 181 claimed entailments, one problem per entailment, exported from an
unreasoned store the way item 7 requires.

| prover | goals refuted | steps replayed | leaves matched | rejected | not reconstructed |
|---|---:|---:|---|---:|---:|
| Vampire 5.1.0 | 181 | 1279 of 3314 | 534, all `identical` | 0 | 0 |
| E 3.2.5 | 181 | 181 of 4644 | 534, all `alpha_equivalent` | 0 | 0 |

Every one of the 362 runs came back `refutation_partially_replayed`. The two provers differ by an
order of magnitude in how much is replayable and the reason is structural rather than a matter of
quality: Vampire prints each inference as its own annotated formula with a formula attached, while
E nests inference records inside parent positions and those carry no formula. The ontology-alone
run, with no conjecture, came back `no_refutation_offered` from both, which is what a consistent
ontology should produce and is the control that the checker is not simply saying yes.

`alpha_equivalent` against `identical` is not cosmetic. E renames the bound variables of every
axiom it reads, so a leaf check that demanded byte identity of the parsed AST would reject every E
proof of every problem. The three levels — `identical`, `alpha_equivalent`,
`associativity_normalised` — are reported separately so that how much normalisation a leaf needed
is visible rather than absorbed.

### What this does not change

- **`AGREE` still means what it meant.** A checked derivation adds that the prover was answering
  about the file we gave it. It does not make the answer a proof, and the differential's summary
  says so in the same paragraph as the new counts.
- **The model direction is still the only certified one.** Decision 0006 is unaffected in every
  particular. `model_checked` rests on `Fol.satisfiable_of_check`; nothing in this addendum rests
  on any theorem at all.
- **Nothing here touches the translation correspondence.** Item 2 stands: `src/tptp.rs` is a
  transcription of `OwlLean/Translation.lean` pinned by tests and not proved, and a checked
  derivation over the emitted file says nothing about whether the file is the right file to have
  emitted.

### Still not done, in this layer

- **Superposition and demodulation are not replayed.** They are the steps that do the work on any
  problem with equality, and replaying one needs a term ordering, which is a project rather than a
  function. Until then every equality-heavy proof will sit near the bottom of the ladder.
- **Clausification is not checked and there is no cheap way to check it.** The honest alternative
  is to ask the prover for the clause set and check that the clause set is equisatisfiable with the
  problem, which needs the same machinery.
- **Nothing runs this in CI.** It needs a prover on `PATH` and the differential skips loudly
  without one, exactly as item "Still not done" above already records for the differential itself.
  The recorded fixtures under `tests/fixtures/tstp/` run everywhere; the live run skips.
- **The variant check is backtracking with a budget.** A clause larger than the budget is reported
  as unchecked with that reason, which is honest and is also a place a forger could aim at. The
  budget is 200,000 pairings and no real step has come near it.

## Addendum, 20 September 2026 · The resolution half is a theorem, and the clausal fragment is certified end to end

The problem statement above says a superposition refutation cannot be certified because checking
one "needs a verified first-order calculus with unification, which does not exist in core Lean."
Half of that sentence was avoidable, and avoiding it is what changed.

### What was avoidable

A checker does not need an algorithm that FINDS a most general unifier. It needs to confirm that
a substitution it was HANDED makes two literals complementary, and that is an equality on terms.
The prover searches; the certificate carries the substitution it used; the checker applies it and
compares. Resolving on a unifier that is not most general is still sound, it merely proves less,
and proving less is the prover's problem. So there is no `mgu` in `lean/Fo` and no theorem about
one.

`Fo.unsat_of_check`: a refutation the checker accepts is a proof that the clause set it was handed
has no model, over any carrier, finite or infinite. Axioms `propext`, `Classical.choice`,
`Quot.sound`; no `sorry`; core Lean, no Mathlib, like everything else in `lean/`. Three choices in
it are load-bearing. Two substitutions per step, one per parent, which removes standardising apart
entirely. Covering rather than deletion for the remainder, because soundness needs no more and
exact deletion put list arithmetic in every proof. And a domain element carried in the structure,
because over an EMPTY carrier there are no environments, the empty clause is vacuously satisfied,
and every refutation would prove nothing.

`oo-resolution REFUTATION.cert` exits 0, 1, 2 on the usual convention and exit 1 names no theorem.
`Fo/Demo.lean` runs the textbook refutation of `∀x. P(x) → Q(x)`, `P(a)`, `¬Q(a)`, the smallest one
that actually needs unification, six forgeries that each fail for a different reason, and
`no_refutation_of_a_satisfiable_set`, a theorem over EVERY refutation rather than an example.

### A real prover's derivation is now checked by it

`tstp::to_fo_certificate` turns the part of a TSTP derivation that IS resolution into that
certificate. The rule NAME is a hint and the witness is the check: E calls every inference `spm`,
including ones that are plain binary resolution, and a translator keyed on names translated
nothing from E; every two-parent step is offered to the resolvent witness instead, and a step is
resolution exactly when a witness exists. What does not translate is named with its rule and
counted, never skipped, and a certificate that does not reach the empty clause is refused.

Measured, the day it was written, with Vampire 5.1.0 and E 3.2.5:

| input | translated | `oo-resolution` |
|---|---|---|
| Vampire, pure CNF, modus ponens | 2 of 2 | exit 0 |
| Vampire, pure CNF, four-link chain | 4 of 4 | exit 0 |
| Vampire, the same content as FOF | 0 of 8 (3 `cnf_transformation`, `ennf_transformation`, `flattening`, `negated_conjecture`) | refused |
| Vampire, with equality | 0 of 2 (`definition_unfolding`) | refused |
| E, pure CNF, four-link chain | 2 of 3 | refused |

E's last step is `cn(rw(spm(c_0_13, c_0_14), c_0_15))`, three inferences in one node with NO
intermediate formulas printed. That is the `Parent::Inline` case the first addendum already calls
unreconstructable; it is a limit of E's output and not of the translation.

### Clausification was the gap, and the fix is not to clausify better

A prover handed `fof` clausifies before it resolves, and clausification is not resolution. The
FOF row above is the whole problem in one line. The fix is to stop making the prover do it, and to
say exactly when that is free. Measured over three exported ontologies, 1,905 formulas, an
existential occurs in exactly two shapes and they are nothing alike:

- `? [X0] : thing(X0)`, the non-empty-domain axiom: one per export, CLOSED, witnessed by a single
  constant.
- `someValuesFrom` in SUPERCLASS position, `A ⊑ ∃r.B`: 126 of 426 formulas in an OWL DL ontology
  (pizza), and ZERO in both OWL 2 RL ones (1,422 of 1,423 formulas in `ies-building-extension`
  have no existential at all).

That zero is not luck: OWL 2 RL forbids an existential restriction in superclass position, because
it is not clausal. And a third shape only the refutation revealed: a NEGATED universal conjecture
is itself a closed existential, `¬∀X.(A ⇒ B)` being `∃X.(A ∧ ¬B)`; it is the one `skolemize` in
Vampire's FOF proof.

So the line is **constant versus function**. `fol --format cnf` strips a closed existential prefix
with one fresh constant per variable, `$domain_witness` and `$sk_<goal>_X<n>`, and the argument is
one sentence: any model interprets the constant as the witness. An existential UNDER a universal
needs a function of that variable, nothing here proves that Skolemisation, and the export REFUSES
it by name rather than clausifying it:

```
this ontology is outside the clausal fragment: 1 of its 1424 formulas need a Skolem FUNCTION
(owl_2_subClassOf), an existential under a universal. ... Export as `tptp` instead; a prover's
refutation of it is an oracle opinion, and `cnf` will not pretend otherwise.
```

On the fragment, the chain closes. `ies-building-extension.ttl`, 1,423 axioms; goal
`Building ⊑ ies:Entity`, two subclass steps deep, so derived and not asserted; `--format cnf`;
Vampire, 5 steps, all resolution; `to_fo_certificate`, 5 of 5; `oo-resolution` exit 0,
`Fo.unsat_of_check`. The FOF encoding of the SAME goal is 18 steps, 13 of them clausification, and
translates nothing. Vampire says `Unsatisfiable` of the one and `Theorem` of the other, one verdict
in two SZS dialects. `tests/cnf_end_to_end_test.rs` is that run and its control.

### The rule, amended rather than reversed

Decision 5 above said an ATP verdict is an oracle opinion everywhere it appears. It now reads:

> An ATP verdict is an oracle opinion, **unless** the problem reached the prover as clauses
> (`onto_fol_prove` tries `to_cnf` first and falls back to FOF only outside the clausal fragment),
> every step of the derivation translated, and `oo-resolution` exited 0. In that case, and only that
> case, the verdict is **`refutation_certified`**, resting on `Fo.unsat_of_check`, with the theorem's
> name read from the checker's own stdout. Otherwise the verdict is one of the first addendum's
> eight words, and the report's `certificate` field says which of five things stopped it:
> `outside_clausal_fragment` (naming the axiom), `steps_untranslated` (naming the rules),
> `does_not_reach_false`, `checker_absent`, or `checker_refused`.

`refutation_certified` is the ninth word and the only one that rests on a theorem.
`refutation_fully_replayed` is still the strongest word the Rust replayer can earn and is still not
a certificate. The ninth word is set only when a `Certified` token exists, and that token is minted
only by `CheckerRun::accepted_naming`, so it cannot be printed by any path that did not run the
checker and read `Fo.unsat_of_check` back from it. `unsatisfiable`, the word `oo-resolution` and
`oo-lrat` print for themselves, joined `CHECKER_OWNED_WORDS`: Rust never says it. `problem_form` in
the run report says whether the prover got `cnf` or `fof`, and no user action selects between them.

### What remains an oracle, in full

- **Equality reasoning.** Paramodulation, demodulation, `definition_unfolding`. `Fo` has no
  equality; the next piece is to interpret the equality predicate as Lean's `=` on the carrier, at
  which point congruence is free and paramodulation's soundness is `subst`. `owl:sameAs` makes
  this unavoidable for real data.
- **Superclass existentials**, and with them every OWL DL ontology. The export refuses them; the
  FOF path and the first addendum's replayer are what they get.
- **Compressed derivations**, E's inline inferences. Prefer Vampire's format.
- **The TSTP parser and the translator**, which are Rust. What is proved is that the CERTIFICATE
  is a proof of the clause set in its leaves. That those leaves are the exported clauses is the
  first addendum's leaf-match check, still Rust and still pinned by tests rather than proved. A
  wrong translation cannot make an accepted certificate unsound; it can only fail to produce one.
- **The correspondence between `src/tptp.rs` and `OwlLean.adequacy`**, exactly as before.

### Measured, in the negative

The first version of the translator keyed on rule names and translated 0 of E's steps; the first
propositional checker consumed hints in order and failed 40 of 53 real picosat traces; the first
CNF export refused every goal because a negated universal is a closed existential. Each is a
test now. None of them could have made an accepted certificate unsound, since soundness is the
theorem's and not the pipeline's; each of them would have made the checker refuse correct proofs,
and a checker that refuses correct proofs teaches its users to stop running it.
