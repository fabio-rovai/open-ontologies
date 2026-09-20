import Fo.Term

/-!
  Literals, clauses, and what it means for a structure to satisfy a clause set.

  A clause is a list of literals read as a DISJUNCTION, with its variables
  UNIVERSALLY quantified. That second half is what makes instantiation free:
  a model of a clause is a model of every instance of it, which is the whole
  reason a resolution step may apply a substitution to its parents without
  asking anyone's permission.
-/

namespace Fo

/-- An atom: a predicate symbol applied to arguments. -/
structure Atom where
  pred : Nat
  args : List Term
  deriving Repr, Inhabited

def Atom.beq (a b : Atom) : Bool := a.pred == b.pred && Term.beqs a.args b.args

instance : BEq Atom := ⟨Atom.beq⟩

theorem Atom.eq_of_beq {a b : Atom} (h : Atom.beq a b = true) : a = b := by
  simp only [Atom.beq, Bool.and_eq_true, beq_iff_eq] at h
  cases a; cases b
  simp only [Atom.mk.injEq]
  exact ⟨h.1, Term.eqs_of_beqs h.2⟩

/-- A literal: an atom and a polarity. Same choice as the propositional
checker, and for the same reason: a polarity kept in a field is a polarity no
proof has to recover from an encoding. -/
structure Lit where
  pos : Bool
  atom : Atom
  deriving Repr, Inhabited

def Lit.neg (l : Lit) : Lit := ⟨!l.pos, l.atom⟩

def Lit.beq (a b : Lit) : Bool := a.pos == b.pos && Atom.beq a.atom b.atom

instance : BEq Lit := ⟨Lit.beq⟩

theorem Lit.eq_of_beq {a b : Lit} (h : Lit.beq a b = true) : a = b := by
  simp only [Lit.beq, Bool.and_eq_true, beq_iff_eq] at h
  cases a; cases b
  simp only [Lit.mk.injEq]
  exact ⟨h.1, Atom.eq_of_beq h.2⟩

/-- A clause: a disjunction of literals, variables universally quantified. -/
abbrev Clause := List Lit

/-- A clause set. Indexed, because a certificate names its premises. -/
abbrev Formula := List (Nat × Clause)

def applyA (σ : Subst) (a : Atom) : Atom := ⟨a.pred, applyTs σ a.args⟩
def applyL (σ : Subst) (l : Lit) : Lit := ⟨l.pos, applyA σ l.atom⟩
def applyC (σ : Subst) (c : Clause) : Clause := c.map (applyL σ)

/-! ## Semantics -/

def holdsA {α : Type} (M : Struct α) (ρ : Env α) (a : Atom) : Prop :=
  M.rel a.pred (evalTs M ρ a.args)

def holdsL {α : Type} (M : Struct α) (ρ : Env α) (l : Lit) : Prop :=
  if l.pos then holdsA M ρ l.atom else ¬ holdsA M ρ l.atom

/-- A clause holds under ONE environment when some literal does. -/
def holdsC {α : Type} (M : Struct α) (ρ : Env α) (c : Clause) : Prop :=
  ∃ l, l ∈ c ∧ holdsL M ρ l

/-- A model satisfies a clause when it holds under EVERY environment: the
variables are universally quantified. -/
def Sat {α : Type} (M : Struct α) (c : Clause) : Prop :=
  ∀ ρ : Env α, holdsC M ρ c

def Models {α : Type} (M : Struct α) (F : Formula) : Prop :=
  ∀ p, p ∈ F → Sat M p.2

/-- No structure, over any carrier, satisfies every clause. -/
def Unsat (F : Formula) : Prop :=
  ∀ (α : Type) (M : Struct α), ¬ Models M F

/-! ## Instantiation is free

The three lemmas that let a resolution step substitute into its parents. Each
is the substitution lemma of `Fo.Term` carried up one level. -/

theorem holdsA_applyA {α : Type} (M : Struct α) (ρ : Env α) (σ : Subst) (a : Atom) :
    holdsA M ρ (applyA σ a) ↔ holdsA M (fun n => evalT M ρ (σ n)) a := by
  simp only [holdsA, applyA]
  rw [evalTs_applyTs]

theorem holdsL_applyL {α : Type} (M : Struct α) (ρ : Env α) (σ : Subst) (l : Lit) :
    holdsL M ρ (applyL σ l) ↔ holdsL M (fun n => evalT M ρ (σ n)) l := by
  obtain ⟨pos, atom⟩ := l
  simp only [holdsL, applyL]
  cases pos <;> simp [holdsA_applyA]

/-- A literal and its negation never hold together. -/
theorem holdsL_neg {α : Type} (M : Struct α) (ρ : Env α) (l : Lit) :
    holdsL M ρ l.neg ↔ ¬ holdsL M ρ l := by
  obtain ⟨pos, atom⟩ := l
  cases pos
  · show holdsA M ρ atom ↔ ¬ ¬ holdsA M ρ atom
    exact ⟨fun h hn => hn h, fun h => Classical.byContradiction h⟩
  · show ¬ holdsA M ρ atom ↔ ¬ holdsA M ρ atom
    exact Iff.rfl

/-- **A model of a clause is a model of every instance of it.** -/
theorem sat_applyC {α : Type} {M : Struct α} {c : Clause} (h : Sat M c) (σ : Subst) :
    Sat M (applyC σ c) := by
  intro ρ
  obtain ⟨l, hl, hv⟩ := h (fun n => evalT M ρ (σ n))
  exact ⟨applyL σ l, List.mem_map_of_mem hl, (holdsL_applyL M ρ σ l).mpr hv⟩

end Fo
