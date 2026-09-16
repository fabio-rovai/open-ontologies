# 0009 · A translation between logics carries its satisfaction condition

- **Status**: implemented · `lean/OOCert/Institution.lean`, `lean/OOCert/Comorphism.lean`,
  `lean/OOCert/InstitutionRdf.lean`, `lean/OOCert/InstitutionFol.lean`,
  `lean/OOCert/InstitutionWitness.lean`, root `lean/Institution.lean`, lakefile target
  `Institution` · three institutions, two comorphisms, twenty-one pinned axiom footprints,
  no `sorry`, no `native_decide`, no Mathlib, no new package · `lake build` 105 jobs → 112
- **Written**: 2026-09-15
- **Related**: decision 0005 (a prover is an oracle and a translation is a theorem), whose second
  half this is; decision 0002 (an inference carries a certificate); decision 0003 (the verdict
  rule), which this does not weaken

## The problem

This repository exports OWL to first-order logic and says, correctly, that the correspondence
between the Rust emitter and the Lean translation is pinned by tests and not proved. It also has
exactly one machine-checked statement of the form "this translation preserves meaning":
`OwlLean.adequacy`, in the sibling `owl-lean` package. That theorem stands alone. Nothing in this
repository abstracts over it, so there is no way to state what a SECOND translation would have to
prove, and no way to state what follows from having proved it.

The discipline for that abstraction has existed since 1992 and is Goguen and Burstall's
institutions, with translations between logics as comorphisms. The Heterogeneous Tool Set is the
engineering that goes with it. Hets is a large Haskell program in which a comorphism is a type-class
instance supplying a signature map, a sentence map and a model map; the satisfaction condition that
makes those three a translation of logics is discharged in the literature rather than by the
program. Decision 0005 already put it that way: "Hets proves its OWL-to-CASL comorphism on paper."
Nobody on this branch has read the Hets source, so that is a statement about its published design
and not a code review.

Reimplementing Hets buys nothing, because the value of Hets is its breadth and breadth is exactly
what a small verified core cannot have. Writing another unverified translation buys less than
nothing, because it is the shape this project exists to attack. What is left is the narrow move:
take the ONE part that carries the semantic weight and machine-check it for a small number of
logics.

## Decisions

1. **An institution is a structure and its satisfaction condition is a field.**
   `OOCert.Institution` carries `Sig`, `SigMor`, `Sen`, `Mod`, `senMap`, `modRed`, `sat` and
   `satCond`. The last says that reducing a model along a signature morphism and translating a
   sentence along it agree. It is a proof obligation of the constructor, so there is no institution
   in this development whose satisfaction condition was assumed. Four separate universe parameters,
   because `OOCert.Interp` and `Fol.Struc` carry a `Type` field and live in `Type 1` while a
   signature here is a list of strings.

2. **A comorphism is a structure and ITS satisfaction condition is a field.** `OOCert.Comorphism`
   carries `sigMap`, `senMap`, `modMap` and `satCond`. `Comorphism.mk` does not accept three maps.
   An unproved translation in this layer is a type error at the point of construction, not a lint
   warning, not a `TODO` and not a documentation gap. That sentence is the whole point of the file
   and everything else in it is downstream of it.

3. **Preservation and reflection are two theorems and they are kept two.**
   `Comorphism.preserves_entailment` has no hypothesis beyond the comorphism: the satisfaction
   condition is the entire proof. `Comorphism.reflects_entailment` takes `ModelExpansive`, and
   `Comorphism.borrowing` is the biconditional the two give together. Conflating them is how a
   translation gets sold as a decision procedure, so they are separated in the types and the
   separation is MEASURED rather than warned about:
   `InstitutionWitness.preservation_does_not_give_reflection` exhibits an entailment of the target
   whose source counterpart is refuted by a Herbrand countermodel, and
   `forgetting_the_conditions_is_not_model_expansive` turns that into a refutation of the
   HYPOTHESIS rather than only of the conclusion.

4. **`ModelExpansive` is stated in its weakest form.** The textbook hypothesis is that the model map
   is surjective. What the proof consumes is weaker: every source model is indistinguishable BY
   SENTENCES from the reduct of some target model. `ModelExpansive.of_surjective` proves the strong
   form implies the weak one, so a caller holding surjectivity loses nothing and nothing downstream
   is tempted to treat the two as the same.

5. **The weakest-conditions discipline applies here and it points the OPPOSITE way from
   `Semantics.lean`, and that had to be worked out rather than copied.** `Semantics.lean` warns that
   a condition stronger than the specification shrinks the model class and silently weakens every
   downstream theorem. A satisfaction condition is a `∀` over morphisms, over models and over
   sentences, so in this layer:
   - more signature morphisms make a STRONGER institution, and every constraint added to `SigMor`
     deletes instances of the law;
   - more sentences make a STRONGER institution, so the textbook restriction of `Sen S` to formulas
     "over `S`" is a WEAKENING and is done only where a downstream comorphism cannot hold without
     it;
   - `Rdf.Sen` therefore constrains the PREDICATE position of a triple and leaves subject and object
     free, because only the predicate's denotation is what the relevant condition turns on;
   - `FolInst.Sen` and `FolInst.Mod` are unrestricted, and the file says at the field that the
     signature constrains nothing, rather than dressing up a `Struc` that interprets every symbol as
     if it did not.

6. **The strengthening is quarantined, as in `W3C.lean`.** Category laws and functoriality are true
   of these instances and are consumed by no theorem here, so they are not fields of `Institution`.
   They live in `Institution.Functorial`, whose `toInstitution` is a FIELD PROJECTION, so every
   theorem about `Institution` applies to a `Functorial` verbatim:
   `functorial_preserves_entailment` is the general theorem applied and not a re-proof.
   `InstitutionWitness.simpleFunctorial` discharges the laws for the RDF instance, so the
   strengthened class is not empty and the omission is discipline rather than concealment.

7. **Three instances, all out of material this repository already had.**
   - `Rdf.simple`: simple RDF interpretations (`OOCert.Interp`, no semantic conditions). A morphism
     is a renaming constrained only by having to take the source vocabulary into the target one,
     which is what `senMap` needs; nothing else, because there is no semantic condition here for a
     renaming to break.
   - `Rdf.rl`: the OWL 2 RL interpretations, `OOCert.Conditions`. Morphisms must fix the reserved
     vocabulary, and `Rdf.conditions_reduct` is the theorem that pays for it: all twenty-three
     condition fields survive a renaming that fixes the fifteen terms the conditions name. That is
     the content of "satisfaction is invariant under change of notation" at this layer, and it is
     also the reason the two institutions have DIFFERENT morphism types: the logical vocabulary of a
     logic is whatever its semantic conditions pin down.
   - `FolInst.folInst`: first-order structures from `Fol/Semantics.lean`, with the satisfaction
     condition for renamings of the three symbol spaces proved by induction over the formula.

8. **Two comorphisms, both with the condition proved.** `Rdf.forgetConditions` goes from simple RDF
   to OWL 2 RL, and is the one that preserves and provably does not reflect. `FolInst.rdfToFol` is
   the translation proper: a triple becomes a binary atom and a first-order model becomes an RDF
   interpretation.

9. **The RDF-to-first-order comorphism is THEOROIDAL, and the obvious plain one does not go
   through.** `OOCert.Interp.iext` is keyed by the DENOTATION of the predicate and `Fol.Struc.p2`
   is keyed by the SYMBOL. Two symbols may denote one element and carry different extensions, and
   then no ternary relation on the domain reproduces both, so the obvious model map cannot be
   defined on arbitrary target structures. **That is an argument about the obvious construction and
   not a theorem**: nothing quantifies over all possible comorphisms, so nothing here rules out a
   plain one reached by another route, and neither this record nor the Lean claims otherwise. The
   fix is
   the one `src/tptp.rs` already ships: the target of the signature map is a THEORY, not a
   signature. `Institution.Th` builds the institution of theories over any institution,
   `Comorphism.Theoroidal` is a comorphism into it, `FolInst.cohAx` is the background axiom, and
   `owl-lean`'s adequacy theorem has the same shape with `background ++ indAxioms inds`. Nothing new
   was invented to make this work; what is new is that the obligation is discharged.

10. **Hets is not a dependency and will not become one.** It is a Haskell program; nothing in this
    repository's CI builds Haskell, and adding it would put a large unverified tool in the trust
    surface to obtain a satisfaction condition that it does not machine-check anyway. The Lean side
    takes no external package at all, by the rule at the top of `lean/lakefile.toml`. What is taken
    from Hets is the IDEA, which is public, and the idea is the only part that carries weight.

11. **The `owl-lean` package is not a dependency either, and this is not `OwlLean.adequacy`.** The
    sibling development proves a biconditional about the OWL 2 abstract syntax with background
    axioms, individual typing axioms and a freshness discipline. It is a separate Lake package, and
    importing it would put a second development inside this one's trust surface for no gain that
    could not be had by re-proving the part that is needed. What is here is the same SHAPE at the
    RDF triple level, over the two model classes this repository already carries, with its own
    satisfaction condition proved from scratch. The relationship is a family resemblance and is
    written here rather than left for a reader to infer as a dependency.

12. **DOL parsing is deferred, deliberately, until there is more than one proved comorphism to
    distribute over.** The Distributed Ontology, Modeling and Specification Language is a syntax for
    saying "interpret this specification into that one along this comorphism". A parser for it,
    written today, would have two comorphisms to name, one of which is a forgetful map between two
    readings of RDF. It would be a front end onto a table with two rows, and every hour spent on it
    would be an hour not spent making the table longer. The order is: prove translations, then build
    the notation that distributes over them. A DOL parser is a later, separate task and is worth
    nothing before this lands.

## What is proved, exactly

| statement | where | axioms |
|---|---|---|
| a comorphism preserves entailment | `Comorphism.preserves_entailment` | none |
| the same, premises as a list | `Comorphism.preserves_entailmentL` | `propext`, `Quot.sound` |
| a model-expansive comorphism reflects entailment | `Comorphism.reflects_entailment` | none |
| borrowing, as a biconditional | `Comorphism.borrowing` | none |
| surjectivity implies model expansiveness | `Comorphism.ModelExpansive.of_surjective` | none |
| theoroidal preservation, unfolded | `Comorphism.preserves_entailment_theoroidal` | none |
| the quarantine costs nothing | `functorial_preserves_entailment` | none |
| the 23 OWL 2 RL conditions survive a vocabulary-fixing renaming | `Rdf.conditions_reduct` | none |
| an `Rdf.rl` entailment is an `OOCert.Entails` | `Rdf.rl_entails_gives_entails` | `propext`, `Quot.sound` |
| simple RDF entailment is OWL 2 RL entailment | `Rdf.simple_entailment_is_rl_entailment` | `propext`, `Quot.sound` |
| first-order satisfaction is invariant under symbol renaming | `FolInst.holds_red` | none |
| `Fol.Entails` gives institution entailment | `FolInst.fol_entails_gives_institution_entails` | none |
| an RDF entailment transfers to first-order logic | `FolInst.rdf_entailment_transfers_to_fol` | none |
| preservation does not give reflection | `InstitutionWitness.preservation_does_not_give_reflection` | `propext` |
| and so that comorphism is not model-expansive | `InstitutionWitness.forgetting_the_conditions_is_not_model_expansive` | `propext` |
| the checker entails strictly more than the institution | `InstitutionWitness.the_checker_entails_more_than_the_institution` | `propext` |
| institution entailment is not `Fol.Entails` on open formulas | `InstitutionWitness.fol_does_not_entail_the_universal_closure` | none |
| three pairwise distinct `Rdf.rl` models | `InstitutionWitness.rl_models_are_pairwise_distinct` | `propext`, `Quot.sound` |
| two distinct `Fol.Struc` models | `InstitutionWitness.fol_models_are_distinct` | none |
| two distinct models of the background theory | `InstitutionWitness.coh_theory_models_are_distinct` | none |
| a translated premise is first-order entailed, end to end | `InstitutionWitness.the_translation_of_a_premise_is_first_order_entailed` | none |

Every footprint above is a subset of `propext`, `Classical.choice`, `Quot.sound`, and in fact no
statement in this layer uses `Classical.choice`. Each is pinned with `#guard_msgs` so that a change
to any of them fails `lake build` rather than appearing in a report.

## What is NOT claimed, and two of these are refutations rather than gaps

- **The Rust exporter is still not verified against the Lean.** `src/tptp.rs` and its translation
  are outside this work entirely. Decision 0005 item 2 stands unchanged: the correspondence is
  pinned by hand-computed tests and is not proved. `FolInst.rdfToFol` is a comorphism between two
  LEAN developments and is not a statement about the emitter.
- **No borrowing for the first-order comorphism.** `FolInst.rdfToFol` is not proved model-expansive
  and is not claimed to be, so `Comorphism.reflects_entailment` does not apply to it. That is
  decision 0005 item 5 restated as a missing theorem rather than as a caution: a first-order
  prover's failure to find a proof is still not an RDF non-entailment, and now there is a named
  theorem whose absence says so.
- **No naturality.** A comorphism in the literature is also natural in the signature morphisms.
  `OOCert.Comorphism` carries the satisfaction condition and nothing else, because no theorem here
  consumes naturality. Anything that composes comorphisms across signature morphisms will need it,
  and it is not there.
- **`OOCert.Entails` is NOT institution entailment, and this is now refuted rather than merely
  unclaimed.** `OOCert.Model` carries four conditions (`int`, `int2`, `uni`, `oneOf`) stated over
  `Chain G`, the RDF list read off the ASSERTED GRAPH. A graph-relative condition is not a sentence
  and not a condition on a signature, so `Rdf.rl.Mod` cannot carry it.
  `the_checker_entails_more_than_the_institution` exhibits a premise set and a conclusion that
  `OOCert.Entails` holds of and `Rdf.rl.EntailsL` does not, using `cls-oo` and the Herbrand model of
  its own premises. The implication runs one way only, and `Rdf.rl_entails_gives_entails` is that
  way.
- **`folInst.sat` is validity and `Fol.Entails` is not.** The institution's satisfaction is "true
  under every assignment" and `Fol.Entails` quantifies the assignment outside the implication. They
  agree on closed formulas and `fol_does_not_entail_the_universal_closure` refutes the direction
  that fails, with a two-element structure, so nobody reads one as the other.
- **Nothing here changes a verdict word.** Decision 0003's rule is untouched: this layer produces no
  certificate, no report field and no executable. It is theorems about the model classes the other
  libraries already define.

## Still not done

- The instances are three and the comorphisms are two, one of which is forgetful. That is enough to
  make the abstraction non-vacuous and it is not enough to be a heterogeneous tool set. The next
  honest additions are the SHACL fragment in `lean/Shacl/` and the description-logic layer in
  `lean/Dl/`, both of which have a model class already and neither of which has been looked at from
  this angle.
- The `Institution.Th` construction is used once. Nothing yet composes a theoroidal comorphism with
  another comorphism, and `Comorphism.comp` is proved but unexercised.
- No signature-morphism-level work is done on the first-order side beyond renaming: no vocabulary
  extension, no pushouts, no amalgamation. Amalgamation is what a structured-specification layer
  would need and is the honest prerequisite for anything DOL-shaped.
- Nobody has measured what a proved comorphism buys the engine at run time, because nothing at run
  time reads this layer.
