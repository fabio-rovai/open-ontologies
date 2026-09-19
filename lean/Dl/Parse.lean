import Dl.Check
import Dl.Refute
import Std.Data.HashMap

/-!
# Reading the two files

`Dl/Syntax.lean` documents the format. This module turns the bytes into the `Axiom` list and
the `Interp` the checker is proved about.

It is NOT part of the theorem. A parse error is exit 2, never an accepted certificate, and
never a rejected one either: "I could not read the file" and "this is not a model" are
different answers and the exit codes keep them apart.

The three extension maps are built as hash tables and handed to `Interp` as closures over
them, so an atomic concept lookup costs one hash rather than a scan of the file. That is a
representation choice inside the parser; `Dl/Semantics.lean` only ever sees the functions.
-/
namespace Dl.Parse

/-- Split on spaces, dropping empties. IRIs keep their `<...>` spelling and contain no
space, so this is exact for the token streams the emitter writes. -/
def tokens (s : String) : List String :=
  (s.splitOn " ").filter (fun t => !t.isEmpty)

/-- Prefix parser for a concept. Every constructor has a fixed arity, so no parentheses are
needed and the grammar is unambiguous. The fuel is the token count, which strictly bounds
the recursion because each step consumes at least one token. -/
def concept? : Nat → List String → Option (Concept × List String)
  | 0, _ => none
  | _ + 1, [] => none
  | fuel + 1, t :: ts =>
    match t with
    | "top" => some (.top, ts)
    | "bot" => some (.bot, ts)
    | "atom" => match ts with
      | a :: ts' => some (.atom a, ts')
      | [] => none
    | "not" => (concept? fuel ts).map (fun p => (.neg p.1, p.2))
    | "and" => do
        let (c, ts1) ← concept? fuel ts
        let (d, ts2) ← concept? fuel ts1
        some (.and c d, ts2)
    | "or" => do
        let (c, ts1) ← concept? fuel ts
        let (d, ts2) ← concept? fuel ts1
        some (.or c d, ts2)
    | "some" => match ts with
      | r :: ts' => (concept? fuel ts').map (fun p => (.ex r p.1, p.2))
      | [] => none
    | "all" => match ts with
      | r :: ts' => (concept? fuel ts').map (fun p => (.all r p.1, p.2))
      | [] => none
    | "min" => match ts with
      | n :: r :: ts' => do
          let k ← n.toNat?
          let (c, ts2) ← concept? fuel ts'
          some (.min k r c, ts2)
      | _ => none
    | "max" => match ts with
      | n :: r :: ts' => do
          let k ← n.toNat?
          let (c, ts2) ← concept? fuel ts'
          some (.max k r c, ts2)
      | _ => none
    | _ => none

/-- A whole field must be exactly one concept, with nothing left over. -/
def parseConcept (s : String) : Option Concept :=
  let ts := tokens s
  match concept? (ts.length + 1) ts with
  | some (c, []) => some c
  | _ => none

def parseAxiomLine (line : String) : Option Axiom :=
  match line.splitOn "\t" with
  | ["sub", c, d] => do some (.sub (← parseConcept c) (← parseConcept d))
  | ["disjoint", c, d] => do some (.disjoint (← parseConcept c) (← parseConcept d))
  | ["domain", r, c] => do some (.dom r (← parseConcept c))
  | ["range", r, c] => do some (.rng r (← parseConcept c))
  | ["subrole", r, s] => some (.subrole r s)
  | ["trans", r] => some (.trans r)
  | ["sym", r] => some (.sym r)
  | ["inv", r, s] => some (.inv r s)
  | ["invfunc", r] => some (.invfunc r)
  | ["inst", a, c] => do some (.inst a (← parseConcept c))
  | ["rel", a, r, b] => some (.rel a r b)
  | ["indiv", a] => some (.indiv a)
  | ["nonempty", c] => do some (.nonempty (← parseConcept c))
  | _ => none

def parseAxioms (content : String) : Except String (List Axiom) := do
  let mut out : Array Axiom := #[]
  let mut n := 0
  for line in (content.replace "\r\n" "\n").splitOn "\n" do
    n := n + 1
    if line.isEmpty then continue
    match parseAxiomLine line with
    | some a => out := out.push a
    | none => throw s!"axioms line {n}: not a well-formed axiom"
  return out.toList

/-- The lookup tables an `Interp` closes over. -/
structure Tables where
  dom : Array Name := #[]
  cmap : Std.HashMap Name (List Name) := ∅
  rmap : Std.HashMap (Name × Name) (List Name) := ∅
  imap : Std.HashMap Name Name := ∅

/-- An individual with no `ind` line denotes the empty name, which is in no domain, so the
`indiv` axiom and the `indInDom` clause of `WellFormed` both fail rather than succeed by
accident. Silence is never taken for agreement. -/
def Tables.toInterp (t : Tables) : Interp where
  dom := t.dom.toList
  cext := fun a => t.cmap.getD a []
  rext := fun r x => t.rmap.getD (r, x) []
  ind := fun a => t.imap.getD a ""

def parseModel (content : String) : Except String Interp := do
  let mut t : Tables := {}
  let mut n := 0
  for line in (content.replace "\r\n" "\n").splitOn "\n" do
    n := n + 1
    if line.isEmpty then continue
    match line.splitOn "\t" with
    | ["domain", e] => t := { t with dom := t.dom.push e }
    | ["class", c, e] => t := { t with cmap := t.cmap.insert c (e :: t.cmap.getD c []) }
    | ["edge", x, r, y] => t := { t with rmap := t.rmap.insert (r, x) (y :: t.rmap.getD (r, x) []) }
    | ["ind", a, e] => t := { t with imap := t.imap.insert a e }
    | _ => throw s!"model line {n}: not a well-formed model fact"
  return t.toInterp

/-! ## The refutation certificate

One token stream in prefix order, exactly like the concept grammar and for the
same reason: every constructor has a fixed arity, so there are no parentheses
and nothing to get unbalanced. The two variable-length rules carry an explicit
count before their name list, which is what keeps the arity fixed.

Newlines are not significant. A producer that writes one step per line and a
producer that writes the whole tree on one line are the same certificate, and
the parser cannot tell them apart. That is deliberate: the tree structure is
carried by the arities, so indentation can be for a human without any risk of
becoming load-bearing. -/

/-- Read exactly `k` names. -/
def names? : Nat → List String → Option (List Name × List String)
  | 0, ts => some ([], ts)
  | k + 1, t :: ts => (names? k ts).map (fun p => (t :: p.1, p.2))
  | _ + 1, [] => none

/-- Prefix parser for a certificate. The fuel is the token count, which strictly
bounds the recursion because every step consumes its own keyword first. -/
def cert? : Nat → List String → Option (Cert × List String)
  | 0, _ => none
  | _ + 1, [] => none
  | fuel + 1, t :: ts =>
    match t, ts with
    | "bot", x :: ts' => some (.botC x, ts')
    | "diff", x :: ts' => some (.diffC x, ts')
    | "disjoint", x :: ts' => do
        let (c, ts1) ← concept? fuel ts'
        let (d, ts2) ← concept? fuel ts1
        some (.disjC x c d, ts2)
    | "neg", x :: ts' => (concept? fuel ts').map (fun p => (.negC x p.1, p.2))
    | "minmax", x :: r :: m :: n :: ts' => do
        let m ← m.toNat?
        let n ← n.toNat?
        let (c, ts1) ← concept? fuel ts'
        some (.minmaxC x r c m n, ts1)
    | "maxclash", x :: r :: n :: k :: ts' => do
        let n ← n.toNat?
        let k ← k.toNat?
        let (ys, ts1) ← names? k ts'
        let (c, ts2) ← concept? fuel ts1
        some (.maxC x r c n ys, ts2)
    | "inst", a :: ts' => do
        let (c, ts1) ← concept? fuel ts'
        let (k, ts2) ← cert? fuel ts1
        some (.instS a c k, ts2)
    | "rel", a :: r :: b :: ts' => (cert? fuel ts').map (fun p => (.relS a r b p.1, p.2))
    | "sub", x :: ts' => do
        let (c, ts1) ← concept? fuel ts'
        let (d, ts2) ← concept? fuel ts1
        let (k, ts3) ← cert? fuel ts2
        some (.subS x c d k, ts3)
    | "domain", x :: y :: r :: ts' => do
        let (c, ts1) ← concept? fuel ts'
        let (k, ts2) ← cert? fuel ts1
        some (.domS x y r c k, ts2)
    | "range", x :: y :: r :: ts' => do
        let (c, ts1) ← concept? fuel ts'
        let (k, ts2) ← cert? fuel ts1
        some (.rngS x y r c k, ts2)
    | "subrole", x :: y :: r :: u :: ts' =>
        (cert? fuel ts').map (fun p => (.subroleS x y r u p.1, p.2))
    | "nonempty", y :: ts' => do
        let (c, ts1) ← concept? fuel ts'
        let (k, ts2) ← cert? fuel ts1
        some (.nonemptyS y c k, ts2)
    | "and", x :: ts' => do
        let (c, ts1) ← concept? fuel ts'
        let (d, ts2) ← concept? fuel ts1
        let (k, ts3) ← cert? fuel ts2
        some (.andS x c d k, ts3)
    | "all", x :: y :: r :: ts' => do
        let (c, ts1) ← concept? fuel ts'
        let (k, ts2) ← cert? fuel ts1
        some (.allS x y r c k, ts2)
    | "some", x :: y :: r :: ts' => do
        let (c, ts1) ← concept? fuel ts'
        let (k, ts2) ← cert? fuel ts1
        some (.exS x y r c k, ts2)
    | "min", x :: r :: n :: k :: ts' => do
        let n ← n.toNat?
        let k ← k.toNat?
        let (ys, ts1) ← names? k ts'
        let (c, ts2) ← concept? fuel ts1
        let (kid, ts3) ← cert? fuel ts2
        some (.minS x r c n ys kid, ts3)
    | "or", x :: ts' => do
        let (c, ts1) ← concept? fuel ts'
        let (d, ts2) ← concept? fuel ts1
        let (l, ts3) ← cert? fuel ts2
        let (r, ts4) ← cert? fuel ts3
        some (.orS x c d l r, ts4)
    | _, _ => none

/-- A certificate is the WHOLE file. Trailing tokens are a parse error, not
something to ignore: a producer that emitted a second tree, or truncated the
first, must not be read as having emitted one good one. -/
def parseCert (content : String) : Except String Cert :=
  let ts := tokens ((content.replace "\r\n" " ").replace "\n" " " |>.replace "\t" " ")
  match cert? (ts.length + 1) ts with
  | some (c, []) => .ok c
  | some (_, rest) => .error s!"certificate: {rest.length} tokens left over after the tree"
  | none => .error "certificate: not a well-formed derivation"

end Dl.Parse
