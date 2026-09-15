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

## What the Lean side never sees

The checkers are pure functions of files on disk. They read `asserted.tsv`, `derivations.tsv`,
`rules.tsv`, `horn.tsv`, `axioms.tsv` and `model.tsv`, and nothing else. In particular they cannot
see:

- the RDF store, so they cannot tell whether `asserted.tsv` is the store's contents;
- the reasoner's execution, so they cannot tell whether a step in `derivations.tsv` was ever taken;
- the store after the run, so they cannot tell whether a triple was materialised without a
  certificate line covering it;
- the rule file the user passed, so they cannot tell whether `rules.tsv` is the table the engine
  evaluated;
- **which graphs the run selected**, so they cannot tell whether `asserted.tsv` is the graph anyone
  meant. This one is different in kind from the other four, because there is no way to fail it:
  every one of the others can make a checker REJECT a legitimate run, and this one cannot. A
  certificate over the union of every version of a bi-temporal register is a true statement, valid
  under `OOCert.certificate_sound`, about a state of the world that never existed. That is issue
  #108, and `scope.tsv` (below) is the mitigation.

Each of those five blind spots is a property below.

## The emission paths

| path | entry point | files written | checker |
|---|---|---|---|
| built-in forward chaining | `Reasoner::run_full` (`src/reason.rs`) | `asserted.tsv`, `derivations.tsv`, `scope.tsv` | `oo-cert` |
| supplied Horn rules | `Reasoner::run_horn` (`src/reason.rs`) | `rules.tsv`, `asserted.tsv`, `horn.tsv`, `scope.tsv` | `oo-horn` |
| DL model certificate | `DlReasoner::write_model_certificate` (`src/tableaux.rs`) | `axioms.tsv`, `model.tsv` | `oo-dlmodel` |
| SHACL | no Rust emission | N-Triples handed to the Lean evaluator | `oo-shacl` |

The SHACL row is different in kind and is treated separately at the end.

## The properties

Each property has an identifier, a statement, the code it is about, and how it is checked. `TCB-*`
identifiers are used by `tests/certificate_boundary_proptest.rs`, which names them in its test
functions.

### Serialisation: the format cannot be forged from inside a term

All six files are tab-separated, carry arbitrary RDF terms, and have no escaping layer of their own.
`src/reason.rs` writes terms with `push('\t')` between them and `push('\n')` at the end of a line,
and nothing in that code inspects a term. `lean/OOCert/Parse.lean` states the assumption that makes
this safe, in a comment rather than a theorem:

> Tabs and newlines cannot occur inside an N-Triples term (they are escaped), so splitting on them is
> exact.

That sentence is the whole of the escaping argument, and it is a claim about a third-party crate.
The terms come from `oxrdf`'s `Display`, through `GraphStore::all_triples`, which calls
`quad.subject.to_string()` and friends. `oxrdf 0.3.3` escapes `\t`, `\n`, `\r`, `"` and `\` inside a
literal's lexical form (`print_quoted_str` in `literal.rs`), and does **not** escape anything inside
an IRI: `NamedNodeRef`'s `Display` is `write!(f, "<{}>", self.as_str())`. Nothing in this repository
re-checks either half.

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
  `\n` or `\r`. This is the property TCB-1 to TCB-3 exist to detect the failure of; stated
  separately because it is the one that has to survive an `oxrdf` upgrade.
- **TCB-5 (term spellings do not collide).** A literal whose lexical form is `<http://example/x>`
  and the IRI `<http://example/x>` must not have the same spelling in a certificate file. They do
  not, because a literal is quoted, but the two checkers compare terms as opaque strings and would
  be unable to tell them apart if they ever agreed.

### The asserted graph

- **TCB-6 (asserted is the graph, no drops).** Every triple in the SCOPE at the start of a certified
  run appears as a line in `asserted.tsv`. A dropped assertion makes the checker reject a legitimate
  step, so this direction is fail-safe, but it is still part of the claim the file makes.
- **TCB-7 (asserted is the graph, no additions).** Every line of `asserted.tsv` corresponds to a
  triple that was in the SCOPE at the start of the run. This is the unsafe direction. An extra line
  is an axiom nobody asserted, and every conclusion resting on it is certified against a graph that
  does not exist.
- **TCB-6a (the scope is recorded).** "The scope" in TCB-6 and TCB-7 means the graphs named by
  `scope.tsv`, written into the certificate directory by the same block that writes
  `asserted.tsv`. Until #108 the scope was always the whole store and was written down nowhere, so
  TCB-6 and TCB-7 quantified over a set the reader had to assume. They now quantify over a set the
  reader can see. `scope.tsv` is `oo-scope/1`, one `key TAB value` line per fact, with one `graph`
  line per named graph read (or `graph\t*` for the whole store) and one `excluded` line per graph
  deliberately not read. **No Lean checker reads it**, and calling it a certificate would be the
  overclaim this page exists to prevent: it is a record, checkable by a person or a script against
  the store, and its value is that the selection stops being invisible.
- **TCB-8 (no inference leaks into the assertions, within a run).** No conclusion of
  `derivations.tsv` appears in `asserted.tsv`. The engine reaches a fixpoint in one pass and
  captures `facts` before materialising, so this holds within a run.
  **Across runs it does not hold for an unscoped run and cannot be made to hold by this layer.**
  Materialising into the default graph turns run N's conclusions into run N+1's assertions, and
  `asserted.tsv` has no column that says "derived". `docs/lean-certificates.md` states this under
  "Known limitations" and decision 0001 is the mitigation (`inference_graph: true` keeps them in a
  named graph). Note that `GraphStore::all_triples` iterates `store.iter()` over every graph, so
  inferences parked in `https://open-ontologies.org/graph/inferred` by an earlier run are read back
  as assertions by a later UNSCOPED certified run. The separation protects `save`, not the
  certificate.
  A run over a VERSIONED store is the exception, on both halves. A scoped one reads through
  `GraphStore::triples_in_scope`, which reads the graphs the scope names and no others, drops the
  inference graph from that set and records the drop in `scope.tsv`. And no run over such a store
  materialises at all, scoped or `all_versions`, so none of them creates a next-run assertion:
  every graph a run could write to is in scope at every instant, which makes the write a poisoning
  of every future snapshot rather than only of the next run.
- **TCB-8a (the scope is the one that was asked for).** The graphs listed in `scope.tsv` are the
  graphs `triples_in_scope` read, and for a snapshot they are exactly the `in_scope` set
  `Temporal::snapshot` reports at the same instant, less the inference graph. This is a reading of
  `Temporal::read_scope`, which calls the same `scope()` the snapshot tool calls, plus
  `tests/temporal_scope_test.rs`, which pins the graph lists at two instants. Nothing derives one
  from the other.
- **TCB-9 (the store is a set, the file is a list).** A triple present in two named graphs is
  written twice, because `all_triples` flattens quads — and so does `triples_in_scope`, where both
  of those graphs are in the scope. Duplicate lines are harmless to soundness
  (the Lean side builds a `HashSet`) but `asserted` in the JSON report counts lines, not distinct
  triples. The Horn path reports both numbers; the built-in path reports only the line count.

### The derivation steps

- **TCB-10 (every inference is certified).** The set of conclusions in `derivations.tsv` equals the
  set of triples the run added to the store. A triple materialised without a line covering it is an
  uncertified triple sitting in the store under the certificate's cover. Structurally this holds
  because the single `emit` closure is the only path that pushes a candidate, and it records the
  first derivation of anything not already in the closure, but nothing outside that reading enforces
  it.
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
  premises positionally per rule. The emitter passes them in a hand-written order at each of the
  thirty-one `emit` call sites. A wrong order is a rejection, not a false pass, and
  `tests/lean_certificate_test.rs` covers every rule, but nothing derives the order from a single
  source.

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
- **TCB-26 (the guard covers every name that is written).** The `names` vector is built by walking
  every axiom variant, every role in `model.rext` and every class in `model.cext`. Individuals in
  `model.ind` are covered only because `DlAxiom::Indiv(i)` is emitted for every individual that
  reaches the model. That is an argument about two separate loops agreeing, not a check.
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

| property | checked by | how |
|---|---|---|
| TCB-1 | `tests/certificate_boundary_proptest.rs`, `src/reason.rs` | property, end to end and on the writer |
| TCB-2, TCB-3 | same | property |
| TCB-4 | `tests/certificate_boundary_proptest.rs` | property, plus two deterministic tests: the parser refuses a separator inside an IRI, and Turtle's long-string form (which carries RAW control characters) comes back escaped |
| TCB-5 | proptest | property |
| TCB-6, TCB-7 | proptest | `asserted.tsv` is re-read through the N-Triples parser into a fresh store and compared |
| TCB-8 | proptest | property within a run; the across-run failure is pinned by a test that asserts the DEFECT, so the day it changes the documentation is forced to change |
| TCB-9 | deterministic test | a quad in two named graphs is two lines |
| TCB-10, TCB-11 | proptest | conclusions compared against the store's before and after |
| TCB-12 | proptest | both the built-in and the Horn path |
| TCB-13 | proptest | the name list is read out of `lean/OOCert/Rules.lean` at test time |
| **TCB-14** | `tests/lean_certificate_test.rs` | the real checker, over every rule. **Not** re-derived here |
| TCB-15, TCB-16, TCB-17 | `src/reason.rs` unit property tests | the interner is private |
| TCB-18 | in the code, and proptest | the engine re-parses what it wrote and refuses on mismatch |
| TCB-19, TCB-21 | proptest, plus the REAL checker | property against a transcription of `OOCert.HornParse` in the test file, because a Lean process per case is not a property test. The transcription's fidelity is then an assumption, so one deterministic test puts the adversarial shapes (a numeric rule name, a non-ASCII one, a `??x` variable, a literal spelled exactly like an IRI as a constant, a blank node, an empty-bodied rule) through `oo-horn check` itself and requires the conditional verdict |
| TCB-20, TCB-22 | proptest | render and parse are inverse, and no rendered line is empty |
| TCB-23, TCB-24 | proptest | the substitution is re-applied from `rules.tsv` independently of the engine |
| TCB-25 | `src/tableaux.rs` unit property tests | `name_is_safe`, `concept_string` token counts, `axiom_line` field counts |
| **TCB-26** | nothing | the argument that `names` covers `model.ind` is two loops agreeing |
| TCB-27 | in the code | `tests/dl_model_certificate_test.rs` exercises it; not property-tested |
| **TCB-28, TCB-29** | nothing here | the SHACL boundary is different in kind, see below |

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
`reason_horn_emit_test`, which were in the same position. `docs/ci-gates.md` is the table of which
gate runs where, and Kani is still in no job at all, which is why the section below reports its
results rather than pointing at a build.

### Bounded model checking

Kani 0.67.0 installed and ran here, on `nightly-2025-11-21-aarch64-apple-darwin`, over the whole
crate. Three harnesses in `src/reason.rs` prove the serialisation properties over every byte pattern
at a fixed shape, rather than over the patterns a generator drew. A fourth was written and does not
terminate; it is reported below rather than quietly dropped. `make verify` runs the three. None are
part of `make check`, because Kani pulls its own toolchain and the same statements are sampled by
`cargo test` for anyone without it.

| harness | property | result |
|---|---|---|
| `asserted_line_round_trips` | TCB-1 | SUCCESSFUL, 1476 checks, 0 failed, 12.8s |
| `triple_fields_append_exactly_three` | TCB-2, TCB-3 | SUCCESSFUL, 1485 checks, 0 failed, 32.5s |
| `writable_triple_decides_both_positions` | the guard that fixed the defect below | SUCCESSFUL, 198 checks, 0 failed, 1.6s |
| `parse_pat_and_render_are_inverse` | TCB-20 | **NO VERDICT.** Abandoned at 14 minutes and 9.5 GB |

**The one that failed to run.** `parse_pat` returns `anyhow::Result` and every refusal formats a
message naming the line and the position, so the function body carries the whole formatting
machinery. CBMC flattens a function before it solves, so an `assume` that makes the refusal paths
infeasible is a constraint for the solver and not a cut in the program: constraining the input made
it worse, not better. Measured, with nothing else competing: four unconstrained ASCII bytes, no
verdict at 7 minutes and 3.0 GB; two bytes, no verdict at 8 minutes and 2.8 GB; two bytes with the
first constrained to the three spellings the function accepts, no verdict at 14 minutes and 9.5 GB.
It is excluded from `make verify`, because a target that hangs is worse than one that is honest
about its coverage, and the property is sampled instead (1024 cases on the function, plus whole
rule tables in the integration suite). What would close it is lifting the classification out of
`parse_pat` into a pure `Option<Pat>` function, leaving the messages where they are. That is a
refactor of shipped code and it was not made here.

**What the bounds are, and they are real.** In the three that verify, terms are a FIXED three bytes,
each byte unconstrained ASCII. The length is fixed rather than symbolic because a symbolic length
makes every offset in the line symbolic and every slice bound a case split: at two bytes per term
with a symbolic length, CBMC reached fifteen gigabytes without a verdict. So the harnesses prove
every byte pattern at one shape, and the property tests sample the shapes, including the empty
string. Neither alone is the whole claim, and anyone quoting "verified" off this table without the
bound is misquoting it.

Two other costs were paid down rather than hidden, because both are the kind of thing that makes a
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

The pattern in all three is the same, and it is the thing to take away from this section: twice, the
first formulation of a harness measured a function nobody asked about. A verification effort that
does not check what it is actually measuring produces a green result about the wrong thing, which is
the failure mode this whole layer exists to attack.

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

This is the part that matters, and it is longer than the part that is checked. Nothing below is
verified. Each entry says what would close it.

1. **`oxrdf`'s escaping is the entire escaping layer.** `asserted.tsv` is safe because
   `print_quoted_str` turns a tab inside a literal into `\t`. This repository has no escaper of its
   own and never inspects a term. TCB-4 pins the behaviour a test can observe, which means an
   upgrade that changed it would fail loudly; it does not mean it cannot change, and it does not
   cover a term that reaches the store by a route the tests do not exercise.
   *To close it:* escape at the certificate writer rather than relying on the term's `Display`, or
   change the format to one that cannot be forged by its payload (length-prefixed, or JSON Lines).
   Both are changes to `lean/`'s parsers as well, which is why neither was done here.
2. **`oxiri`'s IRI validation is why a separator cannot reach an IRI.** `NamedNodeRef`'s `Display`
   is `write!(f, "<{}>", self.as_str())` with no escaping at all, so the only thing standing between
   a tab and `asserted.tsv` is that the parser refused to build the IRI. Tens of thousands of lines
   of parsing sit behind that sentence and none of it is verified here.
   *To close it:* check the term at the writer, as in (1). The check is four lines; the reason it is
   not there is that it would be a second place the question is decided.
3. **TCB-14, the premise ORDER per rule.** `OOCert.checkStep` matches premises positionally, arm by
   arm, and the emitter passes them in a hand-written order at thirty-one call sites. The two agree
   because `tests/lean_certificate_test.rs` walks every rule and the checker accepts. Nothing
   derives one from the other, so a rule added with the wrong order fails at the checker rather than
   at compile time. *To close it:* generate both sides from one table, which is what the Horn path
   already does and the built-in path does not.
4. **TCB-26, the DL name guard's coverage.** `name_is_safe` is applied to a `names` vector built by
   one loop; the files are written by another. Individuals in `model.ind` are covered only because
   `DlAxiom::Indiv(i)` happens to be emitted for every individual that reaches the model.
   *To close it:* check at the point of writing, not in a separate pass.
5. **TCB-8 across runs, on the UNSCOPED path.** Materialising into the default graph turns an
   inference into the next run's assertion, and `asserted.tsv` has no column that says "derived".
   `inference_graph: true` does not fix it either: `GraphStore::all_triples` reads every named
   graph, so a later certified run sees them as assertions. The separation protects `save`; it does
   not protect the certificate. *Closed for a scoped run* (#108): `triples_in_scope` reads the
   graphs the scope names, the inference graph is not one of them, the drop is recorded in
   `scope.tsv`, and a scoped run materialises nothing. *Still open for an unscoped run,* which is
   every run over a store that does not use the temporal vocabulary — that is, most of them. *To
   close it there:* a scope argument that names the asserted graphs on a store with no temporal
   metadata, which is `ReadScope::Graphs` with no selector in front of it and is not wired to any
   tool today.
6. **Materialisation is not atomic, and that is now latent rather than fixed.** `GraphStore::
   load_lines` runs `for quad in parser { store.insert(&quad?)?; }`, so a parse error partway
   through a batch leaves everything before it inserted and propagates the error. That is what
   turned the `rdfs7` defect from a crash into uncertified triples in the store. `writable_triple`
   removes the only way the reasoner could hand it an unparseable line, so the path is unreachable
   TODAY; the hazard is still there for any future caller.
   *To close it:* insert into a transaction, or build the batch through a serialiser that cannot
   produce an unparseable line rather than through string concatenation.
7. **The reasoner's completeness.** The checker proves each recorded step is a sound instance of a
   rule it knows. It says nothing about whether the engine found every inference, so a certificate
   is evidence about what was derived and not about what follows.
8. **The SHIQ tableaux reasoner.** Model certificates cover positive satisfiability answers only.
   Unsatisfiability and inconsistency carry no certificate at all, and those are the answers a
   consistency check is usually asked for.
9. **The Rust SHACL validator (TCB-28, TCB-29).** Not certified. `oo-shacl` is a second, verified
   evaluator that the Rust one is measured against; a measurement is not a proof, and the two read
   RDF with two different parsers whose agreement is by construction on both sides rather than by a
   check.
10. **Everything between the store and the certificate that is not a term:** the SPARQL layer, the
   MCP server, the CLI, the daemon, the file I/O, the roughly fifty thousand lines of `src/`. The
   Kani harnesses cover four pure functions totalling about twenty lines. That is the correct
   proportion to report: this work verified the joint, not the machine.
11. **The scope of a run is recorded, not checked.** `scope.tsv` says which graphs `asserted.tsv`
   was built from. No Lean checker reads it; nothing compares it against the store; a caller who
   ignores it is exactly as exposed as before #108. What changed is that the selection is now a
   value the engine computed and wrote down, rather than a consequence of which method the code
   happened to call, so a disagreement between the scope and the certificate is a thing a reviewer
   CAN find. *To close it:* a checker that takes the store and `scope.tsv` and re-derives
   `asserted.tsv`, which is a verified RDF store away and not a Lean file away.
12. **A blank-node graph name makes a scoped run error rather than answer.** `Temporal`'s graph
   scan binds `?g` inside `GRAPH ?g { … }`, which in oxigraph can bind a blank node, and both
   consumers then build an IRI from it: the scoped reader returns `_:g1 is not an IRI: Invalid IRI
   code point ':'` and `query_at` would splice `FROM NAMED <_:g1>` into a query that does not
   parse. Measured on 15 September 2026 with `reason --valid-at` over a TriG file using `_:g1 { … }`.
   This fails in the safe direction and is pre-existing rather than introduced by #108 — the
   temporal query layer had it already — but a store loaded from TriG or N-Quads with unnamed
   graphs cannot be scoped at all. *To close it:* carry graph names as terms rather than as
   strings through `Temporal::all_graphs`, `ReadScope` and `query_at`.
13. **The default graph enters a snapshot wholesale.** A scoped run reads the in-scope named graphs
   plus the whole default graph, because that is where the schema lives and a snapshot of the ABox
   with no TBox answers a question nobody asked. The default graph also holds the validity metadata
   itself, so `temporal:validFrom` and its siblings are in the reasoner's closure and are counted as
   triples of their graph IRIs by SHACL. On the shapes people write this is inert — the subjects are
   graph IRIs, and the predicates carry no declared domain or range — and it is not inert in
   principle: a shape targeting a class that a graph IRI belongs to, or an `rdfs:domain` asserted on
   a temporal predicate, would see them. It is visible in `asserted.tsv` and pinned by
   `tests/temporal_scope_test.rs`. *To close it:* an explicitly named schema graph, which decides
   for the user where their schema lives, or a predicate-namespace filter, which decides that the
   engine's vocabulary can never be domain data. Neither was worth taking on the user's behalf.

The honest summary is that the trusted base is five things: the serialisation of a term, the
identity of the asserted graph, the SELECTION of the asserted graph, the completeness of the
derivation record, and the identity of the rule table. Those are now property-tested and, for the pure parts, bounded-model-checked. Every one
of them still rests on a dependency's behaviour that this repository observes rather than enforces.
The engine is not verified. It was never going to be, and a report that read as though it were
would be the same defect this project exists to attack.
