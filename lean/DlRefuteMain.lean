import Dl.All

/-!
`oo-dlrefute AXIOMS.tsv REFUTATION.cert`

Reads an axiom set and a closed tableau, replays the tableau, and prints one
JSON object.

Exit codes, matching the convention `oo-cert` and `oo-dlmodel` use:

* `0` the tableau is closed, so the axiom set has NO MODEL, of any size, finite
  or infinite. The statement covering that is `Dl.unsatisfiable_of_check`, and
  `Dl.no_model_of_check` is the same verdict in the finite form the rest of the
  repository states its answers in.
* `1` the tableau is not closed. That is a statement about the certificate and
  NOT about the ontology: a producer can fail to find a refutation that exists,
  and this exit code never means the ontology is consistent.
* `2` a file could not be read or parsed. No verdict either way.

This is the other half of `oo-dlmodel`. That one certifies SATISFIABILITY by
exhibiting a model; this one certifies UNSATISFIABILITY by exhibiting a
refutation, and the two are asymmetric on purpose. A model is a finite object
that can be checked by evaluation. A refutation cannot be, because the thing it
denies is a claim about every interpretation including the infinite ones, which
is why `Dl/General.lean` exists at all: the finite semantics literally cannot
state the conclusion.

What exit 0 does NOT say:

* it says nothing about whether `AXIOMS.tsv` is your ontology. The OWL parser in
  `src/tableaux.rs` is outside the theorem, exactly as the N-Triples parser is
  outside `Shacl.validate_spec`. What is certified is a relation between two
  files;
* it says nothing about transitive, symmetric, inverse or inverse-functional
  roles. The calculus has no rules for them, so an ontology inconsistent only
  through those cannot be refuted here. The emitter declines rather than
  guessing, and "no certificate" is not a verdict.

The diagnostic that names the failing step is unverified and runs only after the
verified checker has already said no. It restates the side conditions a second
time, so it can be wrong about WHICH step failed; it cannot be wrong about THAT
one did, because the verified checker decided that.
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

def stepKind : Cert → String
  | .botC .. => "bot-clash"
  | .negC .. => "negation-clash"
  | .diffC .. => "inequality-clash"
  | .disjC .. => "disjointness-clash"
  | .minmaxC .. => "bound-clash"
  | .maxC .. => "cardinality-clash"
  | .instS .. => "assertion"
  | .relS .. => "role-assertion"
  | .subS .. => "subsumption"
  | .domS .. => "domain"
  | .rngS .. => "range"
  | .subroleS .. => "role-hierarchy"
  | .nonemptyS .. => "non-emptiness"
  | .andS .. => "conjunction"
  | .allS .. => "universal"
  | .exS .. => "existential"
  | .minS .. => "at-least"
  | .orS .. => "disjunction"

def steps : Cert → Nat
  | .botC .. | .negC .. | .diffC .. | .disjC .. | .minmaxC .. | .maxC .. => 1
  | .instS _ _ k | .relS _ _ _ k | .subS _ _ _ k | .domS _ _ _ _ k
  | .rngS _ _ _ _ k | .subroleS _ _ _ _ k | .nonemptyS _ _ k | .andS _ _ _ k
  | .allS _ _ _ _ k | .exS _ _ _ _ k | .minS _ _ _ _ _ k => 1 + steps k
  | .orS _ _ _ l r => 1 + steps l + steps r

/-- The children of a step, each with the branch it is checked against. Mirrors
`Dl.check` and is not proved to; it is read only by the diagnostic. -/
def children : Cert → List Constraint → List (Cert × List Constraint)
  | .instS a c k, Γ => [(k, Constraint.conc a c :: Γ)]
  | .relS a r b k, Γ => [(k, Constraint.role r a b :: Γ)]
  | .subS x _ d k, Γ => [(k, Constraint.conc x d :: Γ)]
  | .domS x _ _ c k, Γ => [(k, Constraint.conc x c :: Γ)]
  | .rngS _ y _ c k, Γ => [(k, Constraint.conc y c :: Γ)]
  | .subroleS x y _ t k, Γ => [(k, Constraint.role t x y :: Γ)]
  | .nonemptyS y c k, Γ => [(k, Constraint.conc y c :: Γ)]
  | .andS x c d k, Γ => [(k, Constraint.conc x c :: Constraint.conc x d :: Γ)]
  | .allS _ y _ c k, Γ => [(k, Constraint.conc y c :: Γ)]
  | .exS x y r c k, Γ => [(k, Constraint.role r x y :: Constraint.conc y c :: Γ)]
  | .minS x r c _ ys k, Γ => [(k, minExpand r x c ys ++ Γ)]
  | .orS x c d l r, Γ => [(l, Constraint.conc x c :: Γ), (r, Constraint.conc x d :: Γ)]
  | _, _ => []

/-- Descend to the shallowest step whose own side conditions fail, or to the
shallowest step none of whose children check. Fuel is the tree size. -/
def firstBad (A : List Axiom) : Nat → Cert → List Constraint → String
  | 0, t, _ => stepKind t
  | fuel + 1, t, Γ =>
    match (children t Γ).find? (fun p => !check A p.1 p.2) with
    | some p => firstBad A fuel p.1 p.2
    | none => stepKind t

def readOrFail (path : String) : IO (Except UInt32 String) := do
  try
    return .ok (← IO.FS.readFile path)
  catch e =>
    IO.eprintln s!"cannot read {path}: {e}"
    return .error 2

def run (axPath cPath : String) : IO UInt32 := do
  let axTxt ← match ← readOrFail axPath with
    | .ok s => pure s
    | .error c => return c
  let cTxt ← match ← readOrFail cPath with
    | .ok s => pure s
    | .error c => return c
  match Parse.parseAxioms axTxt, Parse.parseCert cTxt with
  | .ok A, .ok t =>
    if check A t [] then
      IO.println ("{\"ok\":true,\"axioms\":" ++ toString A.length ++
        ",\"steps\":" ++ toString (steps t) ++
        ",\"verdict\":\"unsatisfiable\"" ++
        ",\"theorem\":\"Dl.unsatisfiable_of_check\"}")
      return 0
    else
      IO.println ("{\"ok\":false,\"axioms\":" ++ toString A.length ++
        ",\"steps\":" ++ toString (steps t) ++
        ",\"first_rejected\":" ++ jsonStr (firstBad A (steps t) t []) ++
        ",\"note\":\"a rejected refutation says nothing about the ontology\"}")
      return 1
  | .error e, _ =>
    IO.eprintln e
    return 2
  | _, .error e =>
    IO.eprintln e
    return 2

def main (args : List String) : IO UInt32 := do
  match args with
  | [a, c] => run a c
  | _ =>
    IO.eprintln "usage: oo-dlrefute AXIOMS.tsv REFUTATION.cert"
    return 2
