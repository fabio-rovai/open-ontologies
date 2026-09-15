import OOCert.Soundness

/-!
# The specification conditions, and the fourteen arms that were assumed

`Semantics.lean` states `Conditions`, and fourteen of its arms are *posited*:
the theorem that `scm-dom1` is sound reads a field off `Conditions` which says
that `scm-dom1` holds. The docstring beside each such field argues in English
that the field follows from the OWL 2 RDF-Based Semantics. This file replaces
those fourteen English arguments with machine-checked proofs, and it does so
without touching `Conditions`, `Interp`, `Entails` or any existing statement.

## What is here

`W3C I IP` carries the specification's semantic conditions at FULL strength,
one field per table row, each with the cell quoted verbatim above it.
`W3CModel I IP G` adds the asserted graph and the three list-constructor rows.
`W3CModel.toModel` builds a `Model I G` from it, and TWELVE of the twenty-three
condition fields it supplies are DERIVATIONS rather than projections. Those
twelve fields carry FOURTEEN of the soundness proof's arms, because `eqc` and
`eqp` each license two conclusions and `Soundness.lean` consumes each of them
twice. Counted as arms, they are exactly the ones that used to be assumed:

| arm | rule id | derived in |
|---|---|---|
| `sc_trans` | rdfs11 | `W3C.sc_trans` |
| `sp_trans` | rdfs5 | `W3C.sp_trans` |
| `eqc` (two conclusions) | scm-eqc1 | `W3C.eqc` |
| `eqp` (two conclusions) | scm-eqp1 | `W3C.eqp` |
| `svf_sc` | scm-svf1 | `W3C.svf_sc` |
| `svf_sp` | scm-svf2 | `W3C.svf_sp` |
| `avf_sc` | scm-avf1 | `W3C.avf_sc` |
| `avf_sp` | scm-avf2 | `W3C.avf_sp` |
| `dom_sc` | scm-dom1 | `W3C.dom_sc` |
| `dom_sp` | scm-dom2 | `W3C.dom_sp` |
| `rng_sc` | scm-rng1 | `W3C.rng_sc` |
| `rng_sp` | scm-rng2 | `W3C.rng_sp` |

Twelve of those fourteen arms (everything but `sc_trans` and `sp_trans`) are
stated by NO cell of any specification table. They conclude a `rdfs:subClassOf`,
`rdfs:subPropertyOf`, `rdfs:domain` or `rdfs:range` triple that no premise
asserts, and the only thing in the OWL 2 RDF-Based Semantics that can put such
a triple into an extension is the BACKWARD direction of Table 5.8's `iff`.
`rdfs11` and `rdfs5` are the other two: RDF 1.1 Semantics section 9 does state
them, so they were licensed, but Table 5.8 makes them redundant and they are
derived here as well.

## Why this is a separate structure and not a change to `Conditions`

Every field of `W3C` is at least as strong as the corresponding field of
`Conditions`, and several are strictly stronger. Adding any of them to
`Conditions` would SHRINK the model class, which makes `Entails` easier to
satisfy and weakens `certificate_sound` with no error appearing anywhere and
every proof still passing. That is the failure mode the weakest-conditions
discipline exists to prevent, so the strengthening is quarantined here, where
it shrinks nothing: `Entails` does not mention this structure.

The theorem that pays for the quarantine is `W3CEntails.of_entails`. It says
that everything `Entails` already gives is true in every `W3CModel`, and the
fourteen arms that used to be posited are steps inside its proof.
`certificate_sound` keeps its exact statement and its exact axiom footprint.

**`W3CModel` is not the same thing as a conforming interpretation, and no
document in this repository may say that it is.** Two gaps used to separate them.
The first, that the reading of one as the other was prose rather than a theorem,
is closed: `Conforming.lean` formalises the specification's interpretation and
`OOCert.Conforming.toW3CModel` is that reading, proved. The second remains:
`W3C` omits every table row no rule consumes, so its class is strictly larger
than the image of the conforming interpretations under that map. Both gaps run in
the safe direction for THIS theorem and neither runs in the safe direction for a
non-entailment result, so a refutation over `W3CModel` is still a statement about
`W3CModel` and not about the specification.

`IP` is a PARAMETER of this structure rather than a field of `Interp`, which is
what makes the whole file additive. It stays completely free INSIDE the
derivations: not one of the fourteen consumes a free-standing `IP` membership,
because every `IP` fact they use is produced by a forward condition applied to a
premise triple.

That is a fact about the derivations and it is NOT a claim about the layer as a
whole, which an earlier draft of this paragraph made. It said that the second
formalisation in `isabelle/` needs three axiomatic-triple consequences
(`c_type_IP`, `c_sco_IP`, `c_spo_IP`, `OO_Semantics.thy`) for the same table
while "this file needs none, because `Interp.sat` has no `IP` conjunct". The
conclusion is inverted. Dropping the `IP` conjunct from `Interp.sat` does not
remove those obligations, it MOVES them out of the proofs and into the bridge,
and the bridge needs FIVE of them where Isabelle needs three. They are listed in
the next section, because a cost that is paid somewhere else is still paid.

## The bridge to a conforming interpretation, WHICH IS NOW A THEOREM

**Everything in this section describes the state of the layer before 15
September 2026 and is superseded by `Conforming.lean`.** It is kept because the
argument below is still the argument, and because the correction of one of its
five assumed facts should stay next to the claim it corrects. What changed is
that there is now a Lean formalisation of an OWL 2 RDF-Based interpretation to
quantify over, `OOCert.Interpretation`, so the reading described here is
`OOCert.Conforming.toW3C` and the five facts listed below are fields of
`OOCert.Conforming`, quoted out of the axiomatic-triple tables and discharged
rather than assumed. `OOCert.certificate_conforming_sound` is the sentence this
file's `certificate_w3c_sound` says of itself that it is not yet.

**One entry in the table below is wrong and the correction is recorded rather
than silently applied.** The `I(rdfs:range) ∈ IP` row sources the fact to
`rdfs:range rdfs:domain rdf:Property .` with the note "same", pointing at the row
above it, which reads "whose truth puts its own predicate in `IP`". The predicate
of that triple is `rdfs:domain`, so its truth gives `I(rdfs:domain) ∈ IP` and not
`I(rdfs:range) ∈ IP`; reaching `rdfs:range` from it needs a further step through
Table 5.8's `rdfs:domain` row, which the entry does not mention.
`Conforming.lean` takes the fact from `rdf:type rdfs:range rdfs:Class .`
instead, whose predicate IS `rdfs:range`, so one application of the truth clause
settles it and no table is consulted.

Given a conforming interpretation in the sense of OWL 2 RDF-Based Semantics
section 5, take `D := IR`, `ι := I`, and

> `iext p x y := IP p ∧ (x, y) ∈ IEXT p`.

Then `Interp.sat t` unfolds to `IP(I(t.p)) ∧ (I(t.s), I(t.o)) ∈ IEXT(I(t.p))`,
which is RDF 1.1 Semantics section 5's truth condition, in that section's
"Semantic conditions for ground graphs" table, read out of the raw HTML of
<https://www.w3.org/TR/rdf11-mt/> on 15 September 2026:

> if E is a ground triple s p o `.` then I(E) = true if
> I(p) is in IP and the pair <I(s),I(o)> is in IEXT(I(p))
> otherwise I(E) = false.

The `IP` conjunct in `iext` is not an addition to that clause, it IS the
clause's own first conjunct, and the same section's item 3 gives the reason it
has to be there: "A mapping IEXT from IP into the powerset of IR x IR", so
`IEXT` is not defined off `IP` at all. Without the conjunct the Lean conclusion
would give membership in an extension without giving the predicate's denotation
to be a property. Under that reading each field of `W3C` below is the
corresponding table cell, and `Interp.cext` is section 9's `ICEXT`.

An earlier draft of this paragraph quoted the RDF 1.0 (2004) wording of the same
clause, "if I(s), I(p) and I(o) are all defined and <I(s),I(o)> is in
IEXT(I(p))", and called it RDF 1.1 verbatim. It is not; RDF 1.1 replaced the
definedness phrasing with the explicit `IP` membership above. The argument is
unaffected and in fact stronger, but a quote that is not the quote is exactly
the defect this file exists to remove, so the correction is recorded rather than
silently applied.

`Interp.sat` is deliberately NOT changed to carry an `IP` conjunct. Doing so
would move `Entails` in two directions at once, making `facts` harder to supply
and the conclusion harder to use, and it buys nothing the bridge definition
above does not already buy.

### The five facts the bridge assumes, which are not cells of anything quoted here

Read `iext` that way and a conforming interpretation does NOT satisfy every field
below for free. `I.sc a b` unfolds to `IP(I(rdfs:subClassOf)) ∧ (a, b) ∈
IEXT(I(rdfs:subClassOf))`, and the table cell supplies only the second conjunct;
`I.cext c x` unfolds to `IP(I(rdf:type)) ∧ (x, c) ∈ IEXT(I(rdf:type))`, and the
same is true there. So five `IP` memberships have to come from somewhere, and
every one of them comes from the RDF and RDFS AXIOMATIC TRIPLE tables, which this
file quotes no cell of.

| fact | needed by | source, re-read in the Recommendation on 15 September 2026 |
|---|---|---|
| `I(rdf:type) ∈ IP` | every field whose conclusion mentions `cext`, which is all of `sc_fwd`, `dom_fwd`, `rng_fwd`, `eqc_fwd`, `svf_eq`, `avf_eq`, `hv_eq`, `restr_IC`, `svf_typ`, `avf_typ`, `onp_typ`, and the `IC` antecedents of `sc_bwd`, `dom_bwd` and `rng_bwd` | RDF axiomatic triple `rdf:type rdf:type rdf:Property .`, whose truth puts its own predicate in `IP` by the section 5 clause above |
| `I(rdfs:subClassOf) ∈ IP` | `sc_bwd` | RDFS axiomatic triple `rdfs:subClassOf rdfs:domain rdfs:Class .`, read through Table 5.8 row 3 forward |
| `I(rdfs:subPropertyOf) ∈ IP` | `sp_bwd` | RDFS axiomatic triple `rdfs:subPropertyOf rdfs:domain rdf:Property .`, same row |
| `I(rdfs:domain) ∈ IP` | `dom_bwd` | RDFS axiomatic triple `rdfs:domain rdfs:domain rdf:Property .`, whose truth puts its own predicate in `IP` |
| `I(rdfs:range) ∈ IP` | `rng_bwd` | RDFS axiomatic triple `rdfs:range rdfs:domain rdf:Property .`, same |

Two fields consume one further `IP` membership in their bridge reading, and it is
NOT a sixth entry because it comes from a cell this file already quotes.
`svf_eq` and `hv_eq` conclude `I.iext p x y` in their left-to-right direction,
which under the bridge carries `IP p`, and Table 5.6 gives only the set
membership; `IP p` comes from Table 5.3's `owl:onProperty` row, quoted above
`onp_typ`. `avf_eq` needs nothing extra, because there `I.iext p x y` is a
hypothesis and supplies its own.

All five axiomatic triples were checked against the raw HTML of
<https://www.w3.org/TR/rdf11-mt/> on 15 September 2026, in the tables of sections
8 and 9. The second formalisation makes three of the same assumptions explicitly,
as `c_type_IP`, `c_sco_IP` and `c_spo_IP` in `isabelle/OO_Semantics.thy`, and it
needs no counterpart to the last two because its conditions are stated over the
raw `IEXT I (IS I rdfs_domain)` rather than over a bridged relation. On this
point the second kernel is AHEAD of this one and the earlier draft of this file
claimed the reverse.

## Sources, re-fetched from the raw HTML rather than from a rendering

All quotes below are from <https://www.w3.org/TR/owl2-rdf-based-semantics/>,
first fetched 14 September 2026 and re-fetched independently on 15 September
2026, read out of the raw HTML with the markup intact. Every cell cited in this
file was checked against that second fetch, one by one: Tables 5.2 (`rdfs:Class`
`= IC`, `owl:Restriction` `⊆ IC`, and the `rdf:Property` `= IP` and `owl:Thing`
`= IR` rows this structure deliberately omits), 5.3 (the `someValuesFrom`,
`allValuesFrom` and `onProperty` typing rows), 5.6 (the `if`/`then` header and
the three set equalities), 5.8, 5.9, 5.12 and 5.13, plus the sequence-expression
definition and, from RDF 1.1 Semantics <https://www.w3.org/TR/rdf11-mt/>,
section 5's truth clause and `IEXT` domain and section 9's transitivity
conditions. A
markdown conversion of these tables loses the `rowspan` attributes and renders
Table 5.8's `iff` as `if`, which would make the twelve-arm result unprovable
and the mistake invisible. The single load-bearing fact is Table 5.8's
connective cell, which in the source reads

```html
</td><th rowspan="4" style="text-align: center"> iff
```

at source line 2676, inside the `<table>` whose caption is "Table 5.8: Semantic
Conditions for the RDFS Vocabulary", and that table's body has exactly four
`<tr>` rows: `rdfs:subClassOf`, `rdfs:subPropertyOf`, `rdfs:domain`,
`rdfs:range`. The section prose agrees: "The semantic conditions provided here
are "iff" conditions, while the original semantic conditions, as specified in
Section 4.1 of the RDF Semantics, are weaker "if-then" conditions."
-/
namespace OOCert

namespace V
/-- `rdfs:Class`. RBS Table 5.2 gives it the row

```
rdfs:Class | ∈ IC | = IC
```

where the third column is headed `ICEXT(I(E))`. So `ICEXT(I(rdfs:Class))` IS
`IC`, which is what licenses `Interp.IC` below being a DEFINITION rather than a
condition. A definition cannot shrink the model class; a condition can. -/
def Class : Term := "<http://www.w3.org/2000/01/rdf-schema#Class>"

/-- `owl:Restriction`. RBS Table 5.2, row

```
owl:Restriction | ∈ IC | ⊆ IC
```

SUBSET and never equality. Writing the equality here would be a strengthening
the specification does not make, and it would force every class into
`ICEXT(owl:Restriction)`. -/
def Restriction : Term := "<http://www.w3.org/2002/07/owl#Restriction>"
end V

namespace Interp

/-- `IC`, the set of classes. RDF 1.1 Semantics section 9 introduces
`ICEXT(y) = { x : <x,y> ∈ IEXT(I(rdf:type)) }` as an abbreviation, which is
`Interp.cext`, and RBS Table 5.2 gives `rdfs:Class` the class-extension entry
`= IC`. Composing the two makes this a definition. -/
def IC (I : Interp) (c : I.D) : Prop := I.cext (I.ι V.Class) c

end Interp

/-! ## The specification conditions

One field per table row, at full strength, with the cell quoted above it. Read
the quotes as the licence for every strengthening in this file: a field with no
quote above it is a field nobody checked.

`Conditions` has twenty-three fields and they land here in three groups. THREE
survive unchanged (`same_fwd`, `sym_fwd`, `trp_fwd`). EIGHT are consequences of
a strictly stronger field here (`sc_sub`, `sp_sub`, `dom`, `rng` and `inv` of
the forward conditions that also carry the `IC` and `IP` memberships; `svf`,
`avf` and `hv` of the Table 5.6 equalities). TWELVE have no counterpart in this
structure at all, because over it they are theorems rather than assumptions.
`Conditions` itself is untouched and still carries all twenty-three; `toModel`
is what supplies them, and for those twelve it supplies a proof. -/

/-- The OWL 2 RDF-Based Semantics conditions that the twenty-nine rules consume,
at FULL specification strength. -/
structure W3C (I : Interp) (IP : I.D → Prop) : Prop where
  /-- RBS Table 5.8, row 1, forward. The `iff` cell carries `rowspan="4"` and
  governs this row and the three below it.

  > `( c₁ , c₂ ) ∈ IEXT(I(rdfs:subClassOf))` **iff** `c₁ , c₂ ∈ IC ,
  > ICEXT(c₁) ⊆ ICEXT(c₂)` -/
  sc_fwd : ∀ a b, I.sc a b → I.IC a ∧ I.IC b ∧ ∀ x, I.cext a x → I.cext b x
  /-- RBS Table 5.8, row 1, backward. Same cell.

  **Trap T2, guarded here.** Dropping the `IC` conjuncts from this direction
  makes every derivation below shorter and never fails a proof. It would also
  force a `rdfs:subClassOf` triple between any two things with nested
  extensions, including things that are not classes. They are kept. -/
  sc_bwd : ∀ a b, I.IC a → I.IC b → (∀ x, I.cext a x → I.cext b x) → I.sc a b
  /-- RBS Table 5.8, row 2, forward.

  > `( p₁ , p₂ ) ∈ IEXT(I(rdfs:subPropertyOf))` **iff** `p₁ , p₂ ∈ IP ,
  > IEXT(p₁) ⊆ IEXT(p₂)` -/
  sp_fwd : ∀ a b, I.sp a b → IP a ∧ IP b ∧ ∀ x y, I.iext a x y → I.iext b x y
  /-- RBS Table 5.8, row 2, backward. Same cell; `IP` conjuncts kept, per T2. -/
  sp_bwd : ∀ a b, IP a → IP b → (∀ x y, I.iext a x y → I.iext b x y) → I.sp a b
  /-- RBS Table 5.8, row 3, forward.

  > `( p , c ) ∈ IEXT(I(rdfs:domain))` **iff** `p ∈ IP , c ∈ IC ,
  > ∀ x , y : ( x , y ) ∈ IEXT(p) implies x ∈ ICEXT(c)`

  RDFS alone gives `rdfs:domain` only the `if-then` direction, and `scm-dom1`
  does NOT follow from it. This row being inside Table 5.8's `rowspan="4"` is
  the whole reason the twelve arms are derivable. -/
  dom_fwd : ∀ p c, I.iext (I.ι V.domain) p c →
    IP p ∧ I.IC c ∧ ∀ x y, I.iext p x y → I.cext c x
  /-- RBS Table 5.8, row 3, backward. Same cell; conjuncts kept, per T2. -/
  dom_bwd : ∀ p c, IP p → I.IC c → (∀ x y, I.iext p x y → I.cext c x) →
    I.iext (I.ι V.domain) p c
  /-- RBS Table 5.8, row 4, forward.

  > `( p , c ) ∈ IEXT(I(rdfs:range))` **iff** `p ∈ IP , c ∈ IC ,
  > ∀ x , y : ( x , y ) ∈ IEXT(p) implies y ∈ ICEXT(c)`

  The range row differs from the domain row in exactly one letter, `y` for `x`
  in the extension clause, and the four range derivations below differ from the
  four domain ones by exactly that letter. -/
  rng_fwd : ∀ p c, I.iext (I.ι V.range) p c →
    IP p ∧ I.IC c ∧ ∀ x y, I.iext p x y → I.cext c y
  /-- RBS Table 5.8, row 4, backward. Same cell; conjuncts kept, per T2. -/
  rng_bwd : ∀ p c, IP p → I.IC c → (∀ x y, I.iext p x y → I.cext c y) →
    I.iext (I.ι V.range) p c
  /-- RBS Table 5.9, forward. The `iff` cell there carries `rowspan="6"`.

  > `( c₁ , c₂ ) ∈ IEXT(I(owl:equivalentClass))` **iff** `c₁ , c₂ ∈ IC ,
  > ICEXT(c₁) = ICEXT(c₂)`

  Note what the cell does NOT say: it gives an extension EQUALITY and never a
  `rdfs:subClassOf` triple. That is why `Conditions.eqc`, which hands back two
  such triples, is a restatement of `scm-eqc1` rather than a reading of a
  table, and why it counts among the fourteen. -/
  eqc_fwd : ∀ a b, I.iext (I.ι V.equivalentClass) a b →
    I.IC a ∧ I.IC b ∧ ∀ x, (I.cext a x ↔ I.cext b x)
  /-- RBS Table 5.9, forward.

  > `( p₁ , p₂ ) ∈ IEXT(I(owl:equivalentProperty))` **iff** `p₁ , p₂ ∈ IP ,
  > IEXT(p₁) = IEXT(p₂)` -/
  eqp_fwd : ∀ a b, I.iext (I.ι V.equivalentProperty) a b →
    IP a ∧ IP b ∧ ∀ x y, (I.iext a x y ↔ I.iext b x y)
  /-- RBS Table 5.9, forward direction only.

  > `( a₁ , a₂ ) ∈ IEXT(I(owl:sameAs))` **iff** `a₁ = a₂`

  The converse would force `owl:sameAs` to hold reflexively across the entire
  universe, and `eq-sym` is the only rule that reads this condition. Weakest
  conditions: the converse is not written. -/
  same_fwd : ∀ a b, I.iext (I.ι V.sameAs) a b → a = b
  /-- RBS Table 5.12, forward direction only.

  > `( p₁ , p₂ ) ∈ IEXT(I(owl:inverseOf))` **iff** `p₁ , p₂ ∈ IP ,
  > IEXT(p₁) = { ( x , y ) | ( y , x ) ∈ IEXT(p₂) }`

  Both inclusions of the set equality are kept: `prp-inv1` and `prp-inv2` use
  opposite ones. -/
  inv_fwd : ∀ p q, I.iext (I.ι V.inverseOf) p q →
    IP p ∧ IP q ∧ ∀ x y, (I.iext p x y ↔ I.iext q y x)
  /-- RBS Table 5.13, the universally quantified part only.

  > `p ∈ ICEXT(I(owl:SymmetricProperty))` **iff** `p ∈ IP , ∀ x , y :
  > ( x , y ) ∈ IEXT(p) implies ( y , x ) ∈ IEXT(p)`

  The `p ∈ IP` conjunct is dropped because `prp-symp`'s head predicate is the
  same `?p` that its body triple already carries. Keeping it would be faithful
  and redundant; dropping it is the weaker condition. -/
  sym_fwd : ∀ p, I.cext (I.ι V.symmetricProperty) p → ∀ x y, I.iext p x y → I.iext p y x
  /-- RBS Table 5.13, the universally quantified part only.

  > `p ∈ ICEXT(I(owl:TransitiveProperty))` **iff** `p ∈ IP , ∀ x , y , z :
  > ( x , y ) ∈ IEXT(p) and ( y , z ) ∈ IEXT(p) implies ( x , z ) ∈ IEXT(p)` -/
  trp_fwd : ∀ p, I.cext (I.ι V.transitiveProperty) p →
    ∀ x y z, I.iext p x y → I.iext p y z → I.iext p x z
  /-- RBS Table 5.6.

  > **if** `( z , c ) ∈ IEXT(I(owl:someValuesFrom)) , ( z , p ) ∈
  > IEXT(I(owl:onProperty))` **then**
  > `ICEXT(z) = { x | ∃ y : ( x , y ) ∈ IEXT(p) and y ∈ ICEXT(c) }`

  Two things about this row are load-bearing and they pull in opposite
  directions. The OUTER connective is an IF-THEN, not an `iff`: the section's
  own note reads "All the semantic conditions are "if-then" conditions, since
  the corresponding OWL 2 language constructs are class expressions", and
  Tables 5.8 and 5.9 two sections away ARE `iff`s, so a reader carrying that
  habit into 5.6 shrinks the model class while every proof still passes. The
  CONSEQUENT is a set EQUALITY, and both inclusions are consumed:
  `⊇` by `cls-svf1`, `⊆` by `scm-svf1` and `scm-svf2`.

  **This is the sharpest trap in the file.** Writing only `⊆` is the safe
  error, because `cls-svf1` then refuses to go through. The dangerous moment is
  the repair: faced with a failing `scm-svf1` the natural move is to add a
  fresh field asserting the missing direction in slightly different words, and
  that is a strengthening in a place the specification did not strengthen, it
  turns everything green, and nobody writes it down. The twelve fields of
  `Conditions` that this structure does not restate are that move, made twelve
  times, and they are derived below instead. The equality is written here ONCE,
  as the specification has it, and no second field in this area may be added to
  close a proof. -/
  svf_eq : ∀ z c p, I.iext (I.ι V.someValuesFrom) z c → I.iext (I.ι V.onProperty) z p →
    ∀ x, (I.cext z x ↔ ∃ y, I.iext p x y ∧ I.cext c y)
  /-- RBS Table 5.6.

  > **if** `( z , c ) ∈ IEXT(I(owl:allValuesFrom)) , ( z , p ) ∈
  > IEXT(I(owl:onProperty))` **then**
  > `ICEXT(z) = { x | ∀ y : ( x , y ) ∈ IEXT(p) implies y ∈ ICEXT(c) }`

  Both inclusions consumed: `⊆` by `cls-avf`, `⊇` by `scm-avf1` and
  `scm-avf2`. Same discipline as `svf_eq`. -/
  avf_eq : ∀ z c p, I.iext (I.ι V.allValuesFrom) z c → I.iext (I.ι V.onProperty) z p →
    ∀ x, (I.cext z x ↔ ∀ y, I.iext p x y → I.cext c y)
  /-- RBS Table 5.6.

  > **if** `( z , a ) ∈ IEXT(I(owl:hasValue)) , ( z , p ) ∈
  > IEXT(I(owl:onProperty))` **then** `ICEXT(z) = { x | ( x , a ) ∈ IEXT(p) }`

  `cls-hv1` needs `⊆`, `cls-hv2` needs `⊇`, so `Conditions.hv` already carried
  the equality and this field is the same statement. -/
  hv_eq : ∀ z a p, I.iext (I.ι V.hasValue) z a → I.iext (I.ι V.onProperty) z p →
    ∀ x, (I.cext z x ↔ I.iext p x a)
  /-- RBS Table 5.2, the `owl:Restriction` row: `ICEXT(I(owl:Restriction)) ⊆ IC`.

  Needed by all four restriction-ordering derivations, whose conclusion is a
  `rdfs:subClassOf` triple and whose `sc_bwd` antecedent therefore demands
  `IC`. Nothing else in this structure supplies it, because those rules'
  premises mention only `someValuesFrom`, `allValuesFrom` and `onProperty`. -/
  restr_IC : ∀ x, I.cext (I.ι V.Restriction) x → I.IC x
  /-- RBS Table 5.3, the `owl:someValuesFrom` row:
  `IEXT(I(owl:someValuesFrom)) ⊆ ICEXT(I(owl:Restriction)) × IC`. -/
  svf_typ : ∀ z c, I.iext (I.ι V.someValuesFrom) z c →
    I.cext (I.ι V.Restriction) z ∧ I.IC c
  /-- RBS Table 5.3, the `owl:allValuesFrom` row:
  `IEXT(I(owl:allValuesFrom)) ⊆ ICEXT(I(owl:Restriction)) × IC`. -/
  avf_typ : ∀ z c, I.iext (I.ι V.allValuesFrom) z c →
    I.cext (I.ι V.Restriction) z ∧ I.IC c
  /-- RBS Table 5.3, the `owl:onProperty` row:
  `IEXT(I(owl:onProperty)) ⊆ ICEXT(I(owl:Restriction)) × IP`. -/
  onp_typ : ∀ z p, I.iext (I.ι V.onProperty) z p →
    I.cext (I.ι V.Restriction) z ∧ IP p

/-! ## The fourteen derivations

Every theorem in this section is a proof term over `W3C`, not a projection out
of it. Each replaces one field of `Conditions` and therefore one arm of
`checkStep_sound`. -/

namespace W3C
variable {I : Interp} {IP : I.D → Prop}

/-! ### Group A: domain and range, four arms

These are the cheapest derivations in the set, and the reason `scm-dom1` had to
be checked rather than assumed. Each is Table 5.8 forward on one argument, the
subclass or subproperty inclusion, then Table 5.8 backward on the other. -/

/-- **scm-dom1.** `T(?p, rdfs:domain, ?c1)`, `T(?c1, rdfs:subClassOf, ?c2)`
gives `T(?p, rdfs:domain, ?c2)`. A domain may be weakened to a superclass.

Table 5.8 row 3 left to right on `c1`, then `ICEXT(c1) ⊆ ICEXT(c2)` from row 1,
then row 3 right to left on `c2`. The `IC c2` the backward direction demands
comes from row 1's forward direction, not from anywhere new. -/
theorem dom_sc (W : W3C I IP) (p c1 c2 : I.D)
    (hd : I.iext (I.ι V.domain) p c1) (hsc : I.sc c1 c2) :
    I.iext (I.ι V.domain) p c2 :=
  W.dom_bwd p c2 (W.dom_fwd p c1 hd).1 (W.sc_fwd c1 c2 hsc).2.1
    (fun x y hxy => (W.sc_fwd c1 c2 hsc).2.2 x ((W.dom_fwd p c1 hd).2.2 x y hxy))

/-- **scm-dom2.** `T(?p2, rdfs:domain, ?c)`, `T(?p1, rdfs:subPropertyOf, ?p2)`
gives `T(?p1, rdfs:domain, ?c)`. A domain is inherited by every subproperty.

Table 5.8 row 3 left to right on `p2`, then `IEXT(p1) ⊆ IEXT(p2)` from row 2,
so every pair of `p1` is a pair of `p2` and its subject is already in
`ICEXT(c)`, then row 3 right to left on `p1`. The composition runs the other
way round from `dom_sc`'s. -/
theorem dom_sp (W : W3C I IP) (p1 p2 c : I.D)
    (hd : I.iext (I.ι V.domain) p2 c) (hsp : I.sp p1 p2) :
    I.iext (I.ι V.domain) p1 c :=
  W.dom_bwd p1 c (W.sp_fwd p1 p2 hsp).1 (W.dom_fwd p2 c hd).2.1
    (fun x y hxy => (W.dom_fwd p2 c hd).2.2 x y ((W.sp_fwd p1 p2 hsp).2.2 x y hxy))

/-- **scm-rng1.** `T(?p, rdfs:range, ?c1)`, `T(?c1, rdfs:subClassOf, ?c2)`
gives `T(?p, rdfs:range, ?c2)`. `dom_sc` with row 4 for row 3, which is `y` for
`x` in the extension clause and nothing else. -/
theorem rng_sc (W : W3C I IP) (p c1 c2 : I.D)
    (hr : I.iext (I.ι V.range) p c1) (hsc : I.sc c1 c2) :
    I.iext (I.ι V.range) p c2 :=
  W.rng_bwd p c2 (W.rng_fwd p c1 hr).1 (W.sc_fwd c1 c2 hsc).2.1
    (fun x y hxy => (W.sc_fwd c1 c2 hsc).2.2 y ((W.rng_fwd p c1 hr).2.2 x y hxy))

/-- **scm-rng2.** `T(?p2, rdfs:range, ?c)`, `T(?p1, rdfs:subPropertyOf, ?p2)`
gives `T(?p1, rdfs:range, ?c)`. `dom_sp` with row 4 for row 3. -/
theorem rng_sp (W : W3C I IP) (p1 p2 c : I.D)
    (hr : I.iext (I.ι V.range) p2 c) (hsp : I.sp p1 p2) :
    I.iext (I.ι V.range) p1 c :=
  W.rng_bwd p1 c (W.sp_fwd p1 p2 hsp).1 (W.rng_fwd p2 c hr).2.1
    (fun x y hxy => (W.rng_fwd p2 c hr).2.2 x y ((W.sp_fwd p1 p2 hsp).2.2 x y hxy))

/-! ### Group B: the two transitivity arms

RDF 1.1 Semantics section 9 does state "IEXT(I(rdfs:subClassOf)) is transitive
and reflexive on IC" as a condition in its own right, so `Conditions.sc_trans`
and `Conditions.sp_trans` are licensed in a way the other twelve are not. Once
Table 5.8 is present they are redundant, and the honest thing is to derive them
rather than assume something a stronger cell already gives.

The second formalisation keeps both as primitive on purpose
(`isabelle/OO_Semantics.thy`, DECISION M10), for a diagnostic reason that does
not apply here: there, keeping them separate puts the six `rdfs*` arms in a
strictly weaker fragment than the twelve that need Table 5.8's backward
direction, and the consumption map shows which arm needs which. Here `W3C` is a
single structure, no arm sits in a distinguished fragment of it, and there is
nothing to diagnose. -/

/-- **rdfs11.** Transitivity of `rdfs:subClassOf`, from Table 5.8 row 1 forward
twice, composition of the two inclusions, and row 1 backward once. -/
theorem sc_trans (W : W3C I IP) (a b c : I.D) (hab : I.sc a b) (hbc : I.sc b c) : I.sc a c :=
  W.sc_bwd a c (W.sc_fwd a b hab).1 (W.sc_fwd b c hbc).2.1
    (fun x hx => (W.sc_fwd b c hbc).2.2 x ((W.sc_fwd a b hab).2.2 x hx))

/-- **rdfs5.** Transitivity of `rdfs:subPropertyOf`, from Table 5.8 row 2. -/
theorem sp_trans (W : W3C I IP) (a b c : I.D) (hab : I.sp a b) (hbc : I.sp b c) : I.sp a c :=
  W.sp_bwd a c (W.sp_fwd a b hab).1 (W.sp_fwd b c hbc).2.1
    (fun x y hxy => (W.sp_fwd b c hbc).2.2 x y ((W.sp_fwd a b hab).2.2 x y hxy))

/-! ### Group C: the four equivalence arms

`scm-eqc1` and `scm-eqp1` each license two conclusions from one premise, so
these two theorems carry four arms between them. The signatures keep
`Conditions.eqc` and `Conditions.eqp`'s conjunction shape exactly, so
`Soundness.lean`'s `scmEqc1` and `scmEqp1` cases need no edit. -/

/-- **scm-eqc1, both conclusions.** `T(?a, owl:equivalentClass, ?b)` gives
`T(?a, rdfs:subClassOf, ?b)` and `T(?b, rdfs:subClassOf, ?a)`.

Table 5.9 hands back an extension EQUALITY and never a `rdfs:subClassOf`
triple; the triples come out of Table 5.8 row 1 backward, once in each
direction, with the `IC` memberships supplied by Table 5.9's own conjuncts. -/
theorem eqc (W : W3C I IP) (a b : I.D) (h : I.iext (I.ι V.equivalentClass) a b) :
    I.sc a b ∧ I.sc b a :=
  ⟨W.sc_bwd a b (W.eqc_fwd a b h).1 (W.eqc_fwd a b h).2.1
     (fun x hx => ((W.eqc_fwd a b h).2.2 x).mp hx),
   W.sc_bwd b a (W.eqc_fwd a b h).2.1 (W.eqc_fwd a b h).1
     (fun x hx => ((W.eqc_fwd a b h).2.2 x).mpr hx)⟩

/-- **scm-eqp1, both conclusions.** Table 5.9's `owl:equivalentProperty` row
and Table 5.8 row 2 backward. -/
theorem eqp (W : W3C I IP) (a b : I.D) (h : I.iext (I.ι V.equivalentProperty) a b) :
    I.sp a b ∧ I.sp b a :=
  ⟨W.sp_bwd a b (W.eqp_fwd a b h).1 (W.eqp_fwd a b h).2.1
     (fun x y hxy => ((W.eqp_fwd a b h).2.2 x y).mp hxy),
   W.sp_bwd b a (W.eqp_fwd a b h).2.1 (W.eqp_fwd a b h).1
     (fun x y hxy => ((W.eqp_fwd a b h).2.2 x y).mpr hxy)⟩

/-! ### Group D: the four restriction-ordering arms

These are the expensive ones. Each needs Table 5.6's equality in the direction
`Conditions` does not have, plus Tables 5.2 and 5.3's typing rows to supply the
`IC` memberships that `sc_bwd`'s antecedent demands.

`svf_sc` and `svf_sp` consume the `⊆` half of Table 5.6's `someValuesFrom`
equality, which `Conditions.svf` does not carry. `avf_sc` and `avf_sp` consume
the `⊇` half of the `allValuesFrom` equality, which `Conditions.avf` does not
carry. Those two halves are the entire strengthening in this file, and the cell
that licenses them is Table 5.6's consequent being a set equality, quoted above
`svf_eq` and `avf_eq`. -/

/-- **scm-svf1.** Two existential restrictions on the same property, ordered by
their fillers: a wider filler makes a wider class.

`ICEXT(c1) ⊆ ICEXT(c2)` by unfolding Table 5.6 on `c1`, widening the found
witness's filler membership through Table 5.8 row 1 forward, and folding Table
5.6 back up on `c2`. Then row 1 backward turns the inclusion into the triple,
with `IC c1` and `IC c2` from Table 5.3's `someValuesFrom` row into Table 5.2's
`owl:Restriction` row. -/
theorem svf_sc (W : W3C I IP) (c1 c2 p y1 y2 : I.D)
    (h1 : I.iext (I.ι V.someValuesFrom) c1 y1) (h2 : I.iext (I.ι V.onProperty) c1 p)
    (h3 : I.iext (I.ι V.someValuesFrom) c2 y2) (h4 : I.iext (I.ι V.onProperty) c2 p)
    (h5 : I.sc y1 y2) : I.sc c1 c2 :=
  W.sc_bwd c1 c2 (W.restr_IC c1 (W.svf_typ c1 y1 h1).1) (W.restr_IC c2 (W.svf_typ c2 y2 h3).1)
    (fun x hx =>
      (W.svf_eq c2 y2 p h3 h4 x).mpr
        (match (W.svf_eq c1 y1 p h1 h2 x).mp hx with
         | ⟨y, hxy, hy⟩ => ⟨y, hxy, (W.sc_fwd y1 y2 h5).2.2 y hy⟩))

/-- **scm-svf2.** Two existential restrictions with the same filler, ordered by
their properties. `svf_sc` with the witness moved across by Table 5.8 row 2
forward instead of the filler being widened by row 1. -/
theorem svf_sp (W : W3C I IP) (c1 c2 p1 p2 y : I.D)
    (h1 : I.iext (I.ι V.someValuesFrom) c1 y) (h2 : I.iext (I.ι V.onProperty) c1 p1)
    (h3 : I.iext (I.ι V.someValuesFrom) c2 y) (h4 : I.iext (I.ι V.onProperty) c2 p2)
    (h5 : I.sp p1 p2) : I.sc c1 c2 :=
  W.sc_bwd c1 c2 (W.restr_IC c1 (W.svf_typ c1 y h1).1) (W.restr_IC c2 (W.svf_typ c2 y h3).1)
    (fun x hx =>
      (W.svf_eq c2 y p2 h3 h4 x).mpr
        (match (W.svf_eq c1 y p1 h1 h2 x).mp hx with
         | ⟨v, hxv, hv⟩ => ⟨v, (W.sp_fwd p1 p2 h5).2.2 x v hxv, hv⟩))

/-- **scm-avf1.** Two universal restrictions on the same property, ordered by
their fillers, and this one runs the SAME way round as the three above.

Widening `ICEXT(c)` weakens the consequent of Table 5.6's implication, so it
widens `ICEXT(z)`. A universal restriction is MONOTONE in its filler and
antitone only in its property, which is why this rule and `avf_sp` run in
opposite directions. -/
theorem avf_sc (W : W3C I IP) (c1 c2 p y1 y2 : I.D)
    (h1 : I.iext (I.ι V.allValuesFrom) c1 y1) (h2 : I.iext (I.ι V.onProperty) c1 p)
    (h3 : I.iext (I.ι V.allValuesFrom) c2 y2) (h4 : I.iext (I.ι V.onProperty) c2 p)
    (h5 : I.sc y1 y2) : I.sc c1 c2 :=
  W.sc_bwd c1 c2 (W.restr_IC c1 (W.avf_typ c1 y1 h1).1) (W.restr_IC c2 (W.avf_typ c2 y2 h3).1)
    (fun x hx =>
      (W.avf_eq c2 y2 p h3 h4 x).mpr
        (fun v hv => (W.sc_fwd y1 y2 h5).2.2 v ((W.avf_eq c1 y1 p h1 h2 x).mp hx v hv)))

/-- **scm-avf2, and the conclusion is REVERSED**: `T(?c2, rdfs:subClassOf, ?c1)`
where the other three restriction-ordering rules conclude
`T(?c1, rdfs:subClassOf, ?c2)`.

**After this derivation the direction is forced by the proof rather than agreed
between two hand-written statements.** `Soundness.lean`'s `scmAvf2` case used to
be defended by a comment saying that `avf_sp` "is stated that way round and the
two must agree, so getting either one backwards fails to compile rather than
shipping a rule no model supports". That is a consistency check between two
things the same author wrote, and it passes if both are wrong. Here `sc_bwd` is
applied to the pair `(c2, c1)`, and it is applied that way round because the
only `avf_eq` whose `.mp` fires on the hypothesis is `c2`'s and the only one
whose `.mpr` closes the goal is `c1`'s. Writing `(c1, c2)` does not merely fail
a sanity check, it fails to typecheck, and it fails against the specification's
antitonicity rather than against a second copy of the claim. -/
theorem avf_sp (W : W3C I IP) (c1 c2 p1 p2 y : I.D)
    (h1 : I.iext (I.ι V.allValuesFrom) c1 y) (h2 : I.iext (I.ι V.onProperty) c1 p1)
    (h3 : I.iext (I.ι V.allValuesFrom) c2 y) (h4 : I.iext (I.ι V.onProperty) c2 p2)
    (h5 : I.sp p1 p2) : I.sc c2 c1 :=
  W.sc_bwd c2 c1 (W.restr_IC c2 (W.avf_typ c2 y h3).1) (W.restr_IC c1 (W.avf_typ c1 y h1).1)
    (fun x hx =>
      (W.avf_eq c1 y p1 h1 h2 x).mpr
        (fun v hv => (W.avf_eq c2 y p2 h3 h4 x).mp hx v ((W.sp_fwd p1 p2 h5).2.2 x v hv)))

end W3C

/-! ## A `W3CModel` of a graph, and the adequacy theorem

NOT a conforming interpretation: see the module docstring for the two gaps, and
`certificate_w3c_sound` for them again. The name is `W3CModel` everywhere and
nothing here abbreviates it to "conforming model". -/

/-- `I` with property set `IP` meets the specification conditions and models
`G`. The three list rows are stated over `Chain`, which reads the list off the
ASSERTED GRAPH.

**This is the one place where the structure is not the specification's**, and
the divergence is recorded rather than argued away. RBS's convention section
defines "`s` sequence of `a₁ , … , aₙ ∈ S`" semantically: "`s` = `I(rdf:nil)`
for `n = 0`; and for `n > 0` there exist `z₁ ∈ IR , … , zₙ ∈ IR`, such that
`s = z₁`, `a₁ ∈ S`, `( z₁ , a₁ ) ∈ IEXT(I(rdf:first))`, `( z₁ , z₂ ) ∈
IEXT(I(rdf:rest))`, … ". It quantifies over `IEXT(I(rdf:first))` and
`IEXT(I(rdf:rest))`, NOT over `G`, and it adds that "there are no semantic
constraints that enforce "well-formed" sequence structures. So, for example, it
is possible for a sequence head `s` to refer to more than one sequence."

`Chain G` fires on strictly fewer lists than the specification's relation does
in any model of `G`, because a model satisfies `G`'s own `rdf:first` and
`rdf:rest` triples and so every graph chain is a semantic sequence, while a
semantic sequence built from pairs the graph never asserts is not a graph
chain. Conditions stated over `Chain` are therefore WEAKER than the
specification's, the model class is larger, and entailment still transfers
outward. What does not follow is the converse: an interpretation meeting these
three fields is not thereby known to meet Tables 5.4 and 5.5. Closing that gap
needs `rdf:first` and `rdf:rest` in the vocabulary and a semantic sequence
relation, which no rule in this checker consumes. -/
structure W3CModel (I : Interp) (IP : I.D → Prop) (G : List Triple) : Prop where
  conds : W3C I IP
  facts : ∀ t ∈ G, I.sat t
  /-- RBS Table 5.4: **if** `s` sequence of `c₁ , … , cₙ ∈ IR` **then**
  `( z , s ) ∈ IEXT(I(owl:intersectionOf))` **iff** `z , c₁ , … , cₙ ∈ IC ,
  ICEXT(z) = ICEXT(c₁) ∩ … ∩ ICEXT(cₙ)`. The equality, both halves; the `IC`
  conjuncts are dropped because no arm consumes them. -/
  int_eq : ∀ c l ms, (⟨c, V.intersectionOf, l⟩ : Triple) ∈ G → Chain G l ms →
    ∀ x, (I.cext (I.ι c) x ↔ ∀ m ∈ ms, I.cext (I.ι m) x)
  /-- RBS Table 5.4: the `owl:unionOf` row, `ICEXT(z) = ICEXT(c₁) ∪ … ∪
  ICEXT(cₙ)`. -/
  uni_eq : ∀ c l ms, (⟨c, V.unionOf, l⟩ : Triple) ∈ G → Chain G l ms →
    ∀ x, (I.cext (I.ι c) x ↔ ∃ m ∈ ms, I.cext (I.ι m) x)
  /-- RBS Table 5.5: **if** `s` sequence of `a₁ , … , aₙ ∈ IR` **then**
  `( z , s ) ∈ IEXT(I(owl:oneOf))` **iff** `z ∈ IC , ICEXT(z) = { a₁ , … , aₙ }`.

  `Model.oneOf` is the `⊇` half of this and nothing else, so a conforming
  interpretation gives it and the `⊆` half is available here but consumed by no
  rule. -/
  oneOf_eq : ∀ c l ms, (⟨c, V.oneOf, l⟩ : Triple) ∈ G → Chain G l ms →
    ∀ x, (I.cext (I.ι c) x ↔ ∃ m ∈ ms, x = I.ι m)

/-- **Every `W3CModel` is a `Model`.**

Not "every conforming interpretation is a model here": this theorem quantifies
over `W3CModel`, which is a Lean structure. That sentence is
`OOCert.Conforming.toModel` in `Conforming.lean`, which composes with this one
and discharges the five axiomatic-triple facts the module docstring above
assumes.

Twelve of the twenty-three `Conditions` fields below are derivations rather than
projections: `sc_trans`, `sp_trans`, `eqc`, `eqp`, `svf_sc`, `svf_sp`,
`avf_sc`, `avf_sp`, `dom_sc`, `dom_sp`, `rng_sc`, `rng_sp`. They are exactly the
fields `Semantics.lean` posits, and they carry fourteen of the soundness proof's
arms, because `Soundness.lean` consumes `eqc` and `eqp` twice each. The other
eleven fields and the four list fields are projections out of the corresponding
quoted table cell, which is what they were meant to be all along. -/
theorem W3CModel.toModel {I : Interp} {IP : I.D → Prop} {G : List Triple}
    (W : W3CModel I IP G) : Model I G where
  conds :=
    { sc_sub := fun a b h x hx => (W.conds.sc_fwd a b h).2.2 x hx
      sc_trans := W.conds.sc_trans
      sp_sub := fun a b h x y hxy => (W.conds.sp_fwd a b h).2.2 x y hxy
      sp_trans := W.conds.sp_trans
      dom := fun p c h x y hxy => (W.conds.dom_fwd p c h).2.2 x y hxy
      rng := fun p c h x y hxy => (W.conds.rng_fwd p c h).2.2 x y hxy
      trp := W.conds.trp_fwd
      symp := W.conds.sym_fwd
      inv := fun p q h x y => (W.conds.inv_fwd p q h).2.2 x y
      same := W.conds.same_fwd
      eqc := W.conds.eqc
      eqp := W.conds.eqp
      svf := fun r p c hop hsv x y hxy hy =>
        (W.conds.svf_eq r c p hsv hop x).mpr ⟨y, hxy, hy⟩
      avf := fun r p c hop hav x y hx hxy =>
        (W.conds.avf_eq r c p hav hop x).mp hx y hxy
      hv := fun r p v hop hhv x => W.conds.hv_eq r v p hhv hop x
      svf_sc := W.conds.svf_sc
      svf_sp := W.conds.svf_sp
      avf_sc := W.conds.avf_sc
      avf_sp := W.conds.avf_sp
      dom_sc := W.conds.dom_sc
      dom_sp := W.conds.dom_sp
      rng_sc := W.conds.rng_sc
      rng_sp := W.conds.rng_sp }
  facts := W.facts
  int := fun c l ms hin hch x hall => (W.int_eq c l ms hin hch x).mpr hall
  int2 := fun c l ms hin hch x hx m hm => (W.int_eq c l ms hin hch x).mp hx m hm
  uni := fun c l ms hin hch x m hm hx => (W.uni_eq c l ms hin hch x).mpr ⟨m, hm, hx⟩
  oneOf := fun c l ms hin hch m hm => (W.oneOf_eq c l ms hin hch (I.ι m)).mpr ⟨m, hm, rfl⟩

/-- Entailment over the conforming interpretations. Stated separately from
`Entails` on purpose: this is a SMALLER class of interpretations, so this
relation is LARGER, and a non-entailment result about it is a strictly stronger
claim than a non-entailment result about `Entails`. Nothing in this file proves
one. -/
def W3CEntails (G : List Triple) (t : Triple) : Prop :=
  ∀ (I : Interp) (IP : I.D → Prop), W3CModel I IP G → I.sat t

/-- **The sentence the layer can put its name to**, and it is about `W3CModel`
and not about a conforming interpretation. Everything `Entails` gives is true in
every `W3CModel` of the graph, and the fourteen conditions that used to be
posited are steps inside the proof of it. Reading that outward, to the
specification's own interpretations, is `OOCert.ConformingEntails.of_entails` in
`Conforming.lean`, which used to be the prose bridge and the five
axiomatic-triple facts in the module docstring above.

The transfer runs in this direction and only this one. `Conditions` admits more
interpretations than `W3C` does, so a claim true in all of them is true in all
of the smaller class. The converse fails: a `¬ Entails` result does NOT give
`¬ W3CEntails`, because the model refuting the first need not be a `W3CModel`.

What that does NOT mean is that every non-entailment here is stuck. It says only
that each one needs its own proof, and every one of them in this repository now
has one. Four came for free, because the interpretations they already used ARE
`W3CModel`s once `IP` is instantiated to the empty predicate. `IP` is a free
parameter of `W3C` and `sp_bwd`, `dom_bwd` and `rng_bwd` are the only fields that
take an `IP` membership as a hypothesis, so `IP := fun _ => False` makes all
three vacuous; a witness graph with no `rdfs:Class` typing makes `sc_bwd` vacuous
too, because `I.IC` is then empty. `W3CWitness.lean` and `Mixed.lean` carry those
four. The rest needed a hand-built finite structure each, and `W3CWitness.lean`'s
`svfI` and `RefuteWitness.lean`'s `refI` are those structures. -/
theorem W3CEntails.of_entails {G : List Triple} {t : Triple} (h : Entails G t) :
    W3CEntails G t :=
  fun _ _ W => h _ W.toModel

/-- The checker's verdict, restated over the specification's conditions. Same
certificates, same checker, same `checkCert`; what changed is that "entailed"
now quantifies over every interpretation meeting the quoted table cells, rather
than over the weaker set of conditions the Lean posited, and that the fourteen
arms which used to be posited are steps inside the proof.

**This is not the sentence "true in every conforming interpretation", and that
sentence is now `OOCert.certificate_conforming_sound` in `Conforming.lean`.**
`W3CModel` is a Lean structure. Until 15 September 2026 nothing in this
repository quantified over the specification's own interpretations, so the step
between the two was the prose bridge in the module docstring above, plus five
assumed facts; `Conforming.lean` builds `OOCert.Interpretation`, proves the
bridge as `OOCert.Conforming.toW3C`, and carries the five facts as quoted
axiomatic triples. This theorem is left exactly as it was, because it is what
`W3CWitness.lean` and `Mixed.lean` consume and because a weaker statement that
still holds is not a defect.

`W3CModel` remains strictly weaker than conformance, because `W3C` deliberately
omits every table row no rule consumes, so its class is larger than the image of
the conforming interpretations under `toW3C`. That gap runs in the safe direction
for THIS theorem, since a larger class makes the conclusion stronger; it does not
run in the safe direction for a non-entailment result, and nothing about
`Conforming.lean` changes that. -/
theorem certificate_w3c_sound (G : List Triple) (steps : List Step)
    (h : checkCert G steps = true) :
    ∀ st ∈ steps, W3CEntails G st.conclusion :=
  fun st hst => W3CEntails.of_entails (certificate_sound G steps h st hst)

/-! ## The axiom tripwire

The same guard `Soundness.lean` carries, on the new theorems. A `sorry` or a
`native_decide` anywhere in the fourteen derivations fails the build here.
`native_decide` would add `Lean.ofReduceBool`, which is why it is banned rather
than merely discouraged. -/

/-- info: 'OOCert.W3CModel.toModel' does not depend on any axioms -/
#guard_msgs in
#print axioms W3CModel.toModel

/-- info: 'OOCert.certificate_w3c_sound' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms certificate_w3c_sound

end OOCert
