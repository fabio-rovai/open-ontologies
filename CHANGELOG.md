# Changelog

All notable changes to Open Ontologies are documented here.

## [Unreleased]

### Added
- **A module carries a theorem; a slice carries a measurement.** `onto_module_extract` computes a
  syntactic locality module over a signature: `⊥`, `⊤` or the iterated `⊥⊤*`, the standard OWL API
  algorithm. Unlike `onto_segment_retrieve`, whose loss then has to be measured by
  `onto_closure_diff`, a locality module CANNOT lose an entailment over its signature, and the
  difference is a theorem rather than a metric (Cuenca Grau, Horrocks, Kazakov and Sattler, JAIR 31,
  2008). That theorem is CITED and is not machine-checked, nothing under `lean/` being about
  locality, so the report names a paper and never a Lean theorem, and
  `the_module_report_never_names_a_lean_theorem` asserts that over the serialised report. What can
  be checked is the consequence: `verify_out_dir` reasons the ontology and the module to a fixpoint
  through the existing closure diff and reports every conclusion over the signature the module does
  not reach, which must be none. Measured on `benchmark/reference/pizza-reference.owl`: 238 of 1,345
  axioms and 510 of 2,332 triples, nothing unclassified, and zero of the 2,583 differences lost
  over the signature. The locality test is written per
  axiom type and an axiom that cannot be classified is INCLUDED rather than dropped, since any `M'`
  with `M ⊆ M' ⊆ O` keeps the coverage property; the kinds and counts are in the payload so a small
  module and an unreadable one cannot render the same. The negative test is the one that matters:
  a naive slice keeping every triple that MENTIONS a signature term is five triples against the
  module's four and has lost `Cat ⊑ LivingThing`, because the chain runs through an axiom that
  mentions neither signature term. Running the verification found two places where the OWL 2 direct
  semantics and this engine's rule table disagree, both now resolved towards the rule table, and one
  of them (`X rdf:type owl:Thing`, a tautology the table does not regenerate) was not predicted.
- **`onto_conservative_check`, and the same check inside `onto_plan`.** Does adding these axioms
  change anything the ontology already said, over the names it already used? It is
  `closure_diff` run in the extension direction: source `base ∪ extension`, projection `base`, and
  the difference restricted to the old signature, with the rule and the premises the base lacked
  already attached by the existing code. A non-conservative extension is a FINDING and never an
  error, and every plan now carries a `conservativity` block that either has the answer or says it
  did not look, because a missing block and a clean block read the same to a dashboard. This is the
  only part of a plan that is about MEANING: one `rdfs:domain` triple retypes every existing
  individual of that property while adding no class, removing nothing, and scoring `low` risk.
  THE FRAGMENT IS NAMED IN THE PAYLOAD. What is computed is conservativity with respect to the Horn
  rule table the engine evaluates. It is not deductive conservativity in a description logic
  (ExpTime-complete for `EL`, 2ExpTime-complete for `ALC`, undecidable for `ALCQIO`) and not model
  conservativity (undecidable already for `EL`), so the verdict field is `conservativity_verdict`
  with four words, the boolean beside it is `conservative_under_rule_table` and is null when
  undecided, and there is deliberately no field called `conservative`. See decision 0011.
- `projection_entailment::skolemise_with_prefix`, so two graphs that are about to be merged can be
  skolemised without `_:b0` on both sides becoming one IRI naming two different existentials.
  `skolemise` is unchanged for every existing caller.
### Fixed
- **Reasoning and SHACL read every graph, so a bi-temporal store was judged over a state that
  never existed (#108).** `Reasoner::run` read the store through `GraphStore::all_triples`, which
  iterates every quad and drops the graph name, and `ShaclValidator::validate` read it through
  `sparql_select_union`, which makes the default graph the union of every graph. Neither had an
  argument that could change it. On the module's own example — `:HEK293` adherent until
  2026-05-01 in `:g1`, suspension from 2026-05-01 in `:g2`, two half-open periods that MEET and
  share no instant — `onto_temporal_conflicts` filed a correction and reported zero
  contradictions, while `onto_reason` reported a `cax-dw` disjointness clash and `onto_shacl`
  reported a `sh:maxCount` violation. Both verdicts were about a moment that never occurred.

  This is the failure mode a certificate cannot catch, and that is why it was worth the size of
  the change. Run with `certificate_dir`, the clash above wrote a `refutation.tsv` that `oo-refute
  check` ACCEPTS: a machine-checked `unsatisfiable_under_disjointness` over two versions that
  never coexisted. The certificate is not wrong. `OOCert.certificate_sound` quantifies over the
  triples in `asserted.tsv`, they were all in the store, every derivation followed, and the Lean
  checker is telling the truth. `asserted.tsv` was the wrong graph, and no checker in this
  project can see that far.

  Three things changed. **The scope is a value.** `crate::graph::ReadScope` says which graphs a
  run may read; `GraphStore::triples_in_scope` and `sparql_select_scoped` read exactly those and
  no others, by dataset selection rather than by copying, so cross-graph joins keep working and a
  scoped run costs no extra memory. The named-graph restriction is applied to a query's available
  named graphs as well as to its default graph, so a `sh:sparql` constraint cannot name its way
  back out. **The scope is refused rather than guessed.** `onto_reason` and `onto_shacl` take
  `valid_at`, `as_of` and `all_versions`; over a store that describes its named graphs with the
  temporal vocabulary, a run with none of them is refused with a message naming the way out. A
  store that uses no temporal vocabulary is unaffected under every request and answers exactly as
  it did at 1.3.0. **This is a behaviour change for a bi-temporal store**: a call that used to
  return a verdict now returns an error until the caller says which question it is asking.
  **The scope is evidence.** Every report carries `scope`, and every certificate directory now
  also gets `scope.tsv` (`oo-scope/1`), one line per graph read and one per graph deliberately
  not read. No Lean checker reads it; it is a record a person or a script can check against the
  store, and its value is that the selection is no longer invisible.

  Four decisions the issue asked for, taken and visible in the manifest: membership is exactly the
  snapshot's `in_scope` set; the default graph is in wholesale, because it holds the schema (and
  therefore, admitted rather than hidden, the validity metadata lands in the reasoner's closure —
  `docs/trusted-computing-base.md` item 13); a graph holding this engine's own materialised
  inferences is dropped and the drop is recorded, which closes the across-run inference leak of
  TCB item 5 for scoped runs; and NO run over a versioned store MATERIALISES, scoped or
  `all_versions`, and it says so rather than dropping the flag. There is nowhere in such a store
  that a conclusion can be written without becoming an axiom of every snapshot: the default graph
  is in scope at every instant because it is timeless, and so is an inference graph carrying no
  validity description. A snapshot's conclusions held at one instant and would be read at all of
  them; an `all_versions` closure was drawn from a state that held at no instant, and writing that
  in is this same leak arriving through the exit rather than the entrance. The CLI and the batch
  runner make any invocation carrying a scope argument a dry run; over MCP, pass
  `materialize: false`. A snapshot
  that selects no graph returns `conforms: null` with its own reason instead of conforming
  vacuously. A temporal description written into a named graph, where `Temporal::validities` cannot
  read it, is refused rather than silently treated as an undescribed and therefore timeless store.
  Scoped runs are not available for `owl-dl`, whose tableaux path still reads
  `GraphStore::all_triples`, and asking for one is refused rather than ignored.
  `onto_extend` chains both tools, takes no scope arguments of its own, and inherits the refusal:
  over a versioned store it returns the error rather than a pipeline report, and the snapshot has
  to be run through the two tools directly. `onto_reason_incremental` has no snapshot form either
  — it reads the union through `sparql_select_union` and materialises into the default graph —
  and is refused over a versioned store unless `all_versions: true` says the union was meant;
  leaving it ungated would have made the gate on `onto_reason` a suggestion.
  `tests/temporal_scope_test.rs` holds the reproduction and every gate.
- **A front-page claim was gated by a test that ran nowhere, and now runs in CI.** `README.md`
  reported that the cross-kernel differential "reports zero divergent rows", and
  `tests/cross_kernel_differential_test.rs` does require exactly that and fails on any row at
  all. No workflow ran it. Its `skip()` wants lake AND Poly/ML: the `build` job has neither, so
  it skipped there, and the `lean` job, which has lake, invoked the file from no leg. A skipped
  test reports `ok`. `docs/ci-gates.md` listed this under "What is still open" and decision 0008
  carried a "Not run in CI" bullet, so it was known, written down, and unable to fail.

  What CI installs is NOT Isabelle, and the old reasoning that it would have to be is what kept
  this open. `isabelle/driver/oo_horn_generated.ML` is committed, so the missing piece was a
  compiler for it: Isabelle publishes Poly/ML as a 39 MB component, pinned here by SHA-256,
  cached, and the same 5.9.2-2 build that generated that file, so CI and the developer machine
  run one compiler and not two. Not `apt-get install polyml`, which was checked rather than
  assumed because `prover9` having no apt candidate on noble broke this job once already: Ubuntu
  noble carries 5.7.1 and ships `/usr/bin/poly` and `libpolymain.a` with no `libpolyml.a`, so
  there is nothing to link against, and both `poly_dir()` and `build_native.sh` now say so by
  name rather than failing in the linker. `isabelle/build_native.sh` was macOS-only, looking for
  `libgmp.dylib` and an app bundle under `/Applications`; it now takes `POLYDIR`, finds gmp in
  either shape or as the bundled versioned copy, adds `-lpthread -lm -ldl` on Linux only because
  macOS has no libdl to pass, and prints the linker's stderr on failure instead of discarding it.

- **The `build` job could not fail on a failing test, and this branch proved it by accident.**
  `cargo test -- --nocapture 2>&1 | tee cargo-test.log` in a `run:` block is `bash -e` without
  `pipefail`, so the step's exit status was `tee`'s. The first CI run of this work carried a test
  that failed: the Linux leg printed `test result: FAILED. 2 passed; 1 failed` in its own log and
  reported SUCCESS, while the Windows leg, whose default shell keeps cargo's exit code, failed on
  the same test. The job that runs the whole suite on two platforms was not a gate on one of them.
  `shell: bash` and `set -o pipefail`, which is what the skip-counter step below it already does
  and for the same reason.

- **A skip in CI is now a failure even in a file no job invokes.** `common::skip_unless` already
  panicked under `OO_REQUIRE_FIXTURES=1` and twenty legs already set it, so no second variable is
  added: a second name for an existing mechanism is a second thing to forget. What that mechanism
  cannot see is a test file that runs nowhere, which is what this defect was.
  `tests/ci_gate_coverage_test.rs` requires every `tests/*.rs` with a skip path to be strict in
  some workflow or on a written exception list with its reason, refuses an exception for a file
  that no longer skips or that is strict after all, holds `docs/ci-gates.md`'s table against what
  the workflows actually say, and fails on any test printing the `SKIPPED_FIXTURE:` marker by hand
  rather than through the helper. Five files are excepted, each because CI deliberately does not
  carry what they need. It does NOT make every file strict: a contributor without Poly/ML must
  still be able to run `cargo test` and get a pass, and can.

- **The corpus figures in the documents are derived rather than typed.** Three documents quoted
  this corpus and none of them had measured the tree they were committed to. The 47-of-1,718 and
  the zero that replaced it were measured on the shallow corpus; the 54-of-2,075 was measured on a
  branch cut before the binding fix. The two were authored eleven minutes apart and merged
  separately, so the combination nobody ran was the deep corpus under the fixed checker, which is
  the combination the README asserted a result for. Measured now, in CI: **2,075 certificates, 402
  accepted by both, 1,134 rejected by both, 539 unparseable on both, and ZERO divergent**, with 332
  rows carrying a binding decision 0008 refuses and both kernels refusing every one. The shape of
  the shallow result repeats exactly: 1,080 plus 54 is 1,134, and the accepted and unparseable
  counts did not move. `the_corpus_exercises_prefix_visibility_at_depth` now formats the corpus
  size, its depth, its fan-out and its prefix-visibility count out of the measurement and fails if
  `README.md` or `docs/reasoning-systems-inventory.md` says anything else, so a growing corpus
  fails until the prose is corrected and a correction that reaches one document out of two fails
  as well. `isabelle/README.md`'s per-bucket counts are deleted rather than corrected: that file
  already said the numbers belong in the test output, three sections before writing them out.

### Changed
- **A certified verdict is now unconstructible without the evidence, so laundering one is a
  compile error rather than a test failure.** The discipline that `model_checked` requires
  `checker_exit == 0`, that `preserved_checked` requires an accepted `oo-cert` run, and that
  `checked` requires a zero exit code off a Lean binary, used to be carried by comments saying
  "the ONE place this word is produced" and by tests that walked a serialised report looking for
  a checked word with nothing behind it. Those tests were right and they still run. They were
  also incomplete by construction: a new module that formatted `"model_checked"` in a new place
  passed every one of them until somebody noticed and pointed a guard at it. `src/verdict.rs`
  now holds the whole certified vocabulary. `Certified` has a private field and no constructor,
  `CheckerRun` has private fields and one constructor that SPAWNS the checker and reads its exit
  status, and `CheckerRun::accepted` is the only function in the crate that returns a `Certified`
  and returns `None` on any non-zero exit. Every certified variant carries one:
  `FolVerdict::ModelChecked`, `GoalVerdict::PreservedChecked`,
  `GoalVerdict::PreservedUnderSuppliedRulesChecked`, `ClosureVerdict::Checked`,
  `Warrant::Checked`, `CheckerStatus::Accepted` and `OwlReading`. The theorem name travels
  inside the token rather than beside it, so `OOCert.certificate_sound`,
  `OOCert.horn_certificate_sound` and `Fol.satisfiable_of_check` are unspeakable on a path that
  did not run a checker either. Six `compile_fail` doctests run under `cargo test` and fail if
  any of that stops being true; the induced error is E0451, "field `theorem` of struct
  `Certified` is private". None of these types implements `Deserialize`, because parsing
  `"preserved_checked"` out of somebody else's JSON is not the same act as earning it and a
  derive would be a public constructor for the certified state; reports are read back as
  `serde_json::Value`, which is what every caller already did. The WIRE FORMAT is unchanged:
  `Serialize` is hand-written to emit the same bare string the derive emitted, verified
  byte-for-byte against a binary built from the previous commit over `preserve`, `fol-model`,
  `closure-diff`, `reason`, `reason --rules`, `rules-import` and three MCP `tools/call`
  responses.
- **The MCP server no longer advertises eight tools it cannot serve.** `onto_plugin_list`,
  `onto_plugin_call`, `onto_embed`, `onto_hnsw_build`, `onto_search`, `onto_similarity`,
  `onto_import_schema` and `onto_sql_ingest` each have a body that is
  `#[cfg(not(feature = ...))] { return "Compiled without X feature" }`, so on a default build
  every call to one of them fails for every input. They were listed in `tools/list` anyway: a
  client read a description about embeddings or a Postgres URL, called the tool, and got an
  error that had nothing to do with what it asked. `toolfilter::remove_unavailable` now drops
  their routes before the operator's own filter and independently of its mode, so a default
  build advertises 106 of the 114 registered tools and says which eight are missing and which
  Cargo feature brings each one back. The server's instructions string used to state two
  hand-typed totals, 114 and 112, neither of which was the number the router advertised, and to
  promise that the eight WERE advertised; it is formatted from `tool_router.list_all()` now, so
  there is no literal left to go stale.

- **The bridge to the specification was prose. It is a theorem.** `lean/OOCert/Conforming.lean`
  formalises an OWL 2 RDF-Based interpretation in core Lean as `OOCert.Interpretation`: the parts
  of RBS Table 5.1, with `IR` as the carrier type so that the table's `IP ⊆ IR`, `IC ⊆ IR`,
  `ICEXT(x) ⊆ IR` and `IEXT(x) ⊆ IR × IR` hold by typing, plus `IS`, `IL`, the blank-node
  assignment of RDF 1.1 Semantics section 5.1, and `ICEXT` and `IC` as definitions per section 9.
  Truth is section 5's clause with its `I(p) is in IP` conjunct, stated as the specification
  states it. `OOCert.Conforming` collects thirty fields, twenty-nine of them a quoted cell and the
  thirtieth Table 5.1's `IR ≠ ∅`: Table 5.8 in both directions, Tables 5.9, 5.12 and 5.13 forward only, Table 5.6's
  three restriction rows with the outer `if-then` and the consequent equality, the typing rows of
  Tables 5.2 and 5.3, Tables 5.4 and 5.5 forward, and five axiomatic triples.
  `OOCert.Conforming.toW3C` proves that every such interpretation carries `W3C` at full strength,
  `OOCert.Conforming.toW3CModel` carries that to `W3CModel` including proving that every `Chain G`
  list is a semantic sequence in the specification's sense, and
  `OOCert.certificate_conforming_sound` is the sentence `certificate_w3c_sound`'s docstring says
  of itself that it is not: a checked certificate's conclusions hold in every conforming
  interpretation of the asserted graph. Its statement mentions no structure of this repository's
  invention.
  **The five `IP` memberships the old bridge assumed are discharged, not moved.** Each is a field
  asserting one axiomatic triple whose own predicate is the wanted term, so the truth clause hands
  the fact back as the triple's first conjunct: `rdf:type rdf:type rdf:Property .`,
  `rdfs:Datatype rdfs:subClassOf rdfs:Class .`,
  `rdfs:isDefinedBy rdfs:subPropertyOf rdfs:seeAlso .`,
  `rdf:type rdfs:domain rdfs:Resource .` and `rdf:type rdfs:range rdfs:Class .`. Taking them at all
  is licensed by RBS Definition 4.2, RBS section 4.2 and RDF 1.1 section 9, quoted in the file.
  **One of the five was previously sourced to the wrong triple.** `W3C.lean`'s table gave
  `I(rdfs:range) ∈ IP` the source `rdfs:range rdfs:domain rdf:Property .` "whose truth puts its own
  predicate in `IP`"; that triple's predicate is `rdfs:domain`, so its truth gives
  `I(rdfs:domain) ∈ IP` and reaching `rdfs:range` from it needs a further step the entry omits. The
  fact is true and the route was wrong; the correction is recorded at both files rather than
  applied silently.
  `lean/OOCert/ConformingWitness.lean` exhibits a two-element interpretation satisfying every one
  of those conditions, every RDF axiom of RDF 1.1 section 8 and every RDFS axiomatic triple of
  section 9 (fifty-four, checked by `decide`), and three identity rows of RBS Table 5.2 that
  `W3CWitness.lean`'s `live` records itself as violating. Table 5.8 is alive there in both
  directions and CONSTRAINING: `IEXT(I(rdfs:subClassOf))` holds three of four possible pairs, the
  missing one excluded by the forward direction and one of the present ones forced by the backward
  direction, both checked. The eleven OWL-vocabulary extensions are empty there and
  `two_dead_fields` says so as a theorem; `two_violates_these_known_rows` records the Table 5.2 row
  it breaks. The file also carries one non-entailment and one entailment (`rdfs9` at an instance),
  so `ConformingEntails` is neither empty nor everything.
  **What is still not a theorem**, stated at the top of the file rather than at the bottom: that
  `Conforming`'s field list is a SUBSET of the Recommendation's conditions. A subset is the safe
  direction, which is why it is allowed, and a reader checks it cell by cell. One modelling
  decision does NOT run in the safe direction and is named at its field: `IL` is total, following
  RBS section 4.2's wording rather than RDF 1.1 section 5's, so interpretations in which a literal
  fails to denote are outside the claim; closing that needs a term-occurrence lemma about
  `checkStep` that nobody has written. One reading is load-bearing and flagged: RBS Table 5.4 at
  `n = 0`, which `Rules.lean`'s `takeChain` and `Semantics.lean`'s `Model.int` already depend on.
  `Conditions`, `Interp`, `Model`, `Entails` and `certificate_sound` are untouched, and
  `Conforming.lean` pins all twenty-three fields of `Conditions` and all five of `Model` with
  `#guard_msgs` on their constructors, so a change to any of them breaks the build.
- **A binding is data, and a certificate that admits two readings is refused.** Running the
  Lean checker in `lean/OOCert/Horn.lean` and the independent Isabelle/HOL one in `isabelle/`
  over 1,718 certificates found 47 rows where the two disagreed, always in the same direction
  and always from one cause: Isabelle validated the binding list as a data structure and Lean
  did not. Neither checker was unsound, because Lean's acceptances held in Lean's own theorem:
  `EntailsR` quantifies over every total substitution. So the defect was in the FORMAT, which
  said nothing about a repeated binding key or an incomplete binding. `checkHornStep` now
  requires the binding to have distinct keys and to cover every variable the cited rule
  mentions, body and head, and the divergence is 47 rows to 0: 349 accepted by both and 465
  unparseable on both are unchanged, and the 47 moved into rejected-by-both, 857 to 904. The
  corpus test now requires both kernels to refuse each of the 286 rows carrying a malformed
  binding, so the analysis that used to excuse a disagreement is a gate that can fail. The
  refusals are strictness with
  no soundness content: `horn_certificate_sound`, `horn_certificate_sound_fo` and
  `SatRuleFO.to_SatRule` are unchanged in statement and in axiom footprint, and the new material
  adds no axiom to either, being free of the `Classical.choice` that `horn_certificate_sound`
  uses. What the checks buy is stated as `OOCert.wellFormed_determines_instantiation` instead,
  and it is existence AND uniqueness, one half per refusal. Read the binding as a set of demands,
  "this variable is that term", one per written pair: distinct keys make those demands
  SATISFIABLE, and coverage makes every substitution satisfying them instantiate the rule the
  SAME WAY. So the certificate has one meaning rather than one per checker. The existence half is
  why a repeated key is refused rather than resolved by a normative tie-break: two demands on one
  variable are met by no total substitution at all, so first-wins and last-wins are not two
  readings of such a certificate but two ways of discarding half of it
  (`no_substitution_extends_a_duplicate_key`). **A binding for a variable the rule
  never mentions is still accepted.** No Isabelle theory is touched, because it was written from the
  W3C sources without reading the Lean and is worth nothing once it is edited to agree.
  `reason --rules` structurally cannot emit either refused shape, and
  `no_emitted_binding_is_one_the_format_now_refuses` checks that against the bytes. See
  [decision 0008](docs/decisions/0008-a-binding-is-data-and-evidence-admits-one-reading.md).
  This does NOT make a conclusion writable RDF: binding `z` to the bare term `z` is required and
  still accepted, and `the_refusal_costs_no_certificate_anybody_meant` pins that boundary rather
  than letting it be assumed away.

### Added
- **A prover's refutation is read back and re-checked, and it is still an oracle opinion.**
  `src/tstp.rs` parses the TSTP derivation Vampire or E prints, and `fol-prove` /
  `onto_fol_prove` / a second column in `tools/fol_differential.py` report on it. Three things
  are established and each costs more than the last. **The prover refuted OUR problem**: every
  leaf is matched against the emitted problem file by the name its own `file('…', NAME)`
  annotation gives, by the PARSED formula, and by the ROLE, so a prover pointed at a stale file,
  a file edited since under the same names, or one whose conjecture was presented as an axiom is
  caught. **The derivation is a well-founded DAG ending in `$false`**: every parent resolves, no
  name is used twice, the relation is acyclic, and nodes the empty clause does not depend on are
  counted and excluded. **Some steps are recomputed**: binary resolution, subsumption resolution
  in its three spellings (whose conclusion IS the binary resolvent, because the side clause's
  remainder is contained in the main clause's), factoring, duplicate literal removal, flattening,
  trivial inequality removal, equality resolution, and the negation of the conjecture, each
  replayed with a syntactic unifier with an occurs check and compared up to a bijective renaming
  of variables. **Everything else is named and counted as unchecked with a reason**:
  clausification, Skolemisation, AVATAR splitting, every SAT-solver step, and every step of E's
  whose premise is an inline inference record and so carries no formula. Eight verdict words,
  and **an unchecked step prevents the strongest one**, which is
  `refutation_fully_replayed` and is still NOT `unsatisfiable`: the calculus's soundness is
  machine-checked nowhere in `lean/` and the replayer is ordinary Rust. A step whose rule IS
  implemented and still does not reconstruct gets its own word,
  `refutation_step_not_reconstructed`, because filing it under "unchecked" would let a forged
  step hide behind a rule name and filing it under "rejected" would accuse someone else's prover
  on this module's word alone. Measured over FOAF, 181 claimed entailments, one problem each:
  Vampire 5.1.0 refuted all 181 with 1279 of 3314 steps replayed, E 3.2.5 refuted all 181 with
  181 of 4644, all 534 leaves matched on both sides, nothing rejected. The order-of-magnitude gap
  is structural: Vampire attaches a conclusion to every inference, E nests inference records that
  carry none. Negative tests take a genuine Vampire refutation apart one mutation at a time — a
  changed leaf, a leaf naming a formula the problem lacks, a conjecture relabelled as an axiom, a
  dangling parent, a cycle, a duplicated name, a resolvent that is not the resolvent, a forged
  negated conjecture, a resolution needing a cyclic binding — and each must be caught by the field
  meant to catch it. See the addendum to
  [decision 0005](docs/decisions/0005-a-prover-is-an-oracle-and-a-translation-is-a-theorem.md).
- **`--atp vampire` has actually been run.** `docs/first-order-export.md` said the Vampire branch
  of the prover detection had never executed because Vampire was not installed. It is installed,
  it has been run over FOAF, and the argument vector gained `--proof tptp`, without which
  `--mode casc` prints a display format rather than a TSTP derivation. E's gained
  `--proof-object` for the same reason.
- **The MCP server's own tool count had gone stale in a second place.** Its instructions string
  states the total twice and the second copy said 112 while the rest of the repository said 114.
  No shape in `readme_claims_test.rs` covered that phrasing, so neither of its two tests noticed.
  Both now do.
- **A conclusion can name the axioms responsible for it, and a derived triple carries an
  algebraic expression over the asserted ones.** Two tools, `onto_justify` and
  `onto_provenance`, over one substrate that already existed and that nothing read back.
  `derivations.tsv` is a derivation DAG, but it records ONE step per inferred triple, the first
  the fixpoint reached, which is right for a checker that re-derives and wrong for an explainer:
  a triple derived two independent ways has two justifications and the file shows one. So
  `Reasoner::derivation_graph` captures every applicable ground rule instance instead, opt-in,
  at the cost of one branch per candidate triple on a run that did not ask for it.
  `onto_justify` returns the MINIMAL sets of asserted triples responsible for a triple or for a
  clash, and it keeps three claims apart under three words. SUFFICIENCY is re-run and, with
  `certificate_dir`, machine-checkable: each justification gets a certified run over exactly its
  own triples, so `lake exe oo-cert` verifies under `OOCert.certificate_sound` that the
  conclusion follows from that subset. MINIMALITY is re-run without each element and is covered
  by NO theorem. COMPLETENESS of the list is Reiter's hitting-set tree, bounded by
  `max_justifications` and `max_oracle_calls`, with `truncated` naming the bound that fired, and
  complete only for a monotone oracle: the engine's restriction and list vocabulary is read into
  maps keyed by the node, so a node with two values for a functional position contributes one and
  a justification can be MISSED there. None can be falsely reported, because each is re-run.
  `candidate` CHECKS a set instead of searching for one, and a superset of a justification comes
  back `not_a_justification_not_minimal` with the useless triples named. For a clash the verdict
  explained is `clash_found_by_this_engine` and never the checker's
  `unsatisfiable_under_disjointness`. `onto_provenance` evaluates six semirings over the same
  DAG: `boolean`, `why`, `lineage`, `counting`, `tropical` (min-plus) and `trust` (max-min, and
  the choice is named rather than left to be guessed). Recursion is reported rather than hidden:
  `counting` diverges on a cycle, so round k counts proof trees of height at most k,
  `depth_bound` rides beside the number, `value_is_exact` is false unless the iteration
  stabilised on its own, and `cycle_in_support` names a triple on the cycle. Convergence is never
  one word covering unrelated reasons, since `why`, `tropical` and `trust` converge by absorption
  while `boolean` and `lineage` converge because their value lattices are finite. A negative
  weight is refused for min-plus and a weight above 1.0 for max-min, each because it breaks the
  semiring rather than because it is unusual; the second was found by a test that expected 5.0
  and got 1.0. `why` truncation keeps the SMALLEST monomials, which is what keeps the survivors
  an antichain, and a truncated run withdraws the claim that they are minimal supports. 35 tests
  in `tests/justify_test.rs` and `tests/provenance_test.rs`, every fixture small enough that the
  answer is known on paper. See
  [decision 0009](docs/decisions/0009-a-conclusion-names-the-axioms-responsible-for-it.md) and
  docs/explanation.md.
- **Common Logic has three dialects and this engine emitted one, so `fol --format cgif` writes a
  second.** ISO/IEC 24707 defines CLIF, CGIF and XCL, and ISO/IEC 21838-1 clause 4.3 names all
  three as qualifying; emitting CLIF and calling the result Common Logic support was a partial
  claim nothing said out loud. `onto_fol_export` now takes `cgif` as a fifth serialiser over the
  SAME translation, so the file is not a second reading of the ontology. What is emitted is **core
  CGIF** (Annex B.2, not the extended B.3 syntax: no type labels, no `@every`, no `[If: … [Then:
  …]]`), in the sub-dialects clause 7.1.1 names: compact (no sequence markers, which clause 6.5
  says is what takes Common Logic past first order), unstructured and single domain, plus no `#?`
  type label and no actor. The encoding is the standard's own — `[]` is truth and `~[]` is falsity
  (B.2.5, B.2.8), conjunction is juxtaposition with no operator (B.2.6), an equation is a
  coreference concept `[: ?X0 ?X1]` because CGIF has no `=`, and implication, disjunction and the
  universal are B.3.5's and B.3.7's own rewrites into core. Every binder is given a context of its
  own, because `trAx` reuses variable indices between an axiom's antecedent and its consequent and
  B.2.10 forbids two defining labels with one name in one context; renumbering was not available,
  since `owl-lean` uses named variables precisely so that freshness is a proof obligation.
  **Conformance here is pinned by our own checker and not by an independent parser, and the two
  are not called the same thing.** The CLIF claims in `docs/first-order-export.md` rest on two
  external parsers disagreeing with us in recorded ways; no installable CGIF parser exists, so the
  CGIF claims rest on a lexer, parser and syntax checker transcribed from Annex B's EBNF in
  `tests/fol_cgif_export_test.rs`, plus a round trip of every sentence back to the `Form` it came
  from, exact up to the two rewrites core CGIF forces. A comment that would close itself early is
  REFUSED with the clause and the reason rather than rewritten, which is the rule
  `UnwritableSymbol` already follows; that path is reachable and an earlier draft of the CGIF
  header tripped it on every run. XCL is still not emitted and the report says so in
  `common_logic_dialects_not_emitted`.
- **`onto_dlp_boundary`: which of YOUR axioms the rule table can actually see.** `onto_rules_import`
  already polices the Description Logic Programs boundary on the way IN, refusing a non-Horn SWRL
  or RIF construct by name and count. Nothing policed the other direction: a user could load an
  ontology, reason, and receive a certificate the Lean checker accepts, with nothing anywhere
  saying how much of the TBox the rules ever read. The certificate is sound about the fragment the
  rules could see and silent about the rest, which is the same assurance-laundering shape with the
  loss moved from the rule table to the ontology. The new tool classifies every schema axiom and
  **keeps two failures apart that a single "not covered" bucket would destroy**: an axiom
  `outside_the_fragment` is not Horn and no implementation effort changes that (disjunction in the
  consequent, existential in the head, cardinality restriction, negation in the antecedent, each
  listed per axiom with the reason), while an axiom `inside_the_fragment_but_a_rule_is_not_implemented`
  IS Horn, has an OWL 2 RL rule, and is invisible only because this engine does not run it —
  `owl:hasKey` (`prp-key`) and `owl:propertyChainAxiom` (`prp-spo2`) are the two that bite. One is
  a rewrite of the ontology and the other a patch to `src/reason.rs`, and merging them tells the
  reader to do the wrong thing. An axiom that splits soundly is `partially_inside`, and the
  asymmetry is checked: a conjunction splits in a consequent and NOT in an antecedent, because
  dropping a conjunct from a rule body makes it fire more often, which is unsound rather than
  weaker. Where the abstract DLP line and OWL 2 RL's differ the difference is named rather than
  smoothed over: `owl:ReflexiveProperty` is a Horn clause OWL 2 RL excludes as an axiom form, and
  it carries `horn_but_outside_owl2_rl`. The figure six other files state as "29 of OWL 2 RL's 78
  rules" is now DERIVED rather than typed: `reason::RULES_EVALUATED` is the 29, a test greps
  `src/reason.rs` for its own `emit` calls and fails if the two disagree, and the 78-rule
  transcription is cross-checked against the independently written list of seventeen
  false-concluding rules already in that file — a list that said SIXTEEN until it was checked
  against the W3C source. Every triple in the store lands in an axiom bucket or in a counted
  `not_classified` one.

- **The cross-kernel corpus was one step deep, and now goes to nineteen.**
  `tests/cross_kernel_differential_test.rs` ran the Lean and the Isabelle
  checker over 1,718 certificates, and exactly ONE of the 61 base certificates
  contained a step citing an earlier step's conclusion. Strict prefix
  visibility — a step may cite only what came strictly before it and never
  itself — is the property the whole induction rests on, and it was
  differentially exercised by a single two-step fixture. Measured before:
  60 base certificates at depth 0, one at depth 1, maximum fan-out 1, 123 rows
  in the whole corpus touching the discipline at all.

  `deep_cases` now generates chains and fans from a subclass ladder — 2, 4, 8
  and 20 steps, plus a 6-step chain fanning to 8 and a flat fan of 12 — and six
  mutations that a flat certificate cannot express join the corpus:
  `move_step_before_its_premise`, `swap_a_step_with_its_producer`,
  `truncate_chain_in_the_middle`, `two_steps_cite_each_other`,
  `a_deep_step_cites_itself` and `a_step_cites_a_later_conclusion`. Measured
  after: 2,075 certificates, base depths 0:61 1:5 3:1 6:1 7:1 19:1, maximum
  fan-out 12, and 484 rows exercising the discipline (258 resting on it to be
  accepted, 226 that it must reject). Depth is the longest chain of citations,
  so an n-step chain reports n-1. The two kernels returned the same answer on
  2,021 of the 2,075 and parted company on 54: every one of those is the D1
  duplicate-key divergence that was already pinned, none is new in kind, and
  NONE is unexplained. **That count was measured on this branch, which was cut
  before the binding fix above landed, and it is history rather than a
  description: on the merged tree the figure is ZERO, and the 54 moved into
  rejected-by-both exactly as the 47 did. See the Fixed section at the top.**
  Depth found no fresh disagreement, which is a result
  about the two formalisations rather than a null one, because the property
  their inductions are built on had until now barely been shown data that could
  violate it. `the_corpus_exercises_prefix_visibility_at_depth` prints the
  distribution and holds a floor under it, and does not need either proof
  assistant, because the shape of the corpus is a fact about the files.

  Two adversarial certificates no generator produces are committed under
  `tests/fixtures/horn/deep/`. Both instantiate `rdfs9` PERFECTLY — every
  variable bound, no repeated key, body matching premises, head matching
  conclusion — so the only thing wrong with either is the order.
  `self_support_cert.tsv` is a step whose sole unasserted premise is its own
  conclusion. `mutual_cert.tsv` is two steps each citing the other, deriving a
  type assertion for an individual the graph never types; it ships with a
  control that adds ONE triple and turns the same certificate into an accepted
  derivation with the absolute verdict. A checker resolving premises against
  "every conclusion in the file" rather than "every conclusion before this one"
  accepts both, and having accepted them derives anything at all.

  Growing the corpus makes the published counts stale, and three documents quote
  them. `README.md` and `docs/reasoning-systems-inventory.md` are corrected here
  from "47 of 1,718" to "54 of 2,075", both figures re-measured rather than
  inferred: the pre-change file was checked out and run to reproduce 1,718 and
  47 before the new one was run. `isabelle/README.md` carries the same numbers
  in more detail — "1,718 certificates: 61 base, 1,291 mutated, 366 fuzzed.
  Both kernels accept 349, reject 857, and refuse 465 as unparseable.
  Forty-seven rows disagree" — and is LEFT STALE here, because this work was
  done under an instruction not to touch `isabelle/`. The current figures for
  that paragraph are 2,075: 70 base, 1,585 mutated, 420 fuzzed; accept 402,
  reject 1,080, refuse 539; 54 rows disagree, all D1. No test guards any of
  these numbers, which is why one copy was left behind. Pinning the total is the
  wrong guard — it moves whenever anyone adds a Turtle file to one of the ten
  directories `source_graphs` reads, which is the reason
  `tests/reason_rl_coverage_test.rs` refuses to pin its own corpus total. The
  right one is the shape `readme_claims_test.rs` already uses for the tool count:
  assert the copies agree WITH EACH OTHER, so a half-finished correction fails
  rather than a growing corpus. That is not added here, because it would fail on
  the copy this work was not allowed to touch.
- **Seven rules fired nowhere, and now have one fixture each.**
  `tools/horn_differential.py` reported that 20 of the 27 rules in the built-in
  table fire somewhere in the repository's own RDF and seven never do:
  `prp-inv2`, `eq-sym`, `cls-avf`, `cls-hv1`, `cls-hv2`, `scm-svf2` and
  `scm-avf2`. The differential therefore compared the Rust and Python engines on
  those seven zero times. `tests/fixtures/horn-coverage/` holds one Turtle file
  per silent rule, each the smallest graph that makes exactly that rule fire —
  one derivation, credited to that rule, nothing else — with the W3C rule, its
  table (OWL 2 Profiles §4.3 Tables 4, 5, 6 and 9) and the expected derivation
  in a comment at the top of the file. All seven fire, all seven agree across
  both engines, and `oo-horn` accepts every certificate.

  They are TEST DATA and are kept out of the corpus: `discover()` excludes the
  directory, `test_the_coverage_fixtures_are_not_part_of_the_corpus` asserts
  that it does, and the report prints TWO figures that are never added — 20 of
  27 on real ontologies, 27 of 27 once graphs written for the purpose are
  included — naming on every run the rules covered only by a fixture. A fixture
  that fires nothing, or that fires more than its rule, fails the run rather
  than quietly leaving the rule uncovered.

  No rule turned out to be unreachable. That was the interesting possible
  outcome, since a rule no graph can fire is a defect in the engine rather than
  a gap in the corpus, and the tool was built to fail loudly on it; it did not
  fire. The full run of 15 September 2026 — 291 single documents, 203,825
  asserted triples, 24 bundles, 322 compared cases, 1,021 seconds — reports
  `rules_never_fired: []`. Each fixture derives exactly one triple, credited to
  its own rule, identical on both engines, certificate accepted. One pair is
  worth naming, because it is the pair a transcription would get wrong and
  nothing would have noticed: `scm-svf2` concludes
  `?c1 rdfs:subClassOf ?c2` and `scm-avf2` concludes `?c2 rdfs:subClassOf ?c1`,
  the reverse, because universal quantification is antitone in the property.
  The engine derives `:R1 rdfs:subClassOf :R2` from the first fixture and
  `:R2 rdfs:subClassOf :R1` from the second, which is the standard's direction
  in both cases and was, until these fixtures existed, checked by nothing.

  Deciding that no eighth fixture was needed surfaced something that was not on
  the list, and it is recorded rather than fixed. `scm-avf1` needed no fixture
  because the Horn differential credits it on the corpus, while
  `tests/reason_rl_coverage_test.rs` asserts that it fires ZERO times across the
  same corpus, and both assertions pass. They measure different rule sets: the
  differential drives `reason --rules tests/fixtures/horn/builtin_rules.tsv`,
  the 27-row supplied table, and that test drives the `owl-rl-ext` PROFILE.
  Measured on `benchmark/reference/pizza-reference.owl` on 15 September 2026,
  the two part company on more than one rule — profile 357 `rdfs11`, 101
  `scm-svf1`, 0 `scm-avf1`; table 387, 102 and 1 — and the profile additionally
  fires `cls-int2` and `cls-oo`, which the table does not contain. Nothing here
  says which is right. What it does say is that "fires somewhere in the corpus"
  is two different claims in this repository depending on which rule set is
  meant, and neither document that quotes a coverage figure said which. The
  comment beside that assertion now does.

- **The Rust/Python differential is not design-independent, and now says so on
  every run.** The agent that built `tools/horn_differential.py` reported that
  the two engines share an algorithm and that the Python's comments cite the
  Rust by line, and asked that the clean sweep never be quoted as "two
  independent reasoners agree" without that sentence attached. A request is not
  a mechanism, so it is now structural: `INDEPENDENCE_CAVEAT` prints next to the
  agreement count on every run, success or failure, and rides along in the
  `--json` output. Agreement between the two engines is strong evidence against
  a TRANSCRIPTION slip and close to no evidence against a SHARED MISREADING of a
  W3C rule, which both would implement and agree on for ever; the independent
  leg is the Lean checker.

  Five sentences that implied more independence than exists were corrected:
  the tool's own "two independent implementations of the same specification";
  `python/README.md`'s "Because this engine and the Rust one are independent
  implementations of one specification"; `horn/reason.py`'s "A second,
  completely independent reasoner written in Python";
  `python/tests/test_horn_differential.py`'s title line "Three independent
  implementations over one corpus" and its "three separate implementations of
  one specification", which the first sweep missed because it searched the two
  READMEs and the module and not the older of the two differential test files;
  and `README.md`'s paragraph on the second engine, which mentioned neither the
  differential nor its limit. `docs/lean-certificates.md` gains the caveat and
  the rule-coverage split as two Known limitations. Overstating this is the same
  defect as overstating a proof. Nothing in the CHANGELOG had to be corrected:
  the three-way differential had never been written up here, which is why the
  overstatement lived in docstrings and READMEs instead.

  **The evidence for the caveat had itself gone stale.** The stated reason the
  two engines are not independent is that the Python cites `src/reason.rs` BY
  LINE, and all three of those citations pointed about 700 lines short:
  `src/reason.rs:1449` for `graph.all_triples()`, which is at 651 and 2170, and
  `src/reason.rs:1513` for `all.sort_unstable()`, which is at 2240. The Rust file
  had grown underneath the comments. A stale line number is worse than none,
  because a reader who follows it lands on unrelated code and concludes the claim
  was invented. The three are corrected to 2170, 2170 and 2240, and
  `test_the_python_engine_cites_the_rust_by_a_line_that_still_says_what_it_claims`
  now resolves every such citation in `python/` against the current file and
  fails when a line stops containing the token it names. A citation written in a
  shape the test cannot read fails too, rather than going quietly unguarded.

- **Fourteen arms of the OWL 2 RL soundness proof were assumed, and are now
  derived.** A second formalisation in Isabelle/HOL, written from the W3C
  specifications with the Lean deliberately unread, found that fourteen arms
  across twelve of the twenty-nine rules were sound because a field of
  `Conditions` said the rule holds. Twelve of those arms are stated by no cell
  of any specification table: `scm-eqc1` and `scm-eqp1` with two conclusions
  each, `scm-svf1`, `scm-svf2`, `scm-avf1`, `scm-avf2`, `scm-dom1`, `scm-dom2`,
  `scm-rng1`, `scm-rng2`. The other two, `rdfs5` and `rdfs11`, were licensed by
  RDF 1.1 Semantics but redundant once the stronger cell is present. For those
  arms the machine-checked content was close to nothing, while the proofs passed
  and the axiom footprints were clean.

  `lean/OOCert/W3C.lean` now states the OWL 2 RDF-Based Semantics conditions at
  full strength, one field per table row with the cell quoted above it, and
  derives all fourteen. `OOCert.W3CModel.toModel` and every one of the twelve
  derivation theorems depend on no axiom at all. `OOCert.W3CEntails.of_entails`
  proves that everything `Entails` already gave is true in every `W3CModel`, and
  `OOCert.certificate_w3c_sound` restates the checker's verdict over that class.

  **That is not the sentence "true in every conforming interpretation" and this
  entry does not claim it is.** Two things separate them, both recorded in the
  Lean file itself. Reading a conforming interpretation as a Lean `Interp` is a
  bridge written in prose, because core Lean has nothing to quantify over on the
  specification's side; and the bridge assumes five `IP` memberships that are
  cells of no table the file quotes, taken instead from the RDF and RDFS
  axiomatic-triple tables of RDF 1.1 Semantics sections 8 and 9.
  `W3CModel` is also strictly weaker than conformance, because `W3C` omits every
  table row no rule consumes.

  `Conditions`, `Interp`, `Entails` and `certificate_sound` are untouched and
  `certificate_sound` keeps its exact axiom footprint, because strengthening
  `Conditions` in place would have shrunk the model class and weakened the
  headline theorem with every proof still passing. The cell that makes the
  twelve derivable is Table 5.8's connective, which carries `rowspan="4"` in the
  specification's HTML and therefore states an `iff` where RDFS alone gives only
  `if-then`. It was re-fetched from the raw HTML and not from a rendering: a
  markdown conversion of that table drops the `rowspan` and shows `if`, which
  would make the result unprovable and the mistake invisible.

- **A live, non-degenerate model of those conditions, with its liveness compiled
  into the build.** `lean/OOCert/W3CWitness.lean` builds a thirty-five-element
  interpretation meeting the quoted cells of Tables 5.2, 5.3, 5.6, 5.8, 5.9,
  5.12 and 5.13, with every condition settled by the kernel's `decide`. A
  soundness theorem over a degenerate model class would prove less than the
  assumption it replaced while looking better, so three theorems pin what the
  model actually does. `live_is_live`: `IC` is not the carrier, one class
  extension is the whole carrier, another is a proper subset witnessed on both
  sides, the filler extension is non-empty and proper,
  `ICEXT(owl:Restriction)` is a proper subset of `IC`, and `IEXT(p1)` is
  non-empty and a proper subset of `IEXT(p2)`. `live_exercises_every_arm`: all
  fourteen derivations fire at concrete instances of this model, which is
  stronger than any field having something in its extension.
  `live_fires_every_field`: a satisfied antecedent for every one of the
  twenty-one fields of `W3C`, so no condition holds for want of anything to
  check. `dom_bwd` and `rng_bwd` are discharged over real pairs rather than
  vacuously.

  **The first version of this model was vacuous exactly where its conditions
  bite, and the rebuild is recorded rather than quietly substituted.** It had
  seventeen elements, `IEXT(p1)` and `ICEXT(Y)` were both empty, nine of the
  twenty-one fields held because nothing was in the extension their antecedent
  reads, and six of the fourteen arms rested entirely on those nine. Worse, the
  refutation the file exists to deliver was vacuous at its own load-bearing
  premise: `avfPremises` asserts `p1 rdfs:subPropertyOf p2`, which held only
  because `IEXT(p1)` was empty, and that emptiness was written into the liveness
  gate as though it were a feature.

  **It took two rebuilds and the second corrected a claim about the
  specification, not just about the model.** The first rebuild, at twenty-six
  elements, left five fields still holding for want of anything to check:
  `same_fwd`, `sym_fwd`, `trp_fwd`, `inv_fwd` and `hv_eq`. The note beside them
  said four of the five were a budget problem and that the fifth, `owl:sameAs`,
  "cannot be exercised by any model at all because the specification makes that
  relation the diagonal". The premise is right and the conclusion does not
  follow. RBS Table 5.9 row 1 is an `iff` whose right-hand side is `a₁ = a₂` over
  unscoped variables, which the specification's conventions section reads as
  ranging over IR, and RBS section 4.2 defines IR as "the universe of I, i.e., a
  nonempty set", so a conforming interpretation's `owl:sameAs` extension is the
  whole NON-EMPTY diagonal. The
  model had it EMPTY, which no conforming interpretation does, and reported that
  as a fact about the specification rather than a hole in the model. All five are
  now exercised, `live_fires_every_field` pins that, and the residual limit is
  one theorem about every interpretation rather than a list about this one:
  `sameAs_has_no_off_diagonal_instance` says the condition can never be exercised
  at `a ≠ b`. A second consequence of the same two cells meeting is recorded as
  `sameAs_diagonal_needs_an_off_diagonal_subproperty`: a model that gives
  `owl:sameAs` the diagonal cannot also have `IEXT(rdfs:subPropertyOf)` be the
  diagonal, so a countermodel needs two distinct properties one below the other.
  Both quotes were re-read in the raw HTML of the Recommendation on 15 September
  2026.

- **The second kernel's non-vacuity witness had the same defect and now has the
  same treatment.** `isabelle/OO_NonVacuity.thy`'s `M3` was presented as "THE REAL
  WITNESS" for all twenty-six conditions of `owl_rl_interp`. FIFTEEN of them held
  in it only because the extension their antecedent reads is EMPTY: every
  `owl:*` IRI it does not use denotes one junk element, so `c_sameAs_fwd`,
  `c_eqc_fwd`, `c_eqp_fwd`, `c_inv_fwd`, `c_sym_fwd`, `c_trp_fwd`, `c_svf`,
  `c_avf`, `c_hv`, `c_restr_IC`, `c_svf_typ`, `c_avf_typ`, `c_onp_typ`,
  `c_dom_fwd` and `c_rng_fwd` were each discharged by a bare `by simp` off that
  emptiness. `M3_leaves_these_extensions_empty` now states that limit as a
  theorem instead of leaving it to be discovered, and `M3` itself is unchanged.

  `M4` is the witness that exercises them: thirty-four elements, `owl_rl_interp`
  and `wf_interp`, kernel proofs only, no `sorry`, no `oops`, and no `eval`, which
  in Isabelle is the counterpart of the banned `native_decide`.
  `M4_every_condition_has_a_live_antecedent` exhibits a satisfied antecedent for
  each of the twenty-six, and `M4_exercises_every_derivation` applies each of the
  fourteen derived monotonicity and subsumption lemmas of `OO_Builtin_Sound.thy`
  at named elements of `M4`, which is the direct counterpart of the Lean's
  `live_exercises_every_arm` and the same gate. `M4_is_live`,
  `M4_is_bridge_coherent` and `M4_avf2_really_is_antitone` pin the rest.
  `sameAs_is_exercised_only_on_the_diagonal` is the same residual limit the Lean
  states, proved over an arbitrary interpretation rather than about this one. The
  oracle audit in `OO_Audit.thy` now covers all of it: thirty-nine theorems, empty
  oracle set, and a second block that fails the build if an `eval`-proved lemma
  ever stops reporting one, so the gate is proved able to fail.

  Two specification citations were corrected against the raw HTML while doing it,
  and both had been repeated in the Lean. IR is defined in RBS section 4.2,
  Vocabulary Interpretations, not 4.4. And Table 5.2 gives `owl:Restriction`,
  `owl:SymmetricProperty` and `owl:TransitiveProperty` a row each whose second
  column is `∈ IC`; both witnesses keep all three out of `IC` and say so, where a
  draft had claimed the last two have no row in that table at all.

- **Two more English claims turned into theorems, one of which was wrong.**
  `Witness.lean` asserted in prose that `saturated` also satisfies the new
  specification conditions. It does, and `saturated_meets_the_w3c_conditions`
  now proves it, which matters only because it shows how little that is worth:
  a model in which every relation is total distinguishes no condition from any
  other. The claim does **not** extend from the conditions to the models, and
  `saturated_is_not_a_w3c_model_of_an_empty_enumeration` proves the gap on the
  one-triple graph `E owl:oneOf rdf:nil`. Table 5.5's equality forces an empty
  enumeration to denote the empty class, and `saturated` makes every class the
  whole universe. `Conditions.oneOf` carries only the `⊇` half and so models the
  same graph without complaint, which is the cost of that omission stated as a
  pair of theorems rather than as a remark.

### Fixed
- **`src/server.rs` disagreed with itself about the tool count, and the gate that exists to catch
  that covered four of the five places it is written.** The MCP instructions string said "MCP
  server with 114 tools" and, one sentence later, "All 112 tools are advertised in a default
  build". `total_claims` in `tests/readme_claims_test.rs` covered the first and no shape covered
  the second, so the file contradicted itself while the suite stayed green. The fifth shape is now
  in `no_stale_tool_count_survives_anywhere`.
- **`docs/first-order-export.md` said nobody here had read ISO/IEC 24707:2018, and the reason it
  gave for that was wrong too.** The page said downloading it "needs a free ISO account, which is
  an owner action". It does not: 24707:2018 is in ISO's *Publicly Available Standards* list behind
  a click-through licence and no account at all. It has now been read (sha256
  `e920b0c43a932e1ad0e2c76d3501f56e2b11ee5547265b14aeb69a5866eae5e3`), which retires two hedges
  the page carried. A.4.2's exactly-conformant sentence is in the second edition VERBATIM, so the
  CLIF claim no longer needs its first-edition caveat, and the sentence saying nothing was claimed
  about the second edition is gone rather than left standing. The 2007 first edition is no longer
  in the PAS list, so the URL Sowa and Wikipedia cite now fails. Neither part of ISO/IEC 21838 has
  been read and every claim sourced to those is still scoped to the text actually read.

- **Four claims the certificate layer made and had not earned, withdrawn rather
  than edited away.** (1) Soundness here does **not** carry over to the OWL 2
  Direct Semantics read through triples; nobody verified it, and it is false as
  written for the twelve arms that need Table 5.8's backward direction. (2) The
  W3C tables do **not** read a list off the graph. The sequence notation
  quantifies over `IEXT(I(rdf:first))` and `IEXT(I(rdf:rest))`, and the
  specification says explicitly that no semantic constraint enforces well-formed
  sequence structures. `Chain` is still right, for the opposite reason to the one
  given: it fires on fewer lists, so the model class stays larger. (3)
  `an_unlisted_individual_is_not_entailed` was nominated as the check that the
  `⊆` half of `owl:oneOf` was really left out, and it cannot detect that half's
  absence: its witness already satisfies the full equality and passes unchanged
  either way. That omission is undetected by anything in this repository. (4)
  Non-entailment never transferred outward: a `¬ Entails`, a `¬ Unsat` or an
  exhibited `Model` is a statement about this layer's model class unless
  something restates it over `W3CModel`. `docs/decisions/0002` carries a dated
  correction.

- **A quote that was not a quote.** The RDF 1.1 truth clause cited in the new
  bridge argument was the RDF 1.0 (2004) wording. RDF 1.1 replaced the
  definedness phrasing with an explicit `I(p) is in IP` conjunct, which makes the
  argument stronger, not weaker. Corrected against the raw HTML, with the
  substitution recorded in the file.

- `docs/lean-certificates.md` said the semantics was "the RDF-based reading of
  the twenty rules' vocabulary". There are twenty-nine rule ids, as
  `tests/certificate_boundary_proptest.rs` has asserted since it landed.

- **The limitation claim above was itself overstated, and four adversarial
  reviews of this branch took it apart. The corrections are here because the
  overstatement shipped in five places.**

  (1) **"Every conforming countermodel has to be a hand-built finite structure"
  was false, and the argument for it was logically inverted.** It ran: a Herbrand
  witness with no `rdf:type` triple has `IC` empty, so Table 5.8's backward
  direction forces `rdfs:subClassOf` and `rdfs:subPropertyOf` triples the witness
  lacks. An empty `IC` makes `sc_bwd` *vacuous*: its antecedents are
  `I.IC a → I.IC b → …` and they have no instances. `sp_bwd`, `dom_bwd` and
  `rng_bwd` are guarded by `IP`, which is a free parameter of `W3C` and not a
  field of `Interp`, so the refuter chooses it. Four of the seven results listed
  as open transfer with their witness graphs unchanged, on
  `IP := fun _ => False`: `an_unlisted_individual_is_not_w3c_entailed`,
  `membership_in_one_member_is_not_w3c_enough`, `Mixed.lean`'s
  `mix_not_absolutely_w3c_entailed`, and `not_everything_is_w3c_entailed`, which
  was already proved in the same file that listed it as open. The three that do
  not transfer, and a fourth nothing had listed at all
  (`and_the_old_verdict_does_not_notice`), now carry the field that fails as a
  `decide`-checked theorem: `svf_witness_misses_the_restriction_typing`,
  `feed_closure_misses_the_class_typing` and
  `graze_closure_misses_the_class_typing`.

  **Those three theorems were true and they were about the wrong thing, and they
  are gone.** Each said the HERBRAND witness is not a `W3CModel`, which is a
  reason to build a different structure rather than a reason the result is out of
  reach. Three finite structures were built and all three results transfer:
  `the_old_svf_derivation_is_not_w3c_entailed` on an eighteen-element model of
  `svfPremises` whose `ICEXT(C)` is empty; `feed_is_not_w3c_refuted` and
  `the_old_verdict_does_not_notice_over_w3c` on one thirteen-element carrier with
  a single row switched, the row being `ICEXT(Herbivore)`, which is empty over
  `feed` and `{leo}` over `graze`. `feed`'s result is stated as `¬ W3CUnsat`,
  which is STRONGER than `¬ Unsat` rather than weaker, because `Unsat` is a
  universal negative over the model class; `w3cUnsat_of_unsat` records the easy
  direction, that a refutation rules out the conforming interpretations too.
  ALL NINE `¬ Entails` and `¬ Unsat` statements in the repository are now over
  `W3CModel`, and `Semantics.lean` carries the complete table. The two `¬ EntailsR` results
  are about a rule-relative relation with no `W3CModel` counterpart, nothing is
  claimed about them, and that is said rather than left to inference. The obstruction about `Der` and strictly
  negative occurrences survives, and says only that there is no general *closure
  operator* taking any graph to a `W3CModel`, which is not what it was used
  for.

  `mix_not_absolutely_w3c_entailed` is the one that matters commercially. It is
  the machine-checked basis for a run reporting `entailed_under_supplied_rules`
  rather than `entailed`, and that sentence is now about the specification's
  conditions rather than about this layer's own.

  (2) **"Each now carries its obstruction at the point of the assumption" was
  false.** `Mixed.lean`, `Refute.lean` and `RefuteWitness.lean` were byte-identical
  to the versions that predate `W3C.lean`, and `mix_not_absolutely_entailed` sat
  in an unchanged file with no caveat while `Semantics.lean` said otherwise. All
  three now carry their own status at the theorem, `Mixed.lean` as a discharge and
  the other two as a checked obstruction.

  (3) **The bridge understated what it assumes, while claiming superiority over
  the second kernel on exactly that point.** `W3C.lean` said the Isabelle needs
  three axiomatic-triple consequences for the same table "while this file needs
  none, because `Interp.sat` has no `IP` conjunct". Dropping that conjunct moves
  the obligation into the bridge rather than removing it, and the bridge needs
  five: `I(rdf:type)`, `I(rdfs:subClassOf)`, `I(rdfs:subPropertyOf)`,
  `I(rdfs:domain)` and `I(rdfs:range)` in `IP`, the first for every field whose
  conclusion mentions `cext` and the rest for the four backward fields. All five
  are listed in the file with their axiomatic triples, re-read in the raw HTML of
  <https://www.w3.org/TR/rdf11-mt/> on 15 September 2026. On this point the second
  kernel was ahead and the claim was backwards.

  (4) **The `W3CEntails.of_entails` sentence.** This changelog said it proves
  everything `Entails` gave is true in every conforming interpretation, while the
  Lean file's own docstring says that is not yet the sentence. The Lean is right;
  the entry above now says what the theorem says.

### Added
- **The refutation checker has a producer.** `lean/OOCert/Refute.lean` has
  checked refutations since it landed and nothing in `src/` could write one: a
  checker with no producer, which is the same shape of defect as a gate that
  cannot fail. `reason --certificate DIR` (and `onto_reason` with
  `certificate_dir`) now looks for a contradiction in the closure the fixpoint
  reached and writes `refutation.tsv` beside `asserted.tsv` and
  `derivations.tsv`, in the `oo-refute/1` format, with the minimal derivation
  prefix that reached the clash. `lake exe oo-refute check` accepts it and
  `oo-refute guard` refuses the derivation certificate over the same graph,
  which is what `OOCert.a_certificate_adds_nothing_when_the_graph_is_refuted`
  says to do.
- **Ten of the seventeen OWL 2 RL rules that conclude `false` are detected, and
  exactly one is certifiable.** `cax-dw`, `cls-com`, `cls-nothing2`,
  `cls-maxc1`, `eq-diff1`, `prp-irp`, `prp-asyp`, `prp-pdw`, `prp-npa1` and
  `prp-npa2` are looked for after the fixpoint. Only `cax-dw` gets a refutation
  file, because `OOCert.RefuteConditions` carries a semantic condition for that
  rule alone and `oo-refute` refuses a refutation naming any other with exit 2.
  The other nine report `clash_found_by_this_engine` and write nothing, and the
  seven not looked for at all (`cax-adc`, `prp-adp`, `eq-diff2`, `eq-diff3`,
  `cls-maxqc1`, `cls-maxqc2`, `dt-not-type`) are listed in the response with the
  reason, so a clean run is never read as a consistency result.
- **The two verdicts are kept apart by a test.** `clash_found_by_this_engine` is
  the engine's word and `unsatisfiable_under_disjointness` is what `oo-refute`
  prints for an ACCEPTED refutation.
  `tests/lean_refutation_producer_test.rs::the_engine_never_states_the_checkers_verdict`
  walks every string in the response and fails if the engine ever states the
  checker's verdict or names its soundness theorem, on the pattern
  `a_user_rule_never_earns_the_absolute_verdict` set for the Horn layer.
  `a_consistent_ontology_is_not_refuted` is the test that matters most: no
  refutation is produced for a consistent ontology and a hand-written one over
  it is rejected.
- **The limit is stated and computed, not papered over.** `cax-dw` needs an
  INDIVIDUAL in two disjoint classes, so a TBox unsatisfiable with no individual
  asserted is invisible to the rule-based route.
  `the_tbox_only_case_is_invisible_here_and_the_tableau_sees_it` runs one
  ontology through both paths: `owl-rl` finds nothing and `owl-dl` reports the
  unsatisfiable class. No refutation is emitted from the tableau, and the
  obstruction is written down: `oo-refute/1` can express one contradiction,
  `cax-dw`, over triples the forward-chaining prefix can reach, and a clash
  reached through `∃`/`∀` expansion or a cardinality bound has no form in it.
  Widening that is a change to `lean/`.
- **The trusted computing base of the certificate layer is written down and
  property-tested.** `docs/trusted-computing-base.md` enumerates, as twenty-nine
  checkable properties, everything `lean/` assumes about the Rust: that
  `asserted.tsv` is the graph the engine reasoned over with nothing dropped and
  nothing added, that every emitted step is a step the engine took, that the
  interner round-trips, that the rule table the certificate is checked against
  is the table that was evaluated, and that no term can carry the field or
  record separator of a format that has no escaping layer of its own.
  `tests/certificate_boundary_proptest.rs` property-tests them with generators
  built to forge a derivation step: literals holding tabs, newlines, carriage
  returns, quotes and backslashes, a literal spelled exactly like an IRI, a
  literal spelled exactly like a whole extra TSV line, combining characters
  against their precomposed form, percent-encoded separators inside IRIs, the
  empty graph. `proptest` is a DEV-dependency; the shipped binary's dependency
  surface is unchanged. Three Kani harnesses in `src/reason.rs` (`make verify`)
  prove the serialisation properties over every byte pattern at a fixed term
  length rather than over a sample; the bound is stated on each harness, along
  with the two formulations that made CBMC measure the wrong function. A fourth,
  over `parse_pat`, does NOT terminate (no verdict at 14 minutes and 9.5GB,
  because `anyhow`'s error formatting is flattened whether or not the refusal
  paths are reachable); it is excluded from `make verify`, carries its
  measurements, and the property is sampled instead. The first property-test run
  found the `rdfs7` defect below, and measuring its reach found the `cls-avf`
  one.
- **The SAT/SMT family, and the asymmetry that makes it worth building.** A
  refutation cannot be replayed in core Lean, so decision 0005 rules a prover's
  verdict an oracle opinion for ever. A MODEL is the opposite: a finite object,
  decidable to check, and `lean/Fol/` holds a verified evaluator for it. So the
  SATISFIABILITY direction is CERTIFIED while the refutation direction stays an
  oracle, and the two never share a word.

  `fol --format smtlib|ladr` are a third and fourth printer over the same
  `FolProblem` the TPTP and CLIF printers walk, never a second translation;
  every run also writes `problem.tsv`, the format `oo-folmodel` reads, with a
  digest the Lean recomputes. `fol-model` (also `onto_fol_model` and batch
  `fol-model`) drives the whole loop: export, run Z3 or Mace4, read the
  structure back, hand it to the verified checker, and report five fields that
  are never collapsed — `solver_verdict`, `encoding`, `checker_exit`, `verdict`
  and `owl_reading`. Only `model_checked` rests on a theorem
  (`Fol.satisfiable_of_check`), and it requires `checker_exit: 0`. An exhausted
  BOUNDED search is `no_model_up_to_size_k` and is not unsatisfiability:
  `∀x∃y (r(x,y) ∧ x≠y)` is unsat at carrier 1 and sat at carrier 2.
  `unsatisfiable_oracle` may come only from a run with no cardinality
  constraint. With a goal, a checked countermodel carries
  `not_entailed_under_unproved_translation` — `Fol.not_entails_of_check`, the
  sentence no prover can produce, weakened by the two things this layer does
  not prove.

  A solver answering `sat` whose model the checker REJECTS is a STOP_THE_LINE
  disagreement with its own block, counted in the summary and exiting non-zero,
  the way `tools/shacl_differential.py` treats a FALSE_CLEAN. It is never
  `rejected` as though the ontology were at fault and never `model_checked`.

  Mace4 is here because the dead toolchain has a live half: Prover9's
  refutations are uncheckable, and Mace4 is a finite model finder whose output
  is exactly what this layer certifies. Its symbols are MANGLED, because LADR
  reads a name beginning with `u`, `v`, `w`, `x`, `y` or `z` as a VARIABLE —
  measured on LADR 2009-11A, `p0(w0). -p0(k0).` is echoed as `p0(x).` and the
  run reports `exit (exhausted)` with no error at all.

### Fixed
- **An assertion on a punned entity was dropped from the first-order export
  SILENTLY, and the report said the axiom set was not weakened.** OWL 2 DL lets
  one IRI be a class and an individual at once; `OwlLean/Syntax.lean` does not,
  so the assertion is correctly outside the fragment — but
  `count_out_of_fragment` knew six named OWL constructs and nothing about
  punning, so `exports_a_weaker_axiom_set` was `false` over an export that was
  weaker than the graph. Found by the new model-certificate pipeline, which
  returned machine-checked countermodels for five of the nine triples the
  OWL-RL reasoner derives on
  `case-studies/blast-furnace-ironmaking/blast-furnace-ontology.ttl`, where
  `bf:Hanging` is an `owl:Class` that also carries `bf:hasSeverity
  bf:HighSeverity`. Now counted as `assertion on a punned entity` with its
  reason; the reading itself is unchanged.
- **A stop-the-line disagreement did not fail a `batch` run.** `src/batch.rs`
  decided the exit code from the presence of an `"error"` key, and a
  stop-the-line is not an error: the command ran and answered. Batch is the
  mode every tool in `tools/` uses, so in the one place the gate has to bite it
  did not. It now also reads the `stop_the_line` count, keyed on the field
  rather than on the command name.

### Added
- **Entailment preservation under graph projection, because coverage is the
  wrong measure and looks like assurance.** `src/projection_entailment.rs`
  (CLI `preserve`, MCP `graph_projection_entailment_check`) takes the claims an
  answer rests on and reports, for each one, whether the retrieved slice entails
  it exactly when the source does, with a sub-certificate the Lean checker
  accepts for every claim it preserves. `src/closure_diff.rs` (CLI
  `closure-diff`, MCP `onto_closure_diff`) is the goal-free form, for auditing a
  retrieval strategy rather than one answer, and reuses the same certificate
  index, checker runner, subset precondition, skolemiser and differential, so
  there is one place in the crate where each verdict word is produced. No new
  Lean was written: `OOCert.certificate_sound` covers the sub-certificates
  unchanged, and `OOCert.horn_certificate_sound` covers a run over a supplied
  Horn table, which earns
  `preserved_under_supplied_rules_checked` and never the plain word.
  Four outcomes are kept apart, because a lossy retriever and a hallucinating
  generator have opposite fixes: `preserved_*`, `lost_under_profile_unchecked`,
  `ungrounded_in_source` (NEITHER graph derives the claim) and
  `projection_only`. Measured on `benchmark/reference/pizza-reference.owl`, a
  file this repository ships: the whole ontology minus one `rdfs:subClassOf`
  triple scores `aggregate_coverage_ratio: 1.0` with `ok: true` while the
  conclusion the answer rests on is gone, and a three-triple slice scores
  0.00128 and preserves every claim with a checked certificate. Both are tests.
  See [decision 0007](docs/decisions/0007-a-slice-preserves-a-conclusion-or-it-does-not.md)
  and [docs/projection-entailment.md](docs/projection-entailment.md).
- **A monotonicity differential that runs on every call and is a defect
  detector for the engine, not for the retrieval.** OWL RL is monotone and a
  projection is a subset, so anything the projection entails and the source does
  not is a soundness bug in this engine. It is reported at `STOP_THE_LINE` with
  exit 2, and it is DISARMED, loudly and with its own reason, whenever its
  antecedent fails: the projection is not a subset (computed every run, never
  inferred from provenance), blank nodes could not be matched, or either run
  stopped at the iteration cap instead of a fixpoint. A green
  `violations: []` under a disarmed gate means "we did not look", so the two can
  never render the same. Run for real over the shipped corpus
  (`tests/projection_monotonicity_corpus_test.rs`): 275 ontologies swept, the
  gate ARMED on all 275, every source certificate accepted by `oo-cert`, and
  **no violations found**.
- `tests/gate_demonstration_test.rs`, which feeds every gate in this work the
  input built to trip it, PRINTS what the tool said, and asserts the same
  thing. A gate that cannot fail is decoration, and a gate whose failure nobody
  has read is close to it. Run it with `--nocapture`; a CI leg does.
- `Reasoner::run_full` reports `fixpoint_reached`, matching `run_horn`. A run
  that stopped at `reasoner_max_iterations` has a closure that is a LOWER BOUND,
  and until now a truncated closure was indistinguishable from a complete one,
  which is what would have made a correct engine look unsound to any consumer
  comparing two closures.
- `GraphStore::graph_store`, `graph_triples`, `named_graph_iris`,
  `materialised_inference_count`, `all_quads`, `parse_triples_ordered` and
  `canonicalise_triples`. The first copies a named graph into a fresh store's
  default graph BY MODEL TERM, which is the only route by which a slice can be a
  subset of its source in the strong sense, blank nodes included.

### Fixed
- **`onto_segment_retrieve` emitted slices that do not parse, which broke the
  pairing it advertises.** Every term was wrapped in angle brackets
  unconditionally, so a blank-node `owl:Restriction` superclass came out as
  `<_:b0>`, which is not a legal IRI. Since issue #93 `load_turtle` collects all
  or nothing, so one restriction superclass made the ENTIRE slice unreadable and
  `graph_projection_lossy_check` then reported `projection_parses: false` with
  `aggregate_coverage_ratio: 0.0` and no diagnosis. Measured on
  `benchmark/ontoaxiom/data/ontoaxiom/ontologies/pizza.ttl` and
  `benchmark/reference/pizza-reference.owl`, which is to say on essentially
  every OWL ontology here. Blank nodes are now written bare, and the
  angle-bracket trim no longer eats the closing bracket of a typed literal's
  datatype IRI.
- **`check_projection_loss` reported `ok: true` on a projection holding MORE
  than the source.** `coverage_ratio` clamps with `.min()`, so a seed whose
  slice carries triples the source does not read exactly 1.0 with empty dropped
  lists: a hallucinating retriever, or a slice of a different graph, scored a
  clean bill of health on the one input that should stop a pipeline. The clamp
  stays, because an unclamped "ratio" above 1.0 is not a ratio; the surplus is
  now named in `seeds_with_surplus` and `ok` requires it to be empty.
- `docs/lean-certificates.md` said the certified corpus was 122 files. Counted
  on 14 September 2026 under the test's own filter it is 290 tracked RDF files,
  21,256,445 bytes.
- The advertised tool count was stale before this change and is now measured.
  `src/server.rs` carried 110 distinct `#[tool(name = ...)]` macros while the
  README, the docs and the server's own instructions said 109: `onto_fol_export`
  landed without the count moving. With the two added here it is 112, counted
  from the source rather than incremented. The narrative comment in
  `src/graph.rs` about the removed store mutex no longer names a number, since
  what it is about is the lock and not the tool count.

### Changed
- **`coverage_ratio` is demoted, not deleted.** `ProjectionLossReport` gains
  `is_a_warrant: false` and a `warning` field carrying the sentence in the
  payload rather than only in the docs, so a renderer that walks the data still
  emits it, and the `graph_projection_lossy_check` tool description now carries
  it too, because a tool description is the text an agent reads when choosing
  and that is where the trap was living unlabelled. The number is neither
  necessary nor sufficient for entailment preservation and it moves the wrong
  way, rising as the projection grows, so a retriever tuned on it learns to
  fetch more rather than the right thing.

- **First-order export, over the translation a machine-checked adequacy theorem
  is about.** `fol --out DIR --format tptp|clif` (also `onto_fol_export` and
  batch `fol`) writes the loaded ontology as TPTP FOF or as ISO/IEC 24707
  Common Logic, and with `--goals FILE` one problem per conjecture, so an
  ontology can be handed to E, Vampire or any other first-order prover. The
  translation in `src/tptp.rs` transcribes `OwlLean/Translation.lean` from the
  sibling `owl-lean` project, whose `OwlLean.adequacy` is machine-checked with
  no `sorry`, no Mathlib and axioms `propext`, `Classical.choice`,
  `Quot.sound`. Everyone else's OWL-to-FOL exporter is validated empirically
  (FOWL, over 168 ChEBI modules) or proved on paper (Hets); LATIN's
  `OWL2toFOL.elf` has its cardinality constructors and `objectPropertyChain`
  commented out, and those are used by 37.2% and 45.9% of constrained real
  ontologies. The background axioms and the individual typing axioms the
  theorem requires are emitted, the freshness side condition is enforced at
  every call site rather than assumed, and both have machine-checked
  countermodels in `OwlLean/Refutations.lean` showing what goes wrong without
  them. **The correspondence between the Rust and the Lean is pinned by
  `tests/fol_translation_correspondence_test.rs` and is NOT itself proved**,
  which is stated in the module docs, in the JSON report and in the header of
  every emitted file. Constructs outside the fragment are named in the output
  with counts and reasons through `exports_a_weaker_axiom_set` and
  `constructs_not_exported`, the shape the description-logic layer already uses
  for `certifies_a_weaker_axiom_set`. CLIF is a second serialiser over the one
  translation, never a second translation, and is restricted to the
  first-order-equivalent fragment of Common Logic: no sequence markers, fixed
  arity, no quantification into a predicate position. See
  docs/first-order-export.md and decision 0005.
- **The CLIF export is gated against leaving the exactly semantically
  conformant subdialect.** ISO/IEC 24707 first edition A.4.2 says the
  subdialect using neither numerals nor quoted strings is exactly semantically
  conformant, so staying inside it makes CLIF entailment and Common Logic
  entailment coincide and the adequacy theorem needs no qualification at the
  CLIF end. `clif_stays_in_the_exactly_conformant_subdialect` walks every
  emitted sentence and fails on a bare decimal or a single-quoted string. The
  scope is stated exactly: no numerals and no quoted strings IN SENTENCE
  POSITIONS, with comment annotations the named exception.
- **`tools/fol_differential.py`, an ATP as a differential oracle and never as
  an authority.** It reasons with a certificate, exports one problem per
  claimed entailment from a separate unreasoned store, and asks E or Vampire.
  A conclusion the engine derives and the prover refutes is
  `CLAIMED_NOT_ENTAILED` and exits 1; a prover that gives up is
  `UNDETERMINED` and is never read as agreement. **An ATP verdict is an oracle
  opinion, exactly like pyshacl's in `tools/shacl_differential.py`**: a
  superposition refutation cannot be checked without a verified first-order
  calculus with unification, which does not exist in core Lean, so a
  disagreement is a bug in one of the two and the tool says so rather than
  adjudicating. With no prover installed it skips loudly with the install line,
  and `FOL_DIFF_REQUIRE_ATP=1` turns that skip into a failure. Run against E
  3.2.5 over FOAF it reported 58 disagreements on first use, all of them
  defects in the new export layer, listed under Fixed below.
- **SWRL and RIF Core front ends, so the logic-programming family is covered in
  fact and not only in architecture.** `lean/OOCert/Horn.lean` proves one
  soundness theorem good for every rule table at once and decision 0003 names
  RIF Core, Datalog and SWRL as the reason, but nothing in the repository could
  produce a rule table from a standard rule syntax: the only format anything
  read was `rules.tsv`, an internal encoding, and grep found no mention of SWRL
  or RIF outside that decision record. `rules-import --from swrl|rif`
  (`onto_rules_import` over MCP, `rules-import` in batch) closes that. SWRL is
  read out of a loaded RDF graph through the `swrl:Imp` encoding, reusing the
  one `rdf:first`/`rdf:rest` reader `src/tableaux.rs` already had for
  `owl:intersectionOf`; RIF Core is read out of its normative XML syntax.
  Datalog is deliberately not offered as a front end, because it has no single
  standard concrete syntax and a Datalog program over triples IS a `rules.tsv`
  table.

  **Only a fragment of each language is a Horn table over triple patterns, and
  the exact fragment is in docs/rule-syntax-front-ends.md and in every
  response.** SWRL built-in atoms, `swrl:SameIndividualAtom`,
  `swrl:DifferentIndividualsAtom`, `swrl:DataRangeAtom`, anonymous class
  expressions and an empty head are refused; RIF `Equal`, `External`, `Expr`,
  `rif:local` constants, `List` terms, `Or`/`Neg`/`Naf`, an existential
  conclusion, an `Atom` of arity 0 or 3+, `rif:Import` and the presentation
  syntax are refused. Every refusal is NAMED AND COUNTED, and by default one
  refusal fails the whole import with no table written: a rule set that quietly
  lost half its rules still reaches a fixpoint and its certificate still checks
  green, which is a sound proof about a rule set nobody wrote.
  `--allow-partial` imports the rest and flags the result
  `certifies_a_weaker_rule_set`, the name the DL model-certificate block
  already uses for the same idea.

  Every rule a front end emits is named `swrl/…` or `rif/…` and no built-in
  rule is, so an imported table can never render identically to the built-in
  one and can never earn the absolute verdict. It always lands on
  `entailed_under_supplied_rules` under `OOCert.horn_certificate_sound`.
  `tests/rule_syntax_frontend_test.rs` runs a real rules file in each syntax
  through the engine into `lake exe oo-horn check` and asserts the verdict it
  gets back; `no_front_end_can_name_a_rule_the_way_a_built_in_is_named` pins
  the naming with no Lean present, and `a_partial_import_can_never_be_silent`
  pins that no combination of arguments writes a table with a refusal recorded
  and the weaker-rule-set flag false.

- **Derivation certificates, checked by a proved-sound Lean checker.**
  `reason --certificate DIR` (also `onto_reason`'s `certificate_dir` and batch
  `reason --certificate`) writes `asserted.tsv` and `derivations.tsv`: every
  inferred triple with the rule that produced it and the premises the rule
  read, in a fixed order per rule. `lean/` holds a checker for that format
  whose soundness is a machine-checked theorem, `OOCert.certificate_sound`,
  with its axioms pinned by `#guard_msgs` so that a `sorry` fails the build: a
  certificate it accepts contains only triples entailed by the asserted graph
  under the RDF-based semantics of the twenty rules' vocabulary. Core Lean, no
  Mathlib, no dependencies. A new CI job builds the proofs, certifies every
  RDF file the repository ships, and appends forged lines the checker must
  reject. `owl-dl` refuses the flag rather than pretending it has a rule
  trace. See docs/lean-certificates.md and decision 0002.

### Fixed
- **An ASSERTED role edge skipped its `rdfs:domain` and `rdfs:range`, so an
  inconsistent ABox was reported consistent.** Domain and range are deliberately
  not GCIs: as `∃p.⊤ ⊑ D` and `⊤ ⊑ ∀p.R` they put a disjunction on every node,
  and hqdm.owl alone carries 525 of them, so they are held as role metadata and
  applied when an edge is created. Two places create an edge and only one
  consulted that metadata. `Tableau::create_successor` applied it; the ABox
  builder wrote asserted role assertions, and the inverse back-edges it
  materialises for them, straight into the edge map. Every other role-sensitive
  rule reads edges through `successors()` and so was unaffected, which is why
  exactly these two constraints were weaker on an asserted edge than on a
  generated one. The consequence was a false clean: an ontology whose only
  contradiction is that an asserted edge forces its subject into a class
  disjoint from one it already carries came back `consistent: true` with
  `undecided: false`, the strongest answer the checker can give. Both paths now
  go through one `Tableau::add_role_edge` primitive. It also applies the
  constraints stated on the role's INVERSE, which NEITHER path applied before:
  `a r b` entails `b r⁻ a`, so `r⁻`'s domain binds `b` and its range binds `a`,
  and a symmetric role is its own inverse and rides the same clause. Found by
  the model-certificate layer, which had been refusing to certify these
  completion graphs because they are not models of the range axiom;
  `tests/dl_model_certificate_test.rs` pinned the defect and now pins the
  repair.
- **GCIs did not reach an individual named only as the object of a role
  assertion.** The ABox builder gives the GCIs to every typed individual and
  `create_successor` gives them to every generated successor, but an IRI that
  appears only as an edge's object got a bare node carrying nothing but its own
  nominal. A GCI holds of every element of the domain, so that node was a hole
  the check could not see into, and an ABox whose only contradiction landed
  there was reported consistent. Same shape as the domain/range split, found in
  the sweep for it.
- **A rule concluding a triple with a non-IRI PREDICATE left the store half
  materialised, with no certificate, and reported failure.** `rdfs7` reads
  `s sub o` and `sub rdfs:subPropertyOf super` and concludes `s super o`, and
  nothing required `super` to be an IRI. `:p rdfs:subPropertyOf [ owl:inverseOf
  :q ]` is enough, and that is exactly what the OWL 2 mapping to RDF produces
  for `SubObjectPropertyOf(:p ObjectInverseOf(:q))`, so an ordinary OWL
  ontology made `reason` fail with `Parser error: The predicate of a triple must
  be an IRI`. The failure was not clean: `GraphStore::load_ntriples` streams,
  inserting each quad as it parses it, so the batch failed PARTWAY. Measured on
  one fixture over three consecutive runs of identical input, 40, 9 and 24 of
  40 good inferences stayed in the store, the number being hash iteration order,
  and `run_full` then returned the error and never wrote the certificate. The
  store was left holding uncertified inferences while the caller was told the
  run had failed. In `--dry-run` the materialiser never ran, so the certificate
  WAS written, containing a conclusion no RDF serialiser can express; the Lean
  checker holds terms as opaque strings and accepts it. Reachable from `rdfs7`,
  `prp-inv1`, `prp-inv2` and `cls-hv1`, each of which takes its conclusion's
  predicate from an object position where RDF permits a blank node or a literal.
  Found by the new property tests at case 115, shrunk to two triples.
- **`cls-avf` could still conclude a triple with a LITERAL subject.** The same
  defect one position over, found while measuring the reach of the one above.
  `cls-avf` derives `y rdf:type c` from `x rdf:type ∀P.c` and `x P y`, so an
  `owl:allValuesFrom` restriction on a property with a literal value derived
  `"x" rdf:type :D` and broke the materialiser the same way. The subject case
  was found on 30 August 2026 and guarded at `prp-symp`, `prp-inv1`, `prp-inv2`
  and `eq-sym`; `cls-avf` was not among them, and this one needs no unusual
  modelling at all. Both positions are now decided in ONE place,
  `writable_triple`, shared with `run_horn`, which was the only path that
  already refused either: a refused conclusion is not materialised, not
  certified and not available as a premise, and the run reports
  `skipped_unserialisable` with examples. `tests/reason_unwritable_predicate_test.rs`
  pins every reachable rule, and pins that `prp-symp` and `prp-trp` are NOT
  reachable, because their property must already be the predicate of a stored
  triple and is therefore an IRI.
- **`owl:FunctionalProperty` and `owl:InverseFunctionalProperty` were
  completely inert, and the module documentation claimed both.** Functionality
  is encoded as the GCI `≤1 R.⊤` and inverse-functionality as `≤1 R⁻.⊤`. Two
  things then cancelled each other out: `add_label` returns early for
  `Concept::Top` and never stores it, and the ≤-rule counted matching
  successors with `labels.contains(&filler)`, which with `filler == ⊤` is false
  on every node that will ever exist. So the bound counted zero successors, could
  not be violated, and no merge ever fired. A functional property with two
  distinct fillers in disjoint classes came back `consistent: true` with
  `undecided: false`. The ≥- and ∃-rules used the same test and so also counted
  zero, manufacturing successors they already had. All three now go through one
  `node_satisfies`, which is where `⊤` is handled; `has_clash` had special-cased
  `⊤` for cardinality LABELS on one node all along, and only the
  successor-counting sites were missed. The ≤-rule also no longer marks a bound
  processed before it branches, which limited it to ONE merge per node: with
  three fillers under `≤1` a single merge leaves two and the branch was returned
  as a model of a constraint it visibly breaks. Termination is argued at the
  change and rests on the recursion depth, which increases by one on every merge
  and is hard-capped. Measured over a 20-ontology corpus this repair costs
  nothing: times within noise, node counts flat or lower (the ∃/≥ half stops
  building redundant successors), and not one verdict moved.
- **The role hierarchy was not closed under inverses, so a constraint on the
  inverse of a super-role never reached a sub-role's edge.** `r ⊑ s` entails
  `r⁻ ⊑ s⁻` — if `(x,y) ∈ r` then `(x,y) ∈ s`, so `(y,x) ∈ r⁻` implies
  `(y,x) ∈ s⁻` — and that was simply never computed. Each role's `rdfs:domain`
  and `rdfs:range` are folded over its transitive super-roles once and consulted
  per edge, so a domain stated on `s` already reached an `r` edge; a domain
  stated on `s⁻` did not, because `r⁻` was not known to be below `s⁻`. The
  closure is one pass, not a fixpoint: `inverse_roles` is an involution, so
  re-applying the rule to a pair it adds yields the pair it came from.
  Transitivity is still left to the existing downstream closure, which now runs
  over the enlarged relation. Because it is applied at parse time, the model
  certificate's `SubRole` axioms describe the same hierarchy the reasoner used.
- **An IRI that is never typed contributed no role assertions at all, which is
  a false clean now that asserted edges carry their domain and range.**
  Individuals were discovered by walking subjects that have an `rdf:type` and
  keeping the ones whose type is recognisable as a class, so a subject that is
  never typed — or is typed only by a vocabulary this reasoner does not treat as
  a class, a `skos:Concept` say — contributed neither its edges nor itself. An
  individual with two role assertions whose domains are disjoint classes is
  inconsistent on those two triples alone and the check never saw them. The
  object side of the same hole was already closed; this is the subject side, and
  the filter is narrow: IRIs only, never something already declared a class or a
  property, and only when the subject actually asserts a declared object
  property. `build_abox_tableau` now builds a node for BOTH endpoints of a role
  assertion, and "this ontology has no ABox" now means no role assertions
  either, in `check_abox` and in `certify_abox_consistent` alike, because those
  two guards must agree or the certificate layer declines to certify an ABox the
  reasoner decided.
- **`individuals_checked` undercounted by exactly the individuals the reasoner
  knew least about.** It was taken before the nodes for role-assertion endpoints
  were added. A node that carries labels, carries the GCIs and can clash has
  been checked whatever its `rdf:type` says, and reporting `0` also suppressed
  the entire `abox` block from the output. It is now one per node in the ABox
  tableau.
- **Every phase of a reasoning run shared one wall-clock deadline fixed when
  the reasoner was built.** The satisfiability sweep, the subsumption sweep and
  the ABox check all read one `Instant` computed in `DlReasoner::from_graph`, so
  whichever ran first spent the clock and the ABox check — which runs last, and
  is the one a user reads about their own data — reported `undecided` on an ABox
  it decides in microseconds. Honest rather than false, which is why it
  survived, but a loaded machine silently degraded an answer that was there. The
  reasoner now holds the budget as a DURATION and each phase opens its own. The
  number is unchanged and no cap was raised. A run that does not finish now also
  says WHICH phase ran out, in `budget_exhausted_in`, because the three draw on
  different settings and "incomplete" alone tells a reader nothing they can act
  on. Cost, measured: on the five corpus ontologies that exhaust the budget the
  wall clock goes from ~10s to 30-40s, since four tableau phases now each get the
  configured per-test budget instead of four sharing one already-expired instant.
  No verdict changed on any of the twenty.
- **`classify_timeout_ms` had never bounded a real run, and now bounds one or
  says it does not.** It defaults to 180 000 ms and its own documentation called
  it "the budget that actually bounds the run". Two separate reasons it was
  neither. AT THE SHIPPED SETTINGS IT COULD NOT FIRE: four phases —
  consistency, satisfiability, subsumption, ABox — each opened a
  `tableaux_test_timeout_ms` budget of 10 000 ms, and four times ten is forty,
  which is less than a hundred and eighty, so the ceiling was dead arithmetic.
  And it DID NOT COVER THE RUN: the setting was read inside `classify_parallel`
  and nowhere else, so the three phases around classification each opened a fresh
  budget outside it and a run could exceed its own ceiling by three further phase
  budgets. WITNESSED, not inferred: with `classify_timeout_ms` set to 1 ms over
  an 80-class ontology, the pre-fix engine reported
  `budget_exhausted_in: ["satisfiability", "subsumption"]` and no `abox` — the
  ABox check had opened a fresh 10 000 ms deadline after the ceiling was already
  spent. The explanation loop, the fifth phase, had no wall clock AT ALL —
  `Tableau::new_with_tracing` leaves `Budget::deadline` at `None` — and `run`
  performs one such tableau per unsatisfiable class, in a loop, straight after a
  classification that may have just been cut short for want of exactly that
  budget.
  The ceiling is now enforced over every phase, by intersecting it with each
  phase's own deadline rather than by minting a fifth budget beside them. That
  costs nothing measurable: a tableau already checks one deadline inside its
  expansion loop, and it is now handed the earlier of the two. Minting a deadline
  per TABLEAU is what would make 180 000 ms the bound that fires, and it is what
  took the corpus from 67s to over 600s with no verdict changing; it is still not
  done. That 67s is a number from an earlier session over a corpus nobody wrote
  down, kept as the reason a route was abandoned and flagged in the source as
  unreproducible, which is why `tests/reasoner_budget_corpus_bench.rs` now pins
  two corpora that ARE in the tree. What is done instead is the part that was missing, which is SAYING SO:
  every `owl-dl` run now carries a `budget` block naming both settings, the five
  phases, the worst case the phase budgets permit, which of the two bounds
  actually stops the run — `global`, `phase`, or `none` — and a sentence of prose
  saying why. A knob that reads as a safety limit may not enforce nothing; where
  it cannot be the binding one, the output says that in words rather than leaving
  a reader to do the arithmetic from two settings and a phase count.
  And the knob is now a knob. Neither budget appeared in `ReasonerConfig`, so
  neither could be set from `config.toml` at all — `apply_reasoner` wrote the
  depth cap, the node cap and the iteration cap and touched neither clock —
  while three comments in `src/tableaux.rs` referred to
  "`[reasoner] classify_timeout_ms`" as though it were a setting. The only way
  to move either was a setter that exists for tests: `set_tableaux_test_timeout_ms`
  was documented as "used by the CLI `--reason-timeout-ms` flag", and there is no
  such flag anywhere in `src/` — grep for it and the only hit is the sentence
  claiming it. Neither reasoner clock had a user-reachable setting of any kind,
  and both were documented as though they did. Worse than unreachable: `Config`
  does not deny unknown fields, so a user who wrote
  `[reasoner] classify_timeout_ms = 30000` got a clean parse, no warning and no
  effect — the setting was accepted and discarded. A ceiling nobody can lower is
  the strongest form of a limit that enforces nothing, and the note this now
  prints tells a reader to lower it. `[reasoner] classify_timeout_ms` and
  `[reasoner] tableaux_test_timeout_ms` are real keys; `0` means OFF for both,
  not "use the default", which is the opposite of the three caps beside them and
  is documented where they are declared.
  Corpus cost, measured with the new and reproducible
  `tests/reasoner_budget_corpus_bench.rs` over two corpora, THREE runs of each
  build alternating so they share the machine's load. Twenty case-study
  ontologies, none of which reaches a budget: 0.33 / 0.09 / 0.09s before,
  0.10 / 0.34 / 0.08s after. Ten from `benchmark/` that do, two of them
  exhausting one: 62.42 / 62.78 / 65.10s before, 63.75 / 62.62 / 64.55s after.
  Unchanged — the "after" range lies inside the "before" range, and two runs of
  the SAME build differ by more than the two builds do. One run of each would not
  have shown that, and the first pair taken did read as a one-second regression.
  It also has to be unchanged, because at the shipped settings the intersection
  produces the same instant the phase budget alone did. Pinned by
  `tests/reasoner_global_budget_test.rs`.
- **Definition realization skipped every individual without a named
  `rdf:type`.** `realize_definitions` iterated `individual_types`, so an
  individual with no `rdf:type` at all — named only by its own role assertions —
  and an individual typed ONLY by an anonymous class expression were never
  realized into a defined class, however completely they met the definition.
  `build_abox_tableau` has given both a node for some time, because `rdfs:domain`
  binds them and a disjointness axiom can refute them: the consistency check saw
  them and the realizer did not. A false NEGATIVE rather than a false clean,
  which is why it survived. Realization now runs over the same individual
  universe the ABox tableau is built from, so the two cannot drift, and a
  conjunct of an anonymous `rdf:type` discharges the definition conjunct it is
  spelled the same as, because `x : C ⊓ D` entails `x : C`. Widening the universe
  cannot widen what matches: `satisfies` declines everything outside its
  fragment, so an empty named-type set can only fail. Pinned by three tests in
  `tests/abox_realization_test.rs`, one of them a negative control.
- **The three-way differential skipped in CI, and the green tick said nothing
  about it.** The `python` job installed the `dev` extra and nothing else: no
  `cargo build`, so no Rust engine; no elan, so no `oo-horn`.
  `test_horn_three_way_differential.py` ran its own preflight, found neither of
  the two implementations it compares against this one, and skipped every test in
  the file. MEASURED on this tree with the engine hidden: `14 skipped`, exit 0.
  A gate that passes by not running, in the repository that exists to attack
  that. Three more files skipped for want of `pyshacl` and `hnswlib`, two extras
  the job did not install.

  Two things were missing and both are here. The job now BUILDS what the tests
  need — `leanprover/lean-action` on `lean/` for `oo-horn`, a release build of
  the engine, and the `shacl` and `align` extras — and `python/tests/conftest.py`
  makes `OO_REQUIRE_FIXTURES=1` mean on this side what `tests/common/mod.rs` has
  made it mean on the Rust side: a skip is a failure. The rule is GLOBAL rather
  than per-call-site, because in Python a skip needs no helper to write and a
  per-call-site rule would miss the next one; a skip that is deliberate rather
  than missing carries `@pytest.mark.oo_opt_in`, and there is exactly one, the
  twelve-minute full-corpus run. Collection-time skips are caught too, through
  `pytest_collectreport`, or the three `importorskip` files would have stayed
  exempt from a rule written only in `pytest_runtest_makereport`.

  Both directions measured, on this tree, same command: with the engine hidden,
  `4 failed, 1 skipped, 9 errors`, exit 1 — it can fail. With everything present,
  `67 passed, 1 skipped`, exit 0, and the one skip is the opt-in twelve-minute
  differential. Four Rust files in the same position — `certificate_boundary_proptest`,
  `reason_rl_coverage_test`, `rule_syntax_frontend_test`, `reason_horn_emit_test`
  — each call `common::skip_unless` on lake, and lake is installed in the `lean`
  job, but no leg there invoked them: so in the one job that could have run them
  they were not run, and in the job that ran them (`build`, with no lake) they
  skipped. They are now four strict legs in `lean`, and pass.
- **`docs/ci-gates.md`: which gates actually run, as a table.** Thirty-four test
  files in this repository can skip, twenty-five Rust and nine Python. For each,
  the table says what it needs, whether any job provides it, and whether the skip
  becomes a failure there. Nineteen Rust and eight Python files are strict; five
  Rust and one Python file skip wherever they run, and the reason is written down
  for each, with what closing it would cost; the twenty-fifth Rust file is the
  new corpus bench, `#[ignore]`d because it measures wall clock, which the table
  records as not run BY DESIGN rather than letting it read as an oversight.
  `claimcheck_pizza_bench.rs` was the
  one skip in the Rust suite invisible to BOTH halves of the convention: it
  printed a bare `SKIP:` that the `build` job's counter does not match and that
  `OO_REQUIRE_FIXTURES=1` could not promote. It goes through `skip_unless` now
  and still skips everywhere, which is now visible. The three commands that
  regenerate the three lists are at the bottom of the page, so the table can be
  re-derived rather than trusted.

### Known defects
- **`owl:AsymmetricProperty` is not modelled.** It is now DECLARED, in
  `unmodelled_constructs`, alongside `owl:ReflexiveProperty` and
  `owl:IrreflexiveProperty`. Asymmetry constrains a PAIR of edges rather than a
  node's concept membership, so SHIQ without nominals has no label that states
  it; modelling it needs an edge-level clash rule and its own termination
  argument. Before this it was in neither place — not implemented, and not
  declared — so an ontology using it got a clean verdict with no sign that a
  constraint had been dropped.
- **The CLIF export produced files that parse cleanly and yield NOTHING.**
  Every sentence was emitted as `(cl:comment '...' SENTENCE)`, the shape
  ISO/IEC 21838-2's BFO files use. Measured against both CLIF parsers that
  exist, that form is discarded: py-typedlogic returns an empty theory and
  Macleod has no production for it, and both do the same to BFO's own files. A
  file that is formally valid and practically empty is the assurance-laundering
  shape this project exists to attack, and it was in this project's own output.
  The default is now a standalone `(cl:comment '...')` phrase followed by a
  bare sentence, which py-typedlogic reads back at exactly the count the
  exporter reports (7, 161, 251, 107 and 705 sentences over five ontologies);
  `--clif-comments wrapped` keeps the old shape and the docs say what it costs.
- **CLIF comment strings were double-quoted, and quote style was bound to
  operator spelling so no flag combination could emit conforming CLIF.**
  ISO/IEC 24707 A.2.2.2 makes the single quote the string delimiter. The choice
  had been justified by matching the CLIF files ISO hosts for ISO/IEC 21838-2,
  and that corpus has been withdrawn by its own maintainers: BFO's release
  notes of 7 December 2025 say "Comment texts are surrounded by single, not
  double quotes". The ISO-hosted files carry 369 double-quoted `cl:comment`
  forms; BFO master carries 356 single-quoted and none double-quoted. Comment
  strings are now single-quoted in both dialects and the two settings are
  independent.
- **The CLIF text was unnamed.** All 227 COLORE texts are named, Macleod
  refuses an unnamed one with "Error in ontology: bad URI", and py-typedlogic
  otherwise reports the first comment as the theory's name. The text now
  carries the ontology's own `owl:Ontology` IRI where it declares one, written
  bare because Macleod's lexer has no double-quote token.
- **Two false claims in the first-order export docs.** They said a CLIF file in
  one dialect's spelling does not parse in the other's tools; py-typedlogic
  maps both spellings to identical results with identical sentence counts. They
  also implied Macleod reads the `colore` output; it reads neither, because it
  cannot lex an IRI in a symbol position at all. Both are corrected and the
  measurements are in docs/first-order-export.md.
- **A property characteristic overrode an explicit `owl:DatatypeProperty`
  declaration in the first-order export.** FOAF declares `foaf:msnChatID` as
  both `owl:DatatypeProperty` and `owl:InverseFunctionalProperty`, which OWL 2
  DL forbids and RDF-serialised vocabularies assert anyway. The characteristic
  won, so the property was exported as an object property throughout and every
  axiom about it used the wrong symbol. `OwlP2` is a disjoint sum, so `op:p`
  and `dp:p` are unrelated predicates and the difference is not cosmetic. The
  declaration now wins and a characteristic on a data property is dropped and
  named. Found by `tools/fol_differential.py` on its first real run.
- **The goal builder did not map `owl:Thing` and `owl:Nothing`, and did not
  consult the entity-kind classification.** A derived triple
  `X rdfs:subClassOf owl:Thing` became a subsumption under an atomic class
  symbol occurring in no axiom rather than under the translation's `top`, and a
  goal about a data property asked about `op:p` while the axioms spoke about
  `dp:p`. Fifty-two of FOAF's 181 claimed entailments came back as
  disagreements that were entirely this; the remaining six were the
  declaration-versus-characteristic defect above. Also found by the
  differential.
- **The differential exported from the store the reasoner had just materialised
  into**, so every conjecture was entailed by the theory already stating it and
  the run could not fail. A deliberately broken exporter still scored clean
  under it. The reasoner and the exporter now run against separate stores, the
  export loads the certificate's own `asserted.tsv` so the two see the same
  graph down to the blank node labels, and a triple-count mismatch aborts the
  run rather than reporting a differential that cannot fail.
- **The refutation layer counted the OWL 2 RL rules that conclude `false` as
  sixteen, and there are seventeen.** The missing one is `dt-not-type`, from
  Table 8 of OWL 2 Profiles, and the undercount reached a consumer: `oo-refute
  check` told a caller whose refutation was rejected that "fifteen other OWL 2
  RL clash rules have no condition in this checker", one fewer than the truth.
  `lean/OOCert/Refute.lean` now carries the full list with the W3C table each
  rule comes from, so the number can be re-counted rather than trusted, and it
  states the split the number hides: of the sixteen this checker does not
  implement, fifteen are expressible here and simply absent, while
  `dt-not-type` cannot be stated at all, because `OOCert.Semantics` has no
  datatype value space and says so. That is the same ground on which
  `src/reason.rs` keeps the whole `dt-*` family out of the engine, and the
  reason is now written where the count is instead of being inferable from
  another file. Prose and one report string only; no theorem changed, and the
  checker still implements exactly `cax-dw`.
- **`sh:sparql` ignored `sh:prefixes` and merged every `sh:declare` in the
  shapes graph into one prologue.** A prefix bound to two namespaces emitted
  two `PREFIX` lines, SPARQL took the last, and which one won was decided by
  store row order: the constraint pointing at the loser matched nothing and the
  run reported `conforms: true` with nothing in `skipped_constraints`. Two
  shapes files that were the same RDF graph could give opposite verdicts. A
  false clean, in the function the `#132` fix sits next to. Declarations are
  now scoped per constraint through `sh:prefixes`, `owl:imports*` and
  `sh:declare` as SHACL 5.2.1 says; a constraint that names no set keeps the
  permissive whole-graph merge, but an ambiguous merge is refused with a
  recorded reason rather than guessed.
- **`sh:deactivated true` was ignored on a `sh:sparql` constraint node.** The
  constraint ran, its rows were reported, and the run returned a confident
  `conforms: false` where SHACL 5.3 says there are no results. The predicate
  was already honoured on node shapes and property shapes. There was also no
  unimplemented-predicate complement over constraint nodes at all, so any other
  `sh:` predicate written there was invisible; there is one now.
- **Any `sh:select`, `sh:pattern` or `sh:message` containing a typed or
  language-tagged literal was truncated.** The helper that strips a literal's
  quoting searched the whole string for `^^` and `"@` and cut there, so
  `FILTER(?d < "2025-01-01"^^xsd:date)` was chopped mid-query. The constraint
  then failed to parse and was recorded as unrunnable with an error blaming the
  author, leaving `conforms: null`, zero violations and exit 0. Date and
  numeric comparisons are the most common SPARQL constraints there are. The
  value is now read with the RDF parser instead of by string surgery.
- **A blank-node node shape applied every property shape in the file to its own
  targets.** The shape was spliced into the query text, and `_:label` in a
  SPARQL body is a non-distinguished variable, so `[] a sh:NodeShape` meant
  "anything that has a property shape". Two such shapes with disjoint targets
  reported conforming data as non-conforming. Shape terms are now bound by
  substitution, the same mechanism `$this` pre-binding uses.
- **`source_shape` named the enclosing node shape rather than the shape
  carrying the constraint**, for all fifteen property-borne constraint sites.
  Two property shapes on one path under one node shape therefore stayed
  indistinguishable, which is the case #131 was filed about. pyshacl returns
  the property shape and this now agrees. The node shape is still reported,
  under `node_shape`.
- **The reasoner was not a fixpoint of its own rule set.** Only the `rdf:type`,
  `rdfs:subClassOf` and `rdfs:subPropertyOf` indices were rebuilt each
  iteration; every schema index was filtered once out of the run-start
  snapshot, so a schema triple the reasoner itself derived was never used and
  running `reason` twice derived more than running it once. That matters most
  for certificates: materialising turns one run's conclusions into the next
  run's premises, so `asserted.tsv` could list the reasoner's own output as an
  axiom.
- **Four rules could conclude a triple with a literal subject.** `prp-symp`,
  `prp-inv1`, `prp-inv2` and `eq-sym` take their conclusion's subject from an
  object position, so a literal object produced a triple no serialisation can
  express. Materialisation then failed on the whole batch, losing every
  inference including the sound ones, at a line number that moved between runs
  because it depends on hash iteration order.
- **`cls-hv1` was not W3C's `cls-hv1`.** It was the composite of `cax-sco` and
  `cls-hv1`: it required an explicit `rdfs:subClassOf` hop, so an individual
  typed directly with the restriction derived nothing, while the certificate
  put a W3C rule name in front of the reader. The W3C form is used now and
  loses nothing, since the composite case is `rdfs9` followed by it.
- **`rdfs3` dropped every range inference onto a blank node.** The guard
  required an IRI when the invariant it needs is "not a literal", so a
  blank-node value never got typed and `rdfs9` starved behind it.
- **`oo-cert` exited 1 rather than the documented 2 when a certificate file
  could not be read**, so a harness written to the contract reported an
  unreadable file as a rejected certificate.
- **The `lean` CI job could not pass.** The `lake --version` probe ran from the
  crate root, the one directory with no `lean-toolchain` in its ancestry, and
  `leanprover/lean-action` installs elan with no default toolchain. The probe
  reported lake as missing and `OO_REQUIRE_FIXTURES=1` turned that into five
  panics. Every local run was green because a developer machine has a default
  toolchain.
- **`sh:sparql` reported `conforms: true` when any single focus node
  conformed (#132).** The author's SELECT was wrapped as a subquery under a
  `VALUES ?this` clause. A subquery is evaluated bottom-up with no outer
  variable in scope, so `$this` was unbound inside it, `FILTER NOT EXISTS`
  asked whether ANY node matched, and one clean record hid every dirty one;
  when every record failed the empty solution joined with all of them and the
  count looked right. `$this` is now pre-bound per focus node through
  Oxigraph's substitution, which is the mechanism SHACL-SPARQL 5.3.2
  specifies. Blank-node focus nodes, previously excluded because VALUES
  cannot name them, are evaluated. On the 39-shape case from the issue the
  engine and pyshacl now agree exactly: 249 results, 245 distinct
  record-shape pairs. A bound `?message` overrides `sh:message`, `?path`
  becomes `result_path` and `?value` becomes `value`.
- **Violations now name the shape and constraint component that produced
  them (#131).** Every violation carries `source_shape` (the shape IRI),
  `source_constraint_component` (the `sh:*ConstraintComponent` IRI) and
  `result_path` wherever a path is known. Existing keys are unchanged.
- **`owl-rl-ext` derived the converse of a subclass axiom.** `cls-svf1`
  inferred `x rdf:type C` from `C rdfs:subClassOf ∃p.D`, `x p y` and
  `y rdf:type D`, and treated `x p D`, with `D` the filler class IRI itself,
  as a witness. Neither has a sound rule; both derivations are gone. The
  `owl:equivalentClass` case that made the first look right is carried by
  `rdfs9` over the `rdfs:subClassOf` triple `scm-eqc` emits. Found while
  giving every rule a soundness proof for the certificate checker.
- **Malformed `owl:intersectionOf` and `owl:unionOf` lists no longer fire the
  class rules.** A list is read only when every node carries `rdf:first` and
  `rdf:rest` and the chain reaches `rdf:nil`; the old lenient walk derived
  from whatever it recovered.

## [1.3.0] - 2026-09-04

> `v1.2.1` was tagged from inside this range (`5208de8`) without a version bump
> and without a section of its own, so the binary published as
> `ghcr.io/fabio-rovai/open-ontologies:1.2.1` reports itself as 1.2.0. Its 114
> commits are documented below rather than separately, and the bump that should
> have accompanied that tag lands here.

### Fixed
- **`reason` published inferences as assertions.** Materialised triples were
  merged into the default graph with no marker, so an inference was
  byte-identical in form to a statement a person had written and `save` wrote it
  out: a source of 8 triples came back as 9, newly asserting
  `<http://ex.org/ghost> a <http://ex.org/Person>`, true only under a range
  entailment over a dangling reference. `InferenceTarget::Inferred`, reachable
  as `onto_reason`'s `inference_graph` option, materialises into the named graph
  `https://open-ontologies.org/graph/inferred` instead. A separate graph rather
  than a marker triple, chosen on failure direction: a marker obliges every
  consumer to filter and the one that forgets publishes an inference as an
  assertion, while a separate graph means the consumer that forgets sees fewer
  triples and never wrong ones. Naming the graph is not what closes it, since
  `serialize` flattens named graphs for the triple formats by design; that one
  graph is now withheld from Turtle, RDF/XML and N-Triples, and kept for TriG,
  N-Quads and JSON-LD, which carry the name and lose nothing. Opt-in, default
  unchanged. `owl-dl` refuses the target rather than merging into the default
  graph while the caller believes otherwise. ADR in
  `docs/decisions/0001-an-inference-is-not-an-assertion.md`.
- **A soundness test read an unfinished reasoner run as a verdict.**
  `is_consistent()` read `consistent` and ignored `complete`. Consistency starts
  true and is falsified by finding a clash, so a run that hits a budget reports
  `consistent: true` with `complete: false`, and reading only the first field
  treats "I ran out of budget" as "I proved it". Two assertions failed once on a
  loaded machine with a message accusing the reasoner of unsoundness; the engine
  had set the flag correctly and the reading was wrong. The assertions are
  unchanged in strength, and an unfinished run now fails saying what happened.
- **The DL reasoner discovered entities by their typing declarations rather than
  by the axioms that use them, and its headline `consistent` flag answered for
  the TBox alone.** Three defects, one root cause, found from the outside by
  probing the shipped binary with deliberately broken ontologies rather than by
  reading the code. A class declared only through `rdfs:subClassOf`,
  `owl:equivalentClass` or `owl:disjointWith` - never typed `owl:Class` - was
  invisible to the satisfiability sweep, so an unsatisfiable class was reported
  satisfiable by omission. An individual typed into a plain class - never typed
  `owl:NamedIndividual`, which instance data in the wild almost never is - never
  reached the ABox check, so the textbook inconsistency of one individual in two
  disjoint classes sailed through. And even when the ABox check DID prove an
  inconsistency, `reason owl-dl` published `"consistent": true` in the same JSON
  object as the proof of false, because the flag was computed from the TBox
  alone. Classes are now harvested from all three axiom positions, individuals
  from any rdf:type pointing at a known class (with schema declarations
  excluded, so a class never doubles as an individual), and the headline flag is
  the conjunction of TBox and ABox verdicts, with `tbox_consistent` exposed
  separately. The three-valued discipline is preserved: an undecided ABox still
  defaults to consistent. The probes that exposed all three are preserved as
  regressions in `tests/tableaux_discovery_test.rs`.
- **Three more tools answered about the default graph alone, so their answers
  depended on the serialisation the data arrived in.** Same defect as the two
  halves of #108, found by loading identical content as Turtle and as TriG and
  comparing the reports rather than by reading the code. `onto_vocab_check`
  read zero declared terms from an ontology loaded as TriG or N-Quads and
  bailed with a warning telling the caller to load an ontology they had already
  loaded. `onto_communities` reported no communities and the note "no relations
  between named subjects and objects", asserting a fact about the corpus it had
  not checked. Shape induction counted no instances and returned an empty
  lattice, indistinguishable from a class that genuinely has none. All three now
  read the union of every graph. `tests/serialisation_invariance_test.rs` pins
  the rule and names the modules still unmeasured, since roughly thirty run
  internally authored SELECTs and only four have been checked (#108).
- **`onto_shacl` validated the default graph and nothing else, so whether data
  was checked at all depended on the serialisation it arrived in.** Every
  data-side query ran through the store's default dataset specification, which
  is the default graph alone, so an ontology and its instances loaded from
  Turtle validated while the identical triples loaded from TriG or N-Quads
  selected no focus nodes: `focus_nodes: 0`, `unmatched_shapes` populated, and
  the `nothing_matched` null verdict. That is a truthful answer to a question
  nobody asked, which is why it never arrived as a bug report. The data-side
  queries now read the union of every graph in the store, added as
  `GraphStore::sparql_select_union` so the plain `sparql_select` keeps the
  dataset a hand-written query expects. `GRAPH ?g` still ranges over the named
  graphs, so nothing written against the old form changes meaning.
  **This can turn a null or a vacuous pass into a real `conforms: false`**,
  which is the point: the violations were always there and were not being
  looked at. Reports now carry `scope`, today always `all_graphs`, so a verdict
  says what it selected over before temporal scoping makes that vary (#108).
- **A constraint asserted on the node shape itself was never evaluated and
  never reported, so `sh:closed true` over data carrying an undeclared
  predicate returned `conforms: true`.** The check that routes unimplemented
  constraints to `skipped_constraints` reached one `sh:property` hop below the
  shape and no further: `sh:closed`, a node-level `sh:not`, `sh:nodeKind`,
  `sh:and`, `sh:or`, `sh:xone`, `sh:in` and `sh:node` never bound the
  predicate it inspects. A second complement now covers the shape node, with a
  whitelist of the predicates the validator reads there (the target forms,
  `sh:property`, `sh:sparql`) plus the annotation predicates, which are never
  constraints, so any other `sh:` predicate on a shape the `sh:targetClass`
  discovery query returns lands in `skipped_constraints` and the verdict becomes
  null. A shape whose only target is `sh:targetNode`, `sh:targetSubjectsOf` or
  `sh:targetObjectsOf` is not returned by that query, so its node-level
  constraints are still dropped without a record. The shape is bound as a query variable and matched
  to the discovered shape, not spliced into the query text: a shape written
  `[] a sh:NodeShape` is a blank node, and a blank-node label inside a SPARQL
  query is a wildcard, not a name. The complement runs once per shape rather
  than once per target class, so a node constraint is recorded once.
  `sh:deactivated` is among them on purpose: a deactivated shape is still
  evaluated here, so the predicate is not honoured and must not read as if it
  were. The complement is restricted to the `sh:` namespace, because an
  implicit class target carries its own class axioms on the same subject and
  those are not constraints. A test now asserts the null verdict is reachable
  from the node shape, from a property shape and from a target, each naming
  its construct, so the next construct added has somewhere obvious to fail.
  Two predicates are exempt at a false value and only at a false value:
  `sh:closed false` is the SHACL default and restricts nothing, and
  `sh:deactivated false` asks for the evaluation this validator performs, so
  both are honoured in full and neither may suppress the verdict. A null on a
  run where nothing went unevaluated is a false undetermined, and it costs
  what the false clean costs, since a null that fires on a complete run
  teaches the reader to ignore null. The value is read by value rather than by
  lexical form, and a control whose value is not a boolean at all stays
  skipped, because a value this validator cannot read is not one it can
  honour (#108).
- **`onto_shacl_check` reported a class or property declared inside a named
  graph as missing.** The three existence lookups behind `missing_target_class`,
  `missing_class_constraint` and `missing_path` ran a bare triple pattern, whose
  default dataset is the default graph only, so an ontology loaded from TriG or
  N-Quads, where every declaration sits in a `GRAPH` block, produced one issue
  per referenced term while the same data in Turtle produced none. A declaration
  is a declaration wherever it lives: the lookups now read the union of the
  default graph and every named graph, unconditionally, with no scope argument
  and no new response key. A class declared in no graph at all is still flagged
  (#108).
- **A daemon started with `[http] token` in config rejected every command it was
  meant to serve.** `serve-http` falls back to the config token and then enforces
  bearer auth, but `daemon start` recorded `token: null` in `daemon.json`
  whenever no `--token` flag or env var was given. Every proxied command then
  sent no `Authorization` header to a daemon demanding one, got 401, and the
  client treated that as fatal, so a single `daemon start` turned the whole CLI
  into an error until `daemon stop`. The token is now resolved the way the child
  will resolve it, from a config path both sides are given explicitly, and a
  daemon that still rejects the client is announced on stderr and fallen back
  from rather than being fatal.
- **`--data-dir` was ignored by the server arms, so a daemon could serve a
  different store than the one its caller was using.** Both `serve` and
  `serve-http` derived their data directory from `[general] data_dir` in config
  and dropped the flag, so `--data-dir /custom daemon start` wrote `daemon.json`
  into `/custom` while the daemon it started served `~/.open-ontologies`. Proxied
  and local commands then saw different data with nothing to indicate it, and
  only when the two paths differed, which is why it survived casual use. The flag
  now takes precedence over config, matching how host, port and token already
  resolved.
- **`push` could not run at all while a daemon was up.** It was serialized for
  proxying but had no arm in `BatchRunner::execute`, so it fell through to
  `unknown batch command` and exited 1 — the only proxy-able command with no
  handler. Running it locally instead would have been worse than the error, since
  the store holding the triples worth pushing is the daemon's. A test now asserts
  that every command the CLI proxies is one the batch runner answers to, because
  the two sides are matched by string across an HTTP boundary and nothing else
  checked that they agreed.
- **Relative paths resolved against the daemon's working directory rather than
  the caller's.** The daemon inherits wherever `daemon start` ran and never
  learns where its caller is, so `cd /data && open-ontologies load ./x.ttl`
  either failed or loaded a different file, and `save ./out.ttl` wrote somewhere
  the caller was not looking. Path arguments are made absolute before they are
  sent.
- **A multi-line or awkwardly quoted argument was destroyed in transit to the
  daemon.** Commands were proxied as a command line that the daemon
  re-tokenized, and `parse_lines` splits on newlines before it looks at quotes,
  so a multi-line SPARQL query — the normal kind — arrived torn across lines and
  failed as an unterminated quote while the identical command succeeded locally.
  The double-quote fallback in the quoting helper also failed to escape
  backslashes, mangling any argument carrying both quote styles. Commands are now
  proxied in the structured form, one argument per array element, where neither
  is possible.
- **A successful `daemon start` reported the daemon it had just started as
  dead.** The human renderer guessed which command produced a payload by
  sniffing its keys, and `{ok, pid, url}` matched a branch written for
  `daemon status` — a branch that defaults liveness to false. That branch now
  requires an explicit `alive`, and on the proxy path the renderer dispatches on
  the command name the batch envelope already carries instead of guessing.
- **`marketplace` answered with a different catalogue depending on whether a
  daemon was running.** The command has two implementations behind it, the local
  one and the batch one that serves it when a daemon is up, and they had drifted:
  the batch copy consulted only the curated catalogue and never loaded community
  packs, so `marketplace list` lost the community tier and `marketplace install`
  refused a community id, with nothing to indicate why. Both now go through
  `marketplace::cli_list` and `marketplace::cli_resolve`. The MCP tool keeps its
  own richer shape, which reports urls, maintainers and shadowing warnings, and
  is documented as a separate surface rather than a third copy of this one.
- **`/api/batch` opened the state database on every request.** The route built a
  fresh `StateDb` per call, on the exact hot path the daemon exists to
  accelerate, while the adjacent `/lineage` route already cloned the handle
  opened at startup. It now clones the same one.
- **`temporal:recordedUntil` closes the transaction interval, so `as_of`
  answers what was believed then.** The recorded axis only ever narrowed
  forward: with no upper bound, `as_of = now` returned the union of every
  version ever recorded rather than the current belief, and after the first
  correction a snapshot showed an assertion beside the one that replaced it.
  `validities()` now reads the predicate as a fourth UNION branch and
  `recorded_by` is two-sided, half-open like `[validFrom, validTo)`.

  Exclusions are reported on the side they fall: a graph recorded after the
  audit instant still says `not yet recorded then`, one whose recorded interval
  has closed says `no longer recorded then`. Those are opposite facts and a
  single reason string flattened them.

  Additive. A store that does not write `recordedUntil` is unchanged, including
  its cap arithmetic — the validity scan counts ROWS, so only a graph that
  actually carries the fourth predicate costs a fourth row. For stores that do,
  the 20,000-row cap covers roughly 5,000 fully described graphs where it
  covered roughly 6,700 with three predicates; there is a test that proves the
  cost rather than asserting it in a comment.
- **`onto_temporal_conflicts` stops calling every non-overlapping pair a
  correction.** The check is `!overlaps`, which proves the two periods share no
  instant and nothing more. It does not establish that one assertion replaced
  the other -- the data carries no link that would -- and the same bucket also
  holds pairs separated by a GAP, which is missing coverage rather than
  history. The results now come back under `non_overlapping` /
  `non_overlapping_count`, and the `note` says what was checked instead of
  claiming adjacency the code never tested -- including the comparison that
  backs it. When this landed, bounds were still compared as text, so two
  periods written with different timezone offsets could share an hour and
  still land in `non_overlapping`; a conformance test pinned that so the
  parsed-time work would turn it into a contradiction as a diff rather than a
  new assertion. Bounds are now read as instants (see Changed below), offsets
  are honoured, that test has flipped on the lines that held the old answer,
  and the note says the comparison is on instants.

  `superseded` / `superseded_count` are still emitted, unconditionally and
  behind no flag, carrying exactly the same rows until 2.0. Deprecated, not
  renamed: when lineage-backed supersession arrives it takes a new key, so no
  key ever names a different set on either side of a major version.
- **A period that holds at no instant no longer overlaps anything** (#118).
  `Period::overlaps` answered true for `[t, t)` against any period containing
  `t`, and unconditionally against an open one, while `valid_at` was false at
  every instant for it, so `onto_temporal_conflicts` filed such a graph as a
  live contradiction where `onto_temporal_snapshot` excluded it everywhere; an
  inverted `[t2, t1)` with `t2 > t1` was filed as `non_overlapping`, a graph
  that holds nowhere presented beside a live one. The case that made it worth
  fixing is the one parsing created: `validFrom "2024"^^xsd:gYear` beside
  `validTo "2024-01-01"^^xsd:date` is two careful assertions from two systems
  writing at different precision, invisible on inspection, and it resolved as
  sound. The classification now happens once, where the bounds are read:
  `Bounds::resolve` yields a third `GraphValidity` beside sound and
  unreadable, the snapshot answers from the variant, `overlaps` from the flag
  the period carries, `valid_at` is never asked about such a period, and the
  two tools agree by construction. With no `valid_at` such a graph is in scope
  like any other, because no instant was asked about and narrowing an
  atemporal query silently is worse than answering it, so `onto_temporal_query`
  with no `valid_at` still reads it; at any instant asked about it is excluded.
  Validity is checked before the recorded side: with a `valid_at` given,
  "holds at no instant" beats "not yet recorded then" whatever `as_of` is
  passed, since only the first is a fact about the data rather than about the
  query; with none given, the recorded reason is the only one that can apply.

  Not `invalid`: both bounds parsed, and every reason in that array says a
  bound could not be read. The graph is EXCLUDED at every instant asked about,
  with a reason that says it holds at no instant and names which of the two it
  is, since validFrom equalling validTo is sometimes intended and validTo
  preceding validFrom almost never is. In
  `conflicts` the pair lands in `non_overlapping`, a true statement that since
  the rename above claims no correction, and the `note` says the bucket can
  hold such a period. The recorded axis is not classified this way here:
  `recordedUntil` before `recordedAt` is out-of-order recording (#109) and
  lands separately.

- **`open-ontologies-lite` no longer installs an MCP server with the library**
  (released as 0.5.0). `mcp` was a core dependency of a package that imports it
  in exactly one module, `server.py`, so every consumer of the library API
  installed a server they had not asked for.

  That was not only weight. `mcp` pulls `starlette`, and forcing that upgrade
  makes a resolver re-solve the whole environment. Measured against a semantica
  0.6.6 checkout: `pip install open-ontologies-lite==0.4.0` **uninstalled
  fastapi**, and every import of that project's web layer then failed with
  `ModuleNotFoundError`. Three of its security-regression tests went from
  passing to erroring on that alone.

  To be precise about the cause, because the first version of this note was not:
  a current fastapi does accept the starlette that `mcp` wants, verified as
  fastapi 0.141.1 beside starlette 1.6.0 in a clean environment. The breakage is
  what the upgrade does to an environment that already holds a pinned or older
  fastapi, which is what a real project has. Either way the fix is the same, and
  a package whose job is verifying someone else's graph has no business
  re-solving their web layer to do it.

  `mcp` moves behind a `server` extra, which the console script and
  `python -m open_ontologies_lite` need and the library API does not.
  `server.py` names the extra when it is missing rather than failing on a bare
  import line. A test runs the library API in a fresh interpreter with `mcp`
  forced unavailable. 0.5.0 rather than 0.4.1 because the install line changes
  for anyone who relied on the bare install giving them the server, even though
  no API moved.

- **JSON-LD is read and written like any other serialisation.** `oxrdfio` has
  supported it all along, but the engine's format handling did not: `parse_format`
  had no JSON-LD arm, and `detect_format` fell back to Turtle for any extension it
  did not recognise, so a `.jsonld` document was handed to the Turtle parser and
  died on `{ is not a valid predicate` — an error that names the wrong problem and
  sends you looking at a file that was never broken. `.jsonld` and `.json` now map
  to JSON-LD, `parse_format` accepts `jsonld`, `json-ld` and `json` (`json-ld` is
  the W3C media-type spelling and what most other tooling takes, so rejecting it
  turned a correct format name into an error), and the body sniffer recognises a
  JSON-LD document published under a misleading extension. The sniff requires an
  opening `{` or `[` *and* a `"@context"`, `"@id"` or `"@graph"` keyword: a bare
  brace proves nothing, since TriG opens its default graph block with `{` and
  Turtle admits `[` as a blank-node subject. `tests/graph_jsonld_test.rs`.
  The Python package had the mirror defect, accepting `jsonld` while rejecting
  `json-ld`; both spellings and `json` now resolve.

- **The compile cache no longer flattens named graphs** (issue #112). It was
  written as N-Triples, a format that cannot carry a graph name, so the cached
  artefact was not equivalent to the source it stood for: a dataset went in and
  a flattened graph came out. The first load parsed the source and answered
  correctly; every load after it read the cache back and returned a store with
  no named graphs at all. Measured on an unchanged TriG file, twice through
  `onto_load`: `origin: "source"` gave two named graphs, `origin: "cache"` gave
  none, and `onto_temporal_snapshot` reported `{"ok": true, "in_scope": []}` —
  a clean, confident, empty answer.

  The freshness key `(source_path, mtime, size, sha256)` was never at fault and
  is unchanged; the cache was correctly judged fresh, and what it held was
  wrong. A second path needed no second load at all: an idle-evicted ontology
  reloads through `ensure_loaded`, from that same flattened file, so a
  long-running `serve-http --idle-ttl-secs` lost its named graphs by being
  idle.

  Cache files are now N-Quads (`.nq`) — line-based and just as fast to parse,
  which was the reason N-Triples was chosen, but able to name a graph. The
  extension doubles as the format marker: an entry pointing at a `.nt` file was
  written before this fix, holds a flattened dataset, and is recompiled instead
  of read. That costs one re-parse per ontology, once, and heals warm caches
  rather than leaving them quietly wrong. Anything whose meaning lives in the
  graph name is affected — the bi-temporal tools above all, which read
  `GRAPH ?g` and saw an empty store. Three tests in `tests/registry_test.rs`,
  each loading TWICE on purpose: a test that loads once passes on the broken
  code, which is why this went unnoticed.

- **Named graphs are preserved when exporting to TriG and N-Quads.**
  `GraphStore::serialize` rendered every format with `serialize_triple`, which
  flattens a quad from a named graph into the default graph. Import and the
  persistent store keep graph names, so the loss was export-only — but it meant
  a TriG save/reload round trip dropped the named-graph structure that
  bi-temporal assertions live in (issue #95): every `validFrom`/`validTo`
  binding on `onto_save`/`onto_convert` output was silently gone. The dataset
  formats now serialize quads; the triple formats (Turtle, N-Triples, RDF/XML)
  keep flattening, which is the only thing they can represent. TriG and N-Quads
  round-trip tests in `tests/graph_named_graph_roundtrip_test.rs`.

- **The bi-temporal tools say when a scan was cut short.** Four queries behind
  `onto_temporal_snapshot`, `onto_temporal_query` and `onto_temporal_conflicts`
  are capped — 20,000 validity rows, 20,000 named graphs, 10,000 result rows,
  5,000 disjointness pairs — and a store that reached one got a confident
  answer with nothing to say it was partial (issue #95). Every response now
  carries `complete`, and a cut run also carries `truncated`: one
  `{scan, limit, consequence}` entry per cap that bit.

  Truncating a result list gives you fewer rows. Truncating the validity scan
  gives you a WRONG answer, so those responses also carry a `warning`. A graph
  whose validity rows fell past the cap reads as having no validity at all, and
  an undescribed graph is timeless and always in scope — the opposite of the
  truth for a graph whose period had ended. In `onto_temporal_conflicts` the
  same gap is a false positive rather than a gap: a timeless period overlaps
  everything, so a superseded correction is republished as a live
  contradiction, which is the one thing that tool exists to prevent.
  `onto_temporal_query` inherits the verdict from the scope it ran over,
  including on the empty-scope path, where "no graphs in scope" may mean
  nothing among the graphs that were read.

  Detection is proof rather than inference: each scan is sent with `LIMIT n+1`
  and the extra row, if it arrives, is evidence that more exist. It is dropped
  before the response is built, so a store holding exactly the cap is reported
  as complete, and every untruncated response keeps its 1.2.0 values. 10 tests
  in `src/temporal.rs`, including one pinning an exactly-full scan as NOT
  truncated and one showing a correction turn into a false contradiction when
  the validity scan is cut.

### Changed
- **Temporal bounds are read as instants on the UTC timeline, not compared as
  text** (issue #95). `onto_temporal_snapshot`,
  `onto_temporal_query` and `onto_temporal_conflicts` accept `xsd:date`,
  `xsd:dateTime`, `xsd:gYearMonth` and `xsd:gYear`; a less precise bound names
  the FIRST instant of the period it names, so `"2026-05-01"^^xsd:date` as a
  `validTo` excludes the whole of 1 May and `"2026"^^xsd:gYear` as a
  `validFrom` starts at midnight on 1 January. A value with no timezone offset
  is UTC: XSD leaves such a value only partially ordered against one that
  carries an offset, and "indeterminate" is not an answer a register query can
  return. All four axes are read this way, `recordedUntil` included: a closing
  bound written with an offset or at month precision closes the recorded
  interval where its instant falls, not where its text sorts, and one that
  matches no grammar makes the graph invalid by the same rule as the other
  three. Every response now carries `semantics_version` (`temporal/2`), since
  the same store answers differently under the two readings and an answer that
  does not say which produced it cannot be replayed or hashed.

  A fractional second is refused only when a digit past the ninth is NONZERO.
  A zero tail is not extra precision: `.1234567890` is 123,456,789 nanoseconds
  exactly and names the instant `.123456789` names, so refusing it made two
  spellings of one instant answer differently, the thing this change resolves
  everywhere else. Fixed-width formatters pad to a fixed digit count, so the
  tail arrives from machines rather than from typos, and it reaches both the
  store bounds and the `valid_at` / `as_of` arguments. Bounds typed
  `^^xsd:dateTime` were shielded by the store's own canonicalisation; bare and
  `xsd:string` bounds (the form this module documents as supported, and the
  form the conformance corpus is written in) were not.

  **Same-precision data with four-digit years, no offsets and no sub-nanosecond
  fractions answers exactly as it did.** That is the constraint this was built
  against, and the divergence is narrower than it sounds: for a coarse value
  and a fine one where the coarse is a lexical prefix of the fine, text said
  "earlier" and instants say "equal", so mixed precision moves ONLY at the
  boundary instant. The inputs whose answers move:

  - **Mixed precision at a boundary.** A date-typed `validTo` against a
    `dateTime` instant at midnight now excludes its own day instead of
    admitting it. Same on the recorded axis: an `as_of` at date precision
    against a `recordedAt` of `T00:00:00Z` on the same day now sees the record,
    which is the inclusive cutoff the tool documents and text comparison
    quietly did not deliver.
  - **Bounds carrying an offset.** `"2026-05-01T00:00:00+02:00"` is
    `2026-04-30T22:00:00Z` and is now compared there, rather than sorting after
    every `2026-04-…` string.
  - **Years that are not four digits.** `"20241-01-01"` and `"-0044-03-15"` now
    order by year rather than by first character, so a graph bounded by one is
    in scope inside its own interval instead of nowhere.
  - **Bounds that match none of the four grammars** (a foreign datatype, a
    language tag, `"01/05/2026"`, `"2024-1-1"`, a calendar-impossible
    `"2024-02-30"`, a fraction finer than a nanosecond) make the graph
    INVALID. It is reported in a new `invalid` array with a per-graph reason,
    excluded from `in_scope`, and above all NOT timeless: "we hold no
    valid-time claim about this" and "the claim is garbage" are different
    answers. Previously such a bound was compared as raw text, so
    `"01/05/2026"` sorted before every ISO value in the store and its graph
    read as valid since the beginning of time.
  - **Two different instants on one axis.** Previously the last row of the
    validity query's UNION won and the other value vanished, on an order the
    query never contracted. The graph is now invalid and both values are
    named: choosing one, or the min, or the max, would publish an interval
    nobody asserted. Two SPELLINGS of one instant (a bare `"2024-01-01"` beside
    `"2024-01-01"^^xsd:date`, which is what a half-finished migration leaves)
    still resolve to that one instant: they invent nothing. So do a coarse
    bound and a fine one naming the same instant, `"2024"^^xsd:gYear` beside
    `"2024-01-01"^^xsd:date`, because agreement is judged on the instants and
    not on the strings; the row shows the lexically first form.
  - **`valid_at` / `as_of` that cannot be read** are now refused rather than
    ignored. Silently dropping one answered a question nobody asked, with the
    whole store in scope and nothing saying why.
  - **`onto_temporal_conflicts`** gains `undecided` and `undecided_count` for
    pairs where a graph's temporal metadata could not be read on at least one
    axis, so the graph is invalid and the pair is classified neither way. An
    unreadable period is not an open one; treating it as timeless would make it
    overlap everything and publish a correction as a live contradiction, which
    is the failure that tool exists to prevent. `non_overlapping` and the
    deprecated `superseded` alias are unchanged, and the `note` now says the
    disjointness check runs on instants, so an offset is honoured.

  A note on `xsd:string`: RDF 1.1 makes a simple literal and an
  `xsd:string`-typed literal with the same lexical form the SAME term
  (`datatype()` answers `xsd:string` for both), so a bound cannot be rejected
  for carrying that datatype: the store holds no such fact to report. Untyped
  bounds are read by shape against the four grammars, which rejects
  `"01/05/2026"` either way while leaving every store written with plain
  literals answering as before. A value wearing one of the four datatypes but
  matching another of them (the shape this crate's own module doc shipped for
  `recordedAt`) is read as what it is. 8 grammar tests in `src/temporal.rs`, and
  the conformance corpus in `tests/temporal_conformance_test.rs` goes from 24
  tests to 34: the four cases pinned for this change, and the offset pair the
  `non_overlapping` fix pinned, flip on the lines that held the old answer, and
  a third section covers what parsing adds.

### Added
- **`onto_defects`, and a `defects` CLI subcommand: the ontology is checked
  against itself before any data is judged by it.** A self-contradicting
  ontology makes every fact-level conclusion suspect, and a reasoner amplifies
  whatever the declarations say. A different question from `onto_dl_check`:
  satisfiability asks whether a model exists, these checks ask whether a pair of
  declarations will manufacture contradictions once instances arrive, so a
  property declared both transitive and functional is satisfiable and is still
  reported. Eight kinds, decided from the TBox alone with no data:
  `transitive_and_functional`, `symmetric_and_asymmetric`, `subclass_cycle`,
  `sub_property_cycle`, `disjoint_with_ancestor`, `inherited_disjoint`,
  `self_inverse`, `inverse_not_mutual`. Findings carry a severity, because a
  sweep over the 153 readable ontologies in this repository returned 27 findings
  of which 25 were the mildest kind, and reporting `inverse_not_mutual`, where
  the entailment holds either way, beside a class that can have no instances
  buries the finding that matters. Each kind is listed at most 50 times with the
  true total under `truncated`. The subcommand exits non-zero on unparseable
  input and returns JSON on a missing file.
- **More SHACL constraint components, and all four target forms.** `sh:in`,
  `sh:nodeKind`, `sh:not`, the length bounds, the string and pair constraints
  and `sh:qualifiedValueShape` are evaluated rather than skipped, and focus
  nodes are selected from `sh:targetClass` including the implicit class target,
  `sh:targetNode`, `sh:targetSubjectsOf` and `sh:targetObjectsOf`. Shapes across
  the estate leaned on components the engine could only skip, so real shapes
  graphs got no verdict at all.
- **`tools/shacl_differential.py`, a differential oracle against pyshacl.** A
  validator is trusted on evidence, not on its own test suite. Both engines run
  over the same (data, shapes) pairs and disagreement is ranked by severity: a
  false clean, where we say conforms and pyshacl does not, is the one failure
  mode the validator must not have; a false alarm makes the gate untrustworthy
  in the other direction; an undetermined verdict is honest and is the work
  list. Where both give a verdict the violation sets are compared as
  (focus_node, path) pairs, because agreeing on the verdict is not the same as
  agreeing on why.
- **`docs/UPSTREAM_ISSUES.md`**, recording reproducible defects found in
  dependencies, staged for a human to file rather than filed.
- **`temporal:supersedes` and `temporal:retracts`: lineage that is asserted,
  never inferred** (#109). Three situations looked identical in the data: a
  correction (one authority replacing its own assertion), a disagreement (two
  sources) and a retraction (withdrawn, nothing put in its place).
  `onto_temporal_conflicts` filed a correction that shares its predecessor's
  valid period as a contradiction, and with no explicit `recordedUntil` the
  predecessor stayed believed for ever. Both predicates are written on the
  NEWER graph and name the graph it replaces or withdraws. `recordedUntil`
  alone governs scope and an explicit bound is authoritative; where a graph
  carries none, its closing bound is derived from the inbound link as the
  successor's `recordedAt`, the earliest where there are several, since belief
  ended at the first replacement and a later one does not revive it. Where
  both are present and disagree the explicit value governs and the
  disagreement is reported, not reconciled. Nothing is written: the derivation
  is a join made when the validity map is read, inside `validities()`, so the
  snapshot, the query and the conflict check share it by construction.

  `onto_temporal_snapshot` and `onto_temporal_query` gain two keys, present
  only when non-empty like `invalid`. `retracted` holds rows shaped like
  `excluded`, with `reason: "retracted"`, `retracted_by` and `retracted_at`,
  for graphs a `retracts` link recorded by `as_of` has withdrawn; a retracted
  graph leaves `in_scope`, its triples are not read, and retraction is checked
  before the bounds, so for a readable graph it beats every other reason,
  a derived closing bound included; an unreadable graph stays in `invalid`,
  whatever links name it. `lineage` holds one `{graph, reason, ...}` row per
  thing the links could not settle: an explicit bound that disagrees with its
  successor, a transaction interval that closes before it opens (a successor
  recorded before its predecessor, reported as inverted and believed at no
  instant, never clamped), a successor with no `recordedAt`, more than one
  successor (reported whether the close was derived or asserted, in one row
  shape: `closed_by` names the successor where the bound was derived, and
  `recorded_until` the explicit bound where it was asserted, since no
  successor closed a graph that closed itself), a retractor with no
  `recordedAt` (which withdraws at every `as_of` given: a withdrawal at an
  unknown time is still a withdrawal), a cycle, a link naming the graph
  itself, an undescribed or unreadable graph, or a term that is not an IRI,
  and a link asserted by a graph whose own description could not be read,
  which closes and withdraws nothing and says so rather than vanishing with
  its asserter. A link the pass rejects is pruned from the map it hands on,
  so every consumer walks effective links only: a `supersedes` naming an
  undescribed graph puts no pair in `corrections`, and the disjoint pair it
  fails to link is the contradiction it is, the undescribed graph being
  timeless. An excluded row whose closing bound was derived carries
  `superseded_by`. With no `as_of` a derived bound is read as an asserted one
  is: no instant was asked about on the recorded axis, so the graph is in
  scope. With no `as_of` the recorded axis is not consulted for retraction
  either: a retracted graph takes the ordinary path, in scope unless its
  valid-time bounds exclude it, exactly like a graph closed by a
  `recordedUntil`, explicit or derived. One rule for every recorded-time
  fact. The alternative, an absent `as_of` read as "now" for the whole
  recorded axis, is a one-line reversal that would then have to change
  explicit `recordedUntil` too. `onto_temporal_conflicts` gains `corrections`
  / `corrections_count`, present only when non-empty like `undecided`: pairs
  where one graph supersedes the other, directly or through a chain, checked
  before the overlap test, so such a pair is never a contradiction whatever
  its periods, and the `note` says so. Retracted graphs are not treated
  specially there. `temporal:authority` is description, not a key, and is
  left for a follow-up.

  Cap arithmetic: the validity scan counts rows over a six-way UNION now. A
  store that writes neither predicate is unchanged. A graph carrying the four
  bounds and one link costs five rows, both links six, so the 20,000-row cap
  covers roughly 4,000 such graphs, or roughly 3,300 with both, against
  roughly 5,000 fully bounded ones; a test proves the fifth row. The
  `validities` truncation texts, on the snapshot, the query and the conflict
  check, name the failure the shared cap adds: a `supersedes` or `retracts`
  row cut while its asserter's bounds survived was never seen, so the graph
  it named stays open or in scope although it is described, and a pair the
  link would have made a correction is compared on its periods.
  `semantics_version` stays `temporal/2`: the lineage predicates ship in the
  same release as the parsed bounds, and a store without them answers as it
  did.
- **CLI: Daemon mode — persistent in-memory store across processes.** New `daemon start / stop / status` subcommand launches `serve-http` as a detached background process and writes its PID + URL to `~/.open-ontologies/daemon.json`. All 24 CLI commands that touch the `Arc<GraphStore>` (load, save, clear, stats, query, lint, reason, shacl, enforce, plan, apply, version, history, rollback, pull, push, ingest, drift, lock, monitor, monitor-clear, marketplace, and `batch`) automatically detect a live daemon and route their request to it via the new `/api/batch` HTTP endpoint — no flags, no code changes per-command. Use `--no-connect` to force local execution. Daemon liveness is checked with a bare `kill(pid, 0)` (Unix) / `tasklist` (Windows), rather than forking `/bin/kill` on the path the daemon exists to make fast; stale `daemon.json` files are removed automatically on the next command. New modules `src/daemon.rs` (PID file management + process control) and `src/connect.rs` (HTTP proxy client), and `libc` as a dependency for the liveness check.

  Commands are proxied in the structured form of `/api/batch` — `{"command": name, "args": [..]}`, one argument per array element — rather than as a command line the daemon re-tokenizes. The lossy round trip was the source of most of what is fixed below: an argument cannot be torn on a newline, mangled by a quoting rule, or resolved against the wrong directory once it is an array element that reaches the daemon exactly as clap parsed it. Path arguments are made absolute before they are sent, and an invocation carrying a flag the batch handler has no arm for is run locally rather than proxied without it. `daemon start` waits for the port to accept a connection instead of sleeping a fixed 600ms, so it neither burns the wait when the bind is fast nor reports success for a child that never bound.
- **CLI: `marketplace` command works via daemon.** `marketplace list [--domain <d>]` and `marketplace install --id <id>` are now proxy-able through the daemon's `/api/batch` endpoint. Installing an ontology from the marketplace with a daemon running loads it into the daemon's persistent store so subsequent `stats`, `query`, and `reason` calls in any other process see the installed triples. Implemented via a new `exec_marketplace` async handler in `BatchRunner`.
- **CLI: Human-readable output behind `--human`.** Passing `--human` renders results as text rather than JSON (e.g. `stats` prints a plain key/value block; `load` prints "Loaded N triples from path"; errors print "Error: message"). JSON remains what every command prints when no format is asked for, so existing scripts and any consumer reading the CLI's stdout are unaffected; `--json` is accepted as an explicit statement of that default, and `--pretty` still gives indented JSON. The branch originally made text the default and JSON opt-in, which `tests/cli_load_ephemeral_test.rs` catches: it runs `load` with no flags and parses stdout, so the flip would have broken every unflagged caller silently, a prose response being detectable as wrong only at the consumer. A `OnceLock<bool>` global (`JSON_MODE`) is resolved once at startup and read through `json_mode()`, which defaults to JSON on any path that runs before startup has set it; `output_json`, `output_result` and both daemon-proxy call sites consult it, so local and proxied commands cannot disagree about the format. Daemon proxy output now matches local output exactly: the `seq`/`command` batch envelope is stripped and only the `result` value is rendered. Human-readable rendering lives in the new `src/output.rs` module (`render_human`), which handles stats tables, SPARQL result tables, marketplace lists, load/save/install confirmation lines, lint/enforce issue lists, version history, SPARQL bindings, and a pretty-JSON fallback for unknown shapes.
- **Studio: Multilingual label filter in Tree view.** `TreeView` gains a language chip bar in its header, listing every BCP-47 tag present in the loaded ontology's `rdfs:label` values and shown only when more than one exists. Labels are loaded with full language-tag preservation (two-pass: collect all variants into a `labelMapRef`, then `pickLabel` to build nodes). Switching language relabels the existing React tree state in-place (`relabelTree` walk) — no SPARQL re-query. `nodeMapRef` is also updated so breadcrumbs and connection chips reflect the selected locale. Fallback chain: preferred language → `en` → untagged literal → first available → URI local name.
- **Studio: Language badges and filter in Property Inspector.** `PropertyInspector` now parses language tags from both engine-native (`"value"@lang`) and standard SPARQL JSON (`xml:lang`) response formats. Each literal row that carries a language tag displays a small monospace badge (e.g. `en`, `cs`) to the right of the value. A language chip bar above the property list (shown only when ≥1 language tag is detected) lets users filter rows to a single locale; URI values are always shown regardless of the filter. `saveEdit` and `deleteProp` include the original language tag in the SPARQL `DELETE`/`INSERT` pattern so editing one locale does not affect sibling translations. The **+ Add** form gains an optional language tag input with quick-pick chips for languages already present on the node.
- **Optional eviction of float32 text vectors from memory**
  (`VecStore::with_text_vectors_evicted`, `turbovec` feature). Without it the
  TurboQuant backend's compression is a smaller SQLite blob rather than less
  RAM, because the 4 bit codes and the float32 vectors they were made from are
  both resident. In eviction mode the float32 lives only in the `embeddings`
  table: upserts write through to the row before touching memory, removals
  delete it, and reads load on demand. Three new accessors carry every path
  that used to reach into the entry map directly: `fetch_text_vecs` pulls just
  the shortlist for the exact re-score (the hot path, one query rather than one
  round trip per candidate), `all_text_vecs` streams the whole set IRI-sorted
  for the paths that genuinely need every vector, and the public
  `load_text_vec` returns one owned vector in either mode. New
  `resident_text_vector_bytes()` reports what is still held.

  `entries_fingerprint` is now public and reads through `all_text_vecs`, so an
  evicted store and a resident one over the same database hash the same stream
  and can share persisted index caches; a test asserts that equality.
  `get_text_vec` still returns a borrowed slice and therefore returns `None`
  under eviction, which is why `align.rs` and `onto_compare` were moved to
  `load_text_vec`: left alone they would have silently scored every pair at
  0.0. Eviction is only coherent with the TurboQuant backend, since an
  `instant-distance` graph holds its own float32 copy of every point. 8 tests
  in `tests/vecstore_eviction_test.rs`, each asserting an evicted store returns
  exactly what a resident one built from identical data returns, plus an
  `#[ignore]`d measurement of what the mode costs per query.

  Measured at 20,000 vectors x 384 dims on an M3 Max: 30.7 MB of resident
  float32 goes to zero, `search_cosine_turbo` costs about 75 us more per query
  (312 us to 387 us) for the shortlist fetch, and the exact `search_cosine`
  scan costs 2.4x more (16.4 ms to 39.4 ms) because it reads every row. Evict
  when the workload queries through the turbo path; do not evict when it leans
  on the exact scan.

### Fixed
- **Studio: Tree connector lines — L vs T shape for last children.** The ancestor-continuation-line loop ran `for lvl = 0; lvl < indent`, but `lvl = indent-1` is the same pixel column as the node's own connector (`(indent-1) × INDENT_W + 16`). When the direct parent was not the last child in its group, this drew a full-height vertical at that column, overriding the correct L-shape with a T even for nodes where `isLastChild = true`. Fixed by stopping the ancestor loop at `lvl < indent - 1`; the own connector exclusively owns its column and correctly renders L or T based on `isLastChild`.
- **`search_cosine` and `search_product` no longer clone the entire corpus per
  query.** Routing them through the new whole-set accessor initially copied
  every float32 vector on every call, which cost 20 ms per query at 20,000 x
  384 and made a resident store measurably slower than an evicted one. The
  accessor now yields `Cow` and borrows when the vectors are resident: the
  exact scan went from 36.6 ms to 16.4 ms. Caught by the eviction measurement,
  not by the tests, which only assert results.
- **TurboQuant cosine index backend** (new optional `turbovec` feature, off by
  default, implies `embeddings`). A second index backend for the text half of
  the vector store, built on Google Research's TurboQuant quantiser
  (arXiv:2504.19874) via the `turbovec` crate, alongside the existing
  `instant-distance` HNSW graph. New `src/turbo_index.rs` with
  `TurboCosineIndex` (`build`, `upsert`, `remove`, `search`, `search_within`,
  `to_bytes`, `from_bytes`), and three new `VecStore` entry points
  (`search_cosine_turbo`, `persist_turbo_index`, `load_turbo_index`) plus the
  `turbo_index_len` accessor.

  The motivation is mutation cost, not compression. An `instant-distance`
  graph is immutable, so every `VecStore::upsert` sets `cosine_index = None`
  and the next search pays a full rebuild; an ontology whose embeddings arrive
  incrementally pays that rebuild once per class. TurboQuant has no training
  phase and no graph, so an insert is an append and a removal is O(1), and
  `upsert`/`remove` now maintain the live index rather than dropping it.

  The quantised index is a candidate generator, never the answer.
  `search_cosine_turbo` pulls a shortlist four times wider than the request
  (floor of `top_k + 32`), re-scores every candidate against the float32
  vector the store already holds, and returns that, so no approximate
  similarity number reaches a caller and the result is identical to the exact
  brute-force scan whenever the shortlist covers the true top-k. That identity
  is asserted directly against `search_cosine` in the tests rather than
  assumed.

  Scope limits, both deliberate. (1) `PoincareIndex` is untouched and stays on
  `instant-distance`: TurboQuant scores inner products, and hyperbolic
  distance is not an inner product on the ambient coordinates. (2) The store
  still holds float32 vectors in memory, because the exact re-score and
  `search_cosine` need them, so this change does not yet realise TurboQuant's
  memory win. Evicting float32 to SQLite with load-on-demand for the re-score
  is the follow-on.

  Implementation notes: vectors are zero-padded to the next multiple of 8
  (`turbovec` requires `dim % 8 == 0`; zero padding leaves every inner product
  unchanged). IRIs map to `u64` ids that are never recycled, so a stale
  allowlist entry naming a removed id fails loudly rather than resolving to
  whatever vector took its place; the id counter is serialised with the index
  so a reload cannot restart allocation at 0. A query is validated before it
  reaches the kernel: `turbovec`'s allowlist-free `search` is the panicking
  form, so a non-finite coordinate from a misbehaving embedding provider, or a
  query whose dimensionality disagrees with the index, is reported as no
  results rather than panicking the server or being silently truncated into a
  plausible-looking ranking against the wrong vector. Persistence reuses the existing
  `hnsw_index_cache` table under `kind = 'turbo_cosine'` with no schema
  change, and both load guards are unchanged: `model_fp` rejects an index
  built under a different embedding configuration, `entries_hash` rejects one
  whose entry set has moved on. 17 new tests in `tests/turbovec_index_test.rs`
  covering top-1 agreement with the exact scan, incremental add/replace/remove,
  byte round-trip, id allocation after a reload, allowlist search (including
  unknown and empty allowlists), sub-8 dimensionality padding, non-finite and
  wrong-width query rejection, the VecStore score-identity guarantee, index
  warmth across mutations, SQLite round-trip and stale-cache rejection, plus an
  `#[ignore]`d measurement against HNSW.

  Measured at 10,000 vectors x 768 dims on an M3 Max (`docs/embeddings.md` has
  the tables): build 116 s vs 178 ms, one added embedding 137 s vs 52 us, query
  4.7 ms vs 0.25 ms, serialised index 34.1 MB vs 5.2 MB against 30.7 MB of raw
  float32.

  The recall picture is worth stating carefully, because the first measurement
  overstated it. On that synthetic corpus recall@10 is 91.6% for HNSW and 100%
  for the re-scored TurboQuant shortlist, and the control shows the gap is
  structural rather than a matter of shortlist width: giving HNSW 40 candidates
  instead of 10 leaves it at exactly 91.6%, because the entries it misses are
  ones the graph walk never reaches. But isotropic random vectors are close to
  the worst case for a graph index, and on a real corpus the gap all but
  vanishes. `measure_recall_on_a_real_corpus` embeds 10,000 real ontology
  labels with the shipped local MiniLM model over two contrasting real corpora,
  a topical taxonomy (mean pairwise cosine 0.25) and a set of real-world entity
  names (0.34), against ~0 for the synthetic corpus. HNSW scores 99.9% and
  99.8% there, against 100% for TurboQuant. So recall is not a reason to switch backends; mutation cost and
  index size are.
- **Four extension surfaces** (ECOSYSTEM.md maps them). (1) **Community
  marketplace packs**: `onto_marketplace` now merges an open runtime-fetched
  registry (`community/registry.json`, override with
  `OPEN_ONTOLOGIES_COMMUNITY_REGISTRY`, `community=false` to skip) with the
  curated catalogue — entries are tagged `"source": "curated"|"community"`,
  curated IDs always shadow community IDs, and the shipped registry is
  validated in CI. Seeded with the Manchester/Stanford Pizza teaching
  ontology. (2) **Community skills**: `skills/community/` with a template —
  zero-code markdown workflow recipes. (3) **Companion servers**: the
  five-rule contract (`docs/companion-servers.md`) naming the compose-over-MCP
  pattern OpenCheir already uses (no embedded LLM, no `onto_*` squatting,
  packs/files as interchange, lineage webhook, graceful degradation).
  (4) **WASM plugins** (`--features plugins`): sandboxed community tools via
  the pure-Rust wasmi interpreter — `onto_plugin_list` / `onto_plugin_call`,
  ABI v1 (no host imports, no IO, fuel-metered, fresh instance per call,
  16 MB return cap), graph access only by caller-passed `sparql` whose rows
  are injected as `bindings`. Reference plugin in
  `examples/plugins/label-case-lint`; ABI exercised by WAT-built plugins in
  `tests/plugin_host_test.rs` (including fuel-exhaustion and oversized-return
  guards). Docs: `docs/plugins.md`.
- **fenic support.** [fenic](https://github.com/typedef-ai/fenic) (typedef-ai's
  semantic DataFrame framework) keeps its local catalog in a plain DuckDB file
  with user tables under the `typedef_default` schema. The DuckDB schema
  introspector now scans all user schemas instead of only `main` (excluding
  `information_schema`, `pg_catalog`, `__`-prefixed internals, and fenic's
  `fenic_system` telemetry schema), so `import-schema` / `onto_import_schema`
  work directly against fenic catalogs; cross-schema table-name collisions are
  disambiguated as `<schema>_<table>`. `open-ontologies-lite` gains a
  duck-typed dataframe bridge — `rows_from_dataframe`, `rows_to_turtle`, and
  `OntologyEngine.load_rows` accept fenic, polars, pandas, and pyarrow objects
  with no new dependencies. New end-to-end example
  `python/examples/fenic_pipeline.py` and a "fenic" section in
  `docs/data-pipeline.md`.

## [1.2.0] - 2026-08-15

### Added
- **`onto_pack` / `onto_unpack`.** Portable verified knowledge artifacts: sorted
  N-Triples plus a manifest (name, version, counts, timestamp, tool version,
  sha256) and the lint/enforce results recorded at pack time. What you promote
  between environments is a graph that has already passed its checks, with the
  evidence attached. `onto_unpack` refuses a pack whose checksum does not match.
- **Bi-temporal facts.** `onto_temporal_snapshot`, `onto_temporal_query` and
  `onto_temporal_conflicts` separate two independent clocks: `valid_at` asks what
  was true then, `as_of` asks what was known then. A disjointness violation only
  counts as a conflict when the two assertions claim overlapping validity, so
  superseded history stops being reported as contradiction. Graphs without
  validity metadata are timeless and always in scope, making the vocabulary
  additive to an existing store.
- **`onto_reason_incremental`.** Derives the consequences of newly added triples
  by joining the delta against the existing closure, so the cost tracks what
  changed rather than the size of the store. Schema axioms are refused with an
  explanation, because those change what the whole store entails.
- **Claim support checking.** `onto_support_check`, `onto_support_verdict` and
  `onto_support_report` add the second axis beside conformance: conformance asks
  whether a claim is expressible, support asks whether it is true to its source,
  and a claim can fail either independently.
- **`onto_communities`.** Deterministic modularity clustering that returns a
  skeleton per community (size, top members by degree, internal relations,
  bridges) so corpus-wide questions can be answered from reports instead of
  traversal from an anchor entity.
- **`onto_ossie_import`.** Compiles Apache Ossie (incubating) ontology documents
  to OWL 2 DL plus SHACL, making a vendor semantic model reasonable and
  validatable. The four constructs OWL 2 DL cannot express are reported and
  preserved as annotations rather than silently dropped.
- **SHACL `sh:inversePath` and `sh:severity`.**
- **Unauthenticated `/health` liveness route on `serve-http`.** Registered
  outside the bearer layer on purpose, so a probe does not need credentials
  while `/api` and `/mcp` stay behind them. The body is limited to status and
  version: an unauthenticated endpoint should not describe loaded state.
- **Optional PROV-O provenance emission on ingest.**
- **Embedding fingerprints.** Each vector records the configuration that
  produced it, including the tokenizer and a hash of the whole model file, so a
  changed embedder invalidates rather than silently mixes vector spaces.
- **Build-time modelling buffer, phase 1.**

### Changed
- **Loads are all-or-nothing.** A failed load no longer leaves a partially
  populated store.
- **`enforce` gained a competing-modelling-pattern rule.**

### Fixed
- **`onto_load` did not set the base IRI from the file path**, so relative IRIs
  resolved against the wrong base.
- **`serve-http` and `serve-unix` shutdown.** The cancellation token is now
  cancelled on ctrl-c and SIGTERM, the server keeps listening so a second signal
  can force the exit, and `serve-unix` unlinks its socket.
- **`plan` / `apply`.** Apply works against the ABox and stops fabricating
  bridges; plans are scoped to their owner and Windows paths are tokenised.
- **Alignment claim strength now tracks evidence strength.**

### Fixed
- **SQLite migrations discarded every error and tracked no schema version.**
  `StateDb::open` upgraded old databases with two `let _ = conn.execute_batch(...)`
  calls. Discarding the result swallowed the expected "duplicate column name" on
  an already-upgraded database, but it swallowed a locked file, an I/O error and
  a partially-applied batch with it, and `open` returned `Ok` regardless.

  `PRAGMA user_version` now records the schema version, and each migration
  applies inside a transaction that commits the DDL and the version bump
  together. Columns are checked individually with `PRAGMA table_info` before
  being added, so only the genuine already-exists case is tolerated and every
  other error propagates.

  Per-column checking also heals the state the old code could produce and not
  detect. `execute_batch` stops at its first error, so a database whose first
  `ALTER` committed and whose second did not was left with `webhook_url` and no
  `webhook_headers` — and re-running the pair would fail on the column that was
  already there, permanently. Verified against real rusqlite semantics, not
  assumed: the old idiom leaves the column missing and still returns `Ok`.

  Databases predating the tracker all report `user_version = 0` whether they
  carry the columns or not, which is why the version alone cannot decide what to
  do and the column probe is the thing that establishes truth.

  `tests/state_migration_test.rs` covers this against hand-built binary fixtures
  in `tests/fixtures/state/`: a pre-migration database, and a half-applied one.
  The fixtures are committed rather than generated, because a fixture built by
  the code under test would agree with a broken migration by construction.
  Reported in #75.

### Added
- **Versioned SQL type → XSD datatype contract.** `docs/data-pipeline.md` now
  documents what `SchemaIntrospector::sql_to_xsd` produces for every SQL type it
  recognises, alongside the declarative `datatype` mapping field it is easy to
  confuse it with. The table is marked v1 and any row that changes gets a
  CHANGELOG entry, so a `schema.rs` refactor can no longer alter the shape of
  generated ontologies invisibly. Also records the decisions the table encodes
  (parameters stripped, timezone not represented, `xsd:string` catch-all) and
  what the schema import derives beyond the datatype.
- **CI builds, lints and tests the optional features.** A `features` job adds a
  breadth leg (`cargo check` + `cargo clippy --all-targets` at `--all-features`)
  and a depth leg (`cargo test --features postgres,duckdb,embeddings,sql`), with
  `rust-cache` keyed per feature set. `default = []`, so none of `postgres`,
  `duckdb`, `sql`, `embeddings` or `causal-pywhy` — nor the `sqlx`, `duckdb`,
  `tract-onnx`, `tokenizers` and `instant-distance` trees — was compiled by
  anything in CI before this.

  The first run of the breadth leg found `clippy::large_enum_variant` in
  `src/embed.rs`, a lint that could not have fired before because nothing
  compiled `embeddings`.

  `scripts/check-test-collection.sh` fails the run when a test file whose gate
  is enabled collects zero tests. Eight files under `tests/` had never been
  collected once, 56 tests in total; a file that collects nothing reports
  "test result: ok" and is indistinguishable from a passing one. The enabled
  set is derived from the cargo flags the test step used and expanded through
  `[features]`, so `--features sql` counts as postgres + duckdb; files whose
  gate is off in a given leg are skipped, so partial-feature legs stay green.
- **Build provenance and checksums on release binaries.** The release job now
  emits a Sigstore attestation via `actions/attest-build-provenance`, binding
  each published binary to the workflow run and the commit it was built from,
  and publishes a `SHASUMS.txt` alongside the assets. The release job gains
  `id-token: write` and `attestations: write` for the signing identity.

  Verify provenance with:

  ```
  gh attestation verify <binary> --repo fabio-rovai/open-ontologies
  ```

  Or check the checksum of the binary you downloaded. `SHASUMS.txt` lists all
  four platforms, so verify the one you have rather than the whole file:

  ```
  grep open-ontologies-x86_64-unknown-linux-gnu SHASUMS.txt | sha256sum -c -
  # macOS: ... | shasum -a 256 -c -
  ```

  A bare `sha256sum -c SHASUMS.txt` expects all four binaries present and exits
  non-zero when three are missing, which is the normal case. With GNU coreutils
  you can also pass `--ignore-missing`; the form above works on macOS too.
- **Opt-in persistent triple store.** A new `[storage]` section selects the
  backend for the main graph: `mode = "memory"` (default, unchanged behaviour)
  or `mode = "persistent"`, which opens a RocksDB-backed Oxigraph store at
  `<data_dir>/triplestore` so triples survive a restart. Override with
  `OPEN_ONTOLOGIES_STORAGE_MODE` or `--storage-mode` on `serve` / `serve-http`;
  precedence is CLI, then env, then config, then the default. Unknown values
  warn and fall back to `memory` rather than failing.

  The one-shot CLI subcommands read the same setting, so `open-ontologies load
  foo.ttl` followed by `open-ontologies query ...` shares state when
  persistence is on. Sandbox stores elsewhere in the codebase stay in-memory;
  only the main graph is ever persistent.

  Note that Oxigraph permits a single read-write handle per directory, so two
  server processes pointed at the same `data_dir` will fail to open the second
  store. Contributed by Ladislav Gazo (@lgazo).

### Fixed
- **SQL floating-point columns were mapped to `xsd:decimal`.** `real`,
  `float4`, `float`, `float8`, `double` and `double precision` all resolved to
  `xsd:decimal` in `SchemaIntrospector::sql_to_xsd`, so every ontology
  generated by `onto_import_schema` / `onto_sql_ingest` from a table with a
  float column carried a wrong range. `xsd:decimal` is integers over powers of
  ten: it cannot represent `NaN`, `INF` or `-INF`, and it asserts an exactness
  IEEE 754 does not have. `real` and `float4` now map to `xsd:float`, and
  `double`, `double precision` and `float8` to `xsd:double`.

  Bare `float` is dialect-dependent (Postgres `float8`, DuckDB `float4`) and
  maps to `xsd:double`, widening rather than narrowing: every `xsd:float`
  value is exactly representable as an `xsd:double`, so the reverse choice
  would declare a range narrower than the data. `numeric` and `decimal` are
  unaffected and still map to `xsd:decimal`.

  **This changes output.** Ontologies generated before this release state
  `xsd:decimal` on float-backed properties; regenerate them, or expect
  range mismatches against ones generated after. Reported in #76.

## 1.1.1 — 2026-08-03

Correctness and reporting. No new features. Two of these change output, so
results produced with 1.1.0 or earlier are not directly comparable.

### Fixed
- **Tableaux classification was nondeterministic.** `named_classes` is a
  `HashSet` and five expansion sites collected `self.nodes.keys()` unordered, so
  hash iteration order decided which subsumption checks completed inside the
  node and depth budgets. Exhaustion correctly yields `Unknown`, so entailments
  were silently absent rather than wrong. Conformance suite went from 4/6 to
  12/12 consecutive runs passing, and that binary's wall time from 1.17-2.57s to
  0.01s. See `docs/determinism.md`.
- **Alignment was nondeterministic.** `extract_classes` returned
  `into_values()` order and the candidate comparator sorted on confidence alone,
  which is not a total order because the zero-structural-signal branch assigns
  many pairs the identical value. Five runs of the same binary produced five
  different alignments. Now sorted by IRI with ties broken on the IRI pair.
- **Security:** `tract` 0.21 -> 0.22.3 and `time` -> 0.3.55, closing
  RUSTSEC-2026-0217 and RUSTSEC-2026-0009. The 0.21 line could not satisfy both
  advisories simultaneously.
- `run_ablation_no_stable.py` had been failing on a stale source marker and had
  never run to completion.
- `score_condition_d.py --legacy` wrote to the canonical results path,
  overwriting the corrected scores with the ones they exist to refute.

### Changed
- **OAEI Anatomy result corrected to P 0.960 / R 0.730 / F1 0.829** (1,152
  correspondences). The previously recorded 0.832 was one draw from the
  nondeterministic distribution above. Rank in the OAEI 2025 field is unchanged
  at 9th of 13.
- Benchmark reporting now uses the complete OAEI 2025 field including both
  baselines, rather than four selected systems.
- Marketplace statistics regenerated; property counts were stale output from a
  counting bug fixed in March.

### Removed
- **The 1,633x OWL-RL vs HermiT speed claim is withdrawn.** It was measured on
  an empty store and inverts when measured correctly. See
  `benchmark/reasoner/README.md`. No speed claim against a Java reasoner should
  be made from this repository.

### Added
- `docs/determinism.md` recording both defects, with reproduction commands.
- `benchmark/oaei/results/ablation_no_stable.json`, the single-variable
  stable-matching ablation.

## 1.1.0 — 2026-07-27

### Added
- `claimcheck` module (library only; no MCP tool and no CLI subcommand calls it): compiled per-claim ontology-consistency verification.
  Token-bitset engine (0.3 µs median per claim, 11M claims/s batched), sound
  two-hop disjointness join with witness extraction, three-valued verdicts
  (`Rejected` / `Undetermined` / `Consistent`), reasoner-backed residual tier
  (`ResidualOracle`) with verdict learn-back, closed-world vocabulary checks,
  and an assumed-disjointness WARN tier for zero-disjointness ontologies.
  Correctness audited against HermiT: 0 unsound rejections over 78,884
  exhaustive class pairs (13 ontologies) and 793 adversarial structural
  claims. The join is sound but incomplete, so this is not full agreement.
- Offline compile tooling (`benchmark/reasoner/`): `CompileOntology` with six
  sound disjointness-propagation rules (restriction, functional, union,
  data-value, counting, dueling-universal idioms), `DisjointnessMatrix`,
  `ClaimConsistency`, `PairOracle`, `VetDisjointness`, `StripDisjointness`.
- **`onto_vocab_check` — closed-world vocabulary check for generated DATA graphs.** Verifies that every predicate and every `rdf:type` class used in a Turtle data graph is actually **declared** in the loaded ontology, flagging hallucinated/undeclared terms. This is the gate open-world SHACL structurally cannot provide: SHACL silently ignores predicates it has no shape for, so a graph full of invented terms (an LLM emitting `ies:hasDeparturePort` when the ontology only defines `ies:scheduledDeparturePort`) still reports `conforms=true`. Only IRIs whose namespace belongs to the ontology (plus any passed via `namespaces`) are policed — standard `rdf`/`rdfs`/`owl`/`xsd`/`sh` vocabulary and the caller's own instance-data IRIs are never flagged. Returns `{conforms, hallucinated_terms, checked_namespaces, predicates_checked, types_checked, ontology_terms}`. **Vacuous-pass guard:** when no ontology vocabulary is present (0 declared terms and no `namespaces`), the tool returns `conforms=false` with an explanatory `warning` rather than silently passing — a closed-world check with nothing to check against must never green-light. New module `src/vocab_check.rs` (SPARQL over Oxigraph, mirroring the term-existence logic of `onto_shacl_check`) with 3 unit tests (clean conforms / hallucinated predicate caught / instance + standard namespaces never flagged). Exposed as MCP tool `onto_vocab_check` (+ `OntoVocabCheckInput`), CLI subcommand `vocab-check`, and batch command `vocab_check`. Verified end-to-end against the real IES4 ontology (714 terms): it flags the non-ontology terms IES4's *own* published sample data uses — `movement.ttl` → `isScheduledDeparturePort`/`isScheduledArrivalPort`/`seatNumber`, `events.ttl` → `happensIn` — every one of which plain SHACL reports as conforming. Companion to `onto_shacl` (open-world data validation) and `onto_shacl_check` (checks proposed shapes); this checks generated data. Follows the MCP-native validation-primitive convention.

### Changed
- OWL-DL reasoner: satisfiability is now three-valued — resource exhaustion
  reports `Unknown` instead of unsatisfiability; per-test (10s) and global
  classification (180s) wall-clock budgets; pairwise blocking; output gains
  `complete`, `undetermined_classes`, `subsumption_sweep_cut_short`, and
  `abox.undecided`.
- RDF loading content-sniffs `.owl` files, so Turtle published under `.owl`
  parses correctly.

## 1.0.0 — 2026-06-13

### Added
- **Causal (v0.5): `certify_action` × PyWhy integration.** Wires the #48 PyWhy scaffold into the live `certify_action` path. New `ActionFrame.identification_mode` field (`Structural` | `DoCalculusBackdoor`, default `Structural` for back-compat) selects which identifiability proof to attempt. New helper `build_causal_dag(graph, target_iris)` extracts a causal DAG from the loaded RDF graph: nodes = target IRIs + their one-hop structural neighbours (same slice CIVeX already hashes); edges = `rdfs:subClassOf` / `rdfs:subPropertyOf` / `rdfs:domain` / `rdfs:range` triples among slice members (treated as cause → effect); plus a synthetic `__utility__` sink downstream of every node. When `DoCalculusBackdoor` is requested AND the `causal-pywhy` Cargo feature is enabled, the verifier calls `civex_pywhy::run_pywhy_backdoor` and on success stamps the certificate's `identification_proof` with DoWhy's adjustment estimand + tags assumptions with `"do_calculus_backdoor"`. On any failure path (Python unavailable / DoWhy unavailable / DoWhy runtime error / target unidentifiable) the verifier **silently falls back to the structural proxy** and records the reason in assumptions as `"do_calculus_unavailable:<kind>"`. When the feature is off, the marker is `"do_calculus_unavailable:feature_disabled"`. The certificate's `identification_proof` field is never empty regardless of branch taken. MCP tool `onto_certify_action` gains `identification_mode: Option<String>` accepting `"structural"` (default) or `"do_calculus_backdoor"`. Two new integration tests in `tests/civex_test.rs`: `structural_mode_records_structural_only_assumption` (back-compat) and `do_calculus_mode_falls_back_to_structural_when_feature_disabled` (verifies the fallback path doesn't crash and records the marker). All 9 civex integration tests pass in both default and `causal-pywhy` build configurations; clippy clean across `--lib --tests --examples` in both configs.
- **Causal: PyWhy/DoWhy backdoor identification subprocess scaffold (#48).** Scaffolds the Causal flagship's substantive v0.5 work without pulling Python into the default build. New optional Cargo feature `causal-pywhy` (off by default) enables a `src/civex_pywhy.rs` module that wraps DoWhy v0.13 as a subprocess — same pattern as `src/plan_classical.rs` does for Fast Downward. The wrapper embeds a self-contained Python driver (`PYWHY_PYTHON_DRIVER`) that reads `{nodes, edges, treatment, outcome}` from stdin, builds a `networkx` DiGraph, runs DoWhy's `identify_effect` for backdoor adjustment, and emits `{identifiable, adjustment_set, estimand_expression}` to stdout. Per the May 2026 roadmap memo: **Pearl–Shpitser ID is not ported to Rust** — DoWhy is a 15-year-stable Python implementation, so we wrap it. Honest behaviour when Python or DoWhy is missing: structured errors with `kind = "python_unavailable"` / `"pywhy_unavailable"` / `"dowhy_runtime_failed"` that the caller (eventually `certify_action` in v0.5) dispatches on to fall back to the structural proxy. Binary resolution order: explicit `python_override` → `PYTHON_BIN` env var → `python3` on PATH. **NOT yet integrated into `certify_action`** — the structural proxy (`"structural_only"` assumption) remains the only identifier in shipped certificates; integration tracks as the v0.5 ship. 9 unit tests cover parser cases (identifiable, unidentifiable, both error kinds, invalid JSON), python resolver (override + default + env var), embedded-driver sanity check, and `python_unavailable` behaviour on missing binary. Zero additional Rust dependencies — the entire feature is embedded Python + subprocess plumbing.
- **Dynamics: non-deterministic outcomes for ActionSchema (#49).** `ActionSchema` gains an additive `outcomes: Vec<Outcome>` field. Each `Outcome` carries a categorical `probability` in `[0, 1]`, its own `effects: Vec<EffectSpec>` list, and an optional human-readable `label` (e.g. `"success"` / `"degraded"` / `"failure"`). When `outcomes` is non-empty, `apply()` samples one outcome and executes its effects; the deterministic `effects` field is ignored. When empty (the default), the schema behaves exactly as before — full back-compat with v0.4 base. Probabilities are validated: the sum must equal `1.0 ± 1e-6` or `apply()` returns an error; negative probabilities are also rejected. Sampling uses an inline xorshift64 PRNG keyed by a seed — **zero new dependencies**, no `rand`/`fastrand` pulled in. New method `ActionSchema::apply_with_seed(graph, db, bindings, seed)` exposes the seed for reproducible sampling; the default `apply()` derives a seed from `SystemTime::now()`. `ApplyResult` gains `sampled_outcome: Option<usize>` and `sampled_outcome_label: Option<String>` so callers can see which branch fired. `onto_action_apply` gains an optional `seed: Option<u64>` parameter — pass it when you need reproducible runs (CIVeX certification, replay-from-audit-log, controlled experiments). Tests (4 new): `nondeterministic_apply_with_seed_is_reproducible`, `nondeterministic_apply_distribution_matches_probabilities` (1000-call smoke check that a 70/30 split lands within `[0.60, 0.80]`), `nondeterministic_apply_rejects_invalid_probability_sum`, `deterministic_schema_still_works_when_outcomes_is_empty` (back-compat).
- **Planner: `onto_plan_classical` — Fast Downward subprocess wrap (#50).** Optional convenience tool that completes the LLM-Modulo Planner pipeline (`compile_pddl` → `classical` → IRI-bind client-side → `validate`). Per the LLM-Modulo convention, the classical solver is still client-side — this wrapper exists so a caller who *does* have Fast Downward installed locally can ask the server to run it for them rather than shelling out themselves. Honest behaviour when Fast Downward is missing: returns a structured `binary_unavailable` error with installation guidance, never falls back to a silent stub. Binary resolution order: explicit `fast_downward_bin` parameter → `FAST_DOWNWARD_BIN` env var → `fast-downward.py` on PATH. Returns the raw `sas_plan` content (preserved verbatim) plus a parsed `operators: [{name, args}]` list and the `; cost = N (...)` footer extracted separately. Search strategy is configurable (default `"lama-first"`; pass `"astar(lmcut())"` etc.). Reads the highest-numbered `sas_plan.N` variant when satisficing search emits multiple plans. New module `src/plan_classical.rs` with 7 unit tests covering parse cases (three-operator plan with cost footer, blank-line + comment skipping, empty input, zero-arg operators) and resolver / error behaviour (explicit override, default fallback, `binary_unavailable` on missing binary). New MCP tool `onto_plan_classical` + `OntoPlanClassicalInput`.
- **Planner: `onto_plan_validate` — LLM-Modulo validator primitive (#45).** Server-side companion to `onto_plan_compile_pddl`. Per the LLM-Modulo convention (Kambhampati arXiv 2402.01817), the server compiles + validates, the orchestrator solves. The validator takes a candidate plan (an ordered list of `{action_name, bindings}` operator instances — typically produced client-side by Fast Downward, LLM prompting, or any other source) and step-by-step: (a) looks up each step's registered `ActionSchema`, (b) re-evaluates its preconditions against the cumulative sandbox state under the step's bindings, (c) if applicable, executes its effects against the sandbox, (d) if not, returns immediately with the failing step index and a diagnostic. **Critically, the validator forks the loaded graph into an isolated sandbox** so the real store is never mutated — verified by a dedicated test. Multi-step plans correctly chain state through: a test exercises Step 1 establishing the precondition that Step 2 needs (declare `ex:Feline` as a class, then add `ex:Cat rdfs:subClassOf ex:Feline`). Optional `goal_facts` are checked post-plan and reported in `unsatisfied_goals` (without invalidating the plan itself — a well-formed plan that just doesn't reach the goal is still well-formed). Internal scratch `StateDb` opened as `:memory:` so per-step lineage entries don't pollute the production audit trail. New module `src/plan_validate.rs` + 6 unit tests covering empty plan, single-step success, missing action, unsatisfied precondition, multi-step state-chain, and goal-checking semantics. New MCP tool `onto_plan_validate` + `OntoPlanValidateInput` / `PlanStepInput`.
- **Dynamics: ramification via OWL-RL closure after apply (#47).** First follow-on to the Dynamics scaffold. New `ActionSchema::apply_with_ramification(graph, db, bindings, profile)` method that runs the existing `reason::Reasoner` immediately after the literal effects land, materialising downstream entailments into the same graph. `ApplyResult` gains two new fields: `derived_triples_added: usize` (count of new triples the reasoner produced beyond the literal effects) and `ramification_profile: Option<String>` (the profile actually run, or `None` when ramification was skipped). The `onto_action_apply` MCP tool gains a `ramify` parameter accepting any of `"rdfs"` / `"owl-rl"` / `"owl-rl-ext"` / `"owl-dl"`; default `None` preserves the previous literal-effects-only behaviour. Validated against the canonical acceptance case from #47: a schema that adds `?child rdfs:subClassOf ?parent`, applied with `ramify="owl-rl"` over a graph containing `ex:tigger a ex:Cat`, materialises `ex:tigger a ex:Animal` via subClassOf transitivity. Two new unit tests in `src/dynamics.rs`.
- **Dynamics layer scaffold + Planner stub (three-layer architecture, #43 + #45).** First two of the three v0.4–v0.6 layers from the May 2026 KR/UAI/ICAPS/AAMAS roadmap land as additive scaffolding on top of v0.2's primitives — no breaking changes to existing tools. **Dynamics** introduces `ActionSchema` (BC+ deterministic-single-effect subset): typed `Parameter` slots, SPARQL `ASK`/`SELECT` `preconditions` with `{param}` substitution, and KGCL-shaped `effects` (`AddTriple` / `RemoveTriple` / `AddClass`). Schemas persist by name in a new `dynamics_action_schemas` SQLite table; `apply()` runs the effects, emits the KGCL Controlled-Natural-Language patch, mints an IES4-style event IRI, and logs to `lineage`. Four new MCP tools: `onto_action_register` (persist a schema from inline JSON), `onto_action_applicable` (evaluate preconditions against the loaded graph under a binding map), `onto_action_apply` (execute effects + return patch + event IRI; re-checks preconditions by default), `onto_action_list` (enumerate registered schema names). **Causal-layer hookup**: `civex::ActionFrame` gains an optional `action_schema_name`; when set, `onto_certify_action` echoes `dynamics_action_schema:<name>` into the certificate's assumptions, so the audit trail is explicit about which Dynamics action was gated. **Planner stub** (`src/plan_pddl.rs` + `onto_plan_compile_pddl`): emits a PDDL domain from registered action schemas plus a problem instance from the loaded graph and a goal Turtle slice. Single-predicate `(triple ?s ?p ?o)` over typed sort `iri`; ASK-shaped preconditions translate cleanly, SELECT-shaped ones surface in `translation_notes` so the lossy translation is honest. Per the LLM-Modulo convention (Kambhampati arXiv 2402.01817), the actual planner (Fast Downward) is delegated to the orchestrator; this primitive only emits the PDDL. New files: `src/dynamics.rs` (~458 LOC including 7 unit tests), `src/plan_pddl.rs` (~280 LOC including 6 unit tests). Causal extension verified by a new integration test `action_schema_name_is_recorded_in_certificate_assumptions` in `tests/civex_test.rs`. Honest deferrals: ramification rules, non-deterministic dynamics, and concurrent action semantics defer to v0.4.x; OWL → PDDL rigour (Borgwardt KR 2025) defers to v0.6 proper.
- **`onto_certify_action` — CIVeX-style causal certificate for state-changing actions** (#42, [arXiv 2605.09168](https://arxiv.org/abs/2605.09168)). New MCP tool that gates any state-changing onto_* operation before execution. Maps a proposed action to a structural identifiability check + Wilson one-sided LCB on the do-effect, returns one of four auditable verdicts: **EXECUTE / REJECT / EXPERIMENT / ABSTAIN**. Each verdict carries a certificate documenting the labelled assumptions, structural-dependency identification proof, point estimate, LCB at level α, provenance SHA-256, and risk bound. Scaffold port: keeps the four-way verdict + Wilson LCB + locked-IRI hard-reject; uses a **structural-dependency proxy** for identifiability (honestly documented as `"structural_only"` in the assumptions list) rather than full do-calculus backdoor/frontdoor algorithms. EXPERIMENT degrades to ABSTAIN unless caller passes `allow_experiment=true`. New module `src/civex.rs` + 6 integration tests in `tests/civex_test.rs` covering EXECUTE, REJECT (cost > risk_threshold), REJECT (locked IRI), ABSTAIN (irreversible + ambiguous LCB), EXPERIMENT (reversible + authorised), and provenance-hash determinism.
- **`graph_projection_lossy_check` — audit projected RAG slices for information loss** (#35, IJCAI 2025). New MCP tool that compares a projected Turtle slice against the loaded ontology's full neighbourhood of seed IRIs and reports dropped predicates, dropped object IRIs, per-seed coverage ratio, and aggregate coverage. Pairs with the upcoming `onto_segment_retrieve` (#34) — the retriever produces the slice; this auditor reports what it left behind, so the calling LLM can decide whether the slice is sufficient. New module `src/projection_check.rs` + 4 inline unit tests covering full-projection-OK, dropped-predicate-flagged, parse-failure path, and missing-seed-in-source-trivially-covered.
- **HNSW polish — per-call tuning, Poincaré variant, async flush.** Three follow-on wins on the HNSW moat: (1) `onto_search` gains `use_hnsw` and `ef_search` parameters, so callers can route a single query through the HNSW cosine index and optionally trigger a rebuild with custom `ef_search` per query. Caveat documented: `instant-distance` bakes `ef_search` into the HNSW structure at build time and doesn't support per-query overrides, so a non-default `ef_search` triggers a rebuild — prefer `onto_hnsw_build` if you query frequently with the same value. (2) New `PoincareIndex` variant alongside `CosineIndex`, indexing structural embeddings (Poincaré ball) instead of text embeddings (cosine). Wired into `VecStore` with `search_poincare_hnsw`, `rebuild_poincare_index`, `persist_poincare_index`, `load_poincare_index`. The two indices coexist independently; both share the `hnsw_index_cache` SQLite table (kind = 'cosine' | 'poincare') with the same entries-fingerprint, so a mutation invalidates both at once. (3) Async background flush via `persist_cosine_index_async` / `persist_poincare_index_async` — returns a `tokio::task::JoinHandle` that resolves when the SQLite write completes. Serialisation happens synchronously (in-memory bincode, < 100ms for ontologies under ~10k classes); only the SQLite write is dispatched to `spawn_blocking`. Useful for keeping MCP tool handlers responsive when persisting large indices. Three new integration tests (Poincaré top-1 vs brute-force, coexistence with cosine, async persist round-trip) plus an additional Poincaré persistence test, taking the vecstore suite from 9 to 15 tests.
- **HNSW persistence + tuning + onto_align prefilter** (completes the moat scaffold). Three follow-on changes wire the prior HNSW scaffold into the rest of the system: (1) the built HNSW index now persists across process restarts via a new `hnsw_index_cache` SQLite table; `VecStore::load_from_db` automatically reinstates the cached index when its entries-fingerprint (deterministic FNV-1a 64-bit hash of sorted iri+text-vec bytes) matches the just-loaded vectors, and rejects stale caches when vectors changed — so a process startup over a populated DB skips the full rebuild. (2) New `onto_hnsw_build` MCP tool exposes the HNSW `ef_construction` and `ef_search` parameters so the connected orchestrator can tune index quality vs build/query time on larger ontologies; the tool optionally persists the rebuilt index. New `OntoHnswBuildInput` in `src/inputs.rs`. (3) `onto_align`'s candidate loop now transparently uses the HNSW index as a pre-filter when both source and target IRIs have embeddings in the vecstore: for each source class, a top-50 cosine shortlist of target candidates is computed once via HNSW, and the inner loop skips pairs not in the shortlist. The optimisation degrades gracefully — sources without embeddings fall back to the full target scan, preserving correctness on partially-embedded inputs. Two new persistence tests in `tests/vecstore_test.rs` cover the round-trip (vectors + index reload on a fresh `VecStore` over the same DB) and the cache-invalidation path (mutated vectors + unmutated index → cache rejected, rebuild on next search).
- **HNSW-accelerated cosine search scaffold** (the "vector-index moat"). New optional `instant-distance` dependency (gated behind the existing `embeddings` feature, no impact on default builds) and a new `src/hnsw_index.rs` module wrapping the HNSW algorithm (Malkov & Yashunin, TPAMI 2020). `VecStore` grows a `search_cosine_hnsw(query, top_k)` method that builds the index lazily on first call and rebuilds whenever the store is mutated; the existing `search_cosine` brute-force linear scan is unchanged and continues to work without HNSW (zero regression risk). Strategic context: per the May 2026 ecosystem research, no Rust knowledge-graph engine ships native HNSW alongside its triple store — the de-facto stack is `Neo4j + Qdrant + a Python adapter`. This module is the foundation for Open Ontologies to fill that gap as a Rust-native MCP server with first-class semantic search inside the same process. Scaffold scope: the core index + integration + tests; follow-up work (persistence layer for the built index, MCP-tool surface for tuning HNSW `ef_search` / `ef_construction`, wiring into `onto_align`'s embedding-similarity signal) is tracked in the project notes. New tests: 3 unit tests in `src/hnsw_index.rs` + 3 integration tests in `tests/vecstore_test.rs` (round-trip top-1 agreement with brute-force, mutation-invalidation behaviour, empty-store edge case).
- **GenOM-style description-based embedding enrichment for `onto_embed`**. New optional `descriptions: HashMap<String, String>` field on `OntoEmbedInput`. When the map is supplied, each class IRI in the map is embedded from its description text instead of from its `rdfs:label`; IRIs absent from the map fall back to the existing label-based embedding. Returns a new `enriched: <count>` field alongside the existing `embedded: <count>` so callers can see how many classes used descriptions vs. labels. This is the MCP-native form of the GenOM pattern (Mensa et al. 2025, accepted World Wide Web Journal, which showed Qwen-32B-generated descriptions lift alignment F1 substantially over raw-label embedding): the server doesn't generate descriptions, the connected orchestrator (Claude) authors them in-conversation using its own reasoning, then passes them in via this field. Net new dependencies: zero. Behaviour with no `descriptions` map is identical to before (back-compat). New inline unit tests in `src/inputs.rs` cover the deserialization both with and without the field present.
- **`ies-4.3.1` marketplace preset — frozen MIT baseline** (#25). New marketplace catalogue entry pointing at the archived `dstl/IES4` repo at tag `v4.3.1` (3 Mar 2025, MIT-licensed, last public release before the IES governance transition to DBT / IES-Org). Distinct from the existing `ies` preset (which tracks `IES-Org/ont-ies` main and shifts as upstream evolves) — use `ies-4.3.1` when you need a reproducible compliance baseline that won't drift. Source: 5,375-line Turtle artefact `ies4.ttl`, baseURI `http://ies.data.gov.uk/ontology/ies4`, the same namespace used by the existing `boro` and `ies4` enforce rule packs. Install via `onto_marketplace install ies-4.3.1`. Inline unit tests in `src/marketplace.rs` verify the URL pins to the `v4.3.1` tag (not `main`) and that the live `ies` and frozen `ies-4.3.1` presets coexist with distinct IDs and URLs.
- **RRF (Reciprocal Rank Fusion) as an opt-in fusion strategy for `onto_align`**. New `OntoAlignInput.fusion` field accepts `"weighted_sum"` (default, unchanged behaviour with self-calibrating learned weights) or `"rrf"` (Cormack et al. SIGIR 2009 at k=60, validated for ontology alignment by Agent-OM at VLDB 2025). RRF is order-based rather than score-based, so it doesn't need feedback to bootstrap; it's a sensible cold-start choice when the `align_feedback` table is empty. The per-signal scores remain on each candidate's `signals` field so downstream `onto_align_feedback` calls keep working identically. New public method `AlignmentEngine::align_with_fusion(source, target, high, low, dry_run, fusion)`; the existing `align_with_thresholds(...)` and `align(...)` entry points are preserved as thin wrappers that pass `"weighted_sum"`. New `tests/align_rrf_test.rs` (5 tests) covering normalisation to [0, 1], per-signal preservation, low-threshold post-rerank filtering, weighted_sum back-compat, and weighted_sum/RRF top-pair agreement on perfect matches.
- **`ies4` enforce rule pack** (#24). New built-in design-pattern pack for the [Information Exchange Standard](https://informationexchangestandard.org/), the UK cross-sector ontology framework custodied by Department for Business and Trade since March 2025 (canonical repo `IES-Org/ont-ies`). Three rules beyond the existing `boro` pack: (1) `ies4_particular_class_overlap` (severity: error) — a class cannot subclass both `ies:Particular` and `ies:ClassOfEntity`, as that violates the type-vs-token distinction foundational to IES4's 4D mereology; (2) `ies4_state_without_subject` (severity: warning) — a class subclassing `ies:State` must declare `ies:isStateOf` via owl:Restriction or have at least one instance using it (the state pattern is meaningless without a bearer); (3) `ies4_event_without_participant` (severity: warning) — a class subclassing `ies:Event` must have a participant pattern via `ies:isParticipantIn` / `ies:involvesParticipant` / `ies:hasParticipant` (events without participants are incomplete 4D models). Invoke via `onto_enforce` MCP tool or `enforce` CLI with `rule_pack = "ies4"`. Academic grounding: FOUST 7 paper "Comparing IES and BORO" (CEUR Vol-4176, JOWO 2024). New `tests/enforce_ies4_test.rs` (5 tests covering each rule's positive and negative cases, plus the instance-level participation accept path).

### Fixed
- **`onto_drift` now canonicalises blank nodes via RDFC 1.0 instead of filtering them out.** Replaces the temporary `_:`-prefix filter shipped in PR #14 (Jason Smith / @rustforrecess, who originally diagnosed the bnode-instability bug and shipped the surgical fix that bought time for this proper successor). New method `GraphStore::canonicalize_blank_nodes()` uses W3C RDF Dataset Canonicalization 1.0 (SHA-256) — available built-in via Oxigraph 0.5.8 — to assign deterministic `_:c14n<n>` identifiers derived from the graph structure. `DriftDetector::detect()` canonicalises each snapshot before vocabulary extraction, so reparses of the same ontology produce identical canonical bnode IDs (the reparse-stability property PR #14 achieved by exclusion), but anonymous restriction classes / quoted axioms now PARTICIPATE in the diff with stable IDs rather than being dropped. Caveat: canonical IDs are a function of the whole graph, so a quad change can shift many bnode IDs — for typical edits the existing rename-pairing logic in `detect()` re-matches shifted bnodes via the 4-signal ensemble (label / domain-range / hierarchy / individuals), so the net result is more informative than PR #14's filter. `tests/drift_blank_node_test.rs` updated to assert the new canonical-stability contract; new test `canonical_bnode_ids_are_stable_across_independent_reparses` exercises the contract directly on two restriction shapes.

### Changed
- **Oxigraph dependency bumped from 0.4 → 0.5.8** (#15). Oxigraph 0.5 ships RDF 1.2 / SPARQL 1.2 support (behind `rdf-12` / `sparql-12` feature flags), a new `SparqlEvaluator` builder-based query API, JSON-LD 1.1 by default, GeoSPARQL functions, a built-in `/sparql` HTTP server, single-pass ORDER BY, and — most relevant here — **built-in RDFC 1.0 canonicalisation** (W3C Recommendation, 21 May 2024), which gives deterministic blank-node identifiers via SHA-256 over canonical N-Quads. RDFC 1.0 is the proper successor to the bnode-filter hotfix in PR #14: a follow-up release can replace the `_:`-prefix filter in `extract_vocabulary` with canonicalisation, keeping semantic content while solving the reparse-instability problem at its root. The 0.4 → 0.5 migration was back-compatible for this codebase (the auto-migrating on-disk format means existing databases load without intervention). All six `Store::query` call sites in `graph.rs`, `shacl.rs`, and `ontology.rs` have been ported to the non-deprecated `SparqlEvaluator::new().parse_query(...).on_store(&store).execute()` chain; no deprecation warnings remain on the lib build. Full test suite (~290 tests) green on 0.5.8.

### Added
- **KGCL output format for `onto_drift`** (#17). The drift detector can now emit results in the [Knowledge Graph Change Language](https://github.com/INCATools/kgcl) (Mungall et al., Database 2025, doi:10.1093/database/baae133) alongside the existing JSON. Two new format options on the MCP tool: `format = "kgcl"` produces line-oriented CNL (`create node <iri>`, `obsolete node <iri>`, `obsolete node <iri> with replacement <iri>`) consumed by ROBOT and BioPortal; `format = "kgcl_json"` produces structured JSON-LD. High-confidence likely_renames (above `rename_threshold`, default 0.7) collapse into a `NodeObsoletion` with `has_direct_replacement` instead of plain add+remove pairs. New module `src/kgcl.rs` with 8 unit tests plus `tests/kgcl_drift_test.rs` integration suite.
- **LLM-orchestrated borderline-candidate review for `onto_align`** (#16). The alignment engine now splits its output into three buckets driven by two thresholds rather than a single `min_confidence` cliff: candidates with confidence above `high_threshold` (default 0.85) auto-apply as today, those in `[low_threshold, high_threshold)` (default low 0.4) surface in a new `borderline` array enriched with `context` (source/target labels and parent IRIs), and those below `low_threshold` are dropped. The MCP tool returns a `summary_for_review` instructing the connected LLM to inspect each borderline pair and call `onto_align_feedback` to record verdicts — those verdicts flow into the existing self-calibrating-weights loop. This is the MCP-native form of the LogMap-LLM "LLM-as-oracle" pattern (Jiménez-Ruiz et al., EACL 2026 main, top-2 in OAEI 2025 Bio-ML): no extra LLM client, no API key, no provider abstraction — the connected orchestrator does the judging via the conversation that already exists. New public method `AlignmentEngine::align_with_thresholds(source, target, high, low, dry_run)`; the legacy `align(source, target, min_confidence, dry_run)` remains and delegates with a degenerate range (empty borderline bucket) for back-compat. New `OntoAlignInput` fields `high_threshold` + `low_threshold` (both optional); `min_confidence` retained as the back-compat alias for `high_threshold`. New `tests/align_borderline_test.rs` (5 tests) covering bucket boundaries, context enrichment, summary text, and back-compat.
- **`onto_shacl_check` MCP tool — structural dry-run for proposed SHACL shapes** (#18). New `ShaclValidator::check_shapes(graph, shapes_ttl)` function and matching MCP tool that verifies (a) the shapes parse as Turtle and (b) every IRI they reference exists in the loaded ontology: `sh:targetClass` and `sh:class` must be declared as `owl:Class`/`rdfs:Class`; `sh:path` must be declared as `owl:ObjectProperty`, `owl:DatatypeProperty`, or `rdf:Property`; `sh:datatype` is prefix-checked against `xsd:`. Does NOT validate data — that's the existing `onto_shacl`. The intended workflow: the connected LLM generates candidate SHACL from a prose specification (the text2shacl paper, CiTIUS 2025, reports F1 0.904 / 0.934 / 0.699 on the EU ERA railway ontology with general-purpose LLMs), calls `onto_shacl_check` to catch missing IRIs, iterates, then runs `onto_shacl` to validate data. This is the MCP-native form of NL-to-SHACL: no LLM inside the server, no API key, the server provides the validation primitive and Claude does the authoring. Output includes per-shape diagnostic detail and an `issues` array categorised by `missing_target_class` / `missing_path` / `missing_class_constraint` / `unrecognised_datatype`. New `tests/shacl_check_test.rs` (7 tests covering well-formed shapes, each issue category, and Turtle parse failure).
- **DuckDB SQL data backbone**. New optional `duckdb` Cargo feature (and `sql` umbrella combining `postgres` + `duckdb`) wires DuckDB in alongside PostgreSQL as a *data integration* backbone — not as a SPARQL parser. DuckDB's extensions (`httpfs`, `parquet`, `csv`, `json`, `postgres_scanner`, `iceberg`, `delta`, …) let one SQL query federate over remote files, object stores, and other databases; rows then flow into the existing mapping/SHACL/reason pipeline.
- **New MCP tool `onto_sql_ingest`** — runs a SQL `SELECT` against PostgreSQL or DuckDB and ingests result rows into the triple store using the same `MappingConfig` shape as `onto_ingest`. Connection-string scheme is auto-detected (`postgres://`, `postgresql://`, `duckdb://`, `:memory:`, or a `*.duckdb` / `*.ddb` file path).
- **New CLI command `sql-ingest`** mirroring the MCP tool, with `--mapping`, `--inline-mapping`, `--base-iri`, and `-` (stdin) for the SQL.
- **`onto_import_schema` extended to DuckDB**. The same MCP tool / `import-schema` CLI now dispatches on the connection-string scheme: PostgreSQL via `sqlx`, DuckDB via the `duckdb` crate's `information_schema` + `duckdb_constraints()` introspection. The generated OWL is identical in shape (classes, datatype/object properties, NOT NULL → `owl:minCardinality 1`).
- **New `sql` tool group** in `[tools]` filter (`@sql` expands to `onto_import_schema` + `onto_sql_ingest`).
- **`SchemaIntrospector::sql_to_xsd` extended** to handle DuckDB-native types (HUGEINT, U{TINY,SMALL,}INT, DOUBLE, parameterised DECIMAL/VARCHAR, DATETIME, UUID, TIME).
- New tests: `tests/sqlsource_test.rs` (driver detection, no features required) and `tests/duckdb_test.rs` (introspection + query → row extraction, gated by the `duckdb` feature).

### Fixed
- **`onto_drift` ignores blank nodes**. Pizza-style ontologies (and any OWL with restriction classes) use anonymous blank-node restriction classes that get freshly reminted on every parse. Two snapshots of the same file would show ~40 added + ~40 removed bnodes plus a Cartesian product of confidence-scored "renames" between them, drowning real entity changes in noise. The vocabulary extractor now filters `_:`-prefixed IRIs from both class- and property-gather loops.

### Documentation
- `docs/data-pipeline.md` rewritten to cover both file-based and SQL-based ingest paths, the supported connection-string forms, federation examples (Parquet on S3 + Postgres scanner + remote CSV in one query), and a build matrix for the new feature flags.
- `SKILL.md`, `skills/ontology-engineering/SKILL.md`, `skills/ontology-engineer.md`, and `CLAUDE.md` Tool Reference tables expanded to cover the SQL backbone tools and previously-missing tools (`onto_status`, `onto_marketplace`, `onto_unload`, `onto_recompile`, `onto_cache_status`, `onto_cache_list`, `onto_cache_remove`, `onto_repo_list`, `onto_repo_load`, `onto_embed`, `onto_search`, `onto_similarity`, `onto_dl_explain`, `onto_dl_check`, `onto_import_schema`, `onto_sql_ingest`).

## [0.1.13] - 2026-05-01

### Added
- **Compile cache + TTL eviction + tool-exposure filter** (PR #1). Parsed ontologies are serialized to N-Triples on disk and reused on subsequent loads. A background evictor unloads idle ontologies after `[cache] idle_ttl_secs` (alias `unload_timeout_secs`); the on-disk cache is preserved and reloaded transparently on the next query. New `[tools]` config and `--tools-allow` / `--tools-deny` CLI flags restrict which `onto_*` tools the MCP server advertises (groups: `read_only`, `mutating`, `governance`, `remote`, `embeddings`).
- **New MCP tools**: `onto_cache_status`, `onto_cache_list`, `onto_cache_remove`, plus optional `name` parameter on `onto_unload` / `onto_recompile` for per-name cache management.
- **Ontology repository directories** (PR #2). New `[general] ontology_dirs` config (alias `data_dirs`) and `OPEN_ONTOLOGIES_ONTOLOGY_DIRS` env var let containerized deployments mount a folder of ontologies. Two new MCP tools enumerate and load from those directories with path-traversal guards: `onto_repo_list`, `onto_repo_load`.
- **OpenAI-compatible embeddings provider** (PR #3). New `[embeddings] provider = "openai"` mode targets any OpenAI-compatible gateway (official OpenAI, Azure, Ollama, vLLM, LocalAI, LM Studio, Together, …). Config fields: `api_base` (alias `base_url`), `api_key`, `model`, `dimensions`, `request_timeout_secs`. Env-var precedence: `OPEN_ONTOLOGIES_EMBEDDINGS_*` > `OPENAI_API_KEY` (for the key) > config > defaults. Remote responses are L2-normalized to remain comparable with local ONNX embeddings.
- **Surfaced operational config** (PR #4). New `[webhook]`, `[http]`, `[monitor]`, `[reasoner]`, `[feedback]`, `[imports]`, `[repo]`, `[socket]`, `[logging]` config sections expose previously hardcoded limits (tableaux depth/nodes, RDFS/OWL-RL fixpoint iterations, monitor interval, webhook timeout, import depth and remote-follow policy, feedback suppress/downgrade thresholds, etc.). A `0` value in the timeout / iteration fields is a sentinel that falls back to the documented default.
- New tests: `tests/registry_test.rs`, `tests/cache_management_test.rs`, `tests/toolfilter_test.rs`, `tests/repo_test.rs`, plus inline tests for embeddings config parsing and runtime knob initialization.

### Documentation
- New `docs/cache-and-registry.md` covering the compile cache, TTL eviction, tool-exposure filter, and ontology repository directories.
- `docs/embeddings.md` expanded with the OpenAI-compatible provider, supported gateways, config block, and env-var precedence.
- `CLAUDE.md` and `SKILL.md` Tool Reference tables updated with the seven new tools.

## [0.1.12] - 2026-03-27

### Added
- Virtualized tree view replacing D3/3D graph (handles 1500+ classes)
- Hierarchy connector lines, breadcrumb, and connections panel
- 13-step deep builder (`/build` command) producing IES-level ontologies
- `/sketch` command for quick prototyping
- `rdfs:Class` and `rdf:Property` support in Studio (not just `owl:Class`)
- Shared cargo target directory

### Fixed
- Static Linux binary via musl target (closes #2)

## [0.1.11] - 2026-03-25

### Added
- IES marketplace presets (`ies-top`, `ies-core`, `ies`)
- IES Building Extension (525 classes, clean-room)
- RDFS inference depth benchmark (662 vs 621)
- Head-to-head IRIS comparison
- Hierarchy enforce rule pack
- EPC benchmark (36/36 vs 18/36)

### Changed
- Default features off (lean build — drops tract-onnx and sqlx from default)

## [0.1.10] - 2026-03-13

### Added
- Quickstart guide (`docs/quickstart.md`)
- Server round-trip integration test (`tests/server_roundtrip_test.rs`)
- Complete architecture table in CONTRIBUTING.md (26 modules)

### Fixed
- Inconsistent CLI output: version/history/rollback/enrich/validate-clinical now respect `--pretty`
- CONTRIBUTING.md architecture table missing 10 modules (error, config, inputs, lineage, mapping, state, schema, embed, structembed)

## [0.1.9] - 2026-03-13

### Added
- Embedding similarity as alignment signal #7 (`onto_align` now uses text+structural embeddings when available)
- `onto_embed`, `onto_search`, `onto_similarity` MCP tools for semantic search
- End-to-end embedding pipeline test
- Embedding tools in architecture diagram and workflow documentation

### Fixed
- Feature gating for `tool_router` macro, clippy warnings, and tokenizer download
- Linux binary now built on ubuntu-22.04 for wider glibc compatibility

## [0.1.8] - 2026-03-12

### Added
- Poincare structural embedding trainer (Riemannian SGD for hierarchy layout)
- ONNX text embedder with tract (bge-small-en-v1.5, downloaded on init)
- Dual-space vector store with cosine + Poincare search and SQLite persistence
- Poincare ball geometry module (distance, exp_map, Riemannian SGD)

### Fixed
- Release binary naming now includes target triple
- Replaced deprecated macos-13 runner with macos-14

## [0.1.6] - 2026-03-11

### Added
- Glama server metadata and author verification

### Fixed
- Docker runtime libs and removed init from Dockerfile

## [0.1.5] - 2026-03-11

### Fixed
- Added build-essential and clang to Docker builder for oxrocksdb-sys compilation

## [0.1.4] - 2026-03-11

### Fixed
- Installed OpenSSL and libpq dev headers in Docker builder stage

## [0.1.3] - 2026-03-10

### Fixed
- Use latest Rust image in Dockerfile (dependencies need Rust 1.88+)

## [0.1.2] - 2026-03-10

### Fixed
- Free disk space in Docker workflow and optimize build
- Bumped server.json to v0.1.1

## [0.1.1] - 2026-03-09

### Added
- MCP Registry server.json, Docker publish workflow, and OCI label
- Streamable HTTP transport (`serve-http` command)
- MCP prompts (build_ontology, validate_ontology, compare_ontologies, ingest_data, explore_ontology)
- Dockerfile for containerized deployment
- OntoAxiom benchmark showdown (tool-augmented vs bare LLMs)
- Claude Code plugin package and ClawHub skill wrapper
- Bare Claude and hybrid benchmarks for three-way comparison
- Self-calibrating feedback for lint and enforce (dismiss 3x to suppress)
- Ontology alignment (`onto_align`, `onto_align_feedback`) with 6 weighted signals
- Terraform-style lifecycle: plan, apply, lock, drift, enforce, monitor, lineage
- Data pipeline: ingest, map, SHACL validate, reason, extend
- Clinical crosswalks (ICD-10, SNOMED, MeSH)
- SHIQ tableaux reasoner with parallel classification
- Design pattern enforcement (generic, BORO, value_partition)
- Version snapshots and rollback
- Core ontology tools: validate, load, save, query, stats, diff, lint, convert, clear, pull, push, import

### Fixed
- Clippy `io_other_error` warning breaking CI
- MCP benchmark scoring (camelCase normalization, pair order)
