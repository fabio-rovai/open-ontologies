import OOCert.W3C
import OOCert.Witness

/-!
# A LIVE `W3CModel`, and the non-entailments that are about the specification

`W3C.lean` derives fourteen arms from quoted specification cells. That result is
worth nothing until something exhibits a `W3CModel`, and it is worth very little
if the thing that exhibits one is degenerate. This file builds a model that is
not, and it collects every non-entailment in the repository that is about the
`W3CModel` class rather than about `Semantics.lean`'s weaker `Conditions`.

## Why `saturated` is not enough

`Witness.lean`'s `saturated` (domain `Unit`, every relation total) satisfies
`W3C` with `IP := fun _ => True`: every field holds because both sides of every
equality are all of `Unit`. So bare satisfiability of `W3C` is one theorem and
it says nothing, because such a model distinguishes no condition from any other
and refutes nothing. `W3CModel.toModel` proved over an unsatisfiable or a
degenerate `W3C` would be a theorem about nothing that LOOKED better than the
assumption it replaced, which is a strictly worse defect than the one being
repaired. The gate is therefore a NON-DEGENERATE model, and `live_is_live`,
`live_exercises_every_arm` and `live_fires_every_field` below compile the
liveness facts and their limits into the build, so that the next person to
shorten this graph to fix a `decide` timeout breaks it rather than hollowing it
out silently.

## The first version of this file was vacuous where it mattered most, and this is the repair

`live` had seventeen elements, `IEXT(p1)` was EMPTY and `ICEXT(Y)` was EMPTY,
and both emptinesses were written into the liveness gate as though they were
features of the model rather than holes in it. Two consequences, in ascending
order of seriousness.

Nine of the twenty-one fields of `W3C` held because nothing was in the extension
their antecedent reads, and SIX of the fourteen arms rested entirely on those
nine: `scm-eqc1` and `scm-eqp1` with two conclusions each, on `eqc_fwd` and
`eqp_fwd`; `scm-svf1` and `scm-svf2`, on `svf_eq` and `svf_typ`.
`owl:equivalentClass`, `owl:equivalentProperty`, `owl:someValuesFrom`,
`owl:hasValue`, `owl:sameAs`, `owl:inverseOf`, `owl:SymmetricProperty` and
`owl:TransitiveProperty` all denoted the same junk element, whose extensions were
empty.

Worse, the refutation the file exists to deliver was vacuous at its own
load-bearing premise. `avfPremises` asserts `p1 rdfs:subPropertyOf p2`, and in a
model with `IEXT(p1)` empty that triple holds because `sp_bwd`'s inclusion clause
has nothing to check. A refutation whose premise is satisfied for want of
anything to test establishes almost nothing, and the emptiness of `IEXT(p1)` was
a conjunct of `live_is_live`, which is the theorem whose job is to catch exactly
that.

The model below is the repair. `carl` is a third individual: `IEXT(p1)` is
`{(alice, carl)}` and `ICEXT(Y)` is `{carl}`, so `IEXT(p1)` is non-empty and a
PROPER subset of `IEXT(p2)`, and `p1 rdfs:subPropertyOf p2` holds here for a
reason. `A2`, `S1`, `S2`, `S3` and `q` are five further elements carrying the
`owl:allValuesFrom`, `owl:someValuesFrom`, `owl:equivalentClass` and
`owl:equivalentProperty` configurations that the six dead arms need.
`live_exercises_every_arm` applies all fourteen derivations at concrete instances
of this model, which is a strictly stronger gate than any field being non-empty.

## Nothing is left vacuous, and getting there took two rebuilds

The 15 September 2026 rebuild closed the six dead arms and left five fields still
holding for want of anything to check: `same_fwd`, `sym_fwd`, `trp_fwd`,
`inv_fwd` and `hv_eq`, none of which carries any of the fourteen arms. The second
rebuild closes those five too. `owl:sameAs`, `owl:inverseOf`, `owl:hasValue`,
`owl:SymmetricProperty` and `owl:TransitiveProperty` had all been falling through
the denotation table to the junk element `other`, whose extension is empty; they
now denote elements of their own, and `r1`, `sy`, `tr` and `H1` are the property
and restriction rows those five conditions read. `live_fires_every_field`
exhibits a satisfied antecedent for each of the twenty-one.

The note that used to sit here said `same_fwd` "cannot be exercised
non-trivially by ANY model", and as a statement about the extension being empty
that is false. RBS Table 5.9 row 1 is an `iff` whose right-hand side is
`a₁ = a₂` over unscoped variables, which the specification's conventions section
reads as ranging over IR, and RBS section 4.2 defines IR as "the universe of I,
i.e., a nonempty set". So a conforming interpretation's `owl:sameAs` extension is
the whole NON-EMPTY diagonal, and a model that leaves it empty is not modelling
that cell, it is failing to. What is true is the narrower statement
`sameAs_has_no_off_diagonal_instance` proves: no interpretation meeting the cell
has an instance at `a ≠ b`. The condition is exercised here, thirty-five times,
and every instance is diagonal because the specification permits no other.

## The model

Carrier: three individuals `alice`, `bob` and `carl`; eight classes, of which
seven are restrictions; six properties; seventeen vocabulary denotations that
have to be told apart; and one junk element `other` absorbing every remaining
IRI. Thirty-five elements.

SEVENTEEN rows are chosen and the conditions determine the other eleven.

* `IEXT(p1) = IEXT(q) := {(alice, carl)}`, `IEXT(p2) := {(alice, bob),
  (alice, carl)}`, `ICEXT(Y) := {carl}`.
* `IEXT(r1) := {(carl, alice)}`, the converse of `IEXT(p1)`, which is what
  `owl:inverseOf` relates `p1` to; `IEXT(sy) := {(bob, carl), (carl, bob)}`,
  symmetric and not transitive; `IEXT(tr) := {(alice, bob), (bob, carl),
  (alice, carl)}`, transitive and not symmetric. `ICEXT(owl:SymmetricProperty)`
  is `{sy}` and `ICEXT(owl:TransitiveProperty)` is `{tr}`, so the two conditions
  are separated rather than met by one relation that satisfies both.
* `owl:sameAs` gets the whole DIAGONAL, which is the only extension RBS
  Table 5.9 row 1 permits and is not empty.
* `owl:allValuesFrom` puts `C1 = ∀p1.Y`, `C2 = ∀p2.Y` and `A2 = ∀p2.C1`;
  `owl:someValuesFrom` puts `S1 = ∃p2.Y`, `S2 = ∃p2.C1` and `S3 = ∃p1.Y`;
  `owl:equivalentClass` relates `S1` and `S2`, whose class extensions are equal;
  `owl:equivalentProperty` relates `p1` and `q`, whose property extensions are
  equal.
* `avf_eq` then forces `ICEXT(C1)` to be the whole carrier, because `alice`'s
  only `p1`-successor is `carl` and `carl` IS in `ICEXT(Y)`, and `ICEXT(C2)` to
  be the carrier minus `alice`, because `alice` also has the `p2`-successor `bob`
  and `bob` is not.
* So `ICEXT(C1)` is not contained in `ICEXT(C2)`, and `sc_fwd` makes
  `(C1, C2)` in `IEXT(rdfs:subClassOf)` IMPOSSIBLE. That is the refutation.
* And `ICEXT(C2)` IS contained in `ICEXT(C1)`, so `sc_bwd` FORCES
  `(C2, C1)` into `IEXT(rdfs:subClassOf)`, which is what `scm-avf2` draws. The
  witness confirms both halves of the pair, where the Herbrand witness in
  `Witness.lean` asserts one and leaves the other out by hand.
* `svf_eq` forces `ICEXT(S1) = ICEXT(S2) = ICEXT(S3) = {alice}`, and `hv_eq`
  forces `ICEXT(H1) = {alice}` as well: `H1` is the `p2`-values restriction on
  `bob` and `alice` is the only element with `bob` as a `p2`-successor.
* `IC := {C1, C2, A2, Y, S1, S2, S3, H1}` through `ICEXT(rdfs:Class)`, and
  `ICEXT(owl:Restriction)` is the seven restrictions, a PROPER subset of `IC`
  per Table 5.2, because `Y` is a class and not a restriction.
  `owl:Restriction` itself is kept out of `IC`, so `sc_bwd` raises no obligation
  about it.
* `sc_bwd`, `sp_bwd`, `dom_bwd` and `rng_bwd` then force the whole of
  `IEXT(rdfs:subClassOf)`, `IEXT(rdfs:subPropertyOf)`, `IEXT(rdfs:domain)` and
  `IEXT(rdfs:range)`, and the last three are a FIXPOINT rather than a free
  choice: each is defined in terms of the others, because `rdfs:subPropertyOf`,
  `rdfs:domain` and `rdfs:range` are themselves members of `IP` and so are
  subject to the very conditions they carry. The tables below are that fixpoint.
  `(p2, p1)` is NOT in `IEXT(rdfs:subPropertyOf)`, because `IEXT(p2)` is not
  contained in `IEXT(p1)`, and the domain and range tables DIFFER, including at
  the rows for `p1`, `q`, `p2`, `r1`, `tr` and `rdf:type`, which is the model
  separating the two rows of Table 5.8 rather than satisfying them both by
  accident. `r1` is the
  sharpest of those: it reaches `Y` on the domain side and the four
  `{alice}`-extension restrictions on the range side, exactly the mirror image of
  what `p1` does.
  `live_rng_bwd_is_exercised` pins one such separation, `(p2, C2)`, as a
  theorem: it is in the range extension and not in the domain extension.

The fixpoint is why several entries look strange at first reading.
`rdfs:domain` has `rdfs:subClassOf` in its subject position, for instance. That
is forced, not chosen: `rdfs:subClassOf` is in `IP`, `C1` is in `IC`, every
subject of `IEXT(rdfs:subClassOf)` lies in `ICEXT(C1)` because `ICEXT(C1)` is
everything, and Table 5.8's `rdfs:domain` row is an `iff`. RDF has no sortal
separation, and this is what that costs.

## `IP`, and why these nineteen

`IP` must contain every element with a non-empty extension, or the structure is
not the image of any conforming interpretation under the bridge in `W3C.lean`
(there, `iext p x y := IP p and (x, y) in IEXT p`, so a pair in an extension
implies its predicate is in `IP`). `live_is_bridge_coherent` checks that below.
It is a property of THIS model and NOT a field of `W3CModel`, which is a
distinction an earlier draft of this file lost, at some cost; see the section on
the three transferred non-entailments below.

The nineteen are `p1`, `p2`, `q`, `r1`, `sy`, `tr` and the thirteen vocabulary
denotations that carry pairs, and every one of them is in `IP` in any conforming
interpretation:

* `owl:allValuesFrom`, `owl:someValuesFrom`, `owl:onProperty`,
  `owl:equivalentClass` and `owl:equivalentProperty` by RBS Table 5.3 directly,
  whose second column reads "∈ IP" for all five. The same table's third column
  gives `IEXT(I(owl:equivalentClass)) ⊆ IC × IC` and
  `IEXT(I(owl:equivalentProperty)) ⊆ IP × IP`, which this model also satisfies
  and which `W3C` does not state, because `eqc_fwd` and `eqp_fwd` get those
  memberships from Table 5.9 instead.
* `rdf:type` by RDF 1.1 Semantics' RDF axiomatic triple
  `rdf:type rdf:type rdf:Property .` together with RBS Table 5.2's row
  `rdf:Property | in IC | = IP`.
* `rdfs:domain`, `rdfs:range` and `rdfs:subPropertyOf` by the RDFS axiomatic
  triples `rdfs:domain rdfs:domain rdf:Property .`,
  `rdfs:range rdfs:domain rdf:Property .` and
  `rdfs:subPropertyOf rdfs:domain rdf:Property .`, each read through the same
  Table 5.2 row.
* `rdfs:subClassOf` by the RDFS axiomatic triple
  `rdfs:subClassOf rdfs:domain rdfs:Class .`, whose truth puts
  `I(rdfs:subClassOf)` into `IP` through the forward direction of Table 5.8's
  `rdfs:domain` row.
* `owl:sameAs`, `owl:inverseOf` and `owl:hasValue` by RBS Table 5.3, whose
  second column reads "∈ IP" for all three.
* `p1`, `p2` and `q` because `onp_typ` demands it of the first two and
  `eqp_fwd` of the third; `r1` because `inv_fwd` demands it; and `sy` and `tr`
  because they carry pairs and bridge coherence therefore demands it.

All five axiomatic triples were re-read in the raw HTML of
<https://www.w3.org/TR/rdf11-mt/> on 15 September 2026; they are the same five
the bridge in `W3C.lean` needs, and that file now lists them as the assumption
they are.

## WHAT THIS WITNESS DOES NOT ESTABLISH, and it is less than it looks

It establishes `not W3CEntails avfPremises (C1, rdfs:subClassOf, C2)`. That is
strictly stronger than the existing Herbrand result, which is about the Lean's
much larger `Conditions` class, and the refuting structure here satisfies
Table 5.8 in both directions, Table 5.6's `someValuesFrom` and `allValuesFrom`
equalities, Table 5.9's two rows, and the typing rows of Tables 5.2 and 5.3,
none of which the Herbrand witness in `Witness.lean` does.

**It is NOT a proof that the triple is not OWL 2 RDF-Based entailed, and nobody
should write that sentence.** Positive transfer runs outward and negative
transfer does not. `W3CModel`'s class is strictly LARGER than the bridge image
of the conforming interpretations, because `W3C` deliberately omits every row no
rule consumes, so a refutation here does not rule out that every genuinely
conforming interpretation satisfies the triple. Concretely, this model violates
at least these rows of RBS Table 5.2, all of which a conforming interpretation
must satisfy:

* `owl:Thing | in IC | = IR`. Here `owl:Thing` denotes `other`, whose class
  extension is empty and is not the carrier.
* `rdf:Property | in IC | = IP`. Here `rdf:Property` also denotes `other`, so
  its class extension is empty rather than the nineteen-element `IP`.
* `rdfs:Resource | in IC | = IR`, and the RDF and RDFS axiomatic triple tables
  themselves, which are simply absent.
* The SECOND column of three rows this model otherwise uses.
  `owl:Restriction | in IC | subset of IC`,
  `owl:SymmetricProperty | in IC | subset of IP` and
  `owl:TransitiveProperty | in IC | subset of IP` each assert a membership as
  well as an inclusion, and this model satisfies the inclusions and keeps all
  three elements OUT of `IC`. That is deliberate and it is a departure:
  `restr_IC` is the third column of the first of them, no field of `W3C` states
  any of the three memberships, and putting `owl:Restriction` into `IC` would
  raise `sc_bwd` obligations about it that nothing here needs. All three rows
  were read in the raw HTML of the Recommendation on 15 September 2026.

Closing that gap means formalising Table 5.2's forty-odd rows, the axiomatic
triple tables, and the parts of the universe from Table 5.1, and then rebuilding
this model over them. That is a different project, and the honest report is that
this witness gets closer to the specification than anything else here and does
not arrive.

## What it leaves untouched, corrected twice

An earlier version of this section said that this file "does NOT rehabilitate
the other non-entailment results", and listed seven, on the ground that all of
them "are discharged by Herbrand or saturated interpretations, and none of those
is a `W3CModel`". That is false, it was false of one of the seven at the moment
it was written, and the reason given for it was inverted. Four of the seven are
`W3CModel`s with their witness graphs unchanged.

The next version said the remaining three could not transfer and pinned, as
checked theorems, the field that fails in each. Those theorems were true and they
were about the HERBRAND WITNESSES, not about the results. A witness that is not a
`W3CModel` is a reason to build one, not a reason the statement is out of reach,
and all three are reached now, each on a finite structure built for it:
`the_old_svf_derivation_is_not_w3c_entailed` at the end of this file, and
`feed_is_not_w3c_refuted` and `the_old_verdict_does_not_notice_over_w3c` in
`RefuteWitness.lean`. Nothing in this repository now states a non-entailment
about `Conditions` alone.
-/
namespace OOCert

/-! ## The carrier -/

/-- The thirty-five elements. `other` absorbs every IRI the model does not need
to tell apart, and its extension and class extension are both empty. Five terms
used to stay in `other` and made their conditions hold vacuously; they have rows
of their own now, and the module docstring says which and why. -/
inductive LiveD where
  /-- An individual with a `p1`-successor and two `p2`-successors, and the only
  element outside `ICEXT(C2)`. -/
  | alice
  /-- A `p2`-successor of `alice` that is NOT in `ICEXT(Y)`, which is what puts
  `alice` outside `ICEXT(C2)` and makes the refutation work. -/
  | bob
  /-- A `p1`-successor of `alice` that IS in `ICEXT(Y)`. `carl` is the element
  the 15 September 2026 rebuild added: without it `IEXT(p1)` is empty, the
  premise `p1 rdfs:subPropertyOf p2` holds only because there is nothing to
  check, and `ICEXT(Y)` is empty as well. -/
  | carl
  /-- `C1`, the universal restriction `∀p1.Y` that `avfPremises` asserts. -/
  | c1
  /-- `C2`, the universal restriction `∀p2.Y`. -/
  | c2
  /-- `A2`, a second universal restriction on `p2`, with filler `C1`. It exists
  so that `scm-avf1` has two `owl:allValuesFrom` restrictions sharing a property
  to order; nothing in `avfPremises` mentions it. -/
  | a2
  /-- `Y`, the shared filler. `ICEXT(Y) = {carl}`, NOT empty. -/
  | filler
  /-- `S1`, the existential restriction `∃p2.Y`. -/
  | s1
  /-- `S2`, the existential restriction `∃p2.C1`. `S1` and `S2` order under
  `scm-svf1` because `Y` is below `C1`. -/
  | s2
  /-- `S3`, the existential restriction `∃p1.Y`. `S3` and `S1` order under
  `scm-svf2` because `p1` is below `p2`. -/
  | s3
  /-- `H1`, the value restriction `∃p2.{bob}`. `owl:hasValue` needs a
  restriction to hold it, and `hv_eq` then forces `ICEXT(H1) = {alice}`, because
  `alice` is the only element with `bob` as a `p2`-successor. -/
  | h1
  /-- `p1`, with the single pair `(alice, carl)`. -/
  | p1
  /-- `p2`, with `(alice, bob)` and `(alice, carl)`, so `IEXT(p1)` is a PROPER
  subset of `IEXT(p2)`. -/
  | p2
  /-- A second property with `IEXT(q) = IEXT(p1)`, exactly so that
  `owl:equivalentProperty` has a pair to relate. -/
  | q
  /-- `r1`, with the single pair `(carl, alice)`, which is the CONVERSE of
  `IEXT(p1)`. It exists so that `owl:inverseOf` relates two DISTINCT properties
  rather than a symmetric one to itself. -/
  | r1
  /-- `sy`, with `(bob, carl)` and `(carl, bob)`. Symmetric and NOT transitive,
  which is what keeps `sym_fwd` and `trp_fwd` separated by this model rather
  than satisfied together by one relation that happens to be both. -/
  | sy
  /-- `tr`, with `(alice, bob)`, `(bob, carl)` and `(alice, carl)`. Transitive
  and NOT symmetric. -/
  | tr
  /-- `rdf:type`, whose extension IS `ICEXT`. -/
  | ty
  /-- `rdfs:subClassOf`. -/
  | sco
  /-- `rdfs:subPropertyOf`. -/
  | spo
  /-- `rdfs:domain`. -/
  | dm
  /-- `rdfs:range`. -/
  | rg
  /-- `owl:allValuesFrom`. -/
  | avf
  /-- `owl:someValuesFrom`. -/
  | svf
  /-- `owl:onProperty`. -/
  | onp
  /-- `owl:equivalentClass`. -/
  | eqc
  /-- `owl:equivalentProperty`. -/
  | eqp
  /-- `owl:sameAs`, whose extension is the DIAGONAL on the carrier. RBS Table
  5.9 row 1 is an `iff` and its right-hand side is `a₁ = a₂`, so that is the
  only extension a conforming interpretation may give it, and it is the whole
  diagonal rather than nothing. -/
  | sa
  /-- `owl:inverseOf`. -/
  | invo
  /-- `owl:hasValue`. -/
  | hv
  /-- `owl:SymmetricProperty`, whose class extension is `{sy}`. -/
  | symp
  /-- `owl:TransitiveProperty`, whose class extension is `{tr}`. -/
  | trp
  /-- `rdfs:Class`, whose class extension IS `IC` (Table 5.2, "= IC"). -/
  | cls
  /-- `owl:Restriction`, whose class extension is a PROPER subset of `IC`
  (Table 5.2 writes a subset, never an equality, and this model makes the
  inclusion strict: `Y` is a class and not a restriction). -/
  | restr
  /-- Every other IRI. -/
  | other
deriving DecidableEq, Repr

namespace LiveD

/-- The carrier as a list, so that quantification over it is decidable without
Mathlib's `Fintype`. -/
def all : List LiveD :=
  [alice, bob, carl, c1, c2, a2, filler, s1, s2, s3, h1, p1, p2, q, r1, sy, tr,
   ty, sco, spo, dm, rg, avf, svf, onp, eqc, eqp, sa, invo, hv, symp, trp,
   cls, restr, other]

theorem mem_all (w : LiveD) : w ∈ all := by cases w <;> decide

/-- Universal quantification over the carrier is decidable. Every condition
below is settled by `decide` through this instance, so the model is an
executable fact and not a tactic script that might be proving something else. -/
instance decForall (p : LiveD → Prop) [DecidablePred p] : Decidable (∀ w, p w) :=
  decidable_of_iff (∀ w ∈ all, p w) ⟨fun h w => h w (mem_all w), fun h w _ => h w⟩

/-- And so is existential quantification, which `svf_eq` needs on its right-hand
side. -/
instance decExists (p : LiveD → Prop) [DecidablePred p] : Decidable (∃ w, p w) :=
  decidable_of_iff (∃ w ∈ all, p w)
    ⟨fun ⟨w, _, h⟩ => ⟨w, h⟩, fun ⟨w, h⟩ => ⟨w, mem_all w, h⟩⟩

end LiveD

/-! ## The interpretation -/

/-- The denotation table. Everything absent from it denotes `other`, so
`owl:intersectionOf`, `owl:unionOf`, `owl:oneOf`, `owl:Thing` and `rdf:Property`
land there together with an empty extension. Those five are where the model
stops being a conforming interpretation, and the module docstring says so.

Every term whose row `W3C` states now has a row here. The 15 September 2026
rebuild added the last five, `owl:sameAs`, `owl:inverseOf`, `owl:hasValue`,
`owl:SymmetricProperty` and `owl:TransitiveProperty`, which used to fall through
to `other` and made their five conditions hold for want of anything to check. -/
def liveTable : List (Term × LiveD) :=
  [ (V.type, .ty), (V.subClassOf, .sco), (V.subPropertyOf, .spo),
    (V.domain, .dm), (V.range, .rg),
    (V.allValuesFrom, .avf), (V.someValuesFrom, .svf), (V.onProperty, .onp),
    (V.equivalentClass, .eqc), (V.equivalentProperty, .eqp),
    (V.sameAs, .sa), (V.inverseOf, .invo), (V.hasValue, .hv),
    (V.symmetricProperty, .symp), (V.transitiveProperty, .trp),
    (V.Class, .cls), (V.Restriction, .restr),
    (tC1, .c1), (tC2, .c2), (tY, .filler), (tp1, .p1), (tp2, .p2) ]

/-- Terms denote their table entry, or `other`. -/
def liveι (t : Term) : LiveD := (List.lookup t liveTable).getD .other

/-- `IP`. The nineteen elements that carry pairs.

Every one of them is in `IP` in any conforming interpretation, by RBS Table 5.3
for `owl:allValuesFrom`, `owl:someValuesFrom` and `owl:onProperty`, by Table 5.9
for `owl:equivalentClass` and `owl:equivalentProperty`, and by the RDF and RDFS
axiomatic triples read through Table 5.2's `rdf:Property | = IP` row for the
rest; the citations are in the module docstring. Nothing else is in `IP`, which
is what keeps `sp_bwd`, `dom_bwd` and `rng_bwd` from forcing schema triples about
individuals. -/
def isIP : LiveD → Bool
  | .p1 | .p2 | .q | .r1 | .sy | .tr | .ty | .sco | .spo | .dm | .rg
  | .avf | .svf | .onp | .eqc | .eqp | .sa | .invo | .hv => true
  | _ => false

/-- `IC`, as a match rather than a disjunction chain. Every table below is
written this way on purpose. A `||` chain costs one `whnf` frame per disjunct,
four nested quantifiers over a thirty-five element carrier stack those frames,
and the default `maxRecDepth` is reached before the proposition is decided; a
pattern match compiles to a `casesOn` tree and costs constant depth. The tables
are also easier to read this way, which is the smaller of the two reasons. -/
def isIC : LiveD → Bool
  | .c1 | .c2 | .a2 | .filler | .s1 | .s2 | .s3 | .h1 => true
  | _ => false

/-- `ICEXT(owl:Restriction)`: the seven restrictions. A PROPER subset of `IC`,
because `Y` is a class and not a restriction, and RBS Table 5.2 writes a subset
there and never an equality. -/
def isRestriction : LiveD → Bool
  | .c1 | .c2 | .a2 | .s1 | .s2 | .s3 | .h1 => true
  | _ => false

/-- The property extensions, as a decidable relation.

SEVENTEEN rows are chosen: `IEXT(p1)`, `IEXT(q)`, `IEXT(p2)`, `IEXT(r1)`,
`IEXT(sy)`, `IEXT(tr)`, `ICEXT(Y)`, the eight constructor tables
`owl:allValuesFrom`, `owl:someValuesFrom`, `owl:onProperty`,
`owl:equivalentClass`, `owl:equivalentProperty`, `owl:inverseOf`, `owl:hasValue`
and `owl:sameAs`, and the four class extensions `ICEXT(rdfs:Class)`,
`ICEXT(owl:Restriction)`, `ICEXT(owl:SymmetricProperty)` and
`ICEXT(owl:TransitiveProperty)` that say which elements are classes, which are
restrictions, and which properties are symmetric or transitive. The other ELEVEN
are determined. The class extensions of `C1`, `C2`, `A2`, `S1`, `S2`, `S3` and
`H1` are FORCED by Table 5.6, and `sco`, `spo`, `dm` and `rg` are the FIXPOINT
that Table 5.8's two directions force from all of it. The fixpoint is mutual,
because `sco`, `spo`, `dm` and `rg` are themselves in `IP` or `IC` and so appear
on both sides of their own conditions. Nothing here can be adjusted without the
build going red. -/
def liveIext : LiveD → LiveD → LiveD → Bool
  -- ICEXT(rdfs:Class) = IC = {C1, C2, A2, Y, S1, S2, S3, H1}. Table 5.2's "= IC" row.
  | .ty, x, .cls => isIC x
  -- ICEXT(owl:Restriction), a PROPER subset of IC.
  | .ty, x, .restr => isRestriction x
  -- ICEXT(C1) is everything. FORCED: C1 is ALL p1.Y, IEXT(p1) = {(alice, carl)} and
  -- carl IS in ICEXT(Y), so alice qualifies and nothing else has a p1-successor.
  | .ty, _, .c1 => true
  -- ICEXT(A2) is everything. FORCED: A2 is ALL p2.C1 and ICEXT(C1) is everything.
  | .ty, _, .a2 => true
  -- ICEXT(C2) is everything but alice. FORCED: C2 is ALL p2.Y and alice has the
  -- p2-successor bob, which is not in ICEXT(Y).
  | .ty, x, .c2 => x != .alice
  -- ICEXT(Y) = {carl}. NOT EMPTY, which is what makes alice's membership of
  -- ICEXT(C1) a real check rather than an empty one.
  | .ty, x, .filler => x == .carl
  -- ICEXT(S1) = ICEXT(S2) = ICEXT(S3) = {alice}. FORCED by Table 5.6's
  -- someValuesFrom row: alice reaches carl by p1 and by p2, carl is in ICEXT(Y),
  -- and ICEXT(C1) is everything.
  | .ty, x, .s1 | .ty, x, .s2 | .ty, x, .s3 => x == .alice
  -- ICEXT(H1) = {alice}, FORCED by Table 5.6's hasValue row: H1 is the p2-values
  -- restriction on bob, and alice is the only element with bob as a p2-successor.
  | .ty, x, .h1 => x == .alice
  -- ICEXT(owl:SymmetricProperty) = {sy} and ICEXT(owl:TransitiveProperty) = {tr}.
  -- Two DIFFERENT properties: sy is not transitive and tr is not symmetric, so this
  -- model separates the two conditions rather than meeting both with one relation
  -- that happens to satisfy each.
  | .ty, x, .symp => x == .sy
  | .ty, x, .trp => x == .tr
  -- ICEXT of everything else is empty.
  | .ty, _, _ => false
  -- IEXT(rdfs:subClassOf): exactly the pairs of IC whose extensions nest.
  -- (C1, C2) is ABSENT, and that absence is the theorem at the end of the file.
  | .sco, .c1, v | .sco, .a2, v => v == .c1 || v == .a2
  | .sco, .c2, v => v == .c1 || v == .c2 || v == .a2
  | .sco, .filler, v => v == .c1 || v == .c2 || v == .a2 || v == .filler
  | .sco, .s1, v | .sco, .s2, v | .sco, .s3, v | .sco, .h1, v =>
      v == .c1 || v == .a2 || v == .s1 || v == .s2 || v == .s3 || v == .h1
  | .sco, _, _ => false
  -- IEXT(rdfs:subPropertyOf): exactly the pairs of IP whose extensions nest. The
  -- diagonal; p1 and q both ways because their extensions are equal, and both inside
  -- p2; p1, q and p2 inside tr, which is forced because IEXT(tr) contains both of
  -- alice's p2-pairs; owl:equivalentClass inside rdfs:subClassOf and
  -- owl:equivalentProperty inside rdfs:subPropertyOf, which are forced and not
  -- chosen. (p2, p1) is ABSENT, and that kind separation is what keeps scm-avf2
  -- antitone here.
  | .spo, .p1, v | .spo, .q, v => v == .p1 || v == .p2 || v == .q || v == .tr
  | .spo, .p2, v => v == .p2 || v == .tr
  | .spo, .r1, v => v == .r1
  | .spo, .sy, v => v == .sy
  | .spo, .tr, v => v == .tr
  | .spo, .ty, v => v == .ty
  | .spo, .sco, v => v == .sco
  | .spo, .spo, v => v == .spo
  | .spo, .dm, v => v == .dm
  | .spo, .rg, v => v == .rg
  | .spo, .avf, v => v == .avf
  | .spo, .svf, v => v == .svf
  | .spo, .onp, v => v == .onp
  | .spo, .eqc, v => v == .sco || v == .eqc
  | .spo, .eqp, v => v == .spo || v == .eqp
  | .spo, .sa, v => v == .sa
  | .spo, .invo, v => v == .invo
  | .spo, .hv, v => v == .hv
  | .spo, _, _ => false
  -- IEXT(rdfs:domain): every p in IP and c in IC such that every subject of p lies
  -- in ICEXT(c). p1, q and p2 have the single subject alice, so they reach the three
  -- existential restrictions and H1 as well; r1's single subject is carl, which is
  -- in ICEXT(Y), so r1 reaches Y where nothing else does; rdf:type, owl:sameAs and
  -- tr have a subject outside ICEXT(C2), so they reach only C1 and A2; the rest have
  -- subjects that are classes or properties and never alice.
  | .dm, .p1, v | .dm, .p2, v | .dm, .q, v =>
      v == .c1 || v == .a2 || v == .s1 || v == .s2 || v == .s3 || v == .h1
  | .dm, .r1, v => v == .c1 || v == .c2 || v == .a2 || v == .filler
  | .dm, .sy, v | .dm, .sco, v | .dm, .spo, v | .dm, .dm, v | .dm, .rg, v | .dm, .avf, v |
    .dm, .svf, v | .dm, .onp, v | .dm, .eqc, v | .dm, .eqp, v | .dm, .invo, v | .dm, .hv, v =>
      v == .c1 || v == .c2 || v == .a2
  | .dm, .tr, v | .dm, .ty, v | .dm, .sa, v => v == .c1 || v == .a2
  | .dm, _, _ => false
  -- IEXT(rdfs:range): the same with objects for subjects, and it DIFFERS from the
  -- domain table. p1 and q reach Y, because their only object carl is in ICEXT(Y)
  -- while their only subject alice is not; neither reaches S1, S2, S3 or H1, where
  -- the domain table does; and r1 is the mirror image of that, reaching those four on
  -- the range side and Y on the domain side.
  | .rg, .p1, v | .rg, .q, v => v == .c1 || v == .c2 || v == .a2 || v == .filler
  | .rg, .p2, v | .rg, .sy, v | .rg, .tr, v | .rg, .ty, v | .rg, .sco, v | .rg, .spo, v |
    .rg, .dm, v | .rg, .rg, v | .rg, .avf, v | .rg, .svf, v | .rg, .onp, v | .rg, .eqc, v |
    .rg, .eqp, v | .rg, .invo, v | .rg, .hv, v =>
      v == .c1 || v == .c2 || v == .a2
  | .rg, .r1, v => v == .c1 || v == .a2 || v == .s1 || v == .s2 || v == .s3 || v == .h1
  | .rg, .sa, v => v == .c1 || v == .a2
  | .rg, _, _ => false
  -- The graph's two universal restrictions, plus A2.
  | .avf, .c1, v | .avf, .c2, v => v == .filler
  | .avf, .a2, v => v == .c1
  | .avf, _, _ => false
  -- The three existential restrictions.
  | .svf, .s1, v | .svf, .s3, v => v == .filler
  | .svf, .s2, v => v == .c1
  | .svf, _, _ => false
  -- Each restriction is on exactly one property.
  | .onp, .c1, v | .onp, .s3, v => v == .p1
  | .onp, .c2, v | .onp, .a2, v | .onp, .s1, v | .onp, .s2, v | .onp, .h1, v => v == .p2
  | .onp, _, _ => false
  -- Two distinct classes with the same extension, so Table 5.9's class row has a
  -- pair to work on and scm-eqc1 is exercised on something other than a diagonal.
  | .eqc, .s1, v => v == .s2
  | .eqc, .s2, v => v == .s1
  | .eqc, _, _ => false
  -- And two distinct properties with the same extension, for scm-eqp1.
  | .eqp, .p1, v => v == .q
  | .eqp, .q, v => v == .p1
  | .eqp, _, _ => false
  -- RBS Table 5.9 row 1 is an iff whose right-hand side is `a1 = a2`, and its
  -- variables are unscoped, which the specification's own conventions read as
  -- ranging over IR. So a conforming interpretation gives owl:sameAs the WHOLE
  -- diagonal on the carrier, and never the empty relation the first version of this
  -- model gave it.
  | .sa, u, v => u == v
  -- One inverse pair, on two DISTINCT properties: IEXT(r1) is the converse of
  -- IEXT(p1) and neither is symmetric, so inv_fwd is exercised on a pair that is not
  -- a property paired with itself.
  | .invo, .p1, v => v == .r1
  | .invo, _, _ => false
  -- One value restriction: H1 is the p2-values restriction on bob.
  | .hv, .h1, v => v == .bob
  | .hv, _, _ => false
  -- The asserted individual pairs. IEXT(p1) is NOT empty and is a PROPER subset of
  -- IEXT(p2), which is what makes `p1 rdfs:subPropertyOf p2` hold here for a reason
  -- rather than for want of anything to check.
  | .p1, .alice, v | .q, .alice, v => v == .carl
  | .p2, .alice, v => v == .bob || v == .carl
  -- The converse of p1; a symmetric relation that is not transitive; and a
  -- transitive one that is not symmetric.
  | .r1, .carl, v => v == .alice
  | .sy, .bob, v => v == .carl
  | .sy, .carl, v => v == .bob
  | .tr, .alice, v => v == .bob || v == .carl
  | .tr, .bob, v => v == .carl
  -- Everything else, `other` included, is empty.
  | _, _, _ => false

/-- The interpretation. -/
def live : Interp where
  D := LiveD
  ι := liveι
  iext := fun p x y => liveIext p x y = true

/-- `IP` as a predicate. A parameter of `W3C` and not a field of `Interp`, so
nothing outside this file sees it. -/
def liveIP (x : LiveD) : Prop := isIP x = true

/-- `IP` membership is decidable, which is what lets every condition mentioning
it be settled by `decide` rather than by a tactic script. -/
instance : DecidablePred liveIP := fun x => decidable_of_iff (isIP x = true) Iff.rfl

@[simp] theorem live_iext (p x y : LiveD) : live.iext p x y ↔ liveIext p x y = true := Iff.rfl
@[simp] theorem live_cext (c x : LiveD) : live.cext c x ↔ liveIext .ty x c = true := Iff.rfl
@[simp] theorem live_sc (u v : LiveD) : live.sc u v ↔ liveIext .sco u v = true := Iff.rfl
@[simp] theorem live_sp (u v : LiveD) : live.sp u v ↔ liveIext .spo u v = true := Iff.rfl
@[simp] theorem live_IC (u : LiveD) : live.IC u ↔ liveIext .ty u .cls = true := Iff.rfl

/-! ## Two elaboration limits, and what they are and are not

These are the only two `set_option` lines in `lean/`, so they get a reason.

`sp_bwd`, `dom_bwd` and `rng_bwd` are quantified over the carrier FOUR times
each, twice outside the implication and twice inside its universally quantified
clause, so `decide` evaluates on the order of the fourth power of the carrier and
nests the `Decidable` instances four deep on top of that.

At twenty-six elements the old model sat inside both defaults, and that is
measured rather than assumed: `git archive` of `lean/` at 0edfe44 into an empty
directory, then `lake build` with no warm cache and no `set_option` anywhere,
completes 105 jobs in about seventy seconds with no error. At thirty-five
elements it does not, and the five conditions the second rebuild added are why
the carrier grew.

Neither option touches what is proved. `maxRecDepth` bounds how deeply the
elaborator may recurse and `maxHeartbeats` how long it may run; a proof term that
gets past them is checked by the same kernel on the same rules as one that does
not, and the axiom tripwires at the end of this file are what guard soundness.
What they do cost is build time. This is by a wide margin the longest single file
in `lean/` to elaborate, and it accounts for most of a cold `lake build`. If that
becomes intolerable the honest repair is a smaller carrier and a smaller claim,
stated as such, and not a quieter gate. -/
set_option maxRecDepth 20000
set_option maxHeartbeats 2000000

/-! ## The conditions, one decidable fact each

Each lemma is stated over `liveIext` and `liveι` directly, which is what makes
it a closed decidable proposition; `live_w3c` takes each as the corresponding
field by definitional unfolding, so nothing is restated in two places that could
drift apart. -/

theorem live_sc_fwd : ∀ a b : LiveD, liveIext .sco a b = true →
    liveIext .ty a .cls = true ∧ liveIext .ty b .cls = true ∧
    ∀ x, liveIext .ty x a = true → liveIext .ty x b = true := by decide

theorem live_sc_bwd : ∀ a b : LiveD, liveIext .ty a .cls = true → liveIext .ty b .cls = true →
    (∀ x, liveIext .ty x a = true → liveIext .ty x b = true) → liveIext .sco a b = true := by
  decide

theorem live_sp_fwd : ∀ a b : LiveD, liveIext .spo a b = true →
    liveIP a ∧ liveIP b ∧ ∀ x y, liveIext a x y = true → liveIext b x y = true := by decide

theorem live_sp_bwd : ∀ a b : LiveD, liveIP a → liveIP b →
    (∀ x y, liveIext a x y = true → liveIext b x y = true) → liveIext .spo a b = true := by decide

theorem live_dom_fwd : ∀ p c : LiveD, liveIext (liveι V.domain) p c = true →
    liveIP p ∧ liveIext .ty c .cls = true ∧
    ∀ x y, liveIext p x y = true → liveIext .ty x c = true := by decide

theorem live_dom_bwd : ∀ p c : LiveD, liveIP p → liveIext .ty c .cls = true →
    (∀ x y, liveIext p x y = true → liveIext .ty x c = true) →
    liveIext (liveι V.domain) p c = true := by decide

theorem live_rng_fwd : ∀ p c : LiveD, liveIext (liveι V.range) p c = true →
    liveIP p ∧ liveIext .ty c .cls = true ∧
    ∀ x y, liveIext p x y = true → liveIext .ty y c = true := by decide

theorem live_rng_bwd : ∀ p c : LiveD, liveIP p → liveIext .ty c .cls = true →
    (∀ x y, liveIext p x y = true → liveIext .ty y c = true) →
    liveIext (liveι V.range) p c = true := by decide

theorem live_eqc_fwd : ∀ a b : LiveD, liveIext (liveι V.equivalentClass) a b = true →
    liveIext .ty a .cls = true ∧ liveIext .ty b .cls = true ∧
    ∀ x, (liveIext .ty x a = true ↔ liveIext .ty x b = true) := by decide

theorem live_eqp_fwd : ∀ a b : LiveD, liveIext (liveι V.equivalentProperty) a b = true →
    liveIP a ∧ liveIP b ∧ ∀ x y, (liveIext a x y = true ↔ liveIext b x y = true) := by decide

theorem live_same_fwd : ∀ a b : LiveD, liveIext (liveι V.sameAs) a b = true → a = b := by decide

theorem live_inv_fwd : ∀ p r : LiveD, liveIext (liveι V.inverseOf) p r = true →
    liveIP p ∧ liveIP r ∧ ∀ x y, (liveIext p x y = true ↔ liveIext r y x = true) := by decide

theorem live_sym_fwd : ∀ p : LiveD, liveIext .ty p (liveι V.symmetricProperty) = true →
    ∀ x y, liveIext p x y = true → liveIext p y x = true := by decide

theorem live_trp_fwd : ∀ p : LiveD, liveIext .ty p (liveι V.transitiveProperty) = true →
    ∀ x y z, liveIext p x y = true → liveIext p y z = true → liveIext p x z = true := by decide

theorem live_svf_eq : ∀ z c p : LiveD, liveIext (liveι V.someValuesFrom) z c = true →
    liveIext (liveι V.onProperty) z p = true →
    ∀ x, (liveIext .ty x z = true ↔ ∃ y, liveIext p x y = true ∧ liveIext .ty y c = true) := by
  decide

theorem live_avf_eq : ∀ z c p : LiveD, liveIext (liveι V.allValuesFrom) z c = true →
    liveIext (liveι V.onProperty) z p = true →
    ∀ x, (liveIext .ty x z = true ↔ ∀ y, liveIext p x y = true → liveIext .ty y c = true) := by
  decide

theorem live_hv_eq : ∀ z a p : LiveD, liveIext (liveι V.hasValue) z a = true →
    liveIext (liveι V.onProperty) z p = true →
    ∀ x, (liveIext .ty x z = true ↔ liveIext p x a = true) := by decide

theorem live_restr_IC : ∀ x : LiveD, liveIext .ty x (liveι V.Restriction) = true →
    liveIext .ty x .cls = true := by decide

theorem live_svf_typ : ∀ z c : LiveD, liveIext (liveι V.someValuesFrom) z c = true →
    liveIext .ty z (liveι V.Restriction) = true ∧ liveIext .ty c .cls = true := by decide

theorem live_avf_typ : ∀ z c : LiveD, liveIext (liveι V.allValuesFrom) z c = true →
    liveIext .ty z (liveι V.Restriction) = true ∧ liveIext .ty c .cls = true := by decide

theorem live_onp_typ : ∀ z p : LiveD, liveIext (liveι V.onProperty) z p = true →
    liveIext .ty z (liveι V.Restriction) = true ∧ liveIP p := by decide

/-- Every asserted triple of `avfPremises` holds. -/
theorem live_facts : ∀ t ∈ avfPremises, liveIext (liveι t.p) (liveι t.s) (liveι t.o) = true := by
  decide

/-- The conditions, assembled. Each field is the decidable fact above, taken by
definitional unfolding of `Interp.sc`, `Interp.sp`, `Interp.cext` and
`Interp.IC`. -/
theorem live_w3c : W3C live liveIP where
  sc_fwd := live_sc_fwd
  sc_bwd := live_sc_bwd
  sp_fwd := live_sp_fwd
  sp_bwd := live_sp_bwd
  dom_fwd := live_dom_fwd
  dom_bwd := live_dom_bwd
  rng_fwd := live_rng_fwd
  rng_bwd := live_rng_bwd
  eqc_fwd := live_eqc_fwd
  eqp_fwd := live_eqp_fwd
  same_fwd := live_same_fwd
  inv_fwd := live_inv_fwd
  sym_fwd := live_sym_fwd
  trp_fwd := live_trp_fwd
  svf_eq := live_svf_eq
  avf_eq := live_avf_eq
  hv_eq := live_hv_eq
  restr_IC := live_restr_IC
  svf_typ := live_svf_typ
  avf_typ := live_avf_typ
  onp_typ := live_onp_typ

/-- **A `W3CModel` of `avfPremises`.** The three list fields are vacuous
because `avfPremises` carries no `owl:intersectionOf`, `owl:unionOf` or
`owl:oneOf` triple; the model says nothing about those rows and does not pretend
to. -/
theorem live_is_a_w3c_model : W3CModel live liveIP avfPremises where
  conds := live_w3c
  facts := live_facts
  int_eq := fun c l _ hc =>
    absurd hc (not_mem_pred avfPremises V.intersectionOf (by decide) c l)
  uni_eq := fun c l _ hc => absurd hc (not_mem_pred avfPremises V.unionOf (by decide) c l)
  oneOf_eq := fun c l _ hc => absurd hc (not_mem_pred avfPremises V.oneOf (by decide) c l)

/-! ## Non-vacuity, compiled into the build

The facts the model exists to carry, in three theorems, so that a later edit that
hollows the model out fails here instead of passing quietly. Everything above
would still compile with an empty carrier and every relation false; this is what
stops that from being reported as a result.

The first version of this section made the opposite mistake. `IEXT(p1)` was
empty and `ICEXT(Y)` was empty, and both emptinesses were written into the
liveness gate as though they were features. They are not: with `IEXT(p1)` empty
the refutation's own premise `p1 rdfs:subPropertyOf p2` holds because there is
nothing to check, which is the defect this file exists to close, one level down.
`carl` is what fixes it, and `live_is_live` now pins the inclusion as PROPER and
both extensions as NON-EMPTY.

Each fact is stated twice: once over `liveIext` and `liveι`, where it is a closed
decidable proposition, and once over `live`, where it is the sentence a reader
wants. The second is the first by definitional unfolding, so the two cannot
drift. -/

theorem live_is_live_raw :
    (liveIext .ty .c1 .cls = true ∧ liveIext .ty .c2 .cls = true ∧
        liveIext .ty .filler .cls = true) ∧
      (¬ liveIext .ty .restr .cls = true ∧ ¬ liveIext .ty .alice .cls = true) ∧
    (∀ x, liveIext .ty x .c1 = true) ∧
      (∀ x, liveIext .ty x .c2 = true → liveIext .ty x .c1 = true) ∧
      (liveIext .ty .alice .c1 = true ∧ ¬ liveIext .ty .alice .c2 = true ∧
        liveIext .ty .bob .c2 = true) ∧
    (liveIext .ty .carl .filler = true ∧ ¬ liveIext .ty .bob .filler = true) ∧
    (liveIext .ty .c1 (liveι V.Restriction) = true ∧
      ¬ liveIext .ty .filler (liveι V.Restriction) = true) ∧
    (liveIext .p1 .alice .carl = true ∧
      (∀ x y, liveIext .p1 x y = true → liveIext .p2 x y = true) ∧
      ¬ liveIext .p1 .alice .bob = true ∧ liveIext .p2 .alice .bob = true) := by decide

/-- **The liveness gate.** `IC` has three of its members named and does not
swallow the carrier; one class extension is the whole carrier; another is a
proper subset of it, witnessed on both sides; the FILLER extension is non-empty
and proper, witnessed on both sides; `ICEXT(owl:Restriction)` is a proper subset
of `IC`; and `IEXT(p1)` is NON-EMPTY and a PROPER subset of `IEXT(p2)`.

That last conjunct is the one the 15 September 2026 rebuild added and it is the
point of the rebuild. `avfPremises` asserts `p1 rdfs:subPropertyOf p2`, and in
the first version of this model that triple held only because `IEXT(p1)` was
empty. A refutation whose own premise is satisfied vacuously establishes almost
nothing, and the emptiness was written into this gate as if it were a feature. -/
theorem live_is_live :
    (live.IC .c1 ∧ live.IC .c2 ∧ live.IC .filler) ∧ (¬ live.IC .restr ∧ ¬ live.IC .alice) ∧
    (∀ x, live.cext .c1 x) ∧ (∀ x, live.cext .c2 x → live.cext .c1 x) ∧
      (live.cext .c1 .alice ∧ ¬ live.cext .c2 .alice ∧ live.cext .c2 .bob) ∧
    (live.cext .filler .carl ∧ ¬ live.cext .filler .bob) ∧
    (live.cext (live.ι V.Restriction) .c1 ∧ ¬ live.cext (live.ι V.Restriction) .filler) ∧
    (live.iext .p1 .alice .carl ∧ (∀ x y, live.iext .p1 x y → live.iext .p2 x y) ∧
      ¬ live.iext .p1 .alice .bob ∧ live.iext .p2 .alice .bob) :=
  live_is_live_raw

theorem live_is_bridge_coherent_raw :
    ∀ p x y : LiveD, liveIext p x y = true → liveIP p := by decide

/-- **The bridge shape, checked.** Every predicate with a pair in its extension
is in `IP`.

This is an EXTRA property of this model and not a requirement of `W3CModel`, and
the distinction matters: an earlier draft of this file, of `Witness.lean` and of
`Semantics.lean` used bridge coherence as though `W3CModel` demanded it, and
concluded from that that no Herbrand witness could ever be one. Four of them
are: `not_everything_is_w3c_entailed` further down this file, the two in the
section after it, and `mix_not_absolutely_w3c_entailed` in `Mixed.lean`.

What it is actually for: `W3C.lean`'s bridge reads a conforming interpretation
into an `Interp` by `iext p x y := IP p and (x, y) in IEXT p`, so a structure
with a pair under a predicate outside `IP` is not the image of any conforming
interpretation. A refutation built on one would be about a structure the bridge
cannot reach. `W3C` does not impose this, because no arm consumes it and imposing
it would shrink the model class for nothing; this witness satisfies it anyway,
and says so here. -/
theorem live_is_bridge_coherent : ∀ p x y : LiveD, live.iext p x y → liveIP p :=
  live_is_bridge_coherent_raw

/-! ### Every one of the fourteen arms fires in this model

Non-vacuity of a FIELD is weaker than non-vacuity of an ARM. A field can have
something in its extension while the derivation that consumes it never has all
its premises met at once. The twelve theorems below apply each derivation of
`W3C.lean` at a concrete instance of this model and land on a triple the tables
contain, which is the stronger statement and the one the reader wants: every arm
that used to be assumed is now discharged over premises this structure actually
satisfies.

`eqc` and `eqp` carry two arms each, so twelve theorems cover fourteen arms. -/

/-- **rdfs11**, at `C2 ⊑ C1 ⊑ A2`. -/
theorem live_arm_sc_trans : live.sc .c2 .a2 :=
  live_w3c.sc_trans .c2 .c1 .a2 (by decide : liveIext (liveι V.subClassOf) LiveD.c2 LiveD.c1 = true)
    (by decide : liveIext (liveι V.subClassOf) LiveD.c1 LiveD.a2 = true)

/-- **rdfs5**, at `p1 ⊑ q ⊑ p2`. -/
theorem live_arm_sp_trans : live.sp .p1 .p2 :=
  live_w3c.sp_trans .p1 .q .p2 (by decide : liveIext (liveι V.subPropertyOf) LiveD.p1 LiveD.q = true)
    (by decide : liveIext (liveι V.subPropertyOf) LiveD.q LiveD.p2 = true)

/-- **scm-eqc1**, both conclusions, at the two existential restrictions `S1` and
`S2`, which are distinct elements with the same class extension. -/
theorem live_arm_eqc : live.sc .s1 .s2 ∧ live.sc .s2 .s1 :=
  live_w3c.eqc .s1 .s2 (by decide : liveIext (liveι V.equivalentClass) LiveD.s1 LiveD.s2 = true)

/-- **scm-eqp1**, both conclusions, at `p1` and `q`, which are distinct
properties with the same extension. -/
theorem live_arm_eqp : live.sp .p1 .q ∧ live.sp .q .p1 :=
  live_w3c.eqp .p1 .q (by decide : liveIext (liveι V.equivalentProperty) LiveD.p1 LiveD.q = true)

/-- **scm-svf1**, at `∃p2.Y ⊑ ∃p2.C1`, ordered because `Y ⊑ C1`. -/
theorem live_arm_svf_sc : live.sc .s1 .s2 :=
  live_w3c.svf_sc .s1 .s2 .p2 .filler .c1
    (by decide : liveIext (liveι V.someValuesFrom) LiveD.s1 LiveD.filler = true)
    (by decide : liveIext (liveι V.onProperty) LiveD.s1 LiveD.p2 = true)
    (by decide : liveIext (liveι V.someValuesFrom) LiveD.s2 LiveD.c1 = true)
    (by decide : liveIext (liveι V.onProperty) LiveD.s2 LiveD.p2 = true)
    (by decide : liveIext (liveι V.subClassOf) LiveD.filler LiveD.c1 = true)

/-- **scm-svf2**, at `∃p1.Y ⊑ ∃p2.Y`, ordered because `p1 ⊑ p2`. -/
theorem live_arm_svf_sp : live.sc .s3 .s1 :=
  live_w3c.svf_sp .s3 .s1 .p1 .p2 .filler
    (by decide : liveIext (liveι V.someValuesFrom) LiveD.s3 LiveD.filler = true)
    (by decide : liveIext (liveι V.onProperty) LiveD.s3 LiveD.p1 = true)
    (by decide : liveIext (liveι V.someValuesFrom) LiveD.s1 LiveD.filler = true)
    (by decide : liveIext (liveι V.onProperty) LiveD.s1 LiveD.p2 = true)
    (by decide : liveIext (liveι V.subPropertyOf) LiveD.p1 LiveD.p2 = true)

/-- **scm-avf1**, at `∀p2.Y ⊑ ∀p2.C1`, ordered because `Y ⊑ C1`. A universal
restriction is MONOTONE in its filler, which is why this runs the same way round
as the two above. -/
theorem live_arm_avf_sc : live.sc .c2 .a2 :=
  live_w3c.avf_sc .c2 .a2 .p2 .filler .c1
    (by decide : liveIext (liveι V.allValuesFrom) LiveD.c2 LiveD.filler = true)
    (by decide : liveIext (liveι V.onProperty) LiveD.c2 LiveD.p2 = true)
    (by decide : liveIext (liveι V.allValuesFrom) LiveD.a2 LiveD.c1 = true)
    (by decide : liveIext (liveι V.onProperty) LiveD.a2 LiveD.p2 = true)
    (by decide : liveIext (liveι V.subClassOf) LiveD.filler LiveD.c1 = true)

/-- **scm-avf2**, at the premises of `avfPremises` themselves, and the conclusion
is REVERSED: `C2 ⊑ C1` and not `C1 ⊑ C2`. This is the arm the whole file is
about. -/
theorem live_arm_avf_sp : live.sc .c2 .c1 :=
  live_w3c.avf_sp .c1 .c2 .p1 .p2 .filler
    (by decide : liveIext (liveι V.allValuesFrom) LiveD.c1 LiveD.filler = true)
    (by decide : liveIext (liveι V.onProperty) LiveD.c1 LiveD.p1 = true)
    (by decide : liveIext (liveι V.allValuesFrom) LiveD.c2 LiveD.filler = true)
    (by decide : liveIext (liveι V.onProperty) LiveD.c2 LiveD.p2 = true)
    (by decide : liveIext (liveι V.subPropertyOf) LiveD.p1 LiveD.p2 = true)

/-- **scm-dom1**, widening `p1`'s domain from `S1` to `S2`. -/
theorem live_arm_dom_sc : live.iext (live.ι V.domain) .p1 .s2 :=
  live_w3c.dom_sc .p1 .s1 .s2 (by decide : liveIext (liveι V.domain) LiveD.p1 LiveD.s1 = true)
    (by decide : liveIext (liveι V.subClassOf) LiveD.s1 LiveD.s2 = true)

/-- **scm-dom2**, inheriting `p2`'s domain `S1` down to the subproperty `p1`. -/
theorem live_arm_dom_sp : live.iext (live.ι V.domain) .p1 .s1 :=
  live_w3c.dom_sp .p1 .p2 .s1 (by decide : liveIext (liveι V.domain) LiveD.p2 LiveD.s1 = true)
    (by decide : liveIext (liveι V.subPropertyOf) LiveD.p1 LiveD.p2 = true)

/-- **scm-rng1**, widening `p1`'s range from `Y` to `C1`. -/
theorem live_arm_rng_sc : live.iext (live.ι V.range) .p1 .c1 :=
  live_w3c.rng_sc .p1 .filler .c1 (by decide : liveIext (liveι V.range) LiveD.p1 LiveD.filler = true)
    (by decide : liveIext (liveι V.subClassOf) LiveD.filler LiveD.c1 = true)

/-- **scm-rng2**, inheriting `p2`'s range `C2` down to the subproperty `p1`. -/
theorem live_arm_rng_sp : live.iext (live.ι V.range) .p1 .c2 :=
  live_w3c.rng_sp .p1 .p2 .c2 (by decide : liveIext (liveι V.range) LiveD.p2 LiveD.c2 = true)
    (by decide : liveIext (liveι V.subPropertyOf) LiveD.p1 LiveD.p2 = true)

/-- **All fourteen, in one statement**, so that a later edit that quietly stops
exercising one fails here. -/
theorem live_exercises_every_arm :
    live.sc .c2 .a2 ∧ live.sp .p1 .p2 ∧
    (live.sc .s1 .s2 ∧ live.sc .s2 .s1) ∧ (live.sp .p1 .q ∧ live.sp .q .p1) ∧
    live.sc .s1 .s2 ∧ live.sc .s3 .s1 ∧ live.sc .c2 .a2 ∧ live.sc .c2 .c1 ∧
    live.iext (live.ι V.domain) .p1 .s2 ∧ live.iext (live.ι V.domain) .p1 .s1 ∧
    live.iext (live.ι V.range) .p1 .c1 ∧ live.iext (live.ι V.range) .p1 .c2 :=
  ⟨live_arm_sc_trans, live_arm_sp_trans, live_arm_eqc, live_arm_eqp,
   live_arm_svf_sc, live_arm_svf_sp, live_arm_avf_sc, live_arm_avf_sp,
   live_arm_dom_sc, live_arm_dom_sp, live_arm_rng_sc, live_arm_rng_sp⟩

/-! ### And nothing is left vacuous, which took a second rebuild to get to

Every one of the twenty-one fields of `W3C` has a SATISFIED ANTECEDENT in this
model. `live_fires_every_field` is that sentence as a theorem, one exhibited
antecedent per field, so the count cannot drift from the tables without the build
going red.

**The 15 September 2026 rebuild left five fields vacuous and said so; this is the
second rebuild, which closes them.** The five were `same_fwd`, `sym_fwd`,
`trp_fwd`, `inv_fwd` and `hv_eq`, and the note beside them said two things. The
first was that closing the other four would cost more carrier than the `decide`
budget allowed. That was a budget, not an obstruction, and it is paid here:
`sy`, `tr`, `r1`, `H1` and the five vocabulary elements that had been falling
through to `other` bring the carrier to thirty-five, the four conditions are
exercised, and the two `set_option` lines earlier in the file are what that
costs, with the reason written beside them.

The second thing it said was wrong, and it is the kind of wrong this file exists
to catch. It said `same_fwd` "cannot be exercised non-trivially by ANY model at
all because the specification makes that relation the diagonal". The premise is
right and the conclusion does not follow. RBS Table 5.9 row 1 is
`( a₁ , a₂ ) ∈ IEXT(I(owl:sameAs))` **iff** `a₁ = a₂`, the `iff` cell carrying
`rowspan="6"`, and the specification's own conventions section reads an unscoped
variable as ranging over IR: "If no explicit scope is given for a variable `x`
... then `x` is unconstrained, which means x ∈ IR". So a conforming
interpretation's `owl:sameAs` extension is the WHOLE diagonal on IR, and RBS
section 4.2 defines IR as "the universe of I, i.e., a nonempty set", so that
extension is never empty. The first version of this model gave `owl:sameAs` the
EMPTY extension, which no
conforming interpretation has, and then reported the emptiness as a fact about
the specification rather than a defect in the model. Both quotes were re-read in
the raw HTML of <https://www.w3.org/TR/owl2-rdf-based-semantics/> on 15 September
2026.

What is true is narrower and is proved below rather than asserted:
`same_fwd` has no instance at `a ≠ b` in any interpretation meeting that cell.
`sameAs_has_no_off_diagonal_instance` is that theorem, and it is a statement
about every interpretation rather than about this one, which is why it is stated
over an arbitrary `Interp`. The condition can be exercised, it is exercised here
at thirty-five instances, and every one of them is a diagonal pair. That is the
whole of the residual limitation, and it is one line rather than five. -/

/-- **All twenty-one fields fire**, one satisfied antecedent each. Nothing in
`W3C` holds here for want of anything to check.

`sc_fwd`, `sp_fwd`, `dom_fwd`, `rng_fwd`, `eqc_fwd`, `eqp_fwd`, `same_fwd`,
`inv_fwd`, `sym_fwd`, `trp_fwd`, `svf_eq`, `svf_typ`, `avf_eq`, `avf_typ`,
`hv_eq`, `onp_typ` and `restr_IC` are given a pair in the extension their
antecedent reads; `sc_bwd`, `sp_bwd`, `dom_bwd` and `rng_bwd` are given the whole
antecedent, guards and universally quantified clause together, because those are
the four whose hypotheses can be met for want of anything to check and were. -/
theorem live_fires_every_field_raw :
    liveIext .sco .c2 .c1 = true ∧
    (liveIext .ty .c2 .cls = true ∧ liveIext .ty .c1 .cls = true ∧
      ∀ x, liveIext .ty x .c2 = true → liveIext .ty x .c1 = true) ∧
    liveIext .spo .p1 .p2 = true ∧
    (liveIP .p1 ∧ liveIP .p2 ∧
      ∀ x y, liveIext .p1 x y = true → liveIext .p2 x y = true) ∧
    liveIext (liveι V.domain) .p2 .c1 = true ∧
    (liveIP .p2 ∧ liveIext .ty .c1 .cls = true ∧
      ∀ x y, liveIext .p2 x y = true → liveIext .ty x .c1 = true) ∧
    liveIext (liveι V.range) .p2 .c2 = true ∧
    (liveIP .p2 ∧ liveIext .ty .c2 .cls = true ∧
      ∀ x y, liveIext .p2 x y = true → liveIext .ty y .c2 = true) ∧
    liveIext (liveι V.equivalentClass) .s1 .s2 = true ∧
    liveIext (liveι V.equivalentProperty) .p1 .q = true ∧
    liveIext (liveι V.sameAs) .alice .alice = true ∧
    liveIext (liveι V.inverseOf) .p1 .r1 = true ∧
    liveIext .ty .sy (liveι V.symmetricProperty) = true ∧
    liveIext .ty .tr (liveι V.transitiveProperty) = true ∧
    (liveIext (liveι V.someValuesFrom) .s1 .filler = true ∧
      liveIext (liveι V.onProperty) .s1 .p2 = true) ∧
    (liveIext (liveι V.allValuesFrom) .c1 .filler = true ∧
      liveIext (liveι V.onProperty) .c1 .p1 = true) ∧
    (liveIext (liveι V.hasValue) .h1 .bob = true ∧
      liveIext (liveι V.onProperty) .h1 .p2 = true) ∧
    liveIext .ty .c1 (liveι V.Restriction) = true := by decide

/-- The same twenty-one over `live`, by definitional unfolding. -/
theorem live_fires_every_field :
    live.sc .c2 .c1 ∧
    (live.IC .c2 ∧ live.IC .c1 ∧ ∀ x, live.cext .c2 x → live.cext .c1 x) ∧
    live.sp .p1 .p2 ∧
    (liveIP .p1 ∧ liveIP .p2 ∧ ∀ x y, live.iext .p1 x y → live.iext .p2 x y) ∧
    live.iext (live.ι V.domain) .p2 .c1 ∧
    (liveIP .p2 ∧ live.IC .c1 ∧ ∀ x y, live.iext .p2 x y → live.cext .c1 x) ∧
    live.iext (live.ι V.range) .p2 .c2 ∧
    (liveIP .p2 ∧ live.IC .c2 ∧ ∀ x y, live.iext .p2 x y → live.cext .c2 y) ∧
    live.iext (live.ι V.equivalentClass) .s1 .s2 ∧
    live.iext (live.ι V.equivalentProperty) .p1 .q ∧
    live.iext (live.ι V.sameAs) .alice .alice ∧
    live.iext (live.ι V.inverseOf) .p1 .r1 ∧
    live.cext (live.ι V.symmetricProperty) .sy ∧
    live.cext (live.ι V.transitiveProperty) .tr ∧
    (live.iext (live.ι V.someValuesFrom) .s1 .filler ∧
      live.iext (live.ι V.onProperty) .s1 .p2) ∧
    (live.iext (live.ι V.allValuesFrom) .c1 .filler ∧
      live.iext (live.ι V.onProperty) .c1 .p1) ∧
    (live.iext (live.ι V.hasValue) .h1 .bob ∧
      live.iext (live.ι V.onProperty) .h1 .p2) ∧
    live.cext (live.ι V.Restriction) .c1 :=
  live_fires_every_field_raw

/-- RBS Table 5.9's `owl:sameAs` cell, both directions, as a property of an
arbitrary interpretation. `W3C.same_fwd` is its left-to-right half and is all any
rule consumes; this is the whole cell, and it is stated here rather than added to
`W3C` because adding it would SHRINK the model class for no rule's benefit. -/
def SameAsIsTheDiagonal (I : Interp) : Prop :=
  ∀ a b : I.D, I.iext (I.ι V.sameAs) a b ↔ a = b

/-- **The residual limitation on `same_fwd`, as a theorem about every
interpretation rather than a remark about this one.**

In any interpretation meeting the cell, `same_fwd`'s antecedent is met exactly on
the diagonal, so the field can be exercised and can never be exercised at
`a ≠ b`. That is a fact about the specification: the cell is an `iff` and its
right-hand side is an equation, so there is nothing for a model builder to
choose. It is NOT the claim an earlier draft of this file made, that the field
cannot be exercised at all; it is exercised here thirty-five times. -/
theorem sameAs_has_no_off_diagonal_instance {I : Interp} (h : SameAsIsTheDiagonal I)
    {a b : I.D} (hne : a ≠ b) : ¬ I.iext (I.ι V.sameAs) a b :=
  fun hab => hne ((h a b).mp hab)

theorem live_sameAs_is_the_diagonal_raw :
    ∀ a b : LiveD, liveIext (liveι V.sameAs) a b = true ↔ a = b := by decide

/-- And this model meets the cell in BOTH directions, not just the forward half
`W3C` states. The diagonal is what a conforming interpretation has there, so
putting it in moves the model towards conformance rather than away from it. -/
theorem live_sameAs_is_the_diagonal : SameAsIsTheDiagonal live :=
  live_sameAs_is_the_diagonal_raw

theorem live_c1_is_a_class : liveIext .ty .c1 .cls = true := by decide
theorem live_c2_is_a_class : liveIext .ty .c2 .cls = true := by decide
theorem live_p2_is_in_IP : liveIP .p2 := by decide

/-- The universally quantified clause `dom_bwd` demands, at `C1`, discharged
over the pairs of `p2` and NOT by there being no pairs. -/
theorem live_p2_subjects_are_c1 :
    ∀ x y : LiveD, liveIext .p2 x y = true → liveIext .ty x .c1 = true := by decide

/-- The same clause for `rng_bwd`, at `C2`. `alice` is outside `ICEXT(C2)` and
both of her `p2`-successors are inside it, which is what makes the domain and
range extensions differ. -/
theorem live_p2_objects_are_c2 :
    ∀ x y : LiveD, liveIext .p2 x y = true → liveIext .ty y .c2 = true := by decide

theorem live_p2_is_not_a_domain_of_c2 : ¬ liveIext (liveι V.domain) .p2 .c2 = true := by decide

/-- `dom_bwd` is discharged over a NON-EMPTY relation: the clause it demands is
proved at `p2`'s two pairs and not by there being no pairs. The second kernel's
`M3` does not manage that, which is what its own `M4` was built for;
`isabelle/OO_NonVacuity.thy`'s `M4_ante_dom_bwd` is the counterpart of this
theorem. -/
theorem live_dom_bwd_is_exercised : live.iext (live.ι V.domain) .p2 .c1 :=
  live_w3c.dom_bwd .p2 .c1 live_p2_is_in_IP live_c1_is_a_class live_p2_subjects_are_c1

/-- `rng_bwd`, likewise, and at a class `dom_bwd` does NOT reach. The two
backward halves of Table 5.8 are separated by this model rather than satisfied
together by accident: `(p2, C2)` is in the range extension and not in the domain
extension, because both of `alice`'s `p2`-successors are in `ICEXT(C2)` and
`alice` herself is not. -/
theorem live_rng_bwd_is_exercised :
    live.iext (live.ι V.range) .p2 .c2 ∧ ¬ live.iext (live.ι V.domain) .p2 .c2 :=
  ⟨live_w3c.rng_bwd .p2 .c2 live_p2_is_in_IP live_c2_is_a_class live_p2_objects_are_c2,
   live_p2_is_not_a_domain_of_c2⟩

/-! ## What it refutes, and what it confirms -/

/-- **The direction the W3C table gives, over this model class.** `scm-avf2`
concludes `C2 rdfs:subClassOf C1`, and that is true in every `W3CModel` of these
premises. Transferred from the existing result rather than reproved, which is
the whole point of `W3CEntails.of_entails`. -/
theorem the_avf2_direction_the_table_gives_is_w3c_entailed :
    W3CEntails avfPremises ⟨tC2, V.subClassOf, tC1⟩ :=
  W3CEntails.of_entails the_avf2_direction_the_table_gives_is_entailed

theorem live_refutes_the_natural_direction :
    ¬ liveIext (liveι V.subClassOf) (liveι tC1) (liveι tC2) = true := by decide

/-- **And the natural-looking direction is refuted by a bridge-coherent
interpretation.**

`Witness.lean`'s `the_natural_avf2_direction_is_not_entailed` refutes the same
triple with a Herbrand interpretation, which satisfies `Conditions` and is not a
`W3CModel`: `avfWitness` contains no `rdf:type` triple at all, so every class
extension is empty there and `avf_typ` and `onp_typ` fail for want of an
`owl:Restriction` typing, `onp_typ` failing a second time on its `IP p` conjunct,
which the empty `IP` cannot supply. Unlike the four non-entailments this file and
`Mixed.lean` transfer, this one genuinely needed a structure built for it. That
is why the structure below exists; it is not a general fact about Herbrand
witnesses, which an earlier version of this paragraph turned it into.

This one is about a class much closer to the specification's: the refuting
structure satisfies Table 5.8 in both directions, Table 5.6's `someValuesFrom`
and `allValuesFrom` equalities, Tables 5.9, 5.12 and 5.13, and the typing rows of
Tables 5.2 and 5.3, and it is bridge-coherent. Every one of the fourteen arms
fires in it at a concrete instance (`live_exercises_every_arm`), and the premise
`p1 rdfs:subPropertyOf p2` holds because `IEXT(p1)` is a proper non-empty subset
of `IEXT(p2)` rather than because `IEXT(p1)` is empty, which is what the first
version of this model got wrong. A rule concluding `C1 rdfs:subClassOf C2` from
these premises, which is what copying the shape of `scm-avf1`, `scm-svf1` and
`scm-svf2` gives, would be making a claim that such a structure refutes.

**Read the module docstring before quoting this anywhere.** It is NOT a proof
that the triple fails to be OWL 2 RDF-Based entailed: `W3CModel`'s class is
larger than the conforming interpretations, because `W3C` omits every row no
rule consumes, and this model violates Table 5.2's `owl:Thing | = IR` and
`rdf:Property | = IP` rows among others. -/
theorem the_natural_avf2_direction_is_not_w3c_entailed :
    ¬ W3CEntails avfPremises ⟨tC1, V.subClassOf, tC2⟩ := fun h =>
  absurd (h live liveIP live_is_a_w3c_model) live_refutes_the_natural_direction

/-- The pair, stated together: the rule's own conclusion holds in every
`W3CModel` and the reversed one fails in this one. A checker that had `scm-avf2`
the natural way round would be accepting certificates whose conclusions this
structure makes false, and that is now a machine-checked sentence rather than an
argument in a docstring. -/
theorem scm_avf2_runs_one_way_under_the_specification :
    W3CEntails avfPremises ⟨tC2, V.subClassOf, tC1⟩ ∧
      ¬ W3CEntails avfPremises ⟨tC1, V.subClassOf, tC2⟩ :=
  ⟨the_avf2_direction_the_table_gives_is_w3c_entailed,
   the_natural_avf2_direction_is_not_w3c_entailed⟩

/-! ## The degenerate model, proved rather than asserted

`Witness.lean` says in English, beside `saturated_is_a_model`, that `saturated`
also satisfies the conditions in `W3C.lean`. An unproved claim in a docstring is
the defect this whole file exists to remove, so it is a theorem here. -/

/-- **`W3C` is satisfiable trivially, and that is exactly why bare satisfiability
is not the gate.** `saturated` has domain `Unit` and every relation total, so
both sides of every equality in `W3C` are all of `Unit` and every field holds
without saying anything. It distinguishes no condition from any other and
refutes nothing.

Note what it does NOT extend to. `W3CModel` would additionally demand
`oneOf_eq`, and that fails here whenever the graph carries an `owl:oneOf` triple
whose list is EMPTY: the left side is `ICEXT(c)`, which is all of `Unit`, and the
right side is `∃ m ∈ [], x = I(m)`, which is false. Table 5.5's equality forces an
empty enumeration to denote the empty class, and a model in which every class is
the whole universe cannot do that. `Conditions.oneOf` carries only the `⊇` half
and so never notices. That is the same asymmetry recorded at `Model.oneOf` in
`Semantics.lean`, seen from the other side. -/
theorem saturated_meets_the_w3c_conditions : W3C saturated (fun _ => True) where
  sc_fwd := fun _ _ _ => ⟨trivial, trivial, fun _ _ => trivial⟩
  sc_bwd := fun _ _ _ _ _ => trivial
  sp_fwd := fun _ _ _ => ⟨trivial, trivial, fun _ _ _ => trivial⟩
  sp_bwd := fun _ _ _ _ _ => trivial
  dom_fwd := fun _ _ _ => ⟨trivial, trivial, fun _ _ _ => trivial⟩
  dom_bwd := fun _ _ _ _ _ => trivial
  rng_fwd := fun _ _ _ => ⟨trivial, trivial, fun _ _ _ => trivial⟩
  rng_bwd := fun _ _ _ _ _ => trivial
  eqc_fwd := fun _ _ _ => ⟨trivial, trivial, fun _ => Iff.rfl⟩
  eqp_fwd := fun _ _ _ => ⟨trivial, trivial, fun _ _ => Iff.rfl⟩
  same_fwd := fun a b _ => by cases a; cases b; rfl
  inv_fwd := fun _ _ _ => ⟨trivial, trivial, fun _ _ => Iff.rfl⟩
  sym_fwd := fun _ _ _ _ _ => trivial
  trp_fwd := fun _ _ _ _ _ _ _ => trivial
  svf_eq := fun _ _ _ _ _ _ => ⟨fun _ => ⟨(), trivial, trivial⟩, fun _ => trivial⟩
  avf_eq := fun _ _ _ _ _ _ => ⟨fun _ _ _ => trivial, fun _ => trivial⟩
  hv_eq := fun _ _ _ _ _ _ => Iff.rfl
  restr_IC := fun _ _ => trivial
  svf_typ := fun _ _ _ => ⟨trivial, trivial⟩
  avf_typ := fun _ _ _ => ⟨trivial, trivial⟩
  onp_typ := fun _ _ _ => ⟨trivial, trivial⟩

/-- A one-triple graph enumerating nothing: `E owl:oneOf rdf:nil`. -/
def emptyOneOfG : List Triple := [⟨"<e:E>", V.oneOf, V.nil⟩]

/-- **And the gap in the docstring above is a theorem too.** `saturated` meets
every field of `W3C`, and it is still not a `W3CModel` of this graph, because
Table 5.5's equality forces an empty enumeration to denote the empty class while
`saturated` makes every class the whole universe.

So `W3C` and `W3CModel` genuinely differ, the list rows are not decoration, and
"`saturated` satisfies the specification conditions" is true of the conditions
and false of the models. `Conditions.oneOf` carries only the `⊇` half and never
notices, which is why `saturated_is_a_model` holds for EVERY graph including
this one. -/
theorem saturated_is_not_a_w3c_model_of_an_empty_enumeration :
    ¬ W3CModel saturated (fun _ => True) emptyOneOfG := by
  intro W
  have h := (W.oneOf_eq "<e:E>" V.nil [] (by simp [emptyOneOfG]) Chain.nil ()).mp trivial
  simp at h

/-- The same graph, modelled by the weaker conditions without complaint. The two
sit side by side on purpose: this is what the `⊆` half of Table 5.5 buys, and
what leaving it out costs. -/
theorem saturated_is_a_model_of_the_same_graph : Model saturated emptyOneOfG :=
  saturated_is_a_model emptyOneOfG

/-! ## And `W3CEntails` is not everything either

`live` closes one vacuity hole: `W3C` has a non-degenerate model, so
`W3CModel.toModel` is not a theorem about an empty class. The other hole is that
`W3CEntails` could hold of every triple, which would make
`certificate_w3c_sound` say nothing at all. It does not, and the cheapest
witness closes it: over the EMPTY graph every antecedent in `W3C` is false, and
`IP := fun _ => False` makes the four backward conditions vacuous too. -/

/-- The Herbrand interpretation of the empty graph is a `W3CModel` of it,
with an empty `IP`. Bridge-coherent for free: `iext` is empty, so nothing has to
be in `IP`. -/
theorem empty_herbrand_is_a_w3c_model : W3CModel (herbrand []) (fun _ => False) [] where
  conds :=
    { sc_fwd := fun a b hab => absurd hab (not_mem_pred [] V.subClassOf (by simp) a b)
      sc_bwd := fun a b ha => absurd ha (not_typed [] V.Class (by simp) a)
      sp_fwd := fun a b hab => absurd hab (not_mem_pred [] V.subPropertyOf (by simp) a b)
      sp_bwd := fun _ _ h => absurd h (fun x => x)
      dom_fwd := fun p c hpc => absurd hpc (not_mem_pred [] V.domain (by simp) p c)
      dom_bwd := fun _ _ h => absurd h (fun x => x)
      rng_fwd := fun p c hpc => absurd hpc (not_mem_pred [] V.range (by simp) p c)
      rng_bwd := fun _ _ h => absurd h (fun x => x)
      eqc_fwd := fun a b hab => absurd hab (not_mem_pred [] V.equivalentClass (by simp) a b)
      eqp_fwd := fun a b hab => absurd hab (not_mem_pred [] V.equivalentProperty (by simp) a b)
      same_fwd := fun a b hab => absurd hab (not_mem_pred [] V.sameAs (by simp) a b)
      inv_fwd := fun p q hpq => absurd hpq (not_mem_pred [] V.inverseOf (by simp) p q)
      sym_fwd := fun p hp => absurd hp (not_typed [] V.symmetricProperty (by simp) p)
      trp_fwd := fun p hp => absurd hp (not_typed [] V.transitiveProperty (by simp) p)
      svf_eq := fun z c _ h => absurd h (not_mem_pred [] V.someValuesFrom (by simp) z c)
      avf_eq := fun z c _ h => absurd h (not_mem_pred [] V.allValuesFrom (by simp) z c)
      hv_eq := fun z a _ h => absurd h (not_mem_pred [] V.hasValue (by simp) z a)
      restr_IC := fun x hx => absurd hx (not_typed [] V.Restriction (by simp) x)
      svf_typ := fun z c h => absurd h (not_mem_pred [] V.someValuesFrom (by simp) z c)
      avf_typ := fun z c h => absurd h (not_mem_pred [] V.allValuesFrom (by simp) z c)
      onp_typ := fun z p h => absurd h (not_mem_pred [] V.onProperty (by simp) z p) }
  facts := by intro t ht; simp at ht
  int_eq := by intro c l _ hc; exact absurd hc (not_mem_pred [] V.intersectionOf (by simp) c l)
  uni_eq := by intro c l _ hc; exact absurd hc (not_mem_pred [] V.unionOf (by simp) c l)
  oneOf_eq := by intro c l _ hc; exact absurd hc (not_mem_pred [] V.oneOf (by simp) c l)

/-- **`W3CEntails` is not trivial**, so `certificate_w3c_sound` is not a theorem
about a relation that holds of everything. This is obligation (2) of
`Witness.lean`'s own header, restated over the `W3CModel` class.

An earlier version of this docstring called it "the one obligation there that
transfers without a new hand-built structure". It is the FIRST, not the only:
`an_unlisted_individual_is_not_w3c_entailed`,
`membership_in_one_member_is_not_w3c_enough` and `Mixed.lean`'s
`mix_not_absolutely_w3c_entailed` transfer the same way, and this file used to
list all three among the results it could not reach. -/
theorem not_everything_is_w3c_entailed :
    ¬ W3CEntails [] ⟨"<http://ex.org/a>", "<http://ex.org/b>", "<http://ex.org/c>"⟩ := by
  intro h
  have hs := h (herbrand []) (fun _ => False) empty_herbrand_is_a_w3c_model
  simp at hs


/-! ## Two more non-entailments here, a third in `Mixed.lean`, on the interpretations they already had

`IP` is a free PARAMETER of `W3C`, not a field of `Interp`, and the four
backward conditions are the only fields that consume it as a hypothesis. Choose
`IP := fun _ => False` and `sp_bwd`, `dom_bwd` and `rng_bwd` become vacuous,
because their antecedents `IP a`, `IP p` have no instances. `sc_bwd` is guarded
by `IC` instead, and over a Herbrand interpretation `I.IC a` is
`⟨a, rdf:type, rdfs:Class⟩ ∈ H`, so a witness graph carrying no `rdfs:Class`
typing makes THAT field vacuous too.

An earlier draft of this file, of `Witness.lean` and of `Semantics.lean` claimed
the opposite in five places: that an empty `IC` makes Table 5.8's backward
direction FORCE `rdfs:subClassOf` and `rdfs:subPropertyOf` triples the witness
lacks, and that every conforming countermodel has to be a hand-built finite
structure. Both are false, and the first is inverted: an empty `IC` is exactly
what makes `sc_bwd` vacuous. The claim also silently promoted
`live_is_bridge_coherent`, which is an EXTRA property the `live` model happens
to have, into a requirement of `W3CModel`, which it is not.

FOUR of the results listed there transfer on the nose, with the existing witness
graphs unchanged and nothing hand-built: two proved in this section, one in
`Mixed.lean`, and one that was already proved further up THIS file at the moment
the list called it open. The rest needed a finite structure each, and have one.

| result | how it transfers |
|---|---|
| `not_everything_is_entailed` | on its own witness: `not_everything_is_w3c_entailed`, below |
| `an_unlisted_individual_is_not_entailed` | on its own witness: `an_unlisted_individual_is_not_w3c_entailed` |
| `membership_in_one_member_does_not_give_the_intersection` | on its own witness: `membership_in_one_member_is_not_w3c_enough` |
| `mix_not_absolutely_entailed` | on its own witness: `mix_not_absolutely_w3c_entailed`, in `Mixed.lean` |
| `the_old_svf_derivation_is_not_entailed` | on `svfI`, built for it: `the_old_svf_derivation_is_not_w3c_entailed`, at the end of this file |
| `feed_is_not_refuted` | on `refI false`, built for it: `feed_is_not_w3c_refuted`, in `RefuteWitness.lean` |
| `and_the_old_verdict_does_not_notice` | on `refI true`, built for it: `the_old_verdict_does_not_notice_over_w3c`, in `RefuteWitness.lean` |
| `not_unsat_of_joint_model` | by unfolding: `not_w3cUnsat_of_joint_w3c_model`, in `RefuteWitness.lean` |

The three at the bottom of that table used to read "no", with the field that
fails pinned as a checked theorem. Those theorems said the HERBRAND witness is
not a `W3CModel`, which is true and is a reason to build a different structure
rather than a reason the result is out of reach.

The two `owl:oneOf` and `owl:intersectionOf` graphs go through because the list
field of `W3CModel` is an `iff` and their witnesses satisfy it in both
directions: `ooWitness` types exactly the two listed members as `E`, and
`intWitness` makes exactly `w` a `K` and exactly `w` both an `M1` and an `M2`.
`v` is an `M1` and not an `M2`, which is what keeps `int_eq` true and the
conclusion false at the same time.
-/

/-- Exactly the two listed members are typed `E`. Decidable over a seven-triple
list, which is what lets `oneOf_eq`'s forward direction be discharged without
case analysis on string literals. -/
private theorem oo_typed_E :
    ∀ t ∈ ooWitness, t.p = V.type → t.o = tE → t.s = tA ∨ t.s = tB := by decide

/-- **The `owl:oneOf` witness is a `W3CModel`.** `ooWitness` carries no
`rdfs:Class` typing, so `IC` is empty and `sc_bwd` is vacuous; with
`IP := fun _ => False` the other three backward fields are vacuous as well; and
every forward field has an antecedent this graph never satisfies.

Table 5.5's equality is the one field that has to be EARNED rather than
dodged, and this witness satisfies it in both directions, because `ICEXT(E)` is
exactly `{a, b}`. `Conditions.oneOf` carries only the `⊇` half, which is the
asymmetry `an_unlisted_individual_is_not_entailed`'s docstring records. -/
theorem oo_witness_is_a_w3c_model :
    W3CModel (herbrand ooWitness) (fun _ => False) ooPremises where
  conds :=
    { sc_fwd := fun a b h => absurd h (not_mem_pred ooWitness V.subClassOf (by decide) a b)
      sc_bwd := fun a _ ha => absurd ha (not_typed ooWitness V.Class (by decide) a)
      sp_fwd := fun a b h => absurd h (not_mem_pred ooWitness V.subPropertyOf (by decide) a b)
      sp_bwd := fun _ _ h => absurd h (fun x => x)
      dom_fwd := fun p c h => absurd h (not_mem_pred ooWitness V.domain (by decide) p c)
      dom_bwd := fun _ _ h => absurd h (fun x => x)
      rng_fwd := fun p c h => absurd h (not_mem_pred ooWitness V.range (by decide) p c)
      rng_bwd := fun _ _ h => absurd h (fun x => x)
      eqc_fwd := fun a b h => absurd h (not_mem_pred ooWitness V.equivalentClass (by decide) a b)
      eqp_fwd := fun a b h =>
        absurd h (not_mem_pred ooWitness V.equivalentProperty (by decide) a b)
      same_fwd := fun a b h => absurd h (not_mem_pred ooWitness V.sameAs (by decide) a b)
      inv_fwd := fun p q h => absurd h (not_mem_pred ooWitness V.inverseOf (by decide) p q)
      sym_fwd := fun p h => absurd h (not_typed ooWitness V.symmetricProperty (by decide) p)
      trp_fwd := fun p h => absurd h (not_typed ooWitness V.transitiveProperty (by decide) p)
      svf_eq := fun z c _ h => absurd h (not_mem_pred ooWitness V.someValuesFrom (by decide) z c)
      avf_eq := fun z c _ h => absurd h (not_mem_pred ooWitness V.allValuesFrom (by decide) z c)
      hv_eq := fun z a _ h => absurd h (not_mem_pred ooWitness V.hasValue (by decide) z a)
      restr_IC := fun x h => absurd h (not_typed ooWitness V.Restriction (by decide) x)
      svf_typ := fun z c h => absurd h (not_mem_pred ooWitness V.someValuesFrom (by decide) z c)
      avf_typ := fun z c h => absurd h (not_mem_pred ooWitness V.allValuesFrom (by decide) z c)
      onp_typ := fun z p h => absurd h (not_mem_pred ooWitness V.onProperty (by decide) z p) }
  facts := fun t ht => List.mem_cons_of_mem _ (List.mem_cons_of_mem _ ht)
  int_eq := fun c l _ hc => absurd hc (not_mem_pred ooPremises V.intersectionOf (by decide) c l)
  uni_eq := fun c l _ hc => absurd hc (not_mem_pred ooPremises V.unionOf (by decide) c l)
  oneOf_eq := by
    intro c l ms hc hchain x
    obtain ⟨rfl, rfl⟩ := oo_only_oneOf _ hc rfl
    rw [oo_chain_l0 ms hchain]
    constructor
    · intro hx
      rcases oo_typed_E ⟨x, V.type, tE⟩ hx rfl rfl with rfl | rfl
      · exact ⟨tA, by simp, rfl⟩
      · exact ⟨tB, by simp, rfl⟩
    · rintro ⟨m, hm, rfl⟩
      rcases List.mem_cons.mp hm with rfl | hm
      · show (⟨tA, V.type, tE⟩ : Triple) ∈ ooWitness
        decide
      · rcases List.mem_cons.mp hm with rfl | hm
        · show (⟨tB, V.type, tE⟩ : Triple) ∈ ooWitness
          decide
        · cases hm

/-- **`cls-oo` stops at the members, over the `W3CModel` class.** The same
statement as `an_unlisted_individual_is_not_entailed` with `W3CEntails` for
`Entails`, and the same interpretation discharges it. -/
theorem an_unlisted_individual_is_not_w3c_entailed :
    ¬ W3CEntails ooPremises ⟨tZ, V.type, tE⟩ := fun h =>
  absurd (h (herbrand ooWitness) (fun _ => False) oo_witness_is_a_w3c_model)
    (by decide : (⟨tZ, V.type, tE⟩ : Triple) ∉ ooWitness)

/-- Whatever this graph makes a `K` it already makes an `M1` and an `M2`. -/
private theorem int_K_gives_both : ∀ t ∈ intWitness, t.p = V.type → t.o = tK →
    (⟨t.s, V.type, tM1⟩ : Triple) ∈ intWitness ∧
      (⟨t.s, V.type, tM2⟩ : Triple) ∈ intWitness := by decide

/-- And whatever it makes both it already makes a `K`. `v` is an `M1` and not an
`M2`, so it never triggers this, which is the whole content of the theorem
below. -/
private theorem int_both_give_K : ∀ t ∈ intWitness, ∀ u ∈ intWitness,
    (t.p = V.type ∧ t.o = tM1 ∧ u.p = V.type ∧ u.o = tM2 ∧ u.s = t.s) →
      (⟨t.s, V.type, tK⟩ : Triple) ∈ intWitness := by decide

/-- **The `owl:intersectionOf` witness is a `W3CModel`.** Table 5.4's
equality holds in both directions here, not just the `⊆` half `Conditions.int`
and `Conditions.int2` carry between them. -/
theorem int_witness_is_a_w3c_model :
    W3CModel (herbrand intWitness) (fun _ => False) intPremises where
  conds :=
    { sc_fwd := fun a b h => absurd h (not_mem_pred intWitness V.subClassOf (by decide) a b)
      sc_bwd := fun a _ ha => absurd ha (not_typed intWitness V.Class (by decide) a)
      sp_fwd := fun a b h => absurd h (not_mem_pred intWitness V.subPropertyOf (by decide) a b)
      sp_bwd := fun _ _ h => absurd h (fun x => x)
      dom_fwd := fun p c h => absurd h (not_mem_pred intWitness V.domain (by decide) p c)
      dom_bwd := fun _ _ h => absurd h (fun x => x)
      rng_fwd := fun p c h => absurd h (not_mem_pred intWitness V.range (by decide) p c)
      rng_bwd := fun _ _ h => absurd h (fun x => x)
      eqc_fwd := fun a b h => absurd h (not_mem_pred intWitness V.equivalentClass (by decide) a b)
      eqp_fwd := fun a b h =>
        absurd h (not_mem_pred intWitness V.equivalentProperty (by decide) a b)
      same_fwd := fun a b h => absurd h (not_mem_pred intWitness V.sameAs (by decide) a b)
      inv_fwd := fun p q h => absurd h (not_mem_pred intWitness V.inverseOf (by decide) p q)
      sym_fwd := fun p h => absurd h (not_typed intWitness V.symmetricProperty (by decide) p)
      trp_fwd := fun p h => absurd h (not_typed intWitness V.transitiveProperty (by decide) p)
      svf_eq := fun z c _ h => absurd h (not_mem_pred intWitness V.someValuesFrom (by decide) z c)
      avf_eq := fun z c _ h => absurd h (not_mem_pred intWitness V.allValuesFrom (by decide) z c)
      hv_eq := fun z a _ h => absurd h (not_mem_pred intWitness V.hasValue (by decide) z a)
      restr_IC := fun x h => absurd h (not_typed intWitness V.Restriction (by decide) x)
      svf_typ := fun z c h => absurd h (not_mem_pred intWitness V.someValuesFrom (by decide) z c)
      avf_typ := fun z c h => absurd h (not_mem_pred intWitness V.allValuesFrom (by decide) z c)
      onp_typ := fun z p h => absurd h (not_mem_pred intWitness V.onProperty (by decide) z p) }
  facts := fun t ht => List.mem_cons_of_mem _ (List.mem_cons_of_mem _ ht)
  int_eq := by
    intro c l ms hc hchain x
    obtain ⟨rfl, rfl⟩ := i_only_int _ hc rfl
    rw [i_chain_l0 ms hchain]
    constructor
    · intro hx m hm
      obtain ⟨h1, h2⟩ := int_K_gives_both ⟨x, V.type, tK⟩ hx rfl rfl
      rcases List.mem_cons.mp hm with rfl | hm
      · exact h1
      · rcases List.mem_cons.mp hm with rfl | hm
        · exact h2
        · cases hm
    · intro hall
      exact int_both_give_K ⟨x, V.type, tM1⟩ (hall tM1 (by simp))
        ⟨x, V.type, tM2⟩ (hall tM2 (by simp)) ⟨rfl, rfl, rfl, rfl, rfl⟩
  uni_eq := fun c l _ hc => absurd hc (not_mem_pred intPremises V.unionOf (by decide) c l)
  oneOf_eq := fun c l _ hc => absurd hc (not_mem_pred intPremises V.oneOf (by decide) c l)

/-- **`cls-int1` still needs every member, over the `W3CModel` class.**
The same statement as `membership_in_one_member_does_not_give_the_intersection`
with `W3CEntails` for `Entails`. -/
theorem membership_in_one_member_is_not_w3c_enough :
    ¬ W3CEntails intPremises ⟨tv, V.type, tK⟩ := fun h =>
  absurd (h (herbrand intWitness) (fun _ => False) int_witness_is_a_w3c_model)
    (by decide : (⟨tv, V.type, tK⟩ : Triple) ∉ intWitness)

/-! ### And the last of the seven transfers, on a countermodel built for it

`the_old_svf_derivation_is_not_entailed` was the one non-entailment in
`Witness.lean` that the empty-`IP` reading could not carry over. The obstruction
was real and was recorded as a checked theorem: `svfWitness` carries
`R owl:onProperty p` and types `R` as nothing at all, so RBS Table 5.3's
`owl:onProperty` row fails on its first conjunct, `z ∈ ICEXT(owl:Restriction)`,
for EVERY choice of `IP`, and `svf_typ` and `sc_fwd` fail beside it.

What the obstruction showed was that the HERBRAND witness is not a `W3CModel`.
It did not show that no `W3CModel` refutes the triple, and the note beside it
said so: "what is open is whether a conforming interpretation also refutes it,
and nothing here suggests it does not". It does. `svfI` below is a hand-built
finite structure of the same kind as `live`, and it settles the question in the
direction the note expected.

The construction is the shortest one there is. `C rdfs:subClassOf R` needs
`ICEXT(C) ⊆ ICEXT(R)`, and the EMPTY set is contained in everything, so
`ICEXT(C) := ∅` satisfies the premise and refutes the conclusion at the same
time. Everything else is then forced: `R` is `∃p.D`, `IEXT(p) = {(x, y)}` and
`ICEXT(D) = {y}`, so Table 5.6 forces `ICEXT(R) = {x}`; `R` is typed
`owl:Restriction` and `C`, `D`, `R` are typed `rdfs:Class`, which is what
`svf_typ`, `onp_typ` and `restr_IC` demand and what the Herbrand witness could
not supply; and the four `rdfs:*` tables are the fixpoint Table 5.8's two
directions force from that.

`q` is the one element here that no premise asks for, and the reason it is
present is `sameAs_diagonal_needs_an_off_diagonal_subproperty` below. -/

/-- **Two distinct properties, one below the other, are FORCED on any model that
gives `owl:sameAs` the extension the specification gives it.**

This is not a remark about model building, it is a consequence of two table cells
meeting. Suppose `IEXT(rdfs:subPropertyOf)` were contained in the diagonal.
Table 5.9 row 1 then makes it a subset of `IEXT(owl:sameAs)`, so Table 5.8 row 2
BACKWARD puts the pair `(I(rdfs:subPropertyOf), I(owl:sameAs))` into
`IEXT(rdfs:subPropertyOf)` itself, and the diagonal assumption collapses that
pair, making the two IRIs denote the same thing.

The practical effect is that a small countermodel cannot have both a
`rdfs:subPropertyOf` extension that is exactly the diagonal on `IP` and a
non-empty `owl:sameAs`, which is why `svfI` carries a second property `q` with
`IEXT(q) = IEXT(p)`. Both are in `IP` by RBS Table 5.3, whose `I(E)` column reads
"∈ IP" for `owl:sameAs` and for `owl:onProperty`, re-read in the raw HTML of
<https://www.w3.org/TR/owl2-rdf-based-semantics/> on 15 September 2026. -/
theorem sameAs_diagonal_needs_an_off_diagonal_subproperty {I : Interp} {IP : I.D → Prop}
    (W : W3C I IP) (hd : SameAsIsTheDiagonal I)
    (hsa : IP (I.ι V.sameAs)) (hspo : IP (I.ι V.subPropertyOf))
    (hne : I.ι V.subPropertyOf ≠ I.ι V.sameAs)
    (hdiag : ∀ a b, I.sp a b → a = b) : False :=
  hne (hdiag _ _ (W.sp_bwd _ _ hspo hsa (fun x y hxy => (hd x y).mpr (hdiag x y hxy))))

/-- The eighteen elements of the countermodel for `svfPremises`. Two individuals;
three classes, of which one is a restriction; two properties with equal
extensions; ten vocabulary denotations that have to be told apart; and one junk
element. -/
inductive SvfD where
  /-- The subject of the asserted `x p y`, and the element the refuted triple
  claims is a `C`. -/
  | ex_x
  /-- Its `p`-successor, and the only member of `ICEXT(D)`. -/
  | ex_y
  /-- `C`, whose class extension is EMPTY. That is the refutation: the premise
  `C rdfs:subClassOf R` holds because the empty set is contained in `ICEXT(R)`,
  and `x rdf:type C` fails because nothing is in `ICEXT(C)`. -/
  | cC
  /-- `D`, the filler, with `ICEXT(D) = {y}`. -/
  | cD
  /-- `R`, the restriction `∃p.D`, with `ICEXT(R) = {x}` FORCED by Table 5.6. -/
  | cR
  /-- `p`, with the single pair `(x, y)`. -/
  | prop
  /-- A second property with `IEXT(q) = IEXT(p)`, present only so that
  `IEXT(rdfs:subPropertyOf)` is not the diagonal, which
  `sameAs_diagonal_needs_an_off_diagonal_subproperty` shows is what a non-empty
  `owl:sameAs` costs. -/
  | qq
  /-- `rdf:type`. -/
  | ty
  /-- `rdfs:subClassOf`. -/
  | sco
  /-- `rdfs:subPropertyOf`. -/
  | spo
  /-- `rdfs:domain`. -/
  | dm
  /-- `rdfs:range`. -/
  | rg
  /-- `owl:onProperty`. -/
  | onp
  /-- `owl:someValuesFrom`. -/
  | svf
  /-- `owl:sameAs`, carrying the diagonal, as Table 5.9 row 1 requires. -/
  | sa
  /-- `rdfs:Class`. -/
  | cls
  /-- `owl:Restriction`. -/
  | restr
  /-- Every other IRI, with both extensions empty. -/
  | other
deriving DecidableEq, Repr

namespace SvfD

/-- The carrier as a list, so quantification over it is decidable without
Mathlib. -/
def all : List SvfD :=
  [ex_x, ex_y, cC, cD, cR, prop, qq, ty, sco, spo, dm, rg, onp, svf, sa, cls, restr, other]

theorem mem_all (w : SvfD) : w ∈ all := by cases w <;> decide

instance decForall (p : SvfD → Prop) [DecidablePred p] : Decidable (∀ w, p w) :=
  decidable_of_iff (∀ w ∈ all, p w) ⟨fun h w => h w (mem_all w), fun h w _ => h w⟩

instance decExists (p : SvfD → Prop) [DecidablePred p] : Decidable (∃ w, p w) :=
  decidable_of_iff (∃ w ∈ all, p w)
    ⟨fun ⟨w, _, h⟩ => ⟨w, h⟩, fun ⟨w, h⟩ => ⟨w, mem_all w, h⟩⟩

end SvfD

/-- The denotations. `owl:allValuesFrom`, `owl:hasValue`, `owl:inverseOf`,
`owl:equivalentClass`, `owl:equivalentProperty`, `owl:SymmetricProperty`,
`owl:TransitiveProperty` and the three list constructors all fall to `other`,
whose extensions are empty; `svfPremises` mentions none of them and no rule
consumes their rows here. `live` is the model that exercises them. -/
def svfTable : List (Term × SvfD) :=
  [ (V.type, .ty), (V.subClassOf, .sco), (V.subPropertyOf, .spo),
    (V.domain, .dm), (V.range, .rg), (V.onProperty, .onp),
    (V.someValuesFrom, .svf), (V.sameAs, .sa),
    (V.Class, .cls), (V.Restriction, .restr),
    (tC, .cC), (tD, .cD), (tR, .cR), (tp, .prop), (tx, .ex_x), (ty, .ex_y) ]

/-- Terms denote their table entry, or `other`. -/
def svfι (t : Term) : SvfD := (List.lookup t svfTable).getD .other

/-- `IP`: the ten elements that carry pairs, so the structure is bridge-coherent
in the sense `live_is_bridge_coherent` states. -/
def svfIsIP : SvfD → Bool
  | .prop | .qq | .ty | .sco | .spo | .dm | .rg | .onp | .svf | .sa => true
  | _ => false

/-- The extensions. SIX rows are chosen, `IEXT(p)`, `IEXT(q)`, `ICEXT(D)`,
`owl:onProperty`, `owl:someValuesFrom` and the two class-typing rows; `ICEXT(C)`
is chosen EMPTY, which is the refutation; `ICEXT(R)` is forced by Table 5.6, the
`owl:sameAs` diagonal is forced by Table 5.9, and `sco`, `spo`, `dm` and `rg` are
the fixpoint Table 5.8's two directions force. -/
def svfIext : SvfD → SvfD → SvfD → Bool
  -- ICEXT(rdfs:Class) = IC = {C, D, R}. All three have to be classes: sc_fwd
  -- demands it of C and R, and svf_typ demands it of D.
  | .ty, w, .cls => w == .cC || w == .cD || w == .cR
  -- ICEXT(owl:Restriction) = {R}, a PROPER subset of IC. This is the typing the
  -- Herbrand witness lacks and the reason that witness is not a W3CModel.
  | .ty, w, .restr => w == .cR
  -- ICEXT(R) = {x}, FORCED by Table 5.6: R is ∃p.D, x reaches y by p, y is a D,
  -- and nothing else has a p-successor.
  | .ty, w, .cR => w == .ex_x
  -- ICEXT(D) = {y}, asserted by the graph.
  | .ty, w, .cD => w == .ex_y
  -- ICEXT of everything else, C INCLUDED, is empty.
  | .ty, _, _ => false
  -- IEXT(rdfs:subClassOf): the pairs of IC whose extensions nest. C is below
  -- everything because its extension is empty, which is what makes the premise
  -- true and the conclusion false at once.
  | .sco, .cC, v => v == .cC || v == .cD || v == .cR
  | .sco, .cD, v => v == .cD
  | .sco, .cR, v => v == .cR
  | .sco, _, _ => false
  -- IEXT(rdfs:subPropertyOf): the pairs of IP whose extensions nest. p and q both
  -- ways, because their extensions are equal, and the diagonal otherwise.
  | .spo, .prop, v | .spo, .qq, v => v == .prop || v == .qq
  | .spo, .ty, v => v == .ty
  | .spo, .sco, v => v == .sco
  | .spo, .spo, v => v == .spo
  | .spo, .dm, v => v == .dm
  | .spo, .rg, v => v == .rg
  | .spo, .onp, v => v == .onp
  | .spo, .svf, v => v == .svf
  | .spo, .sa, v => v == .sa
  | .spo, _, _ => false
  -- IEXT(rdfs:domain) and IEXT(rdfs:range), and they DIFFER: p's only subject is
  -- x, which is an R, and its only object is y, which is a D.
  | .dm, .prop, v | .dm, .qq, v => v == .cR
  | .dm, _, _ => false
  | .rg, .prop, v | .rg, .qq, v => v == .cD
  | .rg, _, _ => false
  -- The restriction, on one property, with one filler.
  | .onp, .cR, v => v == .prop
  | .onp, _, _ => false
  | .svf, .cR, v => v == .cD
  | .svf, _, _ => false
  -- RBS Table 5.9 row 1, which is an iff whose right-hand side is an equation.
  | .sa, u, v => u == v
  -- The asserted pair, and q's copy of it.
  | .prop, .ex_x, v | .qq, .ex_x, v => v == .ex_y
  -- Everything else is empty.
  | _, _, _ => false

/-- The interpretation. -/
def svfI : Interp where
  D := SvfD
  ι := svfι
  iext := fun p x y => svfIext p x y = true

/-- `IP` as a predicate. -/
def svfIP (x : SvfD) : Prop := svfIsIP x = true

instance : DecidablePred svfIP := fun x => decidable_of_iff (svfIsIP x = true) Iff.rfl

theorem svf_sc_fwd : ∀ a b : SvfD, svfIext .sco a b = true →
    svfIext .ty a .cls = true ∧ svfIext .ty b .cls = true ∧
    ∀ x, svfIext .ty x a = true → svfIext .ty x b = true := by decide

theorem svf_sc_bwd : ∀ a b : SvfD, svfIext .ty a .cls = true → svfIext .ty b .cls = true →
    (∀ x, svfIext .ty x a = true → svfIext .ty x b = true) → svfIext .sco a b = true := by decide

theorem svf_sp_fwd : ∀ a b : SvfD, svfIext .spo a b = true →
    svfIP a ∧ svfIP b ∧ ∀ x y, svfIext a x y = true → svfIext b x y = true := by decide

theorem svf_sp_bwd : ∀ a b : SvfD, svfIP a → svfIP b →
    (∀ x y, svfIext a x y = true → svfIext b x y = true) → svfIext .spo a b = true := by decide

theorem svf_dom_fwd : ∀ p c : SvfD, svfIext (svfι V.domain) p c = true →
    svfIP p ∧ svfIext .ty c .cls = true ∧
    ∀ x y, svfIext p x y = true → svfIext .ty x c = true := by decide

theorem svf_dom_bwd : ∀ p c : SvfD, svfIP p → svfIext .ty c .cls = true →
    (∀ x y, svfIext p x y = true → svfIext .ty x c = true) →
    svfIext (svfι V.domain) p c = true := by decide

theorem svf_rng_fwd : ∀ p c : SvfD, svfIext (svfι V.range) p c = true →
    svfIP p ∧ svfIext .ty c .cls = true ∧
    ∀ x y, svfIext p x y = true → svfIext .ty y c = true := by decide

theorem svf_rng_bwd : ∀ p c : SvfD, svfIP p → svfIext .ty c .cls = true →
    (∀ x y, svfIext p x y = true → svfIext .ty y c = true) →
    svfIext (svfι V.range) p c = true := by decide

theorem svf_eqc_fwd : ∀ a b : SvfD, svfIext (svfι V.equivalentClass) a b = true →
    svfIext .ty a .cls = true ∧ svfIext .ty b .cls = true ∧
    ∀ x, (svfIext .ty x a = true ↔ svfIext .ty x b = true) := by decide

theorem svf_eqp_fwd : ∀ a b : SvfD, svfIext (svfι V.equivalentProperty) a b = true →
    svfIP a ∧ svfIP b ∧ ∀ x y, (svfIext a x y = true ↔ svfIext b x y = true) := by decide

theorem svf_same_fwd : ∀ a b : SvfD, svfIext (svfι V.sameAs) a b = true → a = b := by decide

theorem svf_inv_fwd : ∀ p r : SvfD, svfIext (svfι V.inverseOf) p r = true →
    svfIP p ∧ svfIP r ∧ ∀ x y, (svfIext p x y = true ↔ svfIext r y x = true) := by decide

theorem svf_sym_fwd : ∀ p : SvfD, svfIext .ty p (svfι V.symmetricProperty) = true →
    ∀ x y, svfIext p x y = true → svfIext p y x = true := by decide

theorem svf_trp_fwd : ∀ p : SvfD, svfIext .ty p (svfι V.transitiveProperty) = true →
    ∀ x y z, svfIext p x y = true → svfIext p y z = true → svfIext p x z = true := by decide

theorem svf_svf_eq : ∀ z c p : SvfD, svfIext (svfι V.someValuesFrom) z c = true →
    svfIext (svfι V.onProperty) z p = true →
    ∀ x, (svfIext .ty x z = true ↔ ∃ y, svfIext p x y = true ∧ svfIext .ty y c = true) := by
  decide

theorem svf_avf_eq : ∀ z c p : SvfD, svfIext (svfι V.allValuesFrom) z c = true →
    svfIext (svfι V.onProperty) z p = true →
    ∀ x, (svfIext .ty x z = true ↔ ∀ y, svfIext p x y = true → svfIext .ty y c = true) := by
  decide

theorem svf_hv_eq : ∀ z a p : SvfD, svfIext (svfι V.hasValue) z a = true →
    svfIext (svfι V.onProperty) z p = true →
    ∀ x, (svfIext .ty x z = true ↔ svfIext p x a = true) := by decide

theorem svf_restr_IC : ∀ x : SvfD, svfIext .ty x (svfι V.Restriction) = true →
    svfIext .ty x .cls = true := by decide

theorem svf_svf_typ : ∀ z c : SvfD, svfIext (svfι V.someValuesFrom) z c = true →
    svfIext .ty z (svfι V.Restriction) = true ∧ svfIext .ty c .cls = true := by decide

theorem svf_avf_typ : ∀ z c : SvfD, svfIext (svfι V.allValuesFrom) z c = true →
    svfIext .ty z (svfι V.Restriction) = true ∧ svfIext .ty c .cls = true := by decide

theorem svf_onp_typ : ∀ z p : SvfD, svfIext (svfι V.onProperty) z p = true →
    svfIext .ty z (svfι V.Restriction) = true ∧ svfIP p := by decide

/-- Every asserted triple of `svfPremises` holds. -/
theorem svf_facts :
    ∀ t ∈ svfPremises, svfIext (svfι t.p) (svfι t.s) (svfι t.o) = true := by decide

theorem svf_meets_the_w3c_conditions : W3C svfI svfIP where
  sc_fwd := svf_sc_fwd
  sc_bwd := svf_sc_bwd
  sp_fwd := svf_sp_fwd
  sp_bwd := svf_sp_bwd
  dom_fwd := svf_dom_fwd
  dom_bwd := svf_dom_bwd
  rng_fwd := svf_rng_fwd
  rng_bwd := svf_rng_bwd
  eqc_fwd := svf_eqc_fwd
  eqp_fwd := svf_eqp_fwd
  same_fwd := svf_same_fwd
  inv_fwd := svf_inv_fwd
  sym_fwd := svf_sym_fwd
  trp_fwd := svf_trp_fwd
  svf_eq := svf_svf_eq
  avf_eq := svf_avf_eq
  hv_eq := svf_hv_eq
  restr_IC := svf_restr_IC
  svf_typ := svf_svf_typ
  avf_typ := svf_avf_typ
  onp_typ := svf_onp_typ

/-- **A `W3CModel` of `svfPremises`.** The three list fields are vacuous because
`svfPremises` carries no `owl:intersectionOf`, `owl:unionOf` or `owl:oneOf`
triple. -/
theorem svfI_is_a_w3c_model : W3CModel svfI svfIP svfPremises where
  conds := svf_meets_the_w3c_conditions
  facts := svf_facts
  int_eq := fun c l _ hc =>
    absurd hc (not_mem_pred svfPremises V.intersectionOf (by decide) c l)
  uni_eq := fun c l _ hc => absurd hc (not_mem_pred svfPremises V.unionOf (by decide) c l)
  oneOf_eq := fun c l _ hc => absurd hc (not_mem_pred svfPremises V.oneOf (by decide) c l)

theorem svfI_is_live_raw :
    (svfIext .ty .ex_x .cR = true ∧ svfIext .ty .ex_y .cD = true) ∧
    (∀ w, ¬ svfIext .ty w .cC = true) ∧
    (svfIext .prop .ex_x .ex_y = true) ∧
    (svfIext .ty .cR (svfι V.Restriction) = true ∧
      ¬ svfIext .ty .cD (svfι V.Restriction) = true) ∧
    (svfIext .spo .prop .qq = true ∧ ¬ svfIext .spo .prop .ty = true) ∧
    (∀ p x y : SvfD, svfIext p x y = true → svfIP p) := by decide

/-- **The countermodel is not degenerate**, and the last two conjuncts are the
ones that matter. `IEXT(rdfs:subPropertyOf)` is not the diagonal, which is what
lets the `owl:sameAs` row be the diagonal rather than empty; and every predicate
carrying a pair is in `IP`, so the structure is in the image of the bridge in
`W3C.lean`, which the Herbrand witness this replaces is not. -/
theorem svfI_is_live :
    (svfI.cext .cR .ex_x ∧ svfI.cext .cD .ex_y) ∧
    (∀ w, ¬ svfI.cext .cC w) ∧
    svfI.iext .prop .ex_x .ex_y ∧
    (svfI.cext (svfI.ι V.Restriction) .cR ∧ ¬ svfI.cext (svfI.ι V.Restriction) .cD) ∧
    (svfI.sp .prop .qq ∧ ¬ svfI.sp .prop .ty) ∧
    (∀ p x y : SvfD, svfI.iext p x y → svfIP p) :=
  svfI_is_live_raw

theorem svfI_refutes_the_conclusion : ¬ svfIext (svfι V.type) (svfι tx) (svfι tC) = true := by
  decide

/-- **The old `cls-svf1` derivation fails over the specification's model class
too.** `C ⊑ ∃p.D` with `x p y` and `y ∈ D` does not entail `x ∈ C`, and this is
now a statement about `W3CModel` rather than about `Semantics.lean`'s weaker
`Conditions`.

Read the module docstring before quoting it anywhere. `W3CModel`'s class is still
LARGER than the bridge image of the conforming interpretations, because `W3C`
omits every table row no rule consumes, so this is not a proof that the triple
fails to be OWL 2 RDF-Based entailed. It is the strongest statement this
development can make about the defect `tests/reason_rl_ext_soundness_test.rs`
pins, and it is strictly stronger than the Herbrand result it stands beside. -/
theorem the_old_svf_derivation_is_not_w3c_entailed :
    ¬ W3CEntails svfPremises ⟨tx, V.type, tC⟩ := fun h =>
  absurd (h svfI svfIP svfI_is_a_w3c_model) svfI_refutes_the_conclusion

/-! ## Axioms, pinned

A `sorry` or a `native_decide` here would restore the vacuity objection this
file exists to close, and `native_decide` would add `Lean.ofReduceBool` to every
footprint below. -/

/-- info: 'OOCert.live_is_a_w3c_model' depends on axioms: [propext] -/
#guard_msgs in
#print axioms live_is_a_w3c_model

/-- info: 'OOCert.live_is_live' depends on axioms: [propext] -/
#guard_msgs in
#print axioms live_is_live

/-- info: 'OOCert.live_is_bridge_coherent' depends on axioms: [propext] -/
#guard_msgs in
#print axioms live_is_bridge_coherent

/-- info: 'OOCert.live_dom_bwd_is_exercised' depends on axioms: [propext] -/
#guard_msgs in
#print axioms live_dom_bwd_is_exercised

/-- info: 'OOCert.live_rng_bwd_is_exercised' depends on axioms: [propext] -/
#guard_msgs in
#print axioms live_rng_bwd_is_exercised

/-- info: 'OOCert.live_exercises_every_arm' depends on axioms: [propext] -/
#guard_msgs in
#print axioms live_exercises_every_arm

/-- info: 'OOCert.live_fires_every_field' depends on axioms: [propext] -/
#guard_msgs in
#print axioms live_fires_every_field

/-- info: 'OOCert.live_sameAs_is_the_diagonal' depends on axioms: [propext] -/
#guard_msgs in
#print axioms live_sameAs_is_the_diagonal

/-- info: 'OOCert.sameAs_has_no_off_diagonal_instance' does not depend on any axioms -/
#guard_msgs in
#print axioms sameAs_has_no_off_diagonal_instance

/-- info: 'OOCert.scm_avf2_runs_one_way_under_the_specification' depends on axioms: [propext] -/
#guard_msgs in
#print axioms scm_avf2_runs_one_way_under_the_specification

/-- info: 'OOCert.not_everything_is_w3c_entailed' depends on axioms: [propext, Quot.sound] -/
#guard_msgs in
#print axioms not_everything_is_w3c_entailed

/-- info: 'OOCert.saturated_meets_the_w3c_conditions' does not depend on any axioms -/
#guard_msgs in
#print axioms saturated_meets_the_w3c_conditions

/--
info: 'OOCert.saturated_is_not_a_w3c_model_of_an_empty_enumeration' depends on axioms: [propext,
Quot.sound]
-/
#guard_msgs in
#print axioms saturated_is_not_a_w3c_model_of_an_empty_enumeration

/-- info: 'OOCert.oo_witness_is_a_w3c_model' depends on axioms: [propext] -/
#guard_msgs in
#print axioms oo_witness_is_a_w3c_model

/-- info: 'OOCert.an_unlisted_individual_is_not_w3c_entailed' depends on axioms: [propext] -/
#guard_msgs in
#print axioms an_unlisted_individual_is_not_w3c_entailed

/-- info: 'OOCert.int_witness_is_a_w3c_model' depends on axioms: [propext] -/
#guard_msgs in
#print axioms int_witness_is_a_w3c_model

/-- info: 'OOCert.membership_in_one_member_is_not_w3c_enough' depends on axioms: [propext] -/
#guard_msgs in
#print axioms membership_in_one_member_is_not_w3c_enough

/-- info: 'OOCert.svfI_is_a_w3c_model' depends on axioms: [propext] -/
#guard_msgs in
#print axioms svfI_is_a_w3c_model

/-- info: 'OOCert.svfI_is_live' depends on axioms: [propext] -/
#guard_msgs in
#print axioms svfI_is_live

/-- info: 'OOCert.the_old_svf_derivation_is_not_w3c_entailed' depends on axioms: [propext] -/
#guard_msgs in
#print axioms the_old_svf_derivation_is_not_w3c_entailed

/--
info: 'OOCert.sameAs_diagonal_needs_an_off_diagonal_subproperty' does not depend on any axioms
-/
#guard_msgs in
#print axioms sameAs_diagonal_needs_an_off_diagonal_subproperty

end OOCert
