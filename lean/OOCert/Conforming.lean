import OOCert.W3C

/-!
# An RDF-Based Semantics interpretation, and the bridge as a theorem

`W3C.lean` derives fourteen rule arms over `W3C I IP`, which is a structure
*this repository* wrote. The sentence a reader wants is about the
specification's own interpretations, and until now the step between the two was
a paragraph: it said how to read a conforming interpretation as an `Interp`, and
it assumed five `IP` memberships off the RDF and RDFS axiomatic-triple tables
without quoting a cell of either.

This file removes both. An OWL 2 RDF-Based interpretation is formalised here as
a Lean structure in its own right, `Interpretation`, with the parts of RBS Table
5.1 and RDF 1.1 Semantics section 5's truth clause, `I(p) ∈ IP` conjunct
included. `Conforming I` collects semantic conditions, each one a quoted cell.
`Conforming.toW3C` then *proves* that every such interpretation yields an
`Interp` and a `W3C` instance, and the five facts that used to be assumed are
discharged inside that proof from five axiomatic triples that are fields of
`Conforming`.

The sentence at the end, `certificate_conforming_sound`, mentions no structure
of this repository's invention. It reads: if `checkCert` accepts, then every
step's conclusion is true, in the sense of RDF 1.1 Semantics section 5, in every
interpretation satisfying the quoted conditions that also satisfies the asserted
graph.

## What is still not a theorem, stated before anything else

`Conforming` carries a SUBSET of the specification's conditions. It has to: the
Recommendation has fifteen condition tables, thirty-one rows in Table 5.2 and
fifty-five in Table 5.3, a datatype map, facets, and the parts `LV`, `IX`,
`IDC`, `IODP`, `IOXP` and `IOAP` of Table 5.1, and none of the twenty-nine rules
consumes any of them. Rows were counted off the raw HTML on 15 September 2026.

A subset is the SAFE direction and that is why it is allowed. Every condition
below is one the Recommendation imposes, so every conforming interpretation
satisfies all of them, the class of structures this file quantifies over
CONTAINS the conforming interpretations, and a sentence true throughout the
larger class is true of every member of the smaller one. What remains prose is
therefore a CONTAINMENT, checkable cell by cell against the Recommendation, and
no longer a translation plus five unproved facts. It is a smaller thing to check
and a different kind of thing to check, and the difference is the whole point of
this file.

That containment has ONE exception, and it is stated here rather than left to be
found: `IL` is total. RDF 1.1 section 5 makes it partial, so an interpretation
in which a literal fails to denote is NOT one of these, and the sentence at the
end says nothing about it. The next section but one argues the point and names
the obligation that would close it.

Three further consequences a reader should not have to infer:

1. **Nothing here licenses a NEGATIVE result about conforming interpretations.**
   A structure satisfying these conditions need not be a conforming
   interpretation, so `¬ ConformingEntails G t` does not give
   "`G` does not OWL 2 RDF-Based entail `t`". `not_everything_is_conforming_entailed`
   in `ConformingWitness.lean` is a statement about this class and says so.
2. **Three modelling decisions are recorded at their fields**, not folded into
   the word "conforming": rigid blank nodes, a total `IL`, and `IEXT` total on
   the carrier. Each is argued below with its direction of safety, and the
   second of the three does not run in the safe direction.
3. **One reading of a table cell is load-bearing and is flagged**: Table 5.4 at
   `n = 0`. See `int_fwd`.

## Sources, fetched from the raw HTML on 15 September 2026

* RBS = OWL 2 Web Ontology Language RDF-Based Semantics (Second Edition), W3C
  Recommendation 11 December 2012, <https://www.w3.org/TR/owl2-rdf-based-semantics/>.
* RDF11 = RDF 1.1 Semantics, W3C Recommendation 25 February 2014,
  <https://www.w3.org/TR/rdf11-mt/>.

Both were read out of the raw HTML, with markup intact, rather than out of a
rendering. That matters at least twice: a markdown conversion of RBS loses
Table 5.8's `rowspan="4"` and renders its `iff` as `if`, and it loses the
`rowspan` that makes Table 5.9's `iff` govern the `owl:equivalentClass` and
`owl:equivalentProperty` rows as well as the `owl:sameAs` row.

## The chain that licenses the axiomatic triples, quoted

The five `IP` facts used to be assumed because nothing in this repository
quoted a cell of an axiomatic-triple table. The licence is three sentences long
and each link is verbatim.

RBS Definition 4.2: "An OWL 2 RDF-Based interpretation, `I = ( IR , IP , IEXT ,
IS , IL , LV )`, of V with respect to D is a D-interpretation of V with respect
to D that meets all the extra semantic conditions given in Section 5."

RBS Section 4.2: "As detailed in the RDF Semantics [RDF Semantics], a
D-interpretation has to meet all the semantic conditions for ground graphs and
blank nodes, those for RDF interpretations and RDFS interpretations, and the
'general semantic conditions for datatypes'."

RDF11 Section 9: "An RDFS interpretation (recognizing D) is an RDF
interpretation (recognizing D) I which satisfies the semantic conditions in the
following table, and all the triples in the subsequent table of RDFS axiomatic
triples."

So an OWL 2 RDF-Based interpretation satisfies every RDF axiom of RDF11 section
8 and every RDFS axiomatic triple of RDF11 section 9. Five of them are fields of
`Conforming` below, and by section 5's truth clause each one puts its own
predicate's denotation into `IP`. Only five are taken, because importing the
whole tables would be a strengthening nothing needs; RBS's own section 6, which
collects axiomatic triples, is marked "(Informative)", and OWL 2 Profiles
section 4.3 declines to include them, so the minimum is what is taken.

**One of the five is taken by a different route from the one the previous prose
claimed, because that claim was wrong.** `W3C.lean`'s table gave
`I(rdfs:range) ∈ IP` the source "RDFS axiomatic triple
`rdfs:range rdfs:domain rdf:Property .`, same", where "same" pointed at the row
above it reading "whose truth puts its own predicate in `IP`". The predicate of
that triple is `rdfs:domain`, so its truth puts `I(rdfs:domain)` into `IP` and
not `I(rdfs:range)`. Reaching `rdfs:range` from it needs a further step through
Table 5.8's `rdfs:domain` row. The field below uses
`rdf:type rdfs:range rdfs:Class .` instead, whose predicate IS `rdfs:range`, so
one application of the truth clause suffices and no table is consulted at all.
The same simplification is taken for `rdfs:subClassOf` and `rdfs:subPropertyOf`,
where the old prose also went through Table 5.8.

## Three modelling decisions, each with its direction of safety

**Blank nodes are rigid.** `IB` is a component of the interpretation and
`ConformingEntails` quantifies over it along with everything else, where RDF11
section 5.1 quantifies over an assignment `A` existentially inside satisfaction.
Write `⊨₂` for the relation defined here and `⊨₁` for the specification's. Then
`⊨₂` implies `⊨₁`: given `I` and a witness `A` with `[I+A]` satisfying `G`, the
pair `(I, A)` is one of the structures `⊨₂` quantifies over, so it satisfies
`t`, and `A` itself witnesses the consequent of `⊨₁`. The converse fails, so
`⊨₂` is the STRICTLY SMALLER relation and a soundness theorem proved for it is
the STRONGER one. This is the same decision, and the same argument, as
`isabelle/OO_Semantics.thy`'s DECISION M4.

**`IL` is total.** RBS Section 4.2: "IL is a mapping from typed literals
`"s"^^u` in V to their denotations in IR, where `IL("s"^^u) = L2V(d)(s)`,
provided that d is a datatype of D, `IS(u) = d`, and s is in the lexical space
`LS(d)`; otherwise `IL("s"^^u)` is not in LV." An ill-typed literal denotes
something in `IR` outside `LV`; it does not fail to denote. RDF11 section 5
instead lists "A partial mapping IL from literals into IR", and its own note
records the change: "In the 2004 RDF 1.0 semantics, IL was a total, rather than
partial, mapping." RBS is the document that defines the interpretations this
project's claim is about, and its normative reference [RDF Semantics] is the
2004 Recommendation, so total is what is written here.

**This one does NOT run in the safe direction and the cost is stated rather than
argued away.** Requiring `IL` total EXCLUDES any RDF 1.1 interpretation in which
some literal fails to denote, so `certificate_conforming_sound` says nothing
about those. The gap is closable and is not closed here: every term of a
checked step's conclusion occurs in the asserted graph, so in any interpretation
satisfying the graph every such term denotes, and the partial case reduces to
the total one. That argument needs a term-occurrence lemma about `checkStep`,
twenty-nine cases, which is not in this repository. Until it is, the residue is
one sentence long and it is written down here. Taking `IL` partial instead is
not an option: the domain would then have to be `Option IR`, and `Model.int`
quantifies over the whole domain, so an empty `owl:intersectionOf` list would
demand that the extra element be in a class extension, which it cannot be.

**`IEXT` is total on the carrier.** RDF11 section 5 item 3 gives "A mapping IEXT
from IP into the powerset of IR x IR", so the specification's `IEXT` is defined
only on `IP`. Here it is a relation on the whole carrier and every condition
relativises to `IP` exactly where the specification's quantifier does, and
nowhere else. No condition below constrains `IEXT(x)` for an `x` that the same
condition does not first place in `IP`, so sending every non-property to the
empty extension turns any specification interpretation into one of these. Same
decision as `OO_Semantics.thy`'s DECISION M6.

## What the carrier being `IR` settles for free

Table 5.1 gives the parts of the universe, and for `IP` and `IC` its second
column reads `S ⊆ IR`, with the third column giving `ICEXT(x) ⊆ IR` for classes
and `IEXT(x) ⊆ IR × IR` for properties. All four are conditions here by typing:
`IP` and `IC` are predicates on the carrier and `IEXT` is a relation on it, so
nothing can leave `IR`. The first row, `IR | S ≠ ∅`, is the one that needs
saying, and it is the field `IR_nonempty`.

`ICEXT` and `IC` are DEFINITIONS and not conditions, which is what RDF11 section
9 makes them: "ICEXT(y) is defined to be `{ x : < x,y > is in IEXT(I(rdf:type)) }`"
and "IC is defined to be ICEXT(I(rdfs:Class))". A definition cannot shrink the
model class; a condition can.

`LV`, `IX`, `IDC`, `IODP`, `IOXP` and `IOAP` are not modelled. Every condition
mentioning them would be an extra condition, so leaving them out enlarges the
class and is safe.
-/
namespace OOCert

namespace V

/-- `rdf:Property`. Subject of nothing here; object of the RDF axiom
`rdf:type rdf:type rdf:Property .`. -/
def Property : Term := "<http://www.w3.org/1999/02/22-rdf-syntax-ns#Property>"

/-- `rdfs:Resource`. Object of the RDFS axiomatic triple
`rdf:type rdfs:domain rdfs:Resource .`. -/
def Resource : Term := "<http://www.w3.org/2000/01/rdf-schema#Resource>"

/-- `rdfs:Datatype`. Subject of the RDFS axiomatic triple
`rdfs:Datatype rdfs:subClassOf rdfs:Class .`. -/
def Datatype : Term := "<http://www.w3.org/2000/01/rdf-schema#Datatype>"

/-- `rdfs:isDefinedBy`. Subject of the RDFS axiomatic triple
`rdfs:isDefinedBy rdfs:subPropertyOf rdfs:seeAlso .`. -/
def isDefinedBy : Term := "<http://www.w3.org/2000/01/rdf-schema#isDefinedBy>"

/-- `rdfs:seeAlso`. Object of the same triple. -/
def seeAlso : Term := "<http://www.w3.org/2000/01/rdf-schema#seeAlso>"

end V

/-! ## Which of the three mappings a term uses -/

/-- The three sorts of RDF term that RDF11 section 5 gives separate treatment:
IRIs go through `IS`, literals through `IL`, and blank nodes through section
5.1's assignment, here the rigid `IB`. -/
inductive TermKind
  | iri | bnode | lit
  deriving DecidableEq, Repr

/-- The sort of a term, read off its N-Triples spelling, which is how `Term` is
written (see `Triple.lean`). `<…>` is an IRI, `_:…` is a blank node, anything
else is a literal.

The match is on `String.toList` rather than on `String.startsWith` because the
latter does not reduce in the kernel, so `termKind V.type = .iri` would not be
`rfl` and every vocabulary lemma below would need a tactic. -/
def termKind (t : Term) : TermKind :=
  match t.toList with
  | '<' :: _ => .iri
  | '_' :: ':' :: _ => .bnode
  | _ => .lit

/-! ## The interpretation -/

/-- An OWL 2 RDF-Based interpretation's components.

RBS Definition 4.2 writes the tuple `I = ( IR , IP , IEXT , IS , IL , LV )`, and
RDF11 section 5 gives the same list for a simple interpretation: "1. A non-empty
set IR of resources, called the domain or universe of I. 2. A set IP, called the
set of properties of I. 3. A mapping IEXT from IP into the powerset of IR x IR
… 4. A mapping IS from IRIs into (IR union IP) 5. A partial mapping IL from
literals into IR".

`IR` is the carrier TYPE, which is what makes Table 5.1's `IP ⊆ IR`, `IC ⊆ IR`,
`ICEXT(x) ⊆ IR` and `IEXT(x) ⊆ IR × IR` hold by typing. `LV` is absent because
nothing here consumes it. `IB` is the assignment of RDF11 section 5.1, carried
as a component; see the module docstring for the direction of safety.

RDF11 gives `IS` the codomain `IR ∪ IP`, which is wider than `IR`; RBS Table
5.1 gives `IP` the row `S ⊆ IR`, so under the OWL 2 RDF-Based Semantics the two
codomains coincide and `IR` is the faithful one. -/
structure Interpretation where
  /-- `IR`, the universe. Its non-emptiness is the field `Conforming.IR_nonempty`,
  because a structure field cannot state a condition on its own type. -/
  IR : Type
  /-- `IP`, the properties of `I`. -/
  IP : IR → Prop
  /-- `IEXT`. `IEXT p x y` reads "`( x , y ) ∈ IEXT(p)`". Total on the carrier;
  see the module docstring. -/
  IEXT : IR → IR → IR → Prop
  /-- `IS`, the denotation of IRIs. -/
  IS : Term → IR
  /-- The blank node assignment of RDF11 section 5.1, rigid here. -/
  IB : Term → IR
  /-- `IL`, the denotation of literals. Total; see the module docstring. -/
  IL : Term → IR

namespace Interpretation

variable (I : Interpretation)

/-- `I(E)` for a term `E`, which RBS's convention section writes as one mapping:
"for a given interpretation I of a vocabulary V the notation `I(x)` will be used
instead of `IL(x)` and `IS(x)` for the typed literals and IRIs x in V". -/
def den (t : Term) : I.IR :=
  match termKind t with
  | .iri => I.IS t
  | .bnode => I.IB t
  | .lit => I.IL t

/-- `ICEXT`. RDF11 section 9, verbatim: "ICEXT(y) is defined to be
`{ x : < x,y > is in IEXT(I(rdf:type)) }`". A DEFINITION, per that wording, and
per RBS section 4.5 which repeats it. -/
def ICEXT (y x : I.IR) : Prop := I.IEXT (I.IS V.type) x y

/-- `IC`. RDF11 section 9, verbatim: "IC is defined to be ICEXT(I(rdfs:Class))".
RBS Table 5.2's `rdfs:Class` row agrees, reading `rdfs:Class | ∈ IC | = IC`
where the third column is headed `ICEXT(I(E))`. -/
def IC (c : I.IR) : Prop := I.ICEXT (I.IS V.Class) c

/-- Truth of a ground triple. RDF11 section 5, "Semantic conditions for ground
graphs", verbatim:

> if E is a ground triple s p o.
> then I(E) = true if I(p) is in IP and the pair <I(s),I(o)> is in IEXT(I(p))
> otherwise I(E) = false.

The `I(p) ∈ IP` conjunct is the clause's own first conjunct and not an addition.
Section 5 item 3 gives the reason it has to be there: "A mapping IEXT from IP
into the powerset of IR x IR", so `IEXT` is not defined off `IP` at all, and
without the conjunct a triple could be true whose predicate denotes a
non-property.

There is deliberately no case distinction on the predicate. RDF11 section 5
gives ONE truth condition for all triples and the vocabulary's meaning enters
only through conditions on `IEXT`; a formalisation that gives `rdfs:subClassOf`
its own truth clause is not an RDF semantics. -/
def Sat (t : Triple) : Prop :=
  I.IP (I.den t.p) ∧ I.IEXT (I.den t.p) (I.den t.s) (I.den t.o)

/-- `s` is a sequence of `as`. RBS's convention section, "Sequence
expressions", verbatim:

> An expression of the form "s sequence of a1 , … , an ∈ S" means that "s"
> represents an RDF list of n ≥ 0 individuals a1 , … , an, all of them being
> members of the set S. Precisely, s = I(rdf:nil) for n = 0; and for n > 0 there
> exist z1 ∈ IR , … , zn ∈ IR, such that s = z1 , a1 ∈ S ,
> ( z1 , a1 ) ∈ IEXT(I(rdf:first)) , ( z1 , z2 ) ∈ IEXT(I(rdf:rest)) , … ,
> an ∈ S, ( zn , an ) ∈ IEXT(I(rdf:first)) ,
> ( zn , I(rdf:nil) ) ∈ IEXT(I(rdf:rest)) .

The membership `∈ S` is dropped because every use below takes `S = IR`, which
the carrier gives for free. The quantifier is over `IEXT(I(rdf:first))` and
`IEXT(I(rdf:rest))` and never over a graph, which is the point this relation
exists to get right: `Semantics.lean`'s `Chain` reads the list off the ASSERTED
GRAPH and is therefore a strictly finer relation in any model of that graph.
`Conforming.seq_of_chain` is that comparison, proved.

The specification adds, and it is why this is a relation and not a function:
"there are no semantic constraints that enforce "well-formed" sequence
structures. So, for example, it is possible for a sequence head s to refer to
more than one sequence." -/
inductive Seq : I.IR → List I.IR → Prop
  | nil : Seq (I.IS V.nil) []
  | cons {z a z' as} :
      I.IEXT (I.IS V.first) z a → I.IEXT (I.IS V.rest) z z' →
      Seq z' as → Seq z (a :: as)

/-- The `Interp` of `Semantics.lean` that this interpretation is.

`D := IR`, `ι := I(·)`, and `iext p x y := IP p ∧ ( x , y ) ∈ IEXT(p)`. The `IP`
conjunct is RDF11 section 5's truth clause's own first conjunct, which is what
makes `Interp.sat` and `Interpretation.Sat` the SAME PROPOSITION rather than two
that have to be argued equal: `sat_eq` below is `Iff.rfl`. -/
def toInterp : Interp where
  D := I.IR
  ι := I.den
  iext := fun p x y => I.IP p ∧ I.IEXT p x y

/-- The bridge preserves truth, in both directions, by definition. This is the
statement the old prose bridge had to be read for, and it is now `rfl`. -/
theorem sat_eq (t : Triple) : I.toInterp.sat t ↔ I.Sat t := Iff.rfl

end Interpretation

/-! ## The semantic conditions, one field per quoted cell

Every field carries the cell it comes from. A field with no quote above it is a
field nobody checked. Each is the WEAKEST the specification licenses: where the
Recommendation gives an `iff` and only one direction is consumed, only that
direction is written, because the converse would exclude conforming
interpretations and no proof would fail. -/

/-- The semantic conditions of an OWL 2 RDF-Based interpretation that the
twenty-nine rules of this checker reach, together with the five axiomatic
triples the bridge needs. A SUBSET of the Recommendation's conditions; see the
module docstring for why the subset runs in the safe direction. -/
structure Conforming (I : Interpretation) : Prop where
  /-- RBS Table 5.1, row `IR`: `IR | S ≠ ∅`. The only row of that table that is
  not already carried by the carrier being a type. -/
  IR_nonempty : Nonempty I.IR

  /- ### The axiomatic triples, five of them

  RDF11 section 8's RDF axioms and section 9's RDFS axiomatic triples are
  satisfied by every OWL 2 RDF-Based interpretation, through the chain quoted in
  the module docstring. Five are taken, each chosen so that its OWN PREDICATE is
  the term whose `IP` membership is wanted: the truth clause then discharges the
  fact in one step, with no table consulted. -/

  /-- RDF11 section 8, "RDF axioms", first line: `rdf:type rdf:type rdf:Property .`

  Its predicate is `rdf:type`, so its truth gives `I(rdf:type) ∈ IP`, which is
  what every condition mentioning `ICEXT` needs under the bridge. -/
  ax_type : I.Sat ⟨V.type, V.type, V.Property⟩
  /-- RDF11 section 9, "RDFS axiomatic triples":
  `rdfs:Datatype rdfs:subClassOf rdfs:Class .`

  Its predicate is `rdfs:subClassOf`, so its truth gives
  `I(rdfs:subClassOf) ∈ IP`, which `sc_bwd` needs under the bridge in order to
  produce a `rdfs:subClassOf` triple at all. -/
  ax_subClassOf : I.Sat ⟨V.Datatype, V.subClassOf, V.Class⟩
  /-- RDF11 section 9, "RDFS axiomatic triples":
  `rdfs:isDefinedBy rdfs:subPropertyOf rdfs:seeAlso .`
  Predicate `rdfs:subPropertyOf`; needed by `sp_bwd`. -/
  ax_subPropertyOf : I.Sat ⟨V.isDefinedBy, V.subPropertyOf, V.seeAlso⟩
  /-- RDF11 section 9, "RDFS axiomatic triples":
  `rdf:type rdfs:domain rdfs:Resource .`
  Predicate `rdfs:domain`; needed by `dom_bwd`. -/
  ax_domain : I.Sat ⟨V.type, V.domain, V.Resource⟩
  /-- RDF11 section 9, "RDFS axiomatic triples":
  `rdf:type rdfs:range rdfs:Class .`
  Predicate `rdfs:range`; needed by `rng_bwd`. This is the one the previous
  prose sourced to a triple whose predicate is `rdfs:domain`. -/
  ax_range : I.Sat ⟨V.type, V.range, V.Class⟩

  /- ### RBS Table 5.8, the RDFS vocabulary

  The `iff` cell of this table carries `rowspan="4"` in the source and governs
  all four rows. That is the single load-bearing markup fact in this file: a
  rendering that flattens it to `if` makes twelve of the fourteen arms
  underivable, and the mistake is invisible because every proof still compiles
  over the weaker `Conditions`. The section prose agrees: "The semantic
  conditions provided here are "iff" conditions, while the original semantic
  conditions, as specified in Section 4.1 of the RDF Semantics, are weaker
  "if-then" conditions." -/

  /-- Table 5.8, row 1, forward.

  > `( c1 , c2 ) ∈ IEXT(I(rdfs:subClassOf))` **iff**
  > `c1 , c2 ∈ IC , ICEXT(c1) ⊆ ICEXT(c2)` -/
  sc_fwd : ∀ a b, I.IEXT (I.IS V.subClassOf) a b →
    I.IC a ∧ I.IC b ∧ ∀ x, I.ICEXT a x → I.ICEXT b x
  /-- Table 5.8, row 1, backward. Same cell.

  The `IC` conjuncts are KEPT. Dropping them shortens every derivation below and
  fails no proof, and it would force a `rdfs:subClassOf` triple between any two
  things with nested extensions, including things that are not classes. -/
  sc_bwd : ∀ a b, I.IC a → I.IC b → (∀ x, I.ICEXT a x → I.ICEXT b x) →
    I.IEXT (I.IS V.subClassOf) a b
  /-- Table 5.8, row 2, forward.

  > `( p1 , p2 ) ∈ IEXT(I(rdfs:subPropertyOf))` **iff**
  > `p1 , p2 ∈ IP , IEXT(p1) ⊆ IEXT(p2)` -/
  sp_fwd : ∀ a b, I.IEXT (I.IS V.subPropertyOf) a b →
    I.IP a ∧ I.IP b ∧ ∀ x y, I.IEXT a x y → I.IEXT b x y
  /-- Table 5.8, row 2, backward. Same cell; `IP` conjuncts kept. -/
  sp_bwd : ∀ a b, I.IP a → I.IP b → (∀ x y, I.IEXT a x y → I.IEXT b x y) →
    I.IEXT (I.IS V.subPropertyOf) a b
  /-- Table 5.8, row 3, forward.

  > `( p , c ) ∈ IEXT(I(rdfs:domain))` **iff**
  > `p ∈ IP , c ∈ IC , ∀ x , y : ( x , y ) ∈ IEXT(p) implies x ∈ ICEXT(c)`

  RDF11 section 9 gives `rdfs:domain` only the if-then direction, "If < x,y > is
  in IEXT(I(rdfs:domain)) and < u,v > is in IEXT(x) then u is in ICEXT(y)", and
  `scm-dom1` does NOT follow from it. This row sitting inside Table 5.8's
  `rowspan="4"` is the whole reason the twelve arms are derivable. -/
  dom_fwd : ∀ p c, I.IEXT (I.IS V.domain) p c →
    I.IP p ∧ I.IC c ∧ ∀ x y, I.IEXT p x y → I.ICEXT c x
  /-- Table 5.8, row 3, backward. Same cell; conjuncts kept. -/
  dom_bwd : ∀ p c, I.IP p → I.IC c → (∀ x y, I.IEXT p x y → I.ICEXT c x) →
    I.IEXT (I.IS V.domain) p c
  /-- Table 5.8, row 4, forward.

  > `( p , c ) ∈ IEXT(I(rdfs:range))` **iff**
  > `p ∈ IP , c ∈ IC , ∀ x , y : ( x , y ) ∈ IEXT(p) implies y ∈ ICEXT(c)`

  One letter apart from row 3, `y` for `x`. -/
  rng_fwd : ∀ p c, I.IEXT (I.IS V.range) p c →
    I.IP p ∧ I.IC c ∧ ∀ x y, I.IEXT p x y → I.ICEXT c y
  /-- Table 5.8, row 4, backward. Same cell; conjuncts kept. -/
  rng_bwd : ∀ p c, I.IP p → I.IC c → (∀ x y, I.IEXT p x y → I.ICEXT c y) →
    I.IEXT (I.IS V.range) p c

  /- ### RBS Table 5.9, equivalence

  The `iff` cell here carries `rowspan="6"`. Only the forward direction of three
  of those six rows is written: the converses are consumed by nothing, and the
  converse of the `owl:sameAs` row in particular would force `owl:sameAs` to
  hold reflexively across the entire universe. -/

  /-- Table 5.9, row 1, forward only.

  > `( a1 , a2 ) ∈ IEXT(I(owl:sameAs))` **iff** `a1 = a2`

  Cited to Table 5.9 and not to Table 5.10, which is the N-ary disjointness
  table; an earlier document in this repository got that wrong. -/
  same_fwd : ∀ a b, I.IEXT (I.IS V.sameAs) a b → a = b
  /-- Table 5.9, row 3, forward only.

  > `( c1 , c2 ) ∈ IEXT(I(owl:equivalentClass))` **iff**
  > `c1 , c2 ∈ IC , ICEXT(c1) = ICEXT(c2)`

  Note what the cell does NOT contain: a `rdfs:subClassOf` triple. `scm-eqc1`
  concludes one, which is why that rule is a derivation from this row plus
  Table 5.8 backward, and not a reading of any single cell. -/
  eqc_fwd : ∀ a b, I.IEXT (I.IS V.equivalentClass) a b →
    I.IC a ∧ I.IC b ∧ ∀ x, (I.ICEXT a x ↔ I.ICEXT b x)
  /-- Table 5.9, row 5, forward only.

  > `( p1 , p2 ) ∈ IEXT(I(owl:equivalentProperty))` **iff**
  > `p1 , p2 ∈ IP , IEXT(p1) = IEXT(p2)` -/
  eqp_fwd : ∀ a b, I.IEXT (I.IS V.equivalentProperty) a b →
    I.IP a ∧ I.IP b ∧ ∀ x y, (I.IEXT a x y ↔ I.IEXT b x y)

  /-- RBS Table 5.12, forward only.

  > `( p1 , p2 ) ∈ IEXT(I(owl:inverseOf))` **iff**
  > `p1 , p2 ∈ IP , IEXT(p1) = { ( x , y ) | ( y , x ) ∈ IEXT(p2) }`

  Both inclusions of the set equality are kept, because `prp-inv1` and
  `prp-inv2` use opposite ones, and both `IP` conjuncts are kept, because each
  of those two rules has a head predicate that no body triple forces into `IP`. -/
  inv_fwd : ∀ p q, I.IEXT (I.IS V.inverseOf) p q →
    I.IP p ∧ I.IP q ∧ ∀ x y, (I.IEXT p x y ↔ I.IEXT q y x)

  /-- RBS Table 5.13, the `owl:SymmetricProperty` row, forward, universally
  quantified part only.

  > `p ∈ ICEXT(I(owl:SymmetricProperty))` **iff**
  > `p ∈ IP , ∀ x , y : ( x , y ) ∈ IEXT(p) implies ( y , x ) ∈ IEXT(p)`

  The `p ∈ IP` conjunct is dropped because `prp-symp`'s head predicate is the
  same `?p` that its body triple already carries into `IP` through the truth
  clause. Keeping it would be faithful and redundant; dropping it is weaker. -/
  sym_fwd : ∀ p, I.ICEXT (I.IS V.symmetricProperty) p →
    ∀ x y, I.IEXT p x y → I.IEXT p y x
  /-- RBS Table 5.13, the `owl:TransitiveProperty` row, same treatment.

  > `p ∈ ICEXT(I(owl:TransitiveProperty))` **iff** `p ∈ IP , ∀ x , y , z :
  > ( x , y ) ∈ IEXT(p) and ( y , z ) ∈ IEXT(p) implies ( x , z ) ∈ IEXT(p)` -/
  trp_fwd : ∀ p, I.ICEXT (I.IS V.transitiveProperty) p →
    ∀ x y z, I.IEXT p x y → I.IEXT p y z → I.IEXT p x z

  /- ### RBS Table 5.6, property restrictions

  The OUTER connective of this table is an IF-THEN and the CONSEQUENT is a SET
  EQUALITY. Both are load-bearing and they pull opposite ways, which is what
  makes this the easiest row in the Recommendation to get wrong. The table's
  header row reads "if | then", and Tables 5.8 and 5.9 two sections later ARE
  `iff`s, so a reader carrying that habit here shrinks the model class while
  every proof still passes. The equality is written once, as the specification
  has it, and no second field may be added in this area to close a proof. -/

  /-- Table 5.6, row 1.

  > **if** `( z , c ) ∈ IEXT(I(owl:someValuesFrom)) ,
  > ( z , p ) ∈ IEXT(I(owl:onProperty))` **then**
  > `ICEXT(z) = { x | ∃ y : ( x , y ) ∈ IEXT(p) and y ∈ ICEXT(c) }`

  Both inclusions are consumed: `⊇` by `cls-svf1`, `⊆` by `scm-svf1` and
  `scm-svf2`. -/
  svf_eq : ∀ z c p, I.IEXT (I.IS V.someValuesFrom) z c →
    I.IEXT (I.IS V.onProperty) z p →
    ∀ x, (I.ICEXT z x ↔ ∃ y, I.IEXT p x y ∧ I.ICEXT c y)
  /-- Table 5.6, row 3.

  > **if** `( z , c ) ∈ IEXT(I(owl:allValuesFrom)) ,
  > ( z , p ) ∈ IEXT(I(owl:onProperty))` **then**
  > `ICEXT(z) = { x | ∀ y : ( x , y ) ∈ IEXT(p) implies y ∈ ICEXT(c) }`

  Both inclusions consumed: `⊆` by `cls-avf`, `⊇` by `scm-avf1` and
  `scm-avf2`. -/
  avf_eq : ∀ z c p, I.IEXT (I.IS V.allValuesFrom) z c →
    I.IEXT (I.IS V.onProperty) z p →
    ∀ x, (I.ICEXT z x ↔ ∀ y, I.IEXT p x y → I.ICEXT c y)
  /-- Table 5.6, row 5.

  > **if** `( z , a ) ∈ IEXT(I(owl:hasValue)) ,
  > ( z , p ) ∈ IEXT(I(owl:onProperty))` **then**
  > `ICEXT(z) = { x | ( x , a ) ∈ IEXT(p) }` -/
  hv_eq : ∀ z a p, I.IEXT (I.IS V.hasValue) z a →
    I.IEXT (I.IS V.onProperty) z p →
    ∀ x, (I.ICEXT z x ↔ I.IEXT p x a)

  /-- RBS Table 5.2, the `owl:Restriction` row, third column:
  `ICEXT(I(owl:Restriction)) ⊆ IC`.

  SUBSET and never equality. The table writes `⊆` here where it writes `=` for
  `rdfs:Class`, `rdf:Property`, `rdfs:Resource` and `owl:Thing`, and writing the
  equality would force every class to be a restriction. The second column of the
  same row, `owl:Restriction ∈ IC`, is not taken: no rule consumes it. -/
  restr_IC : ∀ x, I.ICEXT (I.IS V.Restriction) x → I.IC x
  /-- RBS Table 5.3, the `owl:someValuesFrom` row, third column:
  `IEXT(I(owl:someValuesFrom)) ⊆ ICEXT(I(owl:Restriction)) × IC`. -/
  svf_typ : ∀ z c, I.IEXT (I.IS V.someValuesFrom) z c →
    I.ICEXT (I.IS V.Restriction) z ∧ I.IC c
  /-- RBS Table 5.3, the `owl:allValuesFrom` row, third column:
  `IEXT(I(owl:allValuesFrom)) ⊆ ICEXT(I(owl:Restriction)) × IC`. -/
  avf_typ : ∀ z c, I.IEXT (I.IS V.allValuesFrom) z c →
    I.ICEXT (I.IS V.Restriction) z ∧ I.IC c
  /-- RBS Table 5.3, the `owl:onProperty` row, third column:
  `IEXT(I(owl:onProperty)) ⊆ ICEXT(I(owl:Restriction)) × IP`.

  The `IP` half is what carries the restriction rules across the bridge: under
  the bridge `Interp.iext p x y` demands `IP p`, and for the property of a
  restriction nothing else supplies it. -/
  onp_typ : ∀ z p, I.IEXT (I.IS V.onProperty) z p →
    I.ICEXT (I.IS V.Restriction) z ∧ I.IP p

  /- ### RBS Tables 5.4 and 5.5, the list constructors

  Forward direction only, and the `IC` conjuncts of each cell are dropped: no
  arm consumes them, so leaving them out enlarges the class. -/

  /-- RBS Table 5.4, the `owl:intersectionOf` row, forward.

  > **if** `s` sequence of `c1 , … , cn ∈ IR` **then**
  > `( z , s ) ∈ IEXT(I(owl:intersectionOf))` **iff**
  > `z , c1 , … , cn ∈ IC , ICEXT(z) = ICEXT(c1) ∩ … ∩ ICEXT(cn)`

  **A READING IS LOAD-BEARING HERE AND IT IS THE ONLY ONE IN THIS FILE.** The
  `∀ c ∈ cs` form below reads `ICEXT(c1) ∩ … ∩ ICEXT(cn)` at `n = 0` as `IR`,
  the empty intersection taken inside the universe. Three things license it and
  none of them is decisive on its own, so it is flagged rather than assumed:
  the convention section defines the sequence expression for "n ≥ 0"; the two
  neighbouring rows of this very table write "n ≥ 1" explicitly where they mean
  it, so the absence of that phrase here is a choice; and Table 5.1 gives
  `ICEXT(x) ⊆ IR`, which is the universe the intersection is taken in.

  The reading is not idle. `Rules.lean`'s `takeChain` accepts an empty chain, so
  `cls-int1` fires on `c owl:intersectionOf rdf:nil` and concludes
  `x rdf:type c` for EVERY `x`. `Semantics.lean`'s `Model.int` quantifies over
  the whole domain and therefore already commits this repository to the same
  reading. Under the other reading, that step would be unsound and this field
  would be too strong. -/
  int_fwd : ∀ z s cs, I.IEXT (I.IS V.intersectionOf) z s → I.Seq s cs →
    ∀ x, (I.ICEXT z x ↔ ∀ c ∈ cs, I.ICEXT c x)
  /-- RBS Table 5.4, the `owl:unionOf` row, forward.

  > **if** `s` sequence of `c1 , … , cn ∈ IR` **then**
  > `( z , s ) ∈ IEXT(I(owl:unionOf))` **iff**
  > `z , c1 , … , cn ∈ IC , ICEXT(z) = ICEXT(c1) ∪ … ∪ ICEXT(cn)`

  At `n = 0` the `∃ c ∈ cs` form reads the empty union as `∅`, which needs no
  convention. -/
  uni_fwd : ∀ z s cs, I.IEXT (I.IS V.unionOf) z s → I.Seq s cs →
    ∀ x, (I.ICEXT z x ↔ ∃ c ∈ cs, I.ICEXT c x)
  /-- RBS Table 5.5, forward.

  > **if** `s` sequence of `a1 , … , an ∈ IR` **then**
  > `( z , s ) ∈ IEXT(I(owl:oneOf))` **iff** `z ∈ IC , ICEXT(z) = { a1 , … , an }`

  `Model.oneOf` is the `⊇` half of this equality and nothing else; the `⊆` half
  is available here and consumed by no rule. -/
  oneOf_fwd : ∀ z s as, I.IEXT (I.IS V.oneOf) z s → I.Seq s as →
    ∀ x, (I.ICEXT z x ↔ ∃ a ∈ as, x = a)

/-! ## The bridge, proved

Each lemma below is the step the old paragraph asked a reader to take. The five
`IP` facts it assumed are the first five, and each is one projection out of one
axiomatic triple. -/

namespace Conforming

variable {I : Interpretation}

/-- `I(rdf:type) ∈ IP`, discharged. The predicate of `rdf:type rdf:type
rdf:Property .` is `rdf:type`, so RDF11 section 5's truth clause hands this back
as the triple's own first conjunct. -/
theorem IP_type (C : Conforming I) : I.IP (I.IS V.type) := C.ax_type.1

/-- `I(rdfs:subClassOf) ∈ IP`, discharged from
`rdfs:Datatype rdfs:subClassOf rdfs:Class .`. -/
theorem IP_subClassOf (C : Conforming I) : I.IP (I.IS V.subClassOf) :=
  C.ax_subClassOf.1

/-- `I(rdfs:subPropertyOf) ∈ IP`, discharged from
`rdfs:isDefinedBy rdfs:subPropertyOf rdfs:seeAlso .`. -/
theorem IP_subPropertyOf (C : Conforming I) : I.IP (I.IS V.subPropertyOf) :=
  C.ax_subPropertyOf.1

/-- `I(rdfs:domain) ∈ IP`, discharged from
`rdf:type rdfs:domain rdfs:Resource .`. -/
theorem IP_domain (C : Conforming I) : I.IP (I.IS V.domain) := C.ax_domain.1

/-- `I(rdfs:range) ∈ IP`, discharged from `rdf:type rdfs:range rdfs:Class .`. -/
theorem IP_range (C : Conforming I) : I.IP (I.IS V.range) := C.ax_range.1

/-- `Interp.cext` IS `ICEXT`. The left-to-right direction is free; the other way
needs `I(rdf:type) ∈ IP`, and that is the whole of what the first assumed fact
was for. -/
theorem cext_iff (C : Conforming I) {c x : I.IR} :
    I.toInterp.cext c x ↔ I.ICEXT c x :=
  ⟨fun h => h.2, fun h => ⟨C.IP_type, h⟩⟩

/-- `Interp.IC` IS `IC`. -/
theorem IC_iff (C : Conforming I) {c : I.IR} : I.toInterp.IC c ↔ I.IC c :=
  C.cext_iff

/-- `Interp.sc` IS membership in `IEXT(I(rdfs:subClassOf))`. -/
theorem sc_iff (C : Conforming I) {a b : I.IR} :
    I.toInterp.sc a b ↔ I.IEXT (I.IS V.subClassOf) a b :=
  ⟨fun h => h.2, fun h => ⟨C.IP_subClassOf, h⟩⟩

/-- `Interp.sp` IS membership in `IEXT(I(rdfs:subPropertyOf))`. -/
theorem sp_iff (C : Conforming I) {a b : I.IR} :
    I.toInterp.sp a b ↔ I.IEXT (I.IS V.subPropertyOf) a b :=
  ⟨fun h => h.2, fun h => ⟨C.IP_subPropertyOf, h⟩⟩

/-- Every graph chain is a semantic sequence, in any interpretation that
satisfies the graph.

This is the comparison `Semantics.lean` and `W3C.lean` argue in English at their
list fields and never state. `Chain G` quantifies over the ASSERTED GRAPH and
the specification's sequence expression quantifies over
`IEXT(I(rdf:first))` and `IEXT(I(rdf:rest))`; the two meet exactly here, because
a satisfied graph puts each of its own `rdf:first` and `rdf:rest` triples into
those extensions. The converse is false and is not needed: a sequence built from
pairs the graph never asserts is not a graph chain, which is why the conditions
stated over `Chain` are the weaker ones. -/
theorem seq_of_chain {G : List Triple} (hG : ∀ t ∈ G, I.Sat t) :
    ∀ {l : Term} {ms : List Term}, Chain G l ms → I.Seq (I.den l) (ms.map I.den) := by
  intro l ms h
  induction h with
  | nil => exact Interpretation.Seq.nil
  | @cons l m l' ms hf hr _ ih =>
      have h1 : I.IEXT (I.den V.first) (I.den l) (I.den m) := (hG _ hf).2
      have h2 : I.IEXT (I.den V.rest) (I.den l) (I.den l') := (hG _ hr).2
      exact Interpretation.Seq.cons h1 h2 ih

end Conforming

/-! ### Two list lemmas

Core Lean only, so they are proved here rather than imported. -/

theorem forall_mem_map_iff {α β : Type} (f : α → β) (P : β → Prop) (l : List α) :
    (∀ b ∈ l.map f, P b) ↔ ∀ a ∈ l, P (f a) := by
  constructor
  · intro h a ha; exact h (f a) (List.mem_map_of_mem ha)
  · intro h b hb
    match List.mem_map.mp hb with
    | ⟨a, ha, he⟩ => exact he ▸ h a ha

theorem exists_mem_map_iff {α β : Type} (f : α → β) (P : β → Prop) (l : List α) :
    (∃ b ∈ l.map f, P b) ↔ ∃ a ∈ l, P (f a) := by
  constructor
  · intro ⟨b, hb, hp⟩
    match List.mem_map.mp hb with
    | ⟨a, ha, he⟩ => exact ⟨a, ha, he ▸ hp⟩
  · intro ⟨a, ha, hp⟩
    exact ⟨f a, List.mem_map_of_mem ha, hp⟩

namespace Conforming

variable {I : Interpretation}

/-- **Every conforming interpretation carries the specification conditions of
`W3C.lean` at full strength.**

This is the theorem the module docstring of `W3C.lean` said did not exist. Each
field is the corresponding quoted cell, read through the bridge; the only work
in the proof is supplying the `IP` memberships that `Interp.iext` demands and
the specification's cells do not mention, and every one of those comes from an
axiomatic triple above rather than from an assumption. -/
theorem toW3C (C : Conforming I) : W3C I.toInterp I.IP where
  sc_fwd := fun a b h =>
    match C.sc_fwd a b h.2 with
    | ⟨ha, hb, hsub⟩ =>
      ⟨C.IC_iff.mpr ha, C.IC_iff.mpr hb,
       fun x hx => C.cext_iff.mpr (hsub x (C.cext_iff.mp hx))⟩
  sc_bwd := fun a b ha hb hsub =>
    ⟨C.IP_subClassOf, C.sc_bwd a b (C.IC_iff.mp ha) (C.IC_iff.mp hb)
      (fun x hx => C.cext_iff.mp (hsub x (C.cext_iff.mpr hx)))⟩
  sp_fwd := fun a b h =>
    match C.sp_fwd a b h.2 with
    | ⟨ha, hb, hsub⟩ =>
      ⟨ha, hb, fun x y hxy => ⟨hb, hsub x y hxy.2⟩⟩
  sp_bwd := fun a b ha hb hsub =>
    ⟨C.IP_subPropertyOf, C.sp_bwd a b ha hb
      (fun x y hxy => (hsub x y ⟨ha, hxy⟩).2)⟩
  dom_fwd := fun p c h =>
    match C.dom_fwd p c h.2 with
    | ⟨hp, hc, hall⟩ =>
      ⟨hp, C.IC_iff.mpr hc, fun x y hxy => C.cext_iff.mpr (hall x y hxy.2)⟩
  dom_bwd := fun p c hp hc hall =>
    ⟨C.IP_domain, C.dom_bwd p c hp (C.IC_iff.mp hc)
      (fun x y hxy => C.cext_iff.mp (hall x y ⟨hp, hxy⟩))⟩
  rng_fwd := fun p c h =>
    match C.rng_fwd p c h.2 with
    | ⟨hp, hc, hall⟩ =>
      ⟨hp, C.IC_iff.mpr hc, fun x y hxy => C.cext_iff.mpr (hall x y hxy.2)⟩
  rng_bwd := fun p c hp hc hall =>
    ⟨C.IP_range, C.rng_bwd p c hp (C.IC_iff.mp hc)
      (fun x y hxy => C.cext_iff.mp (hall x y ⟨hp, hxy⟩))⟩
  eqc_fwd := fun a b h =>
    match C.eqc_fwd a b h.2 with
    | ⟨ha, hb, heq⟩ =>
      ⟨C.IC_iff.mpr ha, C.IC_iff.mpr hb,
       fun x => ⟨fun hx => C.cext_iff.mpr ((heq x).mp (C.cext_iff.mp hx)),
                 fun hx => C.cext_iff.mpr ((heq x).mpr (C.cext_iff.mp hx))⟩⟩
  eqp_fwd := fun a b h =>
    match C.eqp_fwd a b h.2 with
    | ⟨ha, hb, heq⟩ =>
      ⟨ha, hb, fun x y => ⟨fun hxy => ⟨hb, (heq x y).mp hxy.2⟩,
                           fun hxy => ⟨ha, (heq x y).mpr hxy.2⟩⟩⟩
  same_fwd := fun a b h => C.same_fwd a b h.2
  inv_fwd := fun p q h =>
    match C.inv_fwd p q h.2 with
    | ⟨hp, hq, heq⟩ =>
      ⟨hp, hq, fun x y => ⟨fun hxy => ⟨hq, (heq x y).mp hxy.2⟩,
                           fun hyx => ⟨hp, (heq x y).mpr hyx.2⟩⟩⟩
  sym_fwd := fun p h x y hxy =>
    ⟨hxy.1, C.sym_fwd p (C.cext_iff.mp h) x y hxy.2⟩
  trp_fwd := fun p h x y z hxy hyz =>
    ⟨hxy.1, C.trp_fwd p (C.cext_iff.mp h) x y z hxy.2 hyz.2⟩
  svf_eq := fun z c p h1 h2 x =>
    ⟨fun hx =>
      match (C.svf_eq z c p h1.2 h2.2 x).mp (C.cext_iff.mp hx) with
      | ⟨y, hxy, hy⟩ => ⟨y, ⟨(C.onp_typ z p h2.2).2, hxy⟩, C.cext_iff.mpr hy⟩,
     fun hx =>
      match hx with
      | ⟨y, hxy, hy⟩ =>
        C.cext_iff.mpr ((C.svf_eq z c p h1.2 h2.2 x).mpr
          ⟨y, hxy.2, C.cext_iff.mp hy⟩)⟩
  avf_eq := fun z c p h1 h2 x =>
    ⟨fun hx y hxy =>
      C.cext_iff.mpr ((C.avf_eq z c p h1.2 h2.2 x).mp (C.cext_iff.mp hx) y hxy.2),
     fun hall =>
      C.cext_iff.mpr ((C.avf_eq z c p h1.2 h2.2 x).mpr
        (fun y hxy => C.cext_iff.mp (hall y ⟨(C.onp_typ z p h2.2).2, hxy⟩)))⟩
  hv_eq := fun z a p h1 h2 x =>
    ⟨fun hx => ⟨(C.onp_typ z p h2.2).2, (C.hv_eq z a p h1.2 h2.2 x).mp (C.cext_iff.mp hx)⟩,
     fun hxa => C.cext_iff.mpr ((C.hv_eq z a p h1.2 h2.2 x).mpr hxa.2)⟩
  restr_IC := fun x h => C.IC_iff.mpr (C.restr_IC x (C.cext_iff.mp h))
  svf_typ := fun z c h =>
    match C.svf_typ z c h.2 with
    | ⟨h1, h2⟩ => ⟨C.cext_iff.mpr h1, C.IC_iff.mpr h2⟩
  avf_typ := fun z c h =>
    match C.avf_typ z c h.2 with
    | ⟨h1, h2⟩ => ⟨C.cext_iff.mpr h1, C.IC_iff.mpr h2⟩
  onp_typ := fun z p h =>
    match C.onp_typ z p h.2 with
    | ⟨h1, h2⟩ => ⟨C.cext_iff.mpr h1, h2⟩

/-- **Every conforming interpretation that satisfies the graph is a
`W3CModel` of it.**

The three list fields are where `Chain` meets the specification's sequence
expression, through `seq_of_chain`. -/
theorem toW3CModel (C : Conforming I) {G : List Triple} (hG : ∀ t ∈ G, I.Sat t) :
    W3CModel I.toInterp I.IP G where
  conds := C.toW3C
  facts := hG
  int_eq := fun c l ms hin hch x => by
    have hax : I.IEXT (I.den V.intersectionOf) (I.den c) (I.den l) := (hG _ hin).2
    have h := C.int_fwd (I.den c) (I.den l) (ms.map I.den) hax (seq_of_chain hG hch) x
    have hm := forall_mem_map_iff I.den (fun k => I.ICEXT k x) ms
    exact ⟨fun hx m hmem => C.cext_iff.mpr (hm.mp (h.mp (C.cext_iff.mp hx)) m hmem),
           fun hall => C.cext_iff.mpr (h.mpr (hm.mpr
             (fun m hmem => C.cext_iff.mp (hall m hmem))))⟩
  uni_eq := fun c l ms hin hch x => by
    have hax : I.IEXT (I.den V.unionOf) (I.den c) (I.den l) := (hG _ hin).2
    have h := C.uni_fwd (I.den c) (I.den l) (ms.map I.den) hax (seq_of_chain hG hch) x
    have hm := exists_mem_map_iff I.den (fun k => I.ICEXT k x) ms
    exact ⟨fun hx => match hm.mp (h.mp (C.cext_iff.mp hx)) with
                     | ⟨m, hmem, hk⟩ => ⟨m, hmem, C.cext_iff.mpr hk⟩,
           fun hex => match hex with
                      | ⟨m, hmem, hk⟩ =>
                        C.cext_iff.mpr (h.mpr (hm.mpr ⟨m, hmem, C.cext_iff.mp hk⟩))⟩
  oneOf_eq := fun c l ms hin hch x => by
    have hax : I.IEXT (I.den V.oneOf) (I.den c) (I.den l) := (hG _ hin).2
    have h := C.oneOf_fwd (I.den c) (I.den l) (ms.map I.den) hax (seq_of_chain hG hch) x
    have hm := exists_mem_map_iff I.den (fun a => x = a) ms
    exact ⟨fun hx => hm.mp (h.mp (C.cext_iff.mp hx)),
           fun hex => C.cext_iff.mpr (h.mpr (hm.mpr hex))⟩

/-- Every conforming interpretation that satisfies the graph is a `Model` of
it, which is what `certificate_sound` quantifies over. -/
theorem toModel (C : Conforming I) {G : List Triple} (hG : ∀ t ∈ G, I.Sat t) :
    Model I.toInterp G :=
  (C.toW3CModel hG).toModel

end Conforming

/-! ## The sentence the project wants -/

/-- `G` entails `t` over the conforming interpretations: every interpretation
meeting the quoted conditions and satisfying every triple of `G` satisfies `t`,
where "satisfies" is RDF 1.1 Semantics section 5's truth clause and nothing
else.

Compare `Entails`, which quantifies over `Semantics.lean`'s much larger
`Conditions` class, and `W3CEntails`, which quantifies over `W3CModel`. This is
the smallest class of the three, so this is the LARGEST relation of the three,
and a non-entailment stated over it is correspondingly the weakest. -/
def ConformingEntails (G : List Triple) (t : Triple) : Prop :=
  ∀ I : Interpretation, Conforming I → (∀ u ∈ G, I.Sat u) → I.Sat t

/-- Everything `Entails` gives holds in every conforming interpretation. -/
theorem ConformingEntails.of_entails {G : List Triple} {t : Triple}
    (h : Entails G t) : ConformingEntails G t :=
  fun _ C hG => h _ (C.toModel hG)

/-- **A checked certificate's conclusions are true in every conforming
interpretation of the asserted graph.**

`certificate_w3c_sound`'s docstring says of itself that it "is not yet the
sentence "true in every conforming interpretation", and the difference is the
bridge in the module docstring above, which is an argument and not a theorem".
This is that sentence, and the bridge is now `Conforming.toW3C` and
`Conforming.toW3CModel`.

What the statement mentions: `checkCert`, which is the checker; `Interpretation`,
which is RBS Definition 4.2's tuple; `Conforming`, which is a list of quoted
cells; and `Interpretation.Sat`, which is RDF 1.1 Semantics section 5's truth
clause. It mentions no structure invented by this repository.

What it still does not mention, and what therefore remains to be checked by
hand, is that `Conforming` is a SUBSET of the Recommendation's conditions rather
than a paraphrase of them. That is a containment, one field at a time, and the
module docstring says which conditions are left out and why leaving them out
enlarges the class. It is not a reading of one structure as another, and it does
not assume anything about axiomatic triples: the five facts that used to be
assumed are fields of `Conforming`, quoted, and discharged in the proof.

The one remaining modelling decision that does NOT run in the safe direction is
`IL` being total, which excludes RDF 1.1 interpretations in which some literal
fails to denote. The obligation that would close it is named at the
`Interpretation` structure. -/
theorem certificate_conforming_sound (G : List Triple) (steps : List Step)
    (h : checkCert G steps = true) :
    ∀ st ∈ steps, ConformingEntails G st.conclusion :=
  fun st hst => ConformingEntails.of_entails (certificate_sound G steps h st hst)

/-! ## Tripwires

Two kinds, and both are in the build rather than in a document, because a claim
that lives in a document is a claim that goes stale.

The first is the axiom guard that `Soundness.lean` and `W3C.lean` carry. A
`sorry` or a `native_decide` anywhere in the bridge changes the footprint below
and fails the build here. The three axioms that ARE listed are Lean's own, and
they arrive through `String.data`, which `termKind` reads to tell an IRI from a
literal; `#print axioms String.data` reports the same three. What the guard
rules out is `sorryAx` and `Lean.ofReduceBool`, which is what `native_decide`
would add.

The second is new. Everything this file proves rests on `Conditions`, `Interp`,
`Model`, `Entails` and `certificate_sound` being what they were when it was
written, and a layer added on top is exactly the situation in which one of them
gets quietly adjusted to make a proof go through. "Unchanged" is not something to
check by eye. `#check` on a structure's CONSTRUCTOR prints every field's
statement in order, so the blocks below pin all twenty-three fields of
`Conditions` and all six of `Model`, not merely their names. A field weakened,
strengthened, added, removed or reordered anywhere in the repository breaks this
file. That is verbose on purpose: the verbosity is the check.
-/


/-- info: 'OOCert.Conforming.toW3C' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Conforming.toW3C

/-- info: 'OOCert.Conforming.toW3CModel' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms Conforming.toW3CModel

/-- info: 'OOCert.certificate_conforming_sound' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms certificate_conforming_sound

/-- info: Interp.mk : (D : Type) → (Term → D) → (D → D → D → Prop) → Interp -/
#guard_msgs in
#check @Interp.mk

/--
info: @Conditions.mk : ∀ {I : Interp},
  (∀ (a b : I.D), I.sc a b → ∀ (x : I.D), I.cext a x → I.cext b x) →
    (∀ (a b c : I.D), I.sc a b → I.sc b c → I.sc a c) →
      (∀ (a b : I.D), I.sp a b → ∀ (x y : I.D), I.iext a x y → I.iext b x y) →
        (∀ (a b c : I.D), I.sp a b → I.sp b c → I.sp a c) →
          (∀ (p c : I.D), I.iext (I.ι V.domain) p c → ∀ (x y : I.D), I.iext p x y → I.cext c x) →
            (∀ (p c : I.D), I.iext (I.ι V.range) p c → ∀ (x y : I.D), I.iext p x y → I.cext c y) →
              (∀ (p : I.D),
                  I.cext (I.ι V.transitiveProperty) p → ∀ (x y z : I.D), I.iext p x y → I.iext p y z → I.iext p x z) →
                (∀ (p : I.D), I.cext (I.ι V.symmetricProperty) p → ∀ (x y : I.D), I.iext p x y → I.iext p y x) →
                  (∀ (p q : I.D), I.iext (I.ι V.inverseOf) p q → ∀ (x y : I.D), I.iext p x y ↔ I.iext q y x) →
                    (∀ (a b : I.D), I.iext (I.ι V.sameAs) a b → a = b) →
                      (∀ (a b : I.D), I.iext (I.ι V.equivalentClass) a b → I.sc a b ∧ I.sc b a) →
                        (∀ (a b : I.D), I.iext (I.ι V.equivalentProperty) a b → I.sp a b ∧ I.sp b a) →
                          (∀ (r p c : I.D),
                              I.iext (I.ι V.onProperty) r p →
                                I.iext (I.ι V.someValuesFrom) r c →
                                  ∀ (x y : I.D), I.iext p x y → I.cext c y → I.cext r x) →
                            (∀ (r p c : I.D),
                                I.iext (I.ι V.onProperty) r p →
                                  I.iext (I.ι V.allValuesFrom) r c →
                                    ∀ (x y : I.D), I.cext r x → I.iext p x y → I.cext c y) →
                              (∀ (r p v : I.D),
                                  I.iext (I.ι V.onProperty) r p →
                                    I.iext (I.ι V.hasValue) r v → ∀ (x : I.D), I.cext r x ↔ I.iext p x v) →
                                (∀ (c1 c2 p y1 y2 : I.D),
                                    I.iext (I.ι V.someValuesFrom) c1 y1 →
                                      I.iext (I.ι V.onProperty) c1 p →
                                        I.iext (I.ι V.someValuesFrom) c2 y2 →
                                          I.iext (I.ι V.onProperty) c2 p → I.sc y1 y2 → I.sc c1 c2) →
                                  (∀ (c1 c2 p1 p2 y : I.D),
                                      I.iext (I.ι V.someValuesFrom) c1 y →
                                        I.iext (I.ι V.onProperty) c1 p1 →
                                          I.iext (I.ι V.someValuesFrom) c2 y →
                                            I.iext (I.ι V.onProperty) c2 p2 → I.sp p1 p2 → I.sc c1 c2) →
                                    (∀ (c1 c2 p y1 y2 : I.D),
                                        I.iext (I.ι V.allValuesFrom) c1 y1 →
                                          I.iext (I.ι V.onProperty) c1 p →
                                            I.iext (I.ι V.allValuesFrom) c2 y2 →
                                              I.iext (I.ι V.onProperty) c2 p → I.sc y1 y2 → I.sc c1 c2) →
                                      (∀ (c1 c2 p1 p2 y : I.D),
                                          I.iext (I.ι V.allValuesFrom) c1 y →
                                            I.iext (I.ι V.onProperty) c1 p1 →
                                              I.iext (I.ι V.allValuesFrom) c2 y →
                                                I.iext (I.ι V.onProperty) c2 p2 → I.sp p1 p2 → I.sc c2 c1) →
                                        (∀ (p c1 c2 : I.D),
                                            I.iext (I.ι V.domain) p c1 → I.sc c1 c2 → I.iext (I.ι V.domain) p c2) →
                                          (∀ (p1 p2 c : I.D),
                                              I.iext (I.ι V.domain) p2 c → I.sp p1 p2 → I.iext (I.ι V.domain) p1 c) →
                                            (∀ (p c1 c2 : I.D),
                                                I.iext (I.ι V.range) p c1 → I.sc c1 c2 → I.iext (I.ι V.range) p c2) →
                                              (∀ (p1 p2 c : I.D),
                                                  I.iext (I.ι V.range) p2 c → I.sp p1 p2 → I.iext (I.ι V.range) p1 c) →
                                                Conditions I
-/
#guard_msgs in
#check @Conditions.mk

/--
info: @Model.mk : ∀ {I : Interp} {G : List Triple},
  Conditions I →
    (∀ (t : Triple), t ∈ G → I.sat t) →
      (∀ (c l : Term) (ms : List Term),
          { s := c, p := V.intersectionOf, o := l } ∈ G →
            Chain G l ms → ∀ (x : I.D), (∀ (m : Term), m ∈ ms → I.cext (I.ι m) x) → I.cext (I.ι c) x) →
        (∀ (c l : Term) (ms : List Term),
            { s := c, p := V.intersectionOf, o := l } ∈ G →
              Chain G l ms → ∀ (x : I.D), I.cext (I.ι c) x → ∀ (m : Term), m ∈ ms → I.cext (I.ι m) x) →
          (∀ (c l : Term) (ms : List Term),
              { s := c, p := V.unionOf, o := l } ∈ G →
                Chain G l ms → ∀ (x : I.D) (m : Term), m ∈ ms → I.cext (I.ι m) x → I.cext (I.ι c) x) →
            (∀ (c l : Term) (ms : List Term),
                { s := c, p := V.oneOf, o := l } ∈ G → Chain G l ms → ∀ (m : Term), m ∈ ms → I.cext (I.ι c) (I.ι m)) →
              Model I G
-/
#guard_msgs in
#check @Model.mk

/-- info: Entails : List Triple → Triple → Prop -/
#guard_msgs in
#check @Entails

/--
info: certificate_sound : ∀ (G : List Triple) (steps : List Step),
  checkCert G steps = true → ∀ (st : Step), st ∈ steps → Entails G st.conclusion
-/
#guard_msgs in
#check @certificate_sound

end OOCert
