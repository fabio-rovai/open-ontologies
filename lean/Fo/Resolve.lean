import Fo.Clause

/-!
  The resolution rule, and the only theorem that matters about it.

  ## The shape of a step

  A step names two parent clauses, one literal in each, a substitution FOR EACH
  PARENT SEPARATELY, and the remainder of each parent. Two substitutions rather
  than one is what removes the need to standardise variables apart: each parent
  is instantiated on its own, so neither has to be renamed away from the other
  first. A clause is universally quantified, so any instance of it holds in any
  model of it, and `sat_applyC` is exactly that sentence.

  ## What the checker asks, and what it does not

  One question: are the two chosen literals, after their own substitutions, the
  same atom with opposite polarity? That is an equality on terms.

  No unifier is computed here and no theorem about most-generality is stated.
  Decision 0005 names "a verified first-order calculus with unification" as the
  missing piece; the unification half of that is avoidable, because a checker
  does not need an algorithm that FINDS a most general unifier. It needs to
  confirm that a substitution it was HANDED makes two literals complementary.
  Resolving on a unifier that is not most general is still sound; it just
  proves less, and proving less is the prover's problem.

  ## Covering, rather than deletion

  The remainder is supplied by the certificate and CHECKED by covering: every
  literal of the parent must be the resolved one or be in the remainder. Not
  the other way round, and not an exact deletion. Soundness needs no more: if a
  model satisfies the parent then some literal of it is true, and covering says
  that literal is accounted for. A remainder carrying extra literals only makes
  the conclusion weaker, which cannot make a refutation unsound.
-/

namespace Fo

/-- Every literal of `c` is `l`, or is in `rest`. -/
def coveredBy (c : Clause) (l : Lit) (rest : Clause) : Bool :=
  c.all (fun x => Lit.beq x l || rest.any (Lit.beq x))

theorem covered_cases {c : Clause} {l : Lit} {rest : Clause} {x : Lit}
    (h : coveredBy c l rest = true) (hx : x ∈ c) : x = l ∨ x ∈ rest := by
  have := List.all_eq_true.mp h x hx
  rcases Bool.or_eq_true _ _ |>.mp this with h1 | h2
  · exact Or.inl (Lit.eq_of_beq h1)
  · obtain ⟨y, hy, hxy⟩ := List.any_eq_true.mp h2
    exact Or.inr (Lit.eq_of_beq hxy ▸ hy)

/-- **Resolution is sound.** -/
theorem resolve_sound {α : Type} {M : Struct α} {c d restc restd : Clause}
    {σc σd : Subst} {lc ld : Lit}
    (hc : Sat M c) (hd : Sat M d)
    (hcv : coveredBy c lc restc = true) (hdv : coveredBy d ld restd = true)
    (hcomp : applyL σc lc = Lit.neg (applyL σd ld)) :
    Sat M (applyC σc restc ++ applyC σd restd) := by
  intro ρ
  obtain ⟨la, hla, hva⟩ := sat_applyC hc σc ρ
  obtain ⟨lb, hlb, hvb⟩ := sat_applyC hd σd ρ
  obtain ⟨la0, hla0, rfl⟩ := List.mem_map.mp hla
  obtain ⟨lb0, hlb0, rfl⟩ := List.mem_map.mp hlb
  rcases covered_cases hcv hla0 with rfl | hrest
  · rcases covered_cases hdv hlb0 with rfl | hrest2
    · -- Both resolved literals true at once, and they are complementary.
      rw [hcomp] at hva
      exact absurd hvb ((holdsL_neg M ρ (applyL σd lb0)).mp hva)
    · exact ⟨applyL σd lb0,
        List.mem_append.mpr (Or.inr (List.mem_map_of_mem hrest2)), hvb⟩
  · exact ⟨applyL σc la0,
      List.mem_append.mpr (Or.inl (List.mem_map_of_mem hrest)), hva⟩

/-- **Factoring is sound.** Two literals a substitution makes equal collapse to
one, and the clause still holds. -/
theorem factor_sound {α : Type} {M : Struct α} {c rest : Clause} {σ : Subst}
    {la lb : Lit} (h : Sat M c)
    (hcv : coveredBy c la rest = true) (hlb : lb ∈ rest)
    (heq : applyL σ la = applyL σ lb) :
    Sat M (applyC σ rest) := by
  intro ρ
  obtain ⟨l0, hl0, hv⟩ := sat_applyC h σ ρ
  obtain ⟨l1, hl1, rfl⟩ := List.mem_map.mp hl0
  rcases covered_cases hcv hl1 with rfl | hrest
  · rw [heq] at hv
    exact ⟨applyL σ lb, List.mem_map_of_mem hlb, hv⟩
  · exact ⟨applyL σ l1, List.mem_map_of_mem hrest, hv⟩

end Fo
