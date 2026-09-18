# Aeneas at the Rust/Lean boundary

`docs/trusted-computing-base.md` ends on the same sentence every time: `lean/` proves a conditional,
and everything to the left of its "if" is fifty thousand lines of unverified Rust writing down the
truth. The functions that decide whether it does are small, pure and total. This page is the record
of pointing a Rust-to-Lean verification tool at exactly those functions and finding out, honestly,
what it can and cannot do here.

The tool is [Aeneas](https://github.com/AeneasVerif/aeneas) (Inria and Microsoft Research). Its
front end, [Charon](https://github.com/AeneasVerif/charon), is a `rustc` driver that extracts LLBC
(a low-level borrow calculus) from a crate; Aeneas consumes the LLBC and emits a pure functional
model in Lean 4, F\*, Coq or HOL4. It handles a subset of safe Rust.

Read the gate first. It decides everything else on this page.

## The gate

Three questions had to be answered before a line was written, and two of them have answers that
constrain the result permanently.

### 1. Does it install and run on this machine?

**Yes.** macOS 26.6.2 (`darwin 25.6.0`), Apple silicon. Aeneas publishes a nightly release per day with
a `aeneas-macos-aarch64.tar.gz` asset carrying prebuilt `aeneas` (OCaml), `charon` and
`charon-driver` (Rust) binaries plus the four backends, so no OCaml toolchain was needed. This
machine has no `opam` and none was installed. The pinned release is
`nightly-2026.09.15-505b6ca` (`505b6ca35217e7be5c96c3e2f8045edfbdf47291`).

`aeneas/run.sh` fetches it, runs Charon over `aeneas/oo-boundary`, runs Aeneas over the resulting
LLBC and writes `aeneas/lean/OOBoundary/Generated.lean`. End to end it takes under ten seconds once
the release is downloaded.

### 2. Which Lean does it target, and what does its support library depend on?

**This is the finding that shapes everything else.** It does not break the three-axiom pin, and
the measurement further down says so; what it breaks is any hope that the output could live in
`lean/`.

`lean/` is core Lean only. `lean/lakefile.toml` says so in a comment and the manifest is empty: no
Mathlib, no Batteries, no external package at all, `leanprover/lean4:v4.33.1`, and every headline
theorem carries a `#guard_msgs`-checked `#print axioms` pinning it to exactly
`[propext, Classical.choice, Quot.sound]`.

Aeneas's generated files open with `import Aeneas`. That is not optional and there is no smaller
target: the model is written in Aeneas's `Result` monad over its `Slice`, `alloc.vec.Vec` and
`UScalar` types, and every one of those lives in the support library. The support library's
`backends/lean/lakefile.lean` reads, verbatim:

```lean
-- Important: mathlib imports std4 and quote4: we mustn't add a `require std4` line
require mathlib from git
  "https://github.com/leanprover-community/mathlib4.git" @ "v4.31.0"
```

and its `backends/lean/lean-toolchain` reads `leanprover/lean4:v4.31.0`.

So, concretely, measured in this repository rather than quoted from a README:

| | `lean/` | `aeneas/lean/` |
|---|---|---|
| Lean | `v4.33.1` | `v4.31.0` |
| Lake packages required | 0 | 10 (`aeneas`, `mathlib`, `batteries`, `aesop`, `Qq`, `plausible`, `proofwidgets`, `importGraph`, `LeanSearchClient`, `Cli`) |
| `.lake` on disk after a build | 78 MB | 7.6 GB, of which 6.2 GB is Mathlib (8176 `.olean`s, fetched by `lake exe cache get`) |
| axiom pin | `[propext, Classical.choice, Quot.sound]`, no exceptions | see "the axiom accounting" below |

Some care is owed here, because "depends on Mathlib" is not by itself an indictment:

- **Mathlib does not add axioms.** Mathlib's own policy is the same three, so importing it does not
  by itself put a fourth axiom behind a theorem. The axiom accounting below is measured, not
  assumed.
- **Aeneas's `Std` is `sorry`-free.** `grep -rn sorry backends/lean/Aeneas/Std/` is empty. The nine
  `sorry`s in the bundle are all inside `#guard_msgs` doc-strings in `Aeneas/Tactic/`, which are the
  *expected output* of `extract_goal` in the tactic framework's own tests, not sorried declarations.
  `native_decide` appears only in `example`s, which do not propagate.

What is true is that the *trust surface* is a different object. `lean/`'s is "the Lean kernel plus
what is written in this directory, small enough to read in an afternoon". `aeneas/lean/`'s is the
Lean kernel plus Mathlib plus eight more packages plus the Aeneas support library plus the Aeneas
translation itself. The three-axiom pin is worth having precisely because it has no exceptions, so
this work does not touch it: `aeneas/lean/` is a separate Lake workspace with its own
`lean-toolchain`, its own manifest and its own `lake build`, and there is no path from
`lean/lakefile.toml` to it. Nothing in `make check` or `lake build` under `lean/` reaches this
directory.

### 3. Does Charon work with this repository's Rust?

**Not with the shipped crate, and it does not need to.** Charon is a `rustc` driver linking
`rustc_private`, so it pins a nightly exactly: `nightly-2026-08-18` with `rustc-dev`, `llvm-tools`
and `rust-src`. The shipped crate has no `rust-toolchain` file, is `edition = "2024"` and builds on
stable (1.96.0 here). It also has around ninety dependencies and fifty thousand lines, most of it
reaching into `oxrdf`, `oxiri`, `tokio` and `axum`, none of which Aeneas models.

So Charon is pointed at `aeneas/oo-boundary`, a crate with one line of its own:

```rust
#[path = "../../../src/boundary_core.rs"]
pub mod boundary_core;
```

and no dependencies. The `rust-toolchain` pin lives in that directory and nowhere else. This is the
only place in the repository that names a toolchain, and the shipped crate still builds on stable.

## What was extracted, and what it cost

`src/boundary_core.rs` is new and is the pure core of the certificate boundary: seven functions and
one three-constructor enum, over `u8`, `&[u8]`, `Vec<u8>`, `usize` and `bool`. `src/reason.rs` and
`src/tableaux.rs` call it. It is not a copy of them and there is no transcription step: the crate
Charon reads reaches this same file with `#[path]`, so the Lean model is a model of the bytes the
shipped engine compiles.

Six shipped functions became one-line wrappers:

| shipped | file | now |
|---|---|---|
| `writable_triple` | `src/reason.rs` | `writable_triple_bytes(s.as_bytes(), p.as_bytes())` |
| `field_fits_the_format` | `src/reason.rs` | `field_fits_the_format(f.as_bytes())` |
| `term_fits_the_format` | `src/reason.rs` | `term_fits_the_format(t.as_bytes())` |
| `push_asserted_line` | `src/reason.rs` | `push_asserted_line_bytes(out, …)` |
| `push_triple_fields` | `src/reason.rs` | `push_triple_fields_bytes(out, …)` |
| `name_is_safe` | `src/tableaux.rs` | `name_is_safe_bytes(s.as_bytes())` |

The refactor was not free, and neither cost is cosmetic:

1. **The certificate buffers are now `Vec<u8>` rather than `String`.** Aeneas does not model `str`
   well enough to translate a writer that appends into one, and a wrapper cannot bridge that without
   `unsafe`. `std::fs::write` takes `AsRef<[u8]>`, so nothing downstream notices, and every byte
   appended still comes from a `&str`. The two property tests and the two generic Kani harness
   bodies that assert on the buffer (nine instantiations between them) were changed to match; what
   they assert is unchanged, and `make verify` still verifies all fifteen.
2. **`&str` predicates became byte predicates.** `s.contains([' ', '\t', '\n', '\r'])` became a
   byte scan. These agree on every `&str`, because all four are ASCII and no ASCII byte occurs
   inside a multi-byte UTF-8 sequence, but that sentence is an argument and arguments are what this
   repository writes tests for. `tcb_wrappers_agree_with_the_char_level_predicates` in
   `src/reason.rs` is that test: it runs the original `char`-level bodies and the new byte-level
   ones over the same adversarial generator and requires them to agree.

## What Aeneas translated

Everything in `src/boundary_core.rs`, with no opaque declarations and no `sorry`:
`Position` (with its derived `Clone`, `Copy`, `Debug`, `PartialEq`, `Eq` instances),
`starts_with_byte`, `writable_triple_bytes`, `field_fits_the_format`, `term_fits_the_format`,
`name_is_safe_bytes`, `push_asserted_line_bytes`, `push_triple_fields_bytes`. 311 lines of Lean.

The parts most likely to have failed did not, and they are worth naming:

- **The `while` loops translated.** Each becomes a `body` returning `ControlFlow` plus a
  `loop` combinator defined by `partial_fixpoint`, which is a real fixpoint and not an axiom. The
  induction to reason about it has to be written by hand; it is written in
  `aeneas/lean/OOBoundary/Proofs.lean`.
- **`Vec::extend_from_slice` translated to a primitive with a real model**
  (`alloc.vec.Vec.extend_from_slice`, which is `v.val ++ s.val` under a length side-condition), not
  to an opaque declaration. That is what let the writers keep `extend_from_slice` instead of a
  byte-at-a-time push loop, so the refactor costs no memcpy.

## What was proved

See `aeneas/lean/OOBoundary/Proofs.lean`. The decision procedures are stated as equations
`model x = ok (spec x)` rather than as Hoare triples, because an equation covers more ground than a
triple: the model never fails, it never diverges, and its result is the specification.

Fourteen theorems, in `OOBoundary`:

| theorem | says | was |
|---|---|---|
| `starts_with_byte_eq` | the leading-byte test is the head of the list, every length | not stated |
| `writable_triple_bytes_eq` | the unwritable-conclusion guard decides both positions, every length | BOUNDED: `writable_triple_decides_both_positions` fixes both terms at four bytes |
| `field_fits_the_format_eq` / `_iff` | a field is accepted IF AND ONLY IF it is non-empty and carries no tab, line feed or carriage return | SAMPLED |
| `name_is_safe_bytes_eq` / `_iff` | TCB-25: a name is accepted IF AND ONLY IF it is non-empty and carries no space, tab, line feed or carriage return | SAMPLED: `tcb_25_name_is_safe_refuses_every_separator` |
| `term_fits_the_format_eq` | TCB-4 and TCB-5: accepted exactly when non-empty, separator-free and in one of the three N-Triples spellings | SAMPLED |
| `push_asserted_line_bytes_ok` | when the three terms fit, the buffer gains exactly `s TAB p TAB o LF` | BOUNDED: `asserted_line_round_trips_at_0` to `_at_4`, so terms of zero to four bytes |
| `push_triple_fields_bytes_ok` | the same for `TAB s TAB p TAB o` | BOUNDED: `triple_fields_append_exactly_three_at_0`, `_at_2`, `_at_3`, `_at_4` |
| `push_asserted_line_bytes_refuses` / `push_triple_fields_bytes_refuses` | all or nothing: a term that does not fit makes the writer refuse AND hand back the buffer it was given, byte for byte | BOUNDED, and only at a one-byte prefix |
| `asserted_line_reads_back` | TCB-1: an accepted line is one record on line feed, and that record splits on TAB into exactly the three terms that went in | SAMPLED |
| `triple_fields_adds_exactly_three_fields` | TCB-2, TCB-3: a line carrying `n` tab-separated fields carries exactly `n + 3` afterwards, FOR EVERY PREFIX | BOUNDED: prefix fixed at one byte |
| `triple_fields_are_the_three_terms` | and which fields they are, in order | SAMPLED |

One hypothesis, said out loud because "no bound" should not be read as "no
precondition": the two `_ok` theorems assume the buffer plus the line it is about
to gain fits in a `usize`
(`out.length + s.length + p.length + o.length + 2 < Usize.max`). That is not a
length bound on the terms, it is the capacity precondition Aeneas's `Vec` model
carries and that Rust's `Vec` enforces by aborting. The two `_refuses` theorems
carry no such hypothesis, because refusing allocates nothing.

The entries a bounded checker cannot reach at all are worth reading twice:

- **The prefix in `triple_fields_adds_exactly_three_fields` is unconstrained and
  may itself carry tabs.** The Kani version fixes it at a single byte.
  Real prefixes are a rule name, or a rule index and a binding list of any
  length, and then the conclusion and premises of a step already written. The
  Lean theorem covers every one of them because it is proved from
  `splitOnByte_append_cons`, which does not assume the left side is
  separator-free.
- **`writable_triple_bytes_eq` covers the EMPTY term** and the Kani harness
  cannot: it allocates a fixed four-byte array. What the theorem shows is that
  an empty subject PASSES this guard, which is fine only because
  `term_fits_the_format` refuses an empty term and the certificate paths run
  both. That is now written down rather than assumed.

The reader-side theorems are stated against `Spec.splitOnByte`, a one-byte
splitter written out in `aeneas/lean/OOBoundary/Spec.lean` rather than imported,
so that anyone checking whether the statement says what they think it says can
read the splitter in ten lines. It is not `OOCert.Parse`'s splitter, and the
claim is the one `docs/trusted-computing-base.md` already argues is the right
one: the theorems are about the BYTE LAYOUT, which is what any correct splitter
needs and all it needs.

### What Aeneas could not handle, and why

- **`str` and `String`.** Aeneas has a `String` model but not enough of one to
  translate a writer that appends into a `String`, which is why the certificate
  buffers moved to `Vec<u8>` and the predicates moved to `&[u8]`. Everything
  else on this page follows from that one limitation.
- **The interner (TCB-15, TCB-16, TCB-17).** `Interner` is a
  `HashMap<String, u32>` plus a `Vec<String>`. Aeneas does not model
  `std::collections::HashMap`, and the property at issue (that interning is a
  bijection) is about the map. Nothing here touches it.
- **`parse_pat` (TCB-20).** Returns `anyhow::Result` and formats a message
  naming the line and the position, so translating it would mean translating
  `core::fmt` and `anyhow`. This is the same shape of obstacle that killed its
  Kani harness, and the same fix would serve both: lift the classification into
  a pure `Option<Pat>` function and leave the messages where they are.
  **That fix was made, and it did not help here.** `pat_of` is that function, it
  landed on 15 September 2026, and Kani verifies it. Aeneas still cannot take it,
  for the unrelated reason at the top of this list: `pat_of` returns
  `Option<Pat>` and `Pat` carries a `String`, so the obstacle is the string model
  rather than the error formatting. Moving it into the byte discipline the rest
  of this module uses would mean `Pat` over `Vec<u8>`, which changes the type
  every rule-table call site names. That was not done, and TCB-20's whole-line
  half is consequently the one serialisation property on
  `docs/trusted-computing-base.md` that still has no unbounded proof about this
  Rust. Dafny proves it unbounded about a REIMPLEMENTATION, which is a different
  claim; decision 0014 measures what that is worth.
- **Nothing was silently dropped.** The generated file has no `opaque`
  declaration, no `axiom` and no `sorry`: `grep -n "opaque\|axiom\|sorry"
  aeneas/lean/OOBoundary/Generated.lean` is empty. Aeneas reports eight opaque
  functions during the run; they are `core` library functions it models in
  `Aeneas.Std` (the `Debug` derive reaches `core::fmt::Formatter::write_str`),
  they are not emitted into the file, and no theorem here depends on one.


## The honest accounting

### This is a trade, not a gain

Trading "fifty thousand lines of unverified Rust" for "Aeneas's translation from Rust to the Lean
model is faithful" is a large net win at the boundary, and it is still a trade. TCB-30 in
`docs/trusted-computing-base.md` records it.

### The axiom accounting is separate and it is not the same regime

All fourteen theorems pinned in `aeneas/lean/OOBoundary/Axioms.lean` depend on exactly

```
[propext, Classical.choice, Quot.sound]
```

which is the same three `lean/` pins. `aeneas/lean/OOBoundary/Axioms.lean`
checks every one of them with `#guard_msgs`, so a change breaks `lake build` in
this directory rather than widening the trust surface quietly. Measured, not
assumed: the first draft of this page said "Mathlib does not add axioms" as a
fact about Mathlib's policy, and a policy is not a measurement.

**Do not read that as "so it is the same regime."** It is not, and the
difference is not in the axiom list:

- the Lean is `v4.31.0` against `lean/`'s `v4.33.1`, so the two can never share
  a file, a build or a `lake build` invocation;
- checking any of it needs ten packages on disk and a 7.6 GB `.lake`, 6.2 GB of
  it Mathlib `.olean`s, against 78 MB for `lean/`, which needs a Lean toolchain
  and nothing else;
- the elaborated proofs run through Mathlib tactics (`scalar_tac`, `omega`,
  `tauto`, `grind`) rather than only through what is written in the directory,
  so "small enough to read in an afternoon" is false of this workspace and true
  of `lean/`;
- and above all, every theorem here is about AENEAS'S OUTPUT. The axiom list
  measures what the Lean kernel had to assume. It says nothing at all about
  whether the model is the Rust, which is TCB-30 and is the actual price of this
  work.

So the three-axiom pin on `lean/` is untouched and still has no exceptions, and
the useful sentence is the one at the top of this section together with the four
below it, not either half on its own.


### What would close the gaps

- **The translation itself.** Charon and Aeneas are together tens of thousands of lines of Rust and
  OCaml, unverified, and the model is only as good as the translation. What would close it: nothing
  available today. Aeneas has no machine-checked soundness proof of its own pipeline. The partial
  mitigation that IS available is the one `aeneas/run.sh` implements: re-run the translation, diff
  the model, and fail if it moved, so a change to `src/boundary_core.rs` that changes the model
  cannot slip past review with the old proofs still in the tree.
- **Coverage.** Seven functions is seven functions. The interner (TCB-15 to TCB-17) is a `HashMap` and
  Aeneas does not model `std::collections::HashMap`; `parse_pat` (TCB-20) returns `anyhow::Result`
  and formats messages, which is the same thing that killed its Kani harness. Both would need the
  same treatment this work gave the writers: lift the pure decision out, leave the machinery where
  it is.
- **The toolchain split.** `aeneas/lean/` is on Lean v4.31.0 and `lean/` is on v4.33.1, so the two
  can never share a file. That closes when Aeneas's Lean backend catches up, and it will always lag
  by some amount because Mathlib does.
