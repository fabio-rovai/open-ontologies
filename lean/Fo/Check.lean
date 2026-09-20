import Fo.Resolve

/-!
  The certificate, the checker, and `Fo.unsat_of_check`.

  A refutation is a list of steps. Each names its premises by identifier,
  supplies the substitutions and remainders it used, and states the clause it
  concludes. The checker replays each step and adds the stated conclusion to
  the set it carries. Reaching the empty clause means the original set has no
  model, over any carrier.
-/

namespace Fo

/-- A substitution given as a finite table. A variable the table does not
mention maps to itself. -/
def ofList (tbl : List (Nat × Term)) : Subst := fun n =>
  match tbl.find? (fun p => p.1 == n) with
  | some p => p.2
  | none => .var n

/-- One step of a refutation. -/
inductive Step where
  | resolve (ci : Nat) (lc : Lit) (σc : List (Nat × Term)) (restc : Clause)
            (dj : Nat) (ld : Lit) (σd : List (Nat × Term)) (restd : Clause)
  | factor (ci : Nat) (la : Lit) (lb : Lit) (σ : List (Nat × Term)) (rest : Clause)
  deriving Repr

structure Line where
  id : Nat
  concl : Clause
  step : Step
  deriving Repr

abbrev Refutation := List Line

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
      subst this; subst hp; simp
    · rw [if_neg hp] at h
      exact List.mem_cons_of_mem _ (ih h)

/-- Every literal of `a` is in `b`.

Used as `subsumes licensed stated`: everything the RULE gives must appear in
the conclusion the step STATES, and not the other way round. A stated
conclusion may carry extra literals, which only makes it weaker; it may never
DROP one, because dropping a literal claims more than the rule licensed. The
empty clause is the case that matters: stating it forces the licensed clause
to be empty too. -/
def subsumes (a b : Clause) : Bool := a.all (fun x => b.any (Lit.beq x))

theorem mem_of_subsumes {a b : Clause} (h : subsumes a b = true) {x : Lit}
    (hx : x ∈ a) : x ∈ b := by
  obtain ⟨y, hy, hxy⟩ := List.any_eq_true.mp (List.all_eq_true.mp h x hx)
  exact Lit.eq_of_beq hxy ▸ hy

theorem sat_of_subsumes {α : Type} {M : Struct α} {a b : Clause}
    (h : subsumes a b = true) (ha : Sat M a) : Sat M b := by
  intro ρ
  obtain ⟨l, hl, hv⟩ := ha ρ
  exact ⟨l, mem_of_subsumes h hl, hv⟩

/-! ## The two conditions

Split out from `checkStep` so each is a chain of `if`s over ordinary values,
with no `match` in it. The soundness proofs are then three `by_cases` and
nothing about what a multi-scrutinee match binds. -/

def checkResolve (c d : Clause) (lc ld : Lit) (σc σd : Subst)
    (restc restd : Clause) : Option Clause :=
  if coveredBy c lc restc then
    if coveredBy d ld restd then
      if Lit.beq (applyL σc lc) (Lit.neg (applyL σd ld)) then
        some (applyC σc restc ++ applyC σd restd)
      else none
    else none
  else none

def checkFactor (c : Clause) (la lb : Lit) (σ : Subst) (rest : Clause) : Option Clause :=
  if coveredBy c la rest then
    if rest.any (Lit.beq lb) then
      if Lit.beq (applyL σ la) (applyL σ lb) then
        some (applyC σ rest)
      else none
    else none
  else none

theorem checkResolve_sound {α : Type} {M : Struct α} {c d : Clause} {lc ld : Lit}
    {σc σd : Subst} {restc restd out : Clause}
    (hc : Sat M c) (hd : Sat M d)
    (h : checkResolve c d lc ld σc σd restc restd = some out) : Sat M out := by
  unfold checkResolve at h
  by_cases h1 : coveredBy c lc restc = true
  · rw [if_pos h1] at h
    by_cases h2 : coveredBy d ld restd = true
    · rw [if_pos h2] at h
      by_cases h3 : Lit.beq (applyL σc lc) (Lit.neg (applyL σd ld)) = true
      · rw [if_pos h3] at h
        have hout := (Option.some.inj h).symm
        subst hout
        exact resolve_sound hc hd h1 h2 (Lit.eq_of_beq h3)
      · rw [if_neg h3] at h; simp at h
    · rw [if_neg h2] at h; simp at h
  · rw [if_neg h1] at h; simp at h

theorem checkFactor_sound {α : Type} {M : Struct α} {c : Clause} {la lb : Lit}
    {σ : Subst} {rest out : Clause}
    (hc : Sat M c) (h : checkFactor c la lb σ rest = some out) : Sat M out := by
  unfold checkFactor at h
  by_cases h1 : coveredBy c la rest = true
  · rw [if_pos h1] at h
    by_cases h2 : rest.any (Lit.beq lb) = true
    · rw [if_pos h2] at h
      by_cases h3 : Lit.beq (applyL σ la) (applyL σ lb) = true
      · rw [if_pos h3] at h
        have hout := (Option.some.inj h).symm
        subst hout
        obtain ⟨y, hy, hxy⟩ := List.any_eq_true.mp h2
        exact factor_sound hc h1 (Lit.eq_of_beq hxy ▸ hy) (Lit.eq_of_beq h3)
      · rw [if_neg h3] at h; simp at h
    · rw [if_neg h2] at h; simp at h
  · rw [if_neg h1] at h; simp at h

/-- Check one step, returning the clause the rule licenses, or `none`. -/
def checkStep (F : Formula) : Step → Option Clause
  | .resolve ci lc σc restc dj ld σd restd =>
    match lookup F ci with
    | none => none
    | some c =>
      match lookup F dj with
      | none => none
      | some d => checkResolve c d lc ld (ofList σc) (ofList σd) restc restd
  | .factor ci la lb σ rest =>
    match lookup F ci with
    | none => none
    | some c => checkFactor c la lb (ofList σ) rest

theorem checkStep_sound {α : Type} {M : Struct α} {F : Formula} {st : Step}
    {out : Clause} (hm : Models M F) (h : checkStep F st = some out) : Sat M out := by
  cases st with
  | resolve ci lc σc restc dj ld σd restd =>
    simp only [checkStep] at h
    cases hc : lookup F ci with
    | none => rw [hc] at h; simp at h
    | some c =>
      rw [hc] at h
      cases hd : lookup F dj with
      | none => rw [hd] at h; simp at h
      | some d =>
        rw [hd] at h
        exact checkResolve_sound (hm (ci, c) (lookup_mem hc))
          (hm (dj, d) (lookup_mem hd)) h
  | factor ci la lb σ rest =>
    simp only [checkStep] at h
    cases hc : lookup F ci with
    | none => rw [hc] at h; simp at h
    | some c => rw [hc] at h; exact checkFactor_sound (hm (ci, c) (lookup_mem hc)) h

/-- Replay a refutation. Accepts exactly when a line states the empty clause,
having checked every line up to it. -/
def check (F : Formula) : Refutation → Bool
  | [] => false
  | ln :: rest =>
    match checkStep F ln.step with
    | none => false
    | some licensed =>
      if subsumes licensed ln.concl then
        if ln.concl.isEmpty then true
        else check ((ln.id, ln.concl) :: F) rest
      else false

theorem check_sound : ∀ (p : Refutation) {α : Type} (F : Formula) (M : Struct α),
    Models M F → check F p = true → False := by
  intro p
  induction p with
  | nil => intro α F M _ h; simp [check] at h
  | cons ln rest ih =>
    intro α F M hm h
    unfold check at h
    cases hstep : checkStep F ln.step with
    | none => rw [hstep] at h; simp at h
    | some licensed =>
      rw [hstep] at h
      simp only at h
      have hlic : Sat M licensed := checkStep_sound hm hstep
      by_cases hsub : subsumes licensed ln.concl = true
      · rw [if_pos hsub] at h
        have hcon : Sat M ln.concl := sat_of_subsumes hsub hlic
        by_cases he : ln.concl.isEmpty = true
        · have hnil : ln.concl = [] := by
            cases hc : ln.concl with
            | nil => rfl
            | cons a as => rw [hc] at he; simp at he
          obtain ⟨l, hl, _⟩ := hnil ▸ hcon (fun _ => M.elem)
          exact absurd hl (by simp)
        · rw [if_neg he] at h
          refine ih ((ln.id, ln.concl) :: F) M ?_ h
          intro q hq
          rcases List.mem_cons.mp hq with heq | hin
          · subst heq; exact hcon
          · exact hm q hin
      · rw [if_neg hsub] at h; simp at h

/-- **The theorem.** A refutation this checker accepts is a proof that the
clause set it was handed has no model, over any carrier. -/
theorem unsat_of_check {F : Formula} {p : Refutation} (h : check F p = true) :
    Unsat F := fun _ M hm => check_sound p F M hm h

end Fo
