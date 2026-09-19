import Dl.All

/-!
`oo-dlmodel AXIOMS.tsv MODEL.tsv`

Reads an axiom set and a finite interpretation, runs the verified checker, and prints one
JSON object.

Exit codes, matching the convention `oo-cert` uses:

* `0` the interpretation is a model of the axiom set. The axiom set is satisfiable, and the
  statement covering that is `Dl.satisfiable_of_checkModel`.
* `1` the interpretation is NOT a model. By `Dl.checkModel_complete` this is a statement
  about the interpretation, not about the checker giving up: the checker accepts every
  model of the axiom set.
* `2` a file could not be read or parsed. No verdict either way.

What exit 0 does NOT say, and what a reader must not infer from it:

* it says nothing about UNSATISFIABILITY. That answer has its own certificate and its own
  checker, `oo-dlrefute`, and the two must never be read for one another. This header used
  to say certifying it "needs a closed tableau with its blocking argument". Half of that
  was right. A closed tableau is what it needs. Blocking is a COMPLETENESS device, there so
  that a SEARCH for a model terminates, and a refutation never needs one: a closed tableau
  is already a finite tree whose every branch ended in a clash. `lean/Dl/Tableau.lean` is
  the proof and it carries no blocking argument anywhere;
* it says nothing about whether `AXIOMS.tsv` is the ontology. The OWL parser in
  `src/tableaux.rs` is outside the theorem, exactly as the N-Triples parser is outside
  `Shacl.validate_spec`. What is certified is a relation between the two files;
* it says nothing about the fragment the emitter refused to write. `src/tableaux.rs` emits
  nothing rather than something unchecked when the completion graph cannot be turned into a
  finite interpretation, and "no certificate" is not a verdict.

The diagnostic pass that names the first failing axiom is separate and unverified. It runs
only after the verified checker has already said no.
-/
open Dl

def jsonStr (s : String) : String :=
  let escaped := s.foldl (fun acc c =>
    match c with
    | '"' => acc ++ "\\\""
    | '\\' => acc ++ "\\\\"
    | '\n' => acc ++ "\\n"
    | '\t' => acc ++ "\\t"
    | c => acc.push c) ""
  "\"" ++ escaped ++ "\""

/-- A short name for an axiom, for the diagnostic line only. -/
def axiomKind : Axiom → String
  | .sub .. => "sub"
  | .disjoint .. => "disjoint"
  | .dom .. => "domain"
  | .rng .. => "range"
  | .subrole .. => "subrole"
  | .trans .. => "trans"
  | .sym .. => "sym"
  | .inv .. => "inv"
  | .invfunc .. => "invfunc"
  | .inst .. => "inst"
  | .rel .. => "rel"
  | .indiv .. => "indiv"
  | .nonempty .. => "nonempty"

def showConcept : Concept → String
  | .top => "top"
  | .bot => "bot"
  | .atom a => "atom " ++ a
  | .neg c => "not " ++ showConcept c
  | .and c d => "and " ++ showConcept c ++ " " ++ showConcept d
  | .or c d => "or " ++ showConcept c ++ " " ++ showConcept d
  | .ex r c => "some " ++ r ++ " " ++ showConcept c
  | .all r c => "all " ++ r ++ " " ++ showConcept c
  | .min n r c => "min " ++ toString n ++ " " ++ r ++ " " ++ showConcept c
  | .max n r c => "max " ++ toString n ++ " " ++ r ++ " " ++ showConcept c

def showAxiom : Axiom → String
  | .sub c d => "sub\t" ++ showConcept c ++ "\t" ++ showConcept d
  | .disjoint c d => "disjoint\t" ++ showConcept c ++ "\t" ++ showConcept d
  | .dom r c => "domain\t" ++ r ++ "\t" ++ showConcept c
  | .rng r c => "range\t" ++ r ++ "\t" ++ showConcept c
  | .subrole r s => "subrole\t" ++ r ++ "\t" ++ s
  | .trans r => "trans\t" ++ r
  | .sym r => "sym\t" ++ r
  | .inv r s => "inv\t" ++ r ++ "\t" ++ s
  | .invfunc r => "invfunc\t" ++ r
  | .inst a c => "inst\t" ++ a ++ "\t" ++ showConcept c
  | .rel a r b => "rel\t" ++ a ++ "\t" ++ r ++ "\t" ++ b
  | .indiv a => "indiv\t" ++ a
  | .nonempty c => "nonempty\t" ++ showConcept c

/-- Which well-formedness clause failed, if one did. -/
def wfFailure (I : Interp) (A : List Axiom) : Option String :=
  if I.dom.isEmpty then
    some "the domain is empty"
  else if !((atomNames A).all (fun a => (I.cext a).all (fun x => I.dom.contains x))) then
    some "an atomic concept has a member outside the domain"
  else if !((roleNames A).all (fun r =>
      I.dom.all (fun x => (I.rext r x).all (fun y => I.dom.contains y)))) then
    some "a role edge leaves the domain"
  else if !((indNames A).all (fun a => I.dom.contains (I.ind a))) then
    some "a named individual denotes something outside the domain"
  else
    none

def firstFailure (I : Interp) (A : List Axiom) : Option (Nat × Axiom) := Id.run do
  let mut i := 0
  for a in A do
    if !holds I a then
      return some (i, a)
    i := i + 1
  return none

def readOrFail (path : String) : IO (Except UInt32 String) := do
  try
    return .ok (← IO.FS.readFile path)
  catch e =>
    IO.eprintln s!"cannot read {path}: {e}"
    return .error 2

def run (axPath mPath : String) : IO UInt32 := do
  let axTxt ← match ← readOrFail axPath with
    | .ok s => pure s
    | .error c => return c
  let mTxt ← match ← readOrFail mPath with
    | .ok s => pure s
    | .error c => return c
  match Parse.parseAxioms axTxt, Parse.parseModel mTxt with
  | .ok A, .ok I =>
    if checkModel I A then
      IO.println ("{\"ok\":true,\"axioms\":" ++ toString A.length ++
        ",\"domain\":" ++ toString I.dom.length ++
        ",\"theorem\":\"Dl.satisfiable_of_checkModel\"}")
      return 0
    else
      match wfFailure I A with
      | some reason =>
        IO.println ("{\"ok\":false,\"axioms\":" ++ toString A.length ++
          ",\"domain\":" ++ toString I.dom.length ++
          ",\"wellformed\":false,\"reason\":" ++ jsonStr reason ++ "}")
      | none =>
        match firstFailure I A with
        | some (i, a) =>
          IO.println ("{\"ok\":false,\"axioms\":" ++ toString A.length ++
            ",\"domain\":" ++ toString I.dom.length ++
            ",\"wellformed\":true,\"first_rejected\":" ++ toString i ++
            ",\"kind\":" ++ jsonStr (axiomKind a) ++
            ",\"axiom\":" ++ jsonStr (showAxiom a) ++ "}")
        | none =>
          IO.println ("{\"ok\":false,\"error\":\"the verified checker rejected the model " ++
            "and the diagnostic pass could not name an axiom\"}")
      return 1
  | .error e, _ =>
    IO.eprintln e
    return 2
  | _, .error e =>
    IO.eprintln e
    return 2

def main (args : List String) : IO UInt32 := do
  match args with
  | [a, m] => run a m
  | _ =>
    IO.eprintln "usage: oo-dlmodel AXIOMS.tsv MODEL.tsv"
    return 2
