import OOCert.InstitutionFol

/-!
# Witnesses: none of these institutions is empty, and the comorphism that does
# not reflect really does not reflect

A satisfaction condition over an empty model class is true and says nothing. A
comorphism between two one-point institutions preserves entailment and proves
nothing. `W3CWitness.lean` and `ConformingWitness.lean` set the standard for this
repository and this file meets it for the institution layer.

Four jobs, in order.

1. **Two distinct models in every institution defined here**, distinguished by a
   SENTENCE rather than by a term-level inequality, because a sentence is what
   the satisfaction condition and `Entails` are stated over. `Rdf.rl` gets a
   third, `herbrand svfWitness`, which is neither the everything-model nor the
   nothing-model: a model class with only those two would distinguish no
   condition from any other.
2. **Two distinct models of the background theory** the RDF-to-first-order
   comorphism translates into, so `rdf_entailment_transfers_to_fol` is not a
   statement about the empty class. One of them, `eqStruc`, satisfies the
   coherence axioms non-vacuously: its constants all denote the same element, so
   the axiom's antecedent FIRES and the conclusion has to be earned.
3. **The preservation-without-reflection pair, proved.** `rdfs9` is an
   entailment of `Rdf.rl` and is not an entailment of `Rdf.simple`, so the
   comorphism that forgets the OWL 2 RL conditions is not model-expansive.
   `Comorphism.lean` says the two theorems are different; this is the
   countermodel that makes the difference observable.
4. **The quarantined structure is inhabited.** `simpleFunctorial` discharges
   `Institution.Functorial` for `Rdf.simple`, so the four laws left out of
   `Institution` were left out because nothing needs them and not because
   nothing satisfies them.

A fifth section does the same job for a mismatch that is easy to miss:
`folInst`'s satisfaction is "true under every assignment" and `Fol.Entails` is
assignment-wise. One implication holds and the other is refuted here with a
two-element structure, so nobody reads the institution's `Entails` as
`Fol.Entails`.

A sixth refutes the other easy misreading. `OOCert.Entails`, the relation
`certificate_sound` is about, is NOT entailment in `Rdf.rl`: it quantifies over
strictly fewer interpretations, because `OOCert.Model` carries four conditions
read off the asserted graph that no signature can hold. The implication runs one
way only and `the_checker_entails_more_than_the_institution` is the countermodel
for the other.
-/
namespace OOCert

namespace InstitutionWitness

open Rdf FolInst

/-! ## Vocabulary for the witnesses

Fresh names. `Witness.lean` already binds `tC`, `tR`, `tD`, `tp`, `tx`, `ty` in
this namespace and reusing them would silently couple two files' witnesses. -/

def iA : Term := "<http://ex.org/A>"
def iB : Term := "<http://ex.org/B>"
def iX : Term := "<http://ex.org/x>"

/-- The signature every RDF witness below is stated over: `rdf:type` and
`rdfs:subClassOf` in predicate position. -/
def sigT : Rdf.Sig := [V.type, V.subClassOf]

def senXA : Rdf.Sen sigT := ⟨⟨iX, V.type, iA⟩, List.Mem.head _⟩
def senAB : Rdf.Sen sigT := ⟨⟨iA, V.subClassOf, iB⟩, List.Mem.tail _ (List.Mem.head _)⟩
def senXB : Rdf.Sen sigT := ⟨⟨iX, V.type, iB⟩, List.Mem.head _⟩
/-- A sentence about the `svf` witness graph of `Witness.lean`, used only to
tell that model apart from the other two. -/
def senSvf : Rdf.Sen sigT := ⟨⟨tx, V.type, tR⟩, List.Mem.head _⟩

/-! ## 1. Every institution here has at least two distinct models

The four lemmas below are `Iff.rfl`. They exist so that `decide` has a decidable
proposition to work on: satisfaction in these institutions unfolds to membership
in the witness graph, and stating that once keeps every refutation below a
kernel computation over concrete strings rather than a tactic script. -/

theorem simpleEmpty_sat (φ : Rdf.Sen sigT) :
    Rdf.simple.sat (S := sigT) (herbrand []) φ ↔ φ.1 ∈ ([] : List Triple) := Iff.rfl

/-- The everything-model and the nothing-model of `Rdf.simple`, told apart by a
sentence. -/
theorem simple_has_two_distinct_models :
    Rdf.simple.sat (S := sigT) saturated senXA ∧
      ¬ Rdf.simple.sat (S := sigT) (herbrand []) senXA := by
  refine ⟨trivial, ?_⟩
  rw [simpleEmpty_sat]
  decide

theorem simple_models_are_distinct : (saturated : Rdf.simple.Mod sigT) ≠ herbrand [] := by
  intro h
  exact simple_has_two_distinct_models.2 (h ▸ simple_has_two_distinct_models.1)

/-- A model of `Rdf.rl` is an interpretation together with a proof that it
satisfies `OOCert.Conditions`. All three below take that proof from a model
theorem `Witness.lean` already proves, so nothing is re-derived here. -/
def rlSaturated : Rdf.rl.Mod sigT := ⟨saturated, (saturated_is_a_model []).conds⟩
def rlEmpty : Rdf.rl.Mod sigT := ⟨herbrand [], empty_herbrand_is_a_model.conds⟩
/-- The one that is neither everything nor nothing: the Herbrand model of the
`someValuesFrom` witness graph, which satisfies `svf` non-vacuously. -/
def rlSvf : Rdf.rl.Mod sigT := ⟨herbrand svfWitness, svf_witness_is_a_model.conds⟩

theorem rlEmpty_sat (φ : Rdf.Sen sigT) :
    Rdf.rl.sat rlEmpty φ ↔ φ.1 ∈ ([] : List Triple) := Iff.rfl

theorem rlSvf_sat (φ : Rdf.Sen sigT) :
    Rdf.rl.sat rlSvf φ ↔ φ.1 ∈ svfWitness := Iff.rfl

theorem rl_has_three_pairwise_distinct_models :
    (Rdf.rl.sat rlSaturated senXA ∧ ¬ Rdf.rl.sat rlEmpty senXA) ∧
      (Rdf.rl.sat rlSvf senSvf ∧ ¬ Rdf.rl.sat rlEmpty senSvf) ∧
      (Rdf.rl.sat rlSaturated senXA ∧ ¬ Rdf.rl.sat rlSvf senXA) := by
  refine ⟨⟨trivial, ?_⟩, ⟨?_, ?_⟩, ⟨trivial, ?_⟩⟩
  · rw [rlEmpty_sat]; decide
  · rw [rlSvf_sat]; decide
  · rw [rlEmpty_sat]; decide
  · rw [rlSvf_sat]; decide

theorem rl_models_are_pairwise_distinct :
    rlSaturated ≠ rlEmpty ∧ rlSvf ≠ rlEmpty ∧ rlSaturated ≠ rlSvf := by
  refine ⟨?_, ?_, ?_⟩
  · intro h
    exact rl_has_three_pairwise_distinct_models.1.2
      (h ▸ rl_has_three_pairwise_distinct_models.1.1)
  · intro h
    exact rl_has_three_pairwise_distinct_models.2.1.2
      (h ▸ rl_has_three_pairwise_distinct_models.2.1.1)
  · intro h
    exact rl_has_three_pairwise_distinct_models.2.2.2
      (h ▸ rl_has_three_pairwise_distinct_models.2.2.1)

/-- Two first-order structures. `Fol.Struc.nonempty` is a field, so the carrier
is `Unit` rather than `Empty`: an empty carrier is not a structure and
`Fol/Semantics.lean` says why. -/
def allTrue : Fol.Struc where
  Dom := Unit
  nonempty := ⟨()⟩
  p1 := fun _ _ => True
  p2 := fun _ _ _ => True
  c := fun _ => ()

def allFalse : Fol.Struc where
  Dom := Unit
  nonempty := ⟨()⟩
  p1 := fun _ _ => False
  p2 := fun _ _ _ => False
  c := fun _ => ()

/-- A unary atom, open in variable 0. -/
def openP : Fol.Form := .app1 "P" (.var 0)

theorem fol_has_two_distinct_models :
    folInst.sat (S := []) allTrue openP ∧ ¬ folInst.sat (S := []) allFalse openP :=
  ⟨fun _ => trivial, fun h => h (fun _ => ())⟩

theorem fol_models_are_distinct : allTrue ≠ allFalse := by
  intro h
  exact fol_has_two_distinct_models.2 (h ▸ fol_has_two_distinct_models.1)

/-! ## 2. The background theory has models, and one of them earns them

`rdf_entailment_transfers_to_fol` quantifies over the models of `cohTheory S`.
If that class were empty the theorem would be vacuous, so here are three
structures in it.

`allFalse` satisfies the coherence axiom by having a false antecedent inside the
quantifiers, which is the degenerate way. `eqStruc` is the one that matters: every
constant denotes `true`, so `c q = c p` HOLDS for every pair and the axiom's
inner implication has to be discharged on its merits. It is, because every binary
symbol is interpreted by the same relation. -/

/-- All constants denote one element and every binary symbol means equality. The
coherence axioms fire and are satisfied. -/
def eqStruc : Fol.Struc where
  Dom := Bool
  nonempty := ⟨true⟩
  p1 := fun _ _ => True
  p2 := fun _ x y => x = y
  c := fun _ => true

theorem allTrue_models_coh (S : Rdf.Sig) :
    folInst.Models (S := S) allTrue (cohTheory S).ax := by
  rintro f ⟨q, _, p, _, rfl⟩ e
  intro _ _ _ _
  trivial

theorem allFalse_models_coh (S : Rdf.Sig) :
    folInst.Models (S := S) allFalse (cohTheory S).ax := by
  rintro f ⟨q, _, p, _, rfl⟩ e
  intro _ _ _ h
  exact h.elim

/-- The non-degenerate one: the antecedent `c q = c p` is true for every pair,
so this model satisfies the axioms rather than dodging them. -/
theorem eqStruc_models_coh (S : Rdf.Sig) :
    folInst.Models (S := S) eqStruc (cohTheory S).ax := by
  rintro f ⟨q, _, p, _, rfl⟩ e
  intro _ _ _ h
  exact h

/-- A closed sentence that tells `eqStruc` apart from `allTrue`: not everything
is related to everything, because `true ≠ false`. -/
def everythingRelated : Fol.Form := .all 0 (.all 1 (.app2 "r" (.var 0) (.var 1)))

theorem coh_theory_has_two_distinct_models :
    folInst.sat (S := []) allTrue everythingRelated ∧
      ¬ folInst.sat (S := []) eqStruc everythingRelated := by
  refine ⟨fun _ _ _ => trivial, ?_⟩
  intro h
  exact Bool.noConfusion (h (fun _ => true) true false)

theorem coh_theory_models_are_distinct : allTrue ≠ eqStruc := by
  intro h
  exact coh_theory_has_two_distinct_models.2 (h ▸ coh_theory_has_two_distinct_models.1)

/-! ## 3. Preservation without reflection, measured

`rdfs9` — from `x rdf:type A` and `A rdfs:subClassOf B` conclude `x rdf:type B` —
is an entailment of the OWL 2 RL institution, by `sc_sub`. It is NOT an
entailment of the simple RDF institution, because a simple interpretation is
under no obligation to respect `rdfs:subClassOf` at all, and the Herbrand
interpretation of the two premises is the countermodel.

So `forgetConditions` preserves entailment and does not reflect it, and
`Comorphism.reflects_entailment`'s hypothesis is not decoration. -/

/-- The premise set, as a predicate. -/
def Γ9 : Rdf.Sen sigT → Prop := fun ψ => ψ = senXA ∨ ψ = senAB

theorem rdfs9_is_rl_entailed : Rdf.rl.Entails (forgetConditions.image Γ9) senXB := by
  intro M hM
  have h1 : M.1.sat senXA.1 := hM senXA ⟨senXA, Or.inl rfl, rfl⟩
  have h2 : M.1.sat senAB.1 := hM senAB ⟨senAB, Or.inr rfl, rfl⟩
  exact M.2.sc_sub (M.1.ι iA) (M.1.ι iB) h2 (M.1.ι iX) h1

/-- The graph of the two premises, read as a Herbrand interpretation. -/
def rdfs9Premises : List Triple := [⟨iX, V.type, iA⟩, ⟨iA, V.subClassOf, iB⟩]

theorem simpleRdfs9_sat (φ : Rdf.Sen sigT) :
    Rdf.simple.sat (S := sigT) (herbrand rdfs9Premises) φ ↔ φ.1 ∈ rdfs9Premises := Iff.rfl

theorem rdfs9_is_not_simple_entailed : ¬ Rdf.simple.Entails Γ9 senXB := by
  intro h
  have hmod : Rdf.simple.Models (S := sigT) (herbrand rdfs9Premises) Γ9 := by
    intro ψ hψ
    rcases hψ with rfl | rfl
    · exact List.Mem.head _
    · exact List.Mem.tail _ (List.Mem.head _)
  have hno : ¬ Rdf.simple.sat (S := sigT) (herbrand rdfs9Premises) senXB := by
    rw [simpleRdfs9_sat]
    decide
  exact hno (h (herbrand rdfs9Premises) hmod)

/-- **The two theorems of `Comorphism.lean` are different theorems**, and this
is the pair that shows it: the target entails the translation, the source does
not entail the original. -/
theorem preservation_does_not_give_reflection :
    Rdf.rl.Entails (forgetConditions.image Γ9) (forgetConditions.senMap senXB) ∧
      ¬ Rdf.simple.Entails Γ9 senXB :=
  ⟨rdfs9_is_rl_entailed, rdfs9_is_not_simple_entailed⟩

/-- Consequently the comorphism is not model-expansive. Proved from the pair
above through `Comorphism.reflects_entailment`, so the refutation is of the
HYPOTHESIS and not merely of the conclusion. -/
theorem forgetting_the_conditions_is_not_model_expansive :
    ¬ forgetConditions.ModelExpansive := fun hexp =>
  rdfs9_is_not_simple_entailed (forgetConditions.reflects_entailment hexp rdfs9_is_rl_entailed)

/-! ## 4. The quarantined structure is inhabited

`Institution.Functorial` carries the four laws that `Institution` deliberately
does not. If nothing satisfied them, leaving them out would be hiding a defect
rather than following the weakest-conditions discipline. `Rdf.simple` satisfies
them, and every law is `rfl`: renaming by the identity is the identity on
triples by structure eta, and the two composition laws are function composition
read in the two directions a comorphism reads them. -/

def simpleFunctorial : Institution.Functorial where
  toInstitution := Rdf.simple
  id := fun _ => { map := fun t => t, into := fun _ h => h }
  comp := fun σ τ => { map := fun t => τ.map (σ.map t), into := fun t h => τ.into _ (σ.into t h) }
  senMap_id := fun _ _ => rfl
  senMap_comp := fun _ _ _ => rfl
  modRed_id := fun _ _ => rfl
  modRed_comp := fun _ _ _ => rfl

/-! ## 5. `folInst.sat` is validity, and `Fol.Entails` is not

`folInst.sat M f` is "`f` holds under EVERY assignment". `Fol.Entails Γ f`
quantifies the assignment outside the implication. On closed formulas the two
entailment relations agree; on open ones they do not, and the direction that
fails is the one a reader would assume.

`openP` institutionally entails its own universal closure, because a model
satisfying `openP` at every assignment satisfies it at every element. It does not
`Fol.Entails` it, because a single assignment can make `P(x₀)` true while `P`
fails elsewhere. -/

def closedP : Fol.Form := .all 0 openP

theorem institution_entails_the_universal_closure :
    folInst.EntailsL (S := []) [openP] closedP := by
  intro M hM e d
  exact hM openP (List.Mem.head _) (fun _ => d)

/-- The two-element structure that separates the relations. -/
def boolStruc : Fol.Struc where
  Dom := Bool
  nonempty := ⟨true⟩
  p1 := fun _ b => b = true
  p2 := fun _ _ _ => True
  c := fun _ => true

theorem fol_does_not_entail_the_universal_closure : ¬ Fol.Entails [openP] closedP := by
  intro h
  have hp : ∀ g ∈ [openP], Fol.Form.holds boolStruc (fun _ => true) g := by
    intro g hg
    rcases hg with _ | ⟨_, hg⟩
    · exact rfl
    · cases hg
  exact Bool.noConfusion (h boolStruc (fun _ => true) hp false)

/-! ## 6. The checker entails more than the institution does, and here is why

`Rdf.rl_entails_gives_entails` runs one way: an entailment of the OWL 2 RL
institution is an `OOCert.Entails`. The converse used to be recorded as "not
proved and not claimed". It is now refuted.

`OOCert.Model I G` carries four conditions — `int`, `int2`, `uni`, `oneOf` —
stated over `Chain G`, the RDF list read off the ASSERTED GRAPH. A condition
relative to a graph is not a condition on a signature and not a sentence, so
there is nowhere in an institution to put it, and `Rdf.rl.Mod` does not carry it.
`OOCert.Entails` therefore quantifies over strictly fewer interpretations and is
the weaker relation.

`cls-oo` is where the gap is visible. `Witness.lean` proves
`an_enumerated_member_is_entailed`: the members of an `owl:oneOf` list are typed
by the enumerated class, on schema alone. The Herbrand interpretation of those
same premises satisfies every one of the twenty-three `Conditions` fields —
vacuously, because the graph has no reserved predicate in it at all — and it does
NOT type `a` as an `E`. So the same premises and the same conclusion come out
`Entails` and not `Rdf.rl.EntailsL`.

This is a real limit of the institution as defined and not a defect in it. Closing
it means either making the list constructors sentences, which the RDF-Based
Semantics does not do either (its sequences are read off `IEXT`, not off `G`), or
carrying the graph in the signature, which is the theory construction and would
put `Rdf.rl` at `Institution.Th`. Decision 0009 records it as open. -/

/-- The Herbrand interpretation of `ooPremises`, which is `Witness.lean`'s
`owl:oneOf` premise set. Every condition holds vacuously: the graph mentions
`owl:oneOf`, `rdf:first` and `rdf:rest` and no term of `Rdf.Reserved`. -/
theorem oo_premises_conditions : Conditions (herbrand ooPremises) where
  sc_sub := fun a b hab => absurd hab (not_mem_pred ooPremises V.subClassOf (by decide) a b)
  sc_trans := fun a b _ hab => absurd hab (not_mem_pred ooPremises V.subClassOf (by decide) a b)
  sp_sub := fun a b hab => absurd hab (not_mem_pred ooPremises V.subPropertyOf (by decide) a b)
  sp_trans := fun a b _ hab =>
    absurd hab (not_mem_pred ooPremises V.subPropertyOf (by decide) a b)
  dom := fun p c hpc => absurd hpc (not_mem_pred ooPremises V.domain (by decide) p c)
  rng := fun p c hpc => absurd hpc (not_mem_pred ooPremises V.range (by decide) p c)
  trp := fun p hp => absurd hp (not_typed ooPremises V.transitiveProperty (by decide) p)
  symp := fun p hp => absurd hp (not_typed ooPremises V.symmetricProperty (by decide) p)
  inv := fun p q hpq => absurd hpq (not_mem_pred ooPremises V.inverseOf (by decide) p q)
  same := fun a b hab => absurd hab (not_mem_pred ooPremises V.sameAs (by decide) a b)
  eqc := fun a b hab => absurd hab (not_mem_pred ooPremises V.equivalentClass (by decide) a b)
  eqp := fun a b hab => absurd hab (not_mem_pred ooPremises V.equivalentProperty (by decide) a b)
  svf := fun r p _ hop => absurd hop (not_mem_pred ooPremises V.onProperty (by decide) r p)
  avf := fun r p _ hop => absurd hop (not_mem_pred ooPremises V.onProperty (by decide) r p)
  hv := fun r p _ hop => absurd hop (not_mem_pred ooPremises V.onProperty (by decide) r p)
  svf_sc := fun c1 _ _ y1 _ hsv =>
    absurd hsv (not_mem_pred ooPremises V.someValuesFrom (by decide) c1 y1)
  svf_sp := fun c1 _ _ _ y hsv =>
    absurd hsv (not_mem_pred ooPremises V.someValuesFrom (by decide) c1 y)
  avf_sc := fun c1 _ _ y1 _ hav =>
    absurd hav (not_mem_pred ooPremises V.allValuesFrom (by decide) c1 y1)
  avf_sp := fun c1 _ _ _ y hav =>
    absurd hav (not_mem_pred ooPremises V.allValuesFrom (by decide) c1 y)
  dom_sc := fun p c1 _ hd => absurd hd (not_mem_pred ooPremises V.domain (by decide) p c1)
  dom_sp := fun _ p2 c hd => absurd hd (not_mem_pred ooPremises V.domain (by decide) p2 c)
  rng_sc := fun p c1 _ hr => absurd hr (not_mem_pred ooPremises V.range (by decide) p c1)
  rng_sp := fun _ p2 c hr => absurd hr (not_mem_pred ooPremises V.range (by decide) p2 c)

/-- The predicates `ooPremises` and its `cls-oo` conclusion use. -/
def sigOO : Rdf.Sig := [V.oneOf, V.first, V.rest, V.type]

def ooSen : List (Rdf.Sen sigOO) :=
  [ ⟨⟨tE, V.oneOf, oL0⟩, List.Mem.head _⟩,
    ⟨⟨oL0, V.first, tA⟩, List.Mem.tail _ (List.Mem.head _)⟩,
    ⟨⟨oL0, V.rest, oL1⟩, List.Mem.tail _ (List.Mem.tail _ (List.Mem.head _))⟩,
    ⟨⟨oL1, V.first, tB⟩, List.Mem.tail _ (List.Mem.head _)⟩,
    ⟨⟨oL1, V.rest, V.nil⟩, List.Mem.tail _ (List.Mem.tail _ (List.Mem.head _))⟩ ]

def ooGoal : Rdf.Sen sigOO :=
  ⟨⟨tA, V.type, tE⟩, List.Mem.tail _ (List.Mem.tail _ (List.Mem.tail _ (List.Mem.head _)))⟩

/-- The model this refutation runs on. -/
def rlOO : Rdf.rl.Mod sigOO := ⟨herbrand ooPremises, oo_premises_conditions⟩

theorem rlOO_sat (φ : Rdf.Sen sigOO) : Rdf.rl.sat rlOO φ ↔ φ.1 ∈ ooPremises := Iff.rfl

theorem ooSen_are_premises : ∀ ψ ∈ ooSen, ψ.1 ∈ ooPremises := by decide

theorem the_enumeration_is_not_rl_entailed : ¬ Rdf.rl.EntailsL ooSen ooGoal := by
  intro h
  have hno : ¬ Rdf.rl.sat rlOO ooGoal := by
    rw [rlOO_sat]
    decide
  exact hno (h rlOO (fun ψ hψ => ooSen_are_premises ψ hψ))

/-- **The gap, measured.** The checker's relation holds of this premise set and
this conclusion; the institution's does not. `cls-oo` is what separates them, and
the four graph-relative conditions of `OOCert.Model` are why. -/
theorem the_checker_entails_more_than_the_institution :
    Entails ooPremises ooGoal.1 ∧ ¬ Rdf.rl.EntailsL ooSen ooGoal :=
  ⟨an_enumerated_member_is_entailed, the_enumeration_is_not_rl_entailed⟩

/-! ## The transfer, end to end on a concrete sentence

A premise is entailed by itself in `Rdf.simple`, and the general preservation
theorem turns that into a first-order statement about every model of the
background theory. Small, and it is the whole pipeline: no step of it is written
by hand. -/

def ΓXA : Rdf.Sen sigT → Prop := fun ψ => ψ = senXA

theorem the_translation_of_a_premise_is_first_order_entailed :
    ∀ M : Fol.Struc, folInst.Models (S := sigT) M (cohTheory sigT).ax →
      folInst.Models (S := sigT) M (rdfToFol.image ΓXA) →
      folInst.sat (S := sigT) M (rdfToFol.senMap senXA) :=
  rdfToFol.preserves_entailment_theoroidal (Institution.entails_of_mem Rdf.simple rfl)

end InstitutionWitness

/-! ## Axiom footprints

Every top-level statement this layer adds, pinned. A `sorry` or a
`native_decide` anywhere in the institution layer fails the build here. -/

/-- info: 'OOCert.Rdf.conditions_reduct' does not depend on any axioms -/
#guard_msgs in
#print axioms Rdf.conditions_reduct

/-- info: 'OOCert.Rdf.rl_entails_gives_entails' depends on axioms: [propext, Quot.sound] -/
#guard_msgs in
#print axioms Rdf.rl_entails_gives_entails

/-- info: 'OOCert.Rdf.simple_entailment_is_rl_entailment' depends on axioms: [propext, Quot.sound] -/
#guard_msgs in
#print axioms Rdf.simple_entailment_is_rl_entailment

/-- info: 'OOCert.FolInst.holds_red' does not depend on any axioms -/
#guard_msgs in
#print axioms FolInst.holds_red

/-- info: 'OOCert.FolInst.fol_entails_gives_institution_entails' does not depend on any axioms -/
#guard_msgs in
#print axioms FolInst.fol_entails_gives_institution_entails

/-- info: 'OOCert.FolInst.rdf_entailment_transfers_to_fol' does not depend on any axioms -/
#guard_msgs in
#print axioms FolInst.rdf_entailment_transfers_to_fol

/-- info: 'OOCert.InstitutionWitness.rl_models_are_pairwise_distinct' depends on axioms: [propext, Quot.sound] -/
#guard_msgs in
#print axioms InstitutionWitness.rl_models_are_pairwise_distinct

/-- info: 'OOCert.InstitutionWitness.fol_models_are_distinct' does not depend on any axioms -/
#guard_msgs in
#print axioms InstitutionWitness.fol_models_are_distinct

/-- info: 'OOCert.InstitutionWitness.preservation_does_not_give_reflection' depends on axioms: [propext] -/
#guard_msgs in
#print axioms InstitutionWitness.preservation_does_not_give_reflection

/--
info: 'OOCert.InstitutionWitness.forgetting_the_conditions_is_not_model_expansive' depends on axioms: [propext]
-/
#guard_msgs in
#print axioms InstitutionWitness.forgetting_the_conditions_is_not_model_expansive

/-- info: 'OOCert.InstitutionWitness.the_checker_entails_more_than_the_institution' depends on axioms: [propext] -/
#guard_msgs in
#print axioms InstitutionWitness.the_checker_entails_more_than_the_institution

/-- info: 'OOCert.InstitutionWitness.coh_theory_models_are_distinct' does not depend on any axioms -/
#guard_msgs in
#print axioms InstitutionWitness.coh_theory_models_are_distinct

/-- info: 'OOCert.InstitutionWitness.fol_does_not_entail_the_universal_closure' does not depend on any axioms -/
#guard_msgs in
#print axioms InstitutionWitness.fol_does_not_entail_the_universal_closure

/--
info: 'OOCert.InstitutionWitness.the_translation_of_a_premise_is_first_order_entailed' does not depend on any axioms
-/
#guard_msgs in
#print axioms InstitutionWitness.the_translation_of_a_premise_is_first_order_entailed

end OOCert
