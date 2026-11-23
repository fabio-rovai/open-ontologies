import Shacl.Report

/-!
# Witnesses: the specification says something, and it does not say everything

`eval_conforms_iff` and `validate_spec` are worth nothing on their own. If `Conf`
held of every focus node, "the evaluator reports nothing exactly when the node
conforms" would be a promise that the evaluator never reports anything, and the
theorem would still be true. If `Conf` held of none, the mirror image. This file
closes both by construction, the way `lean/OOCert/Witness.lean` does for the
derivation checker.

| theorem | says |
|---|---|
| `alice_conforms` | some node conforms to a real shape, proved from the specification by hand |
| `bob_does_not_conform` | some node does not, so `Conf` is not the trivial relation |
| `the_report_is_empty_when_everything_conforms` | `validate` can return nothing |
| `the_report_names_the_failing_node` | and it can return something, naming the right node and component |
| `duplicate_triples_are_counted_once` | `sh:maxCount` counts the SET of value nodes |
| `two_distinct_values_break_maxCount_one` | and it really does count them |
| `the_evaluator_refuses_rather_than_guessing` | the third answer is reachable, not decoration |
| `four_point_zero_is_at_most_four` | the value-range constraints compare VALUES, not spellings |
| `four_point_one_is_not_at_most_four` | and they still reject something |
| `a_string_is_not_greater_than_a_number` | a SPARQL type error is a violation, not a refusal |
| `mixed_timezones_are_unordered` | a dateTime with a timezone and one without can be UNORDERED |
| `a_day_earlier_is_ordered_even_across_the_window` | and that is not a blanket refusal |
| `a_blank_node_has_no_length` | a blank node fails every length constraint |
| `an_iri_is_as_long_as_it_is_written` | and an IRI does not |
| `a_sequence_path_reaches_two_steps` | a sequence path is a join, not a union |
| `a_sequence_path_does_not_stop_halfway` | and the intermediate node is not a value node |
| `zero_or_one_admits_the_focus_node` | `sh:zeroOrOnePath` gives a bare focus node one value node |

## These have been seen to fail

Changing `strRep` so that a blank node reports `chars []` rather than `noString`,
which is the natural mistake (a blank node has no string, so treat it as the empty
one), leaves the whole development compiling and the agreement theorem true, because
the specification and the evaluator both read `strRep`. This file is what stops it:

```text
error: Shacl/Witness.lean:284:62: Tactic `decide` proved that the proposition
  ¬Conf [] (Shape.minLength 0) "_:b"
is false
```

A witness pins a reading of the Recommendation that no proof about internal
agreement can pin. That is why they are here and why each one comes in a pair.

The two counting theorems are the ones worth reading. SHACL counts distinct value
nodes, so a triple asserted twice must not make a `sh:maxCount 1` fail. An engine
that counts SPARQL solutions rather than value nodes gets that wrong. The
repository's own SHACL validator used to double-count a node selected by two
target declarations, a different counting mistake with the same shape; it now
validates each shape once over the union of its targets
(commit `2e827b8`, this branch), and `docs/lean-certificates.md` records a
dated amendment to the bullet that said otherwise. Here the counting is
settled in the specification and proved.
-/
namespace Shacl

/-! ## A small graph -/

def exAlice : Term := "<http://ex.org/alice>"
def exBob : Term := "<http://ex.org/bob>"
def exPerson : Term := "<http://ex.org/Person>"
def exAgent : Term := "<http://ex.org/Agent>"
def exName : Term := "<http://ex.org/name>"
def exKnows : Term := "<http://ex.org/knows>"
def exShape : Term := "<http://ex.org/AgentShape>"
def litAlice : Term := "\"Alice\""
def litBob : Term := "\"Bob\""

/-- Alice is a Person, Person is a subclass of Agent, Alice has a name, and Bob
knows Alice. Bob has no type and no name. -/
def gEx : Graph :=
  [ ⟨exAlice, V.type, exPerson⟩,
    ⟨exPerson, V.subClassOf, exAgent⟩,
    ⟨exAlice, exName, litAlice⟩,
    ⟨exBob, exKnows, exAlice⟩ ]

/-- An Agent with at least one name, every name a string. Written the way the
compiler writes it: the property shape's constraints wrapped in `named` so results
carry its IRI, the count constraint and the value-node constraint conjoined
transparently. -/
def sAgent : Shape :=
  .named exShape
    (.both (.klass exAgent)
      (.both (.minCount (.pred exName) 1)
        (.forAll (.pred exName) (.datatype V.xsdString))))

/-! ## Conformance holds somewhere, proved from the specification by hand

These three go through the definition of `Conf` and never touch `eval`. If the only
evidence that the specification says anything came from running the evaluator, a
reader would be entitled to suspect the two agree because they are the same
mistake. -/

theorem alice_is_an_agent : IsInstance gEx exAgent exAlice :=
  ⟨exPerson, by simp [gEx], .step (by simp [gEx]) (.refl _)⟩

theorem alice_has_a_name : Conf gEx (.minCount (.pred exName) 1) exAlice := by
  refine ⟨[litAlice], by simp, ?_, by simp⟩
  intro v hv
  rw [List.mem_singleton] at hv
  subst hv
  show (⟨exAlice, exName, litAlice⟩ : Triple) ∈ gEx
  simp [gEx]

theorem alices_names_are_strings :
    Conf gEx (.forAll (.pred exName) (.datatype V.xsdString)) exAlice := by
  intro v hv
  have hv' : (⟨exAlice, exName, v⟩ : Triple) ∈ gEx := hv
  simp only [gEx, List.mem_cons, List.not_mem_nil, or_false, Triple.mk.injEq] at hv'
  rcases hv' with ⟨-, h, -⟩ | ⟨h, -, -⟩ | ⟨-, -, h⟩ | ⟨-, h, -⟩
  · exact absurd h (by decide)
  · exact absurd h (by decide)
  · subst h
    exact ⟨⟨"Alice", V.xsdString, none⟩, by decide, rfl, by decide⟩
  · exact absurd h (by decide)

/-- **Conformance is not the empty relation.** -/
theorem alice_conforms : Conf gEx sAgent exAlice :=
  ⟨alice_is_an_agent, alice_has_a_name, alices_names_are_strings⟩

/-! ## And it fails somewhere -/

theorem bob_has_no_type (t : Term) : (⟨exBob, V.type, t⟩ : Triple) ∉ gEx := by
  intro h
  simp only [gEx, List.mem_cons, List.not_mem_nil, or_false, Triple.mk.injEq] at h
  rcases h with ⟨h, -, -⟩ | ⟨h, -, -⟩ | ⟨h, -, -⟩ | ⟨-, h, -⟩ <;> exact absurd h (by decide)

/-- **Conformance is not the total relation.** Bob has no `rdf:type` at all, so he
is an instance of nothing, so he is not an Agent. -/
theorem bob_does_not_conform : ¬ Conf gEx sAgent exBob := by
  rintro ⟨⟨t, ht, -⟩, -, -⟩
  exact bob_has_no_type t ht

/-! ## The report, end to end

`validate` is what a validator actually runs. These two runs are settled by kernel
computation (`rfl`), so they are executable facts rather than claims about the
code. -/

def declAgent : ShapeDecl :=
  { id := exShape, targets := [.node exAlice], shape := sAgent }

def declBoth : ShapeDecl :=
  { id := exShape, targets := [.node exAlice, .node exBob], shape := sAgent }

/-- Targeting only Alice, the report is empty. -/
theorem the_report_is_empty_when_everything_conforms :
    validate gEx [declAgent] = .ok [] := by rfl

/-- Targeting Bob as well, the report names Bob, blames `sh:class`, and blames
`sh:minCount`: two results, with the property shape's IRI as `sh:sourceShape` and
the right constraint components. Written out in full so that a change to any field
of a result breaks this proof rather than passing quietly. -/
theorem the_report_names_the_failing_node :
    validate gEx [declBoth] = .ok
      [ { focus := exBob, path := none, value := some exBob, source := exShape,
          component := C.klass, blamed := .klass exAgent, blamedNode := exBob },
        { focus := exBob, path := some (.pred exName), value := none, source := exShape,
          component := C.minCount, blamed := .minCount (.pred exName) 1,
          blamedNode := exBob } ] := by rfl

/-- The two are not independent: because the report above is non-empty,
`validate_spec` says some targeted node fails, and because the one before it is
empty, `validate_spec` says every targeted node conforms. Both directions of the
theorem are therefore exercised on real data. -/
theorem the_specification_agrees_with_both_runs :
    (∀ d ∈ [declAgent], ∀ f, Targeted gEx d f → Conf gEx d.shape f) ∧
      ¬ (∀ d ∈ [declBoth], ∀ f, Targeted gEx d f → Conf gEx d.shape f) := by
  refine ⟨(validate_spec the_report_is_empty_when_everything_conforms).1.mp rfl, ?_⟩
  intro hall
  have hnil := (validate_spec the_report_names_the_failing_node).1.mpr hall
  exact absurd hnil (by simp)

/-! ## Counting is about the set of value nodes -/

def gDup : Graph := [⟨exAlice, exName, litAlice⟩, ⟨exAlice, exName, litAlice⟩]
def gTwo : Graph := [⟨exAlice, exName, litAlice⟩, ⟨exAlice, exName, litBob⟩]

/-- One value asserted twice is one value node, so `sh:maxCount 1` holds. Proved
from the specification, which quantifies over duplicate-free lists of value nodes
and so cannot be fooled by a repeated triple. -/
theorem duplicate_triples_are_counted_once :
    Conf gDup (.maxCount (.pred exName) 1) exAlice := by
  intro l hnd hmem
  have hsub : l ⊆ [litAlice] := by
    intro v hv
    have hv' : (⟨exAlice, exName, v⟩ : Triple) ∈ gDup := hmem v hv
    simp only [gDup, List.mem_cons, List.not_mem_nil, or_false, Triple.mk.injEq] at hv'
    rcases hv' with ⟨-, -, h⟩ | ⟨-, -, h⟩ <;> (subst h; simp)
  simpa using List.Nodup.length_le_of_subset hnd hsub

/-- Two distinct values are two value nodes, so the same constraint fails. Without
this the theorem above would be consistent with a specification that counts
nothing. -/
theorem two_distinct_values_break_maxCount_one :
    ¬ Conf gTwo (.maxCount (.pred exName) 1) exAlice := by
  intro h
  have hle := h [litAlice, litBob] (by decide) ?_
  · exact absurd hle (by decide)
  · intro v hv
    rcases List.mem_cons.mp hv with rfl | hv
    · show (⟨exAlice, exName, litAlice⟩ : Triple) ∈ gTwo; simp [gTwo]
    · rw [List.mem_singleton] at hv
      subst hv
      show (⟨exAlice, exName, litBob⟩ : Triple) ∈ gTwo; simp [gTwo]

/-! ## The third answer is reachable

A refusal that never fires is decoration. This one fires on the first
`rdf:XMLLiteral` it meets: RDF 1.1 says that datatype's lexical space is the set of
strings that are well-balanced, self-contained XML content, and deciding that needs
an XML parser this development does not have and will not pretend to have.
`tests/shacl_core_verified_test.rs` counts how often a refusal happens across the
W3C suite. -/

def xmlLiteral : Term := "<http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral>"
def litXml : Term :=
  "\"<b>hi</b>\"^^<http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral>"

/-- The literal has the right datatype IRI, so the constraint turns on whether it is
well formed, and `lexOK` does not know. The evaluator says so by name instead of
choosing an answer. -/
theorem the_evaluator_refuses_rather_than_guessing :
    eval [] (.datatype xmlLiteral) exShape litXml
      = .error (.unknownLexicalSpace xmlLiteral "<b>hi</b>") := by rfl

/-- The specification, meanwhile, treats an unjudgeable literal as non-conforming.
So the two possible behaviours at this point were "report a violation" and "report a
refusal", and the evaluator takes the weaker one. Neither is a false pass, which is
the property that matters. -/
theorem the_refused_literal_does_not_conform :
    ¬ Conf [] (.datatype xmlLiteral) litXml := by
  rintro ⟨l, hl, -, hok⟩
  have : l = ⟨"<b>hi</b>", xmlLiteral, none⟩ := by
    have : asLiteral litXml = some ⟨"<b>hi</b>", xmlLiteral, none⟩ := by decide
    rw [this] at hl
    exact (Option.some.inj hl).symm
  subst this
  exact absurd hok (by decide)

/-! ## The parameters added after the first eleven components

Each one below exhibits a model and something the semantics does NOT entail, the
same discipline the two counting theorems follow. A one-sided witness would leave
open that the new clause is trivially true or trivially false. Every one is settled
by kernel computation, so a change to the decision procedure breaks it rather than
passing quietly. -/

def lit4int : Term := "\"4\"^^<http://www.w3.org/2001/XMLSchema#integer>"
def lit40dec : Term := "\"4.0\"^^<http://www.w3.org/2001/XMLSchema#decimal>"
def lit41dec : Term := "\"4.1\"^^<http://www.w3.org/2001/XMLSchema#decimal>"

/-- **Comparison is by value, not by spelling.** `"4"^^xsd:integer` and
`"4.0"^^xsd:decimal` are two different TERMS, and `sh:maxInclusive 4` admits the
second. A validator comparing spellings would report a violation here. -/
theorem four_point_zero_is_at_most_four :
    Conf [] (.maxInclusive lit4int) lit40dec :=
  CmpIs.of_eq (k := .eq) (by decide) (by decide)

/-- And it is not vacuous: the same constraint rejects a larger value. -/
theorem four_point_one_is_not_at_most_four :
    ¬ Conf [] (.maxInclusive lit4int) lit41dec :=
  CmpIs.not_of_eq (k := .lt) (by decide) (by decide)

/-- **A pair SPARQL cannot compare is a violation, not a refusal.** `sh:minExclusive`
is defined as a SPARQL expression that must return TRUE, and `4 < "Hello"` is a type
error, which is not true. -/
theorem a_string_is_not_greater_than_a_number :
    ¬ Conf [] (.minExclusive lit4int) "\"Hello\"" :=
  CmpIs.not_of_eq (k := .incomparable) (by decide) (by decide)

def dtWithZone : Term :=
  "\"2002-10-10T12:00:00-05:00\"^^<http://www.w3.org/2001/XMLSchema#dateTime>"
def dtNoZone : Term :=
  "\"2002-10-10T12:00:00\"^^<http://www.w3.org/2001/XMLSchema#dateTime>"
def dtDayBefore : Term :=
  "\"2002-10-09T12:00:00-05:00\"^^<http://www.w3.org/2001/XMLSchema#dateTime>"

/-- **A timezoned dateTime and an untimezoned one within fourteen hours are
UNORDERED, and that is a verdict rather than a refusal.** The comparison answers,
and what it answers is that neither is first. -/
theorem mixed_timezones_are_unordered :
    cmpTerms dtWithZone dtNoZone = some .unordered := by decide

/-- So `sh:minInclusive` rejects it, which is what
`core/node/minInclusive-002` requires. -/
theorem an_unordered_value_does_not_conform :
    ¬ Conf [] (.minInclusive dtWithZone) dtNoZone :=
  CmpIs.not_of_eq (k := .unordered) (by decide) (by decide)

/-- Outside the fourteen-hour window the order IS settled, so the rule above is not
a blanket refusal to compare across timezones. -/
theorem a_day_earlier_is_ordered_even_across_the_window :
    cmpTerms dtNoZone dtDayBefore = some .gt := by decide

/-- **A blank node fails both length constraints**, whatever the bound, which is the
rule `core/node/minLength-001` and `core/node/maxLength-001` both turn on. -/
theorem a_blank_node_has_no_length :
    ¬ Conf [] (.minLength 0) "_:b" := StrLen.not_of_blank (by decide)

/-- An IRI, meanwhile, has the length of the IRI itself, so the same constraint at a
bound of three admits `<a:b>` and a bound of four does not. Two directions, one
term, so neither answer can be the constant one. -/
theorem an_iri_is_as_long_as_it_is_written :
    Conf [] (.maxLength 3) "<a:b>" ∧ ¬ Conf [] (.minLength 4) "<a:b>" :=
  ⟨StrLen.of_chars (cs := ['a', ':', 'b']) (by decide) (by decide),
   StrLen.not_of_chars (cs := ['a', ':', 'b']) (by decide) (by decide)⟩

/-! ### A sequence path reaches through an intermediate node -/

def exP1 : Term := "<http://ex.org/p1>"
def exP2 : Term := "<http://ex.org/p2>"
def exMid : Term := "<http://ex.org/mid>"
def exEnd : Term := "<http://ex.org/end>"

def gSeq : Graph := [⟨exAlice, exP1, exMid⟩, ⟨exMid, exP2, exEnd⟩]

/-- The value node of `p1/p2` at Alice is the node two steps away, NOT the node one
step away. A validator that read a sequence path as "either predicate" would put
`exMid` here. -/
theorem a_sequence_path_reaches_two_steps :
    IsValue gSeq (.seq (.pred exP1) (.pred exP2)) exAlice exEnd := by
  refine ⟨exMid, ?_, ?_⟩
  · show (⟨exAlice, exP1, exMid⟩ : Triple) ∈ gSeq
    simp [gSeq]
  · show (⟨exMid, exP2, exEnd⟩ : Triple) ∈ gSeq
    simp [gSeq]

/-- Proved through `mem_valueNodes`, which is the lemma that makes the enumeration
and the specification the same question. -/
theorem a_sequence_path_does_not_stop_halfway :
    ¬ IsValue gSeq (.seq (.pred exP1) (.pred exP2)) exAlice exMid := by
  rw [← mem_valueNodes]
  decide

/-- `sh:zeroOrOnePath` admits the focus node itself, so a focus node with no
outgoing triple at all still has exactly one value node and `sh:minCount 2` fails.
That is the whole content of `core/path/path-zeroOrOne-001`. -/
theorem zero_or_one_admits_the_focus_node :
    valueNodes [] (.zeroOrOne (.pred exP1)) exBob = [exBob] := by decide

/-! ## Axioms, pinned

Every pin below is the footprint `#print axioms` actually reports, not the one the
rest of this repository standardises on. Two of these theorems come out at
`[propext]` alone, a strict subset: they are settled by kernel computation and never
reach `Classical.choice`. Pinning what is true rather than what is expected is the
point of the exercise, and a proof that grew a dependency would break the pin. -/

/-- info: 'Shacl.alice_conforms' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms alice_conforms

/-- info: 'Shacl.bob_does_not_conform' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms bob_does_not_conform

/-- info: 'Shacl.the_report_names_the_failing_node' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms the_report_names_the_failing_node

/-- info: 'Shacl.duplicate_triples_are_counted_once' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms duplicate_triples_are_counted_once

/-- info: 'Shacl.two_distinct_values_break_maxCount_one' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms two_distinct_values_break_maxCount_one

/-- info: 'Shacl.the_evaluator_refuses_rather_than_guessing' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms the_evaluator_refuses_rather_than_guessing

/-- info: 'Shacl.four_point_zero_is_at_most_four' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms four_point_zero_is_at_most_four

/-- info: 'Shacl.four_point_one_is_not_at_most_four' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms four_point_one_is_not_at_most_four

/-- info: 'Shacl.a_string_is_not_greater_than_a_number' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms a_string_is_not_greater_than_a_number

/-- info: 'Shacl.mixed_timezones_are_unordered' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms mixed_timezones_are_unordered

/-- info: 'Shacl.an_unordered_value_does_not_conform' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms an_unordered_value_does_not_conform

/-- info: 'Shacl.a_day_earlier_is_ordered_even_across_the_window' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms a_day_earlier_is_ordered_even_across_the_window

/-- info: 'Shacl.a_blank_node_has_no_length' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms a_blank_node_has_no_length

/-- info: 'Shacl.an_iri_is_as_long_as_it_is_written' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms an_iri_is_as_long_as_it_is_written

/-- info: 'Shacl.a_sequence_path_reaches_two_steps' depends on axioms: [propext] -/
#guard_msgs in
#print axioms a_sequence_path_reaches_two_steps

/-- info: 'Shacl.a_sequence_path_does_not_stop_halfway' depends on axioms: [propext, Classical.choice, Quot.sound] -/
#guard_msgs in
#print axioms a_sequence_path_does_not_stop_halfway

/-- info: 'Shacl.zero_or_one_admits_the_focus_node' depends on axioms: [propext] -/
#guard_msgs in
#print axioms zero_or_one_admits_the_focus_node

end Shacl
