import Fo
import SelfId.All

/-!
`oo-resolution REFUTATION.cert`

Reads a clause set and a resolution refutation, replays it, and prints one JSON
object.

Exit codes, matching the convention the other checkers here use:

* `0` the refutation checks, so **the clause set has no model, over any
  carrier, finite or infinite**. The statement is `Fo.unsat_of_check`.
* `1` the refutation does not check. A statement about the CERTIFICATE and not
  about the clause set: a prover can fail to find a refutation that exists, and
  this exit code never means the set is satisfiable.
* `2` the file could not be read or parsed. No verdict either way.

## What exit 0 does not say

* Nothing about the prover. A checked refutation is checked; the search that
  found it is untrusted, which is why it is worth checking.
* Nothing about whether the clause set is your problem. Clausification and
  Skolemisation happen before this file and are not certified by it. A proof of
  the wrong clause set is a proof of the wrong clause set.

## The format

Tab separated. Terms are `vN` for a variable and `fN` or `fN[t,t]` for a
function. A literal is `+pN[t,t]` or `-pN[t,t]`. A clause is literals separated
by spaces, and the empty clause is the empty field. A substitution is
`N=t;N=t`.

    c   <id>  <clause>
    r   <id>  <concl>  <ci> <lc> <σc> <restc>  <dj> <ld> <σd> <restd>
    f   <id>  <concl>  <ci> <la> <lb> <σ> <rest>
-/

open Fo

/-- Split on a character, dropping empty pieces. -/
def splitDrop (s : String) (c : Char) : List String :=
  (s.splitOn (String.singleton c)).filter (· ≠ "")

mutual
/-- `vN`, `fN`, or `fN[t,t,...]`. Returns the term and the rest of the input. -/
partial def parseTerm (s : List Char) : Option (Term × List Char) :=
  match s with
  | 'v' :: rest =>
    let (ds, rest') := rest.span Char.isDigit
    if ds.isEmpty then none else some (.var (String.mk ds).toNat!, rest')
  | 'f' :: rest =>
    let (ds, rest') := rest.span Char.isDigit
    if ds.isEmpty then none else
      let f := (String.mk ds).toNat!
      match rest' with
      | '[' :: inner =>
        match parseArgs inner with
        | some (args, rest'') => some (.app f args, rest'')
        | none => none
      | _ => some (.app f [], rest')
  | _ => none

partial def parseArgs (s : List Char) : Option (List Term × List Char) :=
  match s with
  | ']' :: rest => some ([], rest)
  | _ =>
    match parseTerm s with
    | none => none
    | some (t, rest) =>
      match rest with
      | ',' :: more =>
        match parseArgs more with
        | some (ts, rest') => some (t :: ts, rest')
        | none => none
      | ']' :: more => some ([t], more)
      | _ => none
end

def parseLit (s : String) : Option Lit :=
  match s.toList with
  | sign :: 'p' :: rest =>
    if sign != '+' && sign != '-' then none else
    let (ds, rest') := rest.span Char.isDigit
    if ds.isEmpty then none else
      let p := (String.mk ds).toNat!
      let args : Option (List Term) :=
        match rest' with
        | '[' :: inner => (parseArgs inner).map Prod.fst
        | [] => some []
        | _ => none
      args.map (fun ts => ⟨sign == '+', ⟨p, ts⟩⟩)
  | _ => none

def parseClause (s : String) : Option Clause :=
  (splitDrop s ' ').foldl (fun acc tok =>
    match acc, parseLit tok with
    | some ls, some l => some (ls ++ [l])
    | _, _ => none) (some [])

def parseSubst (s : String) : Option (List (Nat × Term)) :=
  (splitDrop s ';').foldl (fun acc tok =>
    match acc, tok.splitOn "=" with
    | some ps, [n, t] =>
      match n.toNat?, parseTerm t.toList with
      | some k, some (tm, []) => some (ps ++ [(k, tm)])
      | _, _ => none
    | _, _ => none) (some [])

structure Parsed where
  clauses : Formula
  lines : Refutation

def parseFile (text : String) : Option Parsed := do
  let mut cs : Formula := []
  let mut ls : Refutation := []
  for line in text.splitOn "\n" do
    let fs := line.splitOn "\t"
    match fs with
    | [] => pure ()
    | kind :: rest =>
      if kind == "" || kind.startsWith "#" then pure ()
      else if kind == "c" then
        match rest with
        | [i, cl] =>
          let n ← i.trim.toNat?
          let c ← parseClause cl
          cs := cs ++ [(n, c)]
        | _ => failure
      else if kind == "r" then
        match rest with
        | [i, concl, ci, lc, sc, rc, dj, ld, sd, rd] =>
          let n ← i.trim.toNat?
          ls := ls ++ [⟨n, ← parseClause concl, .resolve
            (← ci.trim.toNat?) (← parseLit lc.trim) (← parseSubst sc) (← parseClause rc)
            (← dj.trim.toNat?) (← parseLit ld.trim) (← parseSubst sd) (← parseClause rd)⟩]
        | _ => failure
      else if kind == "f" then
        match rest with
        | [i, concl, ci, la, lb, sg, rst] =>
          let n ← i.trim.toNat?
          ls := ls ++ [⟨n, ← parseClause concl, .factor
            (← ci.trim.toNat?) (← parseLit la.trim) (← parseLit lb.trim)
            (← parseSubst sg) (← parseClause rst)⟩]
        | _ => failure
      else failure
  some ⟨cs, ls⟩

def main (args : List String) : IO UInt32 := do
  match args with
  -- FIRST, because `oo-resolution` and friends take a single positional
  -- argument and a later arm would swallow `--version` as a path (#204).
  | ["--version"] => SelfId.emitVersion
  | [path] =>
    let text ← try IO.FS.readFile path catch _ =>
      IO.println s!"\{\"ok\":false,\"error\":\"could not read {path}\"}"
      return 2
    match parseFile text with
    | none =>
      IO.println s!"\{\"ok\":false,\"error\":\"could not parse {path}\"}"
      return 2
    | some p =>
      if check p.clauses p.lines then
        SelfId.println s!"\{\"ok\":true,\"verdict\":\"unsatisfiable\",\
          \"clauses\":{p.clauses.length},\"steps\":{p.lines.length},\
          \"theorem\":\"Fo.unsat_of_check\",\
          \"means\":\"this clause set has no model, over any carrier\"}"
        return 0
      else
        IO.println s!"\{\"ok\":false,\"verdict\":\"not_checked\",\
          \"clauses\":{p.clauses.length},\"steps\":{p.lines.length},\
          \"means\":\"the refutation did not check. This says nothing about \
whether the clause set is satisfiable\"}"
        return 1
  | _ =>
    IO.println "usage: oo-resolution REFUTATION.cert"
    return 2
