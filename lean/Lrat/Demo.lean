import Lrat.Check

/-!
  The checker, run. Accepting cases and forgeries, side by side.

  A gate nobody has watched fail is not a gate. Every `false` below is this one
  refusing, and each refuses for a different reason.
-/

namespace Lrat

def p : Lit := ⟨1, true⟩
def q : Lit := ⟨2, true⟩

-- `x` and `not x`: unsatisfiable, and the smallest thing that is.
def F₁ : Formula := [(1, [p]), (2, [p.neg])]

-- Assume the empty clause false, which assumes nothing, propagate `p` from
-- clause 1, and then clause 2 is all false.
def proof₁ : List Line := [⟨3, [], [1, 2]⟩]

/-- info: true -/
#guard_msgs in #eval check F₁ proof₁

theorem F₁_unsat : Unsat F₁ := unsat_of_check (p := proof₁) (by native_decide)

-- Four clauses over two variables: `p ∨ q`, `p ∨ ¬q`, `¬p ∨ q`, `¬p ∨ ¬q`.
def F₂ : Formula :=
  [(1, [p, q]), (2, [p, q.neg]), (3, [p.neg, q]), (4, [p.neg, q.neg])]

-- Derive `p` from 1 and 2, `¬p` from 3 and 4, then the empty clause from both.
def proof₂ : List Line :=
  [⟨5, [p], [2, 1]⟩, ⟨6, [p.neg], [4, 3]⟩, ⟨7, [], [5, 6]⟩]

/-- info: true -/
#guard_msgs in #eval check F₂ proof₂

theorem F₂_unsat : Unsat F₂ := unsat_of_check (p := proof₂) (by native_decide)

/-! ## Forgeries

Five ways to be refused, one per way a proof can be wrong. -/

-- Every line checks, and none of them is the empty clause, so nothing is
-- proved. A prefix of a refutation is not a refutation.
/-- info: false -/
#guard_msgs in #eval check F₂ [⟨5, [p], [2, 1]⟩]

-- Hints that do not propagate. Clauses 3 and 4 are `¬p ∨ q` and `¬p ∨ ¬q`,
-- and under the trail `¬p` neither is unit, so the line is stuck. The hints
-- have to be the clauses that actually do the work.
/-- info: false -/
#guard_msgs in #eval check F₂ [⟨5, [p], [3, 4]⟩, ⟨7, [], [5]⟩]

-- Naming a clause that is not there.
/-- info: false -/
#guard_msgs in #eval check F₂ [⟨7, [], [99]⟩]

-- Claiming the empty clause with no hints at all: the commonest forgery there
-- is, refused because running out of hints WITHOUT a conflict is a failure
-- and not a success.
/-- info: false -/
#guard_msgs in #eval check F₂ [⟨7, [], []⟩]

-- A satisfiable formula has no refutation and the checker will not invent one.
/-- info: false -/
#guard_msgs in #eval check [(1, [p, q])] [⟨2, [], [1]⟩]

/-- And the claim behind that last one, for EVERY proof rather than the one
tried: a formula with a model cannot be refuted. Exhibit the model, and the
soundness theorem does the rest. -/
theorem no_refutation_of_a_satisfiable_formula (prf : List Line) :
    check [(1, [p, q])] prf = false := by
  cases h : check [(1, [p, q])] prf with
  | false => rfl
  | true =>
    exact absurd
      (unsat_of_check h (fun _ => true)
        (by
          intro c hc
          have : c = (1, [p, q]) := by simpa using hc
          subst this
          exact ⟨p, by simp, rfl⟩))
      (by simp)

end Lrat
