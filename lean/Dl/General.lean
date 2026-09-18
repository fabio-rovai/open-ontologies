import Dl.Semantics

/-!
# Interpretations that may be infinite, and what "unsatisfiable" means

`Dl/Semantics.lean` carries `Interp`, whose carrier is `dom : List Name`. That is
the right shape for EXHIBITING a model, which is what `oo-dlmodel` does: a finite
model is a model, so `Dl.satisfiable_of_checkModel` is sound as it stands.

It is the wrong shape for the opposite claim. "No `Interp` models `A`" says only
that `A` has no FINITE model, and that is strictly weaker than unsatisfiability
for the fragment this engine covers. SHIQ lacks the finite model property:
inverse roles together with number restrictions can force every model to be
infinite, so an axiom set with no finite model can still be perfectly
satisfiable. Reporting such a set as inconsistent would be unsound.

So a refutation certificate needs a semantics whose carrier is an arbitrary type.
That is this file. Nothing here computes; it is the target the tableau soundness
theorem in `Dl/Tableau.lean` aims at, and `finite_model_is_a_model` below is what
stops the two semantics drifting apart.
-/
namespace Dl

/-- An interpretation over an arbitrary carrier. The carrier is the type itself
rather than a distinguished subset, so there is no well-formedness side condition
to carry: every element of `α` is a domain element by construction. -/
structure GInterp (α : Type) where
  /-- The carrier. A PREDICATE rather than the whole type, for one reason that
  is worth stating: it makes the bridge from a finite `Interp` almost
  definitional, since `x ∈ I.dom` maps straight onto it. Carrying the carrier as
  the type itself reads more cleanly and pushes the whole cost into a subtype
  construction that the counting clauses then have to be transported across. -/
  dom : α → Prop
  cext : Name → α → Prop
  rext : Name → α → α → Prop
  ind : Name → α

/-- At least `n` distinct elements satisfy `P`. Stated with a list of distinct
witnesses, exactly as the finite version is, so the two read the same. -/
def GAtLeast {α : Type} (n : Nat) (P : α → Prop) : Prop :=
  ∃ ys : List α, ys.Nodup ∧ ys.length = n ∧ ∀ y ∈ ys, P y

/-- At most `n` distinct elements satisfy `P`. -/
def GAtMost {α : Type} (n : Nat) (P : α → Prop) : Prop :=
  ∀ ys : List α, ys.Nodup → (∀ y ∈ ys, P y) → ys.length ≤ n

/-- `x` is in the extension of `c`. Clause for clause the same as `Sat`, with
membership in a list replaced by a relation. -/
def GSat {α : Type} (I : GInterp α) : Concept → α → Prop
  | .top, _ => True
  | .bot, _ => False
  | .atom a, x => I.cext a x
  | .neg c, x => ¬ GSat I c x
  | .and c d, x => GSat I c x ∧ GSat I d x
  | .or c d, x => GSat I c x ∨ GSat I d x
  | .ex r c, x => ∃ y, I.rext r x y ∧ GSat I c y
  | .all r c, x => ∀ y, I.rext r x y → GSat I c y

  | .min n r c, x => GAtLeast n (fun y => I.rext r x y ∧ GSat I c y)
  | .max n r c, x => GAtMost n (fun y => I.rext r x y ∧ GSat I c y)

/-- The axiom holds in `I`. No `∈ dom` guards: the carrier is the whole type. -/
def GHolds {α : Type} (I : GInterp α) : Axiom → Prop
  | .sub c d => ∀ x, I.dom x → GSat I c x → GSat I d x
  | .disjoint c d => ∀ x, I.dom x → ¬ (GSat I c x ∧ GSat I d x)
  | .dom r c => ∀ x, I.dom x → (∃ y, I.rext r x y) → GSat I c x
  | .rng r c => ∀ x, I.dom x → ∀ y, I.rext r x y → GSat I c y
  | .subrole r s => ∀ x, I.dom x → ∀ y, I.rext r x y → I.rext s x y
  | .trans r => ∀ x, I.dom x → ∀ y, I.rext r x y → ∀ z, I.rext r y z → I.rext r x z
  | .sym r => ∀ x, I.dom x → ∀ y, I.rext r x y → I.rext r y x
  | .inv r s =>
      (∀ x, I.dom x → ∀ y, I.rext r x y → I.rext s y x) ∧
      (∀ x, I.dom x → ∀ y, I.rext s x y → I.rext r y x)
  | .invfunc r => ∀ y, I.dom y → GAtMost 1 (fun x => I.dom x ∧ I.rext r x y)
  | .inst a c => GSat I c (I.ind a)
  | .rel a r b => I.rext r (I.ind a) (I.ind b)
  | .indiv a => I.dom (I.ind a)
  | .nonempty c => ∃ x, I.dom x ∧ GSat I c x

/-- `I` is a model of `A`, over a carrier that is not empty. -/
structure GModels {α : Type} (I : GInterp α) (A : List Axiom) : Prop where
  /-- The carrier is not empty, for the reason `WellFormed` gives: without it
  every subclass axiom holds vacuously and satisfiability would mean nothing. -/
  nonempty : ∃ x, I.dom x
  /-- Roles the axioms mention do not lead out of the carrier.

  The finite side carries the same clause as `WellFormed.rextInDom`, and for the
  same reason: without it `GSat` could be evaluated at a point no axiom ever
  constrains. It is also what the existential tableau rule needs, since the
  witness it invents has to be a carrier element for the axioms to bite on it.
  Restricted to mentioned roles exactly as the finite clause is, which is what
  lets the bridge below supply it unchanged. -/
  rextDom : ∀ r ∈ roleNames A, ∀ x y, I.dom x → I.rext r x y → I.dom y
  holds : ∀ a ∈ A, GHolds I a

/-- `A` has NO model, of any size.

This is the statement a refutation certificate has to earn, and it is the reason
this file exists. The quantifier ranges over every carrier type, so it is not
satisfied by ruling out the finite ones. -/
def Unsatisfiable (A : List Axiom) : Prop :=
  ∀ (α : Type) (I : GInterp α), ¬ GModels I A

/-- A finite interpretation read as a general one. Every clause is the finite
one with list membership turned into a predicate. -/
def GInterp.ofInterp (I : Interp) : GInterp Name where
  dom x := x ∈ I.dom
  cext a x := x ∈ I.cext a
  rext r x y := y ∈ I.rext r x
  ind := I.ind

/-! The projections, as simp lemmas. Unfolding `GInterp.ofInterp` itself leaves
a structure literal in the goal that the induction hypothesis no longer matches,
so the clauses are given by name instead. -/

@[simp] theorem ofInterp_dom (I : Interp) (x : Name) :
    (GInterp.ofInterp I).dom x ↔ x ∈ I.dom := Iff.rfl

@[simp] theorem ofInterp_cext (I : Interp) (a x : Name) :
    (GInterp.ofInterp I).cext a x ↔ x ∈ I.cext a := Iff.rfl

@[simp] theorem ofInterp_rext (I : Interp) (r x y : Name) :
    (GInterp.ofInterp I).rext r x y ↔ y ∈ I.rext r x := Iff.rfl

@[simp] theorem ofInterp_ind (I : Interp) (a : Name) :
    (GInterp.ofInterp I).ind a = I.ind a := rfl

/-- Satisfaction agrees, clause for clause. The counting cases go through
because both sides quantify over `List Name` with `Nodup`, which is why the
carrier is a predicate over `α` rather than a subtype. -/
theorem gsat_ofInterp (I : Interp) (c : Concept) (x : Name) :
    GSat (GInterp.ofInterp I) c x ↔ Sat I c x := by
  induction c generalizing x with
  | top => exact Iff.rfl
  | bot => exact Iff.rfl
  | atom a => simp [GSat, Sat, GInterp.ofInterp]
  | neg c ih => simp [GSat, Sat, ih]
  | and c d ihc ihd => simp [GSat, Sat, ihc, ihd]
  | or c d ihc ihd => simp [GSat, Sat, ihc, ihd]
  | ex r c ih => simp [GSat, Sat, ih]
  | all r c ih => simp [GSat, Sat, ih]
  | min n r c ih =>
      simp only [GSat, Sat, GAtLeast, AtLeast, ofInterp_rext]
      constructor
      · rintro ⟨ys, hnd, hlen, hmem⟩
        exact ⟨ys, hnd, hlen, fun y hy => ⟨(hmem y hy).1, (ih y).mp (hmem y hy).2⟩⟩
      · rintro ⟨ys, hnd, hlen, hmem⟩
        exact ⟨ys, hnd, hlen, fun y hy => ⟨(hmem y hy).1, (ih y).mpr (hmem y hy).2⟩⟩
  | max n r c ih =>
      simp only [GSat, Sat, GAtMost, AtMost, ofInterp_rext]
      constructor
      · intro h ys hnd hmem
        exact h ys hnd (fun y hy => ⟨(hmem y hy).1, (ih y).mpr (hmem y hy).2⟩)
      · intro h ys hnd hmem
        exact h ys hnd (fun y hy => ⟨(hmem y hy).1, (ih y).mp (hmem y hy).2⟩)

/-- The bridge, and the reason the two semantics cannot drift apart: a finite
model IS a model, so anything this file calls unsatisfiable has no finite model
either.

Carried as a theorem rather than assumed, because the whole value of the
refutation layer is that its conclusion contradicts the model layer's. If these
two notions of model were unrelated, `oo-dlmodel` accepting a model and
`oo-refute` accepting a refutation of the same axiom set would not be a
contradiction, and neither result would mean anything. -/
theorem no_finite_model_of_unsatisfiable {A : List Axiom} (h : Unsatisfiable A) :
    ¬ Satisfiable A := by
  rintro ⟨I, hI⟩
  refine h Name (GInterp.ofInterp I) ⟨?_, ?_, ?_⟩
  · cases hd : I.dom with
    | nil => exact absurd hd hI.wf.nonempty
    | cons a as => exact ⟨a, by simp [hd]⟩
  · intro r hr x y hx hxy
    exact hI.wf.rextInDom r hr x hx y hxy
  · intro ax hax
    have hh := hI.holds ax hax
    cases ax with
    | dom r c =>
        -- The one clause the two semantics phrase differently: the finite side
        -- says the successor list is non-empty, the general side says a
        -- successor exists. `List.ne_nil_of_mem` is the whole of the gap.
        intro x hx hex
        obtain ⟨y, hy⟩ := hex
        exact (gsat_ofInterp I c x).mpr (hh x hx (List.ne_nil_of_mem hy))
    | sub c d => simp_all [GHolds, Holds, gsat_ofInterp]
    | disjoint c d => simp_all [GHolds, Holds, gsat_ofInterp]
    | rng r c =>
        intro x hx y hy
        exact (gsat_ofInterp I c y).mpr (hh x hx y hy)
    | subrole r s => simp_all [GHolds, Holds]
    | trans r =>
        intro x hx y hy z hz
        exact hh x hx y hy z hz
    | sym r => simp_all [GHolds, Holds]
    | inv r s => simp_all [GHolds, Holds]
    | invfunc r =>
        intro y hy ys hnd hmem
        exact hh y hy ys hnd hmem
    | inst a c => simp_all [GHolds, Holds, gsat_ofInterp]
    | rel a r b => simp_all [GHolds, Holds]
    | indiv a => simp_all [GHolds, Holds]
    | nonempty c => simp_all [GHolds, Holds, gsat_ofInterp]

end Dl
