# `isabelle/` — an independent second formalisation of the Horn certificate checker

This directory is a second, independent formalisation of the OWL 2 RL/RDF Horn
certificate checker and its model theory, in Isabelle/HOL. It exists to test the one
thing a single machine-checked proof cannot test: whether the **definitions** are right.
A proof assistant rules out a bad argument; it says nothing about a bad definition, and a
semantics that is stronger than the specification makes every soundness theorem over it
weaker than it looks while every proof still succeeds.

It was written from the W3C primary sources and the fixture **data** only. Nothing under
`lean/` was read, listed, grepped, or reasoned about while it was written. A
transliteration would have been worthless here, because a shared definitional bug
survives translation untouched.

Every modelling decision is recorded as a comment at the point it is made, with the
specification clause it rests on. That list is what makes a later disagreement
diagnosable instead of merely embarrassing; start with `OO_Semantics.thy`.

## Build

Needs Isabelle2025-2.

    isabelle build -d isabelle -c OOHorn

The session checks everything, including the fixture results and an audit that fails the
build if any soundness or non-vacuity theorem depends on an oracle (`sorry` is the
oracle `Pure.skip_proof`, so this catches it).

## Run the checker

    isabelle/run_checker.sh RULES.tsv ASSERTED.tsv CERT.tsv

Exit 0 accepted, 1 rejected, 2 parse error. One JSON object on stdout carrying **both**
the strict and the lenient verdict. `OO_VERBOSE=1` keeps Poly/ML's chatter.

Regenerate `driver/oo_horn_generated.ML` from the theories with `isabelle/export.sh`.

## Run the checker as a native binary

`run_checker.sh` drives Poly/ML with `--use`, which recompiles 1,957 lines of generated
ML on every invocation — about half a second each, which is the only thing that makes a
corpus of a thousand certificates slow. `build_native.sh` links the same generated code
into an executable instead:

    isabelle/build_native.sh          # -> isabelle/build/oo-horn-isabelle  (gitignored)

    oo-horn-isabelle check RULES.tsv ASSERTED.tsv CERT.tsv   # 0 / 1 / 2, as oo-horn does
    oo-horn-isabelle batch MANIFEST.tsv                      # one JSON line per row
    oo-horn-isabelle table RULES.tsv                         # is this the built-in table?

The CLI shape and the exit codes are `lean/HMain.lean`'s deliberately, so the differential
compares two checkers rather than two translations of two checkers. The wrapper is
`driver/oo_horn_cli.sml` and it is untrusted, exactly as `driver/oo_horn_driver.sml` is.

## Run the differential matrix

    isabelle/differential_matrix.sh

One JSON line per input triple, over the committed fixtures — the by-hand version, useful
for looking at one row. The real comparison is a Rust test:

    cargo test --test cross_kernel_differential_test -- --nocapture

It builds both checkers, generates certificates with the ENGINE over every ontology the
repository ships, mutates and fuzzes them, and runs both kernels over the lot. It skips
loudly (`common::skip_unless`) when either half is missing, and `OO_REQUIRE_FIXTURES=1`
turns that skip into a failure.

**CI runs it, and did not until 15 September 2026.** Until then no workflow installed
Poly/ML, so the file skipped in the only job that ran it and was invoked by no job that
could have made it strict, while `README.md` quoted the result of the comparison on its
front page. What CI installs is not Isabelle: the exported checker is COMMITTED as
`driver/oo_horn_generated.ML`, so the only missing piece is a compiler for it, and
Isabelle publishes Poly/ML on its own as a 39 MB component. That is a `POLYDIR` away from
running anywhere, including here:

    curl -O https://isabelle.in.tum.de/components/polyml-5.9.2-2.tar.gz
    tar xzf polyml-5.9.2-2.tar.gz
    POLYDIR=$PWD/polyml-5.9.2-2/x86_64-linux cargo test --test cross_kernel_differential_test

A full Isabelle2025-2 is still what `export.sh` needs to REGENERATE that ML from the
theories, and what `isabelle build -d isabelle -c OOHorn` needs to check the proofs. CI
does neither, and says so in `docs/ci-gates.md`.

## What the differential found, and what it changed

THE NUMBERS ARE NOT WRITTEN HERE ON PURPOSE. This paragraph carried them once and they went
stale three times in a day: the corpus grew when depth mutations were added, the divergence
count moved with it, and "in two classes" was simply wrong, since every divergent row was the
same one. `cargo test --release --test cross_kernel_differential_test -- --nocapture` prints the
current figures, and it is the only place they should be read from.

What does not drift is the shape. Both kernels run over one corpus of certificates: genuine ones
the engine produced, hand-built probes, systematic mutations and fuzzed inputs. Each row lands in
one of four buckets, accepted by both, rejected by both, refused as unparseable by both, or
divergent, and a divergent row is a finding rather than a nuisance. Nothing has ever been
adjusted on either side to make one go away.

The one divergence this comparison found was real and its cause was in neither proof: the
certificate format never said what a repeated binding key means, so the two kernels read the same
bytes differently and both were defensible. See
[decision 0008](../docs/decisions/0008-a-binding-is-data-and-evidence-admits-one-reading.md).

Those 47 corpus rows were ALL of class D1 below, which is worth stating precisely rather than
rounding to "two classes": D2's three certificates are committed probes with their own tests and
are not members of the generated corpus, so the corpus never counted them. The classification the
corpus test prints says so directly, and D2 is not the weaker finding for it: D2c was found BY the
fuzzer, inside the corpus machinery, before it was committed as a probe.

Both classes had ONE root cause: **Isabelle validates the binding list as a data structure
and Lean did not.** `check_step` requires `distinct (map fst b)` and `binding_covers b r`
before it instantiates anything; Lean's `substOf` is `fun v => (List.lookup v l).getD v`,
which turns any binding list into a total function with a silent default and used to be
applied without the list being inspected at all.

| | certificate | Lean, before | Isabelle |
|---|---|---|---|
| **D1** duplicate binding key | `fixtures-added/bad_dup_key.tsv` | accepted, `entailed` | rejects, `binding_dup_key` |
| **D2a** binding omits a variable whose NAME is a term in the graph | `fixtures-differential/varname_unbound_cert.tsv` | accepted | rejects, `binding_incomplete` |
| **D2b** binding omits a variable the rule's HEAD needs | `fixtures-differential/unsafehead_cert.tsv` | accepted | rejects, `binding_incomplete` |
| **D2c** the same, **found by the fuzzer** in a rule file one character from a probe written for something else | `fixtures-differential/d2c_fuzzfound_*.tsv` | accepted | rejects, `binding_incomplete` |

Neither checker was unsound. Lean's acceptances were all sound in Lean's own theorem, because
`EntailsR` quantifies over every total substitution and the substitution Lean used is a real
one; Isabelle's rejections were false alarms, which is the harmless direction. What the pair
showed is that the FORMAT was underspecified in two places, and that a certificate's validity
therefore depended on which verified checker read it.

D1's resolution rested on `List.lookup` being first-wins. That is a tie-break inside a
standard-library function standing in for a rule about a file format, and it decided
whether a certificate is valid. DECISION M25 in `OO_Check.thy` predicted exactly this and
named `map_of`'s first-wins behaviour as the reason not to rely on it; `substOf`'s docstring
on the Lean side discussed missing bindings only and said nothing about repeated ones.

D2b is the sharp one. The rule is `?s <p> ?o -> ?s <q> ?z`, the graph is one ordinary
triple of IRIs, and the certificate omits `z`. Lean accepted a conclusion whose object is
the bare term `z`, which is not an IRI, not a blank node, not a literal and not writable RDF,
minted out of a variable's NAME in the rule file. The engine refuses to evaluate such a rule
(`parse_rules` in `src/reason.rs` rejects "a head variable the body never binds"), but
that guard is in the PRODUCER, and the checker is the thing you point at a certificate
someone else wrote.

D2c is the one that settles the obvious objection to D2a, which is that nobody writes a
graph whose subject is spelled like a variable. Nobody designed D2c either: the seeded
fuzzer replaced one field of a probe's rule file, `?o` became a bare `?`, and both parsers
read that as a variable whose NAME IS THE EMPTY STRING. The graph's object is an empty
field, so Lean's default sent the unbound variable `""` to the term `""` and the step
checked. The hole was reachable by a typo. This is also why the corpus test works from the
CAUSE of a divergence rather than from the edit that produced it: a quarantine keyed on
the edit would have hidden exactly this row.

D2 also falsified, across the two checkers, something proved inside one of them.
`coverage_implied` says removing Isabelle's coverage check cannot change its accept/reject
bit, and that is true of Isabelle, whose instantiation is partial. It does not transfer:
Lean's instantiation is total, so the same check is load-bearing there and in the opposite
direction. A property proved of one formalisation's checker is not a property of the
format.

### How it was closed, and which side moved

`docs/decisions/0008-a-binding-is-data-and-evidence-admits-one-reading.md` refuses both
shapes, and **the LEAN moved**. `bindingWellFormed` is now a conjunct of `checkHornStep`, so
Lean rejects a repeated key and an incomplete binding exactly where Isabelle already did, and
`OOCert.wellFormed_determines_instantiation` is the Lean-side theorem that a well-formed
binding leaves nothing for a checker's choice of representation to decide.

**Nothing under `isabelle/` was edited to produce that agreement**, and nothing should be.
These theories were written from the W3C sources without reading the Lean, and a formalisation
edited to agree with the one it is checking is worth nothing. If the two disagree again
somewhere else, that is a new finding and it belongs in a report before it belongs in a patch.

The corpus reports ZERO divergent rows, and the 47 that used to diverge moved into the
rejected column and nowhere else. The per-bucket counts that used to be written out here are
not, for the reason given at the top of this section: the corpus grew again the same day and
they were stale before anyone read them. The test prints them, and
`docs/decisions/0008-a-binding-is-data-and-evidence-admits-one-reading.md` carries the pair it
was measured against, dated.

The analysis that used to excuse a disagreement is now a gate that can fail. The rows of the
corpus that carry a binding malformed in one of the two ways, the 47 among them, must be
refused by BOTH kernels, and the corpus test requires it. Most of them were already rejected by
both for some other reason, which is why the gate is checked on every row rather than only on
the ones that disagree: a check that ran only on divergences could be satisfied by silence.

A binding for a variable the cited rule never mentions is still ACCEPTED on both sides
(`probe_extrabind`). It is never consulted, so there was no second reading to remove.

### What agreement on every row is evidence of

The rows that agree went up when decision 0008 landed, and the increase is worth exactly
nothing on its own: it was bought by making one checker refuse more, which is the cheapest way
there is to buy agreement. What the number is worth is fixed by the floor the corpus test
asserts, that at least twenty certificates are ACCEPTED by both.

Checking a Horn certificate is purely syntactic. Neither kernel consults its semantics, so
two checkers built on contradictory model theories agree on every certificate and every
forgery. Agreement here is evidence about the FILE FORMAT and the CHECKING DISCIPLINE, and
it is not a proof of anything. The evidence that the definitions agree lives in
`OO_Builtin_Sound.thy` and `lean/OOCert/HornBuiltin.lean` — which arms each side can
discharge, and from which conditions — and in `OO_NonVacuity.thy`, on whether the
conditions describe anything at all.

Three things the differential did settle, all of them corners neither side had a test for:

* **Non-ASCII IRIs round-trip through both** and a one-byte change to one is rejected by
  both. DECISION M38's worry — that `String.literal` is ASCII-only in the logic and the
  fixtures are all ASCII, so the trap would stay hidden — is a trap that was avoided
  rather than one that was never there. The verified core is over `string = char list`.
* **`"01"^^xsd:integer` is not `"1"^^xsd:integer` on either side.** M1's scope note holds:
  terms are opaque, no datatype-aware rule is in the table, and neither checker
  normalises.
* **Bracket-stripping is invisible to the accept/reject bit.** Isabelle's D-PARSE-2 maps
  `<u>` to `Iri u` and Lean keeps the spelling; `http://ex.org/a` and `<http://ex.org/a>`
  are distinct terms on both sides, which is the injectivity D-PARSE-2 needs.

## Files

| file | what it holds |
|---|---|
| `OO_Format.thy` | terms, triples, rule patterns, rules, steps, instantiation. No semantics. |
| `OO_Semantics.thy` | interpretations, truth of a triple, the 26 semantic conditions, both entailment relations. Never in the export closure. |
| `OO_Check.thy` | the executable checker and the built-in rule table. No semantics. |
| `OO_Sound.thy` | `horn_certificate_sound` — the conditional verdict. |
| `OO_Builtin_Sound.thy` | the 27 arms, and `entails_of_builtin` — the absolute verdict. |
| `OO_NonVacuity.thy` | witness models; entailment is not the trivial relation. |
| `OO_Parse.thy` | TSV field lists to datatypes. Executable. |
| `OO_Pipeline.thy` | what the tool's own output means, end to end. |
| `OO_Fixtures.thy` | the checker run on the real fixture bytes, as build gates. Uses `eval`; no soundness theorem depends on it. |
| `OO_Audit.thy` | the oracle gate, and a check that the gate can fail. |
| `OO_Export.thy` | code generation. |
| `driver/` | the untrusted SML drivers, and the generated core. |
| `fixtures-added/` | fixtures for properties the shipped set cannot reach. |
| `fixtures-differential/` | the two divergences, and the term-format corners neither side had a test for. |
| `build_native.sh` | links the generated core into `build/oo-horn-isabelle`. |

## Trust boundary

Inside: the Isabelle kernel, the theories above, and the code generator — which is
trusted, not verified, exactly as a compiler is on any other side of any comparison.

Outside: file IO, splitting bytes on LF and TAB, exit codes, JSON, and the claim that
the engine's `asserted.tsv` is the store it says it is.
