# 0018. Conformance is not truth, and a report must say which shapes ran

**Status:** accepted, 28 September 2026.

## The thesis

A SHACL conformance verdict is a statement about a shapes graph and a data graph. It is not a
statement about whether the data is true. That sentence is uncontroversial when written down and is
routinely ignored when a green verdict reaches a register, an auditor or a pipeline, because the word
`conforms` reads as an endorsement of the content rather than as the narrow claim it is.

This is the dual of reward hacking on a verifier. Where a policy learns to satisfy a reward model
without doing the task, a data author writes a graph that satisfies a shapes graph without the facts
the shapes were written to require. This engine is the place to run that experiment because its SHACL
Core evaluator is mechanised in Lean and `Shacl.validate_spec` proves the decision procedure agrees
with the specification in both directions, so an attack here cannot be waved away as a validator bug.
Some of what follows IS a validator bug, and those are kept in their own bucket precisely so that the
rest cannot be dismissed with them.

The corpus is `tests/adversarial_shacl_test.rs`, fourteen tests over six fixtures plus one rejected
candidate and one control.

## Two buckets, and they are never blurred

An ENGINE DEFECT is something this engine does that the specification and its own Lean evaluator do
not. It is a bug and it belongs on a fix list.

A STANDARD LIMITATION is a case where the validator is RIGHT and the data is still false, or where
the data cannot be shown false at all. It is a finding about what a conformance verdict means. Nothing
in `src/shacl.rs` should change because of one.

Blurring the two is how this kind of work loses its credibility. A limitation dressed as an exploit
invites the reply that the tool is simply being used wrongly, and that reply is correct.

| fixture | bucket | fixed here | what it shows |
| --- | --- | --- | --- |
| E1, impossible date | engine defect | **no** | `"2026-02-31"^^xsd:date` conforms to `sh:datatype xsd:date` on both evaluation paths |
| E2, two answers to one constraint | engine defect | **yes** | `sh:datatype` under `sh:property` skipped the lexical test the node-shape path ran |
| E3, inference read as assertion | engine defect | **no** | materialising into the inference graph flips a verdict from violated to conforming |
| S1, decorative shape | standard limitation, plus a report gap | report gap only | a shape selecting one node buys a `true` verdict for a shapes graph whose load-bearing shape selected none |
| S2, inconsistent graph | standard limitation | not applicable | a graph that entails everything conforms, because SHACL carries no consistency obligation |
| S3, flat assertion | standard limitation | not applicable | the honest limit case, where nothing in this repository can say the data is false |
| `owl:sameAs` cardinality | NEITHER | not applicable | rejected as an overclaim; see below |
| control, `xsd:byte` | not an attack | not applicable | a probe the existing defence turns into an undetermined verdict rather than a pass |

## Engine defect E1, the impossible date, recorded and deliberately not fixed

SHACL 4.1.2 makes a literal whose lexical form falls outside its datatype's lexical space a violation
of `sh:datatype`. XSD 1.1 section 3.3.9 puts the day-of-month constraint IN the lexical space of
`xsd:date`, so `"2026-02-31"^^xsd:date` is ill formed and must be reported. This engine reports
nothing. `ill_typed_test` in `src/shacl.rs` implements the lexical test as a regex, and for
`xsd:date` that regex is positional and counts digits:
`^-?[0-9]{4}-[0-9]{2}-[0-9]{2}(Z|[+-][0-9]{2}:[0-9]{2})?$`. Both `2026-02-31` and `2026-13-01` match
it, so both conform on both evaluation paths.

The verified evaluator disagrees, and that is what makes this a defect rather than a reading of an
ambiguous specification. Run on 28 September 2026:

```
$ ./lean/.lake/build/bin/oo-shacl validate date_data.nt date_shapes.nt
{"status":"verdict","conforms":false,"shapes":1,"data_triples":4,
 "results":[{"focus":"<http://ex.org/cert1>","path":"<http://ex.org/expiresOn>",
   "value":"\"2026-02-31\"^^<http://www.w3.org/2001/XMLSchema#date>","sourceShape":"_:p1",
   "sourceConstraintComponent":"<http://www.w3.org/ns/shacl#DatatypeConstraintComponent>"},
  {"focus":"<http://ex.org/cert2>","path":"<http://ex.org/expiresOn>",
   "value":"\"2026-13-01\"^^<http://www.w3.org/2001/XMLSchema#date>","sourceShape":"_:p1",
   "sourceConstraintComponent":"<http://www.w3.org/ns/shacl#DatatypeConstraintComponent>"}],
 "theorem":"Shacl.validate_spec",
 "checker":{"name":"oo-shacl",
   "self_sha256":"79fae33fe5451036c50f1004b32e26eed7edba707c8ccae27de177a0427d851c",
   "toolchain":"4.33.1"}}
```

The Lean development checks the day against `daysInMonth` in `parseDateBody`. The Rust does not. Two
evaluators in one repository give opposite answers to the same constraint over the same store, and
the one carrying the theorem is the one saying `false`.

**The fix is deferred, and the reason is the W3C baseline.** Closing this means implementing the XSD
date lexical space in Rust. A hand-written regex approximating it fails in the direction that reports
violations on VALID dates, which is the failure mode a conformance suite exists to catch, and
`tests/w3c_shacl_baseline.json` is the ratchet that would catch it. The sequencing rule is therefore
that the baseline is measured before any datatype change lands and re-measured after, and the
measurement is the gate on the fix rather than a formality after it.

That measurement was taken on this branch and is recorded below. It covers the E2 change, which did
land. It does not cover an E1 fix, which did not, and the next agent to attempt one owes a fresh
measurement rather than a citation of this one.

`a_day_that_does_not_exist_conforms_to_sh_datatype_on_both_paths` records the defect as an
expectation. When the lexical space is implemented that test will FAIL, and the correct response is
to change the expectation and amend this record, never to weaken the test.

## Engine defect E2, one evaluator giving two answers, fixed here

`ill_typed_test` had exactly one call site, the node-shape path. The `sh:property` datatype query was
`FILTER(DATATYPE(?val) != <dt>)` and nothing else. Measured over one store before the fix,
`"aldi"^^xsd:integer` reported one violation through `sh:targetObjectsOf` and `conforms: true`
through `sh:property`. Which answer a shapes graph got to one constraint was decided by where its
author wrote the constraint, and almost every real shapes graph writes it under `sh:property`.

The fix splices the same lexical test into the property path.
`an_ill_formed_literal_violates_sh_datatype_wherever_the_constraint_is_written` is the regression
gate. This closes the divergence and NOT the wider gap: the test is still a regex, so E1 stands on
both paths.

## Engine defect E3, the engine's own conclusion satisfying the constraint that asked for one

`Reasoner::run_with_target` with `InferenceTarget::Inferred` writes into
`https://open-ontologies.org/graph/inferred`, which decision 0001 exists to keep separate from
assertion. `GraphStore::triples_in_scope` honours that under `ReadScope::AllGraphs` and cites TCB-8
in its own comment. `GraphStore::sparql_select_scoped` does not: for `AllGraphs` it reaches
`sparql_select_union`, which is every graph in the store, and every data-side query the SHACL
evaluator emits goes through that path.

The fixture is a claimant writing a subclass axiom about itself, with nobody auditing anything.
`rdfs9` and then `cls-hv1` conclude the evidence triple, the validator reads it, and a shapes graph
requiring evidence reports a pass over a graph whose asserted content fails. The oracle is stated two
ways over the same store: the report before materialisation is `conforms: false`, and reloading
`serialize("turtle")` output, which drops the inference graph by design, gives `conforms: false`
again.

Nothing in the report says any of this happened. The scope block still reads `whole-store` and no
field distinguishes a satisfied constraint from one satisfied by a derived triple.

**Left unfixed here on purpose.** Narrowing the dataset a validation run reads changes the public
answer for every store that has been materialised. That is a decision about what a validation report
is FOR, not a bug fix, and it needs to settle what a caller who deliberately validates a materialised
graph should get. It is stated as open rather than quietly carried.

## Standard limitation S1, the decorative shape, and the one line of defence that shipped

SHACL says a shape selecting no focus node conforms, full stop, so the evaluator is doing exactly
what the Recommendation asks and there is no validator bug here. This engine already goes FURTHER
than the Recommendation and answers `conforms: null` when nothing at all matched. The gap is that
the escalation is decided from a TOTAL: `nothing_matched` compares `focus_nodes_total` against zero,
so one decorative shape selecting one node restores a `true` verdict for a shapes graph whose
load-bearing shape selected none.

`tests/shacl_vacuous_target_test.rs` pins that behaviour deliberately and this branch does not touch
it. **`conforms` keeps SHACL's meaning.** A gate that wants to refuse a vacuous load-bearing shape
needs a word of its own rather than a second writer on that field.

What shipped is `focus_nodes_by_target`: one row per target declaration carrying the number
`count_focus_nodes` already returned for it. The number was computed and the report summed it away,
and a sum cannot be un-added. `unmatched_shapes` already named the empty shape, so the evidence was
technically in the report, but a reader had to know to look for a key that is empty on every ordinary
run. Now "which shapes actually ran" is answered by reading one field.

Per TARGET and not per SHAPE, deliberately. A shape carrying two target declarations is walked twice
and these rows are what the walk did. Adding them up per shape would produce a number that is not a
count of distinct focus nodes.

**This fixture is only an attack when the same party writes the data and the shapes.** Where a
verifier writes the shapes graph and a publisher supplies the data, a target class that matches
nothing is the verifier's own bug and the publisher gained nothing by it. The adversarial reading
applies to self-certification, where a publisher ships a shapes graph alongside its data and a
downstream consumer runs it. That is a common enough arrangement to be worth defending, and it is
narrower than "SHACL can be fooled". The defence works in both readings, because a verifier who
misaimed a target also wants to be told the shape checked nothing.

## Standard limitation S2, the inconsistent graph

Two register entries asserted both `owl:sameAs` and `owl:differentFrom` conform to a one-LEI-per-entity
shape. SHACL is a validation language and carries no consistency obligation, so neither evaluator is
wrong. The falsity is logical and a different tool in the same repository finds it: `eq-diff1` is one
of the ten clash rules this engine detects and one of the rules `OOCert.RefuteConditions` holds a
semantic condition for. An inconsistent graph entails every sentence, including the negation of
anything the shapes graph was written to establish, so a conformance verdict over it certifies
nothing. That is the sense in which the data is false, and it is checkable rather than argued.

## Standard limitation S3, the limit case, which is where the honesty is

The claim is asserted flatly. Every constraint is satisfied, the graph is consistent, no target is
vacuous and no literal is ill formed. **Nothing in this repository can say this data is false**, and
the test does not pretend otherwise. It asserts conformance and then asserts the ABSENCE of every
oracle this file has: the satisfying triple is asserted rather than derived, no rule concluded it, and
there are no clashes.

What it does assert positively is a measurement that needs no oracle. The report the gate produces
for this store is EQUAL, field by field, to the report it produces for the E3 store where the
satisfying triple was the engine's own conclusion. Two graphs, one written by a person and one
assembled by a claimant so that a rule would write it, and one report.

That is the case for binding a constraint to a SOURCE rather than to a triple. It is the strongest
statement this corpus can make without claiming to know what is true in the world, and it is weaker
than an exploit. Saying so is the point.

## The candidate that was rejected, and why it is recorded

Under `owl:sameAs` two register entries are one entity holding two LEIs, which a register with a
one-LEI-per-entity rule forbids. SHACL counts value nodes by term, so each node holds one and both
conform. `lean/Shacl/Spec.lean` states `maxCount` over term-distinct value nodes, so the verified
evaluator agrees, and it is right to: the Recommendation says nothing about `owl:sameAs`.

This is NOT filed as a working attack. No oracle in this repository reports the merged cardinality
either: `eq-rep-s`, `eq-rep-p` and `eq-rep-o` are absent from `RULES_EVALUATED`, so `owl:sameAs`
propagates no triple, and `prp-fp` is absent too, so no functional-property clash is looked for.
`prp-fp` is not even on `CLASH_RULES_NOT_DETECTED`, which is the gap actually worth recording. The
checkable statement is therefore about COVERAGE and not about truth. Calling it an exploit would mean
claiming a falsity nothing here can demonstrate.

It is kept in the file, in its own section, because a rejected candidate is evidence about how the
line between the buckets was drawn.

## The control, and the bound it actually has

`"300"^^xsd:byte` is outside the byte range and is the W3C suite's own ill-formed case. Oxigraph does
not preserve `xsd:byte`, so the literal arrives as `"300"^^xsd:integer`,
`datatype_is_indistinguishable_in_store` routes the constraint to `skipped_constraints`, and the
verdict is `null`. An attack surface where every probe succeeds is not a measurement, so this one is
kept beside the failures rather than in a passing-cases file.

The bound is one-directional and the test now says so. A consumer reading for a PASS is protected,
because the verdict is not `true`. A consumer reading for a FAILURE is not: `onto_extend` stops its
pipeline on `parsed["conforms"] == false` at `src/server.rs`, and `Value::Null` does not equal
`Value::Bool(false)`, so this probe walks through that gate exactly as a clean run would. The
`Commands::Shacl` path in `src/main.rs` uses `output_result`, which never exits non-zero, so the CLI
is the same story.

An earlier draft of this test asserted that "no consumer can read it as a pass". That statement is
false about this repository and has been removed. The test now reads `src/server.rs` and fails if
that gate is ever widened, so the claim is pinned to the source rather than to a comment.

## What is DESIGNED AND NOT BUILT

Only one thing shipped: `focus_nodes_by_target`. Everything else in the defence is a design.

**The modality vocabulary is not built.** The intended shape is that a rule declares itself `must`,
`must-not` or `may`, and that the modality filters before precedence rather than being encoded as a
severity. Nothing in `src/shacl.rs` knows the words. No parser reads them, no report carries them and
no verdict depends on them.

**The provenance binding is not built.** S3 is the argument for binding a constraint to the SOURCE of
a satisfying triple rather than to the triple, so that an asserted claim and a laundered one produce
different reports. Nothing computes that. `the_report_cannot_tell_an_asserted_claim_from_a_laundered_one`
asserts that the two reports are still EQUAL, so the day the binding is built that test fails, which
is the intended way to find out.

**E1 and E3 are open defects, not designs.** They have fixes that are understood and deferred for the
reasons above.

A reader who takes away "this engine now detects assurance laundering" has read this document
backwards. It detects none of it. It reports one more number than it used to.

## What this corpus does not demonstrate

Every fixture here was written to make a point. None of them was found in the wild, no register or
customer graph was examined, and nothing in this record is evidence that anyone has done any of this
deliberately. The claim is that the failures are REACHABLE and cheap to construct, which is a
statement about the gate rather than about anybody's conduct.

No fixture shows that a conformance verdict can be turned into a truth verdict. That is not a gap
waiting to be closed; it is what validation is. The useful question is which of the ways a green
verdict can mislead are the engine's fault, and the two buckets above are the answer to that.

S2 and S3 in particular prove nothing about SHACL that a careful reading of the Recommendation would
not already tell you. They are here because the report they produce is indistinguishable from the
report a sound graph produces, and because a corpus of only the surprising cases would be a
rhetorical selection rather than a measurement.

The `owl:sameAs` candidate demonstrates a coverage gap in this engine's rule table and NOT a false
verdict. It is recorded as rejected for exactly that reason.

## The W3C measurement

The ratchet in `tests/w3c_shacl_conformance_test.rs` was run on this branch with the E2 change in
place, over suite commit `94d8bc2bd4fc4fdc6f2964d1ec4a892329e05f06`:

```
  total 120, PASS 61 (50.8%), FAIL 15 (12.5%), UNDETERMINED 44 (36.7%), ERROR 0
  partial compliance 61 of 120
  ratchet ok: PASS 61 (baseline 61, +0), FAIL 15 (baseline 15), partial compliance 61 (baseline 61)
```

`tests/w3c_shacl_baseline.json` is unmodified against `main`. Regenerating it under
`OO_W3C_SHACL_UPDATE_BASELINE=1` and diffing produced no difference at all, so not one of the 120
named outcomes moved. The E2 fix is therefore measured as neutral on the suite rather than assumed to
be, which is the whole reason for taking the measurement before touching datatype evaluation again.

## Every gate in this corpus was proved able to fail

A test that cannot fail is worse than no test, because it is read as evidence. Each of the fourteen
tests was mutation-proved: the thing it depends on was broken, the test was run, the failure was
recorded, and the mutation was reverted. That exercise changed the corpus, in the places below.

**A mutation that reported the running total on every per-target row left
`the_report_gives_every_target_its_own_focus_node_count` GREEN.** On the S1 shapes graph the vacuous
target happens to be walked first, so the partial sums are 0 and 1 and the per-target counts are also
0 and 1, and the test could not tell a per-target count from a running total. The fixture now carries
a third shape, so the three counts are 1, 2 and 0 and no running total can impersonate all three
rows. The mutation reddens against the strengthened test.

**The control asserted something false about this repository.** Its failure message claimed that no
consumer could read an undetermined verdict as a pass, and `onto_extend` does exactly that. The test
was renamed from `no_attack_gets_past_a_datatype_the_store_cannot_preserve` to
`a_datatype_the_store_cannot_preserve_yields_no_verdict_rather_than_a_pass`, which is what it proves,
and the consumer claim is now pinned to a read of `src/server.rs` that fails if the gate moves.

The mutations worth re-running are these. Tightening the `xsd:date` regex to reject impossible
calendar dates reddens both E1 tests, which is the proof that E1 is a live defect record rather than
a comment. Restricting the SHACL evaluator to the default graph reddens the E3 verdict test and the
S3 equality test, which is the proof that both are live regression gates on the inference-graph
defect. Removing `"byte"` from `datatype_is_indistinguishable_in_store` turns the control's `null`
into a `false`, which is the proof that the existing defence is doing the work the control credits it
with.

**One mutation was inert and that is a note about this codebase rather than about the tests.** Adding
a row to `BUILTIN_RULES` in `src/reason.rs` changes nothing about what the reasoner derives. The
fixpoint hand-codes each rule's firing and reads `BUILTIN_RULES` only for names, arities and the
certificate emitter, so a new row is metadata for a rule nobody runs, and `Rl::ALL` indexes into the
table positionally, so an INSERTED row silently shifts every rule after it onto the wrong metadata.
A mutation that appears to add a rule and changes no behaviour is indistinguishable, at first glance,
from a test that cannot fail. Anyone mutating the rule table should mutate the firing site.
