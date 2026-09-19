import Dl.General

/-!
# A closed tableau is a refutation, and it needs no blocking argument

`DlMain.lean` records that certifying an INCONSISTENCY "needs a closed tableau
with its blocking argument". The first half is right and the second is not, and
the distinction is what makes this file affordable.

Blocking exists so that a reasoner SEARCHING for a model terminates: it is what
makes "no clash was reachable" a decidable answer, and it is the hard part of
any completeness proof. A refutation claims nothing of the sort. A closed
tableau is a finite object in which every branch already ended in a clash, so
the derivation to each clash is finite and blocking never enters it. The checker
replays a finite tree.

What is needed instead is that every rule PRESERVES SATISFIABILITY downward:
if the branch before the rule had a model, some branch after it does too. Then a
tree whose every leaf is unsatisfiable has an unsatisfiable root, which is the
theorem at the bottom of this file.

## What a branch is

A branch is a list of constraints about named individuals, exactly the shape a
tableau reasoner carries: `x : C`, `(x, y) : R`, and `x ≠ y` for the
inequalities number restrictions introduce. Satisfaction is relative to an
interpretation AND an assignment from those names to carrier elements, because
the names in a tableau are syntax the reasoner invented, not individuals the
ontology mentions.
-/
namespace Dl

/-- One entry on a tableau branch. -/
inductive Constraint where
  | conc (x : Name) (c : Concept)
  | role (r : Name) (x y : Name)
  | diff (x y : Name)
deriving DecidableEq, Repr, Inhabited

/-- The names a constraint mentions. Freshness for the existential rule is
stated against this, and nothing else. -/
def Constraint.names : Constraint → List Name
  | .conc x _ => [x]
  | .role _ x y => [x, y]
  | .diff x y => [x, y]

/-- The names a branch mentions. -/
def branchNames (Γ : List Constraint) : List Name :=
  Γ.flatMap Constraint.names

/-- An assignment sends the tableau's invented names to carrier elements. -/
abbrev Assign (α : Type) := Name → α

/-- The constraint holds under `I` and `σ`. -/
def CHolds {α : Type} (I : GInterp α) (σ : Assign α) : Constraint → Prop
  | .conc x c => I.dom (σ x) ∧ GSat I c (σ x)
  | .role r x y => I.rext r (σ x) (σ y)
  | .diff x y => σ x ≠ σ y

/-- The branch has a model: an interpretation of the axioms, and an assignment
under which every constraint on the branch holds.

The assignment must AGREE WITH `I.ind` on every individual the axioms name.
Without that clause the axioms and the branch talk past each other: `.inst a C`
is a claim about `I.ind a` while `CHolds` reads `σ a`, so no rule could ever
move an assertion out of `A` and onto `Γ`. Since `Γ` starts empty, a calculus
without such a rule refutes nothing at all, and the capstone theorem below is
true of no derivation. The clause costs the rules that invent witnesses one
extra side condition each: a fresh name must also be new to the axioms.

It costs the capstone theorem nothing, because `I.ind` is itself an assignment,
so the empty branch is satisfied by taking `σ := I.ind`. -/
def BranchSat (A : List Axiom) (Γ : List Constraint) : Prop :=
  ∃ (α : Type) (I : GInterp α) (σ : Assign α),
    GModels I A ∧ (∀ a ∈ indNames A, σ a = I.ind a) ∧ ∀ κ ∈ Γ, CHolds I σ κ

/-! ## The rules, each as a lemma about satisfiability

Every rule is stated in the direction a refutation needs: if the branch BEFORE
the rule is satisfiable, then the branch AFTER it is too, or in the branching
case one of the two is. Read contrapositively, which is how the tableau uses
them, an unsatisfiable successor forces an unsatisfiable predecessor.
-/

/-- Conjunction. `x : C ⊓ D` licenses `x : C` and `x : D`. -/
theorem and_sound {A : List Axiom} {Γ : List Constraint} {x : Name} {c d : Concept}
    (hmem : Constraint.conc x (.and c d) ∈ Γ) (h : BranchSat A Γ) :
    BranchSat A (Constraint.conc x c :: Constraint.conc x d :: Γ) := by
  obtain ⟨α, I, σ, hM, hag, hΓ⟩ := h
  refine ⟨α, I, σ, hM, hag, ?_⟩
  intro κ hκ
  have hx := hΓ _ hmem
  simp only [CHolds, GSat] at hx
  rcases hκ with _ | ⟨_, hκ⟩
  · exact ⟨hx.1, hx.2.1⟩
  · rcases hκ with _ | ⟨_, hκ⟩
    · exact ⟨hx.1, hx.2.2⟩
    · exact hΓ _ hκ

/-- Universal restriction. `x : ∀R.C` with an `R`-edge to `y` licenses `y : C`.
The domain side comes from the range of the edge being in the carrier, which is
what `GModels` gives for every role the axioms mention. -/
theorem all_sound {A : List Axiom} {Γ : List Constraint} {x y r : Name} {c : Concept}
    (hall : Constraint.conc x (.all r c) ∈ Γ)
    (hedge : Constraint.role r x y ∈ Γ)
    (hr : r ∈ roleNames A)
    (h : BranchSat A Γ) :
    BranchSat A (Constraint.conc y c :: Γ) := by
  obtain ⟨α, I, σ, hM, hag, hΓ⟩ := h
  refine ⟨α, I, σ, hM, hag, ?_⟩
  intro κ hκ
  rcases hκ with _ | ⟨_, hκ⟩
  · have hx := hΓ _ hall
    have he := hΓ _ hedge
    simp only [CHolds, GSat] at hx he
    -- The target is in the carrier because the edge is, which is what `hr` and
    -- `rextDom` say together. Asking the branch for a concept entry on `y`
    -- instead, as this used to, refuses to propagate into an individual the
    -- ontology relates but never classifies, and those are common.
    exact ⟨hM.rextDom r hr (σ x) (σ y) hx.1 he, hx.2 (σ y) he⟩
  · exact hΓ _ hκ

/-- Disjunction. `x : C ⊔ D` licenses a split, and a model of the branch is a
model of one side or the other. This is the only rule that branches, and the
theorem at the bottom is what turns "both children closed" into a refutation. -/
theorem or_sound {A : List Axiom} {Γ : List Constraint} {x : Name} {c d : Concept}
    (hmem : Constraint.conc x (.or c d) ∈ Γ) (h : BranchSat A Γ) :
    BranchSat A (Constraint.conc x c :: Γ) ∨ BranchSat A (Constraint.conc x d :: Γ) := by
  obtain ⟨α, I, σ, hM, hag, hΓ⟩ := h
  have hx := hΓ _ hmem
  simp only [CHolds, GSat] at hx
  rcases hx.2 with hc | hd
  · refine Or.inl ⟨α, I, σ, hM, hag, ?_⟩
    intro κ hκ
    rcases hκ with _ | ⟨_, hκ⟩
    · exact ⟨hx.1, hc⟩
    · exact hΓ _ hκ
  · refine Or.inr ⟨α, I, σ, hM, hag, ?_⟩
    intro κ hκ
    rcases hκ with _ | ⟨_, hκ⟩
    · exact ⟨hx.1, hd⟩
    · exact hΓ _ hκ

/-! ## Freshness, and the rule that invents an individual

The existential rule is the only one that adds a name the branch did not have.
Its soundness is therefore not just "a model of the premise satisfies the
conclusion": the model has a witness, but the ASSIGNMENT has to be extended to
send the new name to it, and that extension must not disturb the constraints
already on the branch. That is the substitution lemma below, and freshness is
exactly the hypothesis that makes it hold. -/

/-- Two assignments agreeing on the names a constraint mentions satisfy it
alike. -/
theorem cholds_congr {α : Type} {I : GInterp α} {σ τ : Assign α} {κ : Constraint}
    (h : ∀ n ∈ κ.names, σ n = τ n) : CHolds I σ κ ↔ CHolds I τ κ := by
  cases κ with
  | conc x c => simp [CHolds, h x (by simp [Constraint.names])]
  | role r x y =>
      simp [CHolds, h x (by simp [Constraint.names]), h y (by simp [Constraint.names])]
  | diff x y =>
      simp [CHolds, h x (by simp [Constraint.names]), h y (by simp [Constraint.names])]

/-- Updating an assignment at a name the branch never mentions changes nothing
on that branch. -/
theorem branch_update_fresh {α : Type} {I : GInterp α} {σ : Assign α}
    {Γ : List Constraint} {y : Name} {w : α}
    (hfresh : y ∉ branchNames Γ) (hΓ : ∀ κ ∈ Γ, CHolds I σ κ) :
    ∀ κ ∈ Γ, CHolds I (fun n => if n = y then w else σ n) κ := by
  intro κ hκ
  refine (cholds_congr ?_).mp (hΓ κ hκ)
  intro n hn
  have : n ≠ y := by
    rintro rfl
    exact hfresh (List.mem_flatMap.mpr ⟨κ, hκ, hn⟩)
  simp [this]

/-- **The existential rule.** `x : ∃R.C` licenses a fresh `y` with an `R`-edge
from `x` and `y : C`.

`y` must not already occur on the branch. That is the whole of the side
condition, and without it the rule is unsound: reusing a name would assert that
an individual the branch already constrains is the witness, which no model
need agree to.

`r` must be a role the axioms mention, which is what puts the witness in the
carrier. Every role a tableau for `A` ever works with comes from a concept in
`A`, so this costs a producer nothing. -/
theorem ex_sound {A : List Axiom} {Γ : List Constraint} {x y r : Name} {c : Concept}
    (hmem : Constraint.conc x (.ex r c) ∈ Γ)
    (hr : r ∈ roleNames A)
    (hfresh : y ∉ branchNames Γ)
    (hfreshA : y ∉ indNames A)
    (h : BranchSat A Γ) :
    BranchSat A (Constraint.role r x y :: Constraint.conc y c :: Γ) := by
  obtain ⟨α, I, σ, hM, hag, hΓ⟩ := h
  have hx := hΓ _ hmem
  simp only [CHolds, GSat] at hx
  obtain ⟨w, hedge, hsat⟩ := hx.2
  -- `x` is on the branch and `y` is not, so they are different names and the
  -- update leaves `σ x` alone.
  have hxy : x ≠ y := by
    rintro rfl
    exact hfresh (List.mem_flatMap.mpr ⟨_, hmem, by simp [Constraint.names]⟩)
  refine ⟨α, I, fun n => if n = y then w else σ n, hM, ?_, ?_⟩
  · intro a ha
    have : a ≠ y := by rintro rfl; exact hfreshA ha
    simpa [this] using hag a ha
  intro κ hκ
  rcases hκ with _ | ⟨_, hκ⟩
  · simpa [CHolds, hxy, if_neg hxy] using hedge
  · rcases hκ with _ | ⟨_, hκ⟩
    · refine ⟨?_, ?_⟩
      · simpa using hM.rextDom r hr (σ x) w hx.1 hedge
      · simpa using hsat
    · exact branch_update_fresh hfresh hΓ _ hκ

/-! ## The ≥ rule

`x : ≥n R.C` invents `n` witnesses in one step, which is the existential rule's
freshness argument generalised from a name to a list. Two things get harder.
The witnesses must be pairwise DISTINCT, or the rule would be vacuous whenever
`n > 1`, so the rule emits the inequalities as well as the edges. And the
assignment is extended at `n` names simultaneously rather than one, so
`branch_update_fresh` has to be redone for a list of updates.
-/

/-- Every inequality between distinct members of a list, both ways round.

Both ways round on purpose: `clash_max` asks for `diff a b` for an unordered
pair, and a producer should not have to guess which orientation this file
happened to emit. -/
def pairDiffs : List Name → List Constraint
  | [] => []
  | y :: ys => ys.flatMap (fun z => [Constraint.diff y z, Constraint.diff z y]) ++ pairDiffs ys

/-- Only genuine inequalities come out. Needs `Nodup`, because the list is what
supplies `a ≠ b`. -/
theorem mem_pairDiffs {ys : List Name} (hnd : ys.Nodup) :
    ∀ κ ∈ pairDiffs ys, ∃ a ∈ ys, ∃ b ∈ ys, a ≠ b ∧ κ = Constraint.diff a b := by
  induction ys with
  | nil => intro κ hκ; simp [pairDiffs] at hκ
  | cons y ys ih =>
      obtain ⟨hy, hnd'⟩ := List.nodup_cons.mp hnd
      intro κ hκ
      rcases List.mem_append.mp hκ with hκ | hκ
      · obtain ⟨z, hz, hz'⟩ := List.mem_flatMap.mp hκ
        have hne : y ≠ z := by rintro rfl; exact hy hz
        simp only [List.mem_cons, List.not_mem_nil, or_false] at hz'
        rcases hz' with rfl | rfl
        · exact ⟨y, by simp, z, by simp [hz], hne, rfl⟩
        · exact ⟨z, by simp [hz], y, by simp, fun hc => hne hc.symm, rfl⟩
      · obtain ⟨a, ha, b, hb, hne, rfl⟩ := ih hnd' κ hκ
        exact ⟨a, by simp [ha], b, by simp [hb], hne, rfl⟩

/-- And every genuine inequality is there. This is the direction a producer
needs: having run the ≥ rule, the branch really does carry the `diff`
constraints `clash_max` will ask for. -/
theorem mem_pairDiffs_of :
    ∀ {ys : List Name}, ∀ a ∈ ys, ∀ b ∈ ys, a ≠ b → Constraint.diff a b ∈ pairDiffs ys := by
  intro ys
  induction ys with
  | nil => intro a ha; cases ha
  | cons y ys ih =>
      intro a ha b hb hab
      by_cases hay : a = y
      · subst hay
        have hb' : b ∈ ys := (List.mem_cons.mp hb).resolve_left (fun hc => hab hc.symm)
        exact List.mem_append_left _ (List.mem_flatMap.mpr ⟨b, hb', by simp⟩)
      · have ha' : a ∈ ys := (List.mem_cons.mp ha).resolve_left hay
        by_cases hby : b = y
        · subst hby
          exact List.mem_append_left _ (List.mem_flatMap.mpr ⟨a, ha', by simp⟩)
        · have hb' : b ∈ ys := (List.mem_cons.mp hb).resolve_left hby
          exact List.mem_append_right _ (ih a ha' b hb' hab)

/-- What the ≥ rule adds to the branch: an `R`-edge to each witness, the
concept at each witness, and every inequality between them. -/
def minExpand (r x : Name) (c : Concept) (ys : List Name) : List Constraint :=
  ys.map (fun y => Constraint.role r x y)
    ++ ys.map (fun y => Constraint.conc y c)
    ++ pairDiffs ys

/-- Extend an assignment at a list of names at once, pairing them off with a
list of carrier elements. Mismatched lengths fall through to the original
assignment; every use here supplies equal lengths. -/
def assignList {α : Type} (σ : Assign α) : List Name → List α → Assign α
  | y :: ys, w :: ws => fun n => if n = y then w else assignList σ ys ws n
  | _, _ => σ

/-- The list version of `branch_update_fresh`: names outside the list are
untouched, so everything already on the branch survives. -/
theorem assignList_not_mem {α : Type} (σ : Assign α) :
    ∀ (ys : List Name) (ws : List α) (n : Name), n ∉ ys → assignList σ ys ws n = σ n := by
  intro ys
  induction ys with
  | nil => intro ws n _; cases ws <;> rfl
  | cons y ys ih =>
      intro ws n hn
      cases ws with
      | nil => rfl
      | cons w ws =>
          have hne : n ≠ y := by rintro rfl; exact hn (by simp)
          simp only [assignList, if_neg hne]
          exact ih ws n (fun h => hn (by simp [h]))

/-- Each name in the list lands on an element of the list it was paired with.
This is what carries the ≥ property from the carrier elements to the witnesses. -/
theorem assignList_mem {α : Type} (σ : Assign α) :
    ∀ (ys : List Name) (ws : List α), ys.Nodup → ys.length = ws.length →
      ∀ y ∈ ys, assignList σ ys ws y ∈ ws := by
  intro ys
  induction ys with
  | nil => intro _ _ _ a ha; cases ha
  | cons y ys ih =>
      intro ws hnd hlen a ha
      cases ws with
      | nil => simp at hlen
      | cons w ws =>
          obtain ⟨hy, hnd'⟩ := List.nodup_cons.mp hnd
          simp only [List.length_cons, Nat.add_right_cancel_iff] at hlen
          by_cases hay : a = y
          · subst hay; simp [assignList]
          · have ha' : a ∈ ys := (List.mem_cons.mp ha).resolve_left hay
            simp only [assignList, if_neg hay]
            exact List.mem_cons_of_mem _ (ih ws hnd' hlen a ha')

/-- Distinct names land on distinct elements. This is what makes the emitted
inequalities true, and it is why the rule needs the witnesses to be `Nodup` on
both sides: distinct names paired with a repeated element would not satisfy the
`diff` constraints the rule itself adds. -/
theorem assignList_inj {α : Type} (σ : Assign α) :
    ∀ (ys : List Name) (ws : List α), ys.Nodup → ws.Nodup → ys.length = ws.length →
      ∀ a ∈ ys, ∀ b ∈ ys, a ≠ b → assignList σ ys ws a ≠ assignList σ ys ws b := by
  intro ys
  induction ys with
  | nil => intro _ _ _ _ a ha; cases ha
  | cons y ys ih =>
      intro ws hnd hwnd hlen a ha b hb hab
      cases ws with
      | nil => simp at hlen
      | cons w ws =>
          obtain ⟨hy, hnd'⟩ := List.nodup_cons.mp hnd
          obtain ⟨hw, hwnd'⟩ := List.nodup_cons.mp hwnd
          simp only [List.length_cons, Nat.add_right_cancel_iff] at hlen
          by_cases hay : a = y
          · subst hay
            have hb' : b ∈ ys := (List.mem_cons.mp hb).resolve_left (fun hc => hab hc.symm)
            have hbne : b ≠ a := fun hc => hab hc.symm
            simp only [assignList, if_neg hbne, if_true]
            intro hcon
            exact hw (by rw [hcon]; exact assignList_mem σ ys ws hnd' hlen b hb')
          · have ha' : a ∈ ys := (List.mem_cons.mp ha).resolve_left hay
            by_cases hby : b = y
            · subst hby
              simp only [assignList, if_neg hay, if_true]
              intro hcon
              exact hw (by rw [← hcon]; exact assignList_mem σ ys ws hnd' hlen a ha')
            · have hb' : b ∈ ys := (List.mem_cons.mp hb).resolve_left hby
              simp only [assignList, if_neg hay, if_neg hby]
              exact ih ws hnd' hwnd' hlen a ha' b hb' hab

/-- **The ≥ rule.** `x : ≥n R.C` licenses `n` pairwise distinct fresh witnesses,
each an `R`-successor of `x` satisfying `C`.

The side condition is the existential rule's, once per name, plus one more:
the names must be distinct from EACH OTHER as well as absent from the branch.
Dropping that would let a producer emit `n` copies of one name and then read
off the inequalities as a clash, which is the whole rule's soundness gone.

`hr` plays the same part it does in `ex_sound`: it is what puts the invented
witnesses in the carrier, via `GModels.rextDom`. -/
theorem min_sound {A : List Axiom} {Γ : List Constraint} {x r : Name} {c : Concept}
    {n : Nat} {ys : List Name}
    (hmem : Constraint.conc x (.min n r c) ∈ Γ)
    (hr : r ∈ roleNames A)
    (hlen : ys.length = n)
    (hnd : ys.Nodup)
    (hfresh : ∀ y ∈ ys, y ∉ branchNames Γ)
    (hfreshA : ∀ y ∈ ys, y ∉ indNames A)
    (h : BranchSat A Γ) :
    BranchSat A (minExpand r x c ys ++ Γ) := by
  obtain ⟨α, I, σ, hM, hag, hΓ⟩ := h
  have hx := hΓ _ hmem
  simp only [CHolds, GSat, GAtLeast] at hx
  obtain ⟨ws, hwnd, hwlen, hwP⟩ := hx.2
  have hlen' : ys.length = ws.length := by omega
  -- `x` is on the branch and every witness is not, so the update leaves `σ x`
  -- alone. Exactly the `x ≠ y` step of `ex_sound`, once for the whole list.
  have hxys : x ∉ ys := fun hc =>
    hfresh x hc (List.mem_flatMap.mpr ⟨_, hmem, by simp [Constraint.names]⟩)
  have hτx : assignList σ ys ws x = σ x := assignList_not_mem σ ys ws x hxys
  refine ⟨α, I, assignList σ ys ws, hM, ?_, ?_⟩
  · intro a ha
    exact (assignList_not_mem σ ys ws a (fun hc => hfreshA a hc ha)).trans (hag a ha)
  intro κ hκ
  rcases List.mem_append.mp hκ with hκ | hκ
  · rcases List.mem_append.mp hκ with hκ | hκ
    · rcases List.mem_append.mp hκ with hκ | hκ
      · obtain ⟨y, hy, rfl⟩ := List.mem_map.mp hκ
        have hw := hwP _ (assignList_mem σ ys ws hnd hlen' y hy)
        simpa [CHolds, hτx] using hw.1
      · obtain ⟨y, hy, rfl⟩ := List.mem_map.mp hκ
        have hw := hwP _ (assignList_mem σ ys ws hnd hlen' y hy)
        exact ⟨hM.rextDom r hr (σ x) _ hx.1 hw.1, hw.2⟩
    · obtain ⟨a, ha, b, hb, hne, rfl⟩ := mem_pairDiffs hnd κ hκ
      exact assignList_inj σ ys ws hnd hwnd hlen' a ha b hb hne
  · refine (cholds_congr ?_).mp (hΓ κ hκ)
    intro m hm
    have hmys : m ∉ ys := fun hc => hfresh m hc (List.mem_flatMap.mpr ⟨κ, hκ, hm⟩)
    exact (assignList_not_mem σ ys ws m hmys).symm

/-! ## The rules that read the axioms

Nothing above this point can move a fact out of `A` and onto `Γ`, and `Γ`
starts empty, so on its own the calculus above refutes nothing whatever: the
capstone theorem would be true of no derivation at all. These are the rules
that start a derivation and then keep it fed.

They are also where the agreement clause in `BranchSat` earns its place. An
axiom `a : C` is a claim about `I.ind a`; a branch entry `a : C` is a claim
about `σ a`. The clause is what makes those the same claim.
-/

/-- **The assertion rule.** `a : C` among the axioms is `a : C` on the branch.

`hind` is doing real work and is not bureaucracy. `GHolds (.inst a c)` says only
that `I.ind a` satisfies `c`, and in a general interpretation the carrier is a
PREDICATE rather than the type, so being named is not the same as being in the
domain. `CHolds` demands the domain, so the producer must point at the axiom
that supplies it. -/
theorem inst_sound {A : List Axiom} {Γ : List Constraint} {a : Name} {c : Concept}
    (hmem : Axiom.inst a c ∈ A) (hind : Axiom.indiv a ∈ A)
    (h : BranchSat A Γ) : BranchSat A (Constraint.conc a c :: Γ) := by
  obtain ⟨α, I, σ, hM, hag, hΓ⟩ := h
  have hσa : σ a = I.ind a := hag a (List.mem_flatMap.mpr ⟨_, hmem, by simp [Axiom.inds]⟩)
  refine ⟨α, I, σ, hM, hag, ?_⟩
  intro κ hκ
  rcases hκ with _ | ⟨_, hκ⟩
  · show I.dom (σ a) ∧ GSat I c (σ a)
    rw [hσa]
    exact ⟨hM.holds _ hind, hM.holds _ hmem⟩
  · exact hΓ _ hκ

/-- **The role assertion rule.** `(a, b) : R` among the axioms is an edge on the
branch. -/
theorem rel_sound {A : List Axiom} {Γ : List Constraint} {a r b : Name}
    (hmem : Axiom.rel a r b ∈ A) (h : BranchSat A Γ) :
    BranchSat A (Constraint.role r a b :: Γ) := by
  obtain ⟨α, I, σ, hM, hag, hΓ⟩ := h
  have hσa : σ a = I.ind a := hag a (List.mem_flatMap.mpr ⟨_, hmem, by simp [Axiom.inds]⟩)
  have hσb : σ b = I.ind b := hag b (List.mem_flatMap.mpr ⟨_, hmem, by simp [Axiom.inds]⟩)
  refine ⟨α, I, σ, hM, hag, ?_⟩
  intro κ hκ
  rcases hκ with _ | ⟨_, hκ⟩
  · show I.rext r (σ a) (σ b)
    rw [hσa, hσb]
    exact hM.holds _ hmem
  · exact hΓ _ hκ

/-- **The subsumption rule.** `C ⊑ D` and `x : C` give `x : D`.

This is the one that makes a TBox bite. It is also why the branch entry carries
`I.dom` alongside the concept: `GHolds (.sub c d)` is guarded by the domain, so
without it the rule could not fire. -/
theorem sub_sound {A : List Axiom} {Γ : List Constraint} {x : Name} {c d : Concept}
    (hsub : Axiom.sub c d ∈ A) (hx : Constraint.conc x c ∈ Γ) (h : BranchSat A Γ) :
    BranchSat A (Constraint.conc x d :: Γ) := by
  obtain ⟨α, I, σ, hM, hag, hΓ⟩ := h
  have hc := hΓ _ hx
  refine ⟨α, I, σ, hM, hag, ?_⟩
  intro κ hκ
  rcases hκ with _ | ⟨_, hκ⟩
  · exact ⟨hc.1, hM.holds _ hsub (σ x) hc.1 hc.2⟩
  · exact hΓ _ hκ

/-- **The domain rule.** An `R`-edge out of `x` puts `x` in `R`'s domain.

`hdom` is the branch's evidence that `x` is in the carrier, the same device
`all_sound` uses: an edge alone does not say its source is a domain element,
because `GInterp.rext` is unrestricted. -/
theorem dom_sound {A : List Axiom} {Γ : List Constraint} {x y r : Name} {c : Concept}
    (hax : Axiom.dom r c ∈ A) (hedge : Constraint.role r x y ∈ Γ)
    (hdom : ∃ e, Constraint.conc x e ∈ Γ) (h : BranchSat A Γ) :
    BranchSat A (Constraint.conc x c :: Γ) := by
  obtain ⟨α, I, σ, hM, hag, hΓ⟩ := h
  obtain ⟨e, he⟩ := hdom
  have hxd := (hΓ _ he).1
  have hE := hΓ _ hedge
  refine ⟨α, I, σ, hM, hag, ?_⟩
  intro κ hκ
  rcases hκ with _ | ⟨_, hκ⟩
  · exact ⟨hxd, hM.holds _ hax (σ x) hxd ⟨σ y, hE⟩⟩
  · exact hΓ _ hκ

/-- **The range rule.** An `R`-edge into `y` puts `y` in `R`'s range.

`hr` supplies `I.dom (σ y)` through `GModels.rextDom`, exactly as it does for
the witnesses the existential and ≥ rules invent. -/
theorem rng_sound {A : List Axiom} {Γ : List Constraint} {x y r : Name} {c : Concept}
    (hax : Axiom.rng r c ∈ A) (hr : r ∈ roleNames A)
    (hedge : Constraint.role r x y ∈ Γ)
    (hdom : ∃ e, Constraint.conc x e ∈ Γ) (h : BranchSat A Γ) :
    BranchSat A (Constraint.conc y c :: Γ) := by
  obtain ⟨α, I, σ, hM, hag, hΓ⟩ := h
  obtain ⟨e, he⟩ := hdom
  have hxd := (hΓ _ he).1
  have hE := hΓ _ hedge
  refine ⟨α, I, σ, hM, hag, ?_⟩
  intro κ hκ
  rcases hκ with _ | ⟨_, hκ⟩
  · exact ⟨hM.rextDom r hr (σ x) (σ y) hxd hE, hM.holds _ hax (σ x) hxd (σ y) hE⟩
  · exact hΓ _ hκ

/-- **The role hierarchy rule.** `R ⊑ S` turns an `R`-edge into an `S`-edge. -/
theorem subrole_sound {A : List Axiom} {Γ : List Constraint} {x y r t : Name}
    (hax : Axiom.subrole r t ∈ A) (hedge : Constraint.role r x y ∈ Γ)
    (hdom : ∃ e, Constraint.conc x e ∈ Γ) (h : BranchSat A Γ) :
    BranchSat A (Constraint.role t x y :: Γ) := by
  obtain ⟨α, I, σ, hM, hag, hΓ⟩ := h
  obtain ⟨e, he⟩ := hdom
  have hxd := (hΓ _ he).1
  have hE := hΓ _ hedge
  refine ⟨α, I, σ, hM, hag, ?_⟩
  intro κ hκ
  rcases hκ with _ | ⟨_, hκ⟩
  · exact hM.holds _ hax (σ x) hxd (σ y) hE
  · exact hΓ _ hκ

/-- **The witness rule.** A non-emptiness axiom names a fresh individual for the
concept it asserts.

This is the entry point for a satisfiability question. "Is `C` satisfiable under
`T`?" is asked by putting `.nonempty C` into the axioms, and refuting it is what
this rule makes reachable from the empty branch. -/
theorem nonempty_sound {A : List Axiom} {Γ : List Constraint} {y : Name} {c : Concept}
    (hax : Axiom.nonempty c ∈ A)
    (hfresh : y ∉ branchNames Γ) (hfreshA : y ∉ indNames A)
    (h : BranchSat A Γ) : BranchSat A (Constraint.conc y c :: Γ) := by
  obtain ⟨α, I, σ, hM, hag, hΓ⟩ := h
  obtain ⟨w, hwd, hwc⟩ := hM.holds _ hax
  refine ⟨α, I, fun n => if n = y then w else σ n, hM, ?_, ?_⟩
  · intro a ha
    have : a ≠ y := by rintro rfl; exact hfreshA ha
    simpa [this] using hag a ha
  · intro κ hκ
    rcases hκ with _ | ⟨_, hκ⟩
    · exact ⟨by simpa using hwd, by simpa using hwc⟩
    · exact branch_update_fresh hfresh hΓ _ hκ

/-! ## Clashes

A clash is a branch that no interpretation can satisfy, and each is a small
lemma rather than an entry in a list the checker trusts. -/

/-- `x : ⊥`. -/
theorem clash_bot {A : List Axiom} {Γ : List Constraint} {x : Name}
    (h : Constraint.conc x .bot ∈ Γ) : ¬ BranchSat A Γ := by
  rintro ⟨α, I, σ, _, _, hΓ⟩
  exact (hΓ _ h).2

/-- `x : C` and `x : ¬C`. -/
theorem clash_neg {A : List Axiom} {Γ : List Constraint} {x : Name} {c : Concept}
    (hp : Constraint.conc x c ∈ Γ) (hn : Constraint.conc x (.neg c) ∈ Γ) :
    ¬ BranchSat A Γ := by
  rintro ⟨α, I, σ, _, _, hΓ⟩
  have hpos := (hΓ _ hp).2
  have hneg := (hΓ _ hn).2
  simp only [GSat] at hneg
  exact hneg hpos

/-- `Nodup` survives a map that is injective ON THE LIST. Core Lean carries the
globally injective version; the branch only ever gives injectivity where it has
asserted `diff`, so this is the shape needed. -/
theorem nodup_map_of_injOn {α β : Type} {f : α → β} :
    ∀ {l : List α}, l.Nodup → (∀ a ∈ l, ∀ b ∈ l, f a = f b → a = b) → (l.map f).Nodup
  | [], _, _ => by simp
  | a :: t, hnd, hinj => by
      rw [List.nodup_cons] at hnd
      refine List.nodup_cons.mpr ⟨?_, ?_⟩
      · intro hmem
        obtain ⟨b, hb, hfb⟩ := List.mem_map.mp hmem
        have : a = b := hinj a (by simp) b (by simp [hb]) hfb.symm
        exact hnd.1 (this ▸ hb)
      · exact nodup_map_of_injOn hnd.2
          (fun p hp q hq h => hinj p (by simp [hp]) q (by simp [hq]) h)

/-- **The cardinality clash.** `x : ≤n R.C` with `n+1` pairwise distinct
`R`-successors all in `C`.

This is where the ≤ restriction earns its place in a REFUTATION, and it is a
clash rather than a rule. The merging rule that identifies two successors is a
COMPLETENESS device: a model search needs it to collapse a branch into a model.
A refutation never needs to build a model, so it never needs to merge, exactly
as it never needs blocking. What it needs is the observation that `n+1`
distinct witnesses contradict a bound of `n`, and that is this lemma.

The distinctness comes from `diff` constraints on the branch rather than from
the names differing, because two different names may denote the same element
unless the branch has said otherwise. -/
theorem clash_max {A : List Axiom} {Γ : List Constraint} {x r : Name} {c : Concept}
    {n : Nat} {ys : List Name}
    (hmax : Constraint.conc x (.max n r c) ∈ Γ)
    (hlen : ys.length = n + 1)
    (hnd : ys.Nodup)
    (hdiff : ∀ a ∈ ys, ∀ b ∈ ys, a ≠ b → Constraint.diff a b ∈ Γ)
    (hedge : ∀ y ∈ ys, Constraint.role r x y ∈ Γ)
    (hc : ∀ y ∈ ys, Constraint.conc y c ∈ Γ) :
    ¬ BranchSat A Γ := by
  rintro ⟨α, I, σ, _, _, hΓ⟩
  have hx := hΓ _ hmax
  simp only [CHolds, GSat, GAtMost] at hx
  -- The images are distinct, because the branch asserted the names differ.
  have hmapnd : (ys.map σ).Nodup := by
    refine nodup_map_of_injOn hnd ?_
    intro a ha b hb hab
    by_cases hne : a = b
    · exact hne
    · exact absurd hab (hΓ _ (hdiff a ha b hb hne))
  have hmem : ∀ z ∈ ys.map σ, I.rext r (σ x) z ∧ GSat I c z := by
    intro z hz
    obtain ⟨y, hy, rfl⟩ := List.mem_map.mp hz
    exact ⟨hΓ _ (hedge y hy), (hΓ _ (hc y hy)).2⟩
  have := hx.2 (ys.map σ) hmapnd hmem
  simp only [List.length_map, hlen] at this
  omega

/-- **The bound clash.** `x : ≥m R.C` against `x : ≤n R.C` with `n < m`.

Cheaper than `clash_max` and worth having separately: it needs no witnesses on
the branch at all, so a producer can close this without ever running the ≥
rule. The two bounds contradict each other directly, because the list the lower
bound promises is itself a list the upper bound must measure. -/
theorem clash_minmax {A : List Axiom} {Γ : List Constraint} {x r : Name} {c : Concept}
    {m n : Nat}
    (hmin : Constraint.conc x (.min m r c) ∈ Γ)
    (hmax : Constraint.conc x (.max n r c) ∈ Γ)
    (hlt : n < m) : ¬ BranchSat A Γ := by
  rintro ⟨α, I, σ, _, _, hΓ⟩
  have hge := (hΓ _ hmin).2
  have hle := (hΓ _ hmax).2
  simp only [GSat, GAtLeast] at hge
  simp only [GSat, GAtMost] at hle
  obtain ⟨ys, hnd, hlen, hmem⟩ := hge
  have := hle ys hnd hmem
  omega

/-- **The disjointness clash.** `C ⊓ D ⊑ ⊥` with an `x` in both.

Worth a rule of its own rather than a translation to `C ⊑ ¬D`, even though the
two say the same thing about every interpretation. The certificate has to cite
an axiom that is IN the axiom file, and the axiom file is a transcription of the
ontology. Rewriting disjointness on the way in would make the certificate be
about a different axiom set from the one the reader was handed. -/
theorem clash_disjoint {A : List Axiom} {Γ : List Constraint} {x : Name} {c d : Concept}
    (hax : Axiom.disjoint c d ∈ A)
    (hc : Constraint.conc x c ∈ Γ) (hd : Constraint.conc x d ∈ Γ) : ¬ BranchSat A Γ := by
  rintro ⟨α, I, σ, hM, _, hΓ⟩
  exact hM.holds _ hax (σ x) (hΓ _ hc).1 ⟨(hΓ _ hc).2, (hΓ _ hd).2⟩

/-- `x ≠ x`. The inequality rule for number restrictions introduces these, and
a branch that has asserted an individual differs from itself is closed. -/
theorem clash_diff {A : List Axiom} {Γ : List Constraint} {x : Name}
    (h : Constraint.diff x x ∈ Γ) : ¬ BranchSat A Γ := by
  rintro ⟨α, I, σ, _, _, hΓ⟩
  exact (hΓ _ h) rfl

/-! ## The certificate, and what a checked one buys

A closed tableau is an inductive object: each constructor is one rule
application or one clash, and the recursive arguments are the branches it
produced. That is the shape a producer emits and a checker replays, and it is
finite by construction, which is the whole reason no blocking argument appears
anywhere in this file. -/

/-- `Closed A Γ` is a closed tableau for the branch `Γ` under the axioms `A`.

Each constructor carries the side conditions its rule needs, so an ill-formed
certificate is not a `Closed` at all and there is nothing for the checker to
trust beyond the constructors themselves. -/
inductive Closed (A : List Axiom) : List Constraint → Prop where
  | bot {Γ x} (h : Constraint.conc x .bot ∈ Γ) : Closed A Γ
  | neg {Γ x c} (hp : Constraint.conc x c ∈ Γ)
        (hn : Constraint.conc x (.neg c) ∈ Γ) : Closed A Γ
  | diff {Γ x} (h : Constraint.diff x x ∈ Γ) : Closed A Γ
  /-- Disjointness: an individual in two classes the ontology declares apart. -/
  | disjointClash {Γ : List Constraint} {x : Name} {c d : Concept}
        (hax : Axiom.disjoint c d ∈ A)
        (hc : Constraint.conc x c ∈ Γ) (hd : Constraint.conc x d ∈ Γ) : Closed A Γ
  | andR {Γ x c d} (hmem : Constraint.conc x (.and c d) ∈ Γ)
        (next : Closed A (Constraint.conc x c :: Constraint.conc x d :: Γ)) : Closed A Γ
  | allR {Γ x y r c} (hall : Constraint.conc x (.all r c) ∈ Γ)
        (hedge : Constraint.role r x y ∈ Γ) (hr : r ∈ roleNames A)
        (next : Closed A (Constraint.conc y c :: Γ)) : Closed A Γ
  | orR {Γ x c d} (hmem : Constraint.conc x (.or c d) ∈ Γ)
        (left : Closed A (Constraint.conc x c :: Γ))
        (right : Closed A (Constraint.conc x d :: Γ)) : Closed A Γ
  /-- The existential rule. `hfresh` is what keeps it sound and `hr` is what
  puts the invented witness in the carrier. -/
  | exR {Γ : List Constraint} {x y r : Name} {c : Concept} (hmem : Constraint.conc x (.ex r c) ∈ Γ)
        (hr : r ∈ roleNames A)
        (hfresh : y ∉ branchNames Γ) (hfreshA : y ∉ indNames A)
        (next : Closed A (Constraint.role r x y :: Constraint.conc y c :: Γ)) : Closed A Γ
  /-- Assertion: an axiom `a : C` becomes a branch entry. `hind` is what puts
  `a` in the carrier. -/
  | instR {Γ : List Constraint} {a : Name} {c : Concept}
        (hmem : Axiom.inst a c ∈ A) (hind : Axiom.indiv a ∈ A)
        (next : Closed A (Constraint.conc a c :: Γ)) : Closed A Γ
  /-- Role assertion: an axiom `(a, b) : R` becomes an edge. -/
  | relR {Γ : List Constraint} {a r b : Name} (hmem : Axiom.rel a r b ∈ A)
        (next : Closed A (Constraint.role r a b :: Γ)) : Closed A Γ
  /-- Subsumption: the rule that makes a TBox bite. -/
  | subR {Γ : List Constraint} {x : Name} {c d : Concept}
        (hsub : Axiom.sub c d ∈ A) (hx : Constraint.conc x c ∈ Γ)
        (next : Closed A (Constraint.conc x d :: Γ)) : Closed A Γ
  /-- Domain. -/
  | domR {Γ : List Constraint} {x y r : Name} {c : Concept}
        (hax : Axiom.dom r c ∈ A) (hedge : Constraint.role r x y ∈ Γ)
        (hdom : ∃ e, Constraint.conc x e ∈ Γ)
        (next : Closed A (Constraint.conc x c :: Γ)) : Closed A Γ
  /-- Range. -/
  | rngR {Γ : List Constraint} {x y r : Name} {c : Concept}
        (hax : Axiom.rng r c ∈ A) (hr : r ∈ roleNames A)
        (hedge : Constraint.role r x y ∈ Γ) (hdom : ∃ e, Constraint.conc x e ∈ Γ)
        (next : Closed A (Constraint.conc y c :: Γ)) : Closed A Γ
  /-- Role hierarchy. -/
  | subroleR {Γ : List Constraint} {x y r t : Name}
        (hax : Axiom.subrole r t ∈ A) (hedge : Constraint.role r x y ∈ Γ)
        (hdom : ∃ e, Constraint.conc x e ∈ Γ)
        (next : Closed A (Constraint.role t x y :: Γ)) : Closed A Γ
  /-- Non-emptiness: the entry point from the EMPTY branch, and so the rule a
  satisfiability question is asked through. -/
  | nonemptyR {Γ : List Constraint} {y : Name} {c : Concept}
        (hax : Axiom.nonempty c ∈ A)
        (hfresh : y ∉ branchNames Γ) (hfreshA : y ∉ indNames A)
        (next : Closed A (Constraint.conc y c :: Γ)) : Closed A Γ
  /-- The ≥ rule. `hfresh` and `hnd` together are the side condition: the
  witnesses are new to the branch AND distinct from each other. -/
  | minR {Γ : List Constraint} {x r : Name} {c : Concept} {n : Nat} {ys : List Name}
        (hmem : Constraint.conc x (.min n r c) ∈ Γ)
        (hr : r ∈ roleNames A)
        (hlen : ys.length = n) (hnd : ys.Nodup)
        (hfresh : ∀ y ∈ ys, y ∉ branchNames Γ) (hfreshA : ∀ y ∈ ys, y ∉ indNames A)
        (next : Closed A (minExpand r x c ys ++ Γ)) : Closed A Γ
  /-- The bound clash: a lower bound above an upper bound on the same role and
  concept. Needs no witnesses, so it closes without running the ≥ rule. -/
  | minmaxClash {Γ : List Constraint} {x r : Name} {c : Concept} {m n : Nat}
        (hmin : Constraint.conc x (.min m r c) ∈ Γ)
        (hmax : Constraint.conc x (.max n r c) ∈ Γ)
        (hlt : n < m) : Closed A Γ
  /-- The cardinality clash: `n+1` pairwise distinct successors against a
  bound of `n`. A leaf, not a rule, which is why no merging appears here. -/
  | maxClash {Γ : List Constraint} {x r : Name} {c : Concept} {n : Nat} {ys : List Name} (hmax : Constraint.conc x (.max n r c) ∈ Γ)
        (hlen : ys.length = n + 1) (hnd : ys.Nodup)
        (hdiff : ∀ a ∈ ys, ∀ b ∈ ys, a ≠ b → Constraint.diff a b ∈ Γ)
        (hedge : ∀ y ∈ ys, Constraint.role r x y ∈ Γ)
        (hc : ∀ y ∈ ys, Constraint.conc y c ∈ Γ) : Closed A Γ

/-- **A closed tableau is a refutation.**

By induction on the tableau. Every clash constructor is a branch nothing can
satisfy; every rule constructor preserves satisfiability downward, so an
unsatisfiable child forces an unsatisfiable parent. The disjunction case is the
only one with two children and it needs both, which is exactly why a tableau is
a tree rather than a list. -/
theorem closed_sound {A : List Axiom} {Γ : List Constraint} (h : Closed A Γ) :
    ¬ BranchSat A Γ := by
  induction h with
  | bot hb => exact clash_bot hb
  | neg hp hn => exact clash_neg hp hn
  | diff hd => exact clash_diff hd
  | disjointClash hax hc hd => exact clash_disjoint hax hc hd
  | andR hmem _ ih => exact fun hs => ih (and_sound hmem hs)
  | allR hall hedge hr _ ih => exact fun hs => ih (all_sound hall hedge hr hs)
  | orR hmem _ _ ihl ihr =>
      intro hs
      rcases or_sound hmem hs with hl | hr
      · exact ihl hl
      · exact ihr hr
  | instR hmem hind _ ih => exact fun hs => ih (inst_sound hmem hind hs)
  | relR hmem _ ih => exact fun hs => ih (rel_sound hmem hs)
  | subR hsub hx _ ih => exact fun hs => ih (sub_sound hsub hx hs)
  | domR hax hedge hdom _ ih => exact fun hs => ih (dom_sound hax hedge hdom hs)
  | rngR hax hr hedge hdom _ ih => exact fun hs => ih (rng_sound hax hr hedge hdom hs)
  | subroleR hax hedge hdom _ ih => exact fun hs => ih (subrole_sound hax hedge hdom hs)
  | nonemptyR hax hfresh hfreshA _ ih =>
      exact fun hs => ih (nonempty_sound hax hfresh hfreshA hs)
  | exR hmem hr hfresh hfreshA _ ih => exact fun hs => ih (ex_sound hmem hr hfresh hfreshA hs)
  | minR hmem hr hlen hnd hfresh hfreshA _ ih =>
      exact fun hs => ih (min_sound hmem hr hlen hnd hfresh hfreshA hs)
  | minmaxClash hmin hmax hlt => exact clash_minmax hmin hmax hlt
  | maxClash hmax hlen hnd hdiff hedge hc => exact clash_max hmax hlen hnd hdiff hedge hc

/-- **A closed tableau on the empty branch means the axioms have no model.**

This is the statement `oo-refute` would print for a description-logic
refutation, and the one `DlMain.lean` currently says is not produced. It is
`Unsatisfiable` from `Dl/General.lean`, so it quantifies over every carrier and
is not the weaker claim that no FINITE model exists.

The assignment is the only thing needing construction: `BranchSat` carries one
and the empty branch constrains it nowhere, so any constant map into the
carrier will do, and `GModels.nonempty` is what says there is something to map
to. -/
theorem unsatisfiable_of_closed {A : List Axiom} (h : Closed A []) : Unsatisfiable A := by
  intro α I hM
  refine closed_sound h ⟨α, I, I.ind, hM, fun _ _ => rfl, ?_⟩
  intro κ hκ
  exact absurd hκ (List.not_mem_nil)

/-! ## Axioms, pinned

The same footprint the rest of the directory carries. A `sorry` anywhere above,
or a `native_decide` smuggled into a rule, changes one of these lines and the
build fails. -/

/-- info: 'Dl.closed_sound' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms closed_sound

/-- info: 'Dl.unsatisfiable_of_closed' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms unsatisfiable_of_closed

/-- info: 'Dl.min_sound' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms min_sound

end Dl
