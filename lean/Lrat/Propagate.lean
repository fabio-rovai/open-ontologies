import Lrat.Basic

/-!
  The two facts every LRAT check rests on, and nothing else.

  `conflict_sound`: a clause of `F` whose literals the trail has all falsified
  rules out every model that respects the trail.

  `unit_sound`: a clause of `F` with exactly one literal left forces that
  literal true in every model that respects the trail.

  Everything above these is induction.
-/

namespace Lrat

theorem isFalse_iff {t : Trail} {l : Lit} : isFalse t l = true ↔ l.neg ∈ t := by
  simp [isFalse]

/-- A literal the trail calls false is false under any model respecting it.
One line, because `Lit` carries its polarity instead of encoding it in a sign. -/
theorem false_of_isFalse {σ : Assign} {t : Trail} (hr : Respects σ t)
    {l : Lit} (hf : isFalse t l = true) : litVal σ l = false := by
  have hneg : litVal σ l.neg = true := hr _ (isFalse_iff.mp hf)
  rw [litVal_neg] at hneg
  cases h : litVal σ l with
  | false => rfl
  | true => rw [h] at hneg; exact absurd hneg (by simp)

/-- Nothing survives `unfalsified` that the trail has falsified, and nothing
the trail has left alone is dropped. Both directions, one induction. -/
theorem mem_unfalsified {t : Trail} {c : Clause} {l : Lit} :
    l ∈ unfalsified t c ↔ (l ∈ c ∧ isFalse t l = false) := by
  induction c with
  | nil => simp [unfalsified]
  | cons a rest ih =>
    unfold unfalsified
    by_cases ha : isFalse t a = true
    · rw [if_pos ha, ih]
      constructor
      · intro h
        exact ⟨List.mem_cons_of_mem _ h.1, h.2⟩
      · intro h
        rcases List.mem_cons.mp h.1 with heq | hin
        · subst heq
          rw [ha] at h
          exact absurd h.2 (by simp)
        · exact ⟨hin, h.2⟩
    · rw [if_neg ha]
      simp only [List.mem_cons]
      constructor
      · intro h
        rcases h with heq | hin
        · subst heq
          exact ⟨Or.inl rfl, by simpa using ha⟩
        · exact ⟨Or.inr (ih.mp hin).1, (ih.mp hin).2⟩
      · intro h
        rcases h.1 with heq | hin
        · exact Or.inl heq
        · exact Or.inr (ih.mpr ⟨hin, h.2⟩)

/-- **Conflict is sound.** -/
theorem conflict_sound {σ : Assign} {t : Trail} {c : Clause}
    (hr : Respects σ t) (hc : classify t c = .conflict)
    (hsat : clauseHolds σ c) : False := by
  have hnil : unfalsified t c = [] := by
    unfold classify at hc
    split at hc
    · assumption
    · split at hc <;> exact absurd hc (by simp)
  obtain ⟨l, hl, hval⟩ := hsat
  by_cases hf : isFalse t l
  · rw [false_of_isFalse hr hf] at hval; exact Bool.noConfusion hval
  · have : l ∈ unfalsified t c := mem_unfalsified.mpr ⟨hl, by simpa using hf⟩
    rw [hnil] at this
    exact absurd this (by simp)

/-- **Unit is sound.** Every literal of `c` is either false on the trail or is
`l` itself, so a model respecting the trail has to make `l` true. -/
theorem unit_sound {σ : Assign} {t : Trail} {c : Clause} {l : Lit}
    (hr : Respects σ t) (hc : classify t c = .unit l)
    (hsat : clauseHolds σ c) : litVal σ l = true := by
  -- The shape the classifier saw: a non-empty remainder, every element of it
  -- equal to the head.
  have hshape : ∃ rest, unfalsified t c = l :: rest ∧ rest.all (· == l) = true := by
    unfold classify at hc
    split at hc
    · exact absurd hc (by simp)
    · next hd rest heq =>
      split at hc
      · next hall =>
        have : hd = l := by simpa using hc
        subst this
        exact ⟨rest, heq, hall⟩
      · exact absurd hc (by simp)
  obtain ⟨rest, heq, hall⟩ := hshape
  obtain ⟨m, hm, hval⟩ := hsat
  by_cases hf : isFalse t m
  · rw [false_of_isFalse hr hf] at hval; exact Bool.noConfusion hval
  · have hmem : m ∈ unfalsified t c := mem_unfalsified.mpr ⟨hm, by simpa using hf⟩
    rw [heq] at hmem
    rcases List.mem_cons.mp hmem with h | h
    · subst h; exact hval
    · have : m = l := by
        have := List.all_eq_true.mp hall m h
        simpa using this
      subst this
      exact hval

/-- Extending a respected trail with a literal the model makes true keeps it
respected. -/
theorem respects_cons {σ : Assign} {t : Trail} {l : Lit}
    (hr : Respects σ t) (hl : litVal σ l = true) : Respects σ (l :: t) := by
  intro x hx
  rcases List.mem_cons.mp hx with h | h
  · subst h; exact hl
  · exact hr x h

end Lrat
