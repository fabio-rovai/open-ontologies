import OOCert.InstitutionRdf
import Fol.Semantics

/-!
# The first-order institution, and the RDF-to-first-order comorphism

`Fol/Semantics.lean` already carries first-order structures and satisfaction,
transcribed from `owl-lean`'s `OwlLean/FOL/Basic.lean`. This file makes that an
institution, with the satisfaction condition for symbol renamings PROVED by
induction over the formula, and then builds the translation that decision 0005
is about: a triple goes to a binary atom, and a first-order model comes back as
an RDF interpretation.

## This is a THEOROIDAL comorphism, and it has to be

`OOCert.Interp.iext` is keyed by the DENOTATION of the predicate: `iext (ι p) x y`.
`Fol.Struc.p2` is keyed by the SYMBOL: `p2 p x y`. Two distinct predicate symbols
may denote the same domain element while carrying different extensions, and then
no ternary relation on the domain reproduces both, so the obvious model map
cannot be defined on arbitrary target structures.

**That is an ARGUMENT about the obvious construction and it is NOT a theorem.**
Nothing here quantifies over all possible signature maps, sentence maps and model
maps, so nothing here rules out a plain comorphism found by some other route, and
this file does not claim one does not exist. What it claims is that the route
taken below needed the theory and got it.

The fix is the one `src/tptp.rs` already ships and decision 0005 already
records: the target is not a signature but a THEORY. `cohAx` is the background
axiom that makes the reconstruction work, `Institution.Th` is where it lives,
and `Comorphism.Theoroidal` is the resulting shape. `owl-lean`'s adequacy
theorem has exactly this form, with `background ++ indAxioms inds` on the
first-order side.

## What is proved here and what is not

* `folInst.satCond` — satisfaction is invariant under renaming of unary
  predicate, binary predicate and constant symbols. Proved, by induction.
* `rdfToFol.satCond` — the translation's satisfaction condition. Proved, and it
  is the field that `Comorphism.mk` will not let anyone skip.
* Entailment preservation, by `Comorphism.preserves_entailment` applied. No new
  argument.
* **Model expansiveness is NOT proved and is NOT claimed.** So entailment
  REFLECTION is not available for this comorphism, which means a first-order
  prover's failure to find a proof says nothing here about RDF non-entailment.
  That is decision 0005 item 5 restated as a missing theorem rather than as a
  caution, and it is the honest state of the work.

## This is not `OwlLean.adequacy`

`OwlLean.adequacy` is a biconditional about the OWL 2 abstract syntax, with
`background`, `indAxioms` and a freshness discipline, and it lives in a separate
Lake package that this build does not and will not depend on: `lean/lakefile.toml`
says core Lean only, no external packages, and pulling `owl-lean` in would put a
second development inside this one's trust surface. What is here is the same
SHAPE at the RDF triple level, over the two institutions this repository already
has, with its own satisfaction condition proved from scratch. The relationship
between the two is stated in decision 0009 and is not a dependency.
-/
namespace OOCert

namespace FolInst

/-! ## The institution

`Sig` records a vocabulary and constrains NOTHING: `Sen` is every `Fol.Form` and
`Mod` is every `Fol.Struc`, whatever the signature says. That is deliberate and
it is the weakest-conditions rule of `Institution.lean` applied twice. A
`Fol.Struc` interprets every symbol, so there is no reduct to a sub-vocabulary to
take; a `Fol.Form` is a formula whatever symbols it mentions, so restricting
`Sen` would only delete instances of the satisfaction condition. The vocabulary
is carried because `Institution.Th` needs a base signature to hang a theory on
and because the comorphism below has to name the finite set its background
axioms range over.

`SigMor` carries THREE independent renamings, one for each symbol space, rather
than one renaming shared between them. `Fol/Syntax.lean` is explicit that the
unary predicate, binary predicate and constant spaces are separate fields and
that the prefixes in `src/tptp.rs` exist to keep them apart. Three maps admit
strictly more morphisms than one, and more morphisms is a stronger satisfaction
condition. -/

/-- A first-order signature: the vocabulary, recorded. -/
abbrev Sig : Type := List Fol.Sym

/-- A signature morphism: a renaming of each of the three symbol spaces. No
constraint, because `Sen` and `Mod` impose none. -/
structure Mor (S S' : Sig) where
  /-- Unary predicate symbols. -/
  un : Fol.Sym → Fol.Sym
  /-- Binary predicate symbols. -/
  bin : Fol.Sym → Fol.Sym
  /-- Constant symbols. -/
  con : Fol.Sym → Fol.Sym

/-- Renaming a first-order term. Variables are untouched: a signature morphism
renames vocabulary, never binders, which is why the satisfaction condition below
can share one assignment between the two sides. -/
def renameTm (rc : Fol.Sym → Fol.Sym) : Fol.Term → Fol.Term
  | .var n => .var n
  | .const k => .const (rc k)

/-- Renaming a formula, constructor for constructor. -/
def renameF (ru rb rc : Fol.Sym → Fol.Sym) : Fol.Form → Fol.Form
  | .app1 p t => .app1 (ru p) (renameTm rc t)
  | .app2 p t u => .app2 (rb p) (renameTm rc t) (renameTm rc u)
  | .eq t u => .eq (renameTm rc t) (renameTm rc u)
  | .tru => .tru
  | .fls => .fls
  | .neg f => .neg (renameF ru rb rc f)
  | .and f g => .and (renameF ru rb rc f) (renameF ru rb rc g)
  | .or f g => .or (renameF ru rb rc f) (renameF ru rb rc g)
  | .imp f g => .imp (renameF ru rb rc f) (renameF ru rb rc g)
  | .all n f => .all n (renameF ru rb rc f)
  | .ex n f => .ex n (renameF ru rb rc f)

/-- The reduct of a structure along a renaming: each symbol is interpreted by
whatever its image was interpreted by. The carrier is untouched, which is what
lets an assignment be shared between a structure and its reduct. -/
def redStruc (ru rb rc : Fol.Sym → Fol.Sym) (M : Fol.Struc) : Fol.Struc where
  Dom := M.Dom
  nonempty := M.nonempty
  p1 := fun s => M.p1 (ru s)
  p2 := fun s => M.p2 (rb s)
  c := fun s => M.c (rc s)

theorem eval_red (ru rb rc : Fol.Sym → Fol.Sym) (M : Fol.Struc)
    (e : Fol.Env M) (t : Fol.Term) :
    Fol.Term.eval (redStruc ru rb rc M) e t = Fol.Term.eval M e (renameTm rc t) := by
  cases t <;> rfl

/-- **The satisfaction condition of the first-order institution**, at the level
of a single assignment. Induction over the formula; the quantifier cases go
through because `Fol.update` does not mention the structure's interpretation
maps, so the same assignment serves both sides. -/
theorem holds_red (ru rb rc : Fol.Sym → Fol.Sym) (M : Fol.Struc) :
    ∀ (f : Fol.Form) (e : Fol.Env M),
      (Fol.Form.holds (redStruc ru rb rc M) e f ↔ Fol.Form.holds M e (renameF ru rb rc f)) := by
  intro f
  induction f with
  | app1 p t => intro e; simp only [renameF, Fol.Form.holds, eval_red]; exact Iff.rfl
  | app2 p t u => intro e; simp only [renameF, Fol.Form.holds, eval_red]; exact Iff.rfl
  | eq t u => intro e; simp only [renameF, Fol.Form.holds, eval_red]; exact Iff.rfl
  | tru => intro e; exact Iff.rfl
  | fls => intro e; exact Iff.rfl
  | neg f ih => intro e; simp only [renameF, Fol.Form.holds]; exact not_congr (ih e)
  | and f g ihf ihg =>
      intro e; simp only [renameF, Fol.Form.holds]; exact and_congr (ihf e) (ihg e)
  | or f g ihf ihg =>
      intro e; simp only [renameF, Fol.Form.holds]; exact or_congr (ihf e) (ihg e)
  | imp f g ihf ihg =>
      intro e; simp only [renameF, Fol.Form.holds]; exact imp_congr (ihf e) (ihg e)
  | all n f ih =>
      intro e
      simp only [renameF, Fol.Form.holds]
      exact ⟨fun h d => (ih (Fol.update e n d)).mp (h d),
             fun h d => (ih (Fol.update e n d)).mpr (h d)⟩
  | ex n f ih =>
      intro e
      simp only [renameF, Fol.Form.holds]
      exact ⟨fun ⟨d, hd⟩ => ⟨d, (ih (Fol.update e n d)).mp hd⟩,
             fun ⟨d, hd⟩ => ⟨d, (ih (Fol.update e n d)).mpr hd⟩⟩

/-- **The institution of first-order structures.**

`sat` is "true under EVERY assignment". `Fol.Entails` is not that relation: it
quantifies the assignment OUTSIDE the implication, so it is assignment-wise
rather than validity-based. The two agree on closed formulas and differ on open
ones, `fol_entails_gives_institution_entails` proves the direction that holds,
and `institution_entailment_is_strictly_weaker_on_open_formulas` refutes the
other. Nothing here quietly identifies them. -/
def folInst : Institution where
  Sig := Sig
  SigMor := Mor
  Sen := fun _ => Fol.Form
  Mod := fun _ => Fol.Struc
  senMap := fun σ f => renameF σ.un σ.bin σ.con f
  modRed := fun σ M => redStruc σ.un σ.bin σ.con M
  sat := fun M f => ∀ e : Fol.Env M, Fol.Form.holds M e f
  satCond := fun σ M f =>
    ⟨fun h e => (holds_red σ.un σ.bin σ.con M f e).mp (h e),
     fun h e => (holds_red σ.un σ.bin σ.con M f e).mpr (h e)⟩

/-- `Fol.Entails` gives institution entailment. The hypothesis is applied once
per assignment, which is exactly the step that does not run backwards. -/
theorem fol_entails_gives_institution_entails {S : Sig} (Γ : List Fol.Form) (f : Fol.Form)
    (h : Fol.Entails Γ f) : folInst.EntailsL (S := S) Γ f := by
  intro M hM e
  exact h M e (fun g hg => hM g hg e)

/-! ## The comorphism

A triple becomes a binary atom. A first-order structure becomes an RDF
interpretation by reading a property's extension off the symbols that denote it,
and that reading is only correct because the theory says symbols denoting the
same thing have the same extension. -/

/-- The background axiom: if two binary predicate symbols denote the same
individual then everything the first relates the second relates.

Only ONE direction is stated. The axiom set below ranges over every ORDERED pair
from the signature, so the other direction arrives as the axiom for the swapped
pair, and a biconditional here would constrain the target models more than the
proof needs. -/
def cohAx (q p : Fol.Sym) : Fol.Form :=
  .imp (.eq (.const q) (.const p))
    (.all 0 (.all 1 (.imp (.app2 q (.var 0) (.var 1)) (.app2 p (.var 0) (.var 1)))))

/-- The theory a signature is sent to: the same vocabulary, and one coherence
axiom per ordered pair of terms in it. A PREDICATE rather than a list, so
nothing has to enumerate the square of the signature. -/
def cohTheory (S : Rdf.Sig) : Institution.ThSig folInst where
  base := S
  ax := fun f => ∃ q, q ∈ S ∧ ∃ p, p ∈ S ∧ f = cohAx q p

/-- **The RDF-to-first-order comorphism, with its satisfaction condition
proved.**

The model map reads `iext d x y` as "some symbol OF THE SIGNATURE denotes `d`
and relates `x` to `y`". Restricting to the signature is not tidiness: a symbol
outside `S` has no coherence axiom, so the forward direction of the satisfaction
condition would have nothing to appeal to. This is the same scoping that makes
`Rdf.Sen` constrain the predicate position. -/
def rdfToFol : Theoroidal Rdf.simple folInst where
  sigMap := cohTheory
  senMap := fun φ => .app2 φ.1.p (.const φ.1.s) (.const φ.1.o)
  modMap := fun {S : Rdf.Sig} M =>
    { D := M.1.Dom
      ι := fun t => M.1.c t
      iext := fun d x y => ∃ q, q ∈ S ∧ M.1.c q = d ∧ M.1.p2 q x y }
  satCond := by
    intro S M φ
    constructor
    · rintro ⟨q, hq, hcq, hpq⟩ e
      have hax : folInst.sat M.1 (cohAx q φ.1.p) :=
        M.2 (cohAx q φ.1.p) ⟨q, hq, φ.1.p, φ.2, rfl⟩
      exact hax e hcq (M.1.c φ.1.s) (M.1.c φ.1.o) hpq
    · intro h
      refine M.1.nonempty.elim (fun d => ?_)
      exact ⟨φ.1.p, φ.2, rfl, h (fun _ => d)⟩

/-- Preservation, by the general theorem applied and nothing else. Unfolded
through `Institution.Th`, it says what decision 0005's shape says: the
first-order side must be asked the question with the background axioms in the
premise set. -/
theorem rdf_entailment_transfers_to_fol {S : Rdf.Sig} {Γ : Rdf.Sen S → Prop} {φ : Rdf.Sen S}
    (h : Rdf.simple.Entails Γ φ) :
    ∀ M : Fol.Struc, folInst.Models (S := S) M (cohTheory S).ax →
      folInst.Models (S := S) M (rdfToFol.image Γ) →
      folInst.sat (S := S) M (rdfToFol.senMap φ) :=
  rdfToFol.preserves_entailment_theoroidal h

end FolInst

end OOCert
