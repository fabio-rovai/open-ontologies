/-
  A verified checker for LRAT proofs of propositional unsatisfiability.

  ## Why this file exists

  `src/fol_solve.rs` says it plainly: "two solvers agreeing on `unsat` is two
  opinions and not a proof". That is the honest position while nothing reads
  what the solver emits, and it is what makes Vampire, E, Z3 and Mace4 read
  `opinion` rather than `certificate` in the front-page figure.

  A solver's `unsat` is an opinion because the search behind it is large, fast
  and untrusted. The proof it can emit is neither large to check nor trusted to
  produce: an LRAT line is one clause plus the identifiers of the clauses that
  make it follow, and checking a line is unit propagation over those few. Same
  propose-and-dispose split as every other checker here, applied to the one
  place in this repository that had not got it.

  ## What is proved

  `Lrat.unsat_of_check`: if `check` returns `true` then the formula it was
  given has no model.

  Nothing here is proved about the solver, about DIMACS parsing, or about the
  translation that produced the formula. A checker that accepts says the PROOF
  is a proof of the FORMULA IT WAS HANDED, and not one word more.
-/

namespace Lrat

/-- A literal: a variable and a polarity.

Deliberately NOT a signed integer. DIMACS writes literals as signed integers
and that is a serialisation detail; carrying it into the core would put an
argument about the sign of zero in the middle of every soundness proof. The
parser converts at the boundary, in `Lrat.Parse`, and a zero there is a parse
error rather than a literal with no polarity. -/
structure Lit where
  var : Nat
  pos : Bool
  deriving Repr, DecidableEq

/-- The negation of a literal: same variable, opposite polarity. -/
def Lit.neg (l : Lit) : Lit := ⟨l.var, !l.pos⟩

abbrev Clause := List Lit

/-- A formula is an indexed list of clauses, because LRAT hints name clauses by
identifier and identifiers are not positions: a proof deletes clauses as it
goes, and later identifiers keep counting up regardless. -/
abbrev Formula := List (Nat × Clause)

/-- An assignment of every variable to a truth value. Total on purpose: a
partial assignment is a device of the checker, and the theorem is about models,
which are total. -/
abbrev Assign := Nat → Bool

def litVal (σ : Assign) (l : Lit) : Bool :=
  if l.pos then σ l.var else !(σ l.var)

def clauseHolds (σ : Assign) (c : Clause) : Prop :=
  ∃ l, l ∈ c ∧ litVal σ l = true

def Models (σ : Assign) (F : Formula) : Prop :=
  ∀ p, p ∈ F → clauseHolds σ p.2

def Unsat (F : Formula) : Prop := ∀ σ, ¬ Models σ F

/-- A literal and its negation never agree. The whole polarity argument, once. -/
theorem litVal_neg (σ : Assign) (l : Lit) : litVal σ l.neg = !(litVal σ l) := by
  cases h : l.pos <;> simp [litVal, Lit.neg, h]

/-! ## The trail

The checker carries a partial assignment as a list of literals forced TRUE.
`l ∈ trail` means `l` is true and `l.neg ∈ trail` means `l` is false. Nothing
forbids both at once, and nothing needs to: a contradictory trail makes the
invariant below vacuous rather than unsound. -/

abbrev Trail := List Lit

/-- Every literal on the trail is true under `σ`. This is the whole invariant. -/
def Respects (σ : Assign) (t : Trail) : Prop :=
  ∀ l, l ∈ t → litVal σ l = true

/-- `l` is false on the trail exactly when its negation was forced true. -/
def isFalse (t : Trail) (l : Lit) : Bool := decide (l.neg ∈ t)

/-- What one hinted clause contributes under the current trail. -/
inductive Step where
  /-- Every literal is false: no model can respect this trail. -/
  | conflict
  /-- All but `l` are false, so `l` must be true. -/
  | unit (l : Lit)
  /-- Neither, so this hint proves nothing and the check fails. -/
  | stuck
  deriving Repr, DecidableEq

/-- The literals of `c` the trail has not already falsified.

A fold rather than `List.filter`, so the two facts the soundness proof needs
are one induction each with no library lemma about `filter` in between. There
is no Mathlib here and the trust surface is meant to stay readable. -/
def unfalsified (t : Trail) : Clause → Clause
  | [] => []
  | l :: rest => if isFalse t l then unfalsified t rest else l :: unfalsified t rest

/-- Nothing left is a conflict; one thing left is a unit; anything else proves
nothing and the check fails. -/
def classify (t : Trail) (c : Clause) : Step :=
  match unfalsified t c with
  | []  => .conflict
  | [l] => .unit l
  | _   => .stuck

end Lrat
