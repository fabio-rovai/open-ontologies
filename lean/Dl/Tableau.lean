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
under which every constraint on the branch holds. -/
def BranchSat (A : List Axiom) (Γ : List Constraint) : Prop :=
  ∃ (α : Type) (I : GInterp α) (σ : Assign α),
    GModels I A ∧ ∀ κ ∈ Γ, CHolds I σ κ

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
  obtain ⟨α, I, σ, hM, hΓ⟩ := h
  refine ⟨α, I, σ, hM, ?_⟩
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
    (hy : ∃ d, Constraint.conc y d ∈ Γ)
    (h : BranchSat A Γ) :
    BranchSat A (Constraint.conc y c :: Γ) := by
  obtain ⟨α, I, σ, hM, hΓ⟩ := h
  refine ⟨α, I, σ, hM, ?_⟩
  intro κ hκ
  rcases hκ with _ | ⟨_, hκ⟩
  · have hx := hΓ _ hall
    have he := hΓ _ hedge
    simp only [CHolds, GSat] at hx he
    obtain ⟨d, hd⟩ := hy
    exact ⟨(hΓ _ hd).1, hx.2 (σ y) he⟩
  · exact hΓ _ hκ

/-- Disjunction. `x : C ⊔ D` licenses a split, and a model of the branch is a
model of one side or the other. This is the only rule that branches, and the
theorem at the bottom is what turns "both children closed" into a refutation. -/
theorem or_sound {A : List Axiom} {Γ : List Constraint} {x : Name} {c d : Concept}
    (hmem : Constraint.conc x (.or c d) ∈ Γ) (h : BranchSat A Γ) :
    BranchSat A (Constraint.conc x c :: Γ) ∨ BranchSat A (Constraint.conc x d :: Γ) := by
  obtain ⟨α, I, σ, hM, hΓ⟩ := h
  have hx := hΓ _ hmem
  simp only [CHolds, GSat] at hx
  rcases hx.2 with hc | hd
  · refine Or.inl ⟨α, I, σ, hM, ?_⟩
    intro κ hκ
    rcases hκ with _ | ⟨_, hκ⟩
    · exact ⟨hx.1, hc⟩
    · exact hΓ _ hκ
  · refine Or.inr ⟨α, I, σ, hM, ?_⟩
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
    (h : BranchSat A Γ) :
    BranchSat A (Constraint.role r x y :: Constraint.conc y c :: Γ) := by
  obtain ⟨α, I, σ, hM, hΓ⟩ := h
  have hx := hΓ _ hmem
  simp only [CHolds, GSat] at hx
  obtain ⟨w, hedge, hsat⟩ := hx.2
  -- `x` is on the branch and `y` is not, so they are different names and the
  -- update leaves `σ x` alone.
  have hxy : x ≠ y := by
    rintro rfl
    exact hfresh (List.mem_flatMap.mpr ⟨_, hmem, by simp [Constraint.names]⟩)
  refine ⟨α, I, fun n => if n = y then w else σ n, hM, ?_⟩
  intro κ hκ
  rcases hκ with _ | ⟨_, hκ⟩
  · simpa [CHolds, hxy, if_neg hxy] using hedge
  · rcases hκ with _ | ⟨_, hκ⟩
    · refine ⟨?_, ?_⟩
      · simpa using hM.rextDom r hr (σ x) w hx.1 hedge
      · simpa using hsat
    · exact branch_update_fresh hfresh hΓ _ hκ

/-! ## Clashes

A clash is a branch that no interpretation can satisfy, and each is a small
lemma rather than an entry in a list the checker trusts. -/

/-- `x : ⊥`. -/
theorem clash_bot {A : List Axiom} {Γ : List Constraint} {x : Name}
    (h : Constraint.conc x .bot ∈ Γ) : ¬ BranchSat A Γ := by
  rintro ⟨α, I, σ, _, hΓ⟩
  exact (hΓ _ h).2

/-- `x : C` and `x : ¬C`. -/
theorem clash_neg {A : List Axiom} {Γ : List Constraint} {x : Name} {c : Concept}
    (hp : Constraint.conc x c ∈ Γ) (hn : Constraint.conc x (.neg c) ∈ Γ) :
    ¬ BranchSat A Γ := by
  rintro ⟨α, I, σ, _, hΓ⟩
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
  rintro ⟨α, I, σ, _, hΓ⟩
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

/-- `x ≠ x`. The inequality rule for number restrictions introduces these, and
a branch that has asserted an individual differs from itself is closed. -/
theorem clash_diff {A : List Axiom} {Γ : List Constraint} {x : Name}
    (h : Constraint.diff x x ∈ Γ) : ¬ BranchSat A Γ := by
  rintro ⟨α, I, σ, _, hΓ⟩
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
  | andR {Γ x c d} (hmem : Constraint.conc x (.and c d) ∈ Γ)
        (next : Closed A (Constraint.conc x c :: Constraint.conc x d :: Γ)) : Closed A Γ
  | allR {Γ x y r c} (hall : Constraint.conc x (.all r c) ∈ Γ)
        (hedge : Constraint.role r x y ∈ Γ)
        (hy : ∃ d, Constraint.conc y d ∈ Γ)
        (next : Closed A (Constraint.conc y c :: Γ)) : Closed A Γ
  | orR {Γ x c d} (hmem : Constraint.conc x (.or c d) ∈ Γ)
        (left : Closed A (Constraint.conc x c :: Γ))
        (right : Closed A (Constraint.conc x d :: Γ)) : Closed A Γ
  /-- The existential rule. `hfresh` is what keeps it sound and `hr` is what
  puts the invented witness in the carrier. -/
  | exR {Γ : List Constraint} {x y r : Name} {c : Concept} (hmem : Constraint.conc x (.ex r c) ∈ Γ)
        (hr : r ∈ roleNames A)
        (hfresh : y ∉ branchNames Γ)
        (next : Closed A (Constraint.role r x y :: Constraint.conc y c :: Γ)) : Closed A Γ
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
  | andR hmem _ ih => exact fun hs => ih (and_sound hmem hs)
  | allR hall hedge hy _ ih => exact fun hs => ih (all_sound hall hedge hy hs)
  | orR hmem _ _ ihl ihr =>
      intro hs
      rcases or_sound hmem hs with hl | hr
      · exact ihl hl
      · exact ihr hr
  | exR hmem hr hfresh _ ih => exact fun hs => ih (ex_sound hmem hr hfresh hs)
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
  refine closed_sound h ⟨α, I, fun _ => hM.nonempty.choose, hM, ?_⟩
  intro κ hκ
  exact absurd hκ (List.not_mem_nil)

end Dl
