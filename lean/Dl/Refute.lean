import Dl.Tableau

/-!
# Replaying a closed tableau

`Dl/Tableau.lean` says what a refutation IS. This says how to check one that
arrived as bytes.

The shape is the one `Dl/Check.lean` already uses for models: a `Bool`-valued
function over a datatype the producer fills in, and one theorem saying `true`
implies the thing. The checker is total and decides only membership, length,
distinctness and freshness, so a certificate that lies is rejected rather than
believed; nothing here searches, and nothing here has to terminate for a reason
more subtle than the certificate being a finite tree.

The asymmetry worth naming: the producer may be wrong, may be buggy, may have
been handed a broken ontology, and none of that can make this accept. What it
CAN do is fail to find a refutation that exists, and this file says nothing
about that. Soundness only.
-/
namespace Dl

/-- A closed tableau as a producer writes it: one constructor per rule, each
carrying the side conditions it needs and its children.

The clash constructors are the leaves. Everything else has one child, except
the disjunction, which has two and is the only reason this is a tree. -/
inductive Cert where
  /-- `x : ⊥`. -/
  | botC (x : Name)
  /-- `x : C` against `x : ¬C`. -/
  | negC (x : Name) (c : Concept)
  /-- `x ≠ x`. -/
  | diffC (x : Name)
  /-- `x` in two classes the ontology declares disjoint. -/
  | disjC (x : Name) (c d : Concept)
  /-- `x : ≥m R.C` against `x : ≤n R.C`, `n < m`. Needs no witnesses. -/
  | minmaxC (x r : Name) (c : Concept) (m n : Nat)
  /-- `n+1` distinct successors against `x : ≤n R.C`. -/
  | maxC (x r : Name) (c : Concept) (n : Nat) (ys : List Name)
  | instS (a : Name) (c : Concept) (k : Cert)
  | relS (a r b : Name) (k : Cert)
  | subS (x : Name) (c d : Concept) (k : Cert)
  | domS (x y r : Name) (c : Concept) (k : Cert)
  | rngS (x y r : Name) (c : Concept) (k : Cert)
  | subroleS (x y r t : Name) (k : Cert)
  | nonemptyS (y : Name) (c : Concept) (k : Cert)
  | andS (x : Name) (c d : Concept) (k : Cert)
  | allS (x y r : Name) (c : Concept) (k : Cert)
  | exS (x y r : Name) (c : Concept) (k : Cert)
  | minS (x r : Name) (c : Concept) (n : Nat) (ys : List Name) (k : Cert)
  | orS (x : Name) (c d : Concept) (l r : Cert)
deriving Repr, Inhabited

/-- Does the branch put `x` in the carrier?

Several rules need `I.dom (σ x)` and the branch supplies it only through a
concept entry, so they carry `∃ e, x : e ∈ Γ`. That is an existential over
`Concept` and so not decidable by `decide`; this searches the branch instead,
which is the same claim and is a scan. -/
def hasConc (Γ : List Constraint) (x : Name) : Bool :=
  Γ.any (fun κ => match κ with | .conc y _ => y == x | _ => false)

theorem hasConc_sound {Γ : List Constraint} {x : Name} (h : hasConc Γ x = true) :
    ∃ e, Constraint.conc x e ∈ Γ := by
  obtain ⟨κ, hκ, hp⟩ := List.any_eq_true.mp h
  cases κ with
  | conc y e => exact ⟨e, by simpa using (by simpa using hp : y = x) ▸ hκ⟩
  | role _ _ _ => simp at hp
  | diff _ _ => simp at hp

/-- Replay the certificate against the branch. Every side condition of every
rule is decided here, so nothing is taken on the producer's word. -/
def check (A : List Axiom) : Cert → List Constraint → Bool
  | .botC x, Γ => decide (Constraint.conc x .bot ∈ Γ)
  | .negC x c, Γ =>
      decide (Constraint.conc x c ∈ Γ) && decide (Constraint.conc x (.neg c) ∈ Γ)
  | .diffC x, Γ => decide (Constraint.diff x x ∈ Γ)
  | .disjC x c d, Γ =>
      decide (Axiom.disjoint c d ∈ A) && decide (Constraint.conc x c ∈ Γ)
        && decide (Constraint.conc x d ∈ Γ)
  | .minmaxC x r c m n, Γ =>
      decide (Constraint.conc x (.min m r c) ∈ Γ)
        && decide (Constraint.conc x (.max n r c) ∈ Γ) && decide (n < m)
  | .maxC x r c n ys, Γ =>
      decide (Constraint.conc x (.max n r c) ∈ Γ)
        && decide (ys.length = n + 1) && decide ys.Nodup
        && decide (∀ a ∈ ys, ∀ b ∈ ys, a ≠ b → Constraint.diff a b ∈ Γ)
        && decide (∀ y ∈ ys, Constraint.role r x y ∈ Γ)
        && decide (∀ y ∈ ys, Constraint.conc y c ∈ Γ)
  | .instS a c k, Γ =>
      decide (Axiom.inst a c ∈ A) && decide (Axiom.indiv a ∈ A)
        && check A k (Constraint.conc a c :: Γ)
  | .relS a r b k, Γ =>
      decide (Axiom.rel a r b ∈ A) && check A k (Constraint.role r a b :: Γ)
  | .subS x c d k, Γ =>
      decide (Axiom.sub c d ∈ A) && decide (Constraint.conc x c ∈ Γ)
        && check A k (Constraint.conc x d :: Γ)
  | .domS x y r c k, Γ =>
      decide (Axiom.dom r c ∈ A) && decide (Constraint.role r x y ∈ Γ) && hasConc Γ x
        && check A k (Constraint.conc x c :: Γ)
  | .rngS x y r c k, Γ =>
      decide (Axiom.rng r c ∈ A) && decide (r ∈ roleNames A)
        && decide (Constraint.role r x y ∈ Γ) && hasConc Γ x
        && check A k (Constraint.conc y c :: Γ)
  | .subroleS x y r t k, Γ =>
      decide (Axiom.subrole r t ∈ A) && decide (Constraint.role r x y ∈ Γ) && hasConc Γ x
        && check A k (Constraint.role t x y :: Γ)
  | .nonemptyS y c k, Γ =>
      decide (Axiom.nonempty c ∈ A) && decide (y ∉ branchNames Γ) && decide (y ∉ indNames A)
        && check A k (Constraint.conc y c :: Γ)
  | .andS x c d k, Γ =>
      decide (Constraint.conc x (.and c d) ∈ Γ)
        && check A k (Constraint.conc x c :: Constraint.conc x d :: Γ)
  | .allS x y r c k, Γ =>
      decide (Constraint.conc x (.all r c) ∈ Γ) && decide (Constraint.role r x y ∈ Γ)
        && decide (r ∈ roleNames A) && check A k (Constraint.conc y c :: Γ)
  | .exS x y r c k, Γ =>
      decide (Constraint.conc x (.ex r c) ∈ Γ) && decide (r ∈ roleNames A)
        && decide (y ∉ branchNames Γ) && decide (y ∉ indNames A)
        && check A k (Constraint.role r x y :: Constraint.conc y c :: Γ)
  | .minS x r c n ys k, Γ =>
      decide (Constraint.conc x (.min n r c) ∈ Γ) && decide (r ∈ roleNames A)
        && decide (ys.length = n) && decide ys.Nodup
        && decide (∀ y ∈ ys, y ∉ branchNames Γ) && decide (∀ y ∈ ys, y ∉ indNames A)
        && check A k (minExpand r x c ys ++ Γ)
  | .orS x c d l r, Γ =>
      decide (Constraint.conc x (.or c d) ∈ Γ)
        && check A l (Constraint.conc x c :: Γ) && check A r (Constraint.conc x d :: Γ)

/-- **What the checker accepts really is closed.**

One case per constructor, and each is the same two moves: take the decided side
conditions apart, then hand them to the rule of `Dl/Tableau.lean` that has
exactly those hypotheses. No case reasons about the tableau; the reasoning was
done once, there. -/
theorem closed_of_check {A : List Axiom} :
    ∀ (t : Cert) (Γ : List Constraint), check A t Γ = true → Closed A Γ := by
  intro t
  induction t with
  | botC x => intro Γ h; exact .bot (by simpa [check] using h)
  | negC x c =>
      intro Γ h
      simp only [check, Bool.and_eq_true, decide_eq_true_eq] at h
      exact .neg h.1 h.2
  | diffC x => intro Γ h; exact .diff (by simpa [check] using h)
  | disjC x c d =>
      intro Γ h
      simp only [check, Bool.and_eq_true, decide_eq_true_eq] at h
      exact .disjointClash h.1.1 h.1.2 h.2
  | minmaxC x r c m n =>
      intro Γ h
      simp only [check, Bool.and_eq_true, decide_eq_true_eq] at h
      exact .minmaxClash h.1.1 h.1.2 h.2
  | maxC x r c n ys =>
      intro Γ h
      simp only [check, Bool.and_eq_true, decide_eq_true_eq] at h
      exact .maxClash h.1.1.1.1.1 h.1.1.1.1.2 h.1.1.1.2 h.1.1.2 h.1.2 h.2
  | instS a c k ih =>
      intro Γ h
      simp only [check, Bool.and_eq_true, decide_eq_true_eq] at h
      exact .instR h.1.1 h.1.2 (ih _ h.2)
  | relS a r b k ih =>
      intro Γ h
      simp only [check, Bool.and_eq_true, decide_eq_true_eq] at h
      exact .relR h.1 (ih _ h.2)
  | subS x c d k ih =>
      intro Γ h
      simp only [check, Bool.and_eq_true, decide_eq_true_eq] at h
      exact .subR h.1.1 h.1.2 (ih _ h.2)
  | domS x y r c k ih =>
      intro Γ h
      simp only [check, Bool.and_eq_true, decide_eq_true_eq] at h
      exact .domR h.1.1.1 h.1.1.2 (hasConc_sound h.1.2) (ih _ h.2)
  | rngS x y r c k ih =>
      intro Γ h
      simp only [check, Bool.and_eq_true, decide_eq_true_eq] at h
      exact .rngR h.1.1.1.1 h.1.1.1.2 h.1.1.2 (hasConc_sound h.1.2) (ih _ h.2)
  | subroleS x y r t k ih =>
      intro Γ h
      simp only [check, Bool.and_eq_true, decide_eq_true_eq] at h
      exact .subroleR h.1.1.1 h.1.1.2 (hasConc_sound h.1.2) (ih _ h.2)
  | nonemptyS y c k ih =>
      intro Γ h
      simp only [check, Bool.and_eq_true, decide_eq_true_eq] at h
      exact .nonemptyR h.1.1.1 h.1.1.2 h.1.2 (ih _ h.2)
  | andS x c d k ih =>
      intro Γ h
      simp only [check, Bool.and_eq_true, decide_eq_true_eq] at h
      exact .andR h.1 (ih _ h.2)
  | allS x y r c k ih =>
      intro Γ h
      simp only [check, Bool.and_eq_true, decide_eq_true_eq] at h
      exact .allR h.1.1.1 h.1.1.2 h.1.2 (ih _ h.2)
  | exS x y r c k ih =>
      intro Γ h
      simp only [check, Bool.and_eq_true, decide_eq_true_eq] at h
      exact .exR h.1.1.1.1 h.1.1.1.2 h.1.1.2 h.1.2 (ih _ h.2)
  | minS x r c n ys k ih =>
      intro Γ h
      simp only [check, Bool.and_eq_true, decide_eq_true_eq] at h
      exact .minR h.1.1.1.1.1.1 h.1.1.1.1.1.2 h.1.1.1.1.2 h.1.1.1.2 h.1.1.2 h.1.2 (ih _ h.2)
  | orS x c d l r ihl ihr =>
      intro Γ h
      simp only [check, Bool.and_eq_true, decide_eq_true_eq] at h
      exact .orR h.1.1 (ihl _ h.1.2) (ihr _ h.2)

/-- The whole story in one line: a certificate that checks against the EMPTY
branch means the axiom set has no model, of any size, finite or not. -/
theorem unsatisfiable_of_check {A : List Axiom} {t : Cert} (h : check A t [] = true) :
    Unsatisfiable A :=
  unsatisfiable_of_closed (closed_of_check t [] h)

/-- And so no finite model either, which is the form the rest of the repository
states its verdicts in. -/
theorem no_model_of_check {A : List Axiom} {t : Cert} (h : check A t [] = true) :
    ¬ Satisfiable A :=
  no_finite_model_of_unsatisfiable (unsatisfiable_of_check h)

end Dl
