import OOBoundary.Generated

/-!
# What the boundary functions are supposed to do

Written in plain Lean over `List UInt8`, with no reference to the generated
model. That separation is the point: a specification written by reading the
translation back is a restatement, and a restatement proves nothing. These
definitions say what `asserted.tsv` and `axioms.tsv` need in order to be
readable, and `OOBoundary/Proofs.lean` is where they meet the model Aeneas
produced from `src/boundary_core.rs`.

The bytes, once, so that no theorem below has to spell them:

* `9` is TAB, the field separator of every certificate file;
* `10` is LINE FEED, the record separator;
* `13` is CARRIAGE RETURN, which is not a separator here but would make one
  half of a CRLF file and is refused for that reason;
* `32` is SPACE, the separator INSIDE a concept in `axioms.tsv` and
  `model.tsv` (`concept_string` in `src/tableaux.rs`), which is why the DL
  path refuses one more byte than the rest;
* `60` is `<`, `34` is `"`, `95` is `_` and `58` is `:`, the leading bytes of
  the three N-Triples term spellings.
-/

namespace OOBoundary.Spec

open Aeneas.Std

/-- A byte no tab-separated, one-record-per-line file can carry. -/
def isTsvSeparator (b : U8) : Bool := b = 9#u8 || b = 10#u8 || b = 13#u8

/-- A byte the DL model certificate cannot carry: the three above and SPACE,
because the concept grammar inside a field is space separated. -/
def isDlSeparator (b : U8) : Bool := b = 32#u8 || isTsvSeparator b

/-- TCB-4, as a property of a list of bytes: a field is non-empty and carries
no separator. This is the half that is about the FORMAT. -/
def fieldFits (l : List U8) : Bool :=
  !l.isEmpty && l.all (fun b => !isTsvSeparator b)

/-- TCB-5, the half that is about N-TRIPLES: a term begins with `<` (an IRI),
`"` (a literal) or `_:` (a blank node), so no two spellings collide when the
checker compares terms as opaque strings. -/
def ntriplesLeadingByte (l : List U8) : Bool :=
  l[0]! = 60#u8 || l[0]! = 34#u8 || (l[0]! = 95#u8 && 1 < l.length && l[1]! = 58#u8)

/-- TCB-4 and TCB-5 together: what a term must satisfy to be written. -/
def termFits (l : List U8) : Bool := fieldFits l && ntriplesLeadingByte l

/-- TCB-25: a name that survives the round trip through `axioms.tsv` and
`model.tsv`. -/
def nameIsSafe (l : List U8) : Bool :=
  !l.isEmpty && l.all (fun b => !isDlSeparator b)

/-- The guard that closed the unwritable-conclusion defect: a literal cannot be
a subject and only an IRI can be a predicate. -/
def writableTriple (subject predicate : List U8) : Bool :=
  !(subject[0]? = some 34#u8) && predicate[0]? = some 60#u8

/-- One line of `asserted.tsv`, as bytes. -/
def assertedLine (s p o : List U8) : List U8 :=
  s ++ [9#u8] ++ p ++ [9#u8] ++ o ++ [10#u8]

/-- Three further fields of a line already begun. -/
def tripleFields (s p o : List U8) : List U8 :=
  [9#u8] ++ s ++ [9#u8] ++ p ++ [9#u8] ++ o

/-!
## Splitting, so that "exactly three fields" is a statement and not a picture

`OOCert.Parse.parseTriples` reads a line by splitting it on TAB. The theorem
worth proving about the writer is the one about what the READER gets back, so
the split has to exist here. This is `List.splitOn` specialised to one byte and
written out rather than imported, because the whole file has to be readable by
someone checking whether the statement says what they think it says.
-/

/-- Split a byte list on a separator byte. Always returns at least one field,
and `n` occurrences of the separator give `n + 1` fields. -/
def splitOnByte (sep : U8) : List U8 → List (List U8)
  | [] => [[]]
  | b :: bs =>
    if b = sep then [] :: splitOnByte sep bs
    else match splitOnByte sep bs with
         | [] => [[b]]          -- unreachable: `splitOnByte` is never empty
         | f :: fs => (b :: f) :: fs

/-- Splitting never returns nothing: there is always at least one field, even
in the empty list. -/
theorem splitOnByte_ne_nil (sep : U8) (l : List U8) : splitOnByte sep l ≠ [] := by
  induction l with
  | nil => simp [splitOnByte]
  | cons b bs ih =>
    by_cases hb : b = sep
    · simp [splitOnByte, hb]
    · cases h : splitOnByte sep bs with
      | nil => exact absurd h ih
      | cons f fs => simp [splitOnByte, hb, h]

/-- A list carrying no occurrence of `sep` is exactly one field. -/
theorem splitOnByte_of_not_mem (sep : U8) (l : List U8) (h : sep ∉ l) :
    splitOnByte sep l = [l] := by
  induction l with
  | nil => simp [splitOnByte]
  | cons b bs ih =>
    have hb : ¬ (b = sep) := by grind
    have hbs : sep ∉ bs := by grind
    simp [splitOnByte, hb, ih hbs]

/-- The one lemma the field-count theorems are built from: a separator between
two pieces closes the left piece's last field and opens the right piece's
first, so the fields of the whole are the fields of the parts, concatenated.

Note what it does NOT assume. `l` may itself carry separators, which is why
this is the lemma that lets a theorem about APPENDING fields to a line already
begun be stated for every prefix rather than for a separator-free one. -/
theorem splitOnByte_append_cons (sep : U8) (l r : List U8) :
    splitOnByte sep (l ++ sep :: r) = splitOnByte sep l ++ splitOnByte sep r := by
  induction l with
  | nil => simp [splitOnByte]
  | cons b bs ih =>
    by_cases hb : b = sep
    · simp [splitOnByte, hb, ih]
    · cases h : splitOnByte sep bs with
      | nil => exact absurd h (splitOnByte_ne_nil sep bs)
      | cons f fs =>
        have hcat : splitOnByte sep (bs ++ sep :: r) = (f :: fs) ++ splitOnByte sep r := by
          rw [ih, h]
        simp [splitOnByte, hb, hcat, h]

/-- Appending a separator and a piece adds exactly that piece's field count. -/
theorem splitOnByte_length_append_cons (sep : U8) (l r : List U8) :
    (splitOnByte sep (l ++ sep :: r)).length
      = (splitOnByte sep l).length + (splitOnByte sep r).length := by
  rw [splitOnByte_append_cons]; simp

end OOBoundary.Spec
