import OOCert.Triple

/-!
# Semantics

The meaning against which a certificate is checked. It is the RDF-based
reading of a graph (RDF 1.1 Semantics, section 5; OWL 2 RDF-Based Semantics,
section 5), restricted to the vocabulary the engine's rules use.

## Shape of an interpretation

An interpretation is a domain `D`, a denotation `ι` for every term, and one
ternary relation `iext p x y`, read "(x, y) is in the extension of the
property p". Class membership is not a separate primitive: `x` is in class
`c` when `(x, c)` is in the extension of `rdf:type`. That is the RDF-based
semantics' `ICEXT`, and it is what makes a triple `s rdf:type o` mean the same
thing whether it was asserted or derived by a domain rule.

A triple `s p o` is satisfied when `iext (ι p) (ι s) (ι o)`. Nothing else.
Every piece of schema meaning is a condition on the interpretation
(`Conditions`), quantified over domain elements, the way the W3C tables state
them ("if (p, c) is in IEXT(I(rdfs:domain)) then ...").

## The conditions are the weakest the rules need

Each condition is the *if* direction of the corresponding W3C semantic
condition, or a consequence of it, never more. Where the W3C table states an
*iff* (subClassOf, for instance, is extensional in the RDF-based semantics)
only the half the rules use is assumed, plus transitivity where a rule
concludes a schema triple.

Weaker assumptions mean more interpretations, so a rule proved sound here is
sound under the OWL 2 RDF-Based Semantics. That transfer is now a theorem and
not a claim: `W3C.lean` states the specification conditions at full strength and
`W3CEntails.of_entails` proves that everything `Entails` gives is true in every
interpretation meeting them.

The claim this paragraph USED to make about the OWL 2 Direct Semantics
translated to triples is withdrawn. Nobody verified it, and it is false as
written for the twelve fields below that need the BACKWARD direction of
RDF-Based Semantics Table 5.8: RDFS gives `rdfs:domain`, `rdfs:range`,
`rdfs:subClassOf` and `rdfs:subPropertyOf` only the `if-then` direction, and the
Direct Semantics has no `rdfs:subClassOf` triples at all. Do not reinstate it
without a formalisation of that translation.

The list constructors are the exception to "conditions on the interpretation
alone". `owl:intersectionOf`, `owl:unionOf` and `owl:oneOf` point at an RDF
list, and `Model` reads that list off the ASSERTED GRAPH, through `Chain`.

**The reason once given for that is false and is replaced here.** The W3C tables
do NOT read the list off the graph. RDF-Based Semantics defines "`s` sequence of
`a₁ , … , aₙ ∈ S`" over the interpretation: "`s` = `I(rdf:nil)` for `n = 0`; and
for `n > 0` there exist `z₁ ∈ IR , … , zₙ ∈ IR`, such that `s = z₁`, `a₁ ∈ S`,
`( z₁ , a₁ ) ∈ IEXT(I(rdf:first))`, `( z₁ , z₂ ) ∈ IEXT(I(rdf:rest))`, … ", and
it adds that "there are no semantic constraints that enforce "well-formed"
sequence structures", so one head may denote more than one sequence. The
quantifier is over `IEXT(I(rdf:first))` and `IEXT(I(rdf:rest))`, never over `G`.

The real justification is that the error runs in the safe direction. Any model
of `G` satisfies `G`'s own `rdf:first` and `rdf:rest` triples, so every `Chain G`
list is a semantic sequence, while a semantic sequence assembled from pairs the
graph never asserts is not a `Chain G` list. `Chain` therefore fires on strictly
fewer lists, the conditions stated over it are WEAKER than Tables 5.4 and 5.5,
the model class is larger, and entailment still transfers outward. What does not
follow is the converse, and `W3CModel` records that gap at its list fields
rather than closing it: no rule in this checker consumes `rdf:first` or
`rdf:rest` semantically, so nothing here needs a sequence relation.

## Conditions that let a rule CONCLUDE a schema triple

`sc_sub` says what a `rdfs:subClassOf` triple means for class extensions. It
does not say when such a triple holds, because the interpretation of
`rdfs:subClassOf` is a primitive relation here and only the half the instance
rules use is assumed. A rule whose CONCLUSION is a schema triple therefore
needs its own condition, and `sc_trans` is the first of them: it is what lets
`rdfs11` conclude `a rdfs:subClassOf c`.

**There are FOURTEEN arms of that kind, not nine, and this paragraph used to say
nine.** The undercount was real: it named the eight restriction-ordering and
domain-and-range fields (`scm-svf1`, `scm-svf2`, `scm-avf1`, `scm-avf2`,
`scm-dom1`, `scm-dom2`, `scm-rng1`, `scm-rng2`) plus `sc_trans`, and left out
`sp_trans` (`rdfs5`) and the two conclusions each that `eqc` (`scm-eqc1`) and
`eqp` (`scm-eqp1`) license. Those four conclude a schema triple by exactly the
same criterion.

`eqc` and `eqp` belong on the list for a reason worth keeping in view: RBS
Table 5.9 gives `owl:equivalentClass` an extension EQUALITY and never a
`rdfs:subClassOf` triple, so a field handing back two such triples is a
restatement of the rule and not a reading of a table. `sc_trans` is NOT of that
kind on its own, because RDF 1.1 Semantics section 9 states transitivity of
`rdfs:subClassOf` as a condition in its own right; it is on the list because
Table 5.8 makes it redundant.

**All fourteen are now DERIVED, in `W3C.lean`, from quoted specification
cells**, and `W3CModel.toModel` supplies each of them as a proof term rather
than a projection. The English derivations that follow each field below are kept
because they say what the machine-checked proof does, but they are no longer the
only thing standing behind these fields. Every field below still carries the
verbatim rule from OWL 2 Profiles Table 9.

## Why the direction of that argument is the only one that is safe

A field added here SHRINKS the model class, which makes `Entails` easier to
satisfy and therefore weakens `certificate_sound` without any error appearing
anywhere. The protection is that every one of these fields is implied by the
RDF-Based Semantics conditions, so every interpretation that document admits is
still a model here. Soundness proved against this weaker set therefore still
holds under the W3C semantics. Assuming the `iff`s themselves instead would be
strictly stronger, would buy nothing any rule needs, and would put the
non-vacuity witnesses at risk, which is the observable symptom of a condition
that has gone too far.

**THE ARGUMENT SECURES POSITIVE RESULTS ONLY, AND THIS IS THE SENTENCE THE FILE
WAS MISSING.** `Conditions` admits MORE interpretations than `W3C.lean`'s `W3C`
does. A triple true in every one of them is true in every `W3CModel`, so
`Entails` transfers outward and `W3CEntails.of_entails` proves it. Reading that
one step further out, to the specification's own conforming interpretations,
costs a prose bridge and five axiomatic-triple facts, and `W3C.lean` states both
rather than folding them into the word "conforming". Nothing transfers the other
way. A model of `Conditions` need not be a conforming
interpretation, so `¬ Entails G t` does NOT give `¬ W3CEntails G t`, and neither
does `¬ Unsat`, and neither does exhibiting a `Model I G`.

Consequently a `¬ Entails`, a `¬ Unsat` or a `Model I G` here is a statement
about THIS model class and not about the specification's, unless something
restates it over `W3CModel`. The state of each is recorded at the theorem itself
as well as here, because a summary that lives in one file is a summary that goes
stale, and this one did.

Nine statements in this repository are a `¬ Entails` or a `¬ Unsat`. ALL NINE are
now restated over `W3CModel`. Four of them transferred with their existing
witness graph unchanged; four needed a finite structure built for them; one is a
lemma and follows by unfolding.

| result | over `W3CModel` | where |
|---|---|---|
| `not_everything_is_entailed` | yes | `not_everything_is_w3c_entailed`, same witness graph |
| `the_natural_avf2_direction_is_not_entailed` | yes | `the_natural_avf2_direction_is_not_w3c_entailed`, on `live` |
| `an_unlisted_individual_is_not_entailed` | yes | `an_unlisted_individual_is_not_w3c_entailed`, same witness graph |
| `membership_in_one_member_does_not_give_the_intersection` | yes | `membership_in_one_member_is_not_w3c_enough`, same witness graph |
| `mix_not_absolutely_entailed` | yes | `mix_not_absolutely_w3c_entailed`, in `Mixed.lean`, same witness graph |
| `the_old_svf_derivation_is_not_entailed` | yes | `the_old_svf_derivation_is_not_w3c_entailed`, on `svfI` |
| `feed_is_not_refuted` | yes | `feed_is_not_w3c_refuted`, on `refI false`, and it is `¬ W3CUnsat`, which is STRONGER than `¬ Unsat` rather than weaker |
| `and_the_old_verdict_does_not_notice` | yes | `the_old_verdict_does_not_notice_over_w3c`, on `refI true` |
| `not_unsat_of_joint_model` | yes | `not_w3cUnsat_of_joint_w3c_model`, by unfolding |

Two further negatives, `Mixed.lean`'s `mix_relative_is_not_everything` and
`HornWitness.lean`'s `demo_not_everything_entailed`, are `¬ EntailsR`: statements
about the models of a graph that ALSO satisfy a supplied rule table. There is no
`W3CEntailsR` and this file does not invent one, so nothing is claimed about them
beyond `Conditions`.

**An earlier version of this paragraph got the reason wrong and the conclusion
with it**, and the correction is recorded rather than edited away because the
wrong version is what the project told people. It read: "a Herbrand witness that
carries no `rdf:type` triple has `IC` empty, so Table 5.8's backward direction
forces `rdfs:subClassOf` and `rdfs:subPropertyOf` triples the witness does not
contain", and it concluded that "every conforming countermodel must therefore be
a hand-built finite structure".

Both halves are false and the first is inverted. An empty `IC` makes `sc_bwd`
VACUOUS, not demanding: its antecedents are `I.IC a → I.IC b → …` and they have
no instances. `sp_bwd`, `dom_bwd` and `rng_bwd` are guarded by `IP`, which is a
free PARAMETER of `W3C` and not a field of `Interp`, so the refuter chooses it;
`IP := fun _ => False` makes all three vacuous. Four of the results above
transfer on exactly that, with the existing Herbrand witnesses unchanged and
nothing hand-built. The mistake was to treat `live_is_bridge_coherent`, which is
an extra property the `W3CWitness.lean` model happens to have, as though it were
a requirement of `W3CModel`.

One obstruction in that paragraph survives and it is worth keeping, because it
says something different from what it was used for. `RefuteWitness.lean`'s `Der`
cannot carry Table 5.8's backward halves: a constructor whose premise contains
`∀ x, Der G ⟨x, type, a⟩ → Der G ⟨x, type, b⟩` puts `Der` to the left of an arrow
and Lean rejects the strictly negative occurrence, and that is mathematical
rather than a Lean artefact, because `sc_bwd` is ANTITONE in `ICEXT(a)` and there
is no least fixed point by monotonicity. What follows from it is that there is no
general CLOSURE OPERATOR taking any graph to a `W3CModel`. What does not
follow is that no particular Herbrand interpretation is a `W3CModel`, and
that is the step the old paragraph took.

`scm-dom1` is the one that had to be checked rather than assumed. Under RDF
Semantics `rdfs:domain` carries only the *if* direction and the rule does NOT
follow. The OWL 2 RDF-Based Semantics is stronger: in Table 5.8 the `iff` cell
spans four rows, `rdfs:domain` and `rdfs:range` among them, and it is that
`iff` the rule needs.

`scm-avf2` is the one to read twice. Its conclusion runs the other way,
`?c2 rdfs:subClassOf ?c1` where the other three conclude `?c1 rdfs:subClassOf
?c2`, because a universal restriction is antitone in its property: if `p1` is a
subproperty of `p2` then everything reachable by `p1` is reachable by `p2`, so
`all p2 y` is contained in `all p1 y` and not the other way round. `avf_sp`
below is stated in the direction the W3C table gives, and `Witness.lean` proves
the natural-looking direction is not a consequence.

## What is not here

Literal values are terms like any other: two spellings of one value are two
terms. No rule compares literals by value, so nothing is lost, but a
consumer must not read `Entails` as datatype-aware. `owl:sameAs` is read as
identity of denotation, which is the standard condition; only its symmetry is
used.
-/
namespace OOCert

namespace V
/-- `owl:oneOf`, the enumeration constructor `cls-oo` reads. The rest of the
vocabulary is in `Triple.lean` and this belongs beside it; it is here because
`cls-oo` is the only rule that mentions it and the condition for that rule is
three lines below. -/
def oneOf : Term := "<http://www.w3.org/2002/07/owl#oneOf>"
end V

structure Interp where
  D : Type
  ι : Term → D
  iext : D → D → D → Prop

namespace Interp
variable (I : Interp)

/-- `x` is in class `c`: `(x, c) ∈ IEXT(rdf:type)`. -/
def cext (c x : I.D) : Prop := I.iext (I.ι V.type) x c
/-- `(a, b) ∈ IEXT(rdfs:subClassOf)`. -/
def sc (a b : I.D) : Prop := I.iext (I.ι V.subClassOf) a b
/-- `(a, b) ∈ IEXT(rdfs:subPropertyOf)`. -/
def sp (a b : I.D) : Prop := I.iext (I.ι V.subPropertyOf) a b
/-- A triple holds when its predicate's extension contains the pair. -/
def sat (t : Triple) : Prop := I.iext (I.ι t.p) (I.ι t.s) (I.ι t.o)

end Interp

/-- The semantic conditions, one group per rule family. -/
structure Conditions (I : Interp) : Prop where
  /-- rdfs9. RDF-based semantics: subClassOf implies extension inclusion. -/
  sc_sub : ∀ a b, I.sc a b → ∀ x, I.cext a x → I.cext b x
  /-- rdfs11. Follows from the extensional reading of subClassOf. -/
  sc_trans : ∀ a b c, I.sc a b → I.sc b c → I.sc a c
  /-- rdfs7. -/
  sp_sub : ∀ a b, I.sp a b → ∀ x y, I.iext a x y → I.iext b x y
  /-- rdfs5. -/
  sp_trans : ∀ a b c, I.sp a b → I.sp b c → I.sp a c
  /-- rdfs2. -/
  dom : ∀ p c, I.iext (I.ι V.domain) p c → ∀ x y, I.iext p x y → I.cext c x
  /-- rdfs3. -/
  rng : ∀ p c, I.iext (I.ι V.range) p c → ∀ x y, I.iext p x y → I.cext c y
  /-- prp-trp. Members of owl:TransitiveProperty have transitive extensions. -/
  trp : ∀ p, I.cext (I.ι V.transitiveProperty) p →
    ∀ x y z, I.iext p x y → I.iext p y z → I.iext p x z
  /-- prp-symp. -/
  symp : ∀ p, I.cext (I.ι V.symmetricProperty) p → ∀ x y, I.iext p x y → I.iext p y x
  /-- prp-inv1, prp-inv2. -/
  inv : ∀ p q, I.iext (I.ι V.inverseOf) p q → ∀ x y, (I.iext p x y ↔ I.iext q y x)
  /-- eq-sym. owl:sameAs is identity of denotation. -/
  same : ∀ a b, I.iext (I.ι V.sameAs) a b → a = b
  /-- scm-eqc1, scm-eqc2. -/
  eqc : ∀ a b, I.iext (I.ι V.equivalentClass) a b → I.sc a b ∧ I.sc b a
  /-- scm-eqp1, scm-eqp2. -/
  eqp : ∀ a b, I.iext (I.ι V.equivalentProperty) a b → I.sp a b ∧ I.sp b a
  /-- cls-svf1. A witness puts `x` into the restriction class. Only this
  direction is assumed; the converse is not needed by any rule. -/
  svf : ∀ r p c, I.iext (I.ι V.onProperty) r p → I.iext (I.ι V.someValuesFrom) r c →
    ∀ x y, I.iext p x y → I.cext c y → I.cext r x
  /-- cls-avf. Everything an `x` in the restriction reaches by `p` is in the
  filler. The mirror of `svf`, and like it only the direction the rule uses. -/
  avf : ∀ r p c, I.iext (I.ι V.onProperty) r p → I.iext (I.ι V.allValuesFrom) r c →
    ∀ x y, I.cext r x → I.iext p x y → I.cext c y
  /-- cls-hv1 needs the forward direction, cls-hv2 the backward one. -/
  hv : ∀ r p v, I.iext (I.ι V.onProperty) r p → I.iext (I.ι V.hasValue) r v →
    ∀ x, (I.cext r x ↔ I.iext p x v)
  /-- **scm-svf1.** Two existential restrictions on the SAME property, ordered
  by their fillers.

  OWL 2 Profiles, Table 9, verbatim: if `T(?c1, owl:someValuesFrom, ?y1)`,
  `T(?c1, owl:onProperty, ?p)`, `T(?c2, owl:someValuesFrom, ?y2)`,
  `T(?c2, owl:onProperty, ?p)`, `T(?y1, rdfs:subClassOf, ?y2)` then
  `T(?c1, rdfs:subClassOf, ?c2)`.

  Derivation, so that this is a consequence of the W3C semantics and not an
  extra assumption. RDF-Based Semantics Table 5.6: "(z, c) ∈ IEXT(I(
  owl:someValuesFrom)), (z, p) ∈ IEXT(I(owl:onProperty))" implies
  "ICEXT(z) = { x | ∃ y : (x, y) ∈ IEXT(p) and y ∈ ICEXT(c) }". Applying it to
  `c1` and to `c2` and Table 5.8's iff "(c1, c2) ∈ IEXT(I(rdfs:subClassOf)) iff
  c1, c2 ∈ IC, ICEXT(c1) ⊆ ICEXT(c2)" to `y1, y2` gives
  ICEXT(c1) ⊆ ICEXT(c2), and the right-to-left half of that same iff turns it
  back into the triple. -/
  svf_sc : ∀ c1 c2 p y1 y2,
    I.iext (I.ι V.someValuesFrom) c1 y1 → I.iext (I.ι V.onProperty) c1 p →
    I.iext (I.ι V.someValuesFrom) c2 y2 → I.iext (I.ι V.onProperty) c2 p →
    I.sc y1 y2 → I.sc c1 c2
  /-- **scm-svf2.** Two existential restrictions with the SAME filler, ordered
  by their properties.

  OWL 2 Profiles, Table 9, verbatim: if `T(?c1, owl:someValuesFrom, ?y)`,
  `T(?c1, owl:onProperty, ?p1)`, `T(?c2, owl:someValuesFrom, ?y)`,
  `T(?c2, owl:onProperty, ?p2)`, `T(?p1, rdfs:subPropertyOf, ?p2)` then
  `T(?c1, rdfs:subClassOf, ?c2)`.

  Derivation: Table 5.6 as above for both classes, and Table 5.8's iff
  "(p1, p2) ∈ IEXT(I(rdfs:subPropertyOf)) iff p1, p2 ∈ IP, IEXT(p1) ⊆
  IEXT(p2)". A witness reached by `p1` is reached by `p2`, so
  ICEXT(c1) ⊆ ICEXT(c2). -/
  svf_sp : ∀ c1 c2 p1 p2 y,
    I.iext (I.ι V.someValuesFrom) c1 y → I.iext (I.ι V.onProperty) c1 p1 →
    I.iext (I.ι V.someValuesFrom) c2 y → I.iext (I.ι V.onProperty) c2 p2 →
    I.sp p1 p2 → I.sc c1 c2
  /-- **scm-avf1.** The mirror of `svf_sc` for universal restrictions, and it
  runs the SAME way round: a wider filler makes a wider class.

  OWL 2 Profiles, Table 9, verbatim: if `T(?c1, owl:allValuesFrom, ?y1)`,
  `T(?c1, owl:onProperty, ?p)`, `T(?c2, owl:allValuesFrom, ?y2)`,
  `T(?c2, owl:onProperty, ?p)`, `T(?y1, rdfs:subClassOf, ?y2)` then
  `T(?c1, rdfs:subClassOf, ?c2)`.

  Derivation: Table 5.6 gives
  "ICEXT(z) = { x | ∀ y : (x, y) ∈ IEXT(p) implies y ∈ ICEXT(c) }" for a
  universal restriction. Widening `ICEXT(c)` weakens the consequent of that
  implication, so it widens `ICEXT(z)`. A universal restriction is MONOTONE in
  its filler and antitone only in its property, which is why this rule and
  `avf_sp` run in opposite directions. -/
  avf_sc : ∀ c1 c2 p y1 y2,
    I.iext (I.ι V.allValuesFrom) c1 y1 → I.iext (I.ι V.onProperty) c1 p →
    I.iext (I.ι V.allValuesFrom) c2 y2 → I.iext (I.ι V.onProperty) c2 p →
    I.sc y1 y2 → I.sc c1 c2
  /-- **scm-avf2, and the conclusion is REVERSED.**

  OWL 2 Profiles, Table 9, verbatim, and this is the quote rather than the
  pattern of its three siblings: if `T(?c1, owl:allValuesFrom, ?y)`,
  `T(?c1, owl:onProperty, ?p1)`, `T(?c2, owl:allValuesFrom, ?y)`,
  `T(?c2, owl:onProperty, ?p2)`, `T(?p1, rdfs:subPropertyOf, ?p2)` then
  **`T(?c2, rdfs:subClassOf, ?c1)`**. The other three conclude
  `T(?c1, rdfs:subClassOf, ?c2)`.

  Derivation, which reaches the same direction independently of the table.
  Table 5.6 gives ICEXT(c1) = { x | ∀ v : (x, v) ∈ IEXT(p1) implies
  v ∈ ICEXT(y) } and the same for `c2` with `p2`; `v` is Table 5.6's bound
  variable renamed, because the table spells it `y` and the rule already uses
  `?y` for the filler. Table 5.8 gives
  IEXT(p1) ⊆ IEXT(p2). Take x ∈ ICEXT(c2) and any v with (x, v) ∈ IEXT(p1);
  then (x, v) ∈ IEXT(p2), so v ∈ ICEXT(y), so x ∈ ICEXT(c1). That is
  ICEXT(c2) ⊆ ICEXT(c1), the reverse inclusion. A universal restriction is
  antitone in its property: the bigger property has more values to constrain,
  so its restriction class is the smaller one.

  Reading the direction off the pattern of the other three is a silent
  unsoundness, and `Witness.lean`'s
  `the_natural_avf2_direction_is_not_entailed` exhibits a model that refutes
  it. -/
  avf_sp : ∀ c1 c2 p1 p2 y,
    I.iext (I.ι V.allValuesFrom) c1 y → I.iext (I.ι V.onProperty) c1 p1 →
    I.iext (I.ι V.allValuesFrom) c2 y → I.iext (I.ι V.onProperty) c2 p2 →
    I.sp p1 p2 → I.sc c2 c1
  /-- **scm-dom1.** A domain may be weakened to a superclass.

  OWL 2 Profiles, Table 9, verbatim: if `T(?p, rdfs:domain, ?c1)`,
  `T(?c1, rdfs:subClassOf, ?c2)` then `T(?p, rdfs:domain, ?c2)`.

  Derivation, and this one is the reason to read Table 5.8 rather than RDF
  Semantics. RDFS alone gives domain only the *if* direction and this rule
  would NOT follow from it. The OWL 2 RDF-Based Semantics makes it an iff: the
  `iff` cell of Table 5.8 carries `rowspan="4"`, so it governs the
  `rdfs:domain` and `rdfs:range` rows as well as the two above them.
  Verbatim: "(p, c) ∈ IEXT(I(rdfs:domain)) iff p ∈ IP, c ∈ IC, ∀ x, y :
  (x, y) ∈ IEXT(p) implies x ∈ ICEXT(c)". Left to right on `c1`, then
  ICEXT(c1) ⊆ ICEXT(c2) from Table 5.8's subClassOf row, then right to left on
  `c2`. -/
  dom_sc : ∀ p c1 c2, I.iext (I.ι V.domain) p c1 → I.sc c1 c2 →
    I.iext (I.ι V.domain) p c2
  /-- **scm-dom2.** A domain is inherited by every subproperty.

  OWL 2 Profiles, Table 9, verbatim: if `T(?p2, rdfs:domain, ?c)`,
  `T(?p1, rdfs:subPropertyOf, ?p2)` then `T(?p1, rdfs:domain, ?c)`.

  Derivation: the same iff of Table 5.8 left to right on `p2`, then
  IEXT(p1) ⊆ IEXT(p2), so every pair of `p1` is a pair of `p2` and its subject
  is already in ICEXT(c), then right to left on `p1`. -/
  dom_sp : ∀ p1 p2 c, I.iext (I.ι V.domain) p2 c → I.sp p1 p2 →
    I.iext (I.ι V.domain) p1 c
  /-- **scm-rng1.** The mirror of `dom_sc`.

  OWL 2 Profiles, Table 9, verbatim: if `T(?p, rdfs:range, ?c1)`,
  `T(?c1, rdfs:subClassOf, ?c2)` then `T(?p, rdfs:range, ?c2)`. Table 5.8's
  range row differs from its domain row only in reading `y ∈ ICEXT(c)` where
  the other reads `x`. -/
  rng_sc : ∀ p c1 c2, I.iext (I.ι V.range) p c1 → I.sc c1 c2 →
    I.iext (I.ι V.range) p c2
  /-- **scm-rng2.** The mirror of `dom_sp`.

  OWL 2 Profiles, Table 9, verbatim: if `T(?p2, rdfs:range, ?c)`,
  `T(?p1, rdfs:subPropertyOf, ?p2)` then `T(?p1, rdfs:range, ?c)`. -/
  rng_sp : ∀ p1 p2 c, I.iext (I.ι V.range) p2 c → I.sp p1 p2 →
    I.iext (I.ι V.range) p1 c

/-- `Chain G l ms`: the RDF list whose head node is `l` in graph `G` has the
members `ms`, read through `rdf:first` / `rdf:rest` down to `rdf:nil`. A node
with two `rdf:first` values has two chains; the class condition then applies to
both, which can only strengthen the constraint on a model. -/
inductive Chain (G : List Triple) : Term → List Term → Prop
  | nil : Chain G V.nil []
  | cons {l m l' ms} :
      (⟨l, V.first, m⟩ : Triple) ∈ G → (⟨l, V.rest, l'⟩ : Triple) ∈ G →
      Chain G l' ms → Chain G l (m :: ms)

/-- `I` is a model of `G`: the conditions hold, every triple of `G` holds, and
the two list constructors mean intersection and union of their members. -/
structure Model (I : Interp) (G : List Triple) : Prop where
  conds : Conditions I
  facts : ∀ t ∈ G, I.sat t
  /-- cls-int1. -/
  int : ∀ c l ms, (⟨c, V.intersectionOf, l⟩ : Triple) ∈ G → Chain G l ms →
    ∀ x, (∀ m ∈ ms, I.cext (I.ι m) x) → I.cext (I.ι c) x
  /-- **cls-int2**, the other half of what an intersection means. `int` says
  membership in every member gives membership in the class; this says
  membership in the class gives membership in each member. Only with both is
  the class extension the intersection, and only `int` was assumed before.

  RDF-Based Semantics Table 5.4, verbatim: "if s sequence of c1, …, cn ∈ IR
  then (z, s) ∈ IEXT(I(owl:intersectionOf)) iff z, c1, …, cn ∈ IC,
  ICEXT(z) = ICEXT(c1) ∩ … ∩ ICEXT(cn)". That equality has two halves. `int`
  is ⊇ and this is ⊆, so each is strictly weaker than the condition and the
  two together are still weaker, because neither asks for
  `z, c1, …, cn ∈ IC`. -/
  int2 : ∀ c l ms, (⟨c, V.intersectionOf, l⟩ : Triple) ∈ G → Chain G l ms →
    ∀ x, I.cext (I.ι c) x → ∀ m ∈ ms, I.cext (I.ι m) x
  /-- cls-uni. -/
  uni : ∀ c l ms, (⟨c, V.unionOf, l⟩ : Triple) ∈ G → Chain G l ms →
    ∀ x m, m ∈ ms → I.cext (I.ι m) x → I.cext (I.ι c) x
  /-- **cls-oo.** Every listed member of an `owl:oneOf` enumeration is an
  instance of the enumerated class.

  RDF-Based Semantics Table 5.5, verbatim: "if s sequence of a1, …, an ∈ IR
  then (z, s) ∈ IEXT(I(owl:oneOf)) iff z ∈ IC, ICEXT(z) = { a1, …, an }".
  This field is the ⊇ half of that equality and nothing else: the class holds
  AT LEAST the listed members. The ⊆ half, that it holds no others, is what a
  rule concluding `owl:sameAs` or a clash would need, and no rule here
  concludes either, so assuming it would shrink the model class for nothing.

  **The claim that `Witness.lean`'s `an_unlisted_individual_is_not_entailed` is
  "the check that it really was left out" is FALSE and is withdrawn here.** That
  witness is `ooWitness`, which is `[a rdf:type E, b rdf:type E] ++ ooPremises`,
  so its `ICEXT(E)` is exactly `{a, b}`, exactly the listed members, and it
  already satisfies Table 5.5's full equality. Adding the `⊆` half changes
  nothing about it: the witness still models the premises and still does not
  type `z`. A witness that passes unchanged whether or not the condition is
  present cannot detect the condition's absence.

  So the omission of the `⊆` half is UNDETECTED. Detecting it needs a model in
  which some `z` outside the list IS in `ICEXT(E)` while the premises hold,
  which the Herbrand construction cannot give, because a Herbrand interpretation
  contains only the triples written down. Nothing else in the repository tests
  it either. -/
  oneOf : ∀ c l ms, (⟨c, V.oneOf, l⟩ : Triple) ∈ G → Chain G l ms →
    ∀ m ∈ ms, I.cext (I.ι c) (I.ι m)

/-- Entailment relative to an arbitrary class of interpretations. The soundness
proof never needs more than "every interpretation in this class models `G`", so
stating it this way lets one proof serve the built-in rules and any further
assumption a certificate chooses to carry, `OOCert.EntailsR` for user-written
Horn rules among them. -/
def EntailsIn (P : Interp → Prop) (t : Triple) : Prop := ∀ I : Interp, P I → I.sat t

/-- `G ⊨ t`: every model of `G` satisfies `t`. -/
def Entails (G : List Triple) (t : Triple) : Prop := EntailsIn (fun I => Model I G) t

theorem Entails.of_mem {G : List Triple} {t : Triple} (h : t ∈ G) : Entails G t :=
  fun _ M => M.facts t h

end OOCert
