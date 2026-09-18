# 0014. A verifier that cannot read the code verifies a rewrite

Dafny is DECLINED as a dependency. `dafny/RuleTable.dfy` is KEPT, because it earned its place by
finding something, and because a declined tool with nothing to show for the decision is the kind of
entry `docs/reasoning-systems-inventory.md` exists to prevent.

The rule: a proof about a reimplementation of a function is a proof about the reimplementation. It
becomes a proof about the engine only when the engine runs the verified artefact, and the price of
making the engine run this one is measured below and is worse than the disease.

## The defect this prevents

Counting a green verifier as evidence about code it never read.

This is decision 0005's rule applied to a verification tool rather than to a prover. A prover's
`unsat` is testimony about the problem it was given, and the question that matters is whether that
problem is the one we asked. A verified rewrite is testimony about the rewrite, and the question that
matters is whether the rewrite is what ships. For Kani and for Aeneas the answer is yes, by
construction: Kani model-checks the actual `src/reason.rs`, and Aeneas translates the actual
`src/boundary_core.rs`. For Dafny the answer is no unless the generated Rust replaces the
hand-written Rust, and that is a much larger decision than "add a verifier".

## What was actually built and run

Dafny 4.11.0, installed with `brew install dafny` on `arm64-apple-darwin`, using the Z3 it bundles.
Homebrew also pulled Z3 4.16.0 as a formula dependency; nothing here invokes it directly.

`dafny/RuleTable.dfy` is 676 lines modelling the rule-table grammar of `src/reason.rs` over bytes:
`Pat`, `AtomPat`, `RulePattern`, `pat_of`, `Pat::render`, `rules_tsv` for one line, and the part of
`parse_rules` that decides which rule a line denotes. `dafny/run.sh` runs it.

| measurement | value |
|---|---|
| proof obligations | 57 verified, 0 errors |
| `dafny verify` | 7.1s wall, 3.9s user |
| `dafny/run.sh`, verification plus three falsifying mutations | 16.3s wall |
| `dafny build -t:rs` | 7.9s wall, exit 0 |
| Rust generated for the model | 872 lines |
| `dafny_runtime` vendored beside it | 8,590 lines, 9,914 with its own tests |
| third-party crates that runtime pulls | `num`, `once_cell`, `itertools` |
| Rust in `src/reason.rs` the model covers | about 127 lines |

The runtime row and the covered-lines row are the decision. Shipping the verified artefact means
adding roughly 8,600 lines of unverified Rust runtime, plus a bignum library, to the trusted
computing base, in order to verify about 127 lines that are already inside it.

## What it proved that nothing here proved before

TCB-20 is `parse_rules(rules_tsv(r)) == r`. It is addressed today by Kani and by Aeneas, and it is
established by neither.

`kani_harnesses::pat_of_and_render_are_inverse_at_2` and `_at_3` prove the PER-FIELD half over every
byte pattern at a field length of exactly two and exactly three bytes. The length is fixed rather
than symbolic because, as `docs/trusted-computing-base.md` records, a symbolic length made CBMC
reach fifteen gigabytes without a verdict. The whole-line statement was out of reach for a second
reason recorded on the same page: asserting over `body.split('\t')` put CBMC inside `CharSearcher`,
which is both expensive and the wrong function to be measuring.

Aeneas removed the length bound from every other serialisation property on that page and did not
reach this one. `docs/aeneas-boundary.md` says why under "What Aeneas could not handle": `Pat`
carries a `String`, and Aeneas "has a `String` model but not enough of one".

`RuleTable.dfy` proves the whole statement with no bound on field length, on field count, or on the
size of the rule body, and it does so in seven seconds. The split-and-join lemma that CBMC could not
afford is one induction here. That is a real capability difference and it is not a small one.

## And what it found, which is a documentation defect rather than a bug

TCB-20 as written on `docs/trusted-computing-base.md` is FALSE.

`parse_rules(rules_tsv(r)) == r` does not hold for an arbitrary `r: RulePattern`. Nothing in the Rust
type stops `r.name`, or the name inside a `Pat::Var`, from containing a tab or a line feed. A rule
carrying one renders to a line with more fields than it has positions, and that line reads back as a
different rule or as an error. Neither outcome is the identity.

Dafny surfaced this the way a verifier does, by refusing to prove a false thing. The theorem needed a
hypothesis, `RuleIsWritable`, and writing that hypothesis is what made the gap visible.
`ATabInANameBreaksTheRoundTrip` and `ATabInAVariableBreaksTheRoundTrip` are the two witnesses, proved
rather than asserted, and `dafny/run.sh` removes the hypothesis on every run and requires the
round-trip theorem to fail without it. A precondition nobody has tried to drop is a precondition
nobody knows is load-bearing.

**The hazard is latent and it is not live, and saying so is part of the finding.** Only two places
construct a `RulePattern`, and both are closed:

- `src/reason.rs:2740`, inside `parse_rules` itself. Its fields come from splitting a line on tabs,
  so none of them can contain a tab, and the line came from splitting on newlines.
- `src/rulesyntax.rs:179`, in `Import::accept`, which is the front end a user's RIF, SWRL or Datalog
  reaches. It already renders the rule to a `rules.tsv` line, re-parses it with `parse_rules`, and
  refuses the rule unless it comes back identical. That is exactly this theorem's hypothesis,
  enforced at the only site where it could otherwise be violated.

So the correction is to the page, not to the engine. TCB-20 should state the condition and name what
enforces it, and `docs/trusted-computing-base.md` now does. The engine was already right; the
document claimed something stronger than the engine delivers, which by this repository's standards is
its own category of defect.

## Why Dafny is still declined

### The theorem is about the Dafny

This is the whole of it. `RuleTable.dfy` is a second implementation of the grammar, written from
`src/reason.rs` and from `lean/OOCert/HornParse.lean`. Every theorem in it is a theorem about
`RuleTable.PatOf` and `RuleTable.ParseRuleLine`. If the model and the Rust disagree, the theorem is
true of a function the engine does not run, and nothing in the file would notice.

That is the same shape as TCB-30, which records the same exposure for Aeneas, but it is strictly
worse in one respect and the difference decides this. Aeneas's model is DERIVED from the shipped
Rust by a program, and `aeneas/run.sh` re-derives it and fails if it moved, so the model cannot drift
away from the code unnoticed. The Dafny model is derived from the Rust by a person reading it, and
nothing can check that correspondence. A hand transcription that drifts is precisely the failure
`tests/certificate_boundary_proptest.rs` already documents for the TCB-19 leg, where the property
test runs against a transcription of the Lean parser and the transcription's fidelity is an
assumption.

### Closing that gap means shipping the generated Rust, and the trade inverts

Dafny's answer to "your theorem is about the wrong artefact" is to compile the Dafny and run that.
The backend works: `dafny build -t:rs` produced a compiling crate in 7.9 seconds. What it produced is
the problem.

```
pub fn PatOf(f: &Sequence<u8>) -> Rc<Option<Rc<Pat>>>
```

That is not a function `src/reason.rs` can call. `Sequence<u8>` is `dafny_runtime`'s persistent
sequence and not `&str` or `&[u8]`; the result is `Rc<Option<Rc<Pat>>>` and not `Option<Pat>`; and
`f.cardinality()` returns a `DafnyInt`, so every length test and every index in the generated code is
arbitrary-precision arithmetic through `num::BigInt` rather than a `usize`. Calling it needs a
marshalling layer converting `&str` to `Sequence<u8>` on the way in and `Rc<Option<Rc<Pat>>>` to
`Option<Pat>` on the way out, with an allocation and a copy in each direction.

That marshalling layer would be hand-written, unverified, and would sit exactly at the trust boundary
this exercise exists to shrink. The verified parser would be reached only through unverified code
converting into and out of it, which is the same position the parser is in now, one level out.

And the runtime it rests on is 8,590 lines of Rust nobody here has read, with `num`, `once_cell` and
`itertools` behind it. `docs/trusted-computing-base.md` names `oxrdf`'s escaping behaviour as a trust
assumption worth closing. Adding a bignum library to the trusted base of a tab-separated-value parser
is not a trade this document can recommend with a straight face.

Two mitigations exist and neither rescues it. `--features small-int` makes `DafnyInt` an `i128`
instead of a `BigInt`, which shrinks the arithmetic but not the runtime and not the marshalling. And
the model could be narrowed to `seq<uint8>` end to end, which is what it already does; the
`Sequence<u8>` representation is the runtime's, not the model's, and there is no flag that makes it
a `&[u8]`.

### The pure functions worth verifying are the ones already covered

The inventory question was whether Dafny reaches something Kani and Aeneas do not. It does, and the
set is small. Of the thirty properties on the TCB page, Kani proves six at a bound and Aeneas removes
the bound from the serialisation core. What is left unreached and pure is TCB-20's whole-line half,
for which this file supplies an unbounded proof ABOUT A MODEL and no proof about the Rust, and the
interner's TCB-15 to TCB-17, which Aeneas cannot reach because `Interner` is a `HashMap` and which
Dafny would model as a `map` almost for free.

That second one is worth naming precisely so nobody has to rediscover it: an interner's bijectivity
is nearly trivial in Dafny and genuinely out of reach for both of the other two. It is also the same
trade. Verifying a Dafny interner and then shipping `Sequence<u8>` keys through `dafny_runtime`'s
`Map` in place of `HashMap<String, u32>` is a performance and trust decision on the engine's hottest
data structure, taken to close a property that three property tests already sample and that has never
failed.

### The defects that actually bite this codebase are not pure functions

This is the argument that would still hold even if the interop were free.

The verification work already done here covers, in its own words, "five pure functions totalling
about sixty lines", and the honest summary on the TCB page is "this work verified the joint, not the
machine". The defects found in this repository over the last month were not in that joint. The
`rdfs7` defect was a rule concluding a triple no serialiser can write, found by a property test
driving the real entry points. The Lean and Isabelle divergence was a gap in the FORMAT that no
single formalisation could see. The skipped-gate defect was a test that reported `ok` because no
workflow installed its toolchain. A separate review of the daemon paths found that in `serve-http`
and daemon mode the ontology evictor can never evict anything, because the registry it consults is
never populated.

Not one of those is a false theorem about a pure function, and not one of them is reachable by
Dafny without rewriting the surrounding program in Dafny. They are wiring, configuration, format
underspecification and state that is never initialised. Dafny bites on the part of this system that
is already the best covered.

The last of those is worth one more sentence, because it is the clearest case. `evictor_tick` in
`src/registry.rs` returns early unless the registry holds an active entry, and the only thing that
sets one is `OntologyRegistry::load_file`, while `src/server.rs` loads through `GraphStore::load_file`
instead, so in `serve-http` and daemon mode the evictor can never evict anything.
`tests/registry_evictor_wiring_test.rs` pins it, and pins it as a defect rather than as behaviour:
`an_unloaded_registry_never_evicts` fails the day somebody fixes the wiring, and says so.
Nothing about that is a false theorem. Every function involved does what it says. The defect is which
function the other one calls, and there is no property of a pure function whose proof would have
caught it. Decision 0012 reached the same conclusion from the concurrency side on the same day, by a
different route.

## What is kept, and on what terms

`dafny/RuleTable.dfy` and `dafny/run.sh` stay in the tree as a SPECIFICATION and as the evidence for
this record. They are not a gate and must never be described as one.

- Nothing in `make check`, `cargo build`, `cargo test` or `lean/`'s `lake build` reaches them. This
  is the arrangement Kani and Aeneas are already under, for the reason `docs/ci-gates.md` gives: a
  gate nobody can run locally is not a gate. `make verify-dafny` runs it for anyone who has Dafny.
- `docs/ci-gates.md` carries a row for it under "What is still open", so the page cannot imply it
  runs.
- No sentence anywhere may say that TCB-20 is verified. What is verified is that a Dafny model of the
  grammar has the property, and that is what `docs/trusted-computing-base.md` now says, in those
  words, in the same row that reports the Kani bound.

`run.sh` proves it can fail rather than asserting it. It mutates the specification three times and
requires every mutation to be rejected: dropping the tab-freeness hypothesis, letting `pat_of` read a
bare `?` as a constant, and dropping the empty-name refusal. Measured on this tree, those produce two,
two and one error respectively. The script also fails if a mutation VERIFIES, which was tested by
feeding it a mutation that changes nothing and confirming it reports `FAIL` and exits 1, and it fails
if a mutation's pattern is no longer in the file, which was also tested. A verification script whose
only possible outcome is success is the same defect as a test that skips.

## Consequences

1. Dafny joins `docs/reasoning-systems-inventory.md` as declined, with the measured reason rather
   than the general one. The general objection in that table, "each needs the code rewritten in its
   subset", is true of Dafny in a stronger form than of Verus, Creusot and Prusti: those three verify
   Rust written in a dialect, and Dafny does not verify Rust at all. The measurement that matters is
   not that the rewrite is required but what the rewrite costs to run, and that is the 8,590-line
   row in the table above.
2. TCB-20 on `docs/trusted-computing-base.md` gains its missing precondition and a note naming the
   two sites that enforce it. The page previously stated a property that is false as written.
3. `docs/aeneas-boundary.md`'s entry for `parse_pat` under "What Aeneas could not handle" is stale in
   one respect and is corrected: it says the fix would be "to lift the classification into a pure
   `Option<Pat>` function", and that was done on 15 September 2026. `pat_of` exists and Kani verifies
   it. Aeneas still cannot reach it, for the unrelated reason that `Pat` carries a `String`.
4. If the interner is ever verified, this record is the place to start, and the question to answer
   first is not "can Dafny prove it" but "what replaces `HashMap<String, u32>` at run time".
5. Nothing about this decision is permanent. What would change it is a Dafny Rust backend that emits
   ordinary Rust types against no runtime, which does not exist today, or a decision that this
   project's engine should be generated rather than written, which is a different project.
