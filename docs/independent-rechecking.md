# Re-checking the Lean proofs outside Lean

`isabelle/` bought this project one kind of independence and it is worth being precise about which
kind, because the obvious next sentence is wrong.

Isabelle/HOL is an INDEPENDENT SECOND FORMALISATION. It was written from the W3C primary sources
with `lean/` deliberately unread, and it found a real defect that a single formalisation could not
have found: the certificate format never said what a repeated binding key means, Lean silently
totalised the binding list and Isabelle refused, and 47 certificates out of 1,718 came apart on
exactly that. [docs/reasoning-systems-inventory.md](reasoning-systems-inventory.md) has the
account. What that bought is **independence of the SPEC READING**. Two people read the same
standard, wrote two definitions, and the definitions disagreed where the standard was silent.

It bought nothing at all about the KERNEL. Every theorem in `lean/` is still checked by exactly
one program, the Lean 4 kernel at the version `lean/lean-toolchain` pins, and if that program
accepts a term it should not, no theorem in this repository would notice, because every theorem in
this repository is stated in Lean and checked by that kernel. The Isabelle side has the same
property about Isabelle's kernel. Two independent formalisations checked by two kernels is better
than one, but the two halves prove different theorems about different definitions, so neither
re-checks the other's proofs. Nothing in this repository's build re-checks a Lean proof, and until
the work recorded below nothing ever had.

This page is the investigation of what it would take to change that. It is an investigation and a
recommendation, not an integration: nothing here is a dependency of this repository, and nothing
here was added to the build.

## What was run, and what is quoted

Claims below are marked. **RAN** means it was executed on this machine against this repository's
own Lean on 15 September 2026, with the outcome as stated including the failures. **READ** means
it comes from the tool's source, its repository metadata or its documentation, and was not
executed here. The distinction is the point of the page and it is kept in every section.

Environment for every RAN result: macOS on arm64, Lean toolchain `leanprover/lean4:v4.33.1` as
pinned by `lean/lean-toolchain`, `lake build` in `lean/` green at 105 jobs with no warnings.

## Route 1: lean4export plus an external checker

This is the route that works.

Lean 4 has no built-in export flag. `lean --help` at v4.33.1 offers `-o`, `--deps`, `--stats` and
much else, and nothing that writes a checkable declaration dump. The export is a separate tool,
[`leanprover/lean4export`](https://github.com/leanprover/lean4export), which walks an imported
environment and writes every non-internal constant as NDJSON.

**RAN.** `lean4export` has no `v4.33.1` tag; the tags track Lean releases and jump from `v4.33.0`
to `v4.34.0-rc1`. Cloning at tag `v4.33.0`, overwriting its `lean-toolchain` with `v4.33.1` and
running `lake build` succeeds in six jobs with no errors. Exporting this repository's four library
roots then works on the first attempt:

    lake env <lean4export> OOCert Shacl.All Dl Fol > oo.ndjson

That produced 515 MB and 9,850,420 lines in about 97 seconds, with empty stderr and exit 0. The
size is not a surprise and is worth understanding: the export is transitive, so it carries all of
Lean core's `Init` and `Std` as well as the roughly 15,288 lines of this repository. The proofs
this project wrote are a small fraction of what has to be re-checked, which is itself a fact about
where the trust actually sits.

**READ.** The export format is NDJSON version 3.1.0, and the format churn is over: the transition
from the old whitespace-delimited format landed on 2025-12-13 and the version has been stable at
3.1.0 across every tag from v4.29.0 to v4.34.0. Anything written against the old format does not
read current output. `lean4export` is actively maintained, its master `lean-toolchain` was bumped
to v4.34.0 on 2026-09-14. `Expr.mdata` is stripped by default, and `unsafe` and `partial`
declarations are omitted unless `--export-unsafe` is passed; neither matters here, because `lean/`
declares no `unsafe` and no `partial`.

### The checker: nanoda

[`ammkrn/nanoda_lib`](https://github.com/ammkrn/nanoda_lib) is a Lean 4 type checker written in
Rust. It shares no code with the Lean kernel, runs on no Lean runtime, and reads only the NDJSON.
That is genuine implementation independence, which is the thing this repository does not otherwise
have anywhere in its Lean layer.

**RAN.** `cargo build --release` at master `4c544ed` (2026-09-09), version 0.4.17, succeeded in 74
seconds with no patching. The tool takes a JSON config and no CLI flags.

The first run FAILED, and the failure is worth recording because it is the kind of thing an
optimistic write-up would omit. Configured with `permitted_axioms` set to exactly this
repository's three, `propext`, `Classical.choice` and `Quot.sound`, and
`unpermitted_axiom_hard_error: false`, it ran for 13 minutes 25 seconds and then panicked:

    thread 'main' panicked at src/tc.rs:230:13:
    declaration not found in infer_const, Lean.trustCompiler

That is not a defect in this repository's proofs. `unpermitted_axiom_hard_error: false` drops an
unpermitted axiom from the environment instead of refusing at startup, and something in Lean
core's own prelude, which the transitive export drags in, genuinely uses `Lean.trustCompiler`. The
fix is to permit it. Mathlib's own nightly nanoda job reaches the same conclusion, and its
configuration was fetched rather than assumed: `.github/workflows/daily.yml` on mathlib4 master
allows exactly `propext`, `Classical.choice`, `Quot.sound` and `Lean.trustCompiler`, with
`unpermitted_axiom_hard_error: false` and both extensions on. The lesson for anyone repeating this
is that the allowlist has to cover the whole export, not the part of the export you wrote.

**RAN.** With `Lean.trustCompiler` added to the allowlist, and `nat_extension` and
`string_extension` both on because they default to off and the export uses both, the second run
SUCCEEDED:

    permitted_axioms: propext, Classical.choice, Quot.sound, Lean.trustCompiler
    unpermitted_axiom_hard_error: false
    nat_extension: true, string_extension: true

    Skipped exported but unpermitted axioms ["Lean.ofReduceNat", "sorryAx", "Lean.ofReduceBool"]
    exit 0
    24 minutes 59 seconds wall

So this repository's Lean developments HAVE been re-checked by a kernel that shares no code with
Lean, once, by hand, on 15 September 2026. That is the headline and it should be stated with its
limits attached.

Start with what the result does and does not rule out. nanoda re-checks the SAME TYPE THEORY. It
is an independent IMPLEMENTATION, not an independent FOUNDATION, so it defends against a bug in
the C++ kernel and not against an error in the type theory itself or in the shape of Lean's
axioms. The independence it buys is real and it is the kind the 2026 bug hunt was about; it is not
the kind Isabelle bought, and the two should never be described as the same thing.

Then note that one line of its output is a genuine finding rather than noise. The skipped-axiom
list names `sorryAx` and `Lean.ofReduceBool`. Those are declared in Lean core's prelude, so they
appear in the export, and nanoda dropped them because they were not on the allowlist. Had ANYTHING
in the export actually used one, the run would have aborted exactly as the first run aborted on
`Lean.trustCompiler`. It did not abort. So a checker outside Lean, reading only the exported
terms, independently confirms what entry 9 of `docs/trusted-computing-base.md` asserts from
inside: no `sorry` and no `native_decide` anywhere under these proofs, including under the 44
theorems the `#guard_msgs` pins do not reach.

That is the single most useful thing this exercise produced, and it is worth being precise about
why it is useful. It does not depend on the pins being complete, it does not depend on reading 51
files, and it does not depend on trusting the elaborator. It is a check over the compiled terms by
a program with no stake in the answer.

The cost is the other half of the honest report. Twenty-five minutes of wall clock and 515 MB of
intermediate file, for a repository whose own proofs are 15,288 lines, because the export is
transitive and Lean core dominates it. That is the number to weigh against the benefit, and it is
why the recommendation below is what it is.

### The checker this repository could run today for free: leanchecker

**READ.** [`leanprover/lean4checker`](https://github.com/leanprover/lean4checker) is deprecated
and archived; its last commit, 2026-03-25, is the deprecation notice. It was merged into Lean
itself and now ships as `leanchecker` with every toolchain from v4.28.0.

**RAN.** `leanchecker` is present in the v4.33.1 toolchain on this machine. Invoked correctly,
from `lean/` so that elan resolves the pinned toolchain rather than the machine default, it passes
on every root:

    lake env leanchecker --fresh OOCert     exit 0
    lake env leanchecker --fresh Shacl.All  exit 0
    lake env leanchecker --fresh Dl         exit 0
    lake env leanchecker --fresh Fol        exit 0

Invoked as a bare binary it fails with `incompatible header`, because it then resolves the default
toolchain's library path, which on this machine is v4.34.0. Anyone adding this to CI must go
through `lake env`.

**What it is NOT.** `leanchecker` uses the Lean kernel. Its own source says so: "This is not an
external verifier, simply a tool to detect 'environment hacking'." It replays the declarations in
a compiled module through `Lean.addDecl` from a fresh environment, so it catches a metaprogram
that built an inconsistent `Environment` behind the elaborator's back, and it catches nothing
whatever that is a kernel bug. It also performs no axiom auditing: there is no allowlist and
nothing is reported, so it is not a substitute for the `#guard_msgs` pins.

That is still worth something this repository does not currently have. `grep` finds no mention of
`leanchecker` anywhere in this tree, and `.github/workflows/ci.yml` runs `lean-action` and the
Rust test suite and nothing else on the Lean side. Adding one `lake env leanchecker --fresh` step
closes the elaborator-tampering gap named in entry 8 of
[docs/trusted-computing-base.md](trusted-computing-base.md) at the cost of one CI line and no new
dependency, because the binary is already in the toolchain.

### The checker that is only half independent: lean4lean

**READ.** [`digama0/lean4lean`](https://github.com/digama0/lean4lean), by Mario Carneiro, is a
Lean 4 kernel written in Lean 4, last commit 2026-08-29, `lean-toolchain` pinned at `v4.33.0-rc2`.
The published version is "Lean4Lean: Verifying a Typechecker for Lean, in Lean", TYPES 2025,
LIPIcs 384 pages 2:1 to 2:23, DOI `10.4230/LIPIcs.TYPES.2025.2`, with the preprint at
arXiv:2403.14064.

Being written in Lean 4 is exactly the limitation the brief anticipated, and the project states it
more bluntly than an outsider would dare. From its README: "It is derived directly from the C++
kernel implementation, and as such likely shares some implementation bugs with it (it's not really
an independent implementation)". So it shares the Lean compiler, the Lean runtime and the bignum
library, and its algorithms are ported from the thing it is meant to check. It is partial
independence, and the honest description is "a second implementation by the same lineage" rather
than "an independent kernel".

What keeps it off the recommendation is more than that. Its verification layer,
`Lean4Lean.Verify`, still contains `sorry` across roughly 27 files, so the verified-typechecker
claim is work in progress and not a finished artefact. And it deliberately does not implement
`reduceBool`, so it cannot check `native_decide` proofs at all, which is a virtue for this
repository's purposes and a limitation in general.

### The context that makes this route not merely theoretical

**READ, with the headcount checked.** The Lean FRO now runs a public conformance suite for
external checkers, the [Lean Kernel Arena](https://github.com/leanprover/lean-kernel-arena). Its
`checkers/` directory was listed on 2026-09-15 and holds 24 entries, in Rust, Haskell, OCaml,
Lean, RPython, Fortran, COBOL, Solidity and plain TeX, alongside the official C++ kernel as a
reference. It exists because of a kernel soundness bug hunt run between 30 July and 20 August
2026, written up by Leonardo de Moura, in which models found genuine soundness bugs in the shipped
C++ kernel by writing bogus proofs directly in the export format. The fixes shipped in
**v4.33.1**, which is the version this repository pins. The arena scores the v4.28.0 C++ kernel,
which it keeps as a reference entry, as wrong on six of its tests. That last figure is the arena's
own published result and was not re-derived here.

That is the strongest available argument that external re-checking is not ceremony. The Lean
kernel had exploitable soundness defects two months ago, they were found by exporting and
re-checking, and several independent checkers get those cases right that the then-current official
kernel got wrong.

## Route 2: Dedukti and Lambdapi

[docs/reasoning-systems-inventory.md](reasoning-systems-inventory.md) already declines Dedukti, on
the ground that it solves a portability problem this project does not have. That reasoning stands
and this section does not overturn it. It answers a different question, which the inventory did
not ask: not "should we publish our certificates in a common framework" but "could a Lean proof be
re-checked in a genuinely different logical foundation". The answer is no, and the reasons are
structural rather than a matter of polish.

**READ.** [Dedukti](https://github.com/Deducteam/Dedukti) is in maintenance-only decline. Its last
release is v2.7 from 2022-06-18, three years and three months ago. Commit volume runs 88 in 2022,
24 in 2023, 14 in 2024, 10 in 2025, 3 in 2026, and the 2026 commits are a dependabot bump, a warning
cleanup and a typo fix. Its advertised homepage returns 404.
[Lambdapi](https://github.com/Deducteam/lambdapi) is the healthy half, release 3.0.0 on 2025-07-16
with commits through 2026-09-13, and it reads `.dk` directly. The funding position matters for any
forecast: Gilles Dowek, who founded Deducteam, died on 21 July 2025, and EuroProofNet, the COST
Action that paid for most of this interoperability work, held its final symposium in September
2025.

**Is there a Lean 4 to Dedukti translator?** Yes, exactly one:
[`Deducteam/lean2dk`](https://github.com/Deducteam/lean2dk), by Rishikesh Vaishnav, built on
[`Deducteam/Lean4Less`](https://github.com/Deducteam/Lean4Less), which first rewrites Lean into a
"Lean minus" without proof irrelevance or K-like reduction by inserting explicit casts. The work
is real and it is a PhD: "Translating proofs from Lean to Dedukti", Université Paris-Saclay,
defended
2026-03-10, with Sebastian Ullrich and Mario Carneiro on the jury.

**RAN.** The version gap was checked directly rather than taken on trust. Fetching each branch's
`lean-toolchain`:

| repository | branch | pinned Lean |
|---|---|---|
| `lean2dk` | `claude-stable` (most complete) | `v4.18.0-rc1` |
| `lean2dk` | `stable` | `v4.18.0-rc1` |
| `lean2dk` | `main` | `v4.22.0-rc4` |
| `lean2dk` | `thesis` | `v4.22.0-rc4` |
| `Lean4Less` | `main` | `v4.28.0-rc1` |

This repository is on `v4.33.1`. The most capable branch of the only Lean-to-Dedukti translator is
fifteen minor versions behind it. No attempt was made to run it, and that is a deliberate choice
rather than a gap in the investigation: porting a translator across fifteen Lean releases is a
research project, not a verification step, and the things it would buy on arrival are already
disqualifying.

**READ, and this is the part that decides it.** Even where `lean2dk` works it does not deliver a
re-check.

- It type-checks one module end to end, `Init.Data.Nat.Lemmas`. Not a library, one module.
- Stock Dedukti cannot check its output. It needs a personal fork of Dedukti adding lazy-delta
  congruence and convertibility memoisation, and without that "checking realistic modules will not
  terminate (or will exhaust memory)".
- It VALUE-STUBS constants it cannot handle, emitting the type and dropping the body, which turns
  them into postulates. Its own README says: "A stubbed constant is trusted, not checked: Dedukti
  does not independently verify it." A re-checker that converts the hard cases into assumptions is
  the exact shape of the assurance-laundering failure this project exists to attack, and it would be
  worse than no re-check because it would produce a green result.
- `String` literals are not translated at all; the tool emits a `STRLIT.FIXME` placeholder. This
  repository's certificates are strings end to end.
- The thesis reports that translation "scales quite poorly", producing output two to three orders of
  magnitude larger than the input.
- Its CI translates and never type-checks, because the patched fork is not available there.

The theoretical obstacles are not where one might guess, and it is worth correcting the folklore.
Lean's lack of cumulativity is an ADVANTAGE, because it makes the theory a functional PTS and lets
the standard encoding apply; it is Rocq's cumulativity that breaks that. Quotients are not an
obstacle either, and go through as rewrite rules on `Quot.lift` and `Quot.ind`. Impredicative
`Prop` is handled. What genuinely resists is definitional proof irrelevance, which is why
Lean4Less exists and costs a measured 10 to 36 percent extra type-checking time, and the `Nat` and
`String` kernel extensions, which are unsolved: the encoding is unary, so `Nat.gcd` under K-like
reduction aborts translation of the constant and everything depending on it.

**Logipedia.** Dead, and its Lean support is Lean 3. `logipedia.science` no longer resolves in
DNS; `logipedia.inria.fr` accepts a TCP connection and returns nothing, over a certificate that
expired on 2021-06-25. The repository's last substantive commit is February 2022. Its Lean backend
emits `constant X : T.` and `fun (x : T) , e`, which is Lean 3 syntax, and its CI downloads Lean
3.4.2. It is also the wrong direction for this purpose: it exports Dedukti content INTO Lean, and
everything goes through simple type theory, so only simply-typed content ever moved.

## Route 3: Coq/Rocq and Agda as re-checking targets

This is not a proposal to re-formalise anything. The value Isabelle delivered came from being a
different foundation read independently from the specification, and Coq and Agda sit in the same
dependent-type-theory family as Lean, so a third and fourth formalisation would buy repetition
rather than independence. The question here is narrower: could either serve as a target for
RE-CHECKING an existing Lean proof, by way of Dedukti.

**READ. No, and the last mile does not exist.** Lambdapi's export targets in released 3.0.0 are
`lp`, `dk`, `raw_dk`, `hrs`, `xtc`, `raw_coq` and `stt_coq`. A Lean export, `src/export/lean.ml`,
was added on 2026-06-25 and is still under "Unreleased". All of these carry a limitation that is
fatal here, stated in Lambdapi's own `doc/cli.rst`: the translation happens just after parsing and
before elaboration, the formats accept only `require`, `open`, `symbol` and `rule`, and "rules are
simply ignored". An undefined symbol with a type is emitted as a Coq `Axiom`.

That is decisive for Lean specifically. A Dedukti encoding of Lean's type theory IS a rewrite
system: universe-level normalisation, recursor reduction, the `Quot` rules. Pushing it through a
backend that drops every rule and emits axioms would produce something that plausibly compiles in
Rocq while proving nothing whatsoever. The output would be a green light with no content behind
it.

The contrast that proves the point is the pipeline that does work. `hol2dk` translates HOL Light
through Lambdapi into Rocq at real scale, shipping as the opam package `coq-hol-light`. It works
because HOL Light is simple type theory: its encoding lives in one fixed theory file shipped once,
and there are no per-development rewrite rules, so "rules are ignored" never bites. The property
that makes HOL Light succeed is the property Lean lacks.

For Agda, `dk2agda` last saw a commit on 2020-07-31 and `Agda2Dedukti` on 2022-01-24, the latter
requiring a fork of Agda. The one working Dedukti to Agda demonstration, `predicativize`,
translated Matita's arithmetic library and pins Dedukti to a specific commit hash.

The authors say this themselves. The thesis figure "Exporting Lean to other systems via Dedukti"
draws Lean to Lean-minus to Dedukti as achieved, and draws the onward boxes to Rocq, Agda and
Isabelle with dashed borders and dashed arrows. Its list of existing Dedukti export tools names
PVS, Matita, Rocq, OpenTheory and Agda, and Lean is absent from it.

## Verdicts

| route | works today | version reality | effort measured here | what it would buy |
|---|---|---|---|---|
| `lean4export` | **yes, RAN** | tag `v4.33.0` with `v4.33.1` copied over the toolchain | clone, one-line toolchain edit, `lake build` 6 jobs; export 97 s, 515 MB | the input every other route needs |
| `nanoda` | **yes, RAN, accepted, exit 0** | master `4c544ed`, v0.4.17, Rust, no Lean runtime | `cargo build --release` 74 s; one JSON config; 25 min per run; one failed run first | an independent IMPLEMENTATION re-checking the same type theory, and a `sorry`/`native_decide` audit that does not rely on the pins |
| `leanchecker` | **yes, RAN** | ships in the v4.33.1 toolchain | zero install, one `lake env` line per root | closes elaborator tampering, NOT an external verifier, free |
| `lean4lean` | probably | pins `v4.33.0-rc2`; not attempted here | unmeasured; a toolchain override plus batteries | partial independence only, by its author's own statement |
| Dedukti via `lean2dk` | **no** | translator pins `v4.18.0-rc1`, fifteen minor versions back | porting a translator across fifteen Lean releases, plus a personal Dedukti fork | would stub what it cannot check, which is worse than nothing |
| Logipedia | **no** | Lean 3.4.2, sites dead since 2021 to 2022 | not applicable | nothing |
| Rocq or Agda via Dedukti | **no** | Lambdapi's exports drop rewrite rules and emit axioms | not applicable, the last mile does not exist | a green light with nothing behind it |

## Recommendation

**Do not integrate any of this yet, with one cheap exception, and here is the condition under
which the rest becomes worth doing.**

The exception is `leanchecker`. It is already in the toolchain, it needs no dependency, it passes
today on all four roots, and one `lake env leanchecker --fresh` line per root in the `lean` CI job
closes the elaborator-tampering half of entry 8 in `docs/trusted-computing-base.md`. There is no
argument for leaving free assurance on the floor. It must be described accurately when added: it
is not an external verifier and must never be written up as one.

Everything beyond that should wait, for a reason that has nothing to do with whether the tools
work, because Route 1 demonstrably does. It is that the proof-side risk is currently NOT the
largest risk in this system, and spending the next increment of effort there would be optimising
the strongest link. `docs/trusted-computing-base.md` names two irreducible items at the
certificate boundary, both statements about an execution that no amount of proof checking reaches,
and entry 9 names 44 hand-written theorems that no axiom pin reaches, which is closable in a dozen
lines. Re-checking
9.85 million exported lines in a second kernel, every run, to defend against a kernel bug class that
the pinned v4.33.1 was specifically released to fix, is not where the marginal defect is.

Adopt Route 1 properly when any ONE of these becomes true:

1. **An outside party has to trust a verdict from this repository without trusting Lean.** A
   regulator, an auditor or an assurance customer who will not take "the Lean kernel accepted it" is
   the case this whole route is for. The argument then changes from internal hygiene to evidence, and
   a second kernel's independent acceptance is exactly the artefact.
2. **A kernel soundness bug lands that affects the pinned version.** The 2026 bug hunt is the
   precedent and the arena is the early-warning system. If v4.33.1 acquires a known unsoundness, the
   question stops being hypothetical the same day.
3. **The Lean layer grows past what one person can read.** The current defence is that `lean/` is
   15,288 lines with no external package and can be read in an afternoon. Taking Mathlib, which
   `docs/reasoning-systems-inventory.md` already flags as a live trade for Duper and lean-smt, would
   end that defence and make an independent re-check the cheaper of the two remaining options.

If and when that trigger fires, the route is settled and does not need re-investigating: export
with `lean4export` at the tag matching the pinned Lean with the toolchain copied over, re-check
with `nanoda` at master with `nat_extension` and `string_extension` on and an explicit
`permitted_axioms` allowlist. Do not use the `debug` branch of `nanoda_lib` that Mathlib CI still
pins, which is from October 2025. Do not pursue Dedukti: it is the intellectually attractive
answer, it is the one that would give genuine foundational diversity rather than implementation
diversity, and on the evidence above it cannot deliver for Lean today at any scale, by its own
authors' account.
