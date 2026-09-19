import Dl.Tableau

/-!
# The calculus refutes something

`Dl/Tableau.lean` proves that a closed tableau is a refutation. That theorem was
true of nothing at all until the axiom rules went in: every rule read from the
branch, the branch starts empty, so no derivation could begin and
`Closed A []` had no inhabitants. This file is the gate on that. It builds
closed tableaux by hand and reads `Unsatisfiable` off them, so if the calculus
ever goes vacuous again, `lake build` fails here rather than passing quietly.

Two derivations, chosen to exercise the two halves that were missing:

* `contradictory_unsat` runs the assertion, subsumption and clash rules. It is
  the smallest thing a TBox can get wrong.
* `cardinality_unsat` runs the ≥ rule, which invents two distinct witnesses at
  once, then closes on the ≤ bound that those witnesses violate. Nothing else
  in the file exercises `minExpand`, `pairDiffs` or `assignList`.
-/
namespace Dl.Demo
open Dl

/-! ## `C ⊑ D`, `C ⊑ ¬D`, and `C` is not empty -/

def A₁ : List Axiom :=
  [ .nonempty (.atom "C"), .sub (.atom "C") (.atom "D"),
    .sub (.atom "C") (.neg (.atom "D")) ]

theorem closed₁ : Closed A₁ [] := by
  refine .nonemptyR (c := .atom "C") (y := "w") (by simp [A₁]) (by simp [branchNames]) (by decide) ?_
  refine .subR (c := .atom "C") (d := .atom "D") (x := "w") (by simp [A₁]) (by simp) ?_
  refine .subR (c := .atom "C") (d := .neg (.atom "D")) (x := "w") (by simp [A₁]) (by simp) ?_
  exact .neg (c := .atom "D") (x := "w") (by simp) (by simp)

theorem contradictory_unsat : Unsatisfiable A₁ := unsatisfiable_of_closed closed₁

/-! ## `≥2 R.C` against `≤1 R.D`, with `C ⊑ D`

The ≥ rule invents `u` and `v`, distinct and both `C`. Subsumption lifts each to
`D`. Then `u` and `v` are two distinct `R`-successors in `D` where at most one
was allowed, which is `clash_max` with `n = 1`. This is the derivation that
merging would otherwise be needed for, and it does not need merging: the bound
enters as a clash rather than as a rule. -/

def A₂ : List Axiom :=
  [ .nonempty (.and (.min 2 "R" (.atom "C")) (.max 1 "R" (.atom "D"))),
    .sub (.atom "C") (.atom "D") ]

theorem closed₂ : Closed A₂ [] := by
  refine .nonemptyR (c := .and (.min 2 "R" (.atom "C")) (.max 1 "R" (.atom "D")))
    (y := "x") (by simp [A₂]) (by simp [branchNames]) (by decide) ?_
  refine .andR (x := "x") (c := .min 2 "R" (.atom "C")) (d := .max 1 "R" (.atom "D"))
    (by simp) ?_
  refine .minR (x := "x") (r := "R") (c := .atom "C") (n := 2) (ys := ["u", "v"])
    (by simp) (by simp [roleNames, A₂, Axiom.roles, Concept.roles])
    (by simp) (by decide) (by decide) (by decide) ?_
  refine .subR (c := .atom "C") (d := .atom "D") (x := "u") (by simp [A₂])
    (by simp [minExpand, pairDiffs]) ?_
  refine .subR (c := .atom "C") (d := .atom "D") (x := "v") (by simp [A₂])
    (by simp [minExpand, pairDiffs]) ?_
  exact .maxClash (x := "x") (r := "R") (c := .atom "D") (n := 1) (ys := ["u", "v"])
    (by simp [minExpand, pairDiffs]) (by simp) (by decide)
    (by decide) (by simp [minExpand, pairDiffs]) (by simp)

theorem cardinality_unsat : Unsatisfiable A₂ := unsatisfiable_of_closed closed₂

/-! ## The negative control

A gate that cannot fail is not a gate. This says the format cannot certify a
satisfiable ontology: hand it one that HAS a model and no closed tableau exists
for it, whatever a producer claims. It follows from soundness alone and needs no
completeness argument, which is the asymmetry the whole design rests on. -/
theorem no_closed_of_satisfiable {A : List Axiom} (h : Satisfiable A) :
    Closed A [] → False :=
  fun hc => no_finite_model_of_unsatisfiable (unsatisfiable_of_closed hc) h

/-! ## Axioms, pinned

These two are the evidence that the calculus is not vacuous, so they are the
lines that matter most: a footprint change here means a refutation stopped being
a refutation. -/

/-- info: 'Dl.Demo.contradictory_unsat' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms contradictory_unsat

/-- info: 'Dl.Demo.cardinality_unsat' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms cardinality_unsat

end Dl.Demo
