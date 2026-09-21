/-!
# A numeric claim, checked

`OOCert` certifies that a triple follows from a graph. Nothing in this
repository could certify that a NUMBER follows from other numbers, and the gap
showed: an ontology pipeline that computes a similarity score, an alignment
metric or a risk figure hands that number on with no more standing than an
assertion.

This is the smallest honest closure of that gap: matrix multiplication over the
integers, checked by recomputation against the definition, with a theorem in
core Lean saying what an acceptance means.

## Why integers, and why this is the exact path only

Freivalds' algorithm checks `A × B = C` in O(n²) by multiplying both sides by a
random vector, and its soundness is a PROBABILISTIC statement: the check passes
a wrong product with probability at most 2⁻ᵏ after k trials. Stating that needs
probability theory, and this development has no Mathlib by deliberate choice —
`lakefile.toml` says why. So the probabilistic path stays where it can be
honest: in Rust, as this engine's own opinion, with its bound printed and the
word `certified` withheld.

Floating point is worse and is excluded outright. A proof over ℝ says nothing
about IEEE-754: rounding makes `A × B = C` fail for a correct implementation
and hold for an incorrect one. A certificate proved over the reals and run on
hardware is unsound, silently. `Mat.check` takes integers and the Rust side
refuses to hand it anything else.

## What the theorem says

`MatCert.mul_of_check` : if `check A B C` returns true then every entry of `C`
is the sum over k of `A[i][k] * B[k][j]`. That is the textbook definition,
written out in `spec`, and the checker is measured against it rather than
against a second implementation that could share its mistakes.

The theorem is deliberately about the DEFINITION and not about `A × B` as some
other function computes it. A checker that agreed with a wrong multiplier would
be worthless, and this one cannot: it recomputes from the spec.
-/

namespace Mat

abbrev Matrix := List (List Int)

/-- `A[i][j]`, or zero outside the matrix. Out of range is not an error here:
the shape check below refuses ragged input before this is ever reached, so a
default keeps the arithmetic total without hiding anything. -/
def at! (M : Matrix) (i j : Nat) : Int :=
  match M[i]? with
  | none => 0
  | some row => match row[j]? with
    | none => 0
    | some v => v

/-- The definition: `C[i][j] = Σ_{k<p} A[i][k] * B[k][j]`. -/
def spec (A B : Matrix) (p i j : Nat) : Int :=
  (List.range p).foldl (fun acc k => acc + at! A i k * at! B k j) 0

/-- Every row has the stated width. Ragged input is refused rather than padded,
because a padded row is a different matrix and would be checked as one. -/
def rectangular (M : Matrix) (w : Nat) : Bool :=
  M.all (fun row => row.length == w)

/-- Shapes agree: `A` is m×p, `B` is p×n, `C` is m×n. -/
def shapesAgree (A B C : Matrix) (m p n : Nat) : Bool :=
  A.length == m && B.length == p && C.length == m
    && rectangular A p && rectangular B n && rectangular C n

/-- The check: every entry of `C` equals the definition. -/
def check (A B C : Matrix) (m p n : Nat) : Bool :=
  shapesAgree A B C m p n
    && (List.range m).all (fun i =>
         (List.range n).all (fun j => at! C i j == spec A B p i j))

end Mat

namespace MatCert

/-- **What an acceptance means.**

If `Mat.check` returns true then every entry of `C` within the stated shape is
the sum of products the definition calls for. Nothing weaker and nothing more:
it says nothing about entries outside the shape, because `shapesAgree` has
already refused those inputs. -/
theorem mul_of_check {A B C : Mat.Matrix} {m p n : Nat}
    (h : Mat.check A B C m p n = true) :
    ∀ i, i < m → ∀ j, j < n → Mat.at! C i j = Mat.spec A B p i j := by
  simp [Mat.check, Bool.and_eq_true] at h
  exact h.2

/-- The shapes the acceptance was about, so a reader cannot apply the theorem
to a different matrix by reading only its conclusion. -/
theorem shapes_of_check {A B C : Mat.Matrix} {m p n : Nat}
    (h : Mat.check A B C m p n = true) : Mat.shapesAgree A B C m p n = true := by
  simp [Mat.check, Bool.and_eq_true] at h
  exact h.1

end MatCert
