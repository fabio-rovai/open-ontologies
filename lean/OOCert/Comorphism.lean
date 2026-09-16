import OOCert.Institution

/-!
# Comorphisms, and the two things they do not both do

A comorphism is a translation from one logic to another: signatures forward,
sentences forward, models BACKWARD, and one law tying the three together. It is
the piece of the Heterogeneous Tool Set that carries the semantic weight, and in
Hets its satisfaction condition is established on paper rather than by the
program. Here it is the last field of the structure, so it cannot be skipped.

## The obligation is a field, and that is the whole point

`Comorphism.satCond` is data-free and proof-only. `Comorphism.mk` will not
accept a signature map, a sentence map and a model map without it. An unproved
translation is therefore not a lint warning, not a `TODO`, and not a
documentation gap: it does not typecheck. Decision 0005's title is the thesis
and this file is where the second half of it is cashed: a prover is an oracle,
and a translation is a theorem. What is NOT cashed here is the Rust exporter's
translation, which is pinned by tests and remains unproved; decision 0009 says
so in the same breath as it says what IS proved.

## Preservation and reflection are NOT the same theorem

This is the distinction the file exists to keep apart, because conflating them
is how a translation gets sold as a decision procedure it is not.

* **Preservation** — if `Γ ⊨ φ` in the source then the translations entail the
  translation in the target — follows from the satisfaction condition ALONE.
  `preserves_entailment` below has no hypothesis beyond the comorphism itself.
* **Reflection** — the converse — does NOT. It needs the source model class to
  be covered by the target's, up to satisfaction. That is `ModelExpansive`, and
  it is the condition under which "borrowing" is legitimate: running the target
  logic's prover and reading the answer back as a source-logic answer.

A comorphism that preserves and does not reflect is sound for proving and USELESS
for refuting: a target non-entailment says nothing about the source. The two are
proved separately, under separate hypotheses, and `InstitutionWitness.lean`
exhibits a comorphism between two institutions of this repository that preserves
entailment and provably does not reflect it, so the separation is a measured
fact here and not a caution.

## The weakest form of the borrowing condition

`ModelExpansive` is usually written as "`modMap` is surjective". That is
stronger than the proof needs and stronger than the instances can give. What the
proof needs is that every source model is INDISTINGUISHABLE BY SENTENCES from
some model in the image of `modMap`, which is what is written below.
`ModelExpansive.of_surjective` proves surjectivity implies it, so nothing is lost
by stating the weak form and a caller with the strong hypothesis is one lemma
away.

## What is NOT here

* No comorphism modifications, no institution morphisms (the other variance), no
  Grothendieck institution. Each is a real thing and none is needed to state or
  discharge a satisfaction condition.
* Nothing that reads a DOL document. Decision 0009 says why.
-/
namespace OOCert

universe u v w x u₂ v₂ w₂ x₂ u₃ v₃ w₃ x₃

/-- A comorphism from the institution `I` to the institution `J`.

`sigMap` and `senMap` run forward, `modMap` runs backward, and `satCond` is the
proof obligation that makes the three a translation of LOGICS rather than three
unrelated functions.

`modMap` is total on `J`-models of `sigMap S`. That is what makes the
preservation theorem unconditional, and it is also what makes a theoroidal
comorphism the right shape when the target needs axioms to reconstruct a source
model: the axioms go into the signature, through `Institution.Th`, rather than
into a side condition on `modMap`. -/
structure Comorphism (I : Institution.{u, v, w, x}) (J : Institution.{u₂, v₂, w₂, x₂}) where
  /-- Signatures, forward. -/
  sigMap : I.Sig → J.Sig
  /-- Sentences, forward, over the image signature. -/
  senMap : {S : I.Sig} → I.Sen S → J.Sen (sigMap S)
  /-- Models, BACKWARD. -/
  modMap : {S : I.Sig} → J.Mod (sigMap S) → I.Mod S
  /-- **The satisfaction condition of the comorphism.** A field. There is no
  way to build a `Comorphism` without proving it. -/
  satCond : ∀ {S : I.Sig} (M : J.Mod (sigMap S)) (φ : I.Sen S),
    I.sat (modMap M) φ ↔ J.sat M (senMap φ)

/-- A theoroidal comorphism: a comorphism into the institution of theories. The
signature map may name target axioms, which is what a translation like OWL to
first-order logic needs and what a plain comorphism cannot express. -/
abbrev Theoroidal (I : Institution.{u, v, w, x}) (J : Institution.{u₂, v₂, w₂, x₂}) :=
  Comorphism I (Institution.Th J)

namespace Comorphism

variable {I : Institution.{u, v, w, x}} {J : Institution.{u₂, v₂, w₂, x₂}}

/-- The image of a premise set under the sentence map. -/
def image (ρ : Comorphism I J) {S : I.Sig} (Γ : I.Sen S → Prop) : J.Sen (ρ.sigMap S) → Prop :=
  fun ψ' => ∃ ψ, Γ ψ ∧ ρ.senMap ψ = ψ'

/-- Half of the satisfaction condition, in the form the proofs below use: the
reduct of a target model is a source model of the translated premises exactly
when the target model satisfies them. -/
theorem models_image (ρ : Comorphism I J) {S : I.Sig} {Γ : I.Sen S → Prop}
    (M : J.Mod (ρ.sigMap S)) :
    I.Models (ρ.modMap M) Γ ↔ J.Models M (ρ.image Γ) := by
  constructor
  · intro h ψ' hψ'
    obtain ⟨ψ, hΓ, rfl⟩ := hψ'
    exact (ρ.satCond M ψ).mp (h ψ hΓ)
  · intro h ψ hΓ
    exact (ρ.satCond M ψ).mpr (h (ρ.senMap ψ) ⟨ψ, hΓ, rfl⟩)

/-- **A comorphism preserves entailment.** No hypothesis beyond the comorphism:
the satisfaction condition is the whole proof.

This is the theorem everybody wants from a translation and it is the one that
comes free. Read it in the direction it runs: a source-logic entailment may be
established in the target logic. Nothing here licenses the converse. -/
theorem preserves_entailment (ρ : Comorphism I J) {S : I.Sig}
    {Γ : I.Sen S → Prop} {φ : I.Sen S} (h : I.Entails Γ φ) :
    J.Entails (ρ.image Γ) (ρ.senMap φ) := by
  intro M hM
  exact (ρ.satCond M φ).mp (h (ρ.modMap M) ((ρ.models_image M).mpr hM))

/-- The list form, which is what the instances write. -/
theorem preserves_entailmentL (ρ : Comorphism I J) {S : I.Sig}
    {Γ : List (I.Sen S)} {φ : I.Sen S} (h : I.EntailsL Γ φ) :
    J.EntailsL (Γ.map ρ.senMap) (ρ.senMap φ) := by
  intro M hM
  refine (ρ.satCond M φ).mp (h (ρ.modMap M) ?_)
  intro ψ hψ
  exact (ρ.satCond M ψ).mpr (hM (ρ.senMap ψ) (List.mem_map_of_mem hψ))

/-- **The borrowing condition, in its weakest form.** Every source model is
indistinguishable, by source sentences, from the reduct of some target model.

Surjectivity of `modMap` implies this and is not implied by it, which is why
the weak form is what the reflection theorem takes. -/
def ModelExpansive (ρ : Comorphism I J) : Prop :=
  ∀ (S : I.Sig) (M : I.Mod S), ∃ M' : J.Mod (ρ.sigMap S),
    ∀ φ : I.Sen S, I.sat M φ ↔ I.sat (ρ.modMap M') φ

/-- Surjectivity of the model map is the textbook hypothesis and it is
stronger. Stated and proved so that a caller holding it need not weaken it by
hand, and so that nothing downstream is tempted to assume the two are the
same. -/
theorem ModelExpansive.of_surjective (ρ : Comorphism I J)
    (h : ∀ (S : I.Sig) (M : I.Mod S), ∃ M' : J.Mod (ρ.sigMap S), ρ.modMap M' = M) :
    ρ.ModelExpansive := by
  intro S M
  obtain ⟨M', hM'⟩ := h S M
  exact ⟨M', fun φ => by rw [hM']⟩

/-- **A model-expansive comorphism reflects entailment.** This is the theorem
that licenses borrowing: a target-logic answer read back as a source-logic
answer, including a NEGATIVE answer.

It is stated separately from `preserves_entailment` and under its own
hypothesis on purpose. Without `ModelExpansive` it is false, and
`InstitutionWitness.lean` exhibits the comorphism that makes it false. -/
theorem reflects_entailment (ρ : Comorphism I J) (hexp : ρ.ModelExpansive) {S : I.Sig}
    {Γ : I.Sen S → Prop} {φ : I.Sen S} (h : J.Entails (ρ.image Γ) (ρ.senMap φ)) :
    I.Entails Γ φ := by
  intro M hM
  obtain ⟨M', hM'⟩ := hexp S M
  have hmods : I.Models (ρ.modMap M') Γ := by
    intro ψ hψ
    exact (hM' ψ).mp (hM ψ hψ)
  have : J.sat M' (ρ.senMap φ) := h M' ((ρ.models_image M').mp hmods)
  exact (hM' φ).mpr ((ρ.satCond M' φ).mpr this)

/-- Borrowing, stated as the biconditional a user actually wants, under the
hypothesis it actually needs. -/
theorem borrowing (ρ : Comorphism I J) (hexp : ρ.ModelExpansive) {S : I.Sig}
    {Γ : I.Sen S → Prop} {φ : I.Sen S} :
    I.Entails Γ φ ↔ J.Entails (ρ.image Γ) (ρ.senMap φ) :=
  ⟨ρ.preserves_entailment, ρ.reflects_entailment hexp⟩

/-! ## Theoroidal preservation, unfolded

A theoroidal comorphism's preservation theorem is `preserves_entailment` at
`Institution.Th J`. Unfolded, it says what the OWL-to-first-order shape of
decision 0005 says: the target must be asked the question with the background
axioms in the premise set. Stating it in that form here means no instance has to
unfold `Th` by hand. -/

theorem preserves_entailment_theoroidal {J : Institution.{u₂, v₂, w₂, x₂}}
    (ρ : Theoroidal I J) {S : I.Sig} {Γ : I.Sen S → Prop} {φ : I.Sen S}
    (h : I.Entails Γ φ) :
    ∀ M : J.Mod (ρ.sigMap S).base, J.Models M (ρ.sigMap S).ax →
      J.Models M (ρ.image Γ) → J.sat M (ρ.senMap φ) := by
  intro M hax hΓ
  exact (Institution.th_entails_iff J (ρ.image Γ) (ρ.senMap φ)).mp
    (ρ.preserves_entailment h) M hax hΓ

/-! ## Identity and composition

Neither is needed by any theorem above. They are here because a translation
layer that cannot compose two proved translations is not a layer, and because
composition is the one operation whose satisfaction condition a reader will
otherwise assume rather than check. It is checked. -/

/-- The identity comorphism. -/
def idC (I : Institution.{u, v, w, x}) : Comorphism I I where
  sigMap := fun S => S
  senMap := fun φ => φ
  modMap := fun M => M
  satCond := fun _ _ => Iff.rfl

/-- Composition of comorphisms, with the composite satisfaction condition
PROVED rather than assumed. -/
def comp {K : Institution.{u₃, v₃, w₃, x₃}}
    (ρ : Comorphism I J) (τ : Comorphism J K) : Comorphism I K where
  sigMap := fun S => τ.sigMap (ρ.sigMap S)
  senMap := fun φ => τ.senMap (ρ.senMap φ)
  modMap := fun M => ρ.modMap (τ.modMap M)
  satCond := fun M φ => (ρ.satCond (τ.modMap M) φ).trans (τ.satCond M (ρ.senMap φ))

end Comorphism

/-- The quarantine pays off here: the general theorem applies to a
`Institution.Functorial` verbatim, by projection, with no re-proof. This is the
analogue of `W3CEntails.of_entails` for this layer, and it is what "the original
is untouched" means operationally. -/
theorem functorial_preserves_entailment (C : Institution.Functorial.{u, v, w, x})
    (D : Institution.Functorial.{u₂, v₂, w₂, x₂})
    (ρ : Comorphism C.toInstitution D.toInstitution) {S : C.toInstitution.Sig}
    {Γ : C.toInstitution.Sen S → Prop} {φ : C.toInstitution.Sen S}
    (h : C.toInstitution.Entails Γ φ) :
    D.toInstitution.Entails (ρ.image Γ) (ρ.senMap φ) :=
  ρ.preserves_entailment h

/-- info: 'OOCert.Comorphism.preserves_entailment' does not depend on any axioms -/
#guard_msgs in
#print axioms Comorphism.preserves_entailment

/-- info: 'OOCert.Comorphism.preserves_entailmentL' depends on axioms: [propext, Quot.sound] -/
#guard_msgs in
#print axioms Comorphism.preserves_entailmentL

/-- info: 'OOCert.Comorphism.reflects_entailment' does not depend on any axioms -/
#guard_msgs in
#print axioms Comorphism.reflects_entailment

/-- info: 'OOCert.Comorphism.borrowing' does not depend on any axioms -/
#guard_msgs in
#print axioms Comorphism.borrowing

/-- info: 'OOCert.Comorphism.ModelExpansive.of_surjective' does not depend on any axioms -/
#guard_msgs in
#print axioms Comorphism.ModelExpansive.of_surjective

/-- info: 'OOCert.Comorphism.preserves_entailment_theoroidal' does not depend on any axioms -/
#guard_msgs in
#print axioms Comorphism.preserves_entailment_theoroidal

/-- info: 'OOCert.functorial_preserves_entailment' does not depend on any axioms -/
#guard_msgs in
#print axioms functorial_preserves_entailment

end OOCert
