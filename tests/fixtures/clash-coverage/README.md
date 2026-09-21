# One graph per clash detector, and a near-miss twin for each

`Reasoner::run_full` looks for a contradiction in the closure it reaches. Ten rules of
OWL 2 RL are looked for; the rest are listed in `reason::CLASH_RULES_NOT_DETECTED` and
are not. On 15 September 2026 all 295 RDF files this repository tracks swept **clean**
under all ten, so the shipped corpus exercised the refutation producer not at all, and a
single synthetic round-trip test was the only thing that touched it. That is issue #160.

This directory holds two Turtle files per detector:

* `<rule>.ttl` is the smallest graph that makes exactly that detector fire.
* `<rule>.clean.ttl` is the same graph with the one premise that closes the
  contradiction removed, and it must yield no clash at all.

The twin is the half that makes the fixture mean something. A graph that is refuted
proves only that it is contradictory, not that it is contradictory **for the reason its
author intended**, and a detector that fired on the vocabulary alone would look identical
from the outside. Removing one premise and getting a clean answer is what separates those
two cases.

Each file carries the W3C rule at the top, in the Profiles document's own notation, so a
reader can check the fixture against the standard without leaving the file.

## These are TEST DATA and are not part of the corpus

They were written to make a detector fire. Swept as corpus they would turn "the shipped
corpus sweeps clean under the ten detectors" from a measurement into a statement about
this directory, which is measuring the ruler. So three corpus walkers exclude this
prefix, in the pattern `tests/fixtures/horn-coverage/` set:

    tests/projection_monotonicity_corpus_test.rs
    tests/reason_rl_coverage_test.rs
    tests/lean_certificate_test.rs

`the_fixtures_are_tracked_and_are_not_corpus` in `tests/clash_detector_coverage_test.rs`
asserts that each of those three still names the prefix, because an exclusion that has
silently gone stale is worse than none: the sweep would report contradictory fixtures as
if they were real ontologies.

## What makes this coverage rather than ten more synthetic tests

Adding ten tests beside the one the issue complains about would leave the same hole:
nothing would notice if a detector stopped firing on a shape it used to catch. So the
list of detectors is **read out of `find_clashes` in `src/reason.rs`** rather than typed
in the test, and the acceptance criterion is that removing a detector turns exactly one
test red. Both ways of removing one were run before this landed:

| what was done to `cls-com`               | what went red                                          |
| ---------------------------------------- | ------------------------------------------------------ |
| its guard broken so it never fires        | `each_fixture_fires_its_own_detector_and_no_other`, naming `cls-com.ttl` |
| the detector deleted from the source      | `every_detector_in_the_source_has_a_fixture_named_for_it`, as an orphaned fixture |

In both cases the other three tests stayed green, which is what "exactly one" means.

## Adding one

Name the file after the rule exactly as `find_clashes` constructs it (`<rule>.ttl`), add
`<rule>.clean.ttl` beside it, and put the W3C rule text in the header comment. The test
discovers both by name; there is no list to update. A detector added without a fixture
turns `every_detector_in_the_source_has_a_fixture_named_for_it` red on the day it lands.
