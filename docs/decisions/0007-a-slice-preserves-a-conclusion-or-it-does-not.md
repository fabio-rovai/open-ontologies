# 0007 · A slice preserves a conclusion, or it does not

- **Status**: implemented, both forms · `src/projection_entailment.rs` (goal-directed, `preserve`,
  `graph_projection_entailment_check`) and `src/closure_diff.rs` (goal-free, `closure-diff`,
  `onto_closure_diff`) · every preserved-and-derived goal carries a sub-certificate `oo-cert`
  accepted, so `OOCert.certificate_sound` is named and no new Lean was written · the monotonicity
  differential runs on EVERY call and is disarmed, loudly, when its antecedent does not hold ·
  `coverage_ratio` is DEMOTED, not deleted · gated by `tests/lean_projection_entailment_test.rs`,
  `tests/closure_diff_test.rs`, `tests/projection_monotonicity_corpus_test.rs`,
  `tests/preserve_cli_test.rs` and `tests/gate_demonstration_test.rs`, the last of which feeds every
  gate the input built to trip it and PRINTS what the tool said
- **Written**: 2026-09-14
- **Related**: decision 0002 (an inference carries a certificate), whose format and checker this
  reuses unchanged; decision 0003 (a rule is data), whose verdict discipline and whose
  `entailed_under_supplied_rules` word this reuses verbatim; decision 0005 item 7 (the differential
  exported from a reasoned store), whose defect is re-armed here in a new place and guarded

## On the number

0007, not 0004 and not 0006. `0004` is a known gap: decision 0002 references it, it is absent from
`docs/decisions/`, and decision 0006 already records that the description-logic model certificate it
points at is implemented while only the record is missing. Writing someone else's decision record is
not this work's to do, so the gap is left rather than filled silently. `0006` is taken by the model
certificate record, which exists in a working tree on another branch and will land; taking the
number here would collide on merge.

## The problem

`src/projection_check.rs` audits a retrieved slice by reporting dropped predicates, dropped objects
and a `coverage_ratio` of projected triples over source triples. It is the primitive
`onto_segment_retrieve` is advertised to pair with, and it is the wrong measure, dangerously so,
because it looks like assurance.

A slice at 99% coverage can have dropped the one triple an answer depends on. A slice at 60%
coverage can preserve every conclusion that matters. Worse, the failure is silent and inverted: the
number goes UP as the projection gets larger, so a system tuned on it learns to retrieve more rather
than to retrieve the right thing. A retrieval-augmented answer grounded in such a slice can be FALSE
while every displayed metric is green.

That is not a hypothetical. Measured on `benchmark/reference/pizza-reference.owl`, a file this
repository ships: the whole ontology minus the single triple `NamedPizza rdfs:subClassOf Pizza`
scores `aggregate_coverage_ratio: 1.0` and `ok: true` over the twenty-three named-pizza seeds, while
`Veneziana rdfs:subClassOf Food`, a conclusion the source derives, is gone. The number cannot see
the damage at all. The converse, from the same file: a three-triple slice scores 0.00128 and
preserves every claim asked of it, each with a Lean-checked certificate. Both are pinned as tests.

## Decisions

1. **The property is entailment preservation, and it is asked per claim.** For a source `G`, a
   projection `P` and a set `Q` of ground positive triples an answer rests on, the question is not
   how much of `G` survived in `P`. It is whether, for each `q` in `Q`, `P` entails `q` exactly when
   `G` does, decided under ONE pinned rule profile.

2. **Four cells, and they are four different answers.** `G` yes / `P` yes is *preserved*, the only
   cell that can carry a certificate. `G` yes / `P` no is *lost by the projection*, the retrieval
   finding. `G` no / `P` yes is either a projection that is not a subset or an unsound engine, and
   is never a retrieval finding. `G` no / `P` no is `ungrounded_in_source`: the claim has no support
   in the graph at all, the generator invented it, and a better retriever will not fix it. A tool
   that reports the last two as "not preserved" sends every investigation to the wrong team, because
   a lossy retriever and a hallucinating generator have opposite fixes. `ungrounded_in_source` does
   not mean the claim is false: RDF entailment is open-world and this engine covers 29 of OWL 2 RL's
   78 rules, so it means "not derivable from `G` under this profile" and nothing stronger.

3. **Membership is decided over the certificate FILES, not over an in-memory closure.**
   `Reasoner::run_full` runs twice with `materialize = false` and a `certificate_dir`, and every
   goal is looked up in `asserted.tsv` and `derivations.tsv` read back off disk. The Lean checker
   only ever sees those two files. If membership were decided against a set the checker never reads,
   the engine could report "derived" over a certificate that does not contain the derivation, and
   the verdict would be unchecked while wearing the checked word. Deciding over the artefact the
   checker reads makes that class of bug impossible rather than tested against.

4. **A preserved goal's certificate is a SUB-CERTIFICATE of the run's, and no new Lean was
   written.** The line concluding `q` is taken from `derivations.tsv`, its premises are walked to
   fixpoint, and the lines are emitted in ascending original-file order beside `P`'s own
   `asserted.tsv`. `oo-cert` exiting 0 over that pair is exactly "`P` entails `q`", machine-checked,
   under `OOCert.certificate_sound` unchanged. The ordering is valid and it is provable from the
   emitter rather than hoped for: `run_full` computes a round's conclusions from `triple_set` as it
   stood at the START of the round and inserts them only afterwards, so every premise of a line was
   asserted or concluded strictly earlier. `oo-cert` rejects a premise that is "neither asserted nor
   derived earlier", so the claim is load-bearing and
   `file_order_is_a_topological_order_of_the_derivations` pins it over the whole pizza ontology
   rather than leaving it as a comment.

5. **The free differential is not optional, and it is DISARMED rather than trusted.** OWL RL is
   monotone and `P` is a subset of `G`, so `closure(P) ⊆ closure(G)`. A conclusion of `P` that is not
   a conclusion of `G` is a SOUNDNESS BUG IN THE ENGINE, reported at `STOP_THE_LINE` with exit 2,
   never filed under "other". It is worth nothing unless its antecedent is established, and there
   are three conditions that fail on real input: the projection is not a subset (COMPUTED every run,
   never inferred from provenance, because a retriever that normalises an IRI or adds a tidy
   `rdf:type owl:Class` declaration produces a non-subset that looks like a faithful slice); blank
   nodes could not be matched; or either run stopped at the iteration cap rather than a fixpoint,
   which makes the larger closure a LOWER BOUND and manufactures spurious `P`-only entailments. Each
   disarming is REPORTED with its own reason, because a green `violations: []` under a disarmed gate
   means "we did not look" and the two must never render the same.

6. **`coverage_ratio` is demoted, not deleted, and the label travels in the payload.**
   `check_projection_loss` keeps running and its output is nested under `coverage_proxy` with
   `is_a_warrant: false` and one sentence a reader cannot miss, carried as a FIELD rather than a doc
   comment so a renderer that walks only the data still emits it. The same sentence went into the
   `graph_projection_lossy_check` tool description, because a tool description is the text an agent
   actually reads when choosing, and that is where the trap was living unlabelled. Removing the
   number would lose a cheap signal; leaving it unlabelled would keep the trap.

7. **Three kinds of word, never collapsed.** `preserved_checked` names
   `OOCert.certificate_sound`. `preserved_under_supplied_rules_checked` names
   `OOCert.horn_certificate_sound`, carries the table's sha256, and is never shortened to the plain
   word, because a rule reading "every supplier is compliant" produces certificates that check green
   for ever. `preserved_asserted` is exact-string membership and names no theorem, so a lookup can
   never inflate the checked count. `preserved_unchecked` is the engine's opinion about its own
   output and carries the install line and the exact `lake exe oo-cert` command.
   `an_unchecked_result_never_prints_the_checked_word` asserts over the SERIALISED report rather
   than over the enum, because the enum is where the discipline is easy and the serialisation is
   where it leaks, and it carries no skip guard, so it is the one gate a machine without Lean still
   enforces.

   The enum is now where the discipline is enforced as well as easy. `GoalVerdict::
   PreservedChecked` and `PreservedUnderSuppliedRulesChecked` each carry a `verdict::Certified`,
   which has a private field, no constructor, and exactly one producer in the crate:
   `CheckerRun::accepted`, which returns `None` unless a process this crate spawned exited zero.
   `CheckerStatus::Accepted` carries one too, so the acceptance those verdicts are read off cannot
   be fabricated either, and `warrant` is now read OUT of the token rather than written beside it,
   which makes `OOCert.certificate_sound` as unspeakable on an unchecked path as the verdict is.
   `GoalVerdict` deliberately does not implement `Deserialize`: parsing `"preserved_checked"` out
   of a report is not the same act as earning it, and a derive would be a public constructor for
   the certified state. The serialisation guard above stays, because the type says nothing about
   what `Serialize` writes, and that is exactly the half it was written to cover.

8. **A negative carries no certificate in this layer, ever.** `q ∉ closure(P)` is the engine's
   opinion bounded by a rule table implementing 29 of OWL 2 RL's 78 rules, so the word is
   `lost_under_profile_unchecked`. Printing "the projection does not entail `q`" flat would do in the
   negative direction exactly what `coverage_ratio` does in the positive: present an unchecked
   judgement in the grammar of a warrant.

9. **A rejected sub-certificate stops the line and downgrades to nothing.** If `oo-cert` exits 1 on a
   slice this module assembled, the defect is in the extractor or in the emitter's premise order.
   The tempting code is `.unwrap_or(PreservedUnchecked)`, which converts a defect in this code into a
   marginally weaker verdict nobody will ever investigate, and it is precisely the treatment
   `tools/shacl_differential.py` refuses to give a `FALSE_CLEAN`. Exit 2, `what:
   "certificate_rejected"`, and `a_rejected_slice_never_downgrades_to_preserved_unchecked` asserts
   the non-downgrade explicitly.

10. **Refusals are counted in the headline.** A blank node in a goal is an existential, not a claim.
    "No supplier is sanctioned" is not a triple, and mapping absence-from-the-closure onto such a
    claim would convert an open-world absence into a closed-world fact with a certificate stapled to
    it, which is the laundering this work exists to attack, produced by the anti-laundering tool.
    Both are refused BY NAME, the closed-world message points at `onto_shacl`, and the count sits in
    the headline so a refusal cannot shrink the denominator unnoticed.

11. **An empty goal set is an error, not a green report.** Every aggregate over zero goals is
    trivially perfect, which is decision 0006 item 8's empty-problem shape, and "all my goals were
    refused" is the realistic way to arrive at zero. The error says how many were refused.

12. **Both forms ship, and they share one implementation.** The goal-directed form is the online one,
    for auditing an answer at query time. The closure-diff form supplies no goals and computes
    `closure(G) \ closure(P)`, which is what an offline audit of a retrieval STRATEGY needs.
    `src/closure_diff.rs` reuses the certificate index, the checker runner, the subset precondition,
    the skolemiser and the differential from `src/projection_entailment.rs`, so there is exactly one
    place in the crate where each verdict word is produced.

13. **The closure-diff headline is `lost_in_projection_vocabulary`, and its gaming direction is in
    the payload.** `|closure(G) \ closure(P)|` grows with `|G|` and is nearly all of `closure(G)` for
    any real slice, so minimising it means retrieving more: the same perverse gradient wearing a
    better name. A lost entailment is dangerous exactly when every one of its terms occurs in `P`,
    because then an answer grounded in `P` is a claim over those terms. That number has its own
    gaming direction, since shrinking `terms(P)` shrinks it, and the report says so as a field, not only
    in the docs, because the docs are not what a dashboard renders.

## What the monotonicity differential found

Run for real over the shipped corpus (`tests/projection_monotonicity_corpus_test.rs`): each ontology
skolemised, sliced by `onto_segment_retrieve` from its twelve busiest subjects at two hops, both
closures certified and diffed.

| | |
|---|---|
| ontologies swept | 275 |
| monotonicity gate ARMED | 275 |
| gate disarmed | 0 |
| source certificates the Lean checker ACCEPTED | 275 |
| **monotonicity violations** | **0** |
| incompleteness warnings | 0 |
| conclusions lost, total | 117,105 |
| of them over terms the slice itself mentions | 899 |
| excluded (over the 1 MB sweep cap, or unparseable) | 9, each named |
| wall clock | 41.6 s |

Nothing was found, and that is the result. It is worth more than it looks because the gate was
ARMED on all 275 rather than disarmed: subsethood was computed and held, both runs reached a
fixpoint, and every source closure carried a certificate `oo-cert` accepted. A sweep in which the
gate is disarmed everywhere establishes nothing, so the two counts are reported separately and the
test fails if the gate is disarmed on more than half the corpus.

## What was found on the way, and fixed

1. **`onto_segment_retrieve` emitted slices that do not parse.** Every term was wrapped in angle
   brackets unconditionally, so a blank-node `owl:Restriction` superclass came out as `<_:b0>`,
   which is not a legal IRI. Since issue #93 `load_turtle` collects all or nothing, so ONE
   restriction superclass made the ENTIRE slice unparseable, and `check_projection_loss` then
   reported `projection_parses: false` with `aggregate_coverage_ratio: 0.0` and no diagnosis. The
   two tools are advertised as a pair, and the pair was broken for essentially every OWL ontology in
   this repository. Measured on `benchmark/ontoaxiom/.../pizza.ttl` and
   `benchmark/reference/pizza-reference.owl`. Blank nodes are now written bare, the angle-bracket
   trim no longer eats the closing bracket of a typed literal's datatype IRI, and
   `a_slice_with_a_blank_node_superclass_still_parses` pins it.

2. **`check_projection_loss` reported `ok: true` on a projection holding MORE than the source.**
   `coverage_ratio` clamps with `.min()`, so a seed whose slice carries triples the source does not
   read exactly 1.0 with empty dropped lists. A hallucinating retriever, or a slice of a different
   graph, scored a clean bill of health. The clamp stays, because an unclamped "ratio" above 1.0 is
   not a ratio; the surplus is now named in `seeds_with_surplus` and `ok` requires it to be empty.

3. **Literal canonicalisation happens on STORE INSERT, not at parse.** Measured: oxigraph's parser
   returns `"01"^^xsd:integer` exactly as written, and the same literal comes out of `all_triples`
   as `"1"^^xsd:integer`. A goal routed through the parser alone therefore matches nothing in a
   certificate and is reported LOST, which is a false loss with a green-looking pipeline round it. Goals are
   now round-tripped through a real `GraphStore`, one triple per named graph so a set cannot collapse
   duplicates or shift the caller's pairing, and both `as_written` and `goal` are printed.

4. **`GraphStore::canonicalize_blank_nodes` is the wrong tool and is now pinned as such.** RDFC-1.0
   labels are a function of the WHOLE graph. Measured on `pizza-reference.owl`, taking a genuine
   subgraph: canonicalising both sides separately and diffing reports 630 triples of the subgraph as
   absent from the source, out of 2,276, while comparing the store's own labels reports 0. It is
   correct for "are these two graphs isomorphic", which is not the question, and it looks
   principled, which is worse. `skolemise` is the answer, and
   `canonicalising_both_sides_separately_is_worse_than_doing_nothing` stops a future implementer
   reaching for the method that is already in the repository.

5. **The FNV-1a 64 digest was written with the prime `0x1000000001b3`, one zero too many.** Nothing
   would have looked broken, because a digest is only ever compared against itself. It was caught by
   pinning the published test vectors instead of a round trip through the function, which would have
   agreed with any constant at all.

## What this does not claim

- **Nothing about the negative side.** `lost_under_profile_unchecked` is the engine's opinion,
  bounded by a rule table covering 29 of OWL 2 RL's 78 rules. The escalation the design allows for is
  a machine-checked countermodel over `P` from the model-certificate layer, whose word would be
  `lost_not_entailed_under_unproved_translation` and which rides on `OwlLean.adequacy` plus a
  Rust-to-Lean correspondence that decision 0005 item 2 states is pinned by tests and NOT proved.
  That layer is not on this branch, so the escalation is **not built**, and the default word stands.
- **`OOCert`'s entailment is not datatype-aware.** `"1"^^xsd:int` and `"1"^^xsd:integer` are two
  distinct goals, and a `preserved_checked` on one says nothing about the other. In the `limits`
  block of every report, not only here.
- **Blank-node subgraph matching is not solved and will not be.** Deciding whether one graph is a
  subgraph of another up to blank-node renaming is simple entailment: NP-complete in general,
  polynomial only when the target is ground. The `not_compared` bucket is a consequence of that
  complexity, documented as such so nobody tries to close it, and skolemising the source before the
  retriever sees it is what empties it.
- **The proxy's own limits are named, not fixed.** `neighbourhood_pairs` is subject-only, so an
  inbound-edge loss is invisible to it, and its SPARQL carries `LIMIT 1000`, so a seed with more
  than a thousand outbound triples has its ratio computed against a truncated source. Both are
  reported (`truncated_seeds`, `known_limits`) and left to their own change.

## Still not done

- **No CI leg installs Lean for these tests.** They skip loudly through `common::skip_unless` and the
  existing `lean` job's `OO_REQUIRE_FIXTURES=1` makes that skip fatal; wiring the new test files into
  that job is a one-line change to `.github/workflows/ci.yml` and is done, but nothing has run it on
  a runner yet.
- **The corpus sweep uses a 1 MB cap** rather than the certificate test's 4 MB, because it does two
  reasoner runs and a retrieval per file. Four ontologies are excluded by it and are named in the
  output. Raising it is a question of CI budget, not of correctness.
- **`closure_diff` has no Horn arm.** A supplied rule table is supported in the goal-directed form
  only. The closure-diff form would need to read `horn.tsv` on both sides and run `oo-horn check`,
  which is the same shape and is simply not written.
