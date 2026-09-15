import OOCert.Conforming

/-!
# A conforming interpretation, and something it does not entail

`Conforming.lean` proves a bridge over a structure. A structure nothing
satisfies proves nothing, and a structure satisfied only by something degenerate
proves very little, so this file exhibits one and then says exactly how much of
it is alive.

## The model

Two elements. `hub` carries the RDFS vocabulary; `rim` absorbs everything else.

* `IEXT(hub)` is every pair EXCEPT `( hub , rim )`. `IEXT(rim)` is empty.
* `I(u) = hub` for `rdf:type`, `rdf:Property`, `rdfs:Class`, `rdfs:Resource`,
  `rdfs:subClassOf`, `rdfs:subPropertyOf`, `rdfs:domain`, `rdfs:range` and
  `owl:Thing`; `I(u) = rim` for every other term, every blank node and every
  literal.
* `IP` is the whole carrier.

Everything else follows. `ICEXT(hub)` is the whole carrier and `ICEXT(rim)` is
`{ rim }`, so `IC` is the whole carrier and `ICEXT(I(rdfs:Class)) = IC`,
`ICEXT(I(rdf:Property)) = IP`, `ICEXT(I(rdfs:Resource)) = IR` and
`ICEXT(I(owl:Thing)) = IR` all hold. The first of those four rows of RBS Table
5.2 cannot fail in any `Interpretation`, because `IC` is DEFINED as
`ICEXT(I(rdfs:Class))`; the other three are checked, and they are three rows that
`W3CWitness.lean`'s `live` records itself as violating.

## What is alive here and what is not

**Table 5.8 is alive in both directions and it is alive because it is
CONSTRAINING.** `IEXT(I(rdfs:subClassOf))` holds three of the four possible
pairs. The missing one, `( hub , rim )`, is missing because `ICEXT(hub)` is not
contained in `ICEXT(rim)`, and the forward direction forbids it; the present
one, `( rim , hub )`, is present because `ICEXT(rim) ⊆ ICEXT(hub)` and the
backward direction forces it. `two_table58_separates` and
`two_table58_forces` are those two facts, checked. A model in which every
relation is total satisfies Table 5.8 for no reason at all and distinguishes
nothing; this one does not.

The same three-of-four shape holds for `rdfs:subPropertyOf`, `rdfs:domain` and
`rdfs:range`, and the reason differs in each case, which is what
`two_table58_row_reasons` records.

**The OWL vocabulary is dead here and the report says so rather than implying
otherwise.** Every OWL term denotes `rim`, whose extension is empty, so Table
5.6's three rows, Table 5.9's three, Table 5.12, and the `owl:someValuesFrom`,
`owl:allValuesFrom` and `owl:onProperty` typing rows of Table 5.3 hold for want
of anything to test. `two_dead_fields` is that list, as a theorem, so it cannot
drift from the file. Table 5.13's two rows and Table 5.2's `owl:Restriction` row
DO fire, at `rim`.

Bringing the OWL vocabulary to life inside a model that also satisfies the
axiomatic triples is the obvious next witness and it is not this one. It is not
a small change: the axiomatic triples force `I(rdf:type)`, `I(rdfs:domain)` and
`I(rdfs:range)` into a mutual fixpoint with `IEXT`, and `W3CWitness.lean`'s
`live` needs twenty-six elements to keep that fixpoint apart from the OWL
configuration while carrying no axiomatic triple at all.

## The axiomatic triples, all of them

This model satisfies EVERY RDF axiom of RDF11 section 8 and EVERY RDFS axiomatic
triple of RDF11 section 9, not just the five that `Conforming` carries, and
`two_satisfies_every_listed_axiomatic_triple` checks the whole table by
`decide`. The reason is structural rather than lucky: every axiomatic triple's
predicate is one of `rdf:type`, `rdfs:domain`, `rdfs:range`, `rdfs:subClassOf`
and `rdfs:subPropertyOf`, all of which denote `hub`, and the only pair `IEXT(hub)`
omits is `( hub , rim )`, which would need an axiomatic triple whose subject is
one of the nine `hub` terms and whose object is not. There is none.

## What this does NOT establish

It does not establish that a fully conforming OWL 2 RDF-Based interpretation
exists in Lean, because `Conforming` is a subset of the Recommendation's
conditions. The rows this model is known to violate are listed at
`two_violates_these_known_rows`. The satisfiability of the FULL Recommendation is
a different and much larger question and nothing here touches it.

It also does not turn any non-entailment below into a claim about OWL 2
RDF-Based entailment, for the same reason: a `Conforming` structure need not be
a conforming interpretation.
-/
namespace OOCert

/-! ## The carrier -/

/-- Two elements. `hub` is where the RDFS vocabulary lands and `rim` is
everything else. -/
inductive Two
  | hub | rim
  deriving DecidableEq, Repr

namespace Two

/-- Both of them. -/
def all : List Two := [.hub, .rim]

theorem mem_all (w : Two) : w ∈ all := by cases w <;> decide

/-- Universal quantification over the carrier is decidable, so every condition
below is settled by `decide` rather than by a tactic script that might be
proving something adjacent to what it says. Same device as
`W3CWitness.lean`'s `LiveD.decForall`, over two elements instead of
twenty-six. -/
instance decForall (p : Two → Prop) [DecidablePred p] : Decidable (∀ w, p w) :=
  decidable_of_iff (∀ w ∈ all, p w) ⟨fun h w => h w (mem_all w), fun h w _ => h w⟩

/-- And so is existential quantification. -/
instance decExists (p : Two → Prop) [DecidablePred p] : Decidable (∃ w, p w) :=
  decidable_of_iff (∃ w ∈ all, p w)
    ⟨fun ⟨w, _, h⟩ => ⟨w, h⟩, fun ⟨w, h⟩ => ⟨w, mem_all w, h⟩⟩

end Two

/-- `IEXT(hub)`, as a relation on the carrier: every pair but `( hub , rim )`.

That one omission is the whole content of the model. Put it back and Table 5.8's
forward direction fails at `rdfs:subClassOf`, because `ICEXT(hub)` is the whole
carrier and `ICEXT(rim)` is not. Take another pair out and the backward
direction fails. -/
def twoR : Two → Two → Bool
  | .hub, .rim => false
  | _, _ => true

/-- `IEXT`. Only `hub` has a non-empty extension. -/
def twoIEXT : Two → Two → Two → Bool
  | .hub, x, y => twoR x y
  | .rim, _, _ => false

/-- `IP`, the whole carrier. RDF11 section 9 is explicit that this is allowed:
"RDFS does not partition the universe into disjoint categories of classes,
properties and individuals … it also permits classes which contain themselves
and properties which apply to themselves." -/
def twoIP : Two → Bool := fun _ => true

/-- `owl:Thing`. Local to this file: no condition in `Conforming` mentions it,
and it is here only so that `two_satisfies_table_52_identity_rows` can check RBS
Table 5.2's `owl:Thing | ∈ IC | = IR` row. -/
def owlThing : Term := "<http://www.w3.org/2002/07/owl#Thing>"

/-- The nine terms that denote `hub`. Eight of them are forced: they are the
subjects and objects of the five axiomatic triples `Conforming` carries, read
through `IEXT(hub)`. `owl:Thing` is the ninth and is free. -/
def twoHubTerms : List Term :=
  [V.type, V.Property, V.Class, V.Resource,
   V.subClassOf, V.subPropertyOf, V.domain, V.range, owlThing]

/-- `IS`. -/
def twoIS (t : Term) : Two := if twoHubTerms.contains t then .hub else .rim

/-- The interpretation. `@[reducible]` so that instance search can see
`two.IR` as `Two`; without it every `Decidable` instance below fails to fire. -/
@[reducible] def two : Interpretation where
  IR := Two
  IP := fun p => twoIP p = true
  IEXT := fun p x y => twoIEXT p x y = true
  IS := twoIS
  IB := fun _ => .rim
  IL := fun _ => .rim

/-! ## The conditions, checked

Every lemma in this section is stated over the `Bool` tables and never over a
term, so `decide` never has to compare a fifty-character IRI. The denotations
are resolved once, where each lemma is used as a field of `two_conforming`, by
definitional unfolding. `W3CWitness.lean` does the opposite: its conditions carry
`liveι V.domain` inside four nested quantifiers over a much larger carrier, which
is why that file raises `maxRecDepth` and `maxHeartbeats` and takes minutes to
elaborate. This one takes under a minute over two elements, and the difference is
where the strings are. -/

/-- `IEXT(rim)` is empty, which is what makes every OWL row below vacuous. -/
theorem two_rim_empty : ∀ z s : Two, ¬ (twoIEXT .rim z s = true) := by decide

theorem two_sc_fwd' : ∀ a b : Two, twoIEXT .hub a b = true →
    twoIEXT .hub a .hub = true ∧ twoIEXT .hub b .hub = true ∧
    ∀ x : Two, twoIEXT .hub x a = true → twoIEXT .hub x b = true := by decide

theorem two_sc_bwd' : ∀ a b : Two, twoIEXT .hub a .hub = true →
    twoIEXT .hub b .hub = true →
    (∀ x : Two, twoIEXT .hub x a = true → twoIEXT .hub x b = true) →
    twoIEXT .hub a b = true := by decide

theorem two_sp_fwd' : ∀ a b : Two, twoIEXT .hub a b = true →
    twoIP a = true ∧ twoIP b = true ∧
    ∀ x y : Two, twoIEXT a x y = true → twoIEXT b x y = true := by decide

theorem two_sp_bwd' : ∀ a b : Two, twoIP a = true → twoIP b = true →
    (∀ x y : Two, twoIEXT a x y = true → twoIEXT b x y = true) →
    twoIEXT .hub a b = true := by decide

theorem two_dom_fwd' : ∀ p c : Two, twoIEXT .hub p c = true →
    twoIP p = true ∧ twoIEXT .hub c .hub = true ∧
    ∀ x y : Two, twoIEXT p x y = true → twoIEXT .hub x c = true := by decide

theorem two_dom_bwd' : ∀ p c : Two, twoIP p = true → twoIEXT .hub c .hub = true →
    (∀ x y : Two, twoIEXT p x y = true → twoIEXT .hub x c = true) →
    twoIEXT .hub p c = true := by decide

theorem two_rng_fwd' : ∀ p c : Two, twoIEXT .hub p c = true →
    twoIP p = true ∧ twoIEXT .hub c .hub = true ∧
    ∀ x y : Two, twoIEXT p x y = true → twoIEXT .hub y c = true := by decide

theorem two_rng_bwd' : ∀ p c : Two, twoIP p = true → twoIEXT .hub c .hub = true →
    (∀ x y : Two, twoIEXT p x y = true → twoIEXT .hub y c = true) →
    twoIEXT .hub p c = true := by decide

/-- RBS Table 5.13's two rows and Table 5.2's `owl:Restriction` row fire at
`rim`, whose class extension contains it. They are not vacuous. -/
theorem two_sym_fwd' : ∀ p : Two, twoIEXT .hub p .rim = true →
    ∀ x y : Two, twoIEXT p x y = true → twoIEXT p y x = true := by decide

theorem two_trp_fwd' : ∀ p : Two, twoIEXT .hub p .rim = true →
    ∀ x y z : Two, twoIEXT p x y = true → twoIEXT p y z = true →
    twoIEXT p x z = true := by decide

theorem two_restr_IC' : ∀ x : Two, twoIEXT .hub x .rim = true →
    twoIEXT .hub x .hub = true := by decide

/-- **The conforming interpretation.** -/
theorem two_conforming : Conforming two where
  IR_nonempty := ⟨.hub⟩
  ax_type := ⟨rfl, rfl⟩
  ax_subClassOf := ⟨rfl, rfl⟩
  ax_subPropertyOf := ⟨rfl, rfl⟩
  ax_domain := ⟨rfl, rfl⟩
  ax_range := ⟨rfl, rfl⟩
  sc_fwd := two_sc_fwd'
  sc_bwd := two_sc_bwd'
  sp_fwd := two_sp_fwd'
  sp_bwd := two_sp_bwd'
  dom_fwd := two_dom_fwd'
  dom_bwd := two_dom_bwd'
  rng_fwd := two_rng_fwd'
  rng_bwd := two_rng_bwd'
  same_fwd := fun a b h => absurd h (two_rim_empty a b)
  eqc_fwd := fun a b h => absurd h (two_rim_empty a b)
  eqp_fwd := fun a b h => absurd h (two_rim_empty a b)
  inv_fwd := fun p q h => absurd h (two_rim_empty p q)
  sym_fwd := two_sym_fwd'
  trp_fwd := two_trp_fwd'
  svf_eq := fun z c _ h => absurd h (two_rim_empty z c)
  avf_eq := fun z c _ h => absurd h (two_rim_empty z c)
  hv_eq := fun z a _ h => absurd h (two_rim_empty z a)
  restr_IC := two_restr_IC'
  svf_typ := fun z c h => absurd h (two_rim_empty z c)
  avf_typ := fun z c h => absurd h (two_rim_empty z c)
  onp_typ := fun z p h => absurd h (two_rim_empty z p)
  int_fwd := fun z s _ h => absurd h (two_rim_empty z s)
  uni_fwd := fun z s _ h => absurd h (two_rim_empty z s)
  oneOf_fwd := fun z s _ h => absurd h (two_rim_empty z s)

/-! ## How much of it is alive

Decidability instances first, so that every liveness fact below is settled by
`decide` on the model's own tables and not by a tactic script. -/

instance decTwoIP {p : two.IR} : Decidable (two.IP p) :=
  inferInstanceAs (Decidable (twoIP p = true))

instance decTwoIEXT {p x y : two.IR} : Decidable (two.IEXT p x y) :=
  inferInstanceAs (Decidable (twoIEXT p x y = true))

instance decTwoICEXT {c x : two.IR} : Decidable (two.ICEXT c x) :=
  inferInstanceAs (Decidable (twoIEXT (twoIS V.type) x c = true))

instance decTwoIC {c : two.IR} : Decidable (two.IC c) :=
  inferInstanceAs (Decidable (twoIEXT (twoIS V.type) c (twoIS V.Class) = true))

instance decTwoSat {t : Triple} : Decidable (two.Sat t) :=
  inferInstanceAs (Decidable
    (twoIP (two.den t.p) = true ∧ twoIEXT (two.den t.p) (two.den t.s) (two.den t.o) = true))

/-- **Table 5.8's forward direction SEPARATES a pair.** `( hub , rim )` is not
in `IEXT(I(rdfs:subClassOf))`, and the reason is the one the table gives:
`ICEXT(hub)` is not contained in `ICEXT(rim)`, because `rim` is a class whose
extension holds only itself while `hub`'s holds everything.

This is the fact a saturated model cannot have. It is what makes the forward
direction of Table 5.8 do work here rather than hold by accident. -/
theorem two_table58_separates :
    ¬ two.IEXT (two.IS V.subClassOf) .hub .rim ∧
    two.ICEXT .hub .hub ∧ ¬ two.ICEXT .rim .hub := by decide

/-- **Table 5.8's backward direction FORCES a pair.** `( rim , hub )` IS in
`IEXT(I(rdfs:subClassOf))`, and nothing put it there by hand: `rim` and `hub`
are both classes and `ICEXT(rim) ⊆ ICEXT(hub)`, so the backward direction of the
`iff` demands the triple. -/
theorem two_table58_forces :
    two.IC .rim ∧ two.IC .hub ∧ (∀ x : Two, two.ICEXT .rim x → two.ICEXT .hub x) ∧
    two.IEXT (two.IS V.subClassOf) .rim .hub := by decide

/-- Each of Table 5.8's four rows holds exactly three of the four pairs, and the
pair each one omits is omitted for its own reason: extension containment for
`rdfs:subClassOf`, property-extension containment for `rdfs:subPropertyOf`, and
the subject and object clauses for `rdfs:domain` and `rdfs:range`. The four rows
therefore agree here, and they agree because each is separately satisfied rather
than because the model cannot tell them apart. -/
theorem two_table58_row_reasons :
    (¬ two.IEXT (two.IS V.subClassOf) .hub .rim) ∧
    (¬ two.IEXT (two.IS V.subPropertyOf) .hub .rim) ∧
    (¬ two.IEXT (two.IS V.domain) .hub .rim) ∧
    (¬ two.IEXT (two.IS V.range) .hub .rim) ∧
    two.IEXT (two.IS V.subClassOf) .rim .rim ∧
    two.IEXT (two.IS V.subPropertyOf) .rim .rim ∧
    two.IEXT (two.IS V.domain) .rim .rim ∧
    two.IEXT (two.IS V.range) .rim .rim := by decide

/-- **The fields that hold for want of anything to test, as a list.** Every OWL
term denotes `rim` and `IEXT(rim)` is empty, so these antecedents have no
instances. Kept as a theorem so that a later edit which brings one of them to
life has to update this list rather than quietly leaving it stale. -/
theorem two_dead_fields :
    (∀ a b, ¬ two.IEXT (two.IS V.sameAs) a b) ∧
    (∀ a b, ¬ two.IEXT (two.IS V.equivalentClass) a b) ∧
    (∀ a b, ¬ two.IEXT (two.IS V.equivalentProperty) a b) ∧
    (∀ p q, ¬ two.IEXT (two.IS V.inverseOf) p q) ∧
    (∀ z c, ¬ two.IEXT (two.IS V.someValuesFrom) z c) ∧
    (∀ z c, ¬ two.IEXT (two.IS V.allValuesFrom) z c) ∧
    (∀ z a, ¬ two.IEXT (two.IS V.hasValue) z a) ∧
    (∀ z p, ¬ two.IEXT (two.IS V.onProperty) z p) ∧
    (∀ z s, ¬ two.IEXT (two.IS V.intersectionOf) z s) ∧
    (∀ z s, ¬ two.IEXT (two.IS V.unionOf) z s) ∧
    (∀ z s, ¬ two.IEXT (two.IS V.oneOf) z s) :=
  ⟨two_rim_empty, two_rim_empty, two_rim_empty, two_rim_empty, two_rim_empty,
   two_rim_empty, two_rim_empty, two_rim_empty, two_rim_empty, two_rim_empty,
   two_rim_empty⟩

/-- **The fields that do fire.** Table 5.13's two rows have `rim` in the class
extension they read, and Table 5.2's `owl:Restriction` row has `rim` in
`ICEXT(I(owl:Restriction))`, so all three are applied at a real instance rather
than held vacuously. -/
theorem two_live_fields :
    two.ICEXT (two.IS V.symmetricProperty) .rim ∧
    two.ICEXT (two.IS V.transitiveProperty) .rim ∧
    two.ICEXT (two.IS V.Restriction) .rim ∧ two.IC .rim := by decide

/-! ## The identity rows of RBS Table 5.2, which the existing witness misses -/

/-- Three rows of RBS Table 5.2, and one semantic condition of RDF11 section 8.

> `rdf:Property | ∈ IC | = IP`
> `rdfs:Resource | ∈ IC | = IR`
> `owl:Thing | ∈ IC | = IR`

The first of those is also RDF11 section 8's first RDF semantic condition, "x is
in IP if and only if `< x, I(rdf:Property) >` is in IEXT(I(rdf:type))", which
section 9 restates as "IP = ICEXT(I(rdf:Property))".

None of the three is a field of `Conforming`, because no rule consumes any of
them, and `W3CWitness.lean`'s `live` records itself as violating all three. They
are checked here because a witness that satisfies more of the Recommendation
than it was asked to is worth more than one that satisfies exactly what it was
asked to.

Table 5.2's fourth identity row, `rdfs:Class | ∈ IC | = IC`, is not checked
because it cannot fail: `Interpretation.IC` is DEFINED as
`ICEXT(I(rdfs:Class))`, following RDF11 section 9, so the row holds in every
`Interpretation` by construction. -/
theorem two_satisfies_table_52_identity_rows :
    (∀ p : Two, two.ICEXT (two.IS V.Property) p ↔ two.IP p) ∧
    (∀ x : Two, two.ICEXT (two.IS V.Resource) x) ∧
    (∀ x : Two, two.ICEXT (two.IS owlThing) x) := by decide

/-! ## Every axiomatic triple, not only the five

RDF11 section 8's RDF axioms and section 9's RDFS axiomatic triples, transcribed
from the raw HTML on 15 September 2026. The `rdf:_1` and `rdf:_2` rows stand for
the infinite container-membership families; both tables end in "…". -/

/-- The RDF axioms of RDF11 section 8 and the RDFS axiomatic triples of section
9, in the order the Recommendation lists them. -/
def axiomaticTriples : List Triple :=
  let rdf := fun (s : String) => "<http://www.w3.org/1999/02/22-rdf-syntax-ns#" ++ s ++ ">"
  let rdfs := fun (s : String) => "<http://www.w3.org/2000/01/rdf-schema#" ++ s ++ ">"
  -- RDF axioms, section 8.
  [ ⟨rdf "type", rdf "type", rdf "Property"⟩,
    ⟨rdf "subject", rdf "type", rdf "Property"⟩,
    ⟨rdf "predicate", rdf "type", rdf "Property"⟩,
    ⟨rdf "object", rdf "type", rdf "Property"⟩,
    ⟨rdf "first", rdf "type", rdf "Property"⟩,
    ⟨rdf "rest", rdf "type", rdf "Property"⟩,
    ⟨rdf "value", rdf "type", rdf "Property"⟩,
    ⟨rdf "nil", rdf "type", rdf "List"⟩,
    ⟨rdf "_1", rdf "type", rdf "Property"⟩,
    ⟨rdf "_2", rdf "type", rdf "Property"⟩,
  -- RDFS axiomatic triples, section 9: the rdfs:domain block.
    ⟨rdf "type", rdfs "domain", rdfs "Resource"⟩,
    ⟨rdfs "domain", rdfs "domain", rdf "Property"⟩,
    ⟨rdfs "range", rdfs "domain", rdf "Property"⟩,
    ⟨rdfs "subPropertyOf", rdfs "domain", rdf "Property"⟩,
    ⟨rdfs "subClassOf", rdfs "domain", rdfs "Class"⟩,
    ⟨rdf "subject", rdfs "domain", rdf "Statement"⟩,
    ⟨rdf "predicate", rdfs "domain", rdf "Statement"⟩,
    ⟨rdf "object", rdfs "domain", rdf "Statement"⟩,
    ⟨rdfs "member", rdfs "domain", rdfs "Resource"⟩,
    ⟨rdf "first", rdfs "domain", rdf "List"⟩,
    ⟨rdf "rest", rdfs "domain", rdf "List"⟩,
    ⟨rdfs "seeAlso", rdfs "domain", rdfs "Resource"⟩,
    ⟨rdfs "isDefinedBy", rdfs "domain", rdfs "Resource"⟩,
    ⟨rdfs "comment", rdfs "domain", rdfs "Resource"⟩,
    ⟨rdfs "label", rdfs "domain", rdfs "Resource"⟩,
    ⟨rdf "value", rdfs "domain", rdfs "Resource"⟩,
  -- the rdfs:range block.
    ⟨rdf "type", rdfs "range", rdfs "Class"⟩,
    ⟨rdfs "domain", rdfs "range", rdfs "Class"⟩,
    ⟨rdfs "range", rdfs "range", rdfs "Class"⟩,
    ⟨rdfs "subPropertyOf", rdfs "range", rdf "Property"⟩,
    ⟨rdfs "subClassOf", rdfs "range", rdfs "Class"⟩,
    ⟨rdf "subject", rdfs "range", rdfs "Resource"⟩,
    ⟨rdf "predicate", rdfs "range", rdfs "Resource"⟩,
    ⟨rdf "object", rdfs "range", rdfs "Resource"⟩,
    ⟨rdfs "member", rdfs "range", rdfs "Resource"⟩,
    ⟨rdf "first", rdfs "range", rdfs "Resource"⟩,
    ⟨rdf "rest", rdfs "range", rdf "List"⟩,
    ⟨rdfs "seeAlso", rdfs "range", rdfs "Resource"⟩,
    ⟨rdfs "isDefinedBy", rdfs "range", rdfs "Resource"⟩,
    ⟨rdfs "comment", rdfs "range", rdfs "Literal"⟩,
    ⟨rdfs "label", rdfs "range", rdfs "Literal"⟩,
    ⟨rdf "value", rdfs "range", rdfs "Resource"⟩,
  -- the rdfs:subClassOf and rdfs:subPropertyOf blocks.
    ⟨rdf "Alt", rdfs "subClassOf", rdfs "Container"⟩,
    ⟨rdf "Bag", rdfs "subClassOf", rdfs "Container"⟩,
    ⟨rdf "Seq", rdfs "subClassOf", rdfs "Container"⟩,
    ⟨rdfs "ContainerMembershipProperty", rdfs "subClassOf", rdf "Property"⟩,
    ⟨rdfs "isDefinedBy", rdfs "subPropertyOf", rdfs "seeAlso"⟩,
    ⟨rdfs "Datatype", rdfs "subClassOf", rdfs "Class"⟩,
  -- the container-membership block.
    ⟨rdf "_1", rdf "type", rdfs "ContainerMembershipProperty"⟩,
    ⟨rdf "_1", rdfs "domain", rdfs "Resource"⟩,
    ⟨rdf "_1", rdfs "range", rdfs "Resource"⟩,
    ⟨rdf "_2", rdf "type", rdfs "ContainerMembershipProperty"⟩,
    ⟨rdf "_2", rdfs "domain", rdfs "Resource"⟩,
    ⟨rdf "_2", rdfs "range", rdfs "Resource"⟩ ]

/-- **Every listed axiomatic triple is true here**, not only the five that
`Conforming` carries. Fifty-four triples, checked one by one. -/
theorem two_satisfies_every_listed_axiomatic_triple :
    ∀ t ∈ axiomaticTriples, two.Sat t := by decide

/-- There are fifty-four of them, so that shortening the list to make a `decide`
cheaper breaks this rather than passing quietly. -/
theorem axiomaticTriples_count : axiomaticTriples.length = 54 := by decide

/-! ## Something it does not entail -/

/-- An ordinary IRI, denoting `rim`. -/
def exA : Term := "<http://example.org/a>"
/-- A second one. -/
def exB : Term := "<http://example.org/b>"
/-- A third, used as a predicate. Its denotation is `rim`, whose extension is
empty, so no triple with this predicate is true here. -/
def exP : Term := "<http://example.org/p>"
/-- A fourth, used as a class. -/
def exC : Term := "<http://example.org/C>"
/-- A fifth, used as a superclass. -/
def exD : Term := "<http://example.org/D>"

/-! ## And something it does entail

A non-entailment on its own leaves open that `ConformingEntails` is empty, which
would make `certificate_conforming_sound` true and useless. This is the other
side: one OWL 2 RL rule, `rdfs9`, at a concrete instance, true in every
conforming interpretation of its premises.

It is stated through `ConformingEntails.of_entails`, which is the same route
`certificate_conforming_sound` takes, so what it exercises is the bridge and not
a shortcut past it. `checkCert` itself is not run here for the reason
`Mixed.lean` gives at its own `decide` boundary: `checkCert` carries its derived
set in a `Std.HashSet`, and deciding a `checkCert` application would put
`Lean.ofReduceBool` on the trust surface. -/

/-- **`rdfs9` is sound in every conforming interpretation**, at one instance:
from `ex:a rdf:type ex:C` and `ex:C rdfs:subClassOf ex:D`, every conforming
interpretation of those two triples satisfies `ex:a rdf:type ex:D`.

The proof goes through `Entails` and `ConformingEntails.of_entails`, so the
bridge carries it; nothing about the two-element model is used. -/
theorem rdfs9_holds_in_every_conforming_interpretation :
    ConformingEntails [⟨exA, V.type, exC⟩, ⟨exC, V.subClassOf, exD⟩]
      ⟨exA, V.type, exD⟩ :=
  ConformingEntails.of_entails (fun I M =>
    M.conds.sc_sub (I.ι exC) (I.ι exD)
      (M.facts ⟨exC, V.subClassOf, exD⟩ (List.Mem.tail _ (List.Mem.head _)))
      (I.ι exA)
      (M.facts ⟨exA, V.type, exC⟩ (List.Mem.head _)))

/-- **The empty graph does not conforming-entail everything.** The predicate of
the conclusion denotes `rim` and `IEXT(rim)` is empty, so the triple is false in
this interpretation and the empty graph is satisfied by it.

`Witness.lean`'s `not_everything_is_entailed` is the same shape over the much
larger `Conditions` class, and `W3CWitness.lean`'s
`not_everything_is_w3c_entailed` over `W3CModel`. This one is over the smallest
class of the three, which makes it the strongest of the three. -/
theorem not_everything_is_conforming_entailed :
    ¬ ConformingEntails [] ⟨exA, exP, exB⟩ := by
  intro h
  exact absurd (h two two_conforming (fun _ hm => nomatch hm)).2 (by decide)

/-- **A non-entailment from a graph the interpretation actually satisfies**,
which the previous one does not deliver: `ex:a rdf:type ex:C` holds here, and
`rdf:type rdf:type ex:C` does not, because `( hub , rim )` is the one pair
`IEXT(hub)` omits.

The rule `rdfs9` would draw the second from the first together with
`ex:C rdfs:subClassOf ex:C`, which is also true here; what fails is the step from
a typing of `ex:a` to a typing of `rdf:type`, and nothing licenses it. -/
theorem a_typed_individual_does_not_type_rdf_type :
    two.Sat ⟨exA, V.type, exC⟩ ∧ ¬ two.Sat ⟨V.type, V.type, exC⟩ ∧
    ¬ ConformingEntails [⟨exA, V.type, exC⟩] ⟨V.type, V.type, exC⟩ := by
  refine ⟨by decide, by decide, ?_⟩
  intro h
  have hsat := h two two_conforming (fun u hu => by
    cases hu with
    | head => exact (by decide : two.Sat ⟨exA, V.type, exC⟩)
    | tail _ hm => exact nomatch hm)
  exact absurd hsat (by decide)

/-! ## What this model is known NOT to satisfy

Stated as a theorem for the same reason the vacuity list is: a limitation that
lives only in prose is a limitation that goes stale. -/

/-- RBS Table 5.2 gives `owl:Nothing` the row `owl:Nothing | ∈ IC | = ∅`, and
this model violates it: `owl:Nothing` denotes `rim`, whose class extension
contains `rim`. It is not a field of `Conforming`, because no rule consumes it,
which is exactly why this model can exist while a fully conforming
interpretation would need more elements.

This is the shape of every remaining gap between `Conforming` and the
Recommendation: a row nothing consumes, left out because leaving it out enlarges
the class, and therefore a row this witness is free to break. -/
theorem two_violates_these_known_rows :
    two.ICEXT (two.IS "<http://www.w3.org/2002/07/owl#Nothing>") .rim := by decide

/-! ## The axiom tripwire

`decide` in core Lean is checked by the kernel and adds no axiom. `native_decide`
would add `Lean.ofReduceBool`, and a `sorry` would add `sorryAx`, so pinning the
footprint here is what keeps every `decide` in this file honest. The three
listed are Lean's own and reach here through `String.data`, which `termKind`
reads to tell an IRI from a literal.
-/

/-- info: 'OOCert.two_conforming' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms two_conforming

/-- info: 'OOCert.two_satisfies_every_listed_axiomatic_triple' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms two_satisfies_every_listed_axiomatic_triple

/-- info: 'OOCert.not_everything_is_conforming_entailed' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms not_everything_is_conforming_entailed

/-- info: 'OOCert.a_typed_individual_does_not_type_rdf_type' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms a_typed_individual_does_not_type_rdf_type

/-- info: 'OOCert.rdfs9_holds_in_every_conforming_interpretation' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms rdfs9_holds_in_every_conforming_interpretation

end OOCert
