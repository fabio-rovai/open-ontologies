import OOCert.Soundness

/-!
# `oo-refute/1`: certifying that a graph has NO model

Seventeen rules in the OWL 2 RL profile conclude `false` rather than a triple.
Counted off OWL 2 Web Ontology Language Profiles (Second Edition), W3C
Recommendation 11 December 2012, section 4.3, the whole list is:

| Table | Rules concluding `false` |
| --- | --- |
| 4, Equality | `eq-diff1`, `eq-diff2`, `eq-diff3` |
| 5, Axioms about Properties | `prp-irp`, `prp-asyp`, `prp-pdw`, `prp-adp`, `prp-npa1`, `prp-npa2` |
| 6, Classes | `cls-nothing2`, `cls-com`, `cls-maxc1`, `cls-maxqc1`, `cls-maxqc2` |
| 7, Class Axioms | `cax-dw`, `cax-adc` |
| 8, Datatypes | `dt-not-type` |

The list is written out rather than summarised because the number alone is a
claim nobody can re-check. Three plus six plus five plus two plus one is
seventeen, out of the 78 rules named across Tables 4 to 9. Table 9, Schema
Vocabulary, contributes none of them: all 20 of its rules conclude a triple, so
the ones this project implements live in `Semantics.lean` as ordinary
conditions.

`cax-dw` is the one implemented here, and the one with the most axioms waiting
for it: given `c1 owl:disjointWith c2`, `x rdf:type c1` and `x rdf:type c2`, the
graph is contradictory. The certificate format in `Rules.lean` cannot express
that, because `Step.conclusion` is a `Triple` and `certificate_sound` concludes
`Entails`. There is no triple to conclude. This file adds the second format.

Sixteen of the seventeen are rules this layer can state, and it states one of
them, `cax-dw`. The seventeenth, `dt-not-type`, it cannot state at all. The
distinction is drawn where the count is, below, because a reader who meets only
the number will otherwise take "not implemented" and "not expressible" for the
same thing, and they are different claims about this checker.

## Why the negative conditions live HERE and not in `Semantics.lean`

`lean/OOCert/Witness.lean` proves `saturated_is_a_model`: every graph has a
model, the one in which every property relates everything. That theorem is what
stops `certificate_sound` being vacuously true, and its truth depends on the
fragment having no negation. A disjointness condition is negative: it says
nothing is in both of two disjoint classes, and the saturated interpretation
puts everything in every class. Adding such a condition to `Conditions` would
make `saturated_is_a_model` FALSE, and that theorem cannot be patched because
being unfalsifiable is the whole point of it.

So `RefuteConditions` is a SEPARATE layer. `Conditions` is untouched, every
existing theorem keeps its meaning, and the two model classes are named
differently so that no report can confuse them.

## What `Unsat` means, and what it does not

`Unsat G` says: no interpretation satisfies BOTH `Model I G` AND
`RefuteConditions I`. It is unsatisfiability RELATIVE TO THOSE CONDITIONS, not
absolute unsatisfiability. A reader who takes it for the latter will be wrong in
a specific way: `Model I G` alone is still satisfiable for every graph, refuted
or not, because `saturated` ignores disjointness. `Unsat G` is the statement
that reading `owl:disjointWith` as disjointness leaves no models, which is what
an OWL 2 RL consumer means by "inconsistent" and is not the same sentence.

`RefuteConditions` carries TWELVE fields, one per certifiable rule: `cax-dw`,
`cls-com`, `cls-nothing2`, `prp-irp`, `prp-asyp`, `prp-pdw`, `eq-diff1`,
`prp-npa1`, `prp-npa2`, `cls-maxc1`, `cls-maxqc1` and `cls-maxqc2`. It carried
one until #163, and the widening is the reason this paragraph reads differently
from the rest of the file's history.

The three cardinality rules are here because the profile's cardinality clashes
are the ZERO cases and nothing else. `cls-maxc1` fires on `owl:maxCardinality 0`
plus a single edge, which needs the literal and no counting. A rule that had to
COUNT successors could not be stated in this step format at all, so their
presence is not evidence that counting arrived.

They match `"0"^^xsd:nonNegativeInteger` by its spelling, as this layer matches
every other term. That is sound and it is not complete: a graph writing the same
value some other way is missed, which is a rejection and never a false pass.

FOUR are missing and expressible: `cax-adc`, `prp-adp`, `eq-diff2` and
`eq-diff3`. Each states its members in an `rdf:List`, and `RefuteStep.premises`
is a fixed-length list of triples, so a premise of unbounded length has no
spelling here. Adding them is ordinary work rather than new mathematics: `Model`
already reads lists through `Chain` for `owl:intersectionOf`, and the step
format would have to carry one. The PAIRWISE form of all four is covered
already, by `cax-dw`, `prp-pdw` and `eq-diff1`, which is why they are worth
calling not done rather than impossible.

ONE is not missing. `dt-not-type` fires when the data value of a literal falls
outside the value space of the datatype it is typed with, and `Semantics.lean`
has no value spaces to fall outside of. An `Interp` there is a domain, a
denotation `ι : Term → D` and one ternary relation; a literal is its N-Triples
spelling like any other term; and that file's "What is not here" section states
the restriction and its consequence, that `Entails` must not be read as
datatype-aware. No field added to `RefuteConditions` could cover the rule,
because what it needs is absent from `Interp` rather than from the condition
structure. Covering it means extending the signature with a datatype map and a
lexical-to-value reading, which is a different piece of work from adding a
clash rule and would change what every existing theorem quantifies over.
`src/reason.rs` keeps the whole `dt-*` family out of the engine on the same
ground, and `tests/reason_rl_coverage_test.rs` pins that exclusion.

A graph that only a missing rule could refute is not refutable by this checker,
whichever group the rule falls in, and the checker says so by rejecting, never
by passing.

## The explosion, and what a consumer must do about it

Once a graph is refutable, every triple is entailed under the disjointness-aware
reading, so an ordinary triple certificate over that graph carries no
information at all. `a_certificate_adds_nothing_when_the_graph_is_refuted`
states this as a theorem rather than as advice: for a refuted `G`, EVERY
conclusion of EVERY list of steps has the disjointness-aware warrant, whether
the checker accepted the certificate or not.

The trap here is subtler than plain explosion, and it is worth being exact.
`certificate_sound`'s verdict, `Entails G t`, is NOT trivialised by a refutation,
because `Entails` quantifies over `Model I G` alone and that class is never
empty. So a triple certificate over a self-contradicting ontology still checks
green, still says something literally true, and still says nothing a consumer
wants. The guarantee the consumer wants is `EntailsIn (RModel G)`, and that one
is empty. A consumer must therefore run the refutation checker FIRST and treat a
successful refutation as invalidating every derivation over the same graph.
`oo-refute guard` does that in one command and refuses the triple certificate.

## The format

```
oo-refute/1
<rule>  <s>  <p>  <o>  [<ps>  <pp>  <po>]*        (zero or more, tab separated)
refute  cax-dw  <c1>  <p>  <c2>  <x>  <p>  <c1>  <x>  <p>  <c2>
```

The first line is the version marker. The lines between it and the last are
ordinary derivation steps in the `derivations.tsv` spelling, so a refutation may
reach its contradiction through inference: the disjoint types are usually
derived by `rdfs9` off a subclass chain rather than asserted. The last line is
the contradiction step, and there is exactly one. A `derivations.tsv` handed to
this checker is rejected, because it has no `refute` line and no version marker.
-/
namespace OOCert

/-! ## Vocabulary

`V` lives in `Triple.lean` and this round belongs to another file's owner, so
the one term this layer adds is kept here. It is the same N-Triples spelling the
engine's interner produces. -/
namespace RV
/-- `owl:disjointWith`. -/
def disjointWith : Term := "<http://www.w3.org/2002/07/owl#disjointWith>"
def complementOf : Term := "<http://www.w3.org/2002/07/owl#complementOf>"
def nothing : Term := "<http://www.w3.org/2002/07/owl#Nothing>"
def irreflexiveProperty : Term := "<http://www.w3.org/2002/07/owl#IrreflexiveProperty>"
def asymmetricProperty : Term := "<http://www.w3.org/2002/07/owl#AsymmetricProperty>"
def propertyDisjointWith : Term := "<http://www.w3.org/2002/07/owl#propertyDisjointWith>"
def differentFrom : Term := "<http://www.w3.org/2002/07/owl#differentFrom>"
def sourceIndividual : Term := "<http://www.w3.org/2002/07/owl#sourceIndividual>"
def assertionProperty : Term := "<http://www.w3.org/2002/07/owl#assertionProperty>"
def targetIndividual : Term := "<http://www.w3.org/2002/07/owl#targetIndividual>"
def targetValue : Term := "<http://www.w3.org/2002/07/owl#targetValue>"
def maxCardinality : Term := "<http://www.w3.org/2002/07/owl#maxCardinality>"
def maxQualifiedCardinality : Term :=
  "<http://www.w3.org/2002/07/owl#maxQualifiedCardinality>"
def onClass : Term := "<http://www.w3.org/2002/07/owl#onClass>"
def thing : Term := "<http://www.w3.org/2002/07/owl#Thing>"
/-- The literal `"0"^^xsd:nonNegativeInteger`, in the spelling the rules use.

The three cardinality rules of the profile that conclude `false` are the ZERO
cases and nothing else, which is why they are here at all: `cls-maxc1` says a
class with `owl:maxCardinality 0` on a property cannot have a member with that
property, and deciding that needs no counting, only the literal. A rule that
needed to COUNT successors could not be stated in this format. -/
def zero : Term :=
  "\"0\"^^<http://www.w3.org/2001/XMLSchema#nonNegativeInteger>"
end RV

/-! ## The negative conditions -/

/-- The conditions a model must satisfy for a REFUTATION to mean anything.
Twelve fields, one per certifiable clash rule.

Each is the *if* direction only, which is all the corresponding rule uses, and
each is a consequence of the OWL 2 condition rather than a strengthening of it.
Adding a field makes `RModel` HARDER to satisfy, so it makes `Unsat` easier to
prove and `¬ Unsat` harder: the non-vacuity witness below has to discharge every
one of them, which is the price of each rule and is paid there rather than
asserted here.

**Twelve of seventeen, and the five that are missing are missing for two
different reasons.** `cax-adc`, `prp-adp`, `eq-diff2` and `eq-diff3` read an
`rdf:List` of members, which this step format cannot carry: its premises are a
fixed-length list of triples and a list of unbounded length is not one.
`dt-not-type` needs a datatype value space, which this development does not
have at all. The first four are NOT IMPLEMENTED and the fifth is NOT
EXPRESSIBLE, and a reader who meets only the number would take those for the
same claim. -/
structure RefuteConditions (I : Interp) : Prop where
  /-- cax-dw. Nothing is in two disjoint classes. -/
  dw : ∀ a b, I.iext (I.ι RV.disjointWith) a b → ∀ x, I.cext a x → I.cext b x → False
  /-- cls-com. Nothing is in a class and its complement. -/
  com : ∀ a b, I.iext (I.ι RV.complementOf) a b → ∀ x, I.cext a x → I.cext b x → False
  /-- cls-nothing2. `owl:Nothing` is empty. -/
  nothing : ∀ x, I.cext (I.ι RV.nothing) x → False
  /-- prp-irp. An irreflexive property relates nothing to itself. -/
  irp : ∀ p, I.cext (I.ι RV.irreflexiveProperty) p → ∀ x, I.iext p x x → False
  /-- prp-asyp. An asymmetric property does not relate both ways. -/
  asyp : ∀ p, I.cext (I.ι RV.asymmetricProperty) p → ∀ x y, I.iext p x y → I.iext p y x → False
  /-- prp-pdw. Disjoint properties share no pair. -/
  pdw : ∀ p q, I.iext (I.ι RV.propertyDisjointWith) p q →
    ∀ x y, I.iext p x y → I.iext q x y → False
  /-- eq-diff1. Nothing is both the same as and different from something. -/
  diff : ∀ x y, I.iext (I.ι V.sameAs) x y → I.iext (I.ι RV.differentFrom) x y → False
  /-- prp-npa1. A negative object property assertion is not satisfied. -/
  npa1 : ∀ n i p j, I.iext (I.ι RV.sourceIndividual) n i →
    I.iext (I.ι RV.assertionProperty) n p → I.iext (I.ι RV.targetIndividual) n j →
    I.iext p i j → False
  /-- prp-npa2. The same for a data value. -/
  npa2 : ∀ n i p j, I.iext (I.ι RV.sourceIndividual) n i →
    I.iext (I.ι RV.assertionProperty) n p → I.iext (I.ι RV.targetValue) n j →
    I.iext p i j → False
  /-- cls-maxc1. A `≤0` restriction has no member with the property. -/
  maxc1 : ∀ c p, I.iext (I.ι RV.maxCardinality) c (I.ι RV.zero) →
    I.iext (I.ι V.onProperty) c p → ∀ u v, I.cext c u → I.iext p u v → False
  /-- cls-maxqc1. The qualified `≤0`, with the filler checked. -/
  maxqc1 : ∀ c p k, I.iext (I.ι RV.maxQualifiedCardinality) c (I.ι RV.zero) →
    I.iext (I.ι V.onProperty) c p → I.iext (I.ι RV.onClass) c k →
    ∀ u v, I.cext c u → I.iext p u v → I.cext k v → False
  /-- cls-maxqc2. The qualified `≤0` onto `owl:Thing`, where the filler is free. -/
  maxqc2 : ∀ c p, I.iext (I.ι RV.maxQualifiedCardinality) c (I.ι RV.zero) →
    I.iext (I.ι V.onProperty) c p → I.iext (I.ι RV.onClass) c (I.ι RV.thing) →
    ∀ u v, I.cext c u → I.iext p u v → False

/-- A model of `G` that ALSO respects disjointness. Named apart from `Model` so
that no verdict can quietly move between the two classes. -/
def RModel (G : List Triple) (I : Interp) : Prop := Model I G ∧ RefuteConditions I

/-- `G` has no model that respects disjointness.

Relative, not absolute: see the module docstring. `Model I G` on its own is
satisfiable for every graph, by `saturated`. -/
def Unsat (G : List Triple) : Prop := ∀ I : Interp, ¬ RModel G I

theorem RModel.model {G : List Triple} {I : Interp} (h : RModel G I) : Model I G := h.1

/-! ## The refutation step -/

/-- The clash rules, one constructor per rule implemented. One so far. -/
inductive RefuteRule
  | caxDw | clsCom | clsNothing2 | prpIrp | prpAsyp | prpPdw | eqDiff1
  | prpNpa1 | prpNpa2 | clsMaxc1 | clsMaxqc1 | clsMaxqc2
deriving DecidableEq, Repr

def RefuteRule.name : RefuteRule → String
  | .caxDw => "cax-dw"
  | .clsCom => "cls-com"
  | .clsNothing2 => "cls-nothing2"
  | .prpIrp => "prp-irp"
  | .prpAsyp => "prp-asyp"
  | .prpPdw => "prp-pdw"
  | .eqDiff1 => "eq-diff1"
  | .prpNpa1 => "prp-npa1"
  | .prpNpa2 => "prp-npa2"
  | .clsMaxc1 => "cls-maxc1"
  | .clsMaxqc1 => "cls-maxqc1"
  | .clsMaxqc2 => "cls-maxqc2"

def RefuteRule.all : List RefuteRule :=
  [.caxDw, .clsCom, .clsNothing2, .prpIrp, .prpAsyp, .prpPdw, .eqDiff1,
   .prpNpa1, .prpNpa2, .clsMaxc1, .clsMaxqc1, .clsMaxqc2]

def RefuteRule.ofName? (s : String) : Option RefuteRule :=
  RefuteRule.all.find? (fun r => r.name == s)

/-- The final step of a refutation. It has premises and no conclusion, because
what it concludes is contradiction.

`cax-dw` premise order, and the checker matches on it: `c1 owl:disjointWith c2`,
`x rdf:type c1`, `x rdf:type c2`. -/
structure RefuteStep where
  rule : RefuteRule
  premises : List Triple
deriving Repr

/-- A refutation: a derivation prefix in the ordinary format, then one
contradiction step. The prefix is what lets the disjoint types be derived rather
than asserted, which is the normal case in a real ontology. -/
structure Refutation where
  steps : List Step
  final : RefuteStep
deriving Repr

/-! ## The checker

`checkAll` in `Rules.lean` threads its derived set through a `Std.HashSet`,
whose `contains` the Lean kernel cannot evaluate: `String.hash` is `opaque`, so
`decide` cannot run it. `checkAllL` is the same function over a list, which the
kernel can run, so the witness file can settle acceptance and rejection by
`decide` instead of by assertion. Both are proved sound, and the CLI uses the
hashed one because it is the one that scales. -/

/-- `checkAll` with a list for the derived set. Quadratic, and the kernel can
run it. -/
def checkAllL (inG : Triple → Bool) : List Step → List Triple → Bool
  | [], _ => true
  | st :: rest, derived =>
      checkStep inG (fun t => decide (t ∈ derived)) st &&
      checkAllL inG rest (st.conclusion :: derived)

/-- Check the contradiction step. `k` answers "asserted, or concluded by the
prefix". Premise order is part of the contract, as it is for every other rule in
this checker: a refutation with the right triples in the wrong order is
rejected, which is a false alarm and never a false pass. -/
def checkRefuteStep (k : Triple → Bool) (rs : RefuteStep) : Bool :=
  match rs.rule, rs.premises with
  | .caxDw, [⟨c1, dw, c2⟩, ⟨x, t1, c1'⟩, ⟨x', t2, c2'⟩] =>
      dw = RV.disjointWith ∧ t1 = V.type ∧ c1' = c1 ∧
      x' = x ∧ t2 = V.type ∧ c2' = c2 ∧
      k ⟨c1, dw, c2⟩ ∧ k ⟨x, t1, c1'⟩ ∧ k ⟨x', t2, c2'⟩
  | .clsCom, [⟨c1, co, c2⟩, ⟨x, t1, c1'⟩, ⟨x', t2, c2'⟩] =>
      co = RV.complementOf ∧ t1 = V.type ∧ c1' = c1 ∧
      x' = x ∧ t2 = V.type ∧ c2' = c2 ∧
      k ⟨c1, co, c2⟩ ∧ k ⟨x, t1, c1'⟩ ∧ k ⟨x', t2, c2'⟩
  | .clsNothing2, [⟨x, t1, n⟩] =>
      t1 = V.type ∧ n = RV.nothing ∧ k ⟨x, t1, n⟩
  | .prpIrp, [⟨p, t1, irp⟩, ⟨x, p', x'⟩] =>
      t1 = V.type ∧ irp = RV.irreflexiveProperty ∧ p' = p ∧ x' = x ∧
      k ⟨p, t1, irp⟩ ∧ k ⟨x, p', x'⟩
  | .prpAsyp, [⟨p, t1, asy⟩, ⟨x, p1, y⟩, ⟨y', p2, x'⟩] =>
      t1 = V.type ∧ asy = RV.asymmetricProperty ∧ p1 = p ∧ p2 = p ∧
      y' = y ∧ x' = x ∧
      k ⟨p, t1, asy⟩ ∧ k ⟨x, p1, y⟩ ∧ k ⟨y', p2, x'⟩
  | .prpPdw, [⟨p1, pdw, p2⟩, ⟨x, q1, y⟩, ⟨x', q2, y'⟩] =>
      pdw = RV.propertyDisjointWith ∧ q1 = p1 ∧ q2 = p2 ∧ x' = x ∧ y' = y ∧
      k ⟨p1, pdw, p2⟩ ∧ k ⟨x, q1, y⟩ ∧ k ⟨x', q2, y'⟩
  | .eqDiff1, [⟨x, sa, y⟩, ⟨x', df, y'⟩] =>
      sa = V.sameAs ∧ df = RV.differentFrom ∧ x' = x ∧ y' = y ∧
      k ⟨x, sa, y⟩ ∧ k ⟨x', df, y'⟩
  | .prpNpa1, [⟨n, si, i⟩, ⟨n1, ap, p⟩, ⟨n2, ti, j⟩, ⟨i', p', j'⟩] =>
      si = RV.sourceIndividual ∧ ap = RV.assertionProperty ∧
      ti = RV.targetIndividual ∧ n1 = n ∧ n2 = n ∧ i' = i ∧ p' = p ∧ j' = j ∧
      k ⟨n, si, i⟩ ∧ k ⟨n1, ap, p⟩ ∧ k ⟨n2, ti, j⟩ ∧ k ⟨i', p', j'⟩
  | .prpNpa2, [⟨n, si, i⟩, ⟨n1, ap, p⟩, ⟨n2, tv, j⟩, ⟨i', p', j'⟩] =>
      si = RV.sourceIndividual ∧ ap = RV.assertionProperty ∧
      tv = RV.targetValue ∧ n1 = n ∧ n2 = n ∧ i' = i ∧ p' = p ∧ j' = j ∧
      k ⟨n, si, i⟩ ∧ k ⟨n1, ap, p⟩ ∧ k ⟨n2, tv, j⟩ ∧ k ⟨i', p', j'⟩
  | .clsMaxc1, [⟨c, mc, z⟩, ⟨c1, op, p⟩, ⟨u, t1, c2⟩, ⟨u', p', v⟩] =>
      mc = RV.maxCardinality ∧ z = RV.zero ∧ op = V.onProperty ∧ c1 = c ∧
      t1 = V.type ∧ c2 = c ∧ u' = u ∧ p' = p ∧
      k ⟨c, mc, z⟩ ∧ k ⟨c1, op, p⟩ ∧ k ⟨u, t1, c2⟩ ∧ k ⟨u', p', v⟩
  | .clsMaxqc1, [⟨c, mq, z⟩, ⟨c1, op, p⟩, ⟨c2, oc, f⟩, ⟨u, t1, c3⟩, ⟨u', p', v⟩,
      ⟨v', t2, f'⟩] =>
      mq = RV.maxQualifiedCardinality ∧ z = RV.zero ∧ op = V.onProperty ∧
      oc = RV.onClass ∧ c1 = c ∧ c2 = c ∧ t1 = V.type ∧ c3 = c ∧ u' = u ∧
      p' = p ∧ v' = v ∧ t2 = V.type ∧ f' = f ∧
      k ⟨c, mq, z⟩ ∧ k ⟨c1, op, p⟩ ∧ k ⟨c2, oc, f⟩ ∧ k ⟨u, t1, c3⟩ ∧
      k ⟨u', p', v⟩ ∧ k ⟨v', t2, f'⟩
  | .clsMaxqc2, [⟨c, mq, z⟩, ⟨c1, op, p⟩, ⟨c2, oc, th⟩, ⟨u, t1, c3⟩, ⟨u', p', v⟩] =>
      mq = RV.maxQualifiedCardinality ∧ z = RV.zero ∧ op = V.onProperty ∧
      oc = RV.onClass ∧ th = RV.thing ∧ c1 = c ∧ c2 = c ∧ t1 = V.type ∧
      c3 = c ∧ u' = u ∧ p' = p ∧
      k ⟨c, mq, z⟩ ∧ k ⟨c1, op, p⟩ ∧ k ⟨c2, oc, th⟩ ∧ k ⟨u, t1, c3⟩ ∧
      k ⟨u', p', v⟩
  | _, _ => false

/-- The conclusions the prefix made available to the contradiction step. A
premise of the final step may be asserted or derived by the prefix; it may not
be assumed. -/
def prefixConclusions (r : Refutation) : List Triple := r.steps.map Step.conclusion

/-- The checker, with the "is asserted" test left abstract so that the list
version and the hashed version are the same function. -/
def checkRefutationWith (inG : Triple → Bool) (r : Refutation) : Bool :=
  checkAllL inG r.steps [] &&
  checkRefuteStep (fun t => inG t || decide (t ∈ prefixConclusions r)) r.final

/-- The kernel-runnable checker. `decide (t ∈ G)` walks the list, so this one is
linear in the size of the asserted graph per premise. The witness file uses it
because the kernel can run it; a caller with a real ontology should not. -/
def checkRefutation (G : List Triple) (r : Refutation) : Bool :=
  checkRefutationWith (fun t => decide (t ∈ G)) r

/-- The checker the CLI runs. Exactly what is hashed, and nothing more: the
ASSERTED graph, which is the part that is large. The derivation prefix is still
scanned as a list, so the check is quadratic in the NUMBER OF STEPS in the
refutation. That is deliberate rather than overlooked. A refutation carries only
the steps needed to reach one contradiction, which is a handful of `rdfs9` hops
down a subclass chain, and buying a hash table for a list of four would cost
more than it saves. A caller who writes thousand-step refutations will notice,
and should say so. -/
def checkRefutationFast (G : List Triple) (r : Refutation) : Bool :=
  let gset := Std.HashSet.ofList G
  checkRefutationWith (fun t => gset.contains t) r

/-! ## Soundness -/

theorem checkAllL_sound {G : List Triple} {P : Interp → Prop} {inG : Triple → Bool}
    (hP : ∀ I, P I → Model I G) (hG : ∀ t, inG t = true → t ∈ G) :
    ∀ (steps : List Step) (derived : List Triple),
      (∀ t ∈ derived, EntailsIn P t) →
      checkAllL inG steps derived = true →
      ∀ st ∈ steps, EntailsIn P st.conclusion := by
  intro steps
  induction steps with
  | nil =>
    intro _ _ _ st hst
    simp at hst
  | cons st rest ih =>
    intro derived hD h st' hst'
    simp only [checkAllL, Bool.and_eq_true] at h
    obtain ⟨h1, h2⟩ := h
    have hD' : ∀ t, decide (t ∈ derived) = true → EntailsIn P t := by
      intro t ht
      exact hD t (of_decide_eq_true ht)
    have hst : EntailsIn P st.conclusion := checkStep_sound hP hG hD' h1
    have hD'' : ∀ t ∈ st.conclusion :: derived, EntailsIn P t := by
      intro t ht
      rcases List.mem_cons.mp ht with rfl | ht
      · exact hst
      · exact hD t ht
    rcases List.mem_cons.mp hst' with rfl | hmem
    · exact hst
    · exact ih _ hD'' h2 st' hmem

/-- The contradiction step, discharged against `RefuteConditions.dw`. -/
theorem checkRefuteStep_sound {G : List Triple} {k : Triple → Bool} {rs : RefuteStep}
    (hk : ∀ t, k t = true → EntailsIn (RModel G) t)
    (h : checkRefuteStep k rs = true) : Unsat G := by
  obtain ⟨rule, premises⟩ := rs
  unfold checkRefuteStep at h
  dsimp only at h
  -- One case per rule, and every one is the same two moves: take the decided
  -- side conditions apart, then hand the premises to the condition of
  -- `RefuteConditions` that has exactly that shape. No case reasons about the
  -- semantics; the reasoning is in the condition.
  cases rule
  case caxDw =>
    rcases premises with
      _ | ⟨⟨c1, dw, c2⟩, _ | ⟨⟨x, t1, c1'⟩, _ | ⟨⟨x', t2, c2'⟩, _ | _⟩⟩⟩ <;>
      simp only [decide_eq_true_eq, Bool.false_eq_true] at h
    obtain ⟨rfl, rfl, rfl, rfl, rfl, rfl, h1, h2, h3⟩ := h
    intro I hI
    exact hI.2.dw _ _ (hk _ h1 I hI) _ (hk _ h2 I hI) (hk _ h3 I hI)
  case clsCom =>
    rcases premises with
      _ | ⟨⟨c1, co, c2⟩, _ | ⟨⟨x, t1, c1'⟩, _ | ⟨⟨x', t2, c2'⟩, _ | _⟩⟩⟩ <;>
      simp only [decide_eq_true_eq, Bool.false_eq_true] at h
    obtain ⟨rfl, rfl, rfl, rfl, rfl, rfl, h1, h2, h3⟩ := h
    intro I hI
    exact hI.2.com _ _ (hk _ h1 I hI) _ (hk _ h2 I hI) (hk _ h3 I hI)
  case clsNothing2 =>
    rcases premises with _ | ⟨⟨x, t1, n⟩, _ | _⟩ <;>
      simp only [decide_eq_true_eq, Bool.false_eq_true] at h
    obtain ⟨rfl, rfl, h1⟩ := h
    intro I hI
    exact hI.2.nothing _ (hk _ h1 I hI)
  case prpIrp =>
    rcases premises with _ | ⟨⟨p, t1, irp⟩, _ | ⟨⟨x, p', x'⟩, _ | _⟩⟩ <;>
      simp only [decide_eq_true_eq, Bool.false_eq_true] at h
    obtain ⟨rfl, rfl, rfl, rfl, h1, h2⟩ := h
    intro I hI
    exact hI.2.irp _ (hk _ h1 I hI) _ (hk _ h2 I hI)
  case prpAsyp =>
    rcases premises with
      _ | ⟨⟨p, t1, asy⟩, _ | ⟨⟨x, p1, y⟩, _ | ⟨⟨y', p2, x'⟩, _ | _⟩⟩⟩ <;>
      simp only [decide_eq_true_eq, Bool.false_eq_true] at h
    obtain ⟨rfl, rfl, rfl, rfl, rfl, rfl, h1, h2, h3⟩ := h
    intro I hI
    exact hI.2.asyp _ (hk _ h1 I hI) _ _ (hk _ h2 I hI) (hk _ h3 I hI)
  case prpPdw =>
    rcases premises with
      _ | ⟨⟨p1, pdw, p2⟩, _ | ⟨⟨x, q1, y⟩, _ | ⟨⟨x', q2, y'⟩, _ | _⟩⟩⟩ <;>
      simp only [decide_eq_true_eq, Bool.false_eq_true] at h
    obtain ⟨rfl, rfl, rfl, rfl, rfl, h1, h2, h3⟩ := h
    intro I hI
    exact hI.2.pdw _ _ (hk _ h1 I hI) _ _ (hk _ h2 I hI) (hk _ h3 I hI)
  case eqDiff1 =>
    rcases premises with _ | ⟨⟨x, sa, y⟩, _ | ⟨⟨x', df, y'⟩, _ | _⟩⟩ <;>
      simp only [decide_eq_true_eq, Bool.false_eq_true] at h
    obtain ⟨rfl, rfl, rfl, rfl, h1, h2⟩ := h
    intro I hI
    exact hI.2.diff _ _ (hk _ h1 I hI) (hk _ h2 I hI)
  case prpNpa1 =>
    rcases premises with
      _ | ⟨⟨n, si, i⟩, _ | ⟨⟨n1, ap, p⟩, _ | ⟨⟨n2, ti, j⟩, _ | ⟨⟨i', p', j'⟩, _ | _⟩⟩⟩⟩ <;>
      simp only [decide_eq_true_eq, Bool.false_eq_true] at h
    obtain ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, h1, h2, h3, h4⟩ := h
    intro I hI
    exact hI.2.npa1 _ _ _ _ (hk _ h1 I hI) (hk _ h2 I hI) (hk _ h3 I hI) (hk _ h4 I hI)
  case prpNpa2 =>
    rcases premises with
      _ | ⟨⟨n, si, i⟩, _ | ⟨⟨n1, ap, p⟩, _ | ⟨⟨n2, tv, j⟩, _ | ⟨⟨i', p', j'⟩, _ | _⟩⟩⟩⟩ <;>
      simp only [decide_eq_true_eq, Bool.false_eq_true] at h
    obtain ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, h1, h2, h3, h4⟩ := h
    intro I hI
    exact hI.2.npa2 _ _ _ _ (hk _ h1 I hI) (hk _ h2 I hI) (hk _ h3 I hI) (hk _ h4 I hI)
  case clsMaxc1 =>
    rcases premises with
      _ | ⟨⟨c, mc, z⟩, _ | ⟨⟨c1, op, p⟩, _ | ⟨⟨u, t1, c2⟩, _ | ⟨⟨u', p', v⟩, _ | _⟩⟩⟩⟩ <;>
      simp only [decide_eq_true_eq, Bool.false_eq_true] at h
    obtain ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, h1, h2, h3, h4⟩ := h
    intro I hI
    exact hI.2.maxc1 _ _ (hk _ h1 I hI) (hk _ h2 I hI) _ _ (hk _ h3 I hI) (hk _ h4 I hI)
  case clsMaxqc1 =>
    rcases premises with
      _ | ⟨⟨c, mq, z⟩, _ | ⟨⟨c1, op, p⟩, _ | ⟨⟨c2, oc, f⟩, _ | ⟨⟨u, t1, c3⟩, _ |
        ⟨⟨u', p', v⟩, _ | ⟨⟨v', t2, f'⟩, _ | _⟩⟩⟩⟩⟩⟩ <;>
      simp only [decide_eq_true_eq, Bool.false_eq_true] at h
    obtain ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl,
      h1, h2, h3, h4, h5, h6⟩ := h
    intro I hI
    exact hI.2.maxqc1 _ _ _ (hk _ h1 I hI) (hk _ h2 I hI) (hk _ h3 I hI) _ _
      (hk _ h4 I hI) (hk _ h5 I hI) (hk _ h6 I hI)
  case clsMaxqc2 =>
    rcases premises with
      _ | ⟨⟨c, mq, z⟩, _ | ⟨⟨c1, op, p⟩, _ | ⟨⟨c2, oc, th⟩, _ | ⟨⟨u, t1, c3⟩, _ |
        ⟨⟨u', p', v⟩, _ | _⟩⟩⟩⟩⟩ <;>
      simp only [decide_eq_true_eq, Bool.false_eq_true] at h
    obtain ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl,
      h1, h2, h3, h4, h5⟩ := h
    intro I hI
    exact hI.2.maxqc2 _ _ (hk _ h1 I hI) (hk _ h2 I hI) (hk _ h3 I hI) _ _
      (hk _ h4 I hI) (hk _ h5 I hI)

/-- **The theorem the refutation checker is trusted for.** An accepted
refutation means no interpretation models `G` and respects disjointness.

Conditional on exactly one thing, and it is named in the statement rather than
in a footnote: the reading of `owl:disjointWith` given by `RefuteConditions`.
Drop that reading and the graph has models again, `saturated` among them. -/
theorem refutation_sound_with {G : List Triple} {inG : Triple → Bool}
    (hG : ∀ t, inG t = true → t ∈ G) (r : Refutation)
    (h : checkRefutationWith inG r = true) : Unsat G := by
  unfold checkRefutationWith at h
  simp only [Bool.and_eq_true] at h
  obtain ⟨h1, h2⟩ := h
  have hsteps : ∀ st ∈ r.steps, EntailsIn (RModel G) st.conclusion :=
    checkAllL_sound (P := RModel G) (fun _ hI => hI.1) hG r.steps [] (by intro t ht; simp at ht) h1
  refine checkRefuteStep_sound (k := fun t => inG t || decide (t ∈ prefixConclusions r)) ?_ h2
  intro t ht
  simp only [Bool.or_eq_true, decide_eq_true_eq] at ht
  rcases ht with hx | hx
  · exact fun I hI => hI.1.facts t (hG t hx)
  · obtain ⟨st, hst, rfl⟩ := List.mem_map.mp hx
    exact hsteps st hst

/-- The kernel-runnable checker is sound. -/
theorem refutation_sound (G : List Triple) (r : Refutation)
    (h : checkRefutation G r = true) : Unsat G :=
  refutation_sound_with (fun _ ht => of_decide_eq_true ht) r h

/-- The hashed checker the CLI runs is sound, by the same theorem. -/
theorem refutation_fast_sound (G : List Triple) (r : Refutation)
    (h : checkRefutationFast G r = true) : Unsat G := by
  refine refutation_sound_with (inG := fun t => (Std.HashSet.ofList G).contains t) ?_ r h
  intro t ht
  rw [Std.HashSet.contains_ofList] at ht
  simpa using ht

/-! ## What a refutation costs the triple certificates over the same graph -/

/-- Once `G` is refuted, every triple has the disjointness-aware warrant. -/
theorem unsat_entails_everything {G : List Triple} (h : Unsat G) (t : Triple) :
    EntailsIn (RModel G) t := fun I hI => absurd hI (h I)

/-- **Why `oo-refute guard` refuses.** Over a refuted graph, EVERY conclusion of
EVERY list of steps already has the disjointness-aware warrant, whether the
checker accepted the certificate or not, whether the steps are derivations or
forgeries. Running the derivation checker on such a graph therefore establishes
nothing that was not already true of an empty file. -/
theorem a_certificate_adds_nothing_when_the_graph_is_refuted {G : List Triple}
    (h : Unsat G) (steps : List Step) :
    ∀ st ∈ steps, EntailsIn (RModel G) st.conclusion :=
  fun st _ => unsat_entails_everything h st.conclusion

/-! ## The replacement for the lost witness, in its general form

`saturated_is_a_model` is not available here, so the non-vacuity of
`refutation_sound` has to be re-earned. If nothing ever satisfied `RModel G`,
`Unsat G` would hold for every graph and the theorem above would say nothing.

`joint_model_of_closure` is the general shape of the answer, and it is stated
without mentioning any particular condition of `Model`, so it does not move when
`Semantics.lean` gains a rule: hand it ANY triple-predicate `D` whose Herbrand
interpretation models `G`, plus the fact that `D` contains no disjointness
violation, and it hands back a joint model. `RefuteWitness.lean` supplies such a
`D` for an arbitrary graph, by induction, and discharges it concretely. -/

/-- Terms denote themselves and a property relates exactly the pairs the
predicate `D` holds of. The list-valued `herbrand` of `Witness.lean` is the
special case `D t := t ∈ H`, and the predicate version is what lets the closure
of an infinite or inductively defined set be used. -/
def herbrandP (D : Triple → Prop) : Interp where
  D := Term
  ι := id
  iext := fun p x y => D ⟨x, p, y⟩

@[simp] theorem herbrandP_iext (D : Triple → Prop) (p x y : Term) :
    (herbrandP D).iext p x y ↔ D ⟨x, p, y⟩ := Iff.rfl

@[simp] theorem herbrandP_sat (D : Triple → Prop) (t : Triple) :
    (herbrandP D).sat t ↔ D t := Iff.rfl

@[simp] theorem herbrandP_cext (D : Triple → Prop) (c x : Term) :
    (herbrandP D).cext c x ↔ D ⟨x, V.type, c⟩ := Iff.rfl

/-- The finite-list special case. This is `Witness.lean`'s `herbrand` with the
membership predicate written through `herbrandP`, definitionally the same
interpretation, and it is spelled again here so that this layer's witnesses
depend on `Semantics.lean` and nothing else. -/
def herbrandL (H : List Triple) : Interp := herbrandP (fun t => t ∈ H)

@[simp] theorem herbrandL_sat (H : List Triple) (t : Triple) :
    (herbrandL H).sat t ↔ t ∈ H := Iff.rfl

@[simp] theorem herbrandL_cext (H : List Triple) (c x : Term) :
    (herbrandL H).cext c x ↔ (⟨x, V.type, c⟩ : Triple) ∈ H := Iff.rfl

/-- **A closure with no disjointness violation is a joint model.** -/
theorem joint_model_of_closure {G : List Triple} {D : Triple → Prop}
    (hD : Model (herbrandP D) G)
    (hdw : ∀ c1 c2 x, D ⟨c1, RV.disjointWith, c2⟩ → D ⟨x, V.type, c1⟩ →
      D ⟨x, V.type, c2⟩ → False)
    (hcom : ∀ c1 c2 x, D ⟨c1, RV.complementOf, c2⟩ → D ⟨x, V.type, c1⟩ →
      D ⟨x, V.type, c2⟩ → False)
    (hnothing : ∀ x, D ⟨x, V.type, RV.nothing⟩ → False)
    (hirp : ∀ p x, D ⟨p, V.type, RV.irreflexiveProperty⟩ → D ⟨x, p, x⟩ → False)
    (hasyp : ∀ p x y, D ⟨p, V.type, RV.asymmetricProperty⟩ → D ⟨x, p, y⟩ →
      D ⟨y, p, x⟩ → False)
    (hpdw : ∀ p q x y, D ⟨p, RV.propertyDisjointWith, q⟩ → D ⟨x, p, y⟩ →
      D ⟨x, q, y⟩ → False)
    (hdiff : ∀ x y, D ⟨x, V.sameAs, y⟩ → D ⟨x, RV.differentFrom, y⟩ → False)
    (hnpa1 : ∀ n i p j, D ⟨n, RV.sourceIndividual, i⟩ →
      D ⟨n, RV.assertionProperty, p⟩ → D ⟨n, RV.targetIndividual, j⟩ →
      D ⟨i, p, j⟩ → False)
    (hnpa2 : ∀ n i p j, D ⟨n, RV.sourceIndividual, i⟩ →
      D ⟨n, RV.assertionProperty, p⟩ → D ⟨n, RV.targetValue, j⟩ →
      D ⟨i, p, j⟩ → False)
    (hmaxc1 : ∀ c p u v, D ⟨c, RV.maxCardinality, RV.zero⟩ →
      D ⟨c, V.onProperty, p⟩ → D ⟨u, V.type, c⟩ → D ⟨u, p, v⟩ → False)
    (hmaxqc1 : ∀ c p f u v, D ⟨c, RV.maxQualifiedCardinality, RV.zero⟩ →
      D ⟨c, V.onProperty, p⟩ → D ⟨c, RV.onClass, f⟩ → D ⟨u, V.type, c⟩ →
      D ⟨u, p, v⟩ → D ⟨v, V.type, f⟩ → False)
    (hmaxqc2 : ∀ c p u v, D ⟨c, RV.maxQualifiedCardinality, RV.zero⟩ →
      D ⟨c, V.onProperty, p⟩ → D ⟨c, RV.onClass, RV.thing⟩ → D ⟨u, V.type, c⟩ →
      D ⟨u, p, v⟩ → False) :
    ∃ I : Interp, Model I G ∧ RefuteConditions I :=
  ⟨herbrandP D, hD,
   ⟨fun a b hab x h1 h2 => hdw a b x hab h1 h2,
    fun a b hab x h1 h2 => hcom a b x hab h1 h2,
    fun x hx => hnothing x hx,
    fun p hp x hx => hirp p x hp hx,
    fun p hp x y h1 h2 => hasyp p x y hp h1 h2,
    fun p q hpq x y h1 h2 => hpdw p q x y hpq h1 h2,
    fun x y h1 h2 => hdiff x y h1 h2,
    fun n i p j h1 h2 h3 h4 => hnpa1 n i p j h1 h2 h3 h4,
    fun n i p j h1 h2 h3 h4 => hnpa2 n i p j h1 h2 h3 h4,
    fun c p h1 h2 u v h3 h4 => hmaxc1 c p u v h1 h2 h3 h4,
    fun c p f h1 h2 h3 u v h4 h5 h6 => hmaxqc1 c p f u v h1 h2 h3 h4 h5 h6,
    fun c p h1 h2 h3 u v h4 h5 => hmaxqc2 c p u v h1 h2 h3 h4 h5⟩⟩

/-- A graph with a joint model is not refutable, so `refutation_sound` is not a
theorem about an empty model class.

**Relative to `Conditions` and `RefuteConditions`, and it is not itself the
statement over the OWL 2 RDF-Based Semantics conditions.** This note is here rather than only in
`Semantics.lean` because a caveat that lives in one file is a caveat that goes
stale; until 15 September 2026 this one did, and this file was byte-identical to
the version that predates `lean/OOCert/W3C.lean`.

The direction is what stops it. `Unsat G` quantifies negatively over a model
class, so shrinking the class to `W3CModel` makes `Unsat` EASIER and `¬ Unsat`
HARDER. The consequence runs the useful way for the positive result and the
useless way for this one: `refutation_sound`'s `Unsat G` does transfer, because
every `W3CModel` of `G` is a `Model` of `G`, so an accepted refutation rules out
conforming interpretations too. A `¬ Unsat` needs a `W3CModel` that also
satisfies `RefuteConditions`, and neither this lemma nor
`RefuteWitness.lean`'s `feed_is_not_refuted` supplies one.

`RefuteWitness.lean` now does supply one, and this lemma is the shape it takes
over there: `W3CUnsat` is `Unsat` over the smaller class, `w3cUnsat_of_unsat`
is the transfer that does run, `not_w3cUnsat_of_joint_w3c_model` is this lemma's
counterpart, and `feed_is_not_w3c_refuted` is the result. The obstruction that
used to be recorded here, that `feedClosure` types nothing as an `rdfs:Class` and
so `W3C.sc_fwd` fails on it, was a fact about the HERBRAND witness and not about
the statement; a thirteen-element model built for it settles the statement. -/
theorem not_unsat_of_joint_model {G : List Triple}
    (h : ∃ I : Interp, Model I G ∧ RefuteConditions I) : ¬ Unsat G := by
  obtain ⟨I, hM, hR⟩ := h
  intro hu
  exact hu I ⟨hM, hR⟩

/-! ## Reading a refutation file

Not part of the proof. A parse error is a rejected file, never an accepted
refutation. -/
namespace RefuteParse

def version : String := "oo-refute/1"

/-- The version marker, as it may arrive. A file written on Windows ends its
lines with a carriage return and the marker would then never match. Nothing
else in the format is trimmed: a term's spelling is its identity, and stripping
whitespace from one would be a silent rewrite of the certificate. -/
def isVersionLine (s : String) : Bool := s == version || s == version ++ "\r"

/-- Three tab fields at a time into triples, as `Parse.groupTriples` does for
the derivation format. -/
def groupTriples : List String → Option (List Triple)
  | [] => some []
  | s :: p :: o :: rest => (groupTriples rest).map (⟨s, p, o⟩ :: ·)
  | _ => none

/-- Parse one contradiction line, already split on tabs and with the leading
`refute` removed. An unimplemented clash rule gets its own message: naming it
"bad syntax" would hide the real answer, which is that the checker has no
condition for that rule and cannot judge a refutation citing it. -/
def refuteStepOf (fields : List String) : Except String RefuteStep :=
  match fields with
  | rule :: rest =>
      match RefuteRule.ofName? rule with
      | none =>
          .error s!"unknown or unimplemented clash rule '{rule}'. This checker implements \
            {String.intercalate ", " (RefuteRule.all.map RefuteRule.name)} and nothing else; \
            the other OWL 2 RL clash rules have no semantic condition here, so a refutation \
            citing one cannot be judged and is refused rather than guessed at."
      | some r =>
          match groupTriples rest with
          | some ps => .ok ⟨r, ps⟩
          | none => .error "the premises of the contradiction step are not whole triples"
  | _ => .error "the contradiction step names no rule"

def stepOf? (fields : List String) : Option Step :=
  match fields with
  | rule :: s :: p :: o :: prem =>
      match Rule.ofName? rule, groupTriples prem with
      | some r, some ps => some ⟨r, ps, ⟨s, p, o⟩⟩
      | _, _ => none
  | _ => none

/-- `oo-refute/1`, then derivation steps, then exactly one `refute` line, last. -/
def parseRefutation (content : String) : Except String Refutation := do
  let lines := (content.splitOn "\n").filter (fun l => !l.isEmpty)
  match lines with
  | [] => throw "refutation: the file is empty; expected the line 'oo-refute/1'"
  | header :: body =>
    if !isVersionLine header then
      throw s!"refutation: first line is '{header}', expected '{version}'. \
        A derivations.tsv is not a refutation and is refused here on purpose."
    let mut steps : Array Step := #[]
    let mut final : Option RefuteStep := none
    let mut n := 1
    for line in body do
      n := n + 1
      if final.isSome then
        throw s!"refutation line {n}: the contradiction step must be the last line"
      match line.splitOn "\t" with
      | "refute" :: rest =>
        match refuteStepOf rest with
        | .ok rs => final := some rs
        | .error e => throw s!"refutation line {n}: {e}"
      | fields =>
        match stepOf? fields with
        | some st => steps := steps.push st
        | none => throw s!"refutation line {n}: bad derivation step; expected \
            'rule TAB s TAB p TAB o' then premises in whole triples"
    match final with
    | some rs => return ⟨steps.toList, rs⟩
    | none => throw "refutation: no 'refute' line. A refutation must reach a \
        contradiction; a file of derivations alone proves nothing here."

end RefuteParse

/-! ## Axioms, pinned

The same tripwire `Soundness.lean` and `Horn.lean` carry. A `sorry` or a
`native_decide` under any of these fails the build here. `joint_model_of_closure`
needs neither choice nor quotients: it is a construction.

The list checker needs no choice. The hashed one does, and only because
`Std.HashSet.contains_ofList` does. A SHORTER footprint than the house one is
the safe direction; a longer one is the alarm. -/

/-- info: 'OOCert.refutation_sound' depends on axioms: [propext, Quot.sound] -/
#guard_msgs in
#print axioms refutation_sound

/-- info: 'OOCert.refutation_fast_sound' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms refutation_fast_sound

/-- info: 'OOCert.a_certificate_adds_nothing_when_the_graph_is_refuted' does not depend on any axioms -/
#guard_msgs in
#print axioms a_certificate_adds_nothing_when_the_graph_is_refuted

/-- info: 'OOCert.joint_model_of_closure' does not depend on any axioms -/
#guard_msgs in
#print axioms joint_model_of_closure

end OOCert
