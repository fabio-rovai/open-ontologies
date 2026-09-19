import Lrat.Propagate

/-!
  The checker, and the theorem that makes it worth running.

  An LRAT line is a clause together with the identifiers of the clauses that
  make it follow by unit propagation. Checking one line is: assume the clause
  is FALSE, propagate through the named clauses in the order given, and require
  a conflict. If that works the clause is implied, so adding it changes no
  model. Derive the empty clause and there were no models to begin with.
-/

namespace Lrat

/-- One line of an LRAT proof. -/
structure Line where
  id : Nat
  clause : Clause
  hints : List Nat
  deriving Repr

/-- Clauses are named by identifier, not by position. -/
def lookup (F : Formula) (i : Nat) : Option Clause :=
  match F with
  | [] => none
  | p :: rest => if p.1 = i then some p.2 else lookup rest i

theorem lookup_mem {F : Formula} {i : Nat} {c : Clause}
    (h : lookup F i = some c) : (i, c) ∈ F := by
  induction F with
  | nil => simp [lookup] at h
  | cons p rest ih =>
    unfold lookup at h
    by_cases hp : p.1 = i
    · rw [if_pos hp] at h
      have : p.2 = c := by simpa using h
      subst this
      subst hp
      simp
    · rw [if_neg hp] at h
      exact List.mem_cons_of_mem _ (ih h)

/-- Propagate through the hints in order, insisting on a conflict.

Running out of hints without one is a FAILURE and not a success: a proof that
leaves the trail merely extended has shown nothing. -/
def runHints (F : Formula) : Trail → List Nat → Bool
  | _, [] => false
  | t, h :: hs =>
    match lookup F h with
    | none => false
    | some c =>
      match classify t c with
      | .conflict => true
      | .unit l => runHints F (l :: t) hs
      | .stuck => false

/-- The trail a RUP check starts from: the clause, negated. -/
def negAll (c : Clause) : Trail := c.map Lit.neg

/-- Is `c` implied by `F`, by the propagation the hints name? -/
def rupCheck (F : Formula) (c : Clause) (hints : List Nat) : Bool :=
  runHints F (negAll c) hints

/-- No model of `F` respects a trail the hints drive to a conflict. -/
theorem runHints_sound {σ : Assign} {F : Formula} (hm : Models σ F) :
    ∀ (hs : List Nat) (t : Trail), Respects σ t → runHints F t hs = true → False := by
  intro hs
  induction hs with
  | nil => intro t _ h; simp [runHints] at h
  | cons hd tl ih =>
    intro t hr h
    unfold runHints at h
    split at h
    · exact absurd h (by simp)
    · next c hl =>
      have hcF : clauseHolds σ c := hm (hd, c) (lookup_mem hl)
      split at h
      · next hcl => exact conflict_sound hr hcl hcF
      · next l hcl =>
        exact ih (l :: t) (respects_cons hr (unit_sound hr hcl hcF)) h
      · exact absurd h (by simp)

/-- A model that falsifies every literal of `c` respects `negAll c`. -/
theorem respects_negAll {σ : Assign} {c : Clause} (h : ¬ clauseHolds σ c) :
    Respects σ (negAll c) := by
  intro x hx
  have : ∃ l, l ∈ c ∧ l.neg = x := by
    unfold negAll at hx
    exact List.mem_map.mp hx
  obtain ⟨l, hl, heq⟩ := this
  subst heq
  rw [litVal_neg]
  cases hv : litVal σ l with
  | false => simp
  | true => exact absurd ⟨l, hl, hv⟩ h

/-- **A checked line is implied.** -/
theorem rup_sound {σ : Assign} {F : Formula} {c : Clause} {hints : List Nat}
    (hm : Models σ F) (h : rupCheck F c hints = true) : clauseHolds σ c := by
  apply Classical.byContradiction
  intro hno
  exact runHints_sound hm hints (negAll c) (respects_negAll hno) h

/-- Check a proof against a formula. Accepts exactly when some checked line is
the empty clause. -/
def check (F : Formula) : List Line → Bool
  | [] => false
  | ln :: rest =>
    if rupCheck F ln.clause ln.hints then
      if ln.clause.isEmpty then true
      else check ((ln.id, ln.clause) :: F) rest
    else false

theorem check_sound : ∀ (p : List Line) (F : Formula) (σ : Assign),
    Models σ F → check F p = true → False := by
  intro p
  induction p with
  | nil => intro F σ _ h; simp [check] at h
  | cons ln rest ih =>
    intro F σ hm h
    unfold check at h
    split at h
    · next hr =>
      have himp : clauseHolds σ ln.clause := rup_sound hm hr
      split at h
      · next he =>
        -- The empty clause holds under no assignment at all.
        have hnil : ln.clause = [] := by
          cases hc : ln.clause with
          | nil => rfl
          | cons a as => rw [hc] at he; simp at he
        rw [hnil] at himp
        obtain ⟨l, hl, _⟩ := himp
        exact absurd hl (by simp)
      · refine ih ((ln.id, ln.clause) :: F) σ ?_ h
        intro q hq
        rcases List.mem_cons.mp hq with heq | hin
        · subst heq; exact himp
        · exact hm q hin
    · exact absurd h (by simp)

/-- **The theorem.** A proof this checker accepts is a proof that the formula
it was handed has no model. -/
theorem unsat_of_check {F : Formula} {p : List Line} (h : check F p = true) :
    Unsat F := fun σ hm => check_sound p F σ hm h

end Lrat
