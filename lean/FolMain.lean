import Fol.All
import SelfId.All

/-!
`oo-folmodel PROBLEM.tsv MODEL.tsv`

Reads a first-order problem and a finite structure, runs the verified checker, and prints one JSON
object. The formats are documented in `lean/Fol/Syntax.lean`.

Exit codes, matching the convention `oo-cert` and `oo-dlmodel` use:

* `0` the structure satisfies every formula of the problem. The problem is satisfiable, and the
  statement covering that is `Fol.satisfiable_of_check`. When the problem carries a `goal_negated`
  line the same run also covers `Fol.not_entails_of_check`, and the report says which.
* `1` the certificate is rejected. Three reasons, and they are not the same sentence:
  `problem_digest_mismatch` means the structure was built for a different problem;
  `undeclared_symbol` means the model file does not interpret something the problem uses, which is
  an ATTRIBUTION failure and not a soundness one; otherwise the verified checker rejected the
  structure, and by `Fol.check_complete` that is a statement about the structure rather than about
  the checker giving up.
* `2` a file could not be read or parsed, or the problem carried no formulas. No verdict either
  way.

What exit 0 does NOT say, and what a reader must not infer from it:

* it says nothing about UNSATISFIABILITY, in any direction. When a solver answers `unsat` no
  certificate exists and none is checked here. That answer is still "trust the solver". Certifying
  it needs a verified first-order calculus with unification, which does not exist in core Lean,
  and decision 0005 rules it an oracle opinion. This layer does not disturb that ruling; it
  observes that the other direction is not symmetric;
* it says nothing about a BOUNDED search that found nothing. A solver asked for a model of size at
  most `k` and answering `unsat` has established `no_model_up_to_size_k`, which is not
  unsatisfiability and must never be reported with that word. Nothing here produces either verdict;
* it says nothing about whether `PROBLEM.tsv` is the ontology. The OWL reader and the translation
  in `src/tptp.rs` are outside the theorem, exactly as the N-Triples parser is outside
  `Shacl.validate_spec`. What is certified is a relation between the two files. The OWL-level
  reading rides additionally on `OwlLean.adequacy` and on the Rust-to-Lean correspondence, which
  decision 0005 item 2 states is pinned by tests and NOT proved, and it carries its own word;
* the absence of a finite model implies NOTHING. SHIQ lacks the finite model property, so a
  satisfiable ontology can have only infinite models and will never receive a certificate here.
  That is a limitation of the method rather than a defect, and it is the honest counterweight to
  the asymmetry this layer is built on;
* `source` and `cardinality_search` are copied out of the model file and are UNTRUSTED. They are
  what the file claims about its own provenance, echoed for a report, and nothing checks them.

The diagnostic pass that names the first failing formula, and the one that names the first
undeclared symbol, are separate and unverified. They run only after the verified checker or the
verified coverage gate has already said no.
-/
open Fol
open Fol.Parse

def jsonStr (s : String) : String :=
  let escaped := s.foldl (fun acc c =>
    match c with
    | '"' => acc ++ "\\\""
    | '\\' => acc ++ "\\\\"
    | '\n' => acc ++ "\\n"
    | '\t' => acc ++ "\\t"
    | c => acc.push c) ""
  "\"" ++ escaped ++ "\""

def jsonBool (b : Bool) : String := if b then "true" else "false"

/-- Which symbol the model file failed to declare. Unverified, and it runs only after `covers` has
already returned false. -/
def firstUndeclared (d1 d2 dc : List Sym) (Γ : List Form) : Option (String × String) :=
  match (Γ.flatMap p1Syms).find? (fun p => !d1.contains p) with
  | some p => some ("unary", p)
  | none =>
    match (Γ.flatMap p2Syms).find? (fun p => !d2.contains p) with
    | some p => some ("binary", p)
    | none =>
      match (Γ.flatMap constSyms).find? (fun c => !dc.contains c) with
      | some c => some ("constant", c)
      | none => none

/-- Which formula the structure failed. Unverified, and it runs only after `check` has already
returned false. -/
def firstFailure {n : Nat} (M : FinModel (n+1)) (es : List Entry) : Option (Nat × Entry) := Id.run do
  let mut i := 0
  for e in es do
    if !eval M (fun _ => 0) e.form then
      return some (i, e)
    i := i + 1
  return none

def readOrFail (path : String) : IO (Except UInt32 String) := do
  try
    return .ok (← IO.FS.readFile path)
  catch e =>
    IO.eprintln s!"cannot read {path}: {e}"
    return .error 2

def unreadable (reason : String) : IO UInt32 := do
  IO.println ("{\"verdict\":\"unreadable\",\"reason\":" ++ jsonStr reason ++ "}")
  IO.eprintln reason
  return 2

def run (pPath mPath : String) : IO UInt32 := do
  let pTxt ← match ← readOrFail pPath with
    | .ok s => pure s
    | .error _ => return ← unreadable s!"cannot read {pPath}"
  let mTxt ← match ← readOrFail mPath with
    | .ok s => pure s
    | .error _ => return ← unreadable s!"cannot read {mPath}"
  match parseProblem pTxt with
  | .error e => unreadable e
  | .ok entries =>
    match parseModel mTxt with
    | .error e => unreadable e
    | .ok pm =>
      let forms := entries.map (fun e => e.form)
      let nf := entries.length
      let card := pm.n + 1
      let closed := forms.all (fun g => (free g).isEmpty)
      let goal := entries.any (fun e => e.role == "goal_negated")
      let common :=
        "\"formulas\":" ++ toString nf ++
        ",\"domain\":" ++ toString card ++
        ",\"closed\":" ++ jsonBool closed ++
        ",\"goal_negated_present\":" ++ jsonBool goal ++
        ",\"source\":" ++ jsonStr pm.source ++
        ",\"cardinality_search\":" ++ jsonStr pm.search
      let expected := problemDigest entries
      if pm.digest != expected then
        IO.println ("{\"verdict\":\"rejected\",\"reason\":\"problem_digest_mismatch\"," ++
          common ++ ",\"expected\":" ++ jsonStr expected ++
          ",\"found\":" ++ jsonStr pm.digest ++ "}")
        return 1
      if !covers pm.decl1 pm.decl2 pm.declc forms then
        let (kind, sym) := (firstUndeclared pm.decl1 pm.decl2 pm.declc forms).getD ("unknown", "")
        IO.println ("{\"verdict\":\"rejected\",\"reason\":\"undeclared_symbol\"," ++
          "\"gate\":\"attribution\"," ++ common ++
          ",\"symbol\":" ++ jsonStr sym ++ ",\"arity\":" ++ jsonStr kind ++ "}")
        return 1
      if check pm.model forms then
        SelfId.println ("{\"verdict\":\"model_checked\"," ++ common ++
          ",\"problem_digest\":" ++ jsonStr expected ++
          ",\"theorem\":\"Fol.satisfiable_of_check\"" ++
          (if goal then ",\"non_entailment_theorem\":\"Fol.not_entails_of_check\"" else "") ++
          "}")
        return 0
      else
        let thm := if closed then "Fol.check_complete_closed" else "Fol.check_complete"
        match firstFailure pm.model entries with
        | some (i, e) =>
          SelfId.println ("{\"verdict\":\"rejected\"," ++ common ++
            ",\"theorem\":" ++ jsonStr thm ++
            ",\"first_rejected\":" ++ toString i ++
            ",\"label\":" ++ jsonStr e.label ++
            ",\"role\":" ++ jsonStr e.role ++
            ",\"formula\":" ++ jsonStr (showForm e.form) ++ "}")
        | none =>
          SelfId.println ("{\"verdict\":\"rejected\"," ++ common ++
            ",\"theorem\":" ++ jsonStr thm ++
            ",\"error\":\"the verified checker rejected the structure and the diagnostic pass " ++
            "could not name a formula\"}")
        return 1

def main (args : List String) : IO UInt32 := do
  match args with
  -- FIRST, because `oo-resolution` and friends take a single positional
  -- argument and a later arm would swallow `--version` as a path (#204).
  | ["--version"] => SelfId.emitVersion
  | [p, m] => run p m
  | _ =>
    IO.eprintln "usage: oo-folmodel PROBLEM.tsv MODEL.tsv"
    return 2
