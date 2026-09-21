import OOCert
import OOCert.HornParse
import SelfId.All

/-!
`oo-horn rules`                              prints the built-in rules as data
`oo-horn check RULES ASSERTED HORN`          checks a Horn certificate

Exit 0 when every step checks, 1 when one does not, 2 on a read or parse error.

# The two verdicts, and why they must never share a word

A certificate over the BUILT-IN rules and a certificate over rules a user wrote
are checked by the same function and prove different things.

The built-ins are discharged against the semantics by `Builtin.asHorn_sound`, so
a certificate citing only them yields `OOCert.Entails`: true in every model of
the asserted graph. That is the same warrant `certificate_sound` gives.

A rule the user supplied is discharged by nobody. It is an ASSUMPTION the
certificate carries, so the result is `OOCert.EntailsR`: true in every model of
the asserted graph THAT ALSO SATISFIES THOSE RULES. A rule reading "every
supplier is compliant" produces certificates that check green for ever. The
certificate certifies the inference and never the premises.

Collapsing those two into one word is how a verified checker becomes a device
for laundering an assumption into a fact, so they are printed as different
verdicts (`entailed` against `entailed_under_supplied_rules`), they name
different theorems, and the relativised one carries a digest of the rule table
that was in force. `tests/lean_horn_certificate_test.rs` fails if a run over
user rules ever reports the absolute verdict.

The digest identifies a rule table; it is not a cryptographic commitment, and
`String.hash` is not a cryptographic hash. It exists so that two reports can be
compared, not so that one can be defended against an adversary who controls the
rule file. Anyone who needs that should hash the file itself.
-/
open OOCert

def readOrFail (path : String) : IO (Except UInt32 String) := do
  try
    return .ok (← IO.FS.readFile path)
  catch e =>
    IO.eprintln s!"cannot read {path}: {e}"
    return .error 2

/-- The canonical rendering of a rule table, used both to compare a supplied
table against the built-ins and to digest it. -/
def tableLines (R : List RulePattern) : List String := R.map HornParse.ruleStr

def tableDigest (R : List RulePattern) : UInt64 :=
  String.hash (String.intercalate "\n" (tableLines R))

def jsonStr (s : String) : String :=
  let escaped := s.foldl (fun acc c =>
    match c with
    | '"' => acc ++ "\\\""
    | '\\' => acc ++ "\\\\"
    | '\n' => acc ++ "\\n"
    | c => acc.push c) ""
  "\"" ++ escaped ++ "\""

def main (args : List String) : IO UInt32 := do
  match args with
  -- FIRST, because `oo-resolution` and friends take a single positional
  -- argument and a later arm would swallow `--version` as a path (#204).
  | ["--version"] => SelfId.emitVersion
  | ["rules"] =>
    for r in Builtin.asHorn do
      IO.println (HornParse.ruleStr r)
    return 0
  | ["check", rPath, gPath, dPath] =>
    let rTxt ← match ← readOrFail rPath with | .ok s => pure s | .error c => return c
    let gTxt ← match ← readOrFail gPath with | .ok s => pure s | .error c => return c
    let dTxt ← match ← readOrFail dPath with | .ok s => pure s | .error c => return c
    match HornParse.parseRules rTxt, Parse.parseTriples gTxt, HornParse.parseHornSteps dTxt with
    | .ok R, .ok G, .ok steps =>
      if checkHornCert G R steps then
        -- Which warrant did this run earn? Exactly the built-in table earns the
        -- absolute one, because that is the table `Builtin.asHorn_sound`
        -- discharges. Anything else, including the built-ins plus one extra
        -- rule, earns the relativised one.
        let builtin := tableLines R == tableLines Builtin.asHorn
        let verdict := if builtin then "entailed" else "entailed_under_supplied_rules"
        let thm :=
          if builtin then "OOCert.entails_of_builtin_horn" else "OOCert.horn_certificate_sound"
        let means :=
          if builtin then
            "every conclusion is true in every model of the asserted graph"
          else
            "every conclusion is true in every model of the asserted graph THAT ALSO SATISFIES \
             the supplied rules; the rules themselves are assumed, not checked"
        SelfId.println s!"\{\"ok\":true,\"verdict\":{jsonStr verdict},\"rules\":{R.length},\
          \"asserted\":{G.length},\"derivations\":{steps.length},\
          \"rules_digest\":\"{tableDigest R}\",\"builtin_rules_digest\":\"{tableDigest Builtin.asHorn}\",\
          \"theorem\":{jsonStr thm},\"means\":{jsonStr means}}"
        return 0
      else
        IO.println s!"\{\"ok\":false,\"rules\":{R.length},\"asserted\":{G.length},\
          \"derivations\":{steps.length},\"rules_digest\":\"{tableDigest R}\"}"
        return 1
    | .error e, _, _ => IO.eprintln e; return 2
    | _, .error e, _ => IO.eprintln e; return 2
    | _, _, .error e => IO.eprintln e; return 2
  | _ =>
    IO.eprintln "usage: oo-horn rules | oo-horn check RULES.tsv ASSERTED.tsv HORN.tsv"
    return 2
