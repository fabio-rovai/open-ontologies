/-
  First-order terms, substitution, and the one lemma everything rests on.

  ## The design decision that makes this tractable

  Decision 0005 says checking a superposition proof "needs a verified
  first-order calculus with unification that does not exist in core Lean". The
  unification half of that is avoidable, and avoiding it is what brings the
  work inside reach.

  A checker does not need an algorithm that FINDS a most general unifier. It
  needs to confirm that a substitution it was HANDED makes two literals
  complementary, and that is a decidable equality on terms. The prover searches;
  the certificate carries the substitution it used; the checker applies it and
  compares. Most-generality is a property of a good search, not a premise of
  soundness: resolving on a non-most-general unifier is still sound, it just
  proves less.

  So there is no `mgu` in this development, and no theorem about one. There is
  `applySubst`, `==`, and the substitution lemma below.
-/

namespace Fo

/-- A first-order term. Function symbols and variables are numbered; arity is
not tracked, because a structure interprets a symbol at whatever list of
arguments it is given and nothing here needs arity to be well formed. -/
inductive Term where
  | var (n : Nat)
  | app (f : Nat) (args : List Term)
  deriving Repr, Inhabited

/-! ## Equality

Written out rather than derived: `Term` is a nested inductive, through
`List Term`, and the `DecidableEq` handler does not reach through the list. The
checker needs a Bool, and soundness needs `beq_eq` below, so both are here. -/

mutual
def Term.beq : Term → Term → Bool
  | .var a, .var b => a == b
  | .app f as, .app g bs => f == g && Term.beqs as bs
  | _, _ => false

def Term.beqs : List Term → List Term → Bool
  | [], [] => true
  | a :: as, b :: bs => Term.beq a b && Term.beqs as bs
  | _, _ => false
end

instance : BEq Term := ⟨Term.beq⟩

mutual
theorem Term.eq_of_beq : ∀ {a b : Term}, Term.beq a b = true → a = b
  | .var _, .var _, h => by simp [Term.beq] at h; simp [h]
  | .app _ as, .app _ bs, h => by
    simp only [Term.beq, Bool.and_eq_true, beq_iff_eq] at h
    rw [h.1, Term.eqs_of_beqs h.2]
  | .var _, .app _ _, h => by simp [Term.beq] at h
  | .app _ _, .var _, h => by simp [Term.beq] at h

theorem Term.eqs_of_beqs : ∀ {as bs : List Term}, Term.beqs as bs = true → as = bs
  | [], [], _ => rfl
  | a :: as, b :: bs, h => by
    simp only [Term.beqs, Bool.and_eq_true] at h
    rw [Term.eq_of_beq h.1, Term.eqs_of_beqs h.2]
  | [], _ :: _, h => by simp [Term.beqs] at h
  | _ :: _, [], h => by simp [Term.beqs] at h
end

/-- A substitution: a total map from variable numbers to terms. Total rather
than finite, so composing two of them needs no bookkeeping; a certificate
supplies a finite table and `ofList` turns it into one of these. -/
abbrev Subst := Nat → Term

def idSubst : Subst := Term.var

mutual
/-- Apply a substitution throughout a term. -/
def applyT (σ : Subst) : Term → Term
  | .var n => σ n
  | .app f args => .app f (applyTs σ args)

def applyTs (σ : Subst) : List Term → List Term
  | [] => []
  | t :: ts => applyT σ t :: applyTs σ ts
end

/-- A first-order structure over a carrier.

`fn` and `rel` are total on lists of arguments. A partial, arity-respecting
version would be more faithful to a textbook and would put an arity side
condition in the middle of every proof below; nothing here needs one, because
the syntax and the semantics agree on how many arguments a symbol was given. -/
structure Struct (α : Type) where
  fn : Nat → List α → α
  rel : Nat → List α → Prop
  /-- The domain is NON-EMPTY, which standard first-order semantics requires
  and which is not a technicality here. `Sat M c` quantifies over every
  environment, and over an empty carrier there are no environments, so the
  EMPTY CLAUSE would be vacuously satisfied and every refutation would prove
  nothing. Carrying a witness in the structure is the cheapest way to say what
  a first-order model is. -/
  elem : α

/-- An assignment of a carrier element to every variable. -/
abbrev Env (α : Type) := Nat → α

mutual
def evalT {α : Type} (M : Struct α) (ρ : Env α) : Term → α
  | .var n => ρ n
  | .app f args => M.fn f (evalTs M ρ args)

def evalTs {α : Type} (M : Struct α) (ρ : Env α) : List Term → List α
  | [] => []
  | t :: ts => evalT M ρ t :: evalTs M ρ ts
end

/-! ## The substitution lemma

Evaluating a substituted term is evaluating the original term in the
environment the substitution describes. Everything in `Fo/Resolve.lean` is this
lemma plus case analysis. -/

mutual
theorem evalT_applyT {α : Type} (M : Struct α) (ρ : Env α) (σ : Subst) :
    ∀ t : Term, evalT M ρ (applyT σ t) = evalT M (fun n => evalT M ρ (σ n)) t
  | .var n => by simp [applyT, evalT]
  | .app f args => by
    simp only [applyT, evalT]
    rw [evalTs_applyTs M ρ σ args]

theorem evalTs_applyTs {α : Type} (M : Struct α) (ρ : Env α) (σ : Subst) :
    ∀ ts : List Term, evalTs M ρ (applyTs σ ts) = evalTs M (fun n => evalT M ρ (σ n)) ts
  | [] => by simp [applyTs, evalTs]
  | t :: ts => by
    simp only [applyTs, evalTs]
    rw [evalT_applyT M ρ σ t, evalTs_applyTs M ρ σ ts]
end

end Fo
