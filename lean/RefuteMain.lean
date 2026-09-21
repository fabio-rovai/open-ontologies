import OOCert.Parse
import OOCert.Refute
import OOCert.RefuteWitness
import SelfId.All

/-!
`oo-refute check ASSERTED.tsv REFUTATION.tsv`
`oo-refute guard ASSERTED.tsv DERIVATIONS.tsv REFUTATION.tsv`

Exit 0 when the refutation checks, 1 when it does not, 2 when a file cannot be
read or parsed. The house convention is about the CERTIFICATE, not about the
ontology: exit 0 from `check` means "this refutation is valid", which is to say
the ontology is contradictory. It is a good exit code reporting bad news.

# Why `guard` exists

`oo-cert` reports `"ok":true` for a certificate over a self-contradicting
ontology, and what it says is true. Its verdict is `OOCert.Entails`, which
quantifies over `OOCert.Model I G`, and that class of interpretations is never
empty: `OOCert.saturated_is_a_model` exhibits a member for every graph, because
`Conditions` carries no negative condition and therefore cannot notice a
disjointness clash.

The verdict a consumer wants is the one that reads `owl:disjointWith` as
disjointness, and once the graph is refutable that one is empty.
`OOCert.a_certificate_adds_nothing_when_the_graph_is_refuted` states it: over a
refuted graph, every conclusion of every list of steps already carries the
disjointness-aware warrant, checked or forged, so running the derivation checker
establishes nothing.

`guard` therefore runs both and refuses the derivation certificate when the
refutation succeeds. It exits 0 only when the certificate checks AND the
refutation does not.

# What a rejected refutation does NOT mean

It does not mean the graph is consistent. Seventeen OWL 2 RL rules conclude
`false`; `RefuteConditions` carries one condition, for `cax-dw`, and the other
sixteen have no condition here at all. Fifteen of those sixteen are expressible
in this layer and simply absent. The sixteenth, `dt-not-type`, is not
expressible at all, because `OOCert.Semantics` has no datatype value space.
`Refute.lean` lists all seventeen with their tables and states that split at the
count. A graph that only `prp-irp` or `eq-diff1` could refute is rejected by
this checker and is still contradictory. The checker is a gate, not an oracle,
in that direction as in the other.
-/
open OOCert

def readOrFail (path : String) : IO (Except UInt32 String) := do
  try
    return .ok (← IO.FS.readFile path)
  catch e =>
    IO.eprintln s!"cannot read {path}: {e}"
    return .error 2

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

/-- The diagnostic pass. Unverified, and it runs only after the verified checker
has already said no, exactly as `oo-cert`'s does. -/
def whyRejected (G : List Triple) (r : Refutation) : String := Id.run do
  let gset := Std.HashSet.ofList G
  let inG := fun t => gset.contains t
  let mut derived : Std.HashSet Triple := ∅
  let mut i := 0
  for st in r.steps do
    if !checkStep inG (fun t => derived.contains t) st then
      return s!"derivation step {i} ({st.rule.name}) does not check: it claims \
        {showTriple st.conclusion}"
    derived := derived.insert st.conclusion
    i := i + 1
  let known := fun t => inG t || derived.contains t
  for t in r.final.premises do
    if !known t then
      return s!"the contradiction step cites {showTriple t}, which is neither asserted nor \
        concluded by a step before it"
  return s!"the contradiction step does not have the shape {r.final.rule.name} requires; \
    premise order is part of the contract"

def meansRefuted : String :=
  "no interpretation satisfies both the RDF-based conditions on the asserted graph and the \
   reading of owl:disjointWith as disjointness. This is unsatisfiability RELATIVE TO that \
   reading: drop it and the graph has models again. Every derivation certificate over this \
   graph is now worthless, because under the same reading every triple is entailed."

def runCheck (gPath rPath : String) : IO UInt32 := do
  let gTxt ← match ← readOrFail gPath with | .ok s => pure s | .error c => return c
  let rTxt ← match ← readOrFail rPath with | .ok s => pure s | .error c => return c
  match Parse.parseTriples gTxt, RefuteParse.parseRefutation rTxt with
  | .ok G, .ok r =>
    if checkRefutationFast G r then
      SelfId.println s!"\{\"ok\":true,\"format\":\"oo-refute/1\",\"asserted\":{G.length},\
        \"prefix\":{r.steps.length},\"rule\":{jsonStr r.final.rule.name},\
        \"verdict\":\"unsatisfiable_under_disjointness\",\
        \"theorem\":\"OOCert.refutation_fast_sound\",\"means\":{jsonStr meansRefuted}}"
      return 0
    else
      IO.println s!"\{\"ok\":false,\"format\":\"oo-refute/1\",\"asserted\":{G.length},\
        \"prefix\":{r.steps.length},\"rule\":{jsonStr r.final.rule.name},\
        \"reason\":{jsonStr (whyRejected G r)},\
        \"means\":\"the refutation was not accepted. That is not a proof of consistency: \
        seventeen OWL 2 RL rules conclude false, this checker implements cax-dw, and the other \
        sixteen have no condition in it.\"}"
      return 1
  | .error e, _ => IO.eprintln e; return 2
  | _, .error e => IO.eprintln e; return 2

def runGuard (gPath dPath rPath : String) : IO UInt32 := do
  let gTxt ← match ← readOrFail gPath with | .ok s => pure s | .error c => return c
  let dTxt ← match ← readOrFail dPath with | .ok s => pure s | .error c => return c
  let rTxt ← match ← readOrFail rPath with | .ok s => pure s | .error c => return c
  match Parse.parseTriples gTxt, Parse.parseSteps dTxt, RefuteParse.parseRefutation rTxt with
  | .ok G, .ok steps, .ok r =>
    let certOk := checkCert G steps
    let refuted := checkRefutationFast G r
    if refuted then
      SelfId.println s!"\{\"ok\":false,\"certificate_ok\":{certOk},\"refuted\":true,\
        \"asserted\":{G.length},\"derivations\":{steps.length},\
        \"verdict\":\"certificate_refused_graph_is_unsatisfiable\",\
        \"theorem\":\"OOCert.a_certificate_adds_nothing_when_the_graph_is_refuted\",\
        \"means\":\"the graph is refutable, so under the disjointness reading EVERY triple is \
        entailed and this certificate carries no information. oo-cert would still report ok:true \
        for it, and that verdict quantifies over a model class that ignores disjointness.\"}"
      return 1
    else if certOk then
      SelfId.println s!"\{\"ok\":true,\"certificate_ok\":true,\"refuted\":false,\
        \"asserted\":{G.length},\"derivations\":{steps.length},\
        \"verdict\":\"entailed_and_no_disjointness_clash_found\",\
        \"theorem\":\"OOCert.certificate_sound\",\
        \"means\":\"every conclusion is entailed, and the supplied refutation did NOT check. \
        No clash was found; that is not a proof of consistency, because only cax-dw is \
        implemented.\"}"
      return 0
    else
      IO.println s!"\{\"ok\":false,\"certificate_ok\":false,\"refuted\":false,\
        \"asserted\":{G.length},\"derivations\":{steps.length},\
        \"verdict\":\"certificate_rejected\",\
        \"means\":\"the derivation certificate itself did not check; run oo-cert for the step \
        that failed.\"}"
      return 1
  | .error e, _, _ => IO.eprintln e; return 2
  | _, .error e, _ => IO.eprintln e; return 2
  | _, _, .error e => IO.eprintln e; return 2

def main (args : List String) : IO UInt32 := do
  match args with
  -- FIRST, because `oo-resolution` and friends take a single positional
  -- argument and a later arm would swallow `--version` as a path (#204).
  | ["--version"] => SelfId.emitVersion
  | ["check", gPath, rPath] => runCheck gPath rPath
  | ["guard", gPath, dPath, rPath] => runGuard gPath dPath rPath
  | _ =>
    IO.eprintln "usage: oo-refute check ASSERTED.tsv REFUTATION.tsv"
    IO.eprintln "       oo-refute guard ASSERTED.tsv DERIVATIONS.tsv REFUTATION.tsv"
    return 2
