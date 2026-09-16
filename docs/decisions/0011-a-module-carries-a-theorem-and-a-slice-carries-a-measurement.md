# 0011 · A module carries a theorem, a slice carries a measurement

- **Status**: implemented, both features · `src/module_extract.rs`
  (`onto_module_extract`) and `src/conservativity.rs` (`onto_conservative_check`, and the
  `conservativity` block of `onto_plan`) · both reuse `src/closure_diff.rs` rather than
  recomputing a closure, a certificate or a verdict word, so there is still exactly ONE place in
  the crate where `checked` is produced · gated by `tests/module_extract_test.rs` (9) and
  `tests/conservativity_test.rs` (10), plus 11 unit tests in `src/module_extract.rs` and 3 in
  `src/conservativity.rs`
- **Written**: 2026-09-15
- **Related**: decision 0007 (a slice preserves a conclusion, or it does not), whose machinery this
  reuses unchanged and whose story it completes; decision 0002 (an inference carries a
  certificate), whose verdict discipline the conservativity report follows; decision 0003 (a rule
  is data), whose `entailed_under_supplied_rules` habit is why the conservativity verdict is
  `conservative_under_rule_table` and never `conservative`

## On the number

0011, not 0009 and not 0010. Both are taken in working trees on other branches, and 0009 is taken
TWICE (`a-clash-is-a-condition-and-twelve-of-seventeen-have-one` and
`a-translation-between-logics-carries-its-satisfaction-condition`), so the collision this repository
already has is not one to join. 0004 is the known gap decision 0007 records and is left alone.

## The problem

Decision 0007 built the right instrument for the wrong half of the question. It can tell you, for
any slice, exactly which conclusions it lost, with the rule and the blocking premises, and it can
do it with a Lean-checked certificate behind the source closure. What it cannot do is produce a
slice that loses nothing, because `onto_segment_retrieve` is a hop-bounded neighbourhood walk and a
neighbourhood has no theorem behind it. The honest summary of the state before this change was: we
measure the damage well and we have no way to avoid it.

Worse, the measurement has a failure mode the decision record itself names. `closure(G) \ closure(P)`
grows with `|G|`, so minimising it means retrieving more. A user who wants "the part of the ontology
that matters for these terms" and is handed a loss report is being asked to tune a number whose
gradient points at retrieving the whole file.

The second gap was in the lifecycle. `onto_plan` reports added classes, removed classes, blast
radius and a risk score, and every one of those is about SHAPE. A change that adds one
`rdfs:domain` triple adds no class, removes nothing, has a blast radius of zero and scores `low`,
while retyping every existing individual of that property. There was no tool in the repository that
could see it, and the engine that computes exactly this difference was already sitting in
`closure_diff` pointed the other way round.

## Decisions

1. **A module is a different object from a slice, and the tool names say so.** `onto_module_extract`
   computes a syntactic locality module: the smallest subset of the axioms the locality test can
   justify, such that `M ⊨ α iff O ⊨ α` for every axiom `α` over the signature. That is a coverage
   theorem, so there is nothing to measure afterwards; `onto_closure_diff` stays for slices, which
   have no theorem and therefore still need the measurement. The two are not competitors and the
   descriptions cross-reference each other.

2. **The theorem is CITED and is not machine-checked, and the report says so in a field.** Syntactic
   locality is Cuenca Grau, Horrocks, Kazakov and Sattler, JAIR 31 (2008). Nothing under `lean/` is
   about locality, so `ModuleReport` names the paper, carries
   `guarantee_is_not_machine_checked`, and `the_module_report_never_names_a_lean_theorem` asserts
   over the SERIALISED report that the strings `OOCert.`, `OwlLean.` and `Fol.` do not appear in it.
   That test is the one that stops a future edit from borrowing a word the work has not earned,
   which is decision 0002's rule applied to somebody else's theorem.

3. **What CAN be checked is the consequence, and it is, on a file this repository ships.**
   `verify_module` reasons the whole ontology and the module to a fixpoint under one rule table and
   reports every conclusion over the signature closure the module does not reach, which must be
   zero. It is `closure_diff` unchanged, called with the module as the projection, because a second
   implementation of "what did this subset lose" would be a second place for the answer to be
   wrong. `the_guarantee_holds_on_the_pizza_ontology` runs it over
   `benchmark/reference/pizza-reference.owl`: a 238-axiom, 510-triple module of a 1,345-axiom,
   2,332-triple ontology, zero conclusions lost over the signature out of 2,583 examined, nothing
   unclassified, and 60 annotation triples dropped by design and counted separately.

4. **An axiom that cannot be classified is INCLUDED, and the count is in the payload.** Any `M'` with
   `M ⊆ M' ⊆ O` still has the coverage property, so over-including costs size and under-including
   costs the theorem. Every recovery failure resolves the same way: an unrecognised predicate in the
   RDF, RDFS, OWL or XSD namespaces, an unrecognised blank-node structure, a malformed `rdf:List`, a
   class expression nested past the depth bound. `owl:sameAs`, `owl:differentFrom` and
   `owl:AllDifferent` join them for a different reason: they mention no class and no property name,
   so no replacement can make them tautologies. All of it is counted under
   `included_conservatively` and named under `unclassified_axioms`, because "the module is small"
   and "the module is small because half the file was unreadable" must not render the same.

5. **An unrecognised predicate in a USER namespace is a property assertion, and that is not the same
   decision.** If every unknown predicate were kept, the module would be the ontology and the
   guarantee would be free. `ex:a ex:p ex:b` is an `ObjectPropertyAssertion` under OWL 2 semantics
   and replacing `ex:p` by the universal property makes it a tautology, so it is `⊤`-local and may
   go. `an_unrecognised_user_predicate_is_a_property_assertion_and_can_be_dropped` pins the
   distinction from the other side, so the conservative rule cannot quietly widen into "keep
   everything".

6. **A datatype is not an external class name.** Replacing `xsd:integer` by `⊥` makes
   `∃hasAge.xsd:integer` look `⊥`-equivalent and drops the axiom, and the module still parses, so
   nothing looks broken. Data ranges are neither `⊥`-equivalent nor `⊤`-equivalent under any
   signature, detected by namespace, by `rdfs:Literal`, and by `owl:onDatatype` /
   `owl:datatypeComplementOf` / `owl:withRestrictions` on a blank node.

7. **The extraction pass iterates to a fixpoint, and that is the soundness condition rather than an
   optimisation.** The working signature grows with every axiom taken, and an axiom that was local
   against `Σ` stops being local once a name it mentions has been pulled in. A single sweep would
   drop `a p₁ b` from a module that kept `p₁ ⊑ p₂`, losing `a p₂ b` over a signature containing
   `p₂`, `a` and `b`. `the_top_pass_iterates_rather_than_sweeping_once` is the case, on the `⊤` pass
   where the first sweep finds every axiom local.

8. **Two places where the OWL 2 direct semantics and this engine's rule table disagree, both
   resolved towards the rule table.** A declaration (`X rdf:type owl:Class`) is logically vacuous in
   OWL 2 and is a PREMISE of OWL 2 RL's `scm-cls`, so it is kept whenever the declared term is in
   the working signature. `X rdf:type owl:Thing` is a tautology in OWL 2 and the rule table does not
   REGENERATE it, so dropping it removes a member of the closure; it is read as a declaration for
   the same reason. The second one was not predicted: it was found by running the verification over
   the pizza ontology, which reported five entailment losses over the signature that were
   `Germany rdf:type owl:Thing` and four like it.

9. **Annotation losses are COUNTED, never filtered.** The extractor treats `rdfs:label`,
   `rdfs:comment` and the other OWL 2 annotation properties as vacuous, so the module drops them and
   the verification sees them as differences over the signature. They are partitioned into
   `lost_over_signature_annotation_only` rather than removed from the scan, and the pizza test
   asserts that number is NON-ZERO, because a partition that silently swallowed everything would
   make every module verify clean. The vacuity is checked rather than assumed: an annotation
   predicate the ontology gives a domain, a range, a superproperty or an equivalent is read as a
   property assertion instead, and `annotation_predicates_treated_as_vacuous` says in the payload
   which ones were not.

10. **The negative test is the whole point of shipping this.** A module and a neighbourhood look
    identical from outside, so `a_naive_signature_slice_drops_an_entailment_the_module_keeps` builds
    the slice a sensible retriever produces (every triple MENTIONING a signature term), shows it is
    FIVE triples against the module's four, and shows it has lost `Cat ⊑ LivingThing` because
    `Mammal ⊑ Animal` mentions neither signature term and the chain runs through it. Bigger and
    wrong, against smaller and right. Without that test the feature is indistinguishable from a
    rename.

11. **Conservativity is `closure_diff` run in the extension direction, and it reuses it literally.**
    Take `G` to be `base ∪ extension` and `P` to be the base, and `closure(G) \ closure(P)` is
    exactly the set of consequences the extension introduced, with the rule and the blocking
    premises already attached by the existing code. Restricting it to triples every name of which
    the base already used gives the conservativity finding. No closure, certificate or verdict word
    is recomputed.

12. **The base is skolemised under its OWN prefix before the extension is merged in.** Two graphs
    skolemised separately under the same prefix collide: the map is `_:label ↦ prefix + label`, so
    `_:b0` in the base and `_:b0` in the extension would become one IRI naming two different
    existentials. `skolemise_with_prefix` is the shared refactor that makes this expressible, and
    the discriminator ends in `/`, which cannot occur in an N-Triples blank node label, so no
    default-prefix skolem IRI can spell a base-prefix one. The payoff is measured rather than
    claimed: `blank_nodes_in_the_base_do_not_disarm_the_gate` puts a restriction on both sides and
    asserts the monotonicity gate still runs.

13. **The verdict is a WORD, and the boolean beside it is null when undecided.** There is no field
    called `conservative`. `conservativity_verdict` takes one of
    `conservative_under_rule_table`, `not_conservative_under_rule_table`,
    `undecided_scan_truncated`, `undecided_not_an_extension` and
    `undecided_engine_soundness_violation`, and `conservative_under_rule_table` is `Option<bool>` so
    "we could not tell" can never render as `false`, which a reader would take for a finding.
    `the_payload_never_carries_a_bare_conservative_flag` asserts over the serialised report. The
    last word exists because the monotonicity gate rides along for free: the base is a subset by
    construction and the rule table is monotone, so a conclusion the base reaches and the extension
    does not is an engine soundness bug, it is checked FIRST, and folding it into
    `not_conservative` would send it to the ontology's author instead of the engine's.

14. **The fragment travels in the payload, not only in the docs.** `what_this_is_not` states, as a
    field, that this is conservativity with respect to the Horn rule table named in `rule_table`;
    that deductive conservativity in a description logic is ExpTime-complete for `EL`,
    2ExpTime-complete for `ALC` (Ghilardi, Lutz and Wolter, KR 2006) and UNDECIDABLE for `ALCQIO`
    (Lutz, Walther and Wolter, IJCAI 2007); and that model conservativity is undecidable already for
    `EL` (Lutz and Wolter, JSC 2010). The asymmetry is stated in the same field: a new consequence
    found IS a real change to what this engine derives over the old names, while finding none
    establishes only that this rule table derives nothing new.

15. **A non-conservative extension is a FINDING and never an error.** Changing what the ontology says
    about existing terms is often the intended change. Exit 1 means "there is something to look at",
    exit 2 means "the question could not be answered", and `finding_not_an_error` says so in the
    payload. Every row names the rule and `premises_the_base_lacked`, so the decision is made on the
    derivation rather than on the verdict word.

16. **Delta or replacement is ASKED FOR, never guessed.** A delta that restates one triple of the
    base and a replacement that dropped everything else are the same bytes, and the two answers are
    opposite. `onto_plan` fixes the mode to `replacement` because that is what it receives;
    `onto_conservative_check` defaults to `delta` and refuses an unknown word by name.

17. **The plan is where the feature lives, and it is opt-in and says when it did not run.**
    `check_conservativity: true` adds the block; without it the block is still present and reads
    `ran: false` with the reason, because a missing block and a clean block look the same to a
    dashboard. It is never fatal: a check that fails to run leaves `conservativity.skipped` set and
    still returns the plan, since a diagnostic that can block a plan is a diagnostic people turn
    off.

## What was found on the way

1. **The pizza ontology is not in the namespace it is cited by.**
   `benchmark/reference/pizza-reference.owl` declares
   `xml:base="https://raw.githubusercontent.com/owlcs/pizza-ontology/refs/heads/master/pizza.owl"`,
   not the `co-ode.org` IRI everything in the literature uses. A signature spelled the usual way
   selects nothing, the module comes back empty, and the empty module verifies perfectly clean. The
   `signature_not_in_ontology` field exists because of this: a module over a misspelled IRI is
   correct and useless, and that is the most expensive way to learn about a typo.

2. **`X rdf:type owl:Thing` is a tautology the rule table does not regenerate.** Decision 8 above.
   Found by running the verification rather than by reading the code.

3. **A verification capped at `max_rows` is a SAMPLE.** The first version reported "0 lost over the
   signature" on pizza while examining 200 of 2,588 differences. `not_examined` now rides in the
   verification report and `scan_rows` bounds the scan separately from the rendering, so a clean
   result over a truncated scan cannot read as a clean result.

4. **The MCP server's own instructions string claimed 114 tools in one sentence and 112 in the
   next.** `tests/readme_claims_test.rs` checks four phrasings and neither of those two matched the
   second shape, so a stale number had been sitting inside the string the server tells every client
   about itself. Corrected to 116 with the rest.

## What this does not claim

- **The locality theorem is not proved here.** It is a citation. The measured verification is a
  consequence of it under ONE rule table and is not a substitute for it.
- **The module is not minimal.** Syntactic locality gives the smallest set the syntactic test can
  justify. A minimal module is a different and harder problem, and `guarantee` says so in the
  payload rather than leaving "smallest" to be read as minimality.
- **Conservativity here is not conservativity in a description logic**, and decision 14 is the long
  form of that sentence.
- **`module_fraction` is not a score.** A module is not better for being smaller; it is correct or
  it is not, and its size is what the signature costs. The field is documented as a reading aid,
  and unlike `coverage_ratio` it has no gaming direction because nothing is tuned on it.

## Still not done

- **No Horn arm.** Like `closure_diff` (decision 0007's own "still not done"), neither feature reads
  a supplied rule table. The conservativity verdict would then have to be
  `conservative_under_supplied_rules`, which is decision 0003's word and is simply not written.
- **`owl:hasKey` is decided on the class only.** `HasKey(C, (p))` with an empty `p` is vacuously
  true and could be local, which is what the OWL API does. The more conservative test is used
  instead, so some `hasKey` axioms are in modules that need not be.
- **`≥n R.C` for `n ≥ 2` is never `⊤`-equivalent here.** `≥2 ⊤.⊤` fails in a one-element domain, so
  the bound is left alone rather than reasoned about. Conservative, and it costs a handful of
  axioms on ontologies that use qualified cardinalities heavily.
- **The module verification has a blank-node blind spot.** The module is serialised with the store's
  own blank node labels and the closure diff skolemises the source, so a conclusion carrying a blank
  node or a Skolem IRI has no term whose name means the same thing on both sides and cannot be
  decided against the signature. `not_decided_blank_node_bearing` counts it rather than hiding it,
  the pizza test asserts it is non-zero so the coverage of the clean result can be read, and
  extracting the module from a skolemised store would empty it. That is decision 0007's
  `not_compared` bucket in a new place, and the same NP-complete reason not to close it by matching.
- **No CI leg installs Lean for these tests either.** Both run without a checker and report
  `engine_opinion` for the certificate; nothing skips, because neither feature's claim depends on
  the checker.
