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

/-- Scan the hints for one that is a conflict or a unit under the current
trail, and return it with the hints that remain.

A SCAN and not a queue, and the difference is what makes real solver output
checkable. LRAT's hints are nominally a propagation order, but a trace from a
solver is a resolution chain, and a chain is not a propagation order: measured
on 53 unsatisfiable instances from picosat's extended trace, 40 failed when the
hints were consumed in the order given and all 53 check when they are scanned.

Scanning is sound for the same reason consuming was: every step still goes
through `conflict_sound` or `unit_sound`, which care about the clause and the
trail and not about where in the list the clause was found. It accepts strictly
more proofs, and every one it accepts is still a proof. -/
def pick (F : Formula) (t : Trail) : List Nat → Option (Nat × Step × List Nat)
  | [] => none
  | h :: hs =>
    match lookup F h with
    | none => none
    | some c =>
      match classify t c with
      | .conflict => some (h, .conflict, hs)
      | .unit l => some (h, .unit l, hs)
      | .stuck =>
        match pick F t hs with
        | some (i, st, rest) => some (i, st, h :: rest)
        | none => none

/-- Whatever `pick` returns, it really did come from a clause of `F`. -/
theorem pick_spec {F : Formula} {t : Trail} : ∀ {hs : List Nat} {i : Nat}
    {st : Step} {rest : List Nat}, pick F t hs = some (i, st, rest) →
    ∃ c, lookup F i = some c ∧ classify t c = st := by
  intro hs
  induction hs with
  | nil => intro i st rest h; simp [pick] at h
  | cons hd tl ih =>
    intro i st rest h
    unfold pick at h
    split at h
    · exact absurd h (by simp)
    · next c hl =>
      split at h
      · next hcl =>
        simp only [Option.some.injEq, Prod.mk.injEq] at h
        obtain ⟨hi, hst, _⟩ := h
        subst hi; subst hst
        exact ⟨c, hl, hcl⟩
      · next l hcl =>
        simp only [Option.some.injEq, Prod.mk.injEq] at h
        obtain ⟨hi, hst, _⟩ := h
        subst hi; subst hst
        exact ⟨c, hl, hcl⟩
      · split at h
        · next res hr =>
          obtain ⟨c', hc1, hc2⟩ := ih hr
          simp only [Option.some.injEq, Prod.mk.injEq] at h
          obtain ⟨hi, hst, _⟩ := h
          subst hi; subst hst
          exact ⟨c', hc1, hc2⟩
        · exact absurd h (by simp)

/-- Propagate until a conflict, taking hints in whatever order they work.

`fuel` is the hint count: each successful step consumes one hint, so a proof
that needs more steps than it gave hints is a proof that does not check. -/
def runHints (F : Formula) : Nat → Trail → List Nat → Bool
  | 0, _, _ => false
  | fuel + 1, t, hs =>
    match pick F t hs with
    | some (_, .conflict, _) => true
    | some (_, .unit l, rest) => runHints F fuel (l :: t) rest
    | _ => false

/-- No model of `F` respects a trail the hints drive to a conflict. -/
theorem runHints_sound {σ : Assign} {F : Formula} (hm : Models σ F) :
    ∀ (fuel : Nat) (t : Trail) (hs : List Nat),
      Respects σ t → runHints F fuel t hs = true → False := by
  intro fuel
  induction fuel with
  | zero => intro t hs _ h; simp [runHints] at h
  | succ f ih =>
    intro t hs hr h
    unfold runHints at h
    split at h
    · next i rest hp =>
      obtain ⟨c, hl, hcl⟩ := pick_spec hp
      exact conflict_sound hr hcl (hm (i, c) (lookup_mem hl))
    · next i l rest hp =>
      obtain ⟨c, hl, hcl⟩ := pick_spec hp
      exact ih (l :: t) rest
        (respects_cons hr (unit_sound hr hcl (hm (i, c) (lookup_mem hl)))) h
    · exact absurd h (by simp)

/-- The trail a RUP check starts from: the clause, negated. -/
def negAll (c : Clause) : Trail := c.map Lit.neg

/-- Is `c` implied by `F`, by the propagation the hints name? -/
def rupCheck (F : Formula) (c : Clause) (hints : List Nat) : Bool :=
  runHints F hints.length (negAll c) hints

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
  exact runHints_sound hm hints.length (negAll c) hints (respects_negAll hno) h

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
