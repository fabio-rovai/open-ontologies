/-!
# Institutions, in core Lean, with the satisfaction condition as an obligation

An institution is Goguen and Burstall's answer to "what is a logic, for the
purpose of moving things between logics". It is four pieces and one law: a
category of signatures, a sentence functor, a model functor running the other
way, a satisfaction relation, and the **satisfaction condition**, that truth is
invariant under change of notation.

This file states that in core Lean. There is no Mathlib here, so there is no
`CategoryTheory.Functor` to lean on, and nothing below pretends there is. What
is written is exactly what the theorems in `Comorphism.lean` consume, and no
more.

## The one field that matters

`satCond` is a FIELD. An institution cannot be constructed without a proof of
it. That is the whole design. In the Heterogeneous Tool Set a comorphism is a
Haskell type-class instance supplying the three maps, and its satisfaction
condition is discharged in the literature rather than by the program; decision
0005 already put it that way, "Hets proves its OWL-to-CASL comorphism on paper".
Here the proof is a constructor argument, so a missing one is a type error at
the point of construction rather than a remark in a manual. Nobody on this
branch has read the Hets source, so that is a statement about its published
design and not a code review.

## The weakest-conditions discipline, applied, and it points the opposite way
## from `Semantics.lean`

`Semantics.lean` carries a warning that a condition stronger than the
specification shrinks the model class and silently weakens every downstream
theorem. The same discipline applies here and it lands somewhere that looks
wrong until it is followed through.

`satCond` is a `∀` over signature morphisms, over target models, and over source
sentences. So:

* **More morphisms make a STRONGER institution.** Every constraint added to
  `SigMor` removes morphisms and therefore removes instances of the law. An
  instance below asks of its morphisms exactly what its `modRed` needs to land
  in `Mod`, and nothing else.
* **More sentences make a STRONGER institution.** Restricting `Sen S` to the
  formulas "over" the signature `S` is the textbook presentation and it is a
  WEAKENING of the law. It is done below only where a downstream comorphism
  genuinely cannot hold without it, and where it is done the restriction is the
  weakest one that works: `Rdf.Sen` constrains the PREDICATE position of a
  triple and leaves subject and object free, because only the predicate's
  denotation is what the relevant satisfaction condition turns on.
* **More models make a STRONGER institution** in the satisfaction condition and
  a WEAKER one in `Entails`. The two pull opposite ways, and every instance
  below says at its `Mod` field which of the two it is trading.

`Sen` and `Mod` are declared as families indexed by `Sig` because the abstract
law needs them to be. ALL THREE instances in this development have a `Mod` that
does not in fact vary with the signature, and each says so at the field rather
than dressing it up: a `Fol.Struc` interprets every symbol and an `OOCert.Interp`
denotes every term, so there is nothing for a signature to cut down, and cutting
the model class down anyway would shrink it for nothing.

## Universes

`Sig`, `SigMor`, `Sen` and `Mod` are separately universe-polymorphic because
they have to be. `OOCert.Interp` carries `D : Type` and so lives in `Type 1`,
`Fol.Struc` likewise, while a signature here is a list of strings and lives in
`Type`. A single-universe `Institution` could not hold both.

## What is NOT here

* No category laws on `SigMor` and no functoriality for `senMap` or `modRed`.
  Not because they are false of the instances — `Institution.Functorial` below
  carries them and `InstitutionWitness.lean` discharges them for the RDF
  instance — but because no theorem in `Comorphism.lean` uses them, so putting
  them in `Institution` would remove institutions from the class for nothing.
  This is the `W3C.lean` quarantine, applied to this layer.
* No room semantics, no structured specifications, no DOL. Decision 0009 says
  why the parser is deferred rather than merely missing.
-/
namespace OOCert

universe u v w x u₂ v₂ w₂ x₂

-- `checkUnivs` fires on both structures below because their own type is a
-- `max` of the four universes and none of the four appears alone in it. That is
-- exactly the shape wanted here: see the Universes section above.
set_option linter.checkUnivs false in
/-- An institution: signatures, signature morphisms, sentences, models,
satisfaction, and the satisfaction condition.

The satisfaction condition reads: for a signature morphism `σ : S ⟶ S'`, an
`S'`-model `M` and an `S`-sentence `φ`,

> the reduct of `M` along `σ` satisfies `φ` exactly when `M` satisfies the
> translation of `φ` along `σ`.

Truth does not depend on notation. Every field below is data except the last,
which is the obligation. -/
structure Institution where
  /-- Signatures. -/
  Sig : Type u
  /-- Signature morphisms. A family rather than a category: see the header. -/
  SigMor : Sig → Sig → Type v
  /-- Sentences over a signature. -/
  Sen : Sig → Type w
  /-- Models of a signature. -/
  Mod : Sig → Type x
  /-- Sentence translation, along the morphism. -/
  senMap : {S S' : Sig} → SigMor S S' → Sen S → Sen S'
  /-- Model reduction, AGAINST the morphism. This is the contravariance that
  makes an institution an institution rather than a pair of maps. -/
  modRed : {S S' : Sig} → SigMor S S' → Mod S' → Mod S
  /-- Satisfaction. -/
  sat : {S : Sig} → Mod S → Sen S → Prop
  /-- **The satisfaction condition.** Not a lemma, not a convention: a field,
  so that no institution exists without it. -/
  satCond : ∀ {S S' : Sig} (σ : SigMor S S') (M : Mod S') (φ : Sen S),
    sat (modRed σ M) φ ↔ sat M (senMap σ φ)

namespace Institution

variable (I : Institution.{u, v, w, x})

/-- `M` satisfies every sentence of `Γ`. `Γ` is a PREDICATE and not a list: the
theorems below never enumerate it, and a predicate is the weaker hypothesis to
state them over. -/
def Models {S : I.Sig} (M : I.Mod S) (Γ : I.Sen S → Prop) : Prop :=
  ∀ ψ, Γ ψ → I.sat M ψ

/-- `Γ ⊨ φ` over the signature `S`: every `S`-model of `Γ` satisfies `φ`. -/
def Entails {S : I.Sig} (Γ : I.Sen S → Prop) (φ : I.Sen S) : Prop :=
  ∀ M : I.Mod S, I.Models M Γ → I.sat M φ

/-- The same relation with the premises given as a list, which is what the
instances below actually write. -/
def EntailsL {S : I.Sig} (Γ : List (I.Sen S)) (φ : I.Sen S) : Prop :=
  I.Entails (fun ψ => ψ ∈ Γ) φ

theorem entails_of_mem {S : I.Sig} {Γ : I.Sen S → Prop} {φ : I.Sen S} (h : Γ φ) :
    I.Entails Γ φ := fun _ hM => hM φ h

theorem entailsL_of_mem {S : I.Sig} {Γ : List (I.Sen S)} {φ : I.Sen S} (h : φ ∈ Γ) :
    I.EntailsL Γ φ := entails_of_mem I h

/-- Entailment is monotone in the premises. Weakening the premise set can only
lose entailments, never gain them, which is the property every theorem about a
translation quietly assumes and none of them states. -/
theorem entails_mono {S : I.Sig} {Γ Δ : I.Sen S → Prop} {φ : I.Sen S}
    (hsub : ∀ ψ, Γ ψ → Δ ψ) (h : I.Entails Γ φ) : I.Entails Δ φ :=
  fun M hM => h M (fun ψ hψ => hM ψ (hsub ψ hψ))

/-! ## Theories

A theoroidal comorphism sends a signature to a THEORY of the target, not to a
signature: `owl-lean`'s adequacy theorem is of that shape, since its right-hand
side is `FOL.Entails (background ++ indAxioms inds ++ O.map trAx) (trAx a)` and
`background` is target-side machinery that the source signature alone does not
name. Rather than carry a second notion of comorphism, this development builds
the institution of theories over `I` and lets a theoroidal comorphism be an
ordinary comorphism into it. That is the standard construction and it keeps
`Comorphism.lean` down to one structure and one obligation.

The satisfaction condition of `Th I` is `I`'s own, unchanged: the extra data is
carried by the signatures and the models, and neither `sat` nor `senMap` sees
it. -/

/-- A theory: a signature and a set of axioms over it. -/
structure ThSig (I : Institution.{u, v, w, x}) where
  /-- The underlying signature. -/
  base : I.Sig
  /-- The axioms. A predicate, so that an infinite axiomatisation is expressible
  and nothing here depends on the set being listed. -/
  ax : I.Sen base → Prop

/-- The institution of theories over `I`.

A morphism of theories carries the SEMANTIC condition that the reduct of a model
of the target theory is a model of the source theory. That is the weakest thing
that makes `modRed` land where it must. The syntactic condition usually written
instead, that every translated axiom is an axiom of the target, is strictly
stronger: it implies this one and removes morphisms that satisfy it, and by the
header's first bullet that would be a weaker institution. -/
def Th (I : Institution.{u, v, w, x}) : Institution.{max u w, v, w, x} where
  Sig := ThSig I
  SigMor := fun T T' =>
    { σ : I.SigMor T.base T'.base //
        ∀ M : I.Mod T'.base, I.Models M T'.ax → I.Models (I.modRed σ M) T.ax }
  Sen := fun T => I.Sen T.base
  Mod := fun T => { M : I.Mod T.base // I.Models M T.ax }
  senMap := fun σ φ => I.senMap σ.1 φ
  modRed := fun σ M => ⟨I.modRed σ.1 M.1, σ.2 M.1 M.2⟩
  sat := fun M φ => I.sat M.1 φ
  satCond := fun σ M φ => I.satCond σ.1 M.1 φ

/-- Satisfaction in `Th I` is satisfaction in `I`. Stated because everything
downstream reads it, and reading it off the definition by hand is how a layer
acquires a silent mismatch. -/
@[simp] theorem th_sat {T : ThSig I} (M : (Th I).Mod T) (φ : (Th I).Sen T) :
    (Th I).sat M φ ↔ I.sat M.1 φ := Iff.rfl

/-- Entailment in `Th I` is `I`-entailment from the axioms together with the
premises. This is what makes the theory construction worth having rather than
an encoding: the extra axioms are premises, and nothing else. -/
theorem th_entails_iff {T : ThSig I} (Γ : I.Sen T.base → Prop) (φ : I.Sen T.base) :
    (Th I).Entails (S := T) Γ φ ↔
      ∀ M : I.Mod T.base, I.Models M T.ax → I.Models M Γ → I.sat M φ := by
  constructor
  · intro h M hax hΓ
    exact h ⟨M, hax⟩ hΓ
  · intro h M hΓ
    exact h M.1 M.2 hΓ

/-! ## The quarantine

`Institution` above has no category laws and no functoriality. They are true of
the instances in this development and they are not needed by any theorem in
`Comorphism.lean`, so they are carried HERE, in a separate structure, exactly as
`W3C.lean` carries the specification conditions away from `Conditions`.

The reason is the one `W3C.lean` gives, transposed. Adding a field to
`Institution` would REMOVE institutions from the class that `Comorphism`'s
theorems quantify over. Every such theorem would still compile and would say
less, with no error anywhere. The protection is that `toInstitution` below is a
FIELD PROJECTION, so the object every theorem is about is unchanged, and
`functorial_preserves_entailment` in `Comorphism.lean` is the demonstration:
it is the general theorem applied, not a re-proof. -/

set_option linter.checkUnivs false in
/-- The functoriality laws, quarantined. An `Institution.Functorial` is an
`Institution` with identities, composition, and the four laws saying that
`senMap` is a functor and `modRed` a contravariant one.

Nothing in this development consumes these fields. They exist so that the base
structure can be checked against the textbook definition without the base
structure paying for it, and `InstitutionWitness.lean` discharges them for the
RDF instance so that the class is not empty. -/
structure Functorial where
  /-- The institution being strengthened. A projection: see the section header. -/
  toInstitution : Institution.{u, v, w, x}
  /-- The identity morphism. -/
  id : ∀ S, toInstitution.SigMor S S
  /-- Composition of morphisms. -/
  comp : ∀ {S S' S'' : toInstitution.Sig},
    toInstitution.SigMor S S' → toInstitution.SigMor S' S'' → toInstitution.SigMor S S''
  senMap_id : ∀ (S : toInstitution.Sig) (φ : toInstitution.Sen S),
    toInstitution.senMap (id S) φ = φ
  senMap_comp : ∀ {S S' S'' : toInstitution.Sig}
    (σ : toInstitution.SigMor S S') (τ : toInstitution.SigMor S' S'')
    (φ : toInstitution.Sen S),
    toInstitution.senMap (comp σ τ) φ = toInstitution.senMap τ (toInstitution.senMap σ φ)
  modRed_id : ∀ (S : toInstitution.Sig) (M : toInstitution.Mod S),
    toInstitution.modRed (id S) M = M
  modRed_comp : ∀ {S S' S'' : toInstitution.Sig}
    (σ : toInstitution.SigMor S S') (τ : toInstitution.SigMor S' S'')
    (M : toInstitution.Mod S''),
    toInstitution.modRed (comp σ τ) M = toInstitution.modRed σ (toInstitution.modRed τ M)

end Institution

end OOCert
