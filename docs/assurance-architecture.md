# The assurance architecture

Four verification technologies are in this repository or proposed for it. They are easy to conflate
and they answer different questions. This document says which question each one answers, what it
cannot answer, and how they compose. It also says what is deliberately left unverified, because an
assurance architecture that does not name its own boundary is a marketing document.

## The questions

| Layer | The question it answers | What it cannot answer |
| --- | --- | --- |
| Kani | Can this Rust code fail within the modelled bounds? | Anything outside the bound. A clean run is "no counterexample up to `k`", never "no counterexample". |
| Aeneas + Lean | Does this Rust function meet its specification for **all** inputs? | Anything outside the translatable subset, and anything about code Aeneas cannot ingest. |
| Certificate checker | Is this particular conclusion justified by this particular graph? | Whether the graph it was handed is the graph the engine actually reasoned over. |
| Isabelle/HOL | Does an independently developed formalisation agree? | Whether either formalisation is the one the W3C intended. Agreement is evidence, not proof. |
| Input provenance | Did the engine reason over the graph it says it did? | Nothing else. This is the row the other four do not cover. |

The fifth row is not decoration. The certificate checker is *told* what the assertions are, and its
soundness theorem is conditional on them. An engine that materialised a previous run's conclusions
into the graph it then certifies produces a green verdict about a graph nobody asserted, and no
amount of Lean catches it, because the Lean is not wrong. That failure is closed by having the
certified path read the asserted graph alone and record in the certificate which graphs it read.
`docs/trusted-computing-base.md` carries the history under TCB-8.

## The verified core

The proposal is a small dependency-free crate holding the computations that decide whether a
certificate is faithful, with the application left outside it.

1. **Canonical term encoding.** The property worth proving is injectivity: two distinct terms never
   encode to the same bytes, and no term's encoding can forge the format's structure from inside its
   own payload. This subsumes most of what the current serialisation guards do by construction.
2. **Rule substitution.** A substitution applied to a rule instance yields exactly the premises and
   conclusion the checker will re-derive. This is the heart of the certificate and it is pure.
3. **Certificate construction.** Field counts, separator discipline, and the property that a written
   line parses back to the value it was written from.
4. **Interval arithmetic**, and *not* interval parsing. Containment, overlap, ordering and the
   half-open convention are provable. Reading `xsd:gYearMonth` with a timezone offset into an instant
   is string handling, and it belongs at the edge: parse once, validate, and hand the core a clean
   instant type. An unreadable bound is already `invalid` rather than open, which is the same shape.
5. **Projection and scope calculation.** Which triples are in scope, and the monotonicity properties
   a slice must satisfy.
6. **The encoding fed to the hash**, and *not* the hash function. Proving a hash primitive correct
   means implementing a specification at large cost, and collision resistance is a cryptographic
   assumption rather than a theorem, so it cannot be proved at all. The provable and bug-prone part
   is that the byte string presented to the hash is a canonical injective encoding of the graph. Take
   the primitive from a verified implementation or leave it explicitly trusted, and say which.

### The load-bearing constraint

Extracting the core is worth nothing by itself. The guarantee comes from the engine being **unable**
to produce a certificate except through the core. If a second path exists, the verified component is
not on the critical path and the assurance is decorative. Enforcing this in Rust's type system, so
that laundering an unchecked verdict is a compile error rather than a code-review finding, is part of
the same piece of work and not a follow-up to it.

### Tiering

Kani and Aeneas do not want the same code. Kani model-checks real Rust and tolerates loops, mutation
and panics. Aeneas translates a functional subset and needs tractable borrow structure. The
translatable part will be a proper subset of the crate, so each function should be recorded in one of
three tiers rather than assumed to be in both:

- **Kani only.** Bounded absence of panics and arithmetic faults.
- **Kani and Aeneas.** Bounded checking plus a universally quantified theorem in Lean.
- **Neither.** Trusted, listed in `docs/trusted-computing-base.md`, with what would close it.

A function moving from the first tier to the second is the unit of progress here, because it is the
move from "no counterexample was found up to a bound" to "there is no counterexample".

### Why "dependency-free" is load-bearing and not tidiness

There is already direct evidence for it in this repository. Four Kani harnesses exist and the
`verify` target runs three. The fourth, `parse_pat_and_render_are_inverse`, does not terminate: no
verdict at fourteen minutes and 9.5GB, and it gets worse when the input is constrained. The cause is
not the property being hard. It is that `parse_pat` returns `anyhow::Result`, so CBMC flattens the
error-formatting machinery of an external crate whether or not the refusal paths are reachable.

A function reaching into a dependency dragged that dependency into the model checker and made the
proof obligation intractable. The same pressure applies to Aeneas more sharply, because it will
simply refuse to translate what it cannot ingest. The core being dependency-free is what makes both
tools usable on it, and the fourth harness is what that costs when it is not.

## What this deliberately does not verify

Oxigraph, the SPARQL layer, the MCP server, the CLI, the daemon, file and network I/O, and the bulk
of `src/`. That is the correct scope. The argument is not that the application is verified. It is
that a conclusion carries evidence a small verified checker accepts, and that the computations
deciding whether that evidence describes the real run are themselves checked. Verifying the
application is neither achievable nor necessary for that claim.

Adding Aeneas also adds a trusted entry rather than removing one for free: its translation from Rust
to the Lean model must be faithful. Trading tens of thousands of lines of unverified Rust for that
one assumption is a large net gain, and it is still a trade.

## Status

Written 15 September 2026. The four-layer decomposition above is the intended architecture, not a
description of the current state. What exists today:

- Kani, on four pure functions. Kept out of `make check` because it pulls its own toolchain.
- The Lean certificate checker, in core Lean with the axiom footprint pinned.
- Isabelle/HOL, as an independent second formalisation written from the W3C specifications.
- No extracted core crate, and no Aeneas.

`docs/trusted-computing-base.md` is the authority on what is trusted at any given time, and this
document defers to it wherever the two disagree.
