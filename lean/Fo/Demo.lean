import Fo.Check

/-!
  The calculus, run.

  The worked example is the one every textbook opens with, and it is here
  because it is the smallest refutation that actually NEEDS unification:

      ∀x. P(x) → Q(x)          ¬P(x) ∨ Q(x)
      P(a)                     P(a)
      ¬Q(a)                    ¬Q(a)

  Resolving the first two requires instantiating `x` to `a`, which is the step
  a propositional checker cannot take and the reason this development exists.
-/

namespace Fo

-- Symbols: variable 0 is `x`, function 0 is the constant `a`, predicate 0 is
-- `P` and predicate 1 is `Q`.
def x : Term := .var 0
def a : Term := .app 0 []
def P (t : Term) : Atom := ⟨0, [t]⟩
def Q (t : Term) : Atom := ⟨1, [t]⟩

def notPx : Lit := ⟨false, P x⟩
def Qx    : Lit := ⟨true,  Q x⟩
def Pa    : Lit := ⟨true,  P a⟩
def notQa : Lit := ⟨false, Q a⟩
def Qa    : Lit := ⟨true,  Q a⟩

def F : Formula := [(1, [notPx, Qx]), (2, [Pa]), (3, [notQa])]

/-- `x ↦ a`, the substitution the first step needs. -/
def xa : List (Nat × Term) := [(0, a)]

def refutation : Refutation :=
  [ -- Resolve `¬P(x) ∨ Q(x)` with `P(a)`, instantiating x to a, giving `Q(a)`.
    ⟨4, [Qa], .resolve 1 notPx xa [Qx] 2 Pa [] []⟩,
    -- Resolve `Q(a)` with `¬Q(a)`, giving the empty clause.
    ⟨5, [], .resolve 4 Qa [] [] 3 notQa [] []⟩ ]

/-- info: true -/
#guard_msgs in #eval check F refutation

/-- The clause set has no model, over any carrier. -/
theorem F_unsat : Unsat F := unsat_of_check (p := refutation) (by native_decide)

/-! ## Forgeries

Six ways to be refused, one per way a step can be wrong. -/

-- The substitution is missing, so `¬P(x)` and `P(a)` are not complementary:
-- `x` and `a` are different terms and the checker compares terms.
/-- info: false -/
#guard_msgs in #eval check F [⟨4, [Qa], .resolve 1 notPx [] [Qx] 2 Pa [] []⟩, ⟨5, [], .resolve 4 Qa [] [] 3 notQa [] []⟩]

-- Substituting the WRONG way: `a ↦ x` does not unify these either.
/-- info: false -/
#guard_msgs in #eval check F [⟨4, [Qa], .resolve 1 notPx [(1, x)] [Qx] 2 Pa [] []⟩]

-- Resolving two literals of the SAME polarity. `Q(a)` against `Q(a)` is not a
-- resolution step however well the terms match.
/-- info: false -/
#guard_msgs in #eval check F [⟨4, [Qa], .resolve 1 notPx xa [Qx] 2 Pa [] []⟩, ⟨5, [], .resolve 4 Qa [] [] 4 Qa [] []⟩]

-- The remainder does not cover the parent: `Q(x)` is dropped from clause 1
-- without being resolved away, so the conclusion claims more than the step did.
/-- info: false -/
#guard_msgs in #eval check F [⟨4, [], .resolve 1 notPx xa [] 2 Pa [] []⟩]

-- Naming a clause that is not there.
/-- info: false -/
#guard_msgs in #eval check F [⟨9, [], .resolve 99 notPx xa [Qx] 2 Pa [] []⟩]

-- A prefix of a refutation is not a refutation: the step checks, and it is
-- not the empty clause.
/-- info: false -/
#guard_msgs in #eval check F [⟨4, [Qa], .resolve 1 notPx xa [Qx] 2 Pa [] []⟩]

/-! ## And the claim behind the forgeries -/

/-- A structure in which `P` and `Q` hold of everything. It satisfies
`P(a)` and `¬P(x) ∨ Q(x)`, so those two alone cannot be refuted. -/
def allTrue : Struct Unit := ⟨fun _ _ => (), fun _ _ => True, ()⟩

/-- **A satisfiable clause set cannot be refuted, by any proof at all.**

Not an example: a theorem over every `Refutation`, which is what separates
"this forgery was caught" from "no forgery can succeed". -/
theorem no_refutation_of_a_satisfiable_set (prf : Refutation) :
    check [(1, [notPx, Qx]), (2, [Pa])] prf = false := by
  cases h : check [(1, [notPx, Qx]), (2, [Pa])] prf with
  | false => rfl
  | true =>
    refine absurd (unsat_of_check h Unit allTrue ?_) (by simp)
    intro q hq ρ
    rcases List.mem_cons.mp hq with heq | hin
    · subst heq; exact ⟨Qx, by simp, trivial⟩
    · have : q = (2, [Pa]) := by simpa using hin
      subst this
      exact ⟨Pa, by simp, trivial⟩

end Fo
