import Lrat
import SelfId.All

/-!
`oo-lrat PROBLEM.cnf PROOF.lrat`

Reads a DIMACS CNF and an LRAT proof, checks the proof against that formula,
and prints one JSON object.

Exit codes, matching the convention the other checkers here use:

* `0` the proof checks, so **the formula in `PROBLEM.cnf` has no model**. The
  statement covering that is `Lrat.unsat_of_check`.
* `1` the proof does not check. That is a statement about the CERTIFICATE and
  not about the formula: a solver can fail to emit a proof that exists, and
  this exit code never means the formula is satisfiable.
* `2` a file could not be read or parsed. No verdict either way.

## What exit 0 does not say

* Nothing about the SOLVER. A checked proof is checked; the search that found
  it is still untrusted, which is the point of checking it.
* Nothing about whether `PROBLEM.cnf` is your problem. The translation that
  produced the CNF is outside this checker, and a proof of the wrong formula
  is a proof of the wrong formula however well it checks.
* Nothing about first-order satisfiability. This is propositional. A CNF that
  came from grounding a first-order problem at some finite size says what it
  says about that grounding and no more.

## Deletion lines

`d` lines are parsed and IGNORED. Deleting a clause is a performance device
for the checker's own propagation, not part of what is proved: every clause
this checker holds is either an original one or one it has already verified is
implied, so keeping a clause that the proof deleted can only make propagation
easier. It cannot make an accepted proof unsound, because `check_sound` proves
the invariant over whatever formula it is actually carrying.
-/

open Lrat

/-- Split on runs of whitespace, dropping empties. -/
def tokens (s : String) : List String :=
  (s.splitOn " ").flatMap (fun a => (a.splitOn "\t").flatMap (fun b => b.splitOn "\r"))
    |>.filter (· ≠ "")

def parseInt? (s : String) : Option Int :=
  if s.startsWith "-" then
    match (s.drop 1).toNat? with
    | some n => if n == 0 then none else some (-(Int.ofNat n))
    | none => none
  else
    match s.toNat? with
    | some n => some (Int.ofNat n)
    | none => none

/-- A DIMACS literal to a `Lit`. Zero is the clause terminator and never a
literal, so it is rejected here rather than given a polarity. -/
def litOfInt? (i : Int) : Option Lit :=
  if i > 0 then some ⟨i.toNat, true⟩
  else if i < 0 then some ⟨(-i).toNat, false⟩
  else none

/-- DIMACS CNF. `c` comments and the `p cnf` header are skipped; every other
line is a clause terminated by `0`. Clauses are numbered from 1 in the order
they appear, which is what LRAT hints refer to. -/
def parseCnf (text : String) : Option Formula := do
  let mut out : Formula := []
  let mut id := 1
  let mut cur : Clause := []
  let mut open_ := false
  for line in text.splitOn "\n" do
    let ts := tokens line
    match ts with
    | [] => pure ()
    | h :: _ =>
      if h == "c" || h == "p" || h == "%" then pure ()
      else
        for t in ts do
          let i ← parseInt? t
          if i == 0 then
            out := out ++ [(id, cur)]
            id := id + 1
            cur := []
            open_ := false
          else
            let l ← litOfInt? i
            cur := cur ++ [l]
            open_ := true
  -- A clause left unterminated is a truncated file, not an empty clause.
  if open_ then none else some out

/-- One LRAT line: `id lit* 0 hint* 0`, or `id d id* 0`, which is ignored. -/
def parseLratLine (ts : List String) : Option (Option Line) :=
  match ts with
  | [] => some none
  | idTok :: rest =>
    match idTok.toNat? with
    | none => none
    | some id =>
      match rest with
      | "d" :: _ => some none          -- a deletion; see the header
      | _ => do
        let mut lits : Clause := []
        let mut hints : List Nat := []
        let mut seenZero := false
        let mut closed := false
        for t in rest do
          let i ← parseInt? t
          if i == 0 then
            if seenZero then closed := true else seenZero := true
          else if !seenZero then
            let l ← litOfInt? i
            lits := lits ++ [l]
          else
            -- Hints may be negative in full LRAT (RAT hints). This checker
            -- implements the RUP fragment only, so a negative hint is a
            -- proof it cannot check rather than one it may skip.
            if i < 0 then failure
            else hints := hints ++ [i.toNat]
        if closed then some (some ⟨id, lits, hints⟩) else none

def parseLrat (text : String) : Option (List Line) := do
  let mut out : List Line := []
  for line in text.splitOn "\n" do
    match ← parseLratLine (tokens line) with
    | none => pure ()
    | some ln => out := out ++ [ln]
  some out

def jsonStr (s : String) : String :=
  "\"" ++ (s.foldl (fun acc c =>
    acc ++ (if c == '"' then "\\\"" else if c == '\\' then "\\\\" else String.singleton c)) "") ++ "\""

def main (args : List String) : IO UInt32 := do
  match args with
  -- FIRST, because `oo-resolution` and friends take a single positional
  -- argument and a later arm would swallow `--version` as a path (#204).
  | ["--version"] => SelfId.emitVersion
  | [cnfPath, prfPath] =>
    let cnfText ← try IO.FS.readFile cnfPath catch _ =>
      IO.println s!"\{\"ok\":false,\"error\":\"could not read {cnfPath}\"}"
      return 2
    let prfText ← try IO.FS.readFile prfPath catch _ =>
      IO.println s!"\{\"ok\":false,\"error\":\"could not read {prfPath}\"}"
      return 2
    match parseCnf cnfText, parseLrat prfText with
    | some F, some prf =>
      if check F prf then
        SelfId.println s!"\{\"ok\":true,\"verdict\":\"unsatisfiable\",\
          \"clauses\":{F.length},\"lines\":{prf.length},\
          \"theorem\":\"Lrat.unsat_of_check\",\
          \"means\":\"the formula in this CNF has no model, of any size\"}"
        return 0
      else
        IO.println s!"\{\"ok\":false,\"verdict\":\"not_checked\",\
          \"clauses\":{F.length},\"lines\":{prf.length},\
          \"means\":\"the proof did not check. This says nothing about whether \
the formula is satisfiable: a solver can fail to emit a proof that exists\"}"
        return 1
    | none, _ =>
      IO.println s!"\{\"ok\":false,\"error\":\"could not parse {cnfPath} as DIMACS CNF\"}"
      return 2
    | _, none =>
      IO.println s!"\{\"ok\":false,\"error\":\"could not parse {prfPath} as LRAT\"}"
      return 2
  | _ =>
    IO.println "usage: oo-lrat PROBLEM.cnf PROOF.lrat"
    return 2
