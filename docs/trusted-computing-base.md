# The trusted computing base of the Lean certificate layer

`lean/` proves a conditional. `OOCert.certificate_sound` says: if the asserted graph is `G`, and
if these derivation steps check, then every conclusion holds in every model of `G`. Everything to
the left of that "if" is the Rust engine writing down the truth. If the engine emits a certificate
whose `asserted.tsv` is not the graph it reasoned over, or whose steps are not the steps it took,
then a valid Lean proof certifies a false claim and the layer is decoration.

That gap is the trusted computing base. This page names it exactly, as a list of properties that can
be checked, and says for each one what checks it today.

The point of writing it down is that an unwritten trusted base cannot be reviewed. A reader who
finishes this page should be able to say how much of the engine is verified (almost none of it) and
which specific, small pieces carry the weight (the ones below).

**Where this stands on 15 September 2026.** Of the twenty-nine properties, six are PROVED by bounded
model checking rather than sampled: TCB-1, TCB-2, TCB-3, TCB-4, TCB-5 and TCB-20. Five moved from
something this repository OBSERVED to something it ENFORCES: TCB-4 and TCB-5 are now a refusal at
the certificate writer rather than a fact about `oxrdf` and `oxiri`; TCB-8 across runs was a DEFECT
pinned by a test and is fixed for the named inference graph; TCB-14 is derived from one rule table
instead of hand-written at thirty-one call sites; TCB-26 is checked at the point of writing instead
of by a second loop. The two lists overlap at TCB-4 and TCB-5, so nine distinct properties changed
level. What is left irreducible is two things, and they are named at the end.

## What the Lean side never sees

The checkers are pure functions of files on disk. They read `asserted.tsv`, `derivations.tsv`,
`rules.tsv`, `horn.tsv`, `axioms.tsv` and `model.tsv`, and nothing else. In particular they cannot
see:

- the RDF store, so they cannot tell whether `asserted.tsv` is the store's contents;
- the reasoner's execution, so they cannot tell whether a step in `derivations.tsv` was ever taken;
- the store after the run, so they cannot tell whether a triple was materialised without a
  certificate line covering it;
- the rule file the user passed, so they cannot tell whether `rules.tsv` is the table the engine
  evaluated.

Each of those four blind spots is a property below.

## The emission paths

| path | entry point | files written | checker |
|---|---|---|---|
| built-in forward chaining | `Reasoner::run_full` (`src/reason.rs`) | `asserted.tsv`, `derivations.tsv` | `oo-cert` |
| supplied Horn rules | `Reasoner::run_horn` (`src/reason.rs`) | `rules.tsv`, `asserted.tsv`, `horn.tsv` | `oo-horn` |
| DL model certificate | `DlReasoner::write_model_certificate` (`src/tableaux.rs`) | `axioms.tsv`, `model.tsv` | `oo-dlmodel` |
| SHACL | no Rust emission | N-Triples handed to the Lean evaluator | `oo-shacl` |

The SHACL row is different in kind and is treated separately at the end.

## The properties

Each property has an identifier, a statement, the code it is about, and how it is checked. `TCB-*`
identifiers are used by `tests/certificate_boundary_proptest.rs`, which names them in its test
functions.

### Serialisation: the format cannot be forged from inside a term

All six files are tab-separated, carry arbitrary RDF terms, and have no escaping layer of their own.
`lean/OOCert/Parse.lean` states the assumption that makes this safe, in a comment rather than a
theorem:

> Tabs and newlines cannot occur inside an N-Triples term (they are escaped), so splitting on them is
> exact.

Until 15 September 2026 that sentence was the whole of the escaping argument, and it was a claim
about a third-party crate. The terms come from `oxrdf`'s `Display`, through the store readers, which
call `quad.subject.to_string()` and friends. `oxrdf 0.3.3` escapes
`\t`, `\n`, `\r`, `"` and `\` inside a literal's lexical form (`print_quoted_str` in `literal.rs`),
and does **not** escape anything inside an IRI: `NamedNodeRef`'s `Display` is
`write!(f, "<{}>", self.as_str())`, so the only thing standing between a tab and `asserted.tsv` was
that `oxiri` refused to parse the IRI. That is a dependency's behaviour observed rather than
enforced, and an upgrade could change it.

**This repository now enforces it.** `term_fits_the_format` in `src/reason.rs` is the predicate, and
the two writers `push_asserted_line` and `push_triple_fields` are the only functions that append a
term to a certificate buffer. A term is written only if it is non-empty, carries no `\t`, `\n` or
`\r`, and is in one of the three N-Triples spellings (`<iri>`, `_:blank`, or a quoted literal). A
term that is not means no certificate: the writer returns `Err(Position)`, the caller turns that into
an error naming the term, and nothing is written. The check happens before anything is appended, so a
refusal cannot leave half a line in the buffer.

The refusing direction is the point. Escaping at the writer would be the other way to close it, and
it would need `lean/OOCert/Parse.lean` and `Shacl.Parse` to learn the same escaping, which is a
change to the verified side for a case that cannot arise from a well-formed store. Refusing needs
nothing from the checkers and fails safe: the engine declines to make a claim rather than making one
a reader cannot trust. The cost is liveness, and it is real: if `oxrdf` ever started emitting a raw
tab inside a literal, this engine would stop writing certificates instead of writing forgeable ones.

- **TCB-1 (field integrity, asserted).** Every line of `asserted.tsv` splits on `\t` into exactly
  three fields, and the number of lines equals the number of triples the run started from. A term
  carrying a tab would split a line into four fields, and a term carrying a newline would split one
  line into two; either forges a triple the store never held.
- **TCB-2 (field integrity, derivations).** Every line of `derivations.tsv` splits into
  `1 + 3 * (1 + premises)` fields with `premises >= 1`.
- **TCB-3 (field integrity, Horn).** Every line of `horn.tsv` splits into
  `2 + 2 * binds + 3 * (1 + body)` fields, with `binds` and `body` read from the line's own header
  and from `rules.tsv`.
- **TCB-4 (no separator in any term).** No term written to any certificate file contains `\t`,
  `\n` or `\r`. **Enforced** by `term_fits_the_format` at the writer and **proved** by
  `kani_harnesses::term_guard_admits_no_separator_at_3` and `_at_6`, which state it over a symbolic
  index rather than restating the implementation's own scan, and by the writer harnesses, which
  prove the writer returns `Ok` exactly when all three terms pass.
- **TCB-5 (term spellings do not collide).** A literal whose lexical form is `<http://example/x>`
  and the IRI `<http://example/x>` must not have the same spelling in a certificate file. **Enforced**
  by the same predicate, whose leading-byte condition makes the kind of a term a function of its
  first byte, and **proved** by `kani_harnesses::term_guard_separates_the_three_spellings`: an
  accepted term is exactly one of the three kinds, and two accepted terms of different kinds are
  different strings. That is what entitles both checkers to compare terms as opaque strings.

### The asserted graph

- **TCB-6 (asserted is the graph, no drops).** Every triple in the store at the start of a certified
  run appears as a line in `asserted.tsv`. A dropped assertion makes the checker reject a legitimate
  step, so this direction is fail-safe, but it is still part of the claim the file makes.
- **TCB-7 (asserted is the graph, no additions).** Every line of `asserted.tsv` corresponds to a
  triple that was in the store at the start of the run. This is the unsafe direction. An extra line
  is an axiom nobody asserted, and every conclusion resting on it is certified against a graph that
  does not exist.
- **TCB-8 (no inference leaks into the assertions).** No conclusion of `derivations.tsv` appears in
  `asserted.tsv`. Within a run this holds because the engine reaches a fixpoint in one pass and
  captures `facts` before materialising.
  **Across runs, the half that was a defect is fixed and the half that is a caller's choice is
  not.** The defect: `GraphStore::all_triples` iterates `store.iter()` over every graph, so
  inferences that `inference_graph: true` parked in `https://open-ontologies.org/graph/inferred`
  under decision 0001 were read back as assertions by the next certified run. The separation
  protected `save` and not the certificate, and a test asserted that failure rather than the
  property. Both certified paths now read `GraphStore::triples_outside(&[INFERRED_GRAPH])`, and both
  report `graphs_read` and `graphs_excluded` in their JSON, so a certificate says which graphs it is
  about and that claim can be checked against the store.
  What remains is `InferenceTarget::DefaultGraph`, which merges conclusions into the default graph
  beside the assertions because the caller asked for that. After the merge nothing distinguishes
  them, `asserted.tsv` has no column that says "derived", and no layer above the store can recover
  the distinction. `tests/certificate_boundary_proptest.rs::tcb_8_across_runs_only_the_default_graph_leaks`
  asserts the fix for the named graph and the leak for the merged one, in one test, so neither half
  can change without the documentation being forced to change with it.
- **TCB-9 (the store is a set, the file is a list).** A triple present in two named graphs is
  written twice, because `all_triples` flattens quads. Duplicate lines are harmless to soundness
  (the Lean side builds a `HashSet`) but `asserted` in the JSON report counts lines, not distinct
  triples. The Horn path reports both numbers; the built-in path reports only the line count.

### The derivation steps

- **TCB-10 (every inference is certified).** The set of conclusions in `derivations.tsv` equals the
  set of triples the run added to the store. A triple materialised without a line covering it is an
  uncertified triple sitting in the store under the certificate's cover. Structurally this holds
  because the single `emit` closure is the only path that pushes a candidate, and it records the
  first derivation of anything not already in the closure, but nothing outside that reading enforces
  it. The reading is shorter than it was: `emit` now computes the conclusion from the rule table for
  twenty-seven of the thirty-one sites, so a site cannot push a triple its rule does not license,
  and a site that passed a conclusion and premises that did not correspond is no longer expressible.
  It is still a reading of a loop plus a property test comparing conclusions against the store's
  before and after.
- **TCB-11 (one line per inference).** `derivations.len() == inferred_count`. A conclusion is
  recorded the first time it is derived and never again.
- **TCB-12 (premises precede conclusions).** Every premise of the step on line `i` is either in
  `asserted.tsv` or is the conclusion of a line `j < i`. No step cites itself.
  `OOCert.checkHornAll` and `OOCert.checkAll` both demand this, so a violation is a rejection rather
  than a false pass. It is in this list because an emitter that violates it produces certificates
  that fail for no reason a user can act on, and because the argument that it holds (indices are
  rebuilt from `triple_set` at the top of each round, and `new` is merged only at the bottom) is a
  reading of the loop, not a check.
- **TCB-13 (the rule name is one the checker knows).** Every rule name in `derivations.tsv` is
  accepted by `OOCert.Rule.ofName?`. An unknown name is a parse error, so this is fail-safe, but a
  rule renamed on one side and not the other silently stops the corpus test from covering it.
- **TCB-14 (the premise order matches the rule's arm).** `OOCert.checkStep` pattern-matches the
  premises positionally per rule. **The emitter no longer chooses the order.** Twenty-seven of the
  thirty-one call sites now fire as `Fired::Bound(rule, binding)`: the site supplies the terms and
  the premise list AND the conclusion are computed from that rule's row in `BUILTIN_RULES`
  (`src/reason.rs`), which is one table. A site has no order to get wrong.
  The remaining four are the list rules `cls-int1`, `cls-int2`, `cls-uni` and `cls-oo`, whose
  premises are a constructor triple plus an RDF list chain as long as the list; the checker matches
  those with `takeChain` rather than positionally, so they are not a fixed pattern on either side
  and they keep an explicit premise vector.
  That leaves one question: is `BUILTIN_RULES` the checker's table? `tests/premise_order_test.rs`
  answers it by parsing the `checkStep` ARMS out of `lean/OOCert/Rules.lean` (the code, not the
  prose table in its docstring), rebuilding each arm's pattern from its premise list, its `x = y` and
  `x = V.foo` conjuncts and its `st.conclusion = ⟨..⟩`, and comparing the two up to renaming of
  variables. A disagreement in order, in arity, in which position is fixed, or in which vocabulary
  term is fixed, fails `cargo test`. Checked by refutation both ways: swapping `rdfs2`'s two premises
  fails it, and writing `scm-avf2`'s conclusion the natural way round fails it twice.

### The interner

Every term in every certificate file goes through `Interner` (`src/reason.rs`), a `HashMap<String,
u32>` plus a `Vec<String>`. Both the reasoning and the writing use interned identifiers, so if the
interner were not a bijection two different terms would print the same and a step would be checked
against a triple that is not the one the engine used.

- **TCB-15 (round trip).** `resolve(intern(s)) == s` for every string.
- **TCB-16 (injectivity).** `intern(a) == intern(b)` if and only if `a == b`.
- **TCB-17 (stability).** An identifier's resolution does not change as more strings are interned.

### The rule table

- **TCB-18 (the table checked is the table evaluated).** `run_horn` writes `rules_tsv(&rules)`
  rather than the user's file, and re-parses what it wrote and compares before writing it out. That
  guard is in the code already and is the single best thing in this layer. The property it does not
  cover is that `OOCert.HornParse.parseRules` reads that file as the same table, because the
  comparison is between two Rust values.
- **TCB-19 (Rust and Lean parse a rule table the same way).** For every table the Rust accepts,
  `parseRules` yields the same rules. The two parsers are independent implementations. Rust is
  strictly stricter on three points, all in the refusing direction: it requires a constant to be in
  N-Triples spelling, it refuses a head variable not bound by the body, and it refuses a carriage
  return. Lean's `patOf` treats any field not starting with `?` as a constant.
- **TCB-20 (render and parse are inverse).** `parse_rules(rules_tsv(r)) == r`. In particular a
  constant never renders to something that reads back as a variable, and a variable name never
  renders to something that reads back as a constant.
- **TCB-21 (`rules_tsv` and `ruleStr` agree byte for byte).** The Rust doc comment on `rules_tsv`
  asserts this of `OOCert.HornParse.ruleStr`. It matters because the JSON report publishes a SHA-256
  of the Rust rendering, and `oo-horn` decides `entailed` against `entailed_under_supplied_rules` by
  comparing tables.
- **TCB-22 (the rule index means the same thing on both sides).** `horn.tsv` cites a rule by index.
  The index is a position in `crules`, which is built by mapping over `rules` in order, and
  `rules.tsv` is rendered from `rules` in the same order. Empty lines are skipped by both parsers and
  `rules_tsv` never emits one.
- **TCB-23 (the binding instantiates the body and the head).** For each `horn.tsv` line, the
  premises equal the rule's body under the written substitution, and the conclusion equals the head
  under it. `OOCert.checkHornStep` checks exactly this, so it is fail-safe, and it is listed because
  a disagreement here is the difference between a certificate that checks and one that does not.
- **TCB-24 (nothing is materialised under a supplied table).** `run_horn` writes no triple into the
  store. A conclusion under a user's rules holds only in models that satisfy those rules, and
  merging it into the store loses that distinction. This is decision 0003.

### The DL model certificate

`axioms.tsv` and `model.tsv` are tab separated at the top level and **space separated inside a
concept** (`concept_string` in `src/tableaux.rs`), so the DL path is exposed to spaces as well as
tabs.

- **TCB-25 (every name survives the format).** `name_is_safe` refuses any name that is empty or
  contains a space, tab, carriage return or newline, and the emitter refuses to write a certificate
  if any name fails. This is the only explicit injection guard in the codebase, and it is in the
  right place.
- **TCB-26 (the guard covers every name that is written).** It used to be an argument about two
  separate loops agreeing: a `names` vector built by walking every axiom variant plus `model.rext`
  and `model.cext`, with individuals in `model.ind` covered only because `DlAxiom::Indiv(i)` happens
  to be emitted for every individual that reaches the model. **The separate loop is gone.**
  `push_name` is the only function that appends a name to either buffer; it applies `name_is_safe`
  as it writes and records the first refusal, and the emitter refuses the whole certificate before
  anything reaches disk. Coverage is therefore not a question: there is no other way to write a
  name. `tableaux::certificate_boundary_tests::tcb_26_only_push_name_writes_a_name` reads the
  source of the serialisers and of the emitter and fails if any other `resolve` reaches a
  certificate buffer, and the TCB-25 property tests now condition on the GUARD's verdict rather
  than on the test author's recollection of which names a variant mentions.
- **TCB-27 (the emitter refuses rather than lies).** Before writing, the emitter runs the same
  semantics the checker runs (`model.well_formed`, then `model.holds` on every axiom) and refuses
  on failure. A certificate that will not check is worse than no certificate.

### SHACL

There is no Rust emission path. `tests/shacl_core_verified_test.rs` hands the Lean evaluator
N-Triples produced by the engine's own parser with an explicit base IRI, and the Lean side parses
them itself. So the boundary here is not a certificate format but the N-Triples serialiser, and the
Lean parser is a second, independent RDF reader.

- **TCB-28 (the two RDF readers agree).** `Shacl.Parse.readTerm` accepts a subset of N-Triples and
  keeps escapes undecoded, so `"a\tb"` is the term `"a\tb"` with a backslash and a `t` in it, not a
  tab. The engine's terms are in the same undecoded spelling, so the two agree, but they agree by
  construction on both sides rather than by a check. A term the Lean parser reads differently is a
  shape applied to a different graph, silently.
- **TCB-29 (the SHACL layer is not a certificate layer).** `oo-shacl` validates; it does not check
  a record of what the Rust validator did. The Rust SHACL validator's agreement with the verified
  one is measured by `tests/shacl_core_verified_test.rs` and by the pyshacl differential, and
  measurement is not proof.

## Status

The `level` column is the point of this table. **proved** means a bounded model checker verified the
statement over every input up to a stated bound; **enforced** means the engine refuses rather than
relies on something being true; **property** means a generator samples it; **argued** means a reading
of the code with nothing checking it. Nothing here is unbounded-verified, and a reader quoting
"proved" off this table without the bound in the section below is misquoting it.

| property | level | checked by | how |
|---|---|---|---|
| TCB-1 | **proved** + property | five Kani harnesses, `tests/certificate_boundary_proptest.rs`, `src/reason.rs` | the byte layout at term lengths 0 to 4, plus the property end to end and on the writer |
| TCB-2, TCB-3 | **proved** + property | four Kani harnesses, same tests | as above at lengths 0, 2, 3 and 4 |
| TCB-4 | **proved** + **enforced** | `kani_harnesses::term_guard_admits_no_separator_at_3` and `_at_6`, `term_fits_the_format` | the writers refuse a term carrying a separator; the harness states "no separator anywhere" over a symbolic index. Two deterministic tests remain: the parser refuses a separator inside an IRI, and Turtle's long-string form (which carries RAW control characters) comes back escaped |
| TCB-5 | **proved** + **enforced** | `kani_harnesses::term_guard_separates_the_three_spellings` | an accepted term is exactly one of the three N-Triples kinds, and the kind is its leading byte |
| TCB-6, TCB-7 | property | proptest | `asserted.tsv` is re-read through the N-Triples parser into a fresh store and compared |
| TCB-8 | **enforced** + property | `GraphStore::triples_outside`, proptest | the certified paths do not read `INFERRED_GRAPH` and report the graphs they did read; within a run the property is sampled; the DefaultGraph merge is pinned in the same test as the leak it still is |
| TCB-9 | deterministic test | | a quad in two named graphs is two lines |
| TCB-10, TCB-11 | property | proptest | conclusions compared against the store's before and after |
| TCB-12 | property | proptest | both the built-in and the Horn path |
| TCB-13 | property | proptest | the name list is read out of `lean/OOCert/Rules.lean` at test time |
| TCB-14 | **enforced** | `BUILTIN_RULES`, `tests/premise_order_test.rs` | 27 of 31 sites get the order from one table; that table is compared with the `checkStep` arms parsed out of `Rules.lean`. `tests/lean_certificate_test.rs` still runs the real checker over the corpus |
| TCB-15, TCB-16, TCB-17 | property | `src/reason.rs` unit property tests | the interner is private |
| TCB-18 | **enforced** + property | in the code, and proptest | the engine re-parses what it wrote and refuses on mismatch |
| TCB-19, TCB-21 | property | proptest, plus the REAL checker | property against a transcription of `OOCert.HornParse` in the test file, because a Lean process per case is not a property test. The transcription's fidelity is then an assumption, so one deterministic test puts the adversarial shapes (a numeric rule name, a non-ASCII one, a `??x` variable, a literal spelled exactly like an IRI as a constant, a blank node, an empty-bodied rule) through `oo-horn check` itself and requires the conditional verdict |
| TCB-20 | **proved** + property | `kani_harnesses::pat_of_and_render_are_inverse_at_2` and `_at_3`, proptest | `pat_of` and `Pat::render` are inverse over every byte pattern at two and three bytes. This is the harness the previous version of this page reported as NOT TERMINATING |
| TCB-22 | property | proptest | the rule index is a position in a list rendered in the same order, and no rendered line is empty |
| TCB-23, TCB-24 | property | proptest | the substitution is re-applied from `rules.tsv` independently of the engine |
| TCB-25 | **enforced** + property | `name_is_safe`, `src/tableaux.rs` unit property tests | the emitter refuses; `concept_string` token counts and `axiom_line` field counts are sampled |
| TCB-26 | **enforced** | `push_name`, `tcb_26_only_push_name_writes_a_name` | the guard is the write, and a source-level test fails if any other `resolve` reaches a certificate buffer |
| TCB-27 | **enforced** | in the code | the emitter runs the checker's own semantics and refuses; `tests/dl_model_certificate_test.rs` exercises it |
| **TCB-28, TCB-29** | nothing here | | the SHACL boundary is different in kind, see below |

The generators build adversarial graphs (literals holding tabs, newlines, carriage returns, quotes
and backslashes; a lexical form spelled exactly like an IRI; a lexical form spelled exactly like a
whole extra TSV line; combining characters against their precomposed form; NUL; the empty graph;
language tags; typed literals; percent-encoded separators inside IRIs) and adversarial rule tables,
and then drive the real public entry points.

**Where this runs.** `tests/certificate_boundary_proptest.rs` needs lake, because the TCB-19/TCB-21
leg puts the adversarial shapes through `oo-horn check` itself. Until 15 September 2026 no CI job
invoked it: the `build` job ran it with no lake and it skipped, and the `lean` job, which has lake,
never named it. It is now a strict leg of the `lean` job under `OO_REQUIRE_FIXTURES=1`, so the
skip is a failure there, and so are `reason_rl_coverage_test`, `rule_syntax_frontend_test` and
`reason_horn_emit_test`, which were in the same position. `tests/premise_order_test.rs` needs no
toolchain at all, because it READS `lean/OOCert/Rules.lean` rather than building it, so TCB-14 is
gated in every job that runs `cargo test`. `docs/ci-gates.md` is the table of which gate runs where,
and Kani is still in no job at all, which is why the section below reports its results rather than
pointing at a build.

### Bounded model checking

Kani 0.67.0 installed and ran here, on `nightly-2025-11-21-aarch64-apple-darwin`, over the whole
crate. Fifteen harnesses in `src/reason.rs` prove the serialisation properties over every byte
pattern at a stated length, rather than over the patterns a generator drew. `make verify` runs all
fifteen; it used to run three of four, because the fourth did not terminate. None are part of
`make check`, because Kani pulls its own toolchain and the same statements are sampled by
`cargo test` for anyone without it.

| harness | property | result |
|---|---|---|
| `asserted_line_round_trips_at_0` | TCB-1, TCB-4 | SUCCESSFUL, 646 checks, 0 failed, 3.6s |
| `asserted_line_round_trips_at_1` | TCB-1, TCB-4 | SUCCESSFUL, 652 checks, 0 failed, 5.4s |
| `asserted_line_round_trips_at_2` | TCB-1, TCB-4 | SUCCESSFUL, 652 checks, 0 failed, 6.3s |
| `asserted_line_round_trips_at_3` | TCB-1, TCB-4 | SUCCESSFUL, 652 checks, 0 failed, 7.0s |
| `asserted_line_round_trips_at_4` | TCB-1, TCB-4 | SUCCESSFUL, 652 checks, 0 failed, 7.8s |
| `triple_fields_append_exactly_three_at_0` | TCB-2, TCB-3, TCB-4 | SUCCESSFUL, 647 checks, 0 failed, 3.5s |
| `triple_fields_append_exactly_three_at_2` | TCB-2, TCB-3, TCB-4 | SUCCESSFUL, 653 checks, 0 failed, 6.2s |
| `triple_fields_append_exactly_three_at_3` | TCB-2, TCB-3, TCB-4 | SUCCESSFUL, 653 checks, 0 failed, 7.0s |
| `triple_fields_append_exactly_three_at_4` | TCB-2, TCB-3, TCB-4 | SUCCESSFUL, 653 checks, 0 failed, 7.7s |
| `term_guard_admits_no_separator_at_3` | TCB-4 | SUCCESSFUL, 131 checks, 0 failed, 0.3s |
| `term_guard_admits_no_separator_at_6` | TCB-4 | SUCCESSFUL, 131 checks, 0 failed, 0.4s |
| `term_guard_separates_the_three_spellings` | TCB-5 | SUCCESSFUL, 264 checks, 0 failed, 1.1s |
| `pat_of_and_render_are_inverse_at_2` | TCB-20 | SUCCESSFUL, 585 checks, 0 failed, 4.2s |
| `pat_of_and_render_are_inverse_at_3` | TCB-20 | SUCCESSFUL, 585 checks, 0 failed, 4.6s |
| `writable_triple_decides_both_positions` | the guard that fixed the defect below | SUCCESSFUL, 198 checks, 0 failed, 0.5s |

15 harnesses, 7754 checks, 0 failed, 66s of solver time in total.

**What changed on 15 September 2026.**

- **The writers now decide, so the harnesses prove the decision.** They used to `assume` that no
  term carried a separator, because nothing in this repository enforced it: it was a fact about
  `oxrdf` and `oxiri`. `term_fits_the_format` is now the predicate and the writers return
  `Err(Position)`, so the assumption became a branch of the function under test: the harnesses prove
  `Ok` exactly when all three terms pass, the byte layout when it is `Ok`, and an untouched buffer
  when it is not.
- **The harness that did not terminate, terminates.** The previous version of this page reported
  `parse_pat_and_render_are_inverse` as NO VERDICT, abandoned at 14 minutes and 9.5 GB, and named
  what would close it: "lifting the classification out of `parse_pat` into a pure `Option<Pat>`
  function, leaving the messages where they are. That is a refactor of shipped code and it was not
  made here." It was made here. `pat_of` is that function, `parse_pat` is now `pat_of` plus the same
  three messages, and `Pat::render` builds its string instead of `format!`-ing it for the same
  reason. The harness verifies in about four seconds. The measurements that motivated it are kept on
  the harness in the source, because the next person to write one against an `anyhow`-returning
  function should not have to rediscover why it is hard.
- **A bound that was too low was found by raising it.** The byte-scan loops run `3N+3` or `3N+4`
  times, so at four bytes per term the second needs seventeen unwindings and had sixteen. CBMC
  reported `1 of 653 failed (652 undetermined)`, which is its own unwinding assertion, and it was
  found
  because `cargo kani --harness NAME` selects by SUBSTRING, so the bare name ran all five length
  instantiations together and one of them failed inside a run reported under another heading. Every
  harness now has a full name ending in its length, and the bound is 24. A verification target that
  reports a failure under the wrong name is worse than one that does not run.

**What the bounds are, and they are real.** Terms are a FIXED number of bytes, each byte
unconstrained ASCII, and the harnesses are instantiated at several lengths rather than one: 0, 1, 2,
3 and 4 for the asserted line, 0, 2, 3 and 4 for the triple fields, 3 and 6 for the term guard, 2 and
3 for the rule-table position. The length is fixed rather than symbolic because a symbolic `n` makes
every offset in the line symbolic and every slice bound a case split, and the cost compounds across
three terms: two bytes per term with a symbolic length reached fifteen gigabytes without a verdict.
So these prove every byte pattern at each of those lengths, and the property tests sample the
lengths beyond them. Neither alone is the whole claim, and anyone quoting "verified" off this table
without the bound is misquoting it.

Two costs were paid down rather than hidden, because both are the kind of thing that makes a
verification effort quietly measure the wrong function:

- `core::str::from_utf8` put CBMC inside `run_utf8_validation` once per buffer: seventeen minutes and
  three gigabytes with no verdict, the log a wall of `Unwinding loop ... run_utf8_validation`. The
  harness assumes ASCII, which is exactly what that function checks, so it uses
  `from_utf8_unchecked`.
- Asserting on `body.split('\t')` put CBMC inside `CharSearcher`: ten minutes and two gigabytes at
  two bytes per term. It is also the wrong claim. The reader is `OOCert.Parse.parseTriples`, which is
  Lean's `String.splitOn`, so a proof about Rust's `str::split` proves a property of the wrong
  splitter. The harnesses assert the BYTE LAYOUT (exactly two tabs, exactly one newline, that
  newline last, the three terms at the offsets those separators imply), which is what any correct
  splitter needs and all it needs. The `split` formulation is kept as a property test, where it is
  free.

The pattern in all of these is one thing, and it is what to take away from this section: three
times, the first formulation of a harness measured a function nobody asked about, and once a green
result was reported under the wrong harness's name. A verification effort that does not check what
it is actually measuring produces a green result about the wrong thing, which is the failure mode
this whole layer exists to attack.

### What this found

The first run of TCB-1, at case 115, found a real defect, shrunk to two triples:

```
<http://e/A> <http://e/q> <http://e/A> .
<http://e/q> <http://www.w3.org/2000/01/rdf-schema#subPropertyOf> _:b0 .
```

`rdfs7` concluded `<http://e/A> _:b0 <http://e/A>`, a blank node in predicate position, which no
RDF serialisation can express. `GraphStore::load_ntriples` streams, so materialising failed
PARTWAY, leaving an arbitrary, hash-order-dependent prefix of the inferences in the store with no
certificate covering any of them, and `run_full` returned the parser's error. In a dry run the
certificate WAS written and contained the unserialisable conclusion, which the Lean checker accepts
because it holds terms as opaque strings. It is reachable from ordinary OWL:
`:p rdfs:subPropertyOf [ owl:inverseOf :q ]` is what the OWL 2 mapping to RDF produces for
`SubObjectPropertyOf(:p ObjectInverseOf(:q))`.

Measuring how far it reached found a second one. The SUBJECT case was found on 30 August 2026 and
guarded at `prp-symp`, `prp-inv1`, `prp-inv2` and `eq-sym`. It was still open at `cls-avf`, which
derives `y rdf:type c` from `x rdf:type ∀P.c` and `x P y`: an `owl:allValuesFrom` restriction on a
property with a literal value derived `"x" rdf:type :D` and broke the materialiser the same way.
That one needs no anonymous property expression and no unusual modelling at all.

Six fixtures, all legal RDF, made the unfixed engine return a parser error: `rdfs7`, `prp-inv1`,
`prp-inv2` and `cls-hv1` for the predicate position, `cls-avf` for the subject position, and
`rdfs7` again with a literal rather than a blank node. There is now one guard, `writable_triple`,
deciding both positions and shared with `run_horn`, which was the only path that already refused
either. Pinned by `tests/reason_unwritable_predicate_test.rs`, which also pins that `prp-symp` and
`prp-trp` cannot reach it.

The lesson is the one this document is for. The 30 August fix was four targeted guards at the sites
its author had found, and the sites its author had not found stayed broken for two weeks. The
central guard is the fix; the property test is what found the sites.

## What is still trusted after all of this

This is the part that matters. Nothing below is verified. Each entry says what would close it. It
was ten entries on 14 September 2026 and it is six, because four of them were closed rather than
re-worded: the two that rested on `oxrdf` and `oxiri`, the premise order, and the DL name guard's
coverage.

1. **TCB-6 and TCB-7, that `asserted.tsv` IS the store's contents.** The engine reads the store,
   interns what it reads, and writes the interned strings back out. Nothing between the store and
   the file checks that the file is the store. `tests/certificate_boundary_proptest.rs` re-reads
   `asserted.tsv` through the N-Triples parser into a fresh store and compares the two, over
   adversarial graphs, which is the strongest thing a test can do here and is not a proof. This is
   the unsafe direction: an extra line is an axiom nobody asserted, and every conclusion resting on
   it is certified against a graph that does not exist.
   *To close it:* nothing pure-functional reaches it. It is a statement about an execution over a
   store, so it needs either a verified store or a checker that can see the store, and neither
   exists.
2. **TCB-10, that `derivations.tsv` covers every triple the run added.** A triple materialised
   without a line covering it sits in the store under the certificate's cover. The argument is that
   the single `emit` closure is the only path that pushes a candidate and records the first
   derivation of anything not already in the closure. That argument is now shorter than it was,
   because `emit` computes the conclusion from the rule table rather than taking one from the call
   site, so a site cannot push a triple the table does not license. It is still a reading of a loop
   plus a property test comparing conclusions against the store's before and after.
   *To close it:* the same obstacle as (1). It is a property of the run, not of a function.
3. **TCB-19 and TCB-21, that the two rule-table parsers agree.** `parse_rules` in Rust and
   `OOCert.HornParse.parseRules` in Lean are independent implementations of one grammar. The
   property test runs against a transcription of the Lean parser, and one deterministic test puts
   the adversarial shapes through `oo-horn check` itself. Measurement, not proof. TCB-18 and TCB-20
   are closed around it, because the engine re-parses what it wrote and refuses on mismatch and
   `pat_of`/`render` are proved inverse, so what is left is exactly the cross-language half.
   *To close it:* generate both parsers from one grammar, or verify the Rust one against the Lean
   one as a refinement. Neither was done here.
4. **TCB-8 under `InferenceTarget::DefaultGraph`.** Merging conclusions into the default graph is
   what the caller asked for and it loses the distinction; a later certified run over that store is
   conditional on a graph that includes inferences. The named-graph path no longer has this problem.
   *To close it:* there is nothing to close in this layer. The caller chooses, and
   `inference_graph: true` is the choice that keeps the certificate honest.
5. **Materialisation is not atomic, and that is latent rather than fixed.** `GraphStore::load_lines`
   runs `for quad in parser { store.insert(&quad?)?; }`, so a parse error partway through a batch
   leaves everything before it inserted and propagates the error. That is what turned the `rdfs7`
   defect from a crash into uncertified triples in the store. `writable_triple` removes the only way
   the reasoner could hand it an unparseable line, so the path is unreachable TODAY; the hazard is
   still there for any future caller.
   *To close it:* insert into a transaction, or build the batch through a serialiser that cannot
   produce an unparseable line rather than through string concatenation.
6. **Everything that is not the certificate boundary at all.** The reasoner's COMPLETENESS: the
   checker proves each recorded step is a sound instance of a rule it knows and says nothing about
   whether the engine found every inference, so a certificate is evidence about what was derived and
   not about what follows. The SHIQ tableaux reasoner: model certificates cover positive
   satisfiability answers only, and unsatisfiability and inconsistency carry no certificate at all.
   The Rust SHACL validator (TCB-28, TCB-29): not certified, measured against a second verified
   evaluator, and the two read RDF with two different parsers whose agreement is by construction on
   both sides rather than by a check. And the SPARQL layer, the MCP server, the CLI, the daemon, the
   file I/O, the roughly fifty thousand lines of `src/`. The Kani harnesses cover five pure
   functions totalling about sixty lines. That is the correct proportion to report: this work
   verified the joint, not the machine.

## What the institution layer does not move

`lean/OOCert/Institution.lean` and the four modules with it prove that two translations between
logics satisfy their satisfaction conditions, and decision 0009 says what that buys. It buys nothing
on this page, and the reason is worth stating because the two things sound alike.

The comorphism proved in `lean/OOCert/InstitutionFol.lean` runs between two LEAN developments: the
RDF interpretations of `OOCert/Semantics.lean` and the first-order structures of
`Fol/Semantics.lean`. It is not `src/tptp.rs`'s translation and says nothing about it. Decision 0005
item 2 is unchanged: the correspondence between the Rust emitter and the Lean translation is pinned
by hand-computed tests and is not proved, and the emitted file still says so.

The layer has no run-time part at all. It writes no file, reads no file, produces no certificate and
no verdict word, and no executable imports it, which is why it is a separate `lean_lib` from
`OOCert`. Nothing in the property list above changes, no `TCB-*` identifier is retired, and the
count of trusted properties is what it was.
The honest summary is that the trusted base used to be four things: the serialisation of a term, the
identity of the asserted graph, the completeness of the derivation record, and the identity of the
rule table. The serialisation of a term is no longer one of them: it is enforced here and proved
over every byte pattern at the bounds below, rather than observed of a dependency. The identity of
the rule table is most of the way off the list: the engine re-parses what it wrote and refuses on
mismatch, and the render-parse round trip is proved; what is left is two independent parsers of one
grammar, which is a different kind of risk from an engine misreporting its own run.

**So it is two things.** That `asserted.tsv` is the graph the engine reasoned over, and that
`derivations.tsv` covers every triple it added. Both are statements about an execution rather than
about a function, which is why no amount of bounded model checking reaches them, and both are
property-tested end to end against the store. The engine is not verified. It was never going to be,
and a report that read as though it were would be the same defect this project exists to attack.
