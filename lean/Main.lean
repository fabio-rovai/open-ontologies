import OOCert
import SelfId.All

/-!
`oo-cert ASSERTED.tsv DERIVATIONS.tsv`

Exit 0 when every step checks, 1 when one does not, 2 when a file cannot be
read or parsed. The verdict comes from `OOCert.checkCert`, the function
`OOCert.certificate_sound` is proved about. The diagnostic pass that names the
first rejected step is separate and unverified; it runs only after the
verified checker has already said no.
-/
open OOCert

def firstFailure (G : List Triple) (steps : List Step) : Option (Nat × Step) := Id.run do
  let gset := Std.HashSet.ofList G
  let inG := fun t => gset.contains t
  let mut derived : Std.HashSet Triple := ∅
  let mut i := 0
  for st in steps do
    if !checkStep inG (fun t => derived.contains t) st then
      return some (i, st)
    derived := derived.insert st.conclusion
    i := i + 1
  return none

def jsonStr (s : String) : String :=
  let escaped := s.foldl (fun acc c =>
    match c with
    | '"' => acc ++ "\\\""
    | '\\' => acc ++ "\\\\"
    | '\n' => acc ++ "\\n"
    | '\t' => acc ++ "\\t"
    | c => acc.push c) ""
  "\"" ++ escaped ++ "\""

def showTriple (t : Triple) : String := s!"{t.s} {t.p} {t.o}"

/-- Read a file, or report why not. A read failure is exit 2, the code reserved
for "a file could not be read or parsed"; letting the exception escape `main`
exits 1, which is the code that means "the checker rejected a step". A harness
written to the documented contract would then report an unreadable file as a
failed certificate, which is the wrong alarm in the wrong direction. -/
def readOrFail (path : String) : IO (Except UInt32 String) := do
  try
    return .ok (← IO.FS.readFile path)
  catch e =>
    IO.eprintln s!"cannot read {path}: {e}"
    return .error 2

def main (args : List String) : IO UInt32 := do
  match args with
  -- FIRST, because `oo-resolution` and friends take a single positional
  -- argument and a later arm would swallow `--version` as a path (#204).
  | ["--version"] => SelfId.emitVersion
  | [gPath, dPath] =>
    let g ← match ← readOrFail gPath with
      | .ok s => pure s
      | .error c => return c
    let d ← match ← readOrFail dPath with
      | .ok s => pure s
      | .error c => return c
    match Parse.parseTriples g, Parse.parseSteps d with
    | .ok G, .ok steps =>
      if checkCert G steps then
        SelfId.println s!"\{\"ok\":true,\"asserted\":{G.length},\"derivations\":{steps.length},\"theorem\":\"OOCert.certificate_sound\"}"
        return 0
      else
        match firstFailure G steps with
        | some (i, st) =>
          IO.println s!"\{\"ok\":false,\"asserted\":{G.length},\"derivations\":{steps.length},\"first_rejected\":{i},\"rule\":{jsonStr st.rule.name},\"conclusion\":{jsonStr (showTriple st.conclusion)},\"premises\":[{String.intercalate "," (st.premises.map (fun t => jsonStr (showTriple t)))}]}"
        | none =>
          IO.println "{\"ok\":false,\"error\":\"the verified checker rejected the certificate and the diagnostic pass could not name a step\"}"
        return 1
    | .error e, _ =>
      IO.eprintln e
      return 2
    | _, .error e =>
      IO.eprintln e
      return 2
  | _ =>
    IO.eprintln "usage: oo-cert ASSERTED.tsv DERIVATIONS.tsv"
    return 2
