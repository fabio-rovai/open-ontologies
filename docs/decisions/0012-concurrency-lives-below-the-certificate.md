# 0012. Concurrency lives below the certificate, and a proof cannot follow it there

Status: **declined**. Written 18 September 2026.

Iris is the higher-order concurrent separation logic built in Rocq. The question put to it was
whether it has any real application in this repository, given that the Rust side is not pure: it has
a store behind an `Arc`, fourteen process-global atomics, three rayon fan-outs, and an MCP server
that turns out to be more parallel than it looks.

The answer is no, and the evidence is an inventory rather than an argument about Iris.
`docs/concurrency-inventory.md` is that inventory, and it should be read first. This record is the
ruling that follows from it.

## The failure this rule prevents

Adopting a program logic for a hazard that lives in a dependency, and calling the result assurance.

That is the same failure decision 0005 rules on for provers and decision 0006 rules on for
refutations, in a third place. The pattern is that a name with a strong reputation gets pointed at a
system, and nobody checks whether the thing the name is strong at is the thing this system does.

## What was measured

What reading the code established, at the level of detail the inventory carries.

**The program is genuinely concurrent, and more so than the design notes say.** `rmcp` spawns every
request as its own tokio task (`rmcp-1.4.0/src/service.rs:963`), so tool calls run in parallel in
stdio mode and not only over HTTP. No tool method takes a lock on entry. `serve-http` shares one
`Arc<GraphStore>` across every session and every REST route (`src/main.rs:1932`, `1941-1947`).

**The one shared mutable object synchronises itself, inside a dependency.** `GraphStore` holds an
`oxigraph::store::Store` directly, and `src/graph.rs:74-80` says why: Oxigraph's `Store` is
`Send + Sync` and synchronises internally, so a mutex bought only the serialisation of
non-conflicting reads. Every `insert`, `clear`, `update` and `iter` in this codebase is a call into
that engine, which is backed by RocksDB, which is C++.

**The concurrency we do write has no invariant to prove.** All fourteen atomics in `src/runtime.rs`
are independent scalars read with `Ordering::Relaxed`, with no reader deriving anything from two of
them jointly, so there is no ordering property to establish. The three rayon fan-outs
(`src/tableaux.rs:2648`, `2712`, `src/claimcheck.rs:554`) are read-only over frozen data:
`DlReasoner` contains no interior mutability at all, and the only cross-thread mutable cell in the
parallel phases is `subsumption_cut_short`, a monotone latch that is only ever stored `true`
(`src/tableaux.rs:2707, 2716`). The double-checked lock at `src/claimcheck.rs:339-351` is textbook
and correct, and its one imperfection loses a cache invalidation rather than an answer.

**The certificate layer has no concurrency by construction.** `docs/trusted-computing-base.md`
states it plainly: the checkers are pure functions of files on disk. Decision 0010 goes further and
makes the reasoner's input an immutable value precisely so that the one window where the store could
drift under a certified run stops existing. Iris reasons about stateful concurrent programs. The
part of this system that carries the assurance is neither, on purpose, and decision 0010 is the
repository choosing to keep it that way.

## Why Iris in particular does not apply

The reasons below run in increasing order of how hard they are to argue with.

**The subset problem, which this repository has already ruled on twice.**
`docs/reasoning-systems-inventory.md` declines Aeneas with Charon, and separately Verus, Creusot and
Prusti, because each needs the code inside its own subset and this codebase is not inside any of
them. The Iris-based route to Rust is RefinedRust (PLDI 2024), and its subset is narrower than
Aeneas's, not wider. Declining the broader tools and adopting the narrower one would be incoherent.

**The part of our concurrency that is delicate is already proved, by someone else, and we get it for
free.** RustBelt (Jung, Jourdan, Krebbers and Dreyer, POPL 2018) used Iris to verify the soundness of
exactly the standard library types this codebase leans on: `Arc`, `Mutex`, `RwLock`, `Cell`,
`RefCell`. The unsafe code that makes `Arc<GraphStore>` sound to share is not our unsafe code. There
is no unsafe concurrent code anywhere in `src/`, and no hand-rolled lock-free structure. Pointing
Iris at a program whose only concurrency primitives are the ones Iris has already verified is
re-proving a published theorem about someone else's crate.

**The state that matters is not reachable.** Suppose the subset problem were solved. The property an
operator actually wants is that a query and a load do not corrupt each other, and that property is a
statement about Oxigraph's transactional behaviour. An Iris proof of it would need a formal model of
Oxigraph's MVCC layer, which nobody has written, and that model would still bottom out in RocksDB,
which is C++ and outside Rocq entirely. The proof would be larger than everything in `lean/`
combined and its conclusion would be conditional on an unverified model of a dependency. That is the
same shape as trusting a prover's refutation, which decision 0005 refuses.

**The defects that are actually here are not the defects Iris prevents.** This is the decisive one,
because it is empirical rather than architectural. The inventory found three defects while looking
for a property worth proving, and not one of them is a race:

- Defect A, live: the evictor spawned in `serve-http` and `daemon` mode is wired to a registry
  nothing loads into (`src/main.rs:1913-1921`), so `evictor_tick` returns `Ok(false)` forever on its
  third line (`src/registry.rs:284`) and `[cache] idle_ttl_secs` is silently inert. The code is
  perfectly synchronised. It simply does nothing. An Iris proof would establish the evictor's
  specification, and the specification is satisfied.
- Defect B, latent: `load_file` clears the store at `src/registry.rs:128` and does not take a lock
  until line 158, so two concurrent loads over the shared store leave it holding the union of two
  ontologies while each registry reports its own count. This is a missing critical section, visible
  by reading thirty lines.
- Defect C, latent: `touch()` stamps `last_access` when a tool starts and nothing holds a read lease
  while it runs (`src/registry.rs:179`, `212`), so a tool outliving `idle_ttl_secs` can have the
  store cleared under it at `src/registry.rs:312`.

A logic adopted to find concurrency defects that finds none of the three concurrency-adjacent
defects present is not being declined on taste.

## If it were attempted anyway, the size

Asked for honestly, because "it would be a lot of work" is the kind of claim this repository does not
accept without a number.

The only candidate property is the registry's lock protocol: that `active`, `reload_lock`,
`last_access` and `evicted` together maintain the invariant that the in-memory store agrees with the
active entry's `evicted` flag. `src/registry.rs` is 540 lines. A RefinedRust proof of it would need,
in order: the file rewritten into RefinedRust's subset, including its `anyhow` error paths and its
`serde_json` output; a specification of `tokio::spawn` and of task detachment, which RefinedRust does
not have; a specification of `std::fs` for the eviction snapshot, which is where the interesting
failure modes live; and a model of `GraphStore::clear` and `serialize`, which is the Oxigraph problem
above. Three of those four are new research rather than new proof. The fourth, the Oxigraph model, is
the largest single formalisation anyone has proposed for this project.

And the theorem at the end would be conditional on that model, so it would be an assurance claim
resting on an unverified translation of a dependency, which is the exact shape of claim decision 0005
exists to refuse.

## What is adopted instead

Nothing exotic, which is the point.

`loom` explores every interleaving of `std::sync` primitives exhaustively and is the standard tool
for defects B and C. It costs a dev-dependency and a `cfg` attribute, it needs no rewrite, and it
fails a test rather than emitting a proof obligation. A two-thread test with a barrier finds defect B
without even that.

Defect A needs no concurrency tooling at all, and
`tests/registry_evictor_wiring_test.rs` is the test that pins it. It asserts the current wrong
behaviour deliberately, in both directions, so that closing the defect breaks one assertion and the
tempting wrong repair breaks the other.

Kani stays where `docs/reasoning-systems-inventory.md` already puts it, on the trusted boundary, and
decision 0010's `CertifiedInput` stays the answer to defect D. Removing shared state beats reasoning
about it.

## What would have to become true to revisit this

Named concretely, so that a future reader can check them rather than re-argue the question.

1. **Hand-written unsafe concurrent code in the trusted path.** Today there is none. If someone
   writes a lock-free structure whose invariant no type system enforces, and that structure sits
   between the store and a certificate, the calculation changes.
2. **A concurrent phase inside the certified reasoner.** Decision 0010 makes the input a value
   precisely to avoid this. If a certified run ever acquires genuinely concurrent access to shared
   mutable state, there is a property worth proving and it is on the assurance path.
3. **A verified store.** If the RDF store were replaced by something with a machine-checked
   concurrency model, the third objection above dissolves and the second and fourth would have to be
   re-weighed on their own.
4. **RefinedRust's subset growing to contain async Rust and this codebase.** This is the least likely
   of the four and the easiest to check.

None of the four is true today. Two of them, 1 and 2, are things this project is actively arranging
not to be true.

## Consequences

The inventory is the deliverable and it stands on its own: `docs/concurrency-inventory.md` names
every piece of shared mutable state in the engine with a file and a line, which nothing did before.
One live defect, A, is recorded and pinned by a test. Two latent ones, B and C, are recorded with the
interleaving that reaches them.

`docs/reasoning-systems-inventory.md` gains a row, because a system considered and declined belongs
in that table rather than in a commit message. That is the whole purpose of the table: a reader
should be able to see that Iris was looked at, and why the answer was no, without reading this file.
