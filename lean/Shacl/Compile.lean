import Shacl.Report

/-!
# Compiling a shapes graph into shapes

This module turns RDF into the `Shape` datatype the proved evaluator consumes. It
is NOT part of any theorem, and that is the design, not an omission: a compiler
from RDF into an abstract syntax is a parser, and the project's existing position on
parsers is that a parse error rejects rather than accepts.

## It fails closed, and that is the only reason it can be trusted at all

The dangerous failure for a validator is to ignore a constraint it does not
understand and report `conforms`. So:

* **Every `sh:` predicate on every shape the compiler walks must be known.** One it
  has not implemented is an error naming the predicate, not a silently dropped
  constraint. `ignoredParams` lists the handful that carry no conformance meaning,
  each with a reason.
* **A shapes graph that defines a constraint COMPONENT is refused outright.** A
  custom component introduces parameters in the user's own namespace, so the
  `sh:` check above cannot see them, and a shape using one would look empty. The
  guard is a scan for `sh:parameter`, `sh:validator`, `sh:nodeValidator`,
  `sh:propertyValidator`, `sh:sparql` and `rdf:type sh:ConstraintComponent`
  anywhere in the shapes graph.
* **Recursion is refused.** SHACL says it "does not define the semantics of
  recursive shape definitions". `Shape` is an inductive type, so a cyclic shapes
  graph exhausts the nesting budget and is reported as recursive.
* **A malformed shape is refused**, not guessed at: two `sh:path` values, a
  `sh:minCount` on a shape with no path, an RDF list that does not end at
  `rdf:nil`.

Every refusal comes out as `undetermined` in the report and in the conformance
measurement, which counts it separately from a pass and from a failure.

## What is deliberately ignored, and why it is safe here

`sh:message`, `sh:name`, `sh:description`, `sh:order`, `sh:group` and
`sh:defaultValue` carry no conformance meaning at all.

`sh:severity` does carry meaning, for `sh:resultSeverity` in the report. It is
ignored, so every result this validator produces is a violation with no severity
recorded. That is a real gap and it is named here rather than discovered: a report
consumer that acts on severity must not use this validator.
-/
namespace Shacl.Compile

open Shacl

/-! ## Vocabulary -/

namespace SH
def ns : String := "http://www.w3.org/ns/shacl#"
def iri (l : String) : Term := "<" ++ ns ++ l ++ ">"

def NodeShape : Term := iri "NodeShape"
def PropertyShape : Term := iri "PropertyShape"
def ConstraintComponent : Term := iri "ConstraintComponent"

def targetNode : Term := iri "targetNode"
def targetClass : Term := iri "targetClass"
def targetSubjectsOf : Term := iri "targetSubjectsOf"
def targetObjectsOf : Term := iri "targetObjectsOf"

def path : Term := iri "path"
def inversePath : Term := iri "inversePath"
def alternativePath : Term := iri "alternativePath"
def zeroOrMorePath : Term := iri "zeroOrMorePath"
def oneOrMorePath : Term := iri "oneOrMorePath"
def zeroOrOnePath : Term := iri "zeroOrOnePath"

def klass : Term := iri "class"
def datatype : Term := iri "datatype"
def nodeKind : Term := iri "nodeKind"
def minCount : Term := iri "minCount"
def maxCount : Term := iri "maxCount"
def minInclusive : Term := iri "minInclusive"
def maxInclusive : Term := iri "maxInclusive"
def minExclusive : Term := iri "minExclusive"
def maxExclusive : Term := iri "maxExclusive"
def minLength : Term := iri "minLength"
def maxLength : Term := iri "maxLength"
def languageIn : Term := iri "languageIn"
def pattern : Term := iri "pattern"
def flags : Term := iri "flags"
def equals : Term := iri "equals"
def disjoint : Term := iri "disjoint"
def lessThan : Term := iri "lessThan"
def lessThanOrEquals : Term := iri "lessThanOrEquals"
def uniqueLang : Term := iri "uniqueLang"
def closed : Term := iri "closed"
def ignoredProperties : Term := iri "ignoredProperties"
def xone : Term := iri "xone"
def qualifiedValueShape : Term := iri "qualifiedValueShape"
def qualifiedMinCount : Term := iri "qualifiedMinCount"
def qualifiedMaxCount : Term := iri "qualifiedMaxCount"
def qualifiedValueShapesDisjoint : Term := iri "qualifiedValueShapesDisjoint"
def hasValue : Term := iri "hasValue"
def inList : Term := iri "in"
def notC : Term := iri "not"
def andC : Term := iri "and"
def orC : Term := iri "or"
def nodeC : Term := iri "node"
def property : Term := iri "property"
def deactivated : Term := iri "deactivated"

def message : Term := iri "message"
def name : Term := iri "name"
def description : Term := iri "description"
def order : Term := iri "order"
def group : Term := iri "group"
def defaultValue : Term := iri "defaultValue"
def severity : Term := iri "severity"

def parameter : Term := iri "parameter"
def validator : Term := iri "validator"
def nodeValidator : Term := iri "nodeValidator"
def propertyValidator : Term := iri "propertyValidator"
def sparql : Term := iri "sparql"

def kIRI : Term := iri "IRI"
def kBlankNode : Term := iri "BlankNode"
def kLiteral : Term := iri "Literal"
def kBlankNodeOrIRI : Term := iri "BlankNodeOrIRI"
def kBlankNodeOrLiteral : Term := iri "BlankNodeOrLiteral"
def kIRIOrLiteral : Term := iri "IRIOrLiteral"

end SH

/-- Parameters with a conformance meaning that this development implements. -/
def knownParams : List Term :=
  [ SH.path, SH.klass, SH.datatype, SH.nodeKind, SH.minCount, SH.maxCount,
    SH.hasValue, SH.inList, SH.notC, SH.andC, SH.orC, SH.nodeC, SH.property,
    SH.deactivated, SH.targetNode, SH.targetClass, SH.targetSubjectsOf,
    SH.targetObjectsOf, SH.minInclusive, SH.maxInclusive, SH.minExclusive,
    SH.maxExclusive, SH.minLength, SH.maxLength, SH.languageIn, SH.equals,
    SH.disjoint, SH.lessThan, SH.lessThanOrEquals, SH.uniqueLang, SH.closed,
    SH.ignoredProperties, SH.xone, SH.qualifiedValueShape, SH.qualifiedMinCount,
    SH.qualifiedMaxCount, SH.qualifiedValueShapesDisjoint, SH.pattern, SH.flags ]

/-- Parameters with no effect on the verdict or on the result fields this
validator reports. `sh:severity` is the one that is a real gap; see the module
header. -/
def ignoredParams : List Term :=
  [ SH.message, SH.name, SH.description, SH.order, SH.group, SH.defaultValue,
    SH.severity ]

def isShaclPred (p : Term) : Bool := p.startsWith ("<" ++ SH.ns)

/-! ## Reading the graph -/

def objectsOf (G : Graph) (s p : Term) : List Term :=
  (G.filter (fun t => t.s == s && t.p == p)).map Triple.o

def predsOf (G : Graph) (s : Term) : List Term :=
  dedup ((G.filter (fun t => t.s == s)).map Triple.p)

def hasTriple (G : Graph) (s p o : Term) : Bool :=
  G.any (fun t => t.s == s && t.p == p && t.o == o)

/-- An RDF list, or an error naming where it went wrong. -/
def readList (G : Graph) : Nat → Term → Except String (List Term)
  | 0, node => .error s!"RDF list at {node} is longer than the shapes graph, so it is cyclic"
  | fuel + 1, node =>
      if node == V.nil then .ok []
      else
        match objectsOf G node V.first, objectsOf G node V.rest with
        | [f], [r] => do
            let tl ← readList G fuel r
            .ok (f :: tl)
        | _, _ => .error s!"malformed RDF list at {node}: expected one rdf:first and one rdf:rest"

/-- The natural number a literal denotes, for `sh:minCount` and `sh:maxCount`. -/
def readNat (o : Term) : Except String Nat :=
  match asLiteral o with
  | some l =>
      if isDigits l.lex.toList then .ok (natOfDigits l.lex.toList)
      else .error s!"{o} is not a non-negative integer literal"
  | none => .error s!"{o} is not a literal, so it cannot be a count"

def readNodeKind (o : Term) : Except String NodeKind :=
  if o == SH.kIRI then .ok .iri
  else if o == SH.kBlankNode then .ok .blankNode
  else if o == SH.kLiteral then .ok .literal
  else if o == SH.kBlankNodeOrIRI then .ok .blankNodeOrIRI
  else if o == SH.kBlankNodeOrLiteral then .ok .blankNodeOrLiteral
  else if o == SH.kIRIOrLiteral then .ok .iriOrLiteral
  else .error s!"{o} is not one of the six sh:nodeKind values"

/-- The path forms present on a blank path node, named. -/
def pathForms (G : Graph) (node : Term) : List String :=
  (if (objectsOf G node SH.inversePath).isEmpty then [] else ["sh:inversePath"]) ++
  (if (objectsOf G node SH.alternativePath).isEmpty then [] else ["sh:alternativePath"]) ++
  (if (objectsOf G node SH.zeroOrMorePath).isEmpty then [] else ["sh:zeroOrMorePath"]) ++
  (if (objectsOf G node SH.oneOrMorePath).isEmpty then [] else ["sh:oneOrMorePath"]) ++
  (if (objectsOf G node SH.zeroOrOnePath).isEmpty then [] else ["sh:zeroOrOnePath"]) ++
  (if (objectsOf G node V.first).isEmpty then [] else ["a sequence path"])

/-- Fold a sequence path's member list to the right. SHACL's sequence path is
n-ary; `Path.seq` is binary, and `a/b/c` means `a/(b/c)` under either bracketing
because the value nodes of a sequence are the same set either way. -/
def foldSeq : List Path → Except String Path
  | [] => .error "a sequence path with no members"
  | [p] => .ok p
  | p :: rest => do
      let t ← foldSeq rest
      .ok (.seq p t)

/-- The same for `sh:alternativePath`, where the folding is associative for the
same reason: alternative is union. -/
def foldAltPath : List Path → Except String Path
  | [] => .error "an sh:alternativePath with no members"
  | [p] => .ok p
  | p :: rest => do
      let t ← foldAltPath rest
      .ok (.alt p t)

/-- A path, or an error naming the path form that was found. `sh:zeroOrMorePath`
and `sh:oneOrMorePath` are the two forms refused; see `Shape.lean` for why.

A node carrying TWO path forms is refused rather than read as either. SHACL
requires a well-formed path node to have exactly one form, and the suite's
`core/path/path-strange-001` is a node that is simultaneously a sequence path and
an inverse path. Reading it as the first form the code happens to check is how this
compiler produced its one wrong answer against the suite: it chose the inverse
reading and blamed the wrong node. Refusing an ill-formed shapes graph and saying
so is the answer that cannot be wrong, and it stays refused now that sequence paths
ARE implemented: the Working Group's expected report reads that node as a sequence,
but the shapes graph does not say so and this compiler will not pick.

`fuel` bounds the nesting of path nodes. One level of nesting consumes at least one
triple of the shapes graph, so a budget of `|G| + 1` exhausts only on a cycle. -/
def compilePath (G : Graph) : Nat → Term → Except String Path
  | 0, node =>
      .error s!"the path at {node} nests deeper than the shapes graph is long, so it is cyclic"
  | fuel + 1, node =>
    -- The path FORMS are examined before the spelling, and the order is the
    -- whole of #205.
    --
    -- This used to read `if isIriSpelling node then .ok (.pred node)` first, so
    -- a path node spelled as a named IRI that carried `sh:zeroOrMorePath` was
    -- compiled as a plain predicate. The evaluator then answered a question
    -- nobody asked, exit 0, with a verdict: the same shape written with a blank
    -- node was refused by name with exit 3. A shapes author chooses that
    -- spelling freely and has no reason to think it matters.
    --
    -- That is the collapse the three-state verdict exists to prevent, reached
    -- by a route the theorem cannot see: `Shacl.validate_spec` covers the
    -- EVALUATOR, and the compiler had handed it a different shape from the one
    -- that was written. A node carrying any path predicate now takes the same
    -- branch whatever its spelling, so the two unimplemented forms are refused
    -- by name either way, and only a node carrying NO path predicate is read as
    -- a plain predicate.
    match pathForms G node with
    | [] =>
        if isIriSpelling node then .ok (.pred node)
        else .error s!"{node} is not a path this development understands"
    | ["sh:inversePath"] =>
        match objectsOf G node SH.inversePath with
        | [inner] =>
            if isIriSpelling inner then .ok (.inv inner)
            else .error "sh:inversePath of something other than a predicate is not implemented"
        | _ => .error "more than one sh:inversePath on a path node"
    | ["sh:alternativePath"] =>
        match objectsOf G node SH.alternativePath with
        | [l] => do
            let members ← readList G (G.length + 1) l
            let mut ps : List Path := []
            for m in members do
              ps := ps ++ [← compilePath G fuel m]
            foldAltPath ps
        | _ => .error "more than one sh:alternativePath on a path node"
    | ["sh:zeroOrOnePath"] =>
        match objectsOf G node SH.zeroOrOnePath with
        | [inner] => do
            let p ← compilePath G fuel inner
            .ok (.zeroOrOne p)
        | _ => .error "more than one sh:zeroOrOnePath on a path node"
    | ["a sequence path"] => do
        let members ← readList G (G.length + 1) node
        let mut ps : List Path := []
        for m in members do
          ps := ps ++ [← compilePath G fuel m]
        foldSeq ps
    | [one] => .error s!"{one} is not implemented"
    | many =>
        .error s!"the path node {node} carries more than one path form \
          ({String.intercalate ", " many}); SHACL allows exactly one, so this shapes graph is \
          ill formed and no reading of it is chosen"

/-- Is this parameter present with the literal value `true`?

Only the lexical form `true` counts. `"1"^^xsd:boolean` denotes the same boolean and
does NOT activate the constraint, which is what `core/property/uniqueLang-002`
requires and what the Recommendation's wording ("if `$uniqueLang` is `true`")
licenses: the parameter value is compared as a term. -/
def hasTrue (G : Graph) (node p : Term) : Bool :=
  (objectsOf G node p).any fun o =>
    match asLiteral o with
    | some l => l.lex == "true"
    | none => false

def isDeactivated (G : Graph) (node : Term) : Bool := hasTrue G node SH.deactivated

/-- The lexical form of a literal, for the string-valued parameters. -/
def readStringLit (o : Term) : Except String String :=
  match asLiteral o with
  | some l => .ok l.lex
  | none => .error s!"{o} is not a literal, so it cannot be a string-valued parameter"

/-! ## Reading a `sh:pattern`

Outside the theorem like everything else in this file, and refusing BY NAME. The
subset the matcher decides is documented in `Shacl/Shape.lean`; every construct
outside it is an error naming the character, so a shapes graph using one is
undetermined rather than answered by a matcher that quietly ignored it. -/

/-- The ranges of a character class, starting after the opening bracket and the
optional negation, ending at the closing bracket. -/
def classRanges : List Char → Option (List (Char × Char) × List Char)
  | [] => none
  | ']' :: rest => some ([], rest)
  | a :: '-' :: b :: rest =>
      if a == '\\' || b == '\\' || b == ']' then none
      else (classRanges rest).map fun p => ((a, b) :: p.1, p.2)
  | c :: rest =>
      if c == '\\' || c == '[' then none
      else (classRanges rest).map fun p => ((c, c) :: p.1, p.2)

/-- The characters that mean something in a regular expression and that this
development does not implement. `-` is absent on purpose: outside a character class
it is an ordinary character. -/
def isPatternMeta (c : Char) : Bool :=
  c == '[' || c == ']' || c == '(' || c == ')' || c == '{' || c == '}' ||
  c == '|' || c == '*' || c == '+' || c == '?' || c == '.' || c == '^' ||
  c == '$' || c == '\\'

def readQuant : List Char → Quant × List Char
  | '*' :: rest => (.star, rest)
  | cs => (.one, cs)

def readItems : Nat → List Char → Except String (List Item)
  | 0, _ =>
      .error "the pattern consumed more elements than it has characters, which cannot happen"
  | _ + 1, [] => .ok []
  | fuel + 1, '[' :: rest =>
      let (neg, r1) := match rest with | '^' :: r => (true, r) | r => (false, r)
      match classRanges r1 with
      | none => .error "the pattern uses a character class this matcher does not parse"
      | some (rs, r2) => do
          let (q, r3) := readQuant r2
          let tl ← readItems fuel r3
          .ok ({ neg := neg, ranges := rs, quant := q } :: tl)
  | fuel + 1, c :: rest =>
      if isPatternMeta c then
        .error s!"the pattern uses the regular-expression construct '{c}', which is not \
          implemented; see the subset in lean/Shacl/Shape.lean"
      else do
        let (q, r3) := readQuant rest
        let tl ← readItems fuel r3
        .ok ({ neg := false, ranges := [(c, c)], quant := q } :: tl)

/-- `sh:flags`, of which only `i` is implemented. -/
def readFlags (G : Graph) (node : Term) : Except String Bool := do
  let mut fold := false
  for o in objectsOf G node SH.flags do
    let f ← readStringLit o
    for c in f.toList do
      if c == 'i' then fold := true
      else throw s!"{node}: sh:flags \"{f}\" uses the flag '{c}', which is not implemented"
  return fold

def readPattern (fold : Bool) (lex : String) : Except String Regex :=
  if hasBackslash lex then
    .error "the pattern carries a backslash, and this development never decodes an escape, so \
      it cannot tell a literal backslash from an escape sequence"
  else
    let cs := lex.toList
    let (anchorStart, c1) := match cs with | '^' :: r => (true, r) | r => (false, r)
    let (anchorEnd, c2) :=
      match c1.getLast? with
      | some '$' => (true, c1.dropLast)
      | _ => (false, c1)
    do
      let items ← readItems (c2.length + 1) c2
      .ok { fold := fold, anchorStart := anchorStart, anchorEnd := anchorEnd, items := items }

/-- Wrap a value-node constraint for the shape it sits on: on a property shape it
applies to every value node, on a node shape to the focus node itself. -/
def wrapV : Option Path → Shape → Shape
  | none, s => s
  | some pa, s => .forAll pa s

def foldAnd : List Shape → Shape
  | [] => .top
  | s :: rest => .andC s (foldAnd rest)

def foldOr : List Shape → Shape
  | [] => .bot
  | s :: rest => .orC s (foldOr rest)

/-- One disjunct per member: that member conforms and none of the others does. -/
def pickOne : List Shape → List Shape → List Shape
  | _, [] => []
  | others, s :: rest =>
      Shape.andC s (foldAnd ((others ++ rest).map Shape.notC)) :: pickOne (others ++ [s]) rest

/-- `sh:xone`: EXACTLY one of the shapes conforms.

Compiled out of `sh:or`, `sh:and` and `sh:not` rather than given a constructor of
its own, so the semantics is the one `Spec.lean` already gives those three and the
agreement proof already covers it. The enclosing `report` is what makes the single
result carry `sh:XoneConstraintComponent`.

The cost is that each member shape is evaluated once per disjunct, so an `sh:xone`
of `n` members does `n` squared shape evaluations. The Working Group's tests use two
and three. A shapes graph with a long `sh:xone` list would be slow here, and that is
a real limit rather than a theoretical one. -/
def exactlyOne (ss : List Shape) : Shape := foldOr (pickOne [] ss)

/-- The sibling shapes of a property shape, for `sh:qualifiedValueShapesDisjoint`:
the `sh:qualifiedValueShape` of every OTHER property shape that the same parent
references through `sh:property`. Comparison is by term, so a parent that lists the
same property shape twice still has it as itself and not as its own sibling. -/
def siblingQualifiedShapes (G : Graph) (parent : Option Term) (self : Term) : List Term :=
  match parent with
  | none => []
  | some p =>
      ((dedup (objectsOf G p SH.property)).filter (fun ps => ps != self)).flatMap
        (fun ps => objectsOf G ps SH.qualifiedValueShape)

/-- Compile one shape node. `fuel` bounds the nesting; one level of nesting always
consumes at least one triple of the shapes graph, so a budget of `|G| + 1`
exhausts only on a cycle.

`parent` is the shape that reached this one through `sh:property`, and it is
threaded only because `sh:qualifiedValueShapesDisjoint` asks about siblings. A shape
reached any other way, or reached as a top-level target, has no parent for that
purpose and gets `none`. -/
def compileShape (G : Graph) : Nat → Option Term → Term → Except String Shape
  | 0, _, node =>
      .error s!"shape nesting at {node} exceeds the size of the shapes graph, so the shapes \
        are recursive; SHACL does not define the semantics of recursive shapes"
  | fuel + 1, parent, node => do
      for p in predsOf G node do
        if isShaclPred p && !(knownParams.contains p) && !(ignoredParams.contains p) then
          throw s!"{node}: the constraint parameter {p} is not implemented"
      if isDeactivated G node then
        return .top
      let pathOpt : Option Path ← (
        match objectsOf G node SH.path with
        | [] => pure none
        | [po] => do
            let pa ← compilePath G (G.length + 1) po
            pure (some pa)
        | _ => throw s!"{node}: more than one sh:path")
      let mut acc : Shape := .top
      for o in objectsOf G node SH.klass do
        acc := .both acc (wrapV pathOpt (.klass o))
      for o in objectsOf G node SH.datatype do
        acc := .both acc (wrapV pathOpt (.datatype o))
      for o in objectsOf G node SH.nodeKind do
        let k ← readNodeKind o
        acc := .both acc (wrapV pathOpt (.nodeKind k))
      for o in objectsOf G node SH.inList do
        let vs ← readList G (G.length + 1) o
        acc := .both acc (wrapV pathOpt (.inSet vs))
      for o in objectsOf G node SH.minInclusive do
        acc := .both acc (wrapV pathOpt (.minInclusive o))
      for o in objectsOf G node SH.maxInclusive do
        acc := .both acc (wrapV pathOpt (.maxInclusive o))
      for o in objectsOf G node SH.minExclusive do
        acc := .both acc (wrapV pathOpt (.minExclusive o))
      for o in objectsOf G node SH.maxExclusive do
        acc := .both acc (wrapV pathOpt (.maxExclusive o))
      for o in objectsOf G node SH.minLength do
        let n ← readNat o
        acc := .both acc (wrapV pathOpt (.minLength n))
      for o in objectsOf G node SH.maxLength do
        let n ← readNat o
        acc := .both acc (wrapV pathOpt (.maxLength n))
      let patterns := objectsOf G node SH.pattern
      if !patterns.isEmpty then
        let fold ← readFlags G node
        for o in patterns do
          let lex ← readStringLit o
          let re ← readPattern fold lex
          acc := .both acc (wrapV pathOpt (.pattern re))
      for o in objectsOf G node SH.languageIn do
        let members ← readList G (G.length + 1) o
        let mut tags : List String := []
        for m in members do
          tags := tags ++ [← readStringLit m]
        acc := .both acc (wrapV pathOpt (.languageIn tags))
      for o in objectsOf G node SH.equals do
        acc := .both acc (.equals pathOpt o)
      for o in objectsOf G node SH.disjoint do
        acc := .both acc (.disjoint pathOpt o)
      for o in objectsOf G node SH.lessThan do
        acc := .both acc (.lessThan pathOpt o)
      for o in objectsOf G node SH.lessThanOrEquals do
        acc := .both acc (.lessThanOrEq pathOpt o)
      if hasTrue G node SH.uniqueLang then
        acc := .both acc (.uniqueLang pathOpt)
      for o in objectsOf G node SH.notC do
        let s ← compileShape G fuel none o
        acc := .both acc (wrapV pathOpt (.notC s))
      for o in objectsOf G node SH.nodeC do
        let s ← compileShape G fuel none o
        acc := .both acc (wrapV pathOpt (.nodeC s))
      for o in objectsOf G node SH.andC do
        let members ← readList G (G.length + 1) o
        let mut ss : List Shape := []
        for m in members do
          ss := ss ++ [← compileShape G fuel none m]
        acc := .both acc (wrapV pathOpt (foldAnd ss))
      for o in objectsOf G node SH.orC do
        let members ← readList G (G.length + 1) o
        let mut ss : List Shape := []
        for m in members do
          ss := ss ++ [← compileShape G fuel none m]
        acc := .both acc (wrapV pathOpt (foldOr ss))
      for o in objectsOf G node SH.xone do
        let members ← readList G (G.length + 1) o
        let mut ss : List Shape := []
        for m in members do
          ss := ss ++ [← compileShape G fuel none m]
        acc := .both acc (wrapV pathOpt (.report C.xone (exactlyOne ss)))
      if hasTrue G node SH.closed then
        -- The allowed predicates: the `sh:path` of every property shape this shape
        -- references, when that path is an IRI, plus `sh:ignoredProperties`. A path
        -- that is not an IRI contributes nothing, which is what the Recommendation
        -- says ("all values of sh:path ... assuming these are IRIs").
        let mut allowed : List Term := []
        for ps in objectsOf G node SH.property do
          for p in objectsOf G ps SH.path do
            if isIriSpelling p then allowed := allowed ++ [p]
        for o in objectsOf G node SH.ignoredProperties do
          allowed := allowed ++ (← readList G (G.length + 1) o)
        acc := .both acc (wrapV pathOpt (.closed allowed))
      -- `sh:qualifiedValueShape` is the parameter that ACTIVATES the qualified
      -- components. Without it the count parameters carry no constraint at all, and
      -- `core/node/qualified-001` is the Working Group's test that they are then
      -- ignored rather than misread.
      match objectsOf G node SH.qualifiedValueShape with
      | [] => pure ()
      | [qv] =>
          match pathOpt with
          | none =>
              throw s!"{node}: sh:qualifiedValueShape on a shape with no sh:path; SHACL defines \
                the qualified components for property shapes only"
          | some pa => do
              let inner ← compileShape G fuel none qv
              let mut q : Shape := inner
              if hasTrue G node SH.qualifiedValueShapesDisjoint then
                for sv in siblingQualifiedShapes G parent node do
                  let sibShape ← compileShape G fuel none sv
                  q := .both q (.notC sibShape)
              for o in objectsOf G node SH.qualifiedMinCount do
                let n ← readNat o
                acc := .both acc (.qualifiedMin pa q n)
              for o in objectsOf G node SH.qualifiedMaxCount do
                let n ← readNat o
                acc := .both acc (.qualifiedMax pa q n)
      | _ => throw s!"{node}: more than one sh:qualifiedValueShape"
      for o in objectsOf G node SH.property do
        let s ← compileShape G fuel (some node) o
        acc := .both acc (wrapV pathOpt s)
      for o in objectsOf G node SH.hasValue do
        match pathOpt with
        | none => acc := .both acc (.hasValue o)
        | some pa => acc := .both acc (.hasValueOn pa o)
      for o in objectsOf G node SH.minCount do
        let n ← readNat o
        match pathOpt with
        | none => throw s!"{node}: sh:minCount on a shape with no sh:path"
        | some pa => acc := .both acc (.minCount pa n)
      for o in objectsOf G node SH.maxCount do
        let n ← readNat o
        match pathOpt with
        | none => throw s!"{node}: sh:maxCount on a shape with no sh:path"
        | some pa => acc := .both acc (.maxCount pa n)
      return .named node acc

/-! ## Targets -/

def targetsOf (G : Graph) (node : Term) : List Target :=
  let explicit :=
    (objectsOf G node SH.targetNode).map Target.node ++
    (objectsOf G node SH.targetClass).map Target.klass ++
    (objectsOf G node SH.targetSubjectsOf).map Target.subjectsOf ++
    (objectsOf G node SH.targetObjectsOf).map Target.objectsOf
  -- The implicit class target: a node that is both a shape and a class targets its
  -- own instances.
  let isShape :=
    hasTriple G node V.type SH.NodeShape || hasTriple G node V.type SH.PropertyShape
  if isShape && hasTriple G node V.type V.rdfsClass then
    explicit ++ [Target.klass node]
  else explicit

/-- Every node in the shapes graph that carries at least one target. -/
def targetedNodes (G : Graph) : List Term :=
  dedup ((G.filterMap fun t =>
    if t.p == SH.targetNode || t.p == SH.targetClass || t.p == SH.targetSubjectsOf
       || t.p == SH.targetObjectsOf then some t.s
    else if t.p == V.type && t.o == V.rdfsClass then
      if hasTriple G t.s V.type SH.NodeShape || hasTriple G t.s V.type SH.PropertyShape
      then some t.s else none
    else none))

/-- The global guard: a shapes graph that defines its own constraint components
introduces parameters outside the `sh:` namespace, so the per-shape check cannot
see them and a shape using one would look empty. Refuse the whole graph. -/
def extensionMechanism (G : Graph) : Option String :=
  let hit := G.find? fun t =>
    t.p == SH.parameter || t.p == SH.validator || t.p == SH.nodeValidator
      || t.p == SH.propertyValidator || t.p == SH.sparql
      || (t.p == V.type && t.o == SH.ConstraintComponent)
  match hit with
  | some t =>
      some s!"the shapes graph uses the SHACL extension mechanism ({t.p} on {t.s}); a \
        constraint component can declare parameters outside the sh: namespace, which this \
        compiler cannot detect, so the whole graph is refused rather than partly ignored"
  | none => none

/-- Compile a whole shapes graph into declarations, or refuse with a reason. -/
def compileShapes (G : Graph) : Except String (List ShapeDecl) := do
  match extensionMechanism G with
  | some why => throw why
  | none => pure ()
  let mut out : Array ShapeDecl := #[]
  for node in targetedNodes G do
    if isDeactivated G node then
      continue
    let s ← compileShape G (G.length + 1) none node
    out := out.push { id := node, targets := targetsOf G node, shape := s }
  return out.toList

end Shacl.Compile
