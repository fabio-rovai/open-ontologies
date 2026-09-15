import OOBoundary.Spec

/-!
# The boundary functions do what the certificate format needs

Every theorem here is about `OOBoundary.boundary_core.*`, which is Aeneas's
translation of `src/boundary_core.rs`, and every one is stated for EVERY input.
That is the only reason this directory exists.
`tests/certificate_boundary_proptest.rs` samples these properties and the Kani
harnesses in `src/reason.rs` prove several of them for every byte pattern at a
term length of at most four bytes. Nothing below has a length bound
and nothing below has a shape bound.

The decision procedures are stated as equations `f x = ok (spec x)` rather than
as Hoare triples. An equation says three things at once where a triple says one:
the model never fails, it never diverges, and its result is the specification.
For a function whose whole job is to decide whether a byte string is safe to
write, "it returns the right answer and it always returns" is the claim worth
making.

The proof style is deliberately explicit. The first attempt drove the loop
inductions with large `simp` calls and they either failed to fire or ran out of
steps inside the `partial_fixpoint` unfolding of `loop`. What works is to prove
one equation per BODY outcome first, with no `loop` in sight, and then let the
induction rewrite with those. It is more lines and it is predictable.
-/

-- `autoImplicit` turns a typo into a universe-polymorphic variable and the
-- error into a puzzle three screens away. It also hid, for one build, the fact
-- that `ok` was not in scope.
set_option autoImplicit false

-- The generated file sets these too, and the setting is file-local. The loop
-- inductions rewrite under a `partial_fixpoint` unfolding, which is what needs
-- the depth.
set_option maxRecDepth 40000
set_option maxHeartbeats 1000000

namespace OOBoundary

open Aeneas Aeneas.Std Result ControlFlow Error OOBoundary.Spec

/-! ## The leading-byte tests -/

/-- `starts_with_byte` is the head of the list, on EVERY input including the
empty one. The empty case is the one the bounded harness cannot reach:
`writable_triple_decides_both_positions` fixes both terms at four bytes, so it
never evaluates this function on an empty slice at all. -/
theorem starts_with_byte_eq (s : Slice U8) (b : U8) :
    boundary_core.starts_with_byte s b = ok (decide (s.val[0]? = some b)) := by
  unfold boundary_core.starts_with_byte
  cases hs : s.val with
  | nil => simp [core.slice.Slice.is_empty, Slice.length, hs]
  | cons x xs =>
    have hne : ¬ (s.length = 0) := by simp [Slice.length, hs]
    have hidx : Slice.index_usize s 0#usize = ok x := by
      simp [Slice.index_usize, hs]
    simp [core.slice.Slice.is_empty, hidx, hs]

/-- The guard that closed the unwritable-conclusion defect decides both
positions for terms of EVERY length, the empty term included.
`writable_triple_decides_both_positions` (Kani) proves this at exactly four
bytes per term; this is the same statement with the bound removed.

Note what it says about an EMPTY subject: `"".starts_with('"')` is false, so an
empty subject passes. That is not a hole, because `term_fits_the_format` refuses
an empty term and every certificate path runs both guards, but it is exactly the
kind of edge a fixed-length model check cannot see. -/
theorem writable_triple_bytes_eq (subject predicate : Slice U8) :
    boundary_core.writable_triple_bytes subject predicate
      = ok (writableTriple subject.val predicate.val) := by
  unfold boundary_core.writable_triple_bytes writableTriple
  simp only [starts_with_byte_eq]
  by_cases h : subject.val[0]? = some 34#u8 <;> simp [h]

/-! ## The separator scans

Two loops and two inductions, and the inductions are the whole content. A
property test draws a few hundred byte strings and a Kani harness fixes the
length; these cover every list of bytes there is. -/

section FieldScan

private theorem field_body_end (f : Slice U8) (i : Usize) (hge : f.val.length ≤ i.val) :
    boundary_core.field_fits_the_format_loop.body f i = ok (done true) := by
  unfold boundary_core.field_fits_the_format_loop.body
  have hcmp : ¬ (i < Slice.len f) := by simp only [Slice.len]; scalar_tac
  rw [if_neg hcmp]

private theorem field_body_sep (f : Slice U8) (i : Usize) (hlt : i.val < f.val.length)
    (hsep : isTsvSeparator f.val[i.val] = true) :
    boundary_core.field_fits_the_format_loop.body f i = ok (done false) := by
  unfold boundary_core.field_fits_the_format_loop.body
  have hcmp : (i < Slice.len f) := by simp only [Slice.len]; scalar_tac
  obtain ⟨xi, hidx, hxi⟩ :=
    WP.spec_imp_exists (Slice.index_usize_spec f i (by scalar_tac))
  rw [← hxi] at hsep
  unfold isTsvSeparator at hsep
  simp only [Bool.or_eq_true, decide_eq_true_eq] at hsep
  have hcases : xi = 9#u8 ∨ xi = 10#u8 ∨ xi = 13#u8 := by tauto
  rcases hcases with h | h | h <;> simp [hcmp, hidx, h]

private theorem field_body_step (f : Slice U8) (i : Usize) (hlt : i.val < f.val.length)
    (hsep : isTsvSeparator f.val[i.val] = false) :
    ∃ j : Usize, j.val = i.val + 1
      ∧ boundary_core.field_fits_the_format_loop.body f i = ok (cont j) := by
  unfold boundary_core.field_fits_the_format_loop.body
  have hcmp : (i < Slice.len f) := by simp only [Slice.len]; scalar_tac
  obtain ⟨xi, hidx, hxi⟩ :=
    WP.spec_imp_exists (Slice.index_usize_spec f i (by scalar_tac))
  rw [← hxi] at hsep
  have hmax : i.val + (1#usize).val ≤ Usize.max := by
    have hb := f.property; scalar_tac
  obtain ⟨j, hj, hjv⟩ := WP.spec_imp_exists (Usize.add_spec (x := i) (y := 1#usize) hmax)
  refine ⟨j, by simpa using hjv, ?_⟩
  unfold isTsvSeparator at hsep
  simp only [Bool.or_eq_false_iff, decide_eq_false_iff_not] at hsep
  obtain ⟨⟨h9, h10⟩, h13⟩ := hsep
  simp [hcmp, hidx, h9, h10, h13, hj]

private theorem field_loop_eq (f : Slice U8) :
    ∀ (k : Nat) (i : Usize), f.val.length - i.val ≤ k →
      boundary_core.field_fits_the_format_loop f i
        = ok ((f.val.drop i.val).all (fun b => !isTsvSeparator b)) := by
  intro k
  induction k with
  | zero =>
    intro i hk
    have hge : f.val.length ≤ i.val := by omega
    unfold boundary_core.field_fits_the_format_loop
    rw [loop]
    simp only [field_body_end f i hge, bind_ok]
    rw [List.drop_eq_nil_of_le hge]
    simp
  | succ k ih =>
    intro i hk
    by_cases hlt : i.val < f.val.length
    · have hdrop : f.val.drop i.val = f.val[i.val] :: f.val.drop (i.val + 1) :=
        List.drop_eq_getElem_cons hlt
      by_cases hsep : isTsvSeparator f.val[i.val] = true
      · unfold boundary_core.field_fits_the_format_loop
        rw [loop]
        simp only [field_body_sep f i hlt hsep, bind_ok, hdrop, List.all_cons, hsep,
          Bool.not_true, Bool.false_and]
      · simp only [Bool.not_eq_true] at hsep
        obtain ⟨j, hjv, hbody⟩ := field_body_step f i hlt hsep
        have hrec := ih j (by omega)
        unfold boundary_core.field_fits_the_format_loop at hrec ⊢
        rw [loop]
        simp only [hbody, bind_ok, hrec, hjv, hdrop, List.all_cons, hsep,
          Bool.not_false, Bool.true_and]
    · have hge : f.val.length ≤ i.val := by omega
      unfold boundary_core.field_fits_the_format_loop
      rw [loop]
      simp only [field_body_end f i hge, bind_ok]
      rw [List.drop_eq_nil_of_le hge]
      simp

end FieldScan

/-- TCB-4, unbounded: the format half of the term guard accepts a field IF AND
ONLY IF it is non-empty and carries no tab, line feed or carriage return. -/
theorem field_fits_the_format_eq (f : Slice U8) :
    boundary_core.field_fits_the_format f = ok (fieldFits f.val) := by
  unfold boundary_core.field_fits_the_format fieldFits
  cases hf : f.val with
  | nil => simp [core.slice.Slice.is_empty, Slice.length, hf]
  | cons x xs =>
    have hne : ¬ (f.length = 0) := by simp [Slice.length, hf]
    have hloop := field_loop_eq f f.val.length 0#usize (by simp)
    simp [core.slice.Slice.is_empty, hloop, hf]

/-- The `iff` form, which is what "accepts exactly the fields that survive the
format" means, and what the property test can only sample. -/
theorem field_fits_the_format_iff (f : Slice U8) :
    boundary_core.field_fits_the_format f = ok true
      ↔ (f.val ≠ [] ∧ ∀ b ∈ f.val, b ≠ 9#u8 ∧ b ≠ 10#u8 ∧ b ≠ 13#u8) := by
  rw [field_fits_the_format_eq, Result.ok.injEq]
  constructor
  · intro hb
    unfold fieldFits at hb
    simp only [Bool.and_eq_true, Bool.not_eq_true', List.isEmpty_eq_false_iff,
      List.all_eq_true] at hb
    obtain ⟨hne, hall⟩ := hb
    refine ⟨hne, fun b hbm => ?_⟩
    have := hall b hbm
    simp only [isTsvSeparator, Bool.or_eq_false_iff,
      decide_eq_false_iff_not] at this
    exact ⟨this.1.1, this.1.2, this.2⟩
  · rintro ⟨hne, hall⟩
    have : fieldFits f.val = true := by
      unfold fieldFits
      simp only [Bool.and_eq_true, Bool.not_eq_true', List.isEmpty_eq_false_iff,
        List.all_eq_true]
      refine ⟨hne, fun b hbm => ?_⟩
      obtain ⟨h9, h10, h13⟩ := hall b hbm
      simp [isTsvSeparator, h9, h10, h13]
    rw [this]

section NameScan

private theorem name_body_end (f : Slice U8) (i : Usize) (hge : f.val.length ≤ i.val) :
    boundary_core.name_is_safe_bytes_loop.body f i = ok (done true) := by
  unfold boundary_core.name_is_safe_bytes_loop.body
  have hcmp : ¬ (i < Slice.len f) := by simp only [Slice.len]; scalar_tac
  rw [if_neg hcmp]

private theorem name_body_sep (f : Slice U8) (i : Usize) (hlt : i.val < f.val.length)
    (hsep : isDlSeparator f.val[i.val] = true) :
    boundary_core.name_is_safe_bytes_loop.body f i = ok (done false) := by
  unfold boundary_core.name_is_safe_bytes_loop.body
  have hcmp : (i < Slice.len f) := by simp only [Slice.len]; scalar_tac
  obtain ⟨xi, hidx, hxi⟩ :=
    WP.spec_imp_exists (Slice.index_usize_spec f i (by scalar_tac))
  rw [← hxi] at hsep
  unfold isDlSeparator isTsvSeparator at hsep
  simp only [Bool.or_eq_true, decide_eq_true_eq] at hsep
  have hcases : xi = 32#u8 ∨ xi = 9#u8 ∨ xi = 10#u8 ∨ xi = 13#u8 := by tauto
  rcases hcases with h | h | h | h <;> simp [hcmp, hidx, h]

private theorem name_body_step (f : Slice U8) (i : Usize) (hlt : i.val < f.val.length)
    (hsep : isDlSeparator f.val[i.val] = false) :
    ∃ j : Usize, j.val = i.val + 1
      ∧ boundary_core.name_is_safe_bytes_loop.body f i = ok (cont j) := by
  unfold boundary_core.name_is_safe_bytes_loop.body
  have hcmp : (i < Slice.len f) := by simp only [Slice.len]; scalar_tac
  obtain ⟨xi, hidx, hxi⟩ :=
    WP.spec_imp_exists (Slice.index_usize_spec f i (by scalar_tac))
  rw [← hxi] at hsep
  have hmax : i.val + (1#usize).val ≤ Usize.max := by
    have hb := f.property; scalar_tac
  obtain ⟨j, hj, hjv⟩ := WP.spec_imp_exists (Usize.add_spec (x := i) (y := 1#usize) hmax)
  refine ⟨j, by simpa using hjv, ?_⟩
  unfold isDlSeparator isTsvSeparator at hsep
  simp only [Bool.or_eq_false_iff, decide_eq_false_iff_not] at hsep
  obtain ⟨h32, ⟨h9, h10⟩, h13⟩ := hsep
  simp [hcmp, hidx, h32, h9, h10, h13, hj]

private theorem name_loop_eq (f : Slice U8) :
    ∀ (k : Nat) (i : Usize), f.val.length - i.val ≤ k →
      boundary_core.name_is_safe_bytes_loop f i
        = ok ((f.val.drop i.val).all (fun b => !isDlSeparator b)) := by
  intro k
  induction k with
  | zero =>
    intro i hk
    have hge : f.val.length ≤ i.val := by omega
    unfold boundary_core.name_is_safe_bytes_loop
    rw [loop]
    simp only [name_body_end f i hge, bind_ok]
    rw [List.drop_eq_nil_of_le hge]
    simp
  | succ k ih =>
    intro i hk
    by_cases hlt : i.val < f.val.length
    · have hdrop : f.val.drop i.val = f.val[i.val] :: f.val.drop (i.val + 1) :=
        List.drop_eq_getElem_cons hlt
      by_cases hsep : isDlSeparator f.val[i.val] = true
      · unfold boundary_core.name_is_safe_bytes_loop
        rw [loop]
        simp only [name_body_sep f i hlt hsep, bind_ok, hdrop, List.all_cons, hsep,
          Bool.not_true, Bool.false_and]
      · simp only [Bool.not_eq_true] at hsep
        obtain ⟨j, hjv, hbody⟩ := name_body_step f i hlt hsep
        have hrec := ih j (by omega)
        unfold boundary_core.name_is_safe_bytes_loop at hrec ⊢
        rw [loop]
        simp only [hbody, bind_ok, hrec, hjv, hdrop, List.all_cons, hsep,
          Bool.not_false, Bool.true_and]
    · have hge : f.val.length ≤ i.val := by omega
      unfold boundary_core.name_is_safe_bytes_loop
      rw [loop]
      simp only [name_body_end f i hge, bind_ok]
      rw [List.drop_eq_nil_of_le hge]
      simp

end NameScan

/-- TCB-25, unbounded. The DL certificate refuses a SPACE as well as the three
bytes the rest of the format refuses, because `concept_string` in
`src/tableaux.rs` is space separated inside a field that is tab separated
outside it. -/
theorem name_is_safe_bytes_eq (n : Slice U8) :
    boundary_core.name_is_safe_bytes n = ok (nameIsSafe n.val) := by
  unfold boundary_core.name_is_safe_bytes nameIsSafe
  cases hn : n.val with
  | nil => simp [core.slice.Slice.is_empty, Slice.length, hn]
  | cons x xs =>
    have hne : ¬ (n.length = 0) := by simp [Slice.length, hn]
    have hloop := name_loop_eq n n.val.length 0#usize (by simp)
    simp [core.slice.Slice.is_empty, hloop, hn]

/-- TCB-25 as an `iff`: `name_is_safe` accepts a name IF AND ONLY IF the name is
non-empty and contains no space, tab, line feed or carriage return. This is the
theorem the sampled cases in `tcb_25_name_is_safe_refuses_every_separator` can
only support. -/
theorem name_is_safe_bytes_iff (n : Slice U8) :
    boundary_core.name_is_safe_bytes n = ok true
      ↔ (n.val ≠ [] ∧ ∀ b ∈ n.val, b ≠ 32#u8 ∧ b ≠ 9#u8 ∧ b ≠ 10#u8 ∧ b ≠ 13#u8) := by
  rw [name_is_safe_bytes_eq, Result.ok.injEq]
  constructor
  · intro hb
    unfold nameIsSafe at hb
    simp only [Bool.and_eq_true, Bool.not_eq_true', List.isEmpty_eq_false_iff,
      List.all_eq_true] at hb
    obtain ⟨hne, hall⟩ := hb
    refine ⟨hne, fun b hbm => ?_⟩
    have := hall b hbm
    simp only [isDlSeparator, isTsvSeparator, Bool.or_eq_false_iff,
      decide_eq_false_iff_not] at this
    exact ⟨this.1, this.2.1.1, this.2.1.2, this.2.2⟩
  · rintro ⟨hne, hall⟩
    have : nameIsSafe n.val = true := by
      unfold nameIsSafe
      simp only [Bool.and_eq_true, Bool.not_eq_true', List.isEmpty_eq_false_iff,
        List.all_eq_true]
      refine ⟨hne, fun b hbm => ?_⟩
      obtain ⟨h32, h9, h10, h13⟩ := hall b hbm
      simp [isDlSeparator, isTsvSeparator, h32, h9, h10, h13]
    rw [this]

/-! ## The term guard -/

/-- TCB-4 and TCB-5 together, unbounded: a term is written IF AND ONLY IF it is
non-empty, carries no separator, and is in one of the three N-Triples spellings.
-/
theorem term_fits_the_format_eq (t : Slice U8) :
    boundary_core.term_fits_the_format t = ok (termFits t.val) := by
  unfold boundary_core.term_fits_the_format termFits ntriplesLeadingByte
  rw [field_fits_the_format_eq]
  by_cases hf : fieldFits t.val = true
  · have hpos : 0 < t.val.length := by
      unfold fieldFits at hf
      simp only [Bool.and_eq_true, Bool.not_eq_true', List.isEmpty_eq_false_iff] at hf
      cases h : t.val with
      | nil => exact absurd h hf.1
      | cons _ _ => simp
    obtain ⟨x0, h0, hx0⟩ :=
      WP.spec_imp_exists (Slice.index_usize_spec t 0#usize (by scalar_tac))
    have hg0 : t.val[0]! = x0 := by
      rw [hx0]; simp [hpos]
    rw [bind_tc_ok, if_pos (by simpa using hf), h0, bind_tc_ok, hf, Bool.true_and, hg0]
    by_cases h60 : x0 = 60#u8
    · rw [if_pos h60]; simp [h60]
    · by_cases h34 : x0 = 34#u8
      · rw [if_neg h60, if_pos h34]; simp [h34]
      · by_cases h95 : x0 = 95#u8
        · by_cases hlen : 1 < t.val.length
          · obtain ⟨x1, h1, hx1⟩ :=
              WP.spec_imp_exists (Slice.index_usize_spec t 1#usize (by scalar_tac))
            have hg1 : t.val[1]! = x1 := by
              rw [hx1]; simp [hlen]
            have hgt : (Slice.len t > 1#usize) := by simp only [Slice.len]; scalar_tac
            rw [if_neg h60, if_neg h34, if_pos h95, if_pos hgt, h1, bind_tc_ok]
            simp [h95, hlen, hx1]
          · have hgt : ¬ (Slice.len t > 1#usize) := by simp only [Slice.len]; scalar_tac
            rw [if_neg h60, if_neg h34, if_pos h95, if_neg hgt]
            simp [h95, hlen]
        · rw [if_neg h60, if_neg h34, if_neg h95]
          simp [h60, h34, h95]
  · simp only [Bool.not_eq_true] at hf
    rw [bind_tc_ok, if_neg (by simp [hf]), hf]
    simp

/-! ## The writers

Three claims: what the buffer gains when the writer accepts, what it does NOT
gain when the writer refuses, and what a reader splitting on the separators gets
back. The middle one is the all-or-nothing discipline, and it is the one that
cannot be seen by looking at the output file, because a half-line in a buffer
looks like a whole line to everything downstream. -/

private theorem clone_any (cl : U8 → Result U8) (hcl : ∀ x, cl x = ok x)
    (s : Slice U8) : Slice.clone cl s = ok s := by
  obtain ⟨s', he, hp⟩ :=
    WP.spec_imp_exists (Slice.clone_spec (clone := cl) (s := s) (fun x _ => hcl x))
  rw [he, ← hp]

/-- Both spellings of the `u8` clone, because `core.clone.CloneU8` is reducible
and the goal carries whichever one the elaborator left behind. -/
private theorem clone_u8_inst (s : Slice U8) :
    Slice.clone core.clone.CloneU8.clone s = ok s := clone_any _ (fun _ => rfl) s

private theorem clone_u8_lifted (s : Slice U8) :
    Slice.clone (liftFun1 core.clone.impls.CloneU8.clone) s = ok s :=
  clone_any _ (fun _ => rfl) s

private theorem extend_u8 (v : alloc.vec.Vec U8) (s : Slice U8)
    (h : v.val.length + s.val.length ≤ Usize.max) :
    ∃ v', alloc.vec.Vec.extend_from_slice core.clone.CloneU8 v s = ok v'
          ∧ v'.val = v.val ++ s.val := by
  unfold alloc.vec.Vec.extend_from_slice
  have hd : v.length + s.length ≤ Usize.max := by
    simpa [alloc.vec.Vec.length, Slice.length] using h
  rw [dif_pos hd]
  -- The `match h' : ... with` is DEPENDENT, so neither `rw` nor `simp` can
  -- reach the discriminant. `split` can, and the equation it hands back is an
  -- ordinary hypothesis that `rw` has no trouble with.
  split
  next s' heq =>
    rw [clone_u8_lifted, Result.match.ok] at heq
    injection heq with hs'
    subst hs'
    exact ⟨_, rfl, by simp⟩
  next eff k heq =>
    rw [clone_u8_lifted, Result.match.ok] at heq
    simp at heq
  next heq =>
    rw [clone_u8_lifted, Result.match.ok] at heq
    simp at heq

private theorem push_u8 (v : alloc.vec.Vec U8) (x : U8) (h : v.val.length < Usize.max) :
    ∃ v', alloc.vec.Vec.push v x = ok v' ∧ v'.val = v.val ++ [x] :=
  WP.spec_imp_exists (alloc.vec.Vec.push_spec v x h)

/-- When every term fits the format, the buffer gains exactly
`s TAB p TAB o LF` and nothing else. Every length, every byte pattern.
`asserted_line_round_trips_at_0` to `_at_4` (Kani) prove this at term lengths
zero to four. -/
theorem push_asserted_line_bytes_ok (out : alloc.vec.Vec U8) (s p o : Slice U8)
    (hs : termFits s.val = true) (hp : termFits p.val = true) (ho : termFits o.val = true)
    (hcap : out.val.length + s.val.length + p.val.length + o.val.length + 2 < Usize.max) :
    ∃ out', boundary_core.push_asserted_line_bytes out s p o
              = ok (core.result.Result.Ok (), out')
            ∧ out'.val = out.val ++ assertedLine s.val p.val o.val := by
  unfold boundary_core.push_asserted_line_bytes
  rw [term_fits_the_format_eq, bind_tc_ok, if_pos hs,
      term_fits_the_format_eq, bind_tc_ok, if_pos hp,
      term_fits_the_format_eq, bind_tc_ok, if_pos ho]
  -- `omega` cannot see through `Usize.max`, so it is never shown it: every
  -- capacity side condition below goes through these two, and the arithmetic
  -- `omega` does is over list lengths alone.
  have hlt' : ∀ n : Nat,
      n ≤ out.val.length + s.val.length + p.val.length + o.val.length + 2 → n < Usize.max :=
    fun n hn => Nat.lt_of_le_of_lt hn hcap
  have hle' : ∀ n : Nat,
      n ≤ out.val.length + s.val.length + p.val.length + o.val.length + 2 → n ≤ Usize.max :=
    fun n hn => Nat.le_of_lt (Nat.lt_of_le_of_lt hn hcap)
  obtain ⟨v1, h1, e1⟩ := extend_u8 out s (hle' _ (by omega))
  obtain ⟨v2, h2, e2⟩ := push_u8 v1 9#u8 (by simp only [e1, List.length_append]; exact hlt' _ (by omega))
  obtain ⟨v3, h3, e3⟩ := extend_u8 v2 p (by simp only [e2, e1, List.length_append, List.length_cons, List.length_nil]; exact hle' _ (by omega))
  obtain ⟨v4, h4, e4⟩ := push_u8 v3 9#u8 (by simp only [e3, e2, e1, List.length_append, List.length_cons, List.length_nil]; exact hlt' _ (by omega))
  obtain ⟨v5, h5, e5⟩ := extend_u8 v4 o (by simp only [e4, e3, e2, e1, List.length_append, List.length_cons, List.length_nil]; exact hle' _ (by omega))
  obtain ⟨v6, h6, e6⟩ := push_u8 v5 10#u8 (by simp only [e5, e4, e3, e2, e1, List.length_append, List.length_cons, List.length_nil]; exact hlt' _ (by omega))
  rw [h1, bind_tc_ok, h2, bind_tc_ok, h3, bind_tc_ok, h4, bind_tc_ok, h5, bind_tc_ok,
      h6, bind_tc_ok]
  exact ⟨v6, rfl, by simp [assertedLine, e6, e5, e4, e3, e2, e1]⟩

/-- The same for the three-fields writer. -/
theorem push_triple_fields_bytes_ok (out : alloc.vec.Vec U8) (s p o : Slice U8)
    (hs : termFits s.val = true) (hp : termFits p.val = true) (ho : termFits o.val = true)
    (hcap : out.val.length + s.val.length + p.val.length + o.val.length + 3 < Usize.max) :
    ∃ out', boundary_core.push_triple_fields_bytes out s p o
              = ok (core.result.Result.Ok (), out')
            ∧ out'.val = out.val ++ tripleFields s.val p.val o.val := by
  unfold boundary_core.push_triple_fields_bytes
  rw [term_fits_the_format_eq, bind_tc_ok, if_pos hs,
      term_fits_the_format_eq, bind_tc_ok, if_pos hp,
      term_fits_the_format_eq, bind_tc_ok, if_pos ho]
  -- `omega` cannot see through `Usize.max`, so it is never shown it: every
  -- capacity side condition below goes through these two, and the arithmetic
  -- `omega` does is over list lengths alone.
  have hlt' : ∀ n : Nat,
      n ≤ out.val.length + s.val.length + p.val.length + o.val.length + 3 → n < Usize.max :=
    fun n hn => Nat.lt_of_le_of_lt hn hcap
  have hle' : ∀ n : Nat,
      n ≤ out.val.length + s.val.length + p.val.length + o.val.length + 3 → n ≤ Usize.max :=
    fun n hn => Nat.le_of_lt (Nat.lt_of_le_of_lt hn hcap)
  obtain ⟨v1, h1, e1⟩ := push_u8 out 9#u8 (hlt' _ (by omega))
  obtain ⟨v2, h2, e2⟩ := extend_u8 v1 s (by simp only [e1, List.length_append, List.length_cons, List.length_nil]; exact hle' _ (by omega))
  obtain ⟨v3, h3, e3⟩ := push_u8 v2 9#u8 (by simp only [e2, e1, List.length_append, List.length_cons, List.length_nil]; exact hlt' _ (by omega))
  obtain ⟨v4, h4, e4⟩ := extend_u8 v3 p (by simp only [e3, e2, e1, List.length_append, List.length_cons, List.length_nil]; exact hle' _ (by omega))
  obtain ⟨v5, h5, e5⟩ := push_u8 v4 9#u8 (by simp only [e4, e3, e2, e1, List.length_append, List.length_cons, List.length_nil]; exact hlt' _ (by omega))
  obtain ⟨v6, h6, e6⟩ := extend_u8 v5 o (by simp only [e5, e4, e3, e2, e1, List.length_append, List.length_cons, List.length_nil]; exact hle' _ (by omega))
  rw [h1, bind_tc_ok, h2, bind_tc_ok, h3, bind_tc_ok, h4, bind_tc_ok, h5, bind_tc_ok,
      h6, bind_tc_ok]
  exact ⟨v6, rfl, by simp [tripleFields, e6, e5, e4, e3, e2, e1]⟩

/-- All or nothing. A term that does not fit makes the writer refuse, and the
buffer it hands back is the buffer it was given, byte for byte. No prefix of the
line is appended before the refusal, so a refused line cannot leave a fragment
for the next line to run into.

There is no capacity hypothesis here, unlike the two above, because refusing
allocates nothing. -/
theorem push_asserted_line_bytes_refuses (out : alloc.vec.Vec U8) (s p o : Slice U8)
    (h : termFits s.val = false ∨ termFits p.val = false ∨ termFits o.val = false) :
    ∃ pos, boundary_core.push_asserted_line_bytes out s p o
            = ok (core.result.Result.Err pos, out) := by
  unfold boundary_core.push_asserted_line_bytes
  rw [term_fits_the_format_eq, bind_tc_ok]
  by_cases hs : termFits s.val = true
  · rw [if_pos hs, term_fits_the_format_eq, bind_tc_ok]
    by_cases hp : termFits p.val = true
    · rw [if_pos hp, term_fits_the_format_eq, bind_tc_ok]
      have ho : termFits o.val = false := by
        rcases h with h | h | h
        · rw [h] at hs; exact absurd hs (by simp)
        · rw [h] at hp; exact absurd hp (by simp)
        · exact h
      rw [if_neg (by simp [ho])]
      exact ⟨_, rfl⟩
    · simp only [Bool.not_eq_true] at hp
      rw [if_neg (by simp [hp])]
      exact ⟨_, rfl⟩
  · simp only [Bool.not_eq_true] at hs
    rw [if_neg (by simp [hs])]
    exact ⟨_, rfl⟩

/-- The same for the three-fields writer. -/
theorem push_triple_fields_bytes_refuses (out : alloc.vec.Vec U8) (s p o : Slice U8)
    (h : termFits s.val = false ∨ termFits p.val = false ∨ termFits o.val = false) :
    ∃ pos, boundary_core.push_triple_fields_bytes out s p o
            = ok (core.result.Result.Err pos, out) := by
  unfold boundary_core.push_triple_fields_bytes
  rw [term_fits_the_format_eq, bind_tc_ok]
  by_cases hs : termFits s.val = true
  · rw [if_pos hs, term_fits_the_format_eq, bind_tc_ok]
    by_cases hp : termFits p.val = true
    · rw [if_pos hp, term_fits_the_format_eq, bind_tc_ok]
      have ho : termFits o.val = false := by
        rcases h with h | h | h
        · rw [h] at hs; exact absurd hs (by simp)
        · rw [h] at hp; exact absurd hp (by simp)
        · exact h
      rw [if_neg (by simp [ho])]
      exact ⟨_, rfl⟩
    · simp only [Bool.not_eq_true] at hp
      rw [if_neg (by simp [hp])]
      exact ⟨_, rfl⟩
  · simp only [Bool.not_eq_true] at hs
    rw [if_neg (by simp [hs])]
    exact ⟨_, rfl⟩

/-! ## What the reader gets back

The writer's job is done only if a reader splitting the file on its separators
recovers the terms that went in. These are the theorems about that, and they are
the reason `Spec.splitOnByte` exists. -/

/-- A term the guard accepts carries no separator. -/
theorem no_separator_of_termFits {l : List U8} (h : termFits l = true) :
    9#u8 ∉ l ∧ 10#u8 ∉ l ∧ 13#u8 ∉ l := by
  unfold termFits fieldFits at h
  simp only [Bool.and_eq_true, List.all_eq_true] at h
  obtain ⟨⟨_, hall⟩, _⟩ := h
  refine ⟨?_, ?_, ?_⟩ <;> intro hm <;>
    simpa [isTsvSeparator] using hall _ hm

/-- TCB-1, unbounded: an accepted line is exactly one record, and that record
splits on TAB into exactly the three terms that went in. Every length, and no
assumption that the terms came from a generator. -/
theorem asserted_line_reads_back (s p o : List U8)
    (hs : termFits s = true) (hp : termFits p = true) (ho : termFits o = true) :
    splitOnByte 10#u8 (assertedLine s p o) = [s ++ 9#u8 :: (p ++ 9#u8 :: o), []]
    ∧ splitOnByte 9#u8 (s ++ 9#u8 :: (p ++ 9#u8 :: o)) = [s, p, o] := by
  obtain ⟨hs9, hs10, _⟩ := no_separator_of_termFits hs
  obtain ⟨hp9, hp10, _⟩ := no_separator_of_termFits hp
  obtain ⟨ho9, ho10, _⟩ := no_separator_of_termFits ho
  constructor
  · have hshape : assertedLine s p o = (s ++ 9#u8 :: (p ++ 9#u8 :: o)) ++ 10#u8 :: [] := by
      simp [assertedLine]
    have hno : 10#u8 ∉ s ++ 9#u8 :: (p ++ 9#u8 :: o) := by
      intro hm
      rcases List.mem_append.mp hm with hx | hx
      · exact hs10 hx
      · rcases List.mem_cons.mp hx with hy | hy
        · exact absurd hy (by decide)
        · rcases List.mem_append.mp hy with hz | hz
          · exact hp10 hz
          · rcases List.mem_cons.mp hz with hw | hw
            · exact absurd hw (by decide)
            · exact ho10 hw
    rw [hshape, splitOnByte_append_cons, splitOnByte_of_not_mem _ _ hno]
    simp [splitOnByte]
  · rw [splitOnByte_append_cons, splitOnByte_of_not_mem _ _ hs9,
        splitOnByte_append_cons, splitOnByte_of_not_mem _ _ hp9,
        splitOnByte_of_not_mem _ _ ho9]
    simp

/-- TCB-2 and TCB-3, unbounded and stated the way they are used: a line already
carrying `n` tab-separated fields carries exactly `n + 3` after the writer
appends a triple, WHATEVER the prefix is. The prefix is unconstrained, so this
covers a rule name, a rule index, a binding list of any length, and the
conclusion and premises of a step already written.

The Kani version proves the same shape with the prefix fixed at one byte and each
term at four bytes or fewer. -/
theorem triple_fields_adds_exactly_three_fields (pre s p o : List U8)
    (hs : termFits s = true) (hp : termFits p = true) (ho : termFits o = true) :
    (splitOnByte 9#u8 (pre ++ tripleFields s p o)).length
      = (splitOnByte 9#u8 pre).length + 3 := by
  obtain ⟨hs9, _, _⟩ := no_separator_of_termFits hs
  obtain ⟨hp9, _, _⟩ := no_separator_of_termFits hp
  obtain ⟨ho9, _, _⟩ := no_separator_of_termFits ho
  have hshape : pre ++ tripleFields s p o
      = pre ++ 9#u8 :: (s ++ 9#u8 :: (p ++ 9#u8 :: o)) := by
    simp [tripleFields]
  rw [hshape, splitOnByte_length_append_cons, splitOnByte_length_append_cons,
      splitOnByte_of_not_mem _ _ hs9, splitOnByte_length_append_cons,
      splitOnByte_of_not_mem _ _ hp9, splitOnByte_of_not_mem _ _ ho9]
  simp

/-- And which fields, not only how many: the three appended fields are the three
terms, in order, after whatever the prefix contributed. -/
theorem triple_fields_are_the_three_terms (pre s p o : List U8)
    (hs : termFits s = true) (hp : termFits p = true) (ho : termFits o = true) :
    splitOnByte 9#u8 (pre ++ tripleFields s p o)
      = splitOnByte 9#u8 pre ++ [s, p, o] := by
  obtain ⟨hs9, _, _⟩ := no_separator_of_termFits hs
  obtain ⟨hp9, _, _⟩ := no_separator_of_termFits hp
  obtain ⟨ho9, _, _⟩ := no_separator_of_termFits ho
  have hshape : pre ++ tripleFields s p o
      = pre ++ 9#u8 :: (s ++ 9#u8 :: (p ++ 9#u8 :: o)) := by
    simp [tripleFields]
  rw [hshape, splitOnByte_append_cons, splitOnByte_append_cons,
      splitOnByte_of_not_mem _ _ hs9, splitOnByte_append_cons,
      splitOnByte_of_not_mem _ _ hp9, splitOnByte_of_not_mem _ _ ho9]
  simp

end OOBoundary
