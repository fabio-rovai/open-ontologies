# 0018. Conformance is not truth, and a report must say which shapes ran

**Status:** accepted, 28 September 2026. **Amended 28 September 2026: engine defect E1 is
fixed.** The amendment is in place rather than appended, because a record whose table says a defect
is open is read by the table and not by a footnote. Every sentence that changed is marked, and the
deferral argument is kept rather than deleted, because it is the reason the fix has the shape it has.

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
| E1, impossible date | engine defect | **yes, in the amendment** | `"2026-02-31"^^xsd:date` conformed to `sh:datatype xsd:date` on both evaluation paths |
| E2, two answers to one constraint | engine defect | **yes** | `sh:datatype` under `sh:property` skipped the lexical test the node-shape path ran |
| E3, inference read as assertion | engine defect | **no** | materialising into the inference graph flips a verdict from violated to conforming |
| S1, decorative shape | standard limitation, plus a report gap | report gap only | a shape selecting one node buys a `true` verdict for a shapes graph whose load-bearing shape selected none |
| S2, inconsistent graph | standard limitation | not applicable | a graph that entails everything conforms, because SHACL carries no consistency obligation |
| S3, flat assertion | standard limitation | not applicable | the honest limit case, where nothing in this repository can say the data is false |
| `owl:sameAs` cardinality | NEITHER | not applicable | rejected as an overclaim; see below |
| control, `xsd:byte` | not an attack | not applicable | a probe the existing defence turns into an undetermined verdict rather than a pass |

## Engine defect E1, the impossible date, FIXED in the amendment

SHACL 4.1.2 makes a literal whose lexical form falls outside its datatype's lexical space a violation
of `sh:datatype`. XSD 1.1 section 3.3.9 puts the day-of-month constraint IN the lexical space of
`xsd:date`, so `"2026-02-31"^^xsd:date` is ill formed and must be reported. This engine reported
nothing. `ill_typed_test` in `src/shacl.rs` implemented the lexical test as a regex, and for
`xsd:date` that regex was positional and counted digits:
`^-?[0-9]{4}-[0-9]{2}-[0-9]{2}(Z|[+-][0-9]{2}:[0-9]{2})?$`. Both `2026-02-31` and `2026-13-01` matched
it, so both conformed on both evaluation paths.

The verified evaluator disagreed, and that is what made this a defect rather than a reading of an
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

The Lean development checks the day against `daysInMonth` in `parseDateBody`. The Rust did not. Two
evaluators in one repository gave opposite answers to the same constraint over the same store, and
the one carrying the theorem was the one saying `false`.

### The deferral argument, which was right, and what it settles about the fix

The fix was deferred here on the following reasoning, kept verbatim in substance because it is what
decides the SHAPE the fix ended up having. Closing this means implementing the XSD date lexical space
in Rust. A hand-written regex approximating it fails in the direction that reports violations on
VALID dates, which is the failure mode a conformance suite exists to catch, and
`tests/w3c_shacl_baseline.json` is the ratchet that would catch it. The sequencing rule is therefore
that the baseline is measured before any datatype change lands and re-measured after, and the
measurement is the gate on the fix rather than a formality after it.

That reasoning is why the fix is NOT a cleverer regex.

### What was actually built, and why it is not a regex

Month lengths are regex work and they are in the regex. Two things are not.

The first is 29 February, which is the only day whose existence depends on the year, under the
proleptic Gregorian rule `(y mod 4 = 0 and y mod 100 != 0) or y mod 400 = 0`, over a year XSD does not
bound in length. A regex CAN encode that: divisibility by 400 depends only on a decimal number's last
four digits, so the automaton is finite. It is also unreadable, unreviewable, and wrong in the
direction the paragraph above says is the dangerous one if a single alternative is mistyped.

The second is `xsd:dateTime`, which carries the whole date portion and three more fields the old
regex never ranged at all: `[0-9]{2}:[0-9]{2}:[0-9]{2}` accepted `99:99:99`, and
`[+-][0-9]{2}:[0-9]{2}` accepted a `+99:99` timezone. Closing only the day would have left two more
fields open in the same helper, which is the shape of E2 wearing a different datatype.

So the regex became an UNDER-approximation and is now a PRE-FILTER rather than a verdict. `CERTAIN_DATE`
matches only forms that are certainly in the lexical space: 31-day months to 31, 30-day months to 30,
February to 28, four-digit year, and a timezone up to 13:59. Everything it cannot vouch for is handed
to `xsd_temporal_lex`, which parses the form the way `parseDateBody`, `parseTimeBody` and `parseTzEnd`
in `lean/Shacl/Term.lean` do. The pre-filter earns its place on volume: over a store of dates the
parser sees the ill-formed ones plus 29 February and nothing else.

**Both evaluation paths run it.** E2 below is what happens when they do not, so the node-shape path
and the `sh:property` path were changed together and
`tests/shacl_xsd_date_lexical_test.rs` asserts every fixture through both. Two mutations were run to
prove that is not decoration: fixing only the `sh:property` path and fixing only the node-shape path
each redden the table, and each failure message names the path left behind.

**The century rule is the real one, and 1900 and 2000 are the test.** 1900 is divisible by 100 and not
by 400, so it is not a leap year and `1900-02-29` is reported. 2000 is divisible by 400, so it is, and
`2000-02-29` is not reported. 1600 and 2100 are in the table for the same reason.
`the_leap_rule_is_gregorian_so_1900_is_not_a_leap_year_and_2000_is` exists because a rule of
"divisible by four" passes every other fixture in the file.

**Only the year modulo 400 is ever computed**, folded digit by digit, so a forty-digit year overflows
nothing. That is the one place the Rust differs mechanically from the Lean's arbitrary-precision
`Nat`, and it agrees with it on every input, because 400 is a multiple of both 4 and 100.

### The Lean and the Rust answers, measured rather than asserted

The headline is that the two evaluators in this repository no longer give opposite answers to one
calendar constraint, and that is a measurement. `the_two_evaluators_agree_on_every_xsd_date_fixture`
and `the_two_evaluators_agree_on_every_xsd_date_time_fixture` in
`tests/shacl_verified_tool_test.rs` build ONE store carrying every fixture as its own subject, run
`ShaclValidator::validate` and `validate_verified` over it, and compare the SET of focus nodes each
report names, not the conformance boolean, because a boolean over twenty-nine subjects agrees as soon
as both sides report anything at all. Over `xsd:date` both name the same 18 of 29 subjects; over
`xsd:dateTime` both name the same 7 of 11. Both sets are then compared against the fixture table, so
this is not two engines being wrong together. The Lean verdict is only accepted when it carries
`Shacl.validate_spec`, which the test asserts before comparing anything.

Twelve mutations were run against those two tests and the rest of the corpus. The table is at the end
of this record.

### `xsd:dateTime` was fixed at the same time, and this says so rather than implying it

It carries the same date portion. Leaving it would have meant `"2026-02-31"^^xsd:date` reported and
`"2026-02-31T00:00:00"^^xsd:dateTime` not. The hour, minute, second, fractional-second and timezone
ranges are implemented too, including `24:00:00`, which XSD allows and which denotes midnight starting
the next day, and which therefore admits no other non-zero field.

Two `xsd:dateTime` fixtures are NOT in the table and the reason is recorded rather than left as a
hole. Oxigraph canonicalises the 24-hour form on the way in: `2026-01-01T24:00:00` is stored as
`2026-01-02T00:00:00`, and the ill-formed `2026-01-01T24:00:01` is stored as the well-formed
`2026-01-02T00:00:01`. No verdict over this store is a verdict about the form that was written, in
either direction. `the_store_rewrites_the_twenty_four_hour_forms` pins that measurement, and
`verdict()` in the same file asserts every other fixture survives the store, so a future Oxigraph
that starts rewriting one fails loudly instead of quietly changing what is measured.

### The residual gap, which is narrower and is NOT closed

A negative year, and the year `0000`, are well formed XSD 1.1. `lean/Shacl/Term.lean` declines to read
either, in `yearJudgeable`, because every day-number convention it relies on agrees only on
non-negative years. Measured on 28 September 2026: handed `"-0044-03-15"^^xsd:date`, `oo-shacl`
answers

```
{"status":"undetermined","reason":"the evaluator declined: sh:datatype: the lexical space of
 <http://www.w3.org/2001/XMLSchema#date> is not implemented, so \"-0044-03-15\" cannot be judged
 well formed"}
```

and declines the WHOLE validation run, not the one value. This engine reports no violation over those
forms instead, which is exactly what the old regex did, so nothing about them changed here. The two
answers are not opposite, because `undetermined` is not a verdict, and they are not the same answer
either. `the_lean_evaluator_declines_the_years_this_one_declines` measures both sides so this
paragraph cannot go stale. Closing it would mean per-VALUE entries in `skipped_constraints`, which
turns `conforms` to `null` for any store holding one BCE date, and that is a decision about what a
validation report is for rather than a bug fix. It is stated as open rather than quietly carried.

### The test that recorded the defect, flipped and not weakened

`a_day_that_does_not_exist_conforms_to_sh_datatype_on_both_paths` recorded the defect as an
expectation, and this record said that when the lexical space was implemented that test would FAIL and
the correct response would be to change the expectation and amend this record, never to weaken the
test. That is what happened. It is now
`a_day_that_does_not_exist_violates_sh_datatype_on_both_paths`, because the old name states something
false. Nothing was relaxed: both paths are still asserted, the violation count is still asserted, the
focus-node count is still asserted, and the constraint component is now asserted as well.

`the_lexical_test_runs_and_is_a_strict_under_approximation_of_the_date_space` is renamed to
`the_lexical_test_reaches_the_calendar_and_not_only_the_spelling` for the same reason, and its second
half now measures the direction that would make the fix worse than the defect: a real 29 February, a
five-digit year and a negative year must all still conform. The five-digit year is a defect the old
regex had in that direction and nothing had noticed: `^-?[0-9]{4}-` did not match `12026-01-01`, so a
valid XSD date was reported as a violation. It is not any more.

## Engine defect E2, one evaluator giving two answers, fixed here

`ill_typed_test` had exactly one call site, the node-shape path. The `sh:property` datatype query was
`FILTER(DATATYPE(?val) != <dt>)` and nothing else. Measured over one store before the fix,
`"aldi"^^xsd:integer` reported one violation through `sh:targetObjectsOf` and `conforms: true`
through `sh:property`. Which answer a shapes graph got to one constraint was decided by where its
author wrote the constraint, and almost every real shapes graph writes it under `sh:property`.

The fix splices the same lexical test into the property path.
`an_ill_formed_literal_violates_sh_datatype_wherever_the_constraint_is_written` is the regression
gate. **Amended:** that sentence used to end "this closes the divergence and NOT the wider gap: the
test is still a regex, so E1 stands on both paths". E1 no longer stands on either, and the reason this
section still matters is that the divergence it names is the reason the E1 fix was made to both paths
in one change rather than to the one that was easier to reach.

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

`"300"^^xsd:byte` is outside the byte range and is the W3C suite's own ill-formed case. Oxigraph
0.5.9 did not preserve `xsd:byte`, so the literal arrived as `"300"^^xsd:integer`. Since #335 the
store keeps it as written, but the SPARQL evaluator still reads `xsd:byte` as `xsd:integer`, so
`DATATYPE()` cannot judge it, `datatype_is_indistinguishable_in_store` routes the constraint to
`skipped_constraints`, and the verdict is `null`. An attack surface where every probe succeeds is not a measurement, so this one is
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

**E3 is an open defect, not a design.** It has a fix that is understood and deferred for the reason
above. **Amended:** this paragraph used to read "E1 and E3 are open defects". E1 is fixed; what is
left of it is the narrower residual on years neither evaluator will read, which is stated in its own
section and is not the same defect.

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

**That measurement covers the E2 change only, and the next agent to attempt an E1 fix owes a fresh one
rather than a citation of this one.** The amendment below is that fresh measurement.

## The W3C measurement for the E1 fix

Taken on the E1 branch, over the same suite commit `94d8bc2bd4fc4fdc6f2964d1ec4a892329e05f06`, before
the change and again after it.

BEFORE, at `dab66383`:

```
  total 120, PASS 61 (50.8%), FAIL 15 (12.5%), UNDETERMINED 44 (36.7%), ERROR 0
  partial compliance 61 of 120
  ratchet ok: PASS 61 (baseline 61, +0), FAIL 15 (baseline 15), partial compliance 61 (baseline 61)
```

AFTER, with the calendar lexical spaces implemented on both paths:

```
  total 120, PASS 61 (50.8%), FAIL 15 (12.5%), UNDETERMINED 44 (36.7%), ERROR 0
  partial compliance 61 of 120
  ratchet ok: PASS 61 (baseline 61, +0), FAIL 15 (baseline 15), partial compliance 61 (baseline 61)
```

`tests/w3c_shacl_baseline.json` was regenerated under `OO_W3C_SHACL_UPDATE_BASELINE=1` after the
change and came back BYTE-IDENTICAL to the committed file, so not one of the 120 named outcomes moved
and no fixture needs justifying. The baseline in the commit is the one that was already there.

**And the ratchet is not blind to this change, which is the part a zero delta does not establish on its
own.** The suite carries `sh:datatype xsd:date` over two well-formed values in
`core/property/datatype-001`. A mutation was run that does exactly what the deferral argument was
afraid of, namely a hand-written regex reporting a violation on a VALID date: the thirty-day-month
alternative was narrowed to days 10 to 30 and the parser was taken out of the loop, so `2014-09-01`
became a violation. The ratchet caught it:

```
  1 test(s) that passed in the baseline no longer pass: core/property/datatype-001
  core/property/datatype-001 -> FAIL (results: verdict agrees (conforms=false) but the result set
    differs; 0 missing [], 1 unexpected [(ValidResource, TestShape-dateProperty,
    DatatypeConstraintComponent)])
```

The mutation was reverted and the suite went green again. A zero delta from a gate proved able to move
is a measurement; a zero delta from a gate nobody tried to move is not.

## Every gate in the E1 amendment was proved able to fail

Twelve mutations, each applied, run, recorded and reverted. The three columns count FAILING tests in
`tests/shacl_xsd_date_lexical_test.rs`, `tests/adversarial_shacl_test.rs` and
`tests/shacl_verified_tool_test.rs`, which are 6, 15 and 9 tests green.

| mutation | lexical | adversarial | verified |
| --- | --- | --- | --- |
| drop the century rule from `is_leap_year`, leaving `y mod 4 = 0` | 3 | 0 | 2 |
| make the regex the verdict again, with no parser behind it | 4 | 1 | 3 |
| stop checking the day against the month | 3 | 2 | 2 |
| fix ONLY the `sh:property` path | 3 | | |
| fix ONLY the node-shape path | 4 | | |
| stop ranging the `xsd:dateTime` hour, minute and second | 1 | | 1 |
| stop ranging the timezone offset | 1 | | 1 |
| report the years this engine declines, instead of declining | 1 | | 1 |
| widen the `xsd:integer` lexical test to `^.*$` | 1 | 1 | |
| add a fixture to one of the two copies of the list | | | 1 |
| claim the store keeps the 24-hour form | 1 | | |
| feed the store-survival guard `"1."^^xsd:decimal`, which Oxigraph rewrites | 1 | | |

The single-path mutations are the E2-shaped ones and each failure message names the path that was
left behind, "the node shape path said otherwise" in one direction and "the sh:property path said
otherwise" in the other. The two path mutations differ by one test because the leap-rule test is
written through the property path only.

The last one was not planned. It fired on its own while the fixture table was being written, which is
the only kind of evidence that a guard against a store rewrite is live rather than decorative:

```
the store rewrote 1., so nothing below is about the validator:
{"results":[{"o":"\"1\"^^<http://www.w3.org/2001/XMLSchema#decimal>"}],"variables":["o"]}
```

The adversarial file's own E1 test stays GREEN under the leap-rule mutation, because its two fixtures
are both in 2026 and neither is 29 February. That is a statement about what that file covers rather
than a defect in it, and it is why the leap rule has a fixture table of its own.

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

The mutations worth re-running are these. Restricting the SHACL evaluator to the default graph
reddens the E3 verdict test and the S3 equality test, which is the proof that both are live regression
gates on the inference-graph defect. Removing `"byte"` from `datatype_is_indistinguishable_in_store`
turns the control's `null` into a `false`, which is the proof that the existing defence is doing the
work the control credits it with.

**Amended.** The E1 mutation named here was "tightening the `xsd:date` regex to reject impossible
calendar dates reddens both E1 tests, which is the proof that E1 is a live defect record rather than a
comment". That mutation has been performed for real and is now the fix, so the E1 entry is replaced by
its inverse: loosening the calendar check reddens the same two tests, and the twelve mutations run
against the fix are listed under the amendment's own measurement below.

**One of those mutations found a gate that could not fail, and it was strengthened.** Dropping the
day-versus-month check while keeping the month-range check left
`the_lexical_test_reaches_the_calendar_and_not_only_the_spelling` GREEN, because the E1 fixture holds
two literals and `2026-13-01` alone is enough to make the conformance boolean false. A boolean over
two fixtures cannot say which of them was caught. The test now asserts `violation_count` is 2 and
reddens against that mutation.

**One mutation was inert and that is a note about this codebase rather than about the tests.** Adding
a row to `BUILTIN_RULES` in `src/reason.rs` changes nothing about what the reasoner derives. The
fixpoint hand-codes each rule's firing and reads `BUILTIN_RULES` only for names, arities and the
certificate emitter, so a new row is metadata for a rule nobody runs, and `Rl::ALL` indexes into the
table positionally, so an INSERTED row silently shifts every rule after it onto the wrong metadata.
A mutation that appears to add a rule and changes no behaviour is indistinguishable, at first glance,
from a test that cannot fail. Anyone mutating the rule table should mutate the firing site.
