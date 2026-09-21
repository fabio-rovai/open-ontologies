import Shacl.All
import SelfId.All

/-!
`oo-shacl validate DATA.nt SHAPES.nt`

Reads two N-Triples files, compiles the shapes graph, runs the verified evaluator,
and prints one JSON object.

Exit codes, chosen so that a harness cannot mistake one answer for another:

* `0` a verdict was reached. `"conforms"` is `true` or `false`.
* `2` a file could not be read or parsed. No verdict.
* `3` UNDETERMINED. The shapes graph uses something this development does not
  implement, or the evaluator declined to judge a literal, to order two terms, or to
  count the characters of an escaped spelling. No verdict, and the reason is printed.
  This is the code that matters: it is the difference between "everything conforms"
  and "I did not check everything", and collapsing the two is the failure this whole
  layer exists to avoid.

The verdict and the results carry `"theorem": "Shacl.validate_spec"`, which is the
machine-checked statement they are covered by: the report is empty exactly when
every targeted node conforms to its shape under `Shacl/Spec.lean`, and every result
blames a node that really fails the constraint it is blamed for.

What that theorem does NOT cover, and what a reader must therefore not assume:

* that `Shacl/Spec.lean` is the W3C Recommendation. It is a reading of it, measured
  against the Working Group's suite by `tests/shacl_core_verified_test.rs`;
* that the compiler in `Shacl/Compile.lean` reads the shapes graph correctly. It is
  outside the theorem, and it refuses rather than guesses;
* that the N-Triples parser is correct. Also outside, and a parse error is exit 2;
* `sh:resultSeverity`. Never emitted; `sh:severity` in the shapes graph is ignored.
  It is no longer ignored in SILENCE: the verdict carries an `ignored` list naming
  every `(subject, predicate)` pair whose predicate this compiler waves through, so
  a reader learns from the output that a severity or a message was present and had
  no effect. The list says nothing about what those predicates MEAN; the theorem is
  about conformance and is silent on both.
-/
open Shacl

def jsonStr (s : String) : String :=
  let escaped := s.foldl (fun acc c =>
    match c with
    | '"' => acc ++ "\\\""
    | '\\' => acc ++ "\\\\"
    | '\n' => acc ++ "\\n"
    | '\t' => acc ++ "\\t"
    | c => acc.push c) ""
  "\"" ++ escaped ++ "\""

def showPath : Path → String
  | .pred p => p
  | .inv p => "^" ++ p
  | .seq a b => "(" ++ showPath a ++ "/" ++ showPath b ++ ")"
  | .alt a b => "(" ++ showPath a ++ "|" ++ showPath b ++ ")"
  | .zeroOrOne a => "(" ++ showPath a ++ ")?"

def jsonOpt (o : Option String) : String :=
  match o with
  | none => "null"
  | some s => jsonStr s

def showResult (r : Result) : String :=
  "{\"focus\":" ++ jsonStr r.focus ++
  ",\"path\":" ++ jsonOpt (r.path.map showPath) ++
  ",\"value\":" ++ jsonOpt r.value ++
  ",\"sourceShape\":" ++ jsonStr r.source ++
  ",\"sourceConstraintComponent\":" ++ jsonStr r.component ++ "}"

def readOrFail (path : String) : IO (Except String String) := do
  try
    return .ok (← IO.FS.readFile path)
  catch e =>
    return .error s!"cannot read {path}: {e}"

/-- One `(shape, predicate)` pair the compiler ignored. -/
def showIgnored (pair : Shacl.Term × Shacl.Term) : String :=
  "{\"subject\":" ++ jsonStr pair.1 ++ ",\"predicate\":" ++ jsonStr pair.2 ++ "}"

/-- What the `ignored` list means, carried beside it so a consumer reading the
JSON alone cannot mistake it for a list of violations. -/
def ignoredMeans : String :=
  "these SHACL predicates are present in the shapes graph and had NO effect on the verdict \
   or on any field of any result. sh:severity and sh:message are the two that carry meaning \
   to a reader: a shapes graph marking a constraint sh:Warning produces the same verdict \
   here as one marking it sh:Violation, and sh:resultSeverity is never emitted. \
   Shacl.validate_spec is about conformance and says NOTHING about either of them. Listed \
   rather than dropped in silence, because the rest of this compiler refuses what it cannot \
   do and this was the one place it discarded instead."

def emitError (reason : String) : IO UInt32 := do
  IO.println ("{\"status\":\"error\",\"reason\":" ++ jsonStr reason ++ "}")
  return 2

def emitUndetermined (reason : String) : IO UInt32 := do
  IO.println ("{\"status\":\"undetermined\",\"reason\":" ++ jsonStr reason ++ "}")
  return 3

def run (dataPath shapesPath : String) : IO UInt32 := do
  let dataTxt ← match ← readOrFail dataPath with
    | .ok s => pure s
    | .error e => return ← emitError e
  let shapesTxt ← match ← readOrFail shapesPath with
    | .ok s => pure s
    | .error e => return ← emitError e
  let dataG ← match Parse.parseNTriples dataTxt with
    | .ok g => pure g
    | .error e => return ← emitError s!"data graph: {e}"
  let shapesG ← match Parse.parseNTriples shapesTxt with
    | .ok g => pure g
    | .error e => return ← emitError s!"shapes graph: {e}"
  let decls ← match Compile.compileShapes shapesG with
    | .ok d => pure d
    | .error e => return ← emitUndetermined s!"shapes graph not compiled: {e}"
  match validate dataG decls with
  | .error refusal =>
      emitUndetermined s!"the evaluator declined: {refusal.describe}"
  | .ok results =>
      let conforms := results.isEmpty
      let ignored := Compile.ignoredPairs shapesG
      SelfId.println ("{\"status\":\"verdict\",\"conforms\":" ++ (if conforms then "true" else "false") ++
        ",\"shapes\":" ++ toString decls.length ++
        ",\"data_triples\":" ++ toString dataG.length ++
        ",\"results\":[" ++ String.intercalate "," (results.map showResult) ++ "]" ++
        ",\"ignored\":[" ++ String.intercalate "," (ignored.map showIgnored) ++ "]" ++
        ",\"ignored_means\":" ++ jsonStr ignoredMeans ++
        ",\"theorem\":\"Shacl.validate_spec\"}")
      return 0

def main (args : List String) : IO UInt32 := do
  match args with
  -- FIRST, because `oo-resolution` and friends take a single positional
  -- argument and a later arm would swallow `--version` as a path (#204).
  | ["--version"] => SelfId.emitVersion
  | ["validate", d, s] => run d s
  | _ =>
      IO.eprintln "usage: oo-shacl validate DATA.nt SHAPES.nt"
      return 2
