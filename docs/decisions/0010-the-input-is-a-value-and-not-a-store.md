# 0010. The input is a value and not a store

Status: accepted, not yet implemented. Written 15 September 2026.

(Number provisional. 0009 is taken on `certify-refutations` and several branches are open at once.)

## The defect

`run_horn` takes `&Arc<GraphStore>`. It selects assertions from the live store, writes them to
`asserted.tsv`, and reasons. Nothing binds the first act to the second. The following sequence
produces a certificate the Lean checker accepts, and the Lean is not wrong:

    export graph A into asserted.tsv
    reason over graph B
    emit a valid-looking certificate

The checker's soundness theorem is conditional on the assertions, and it is *told* what they are. It
asks whether a conclusion follows from a graph. It cannot ask whether that graph is the one the
engine reasoned over, because it never sees the engine.

`docs/trusted-computing-base.md` has carried this as TCB-8 and as trusted entry 5, described in
prose. This decision replaces the prose with a type.

## The decision

The certified reasoner accepts an immutable `CertifiedInput` and has no second path to the store.

    let input = store.certified_snapshot(scope)?;
    let result = reasoner.run(&input)?;
    let certificate = emit_certificate(&input, &result)?;

`CertifiedInput` owns the canonical bytes of the assertions it was built from. Selection copies once;
everything downstream reads an owned value. There are no phases that re-read the store, so there is
no window in which the store can drift between hashing and reasoning. That matters here because
`GraphStore::snapshot` is a serialiser that takes a format string, not an isolation primitive.

This paragraph used to continue "and no transactional snapshot is available to lean on. Copying is
the only honest option available". **That was false**, and the correction is below under *The
premise was wrong and the decision survives it*. A transactional snapshot IS available. Copying is
still what we do, for a reason that had to be measured rather than assumed.

The invariant to enforce and then to prove:

> The assertions hashed, the assertions reasoned over, the assertions written into the certificate,
> and the assertions replayed by the checker are the same immutable value.

The fourth conjunct is what Lean can establish. The first three are Rust-side, and are the target of
the type-level gating, Kani and Aeneas described in `docs/assurance-architecture.md`.

## The manifest has two halves and they must not be one object

This is the part most easily got wrong. Manifest fields divide by whether the checker can confirm
them from the artifact alone.

**Derived.** Recomputed by the checker from the canonical bytes, and refused on mismatch. Never read
as input. Includes the dataset digest, every per-graph digest and statement count, the total
assertion count, and whether the default graph is included.

**Attested.** Not confirmable from the artifact by anyone. A claim by the producer. Includes
`valid_at` and `as_of`, the engine version, the store revision, the import-closure policy, and the
identity of the operator. These are not lesser fields and they are not deletable, because an operator
needs them. They carry a different warrant and must be labelled with it.

Collapsing the two into one JSON object invites a reader to treat an attestation as a fact, which is
the failure this repository exists to prevent. The format therefore separates them by name, in the
same discipline that keeps `solver_verdict`, `encoding`, `checker_exit`, `verdict` and `owl_reading`
apart in `onto_fol_model`.

The certificate header binds to the input by digest: `input_digest`, `rules_digest`, `scope_digest`,
`manifest_digest`. A conclusion certificate then means *this conclusion follows from the dataset whose
canonical bytes hash to d*, and not *from some accompanying file*.

## Canonical bytes, and blank nodes

Most of this already exists. `src/pack.rs` writes sorted N-Triples so two packs of the same graph are
byte-identical, hashes them with SHA-256, and `onto_unpack` refuses a mismatch before loading a
single triple. The certificate path does not use any of it.

Two gaps: pack serialises N-Triples, which flattens graph identity and makes per-graph digests
impossible, so the canonical form must be N-Quads; and pack states no blank-node policy.

Blank nodes are the hard part and there is no free option.

- **RDFC-1.0**, the W3C Recommendation, is the recognised algorithm. It also carries a documented
  hazard: pathological blank-node symmetry drives it superlinear, and the specification tells
  implementers to bound the work. For a certificate system that is an availability failure rather
  than a soundness one, but it still needs a budget and a named refusal.
- **Skolemisation at ingestion** is precedented here, since five modules already skolemise, but doing
  it at ingestion mutates the user's data, which is a much larger commitment than skolemising for a
  comparison.
- **Refusing blank nodes in certified input** is honest and shippable now.

v1 refuses, with a named limitation and a pointer to this section. RDFC-1.0 behind a work budget is
the follow-up. What is never acceptable is hashing process-local blank-node labels, because the same
dataset would then acquire a different digest after a reload, and a content-addressed certificate
whose address changes for no semantic reason is worse than no address.

## Attestation is a separate layer and is deliberately deferred

The checker can establish that a conclusion follows from the assertions whose bytes hash to `d`. It
cannot establish that those assertions were in the operational database at half past ten. Only an
external attestation can, and an attestation is worth exactly what its key custody is worth.

Signed manifests are therefore **not** part of this work. Earlier in this project a commit-signing
scheme was built and then reversed and its key deleted, because a passphrase-less key is forgeable and
a forgeable assurance signal is worse than none: it manufactures confidence that does not hold. That
reasoning applies unchanged to a signed manifest. Build the attestation layer when someone can say
who holds the key and what happens when it leaks. Until then the honest artifact is an unsigned
manifest that says what the bytes are and does not claim to vouch for where they came from.

## Adversarial tests

Every one of these must fail closed, and each is a test:

export graph A and reason over graph B; keep a graph name and change its contents; mutate the store
after hashing and before reasoning; add or remove the default graph; change one imported ontology;
change `valid_at`; change `as_of`; reorder triples without changing semantics; relabel blank nodes
without changing semantics; add a duplicate statement; change one supplied rule; materialise an
inference with no certificate row; carry a certificate row for an inference never materialised;
truncate the selection at a limit; inject a temporally invalid graph into the selected dataset; and
present a manifest whose derived fields disagree with the canonical bytes.

The last one is the one an implementer forgets, because it is the test that the checker recomputes
rather than trusts.

## Order of work

Adjusted from the obvious order, because the canonical-bytes machinery is already built.

1. `InputManifest`, split into its derived and attested halves.
2. `CertifiedInput`, immutable, owning canonical bytes.
3. The certified reasoner accepts only `CertifiedInput`, with no second path to the store. This lands
   with the type-level gating, not after it: a verified core the engine can route around is
   decorative.
4. Generalise `pack.rs`'s canonical serialisation from N-Triples to N-Quads and move it somewhere
   both packs and certificates use. This is mostly reuse.
5. Bind the certificate header to the input, rules and scope digests, and make the checker recompute
   every derived field.
6. The adversarial tests above.
7. Kani on the boundary: mutation between phases, missing and duplicated records, counter mismatches,
   partial writes, truncation, hash-input framing ambiguity.
8. Aeneas on the pure kernel: scope membership over parsed metadata, canonical record construction,
   deterministic sorting, digest preimage construction, manifest construction, and equality between
   the reasoner's input and the certificate's input. Not Oxigraph, and not the hash primitive. Prove
   the correct bytes are constructed and handed to a conventional implementation.
9. Attestation, only under the condition in the section above.

## The premise was wrong and the decision survives it

*Added 19 September 2026, closing #181.*

The paragraph above rested on a claim about this repository's own dependency, and the claim did not
hold. `oxigraph = "0.5"` is pinned; `Store::start_transaction` is public and the crate documents it
as repeatable read. A decision record resting on a false statement about its dependency is worth
correcting whether or not the decision changes. This one does not change, and the reason it does not
is a cost the original text never weighed, because it believed there was nothing to weigh.

Three things were measured rather than argued. `tests/transaction_isolation_test.rs` and
`tests/transaction_memory_test.rs` are the measurements, and they are gates: each carries the
assertion that would fail if a future Oxigraph changed the answer, worded so that the fix is to
re-argue this section rather than to relax the assertion.

**The two storage backends do not behave alike.** This is the finding that matters, because
`GraphStore::new` builds the in-memory one and almost everything in this codebase uses it.

| | in memory (`Store::new`) | on disk (`Store::open`) |
|---|---|---|
| a writer, while a read transaction is open | **blocked until the reader commits** | proceeds |
| the reader's view, meanwhile | stable | stable |

In memory, a concurrent `insert` does not return while a read transaction is open. It is a lock and
not a deadlock: the writer proceeds the moment the reader commits, and a test asserts exactly that,
because "blocked forever" and "blocked for the duration" are different defects. The block is inside
`MemoryStorage::start_transaction`, reached from `Store::insert`, which opens a transaction of its
own. On disk the same probe shows the writer completing while the reader's repeated scan returns
the same count, which is what repeatable read is supposed to feel like.

**A read-only transaction costs no memory.** Oxigraph warns that "the transaction keeps the complete
set of changes into memory". That is a warning about writes. Over 20,000 triples, measuring NET LIVE
bytes with a counting global allocator, one run on macOS in a debug build:

| | net live bytes |
|---|---|
| held open, read-only transaction | **0** |
| copied out, the owned value this decision builds | **4,417,076** |

about 221 bytes per triple for the copy, against nothing for the transaction. The transaction's zero
is net rather than churn: the scan allocates and frees, and keeps nothing, which is the claim that
matters here.

Those two constants are from one run and are not gated. What is gated is the inequality between
them, by a factor of four in each direction, which is the shape the argument rests on and is not
going to move with an allocator or a platform. A test that pinned 4,417,076 would fail on Linux for
no reason anybody cares about, and would then be relaxed rather than read.

**So the trade is the opposite way round from the one the original text implied.** It is not
"copying is the only honest option". It is: copying spends about 221 bytes per triple in order to
hold an exclusive window that is short rather than one that lasts the whole reasoning run. On the
in-memory store a certified run that held its transaction open would stall every writer for as long
as it reasoned, and a reasoning run is not bounded by anything the reader controls. Copy once,
release, and reason on an owned value.

**Phantoms do not occur, measured rather than inferred from the isolation level's name.**
Repeatable read permits phantoms in general: a second evaluation of the same PATTERN may return rows
a concurrent transaction inserted. The probe repeats one pattern scan twenty times while a writer
commits between every pair of scans, into the very pattern being scanned, on the on-disk store
because that is the only one where a writer can commit at all while a reader is open. Every scan
returned the first count. The probe also asserts that the writer committed something, so a run in
which the writer never got going fails rather than passing as a clean result.

**The route this issue suggested for TCB-6 and TCB-7 does not exist in Oxigraph 0.5.** The
suggestion was that a transaction plus a recorded store version would turn "the assertions the
engine reasoned over" from a claim about an EXECUTION into a claim about a VALUE, which a checker
could confirm. There is no version to record: `Store` exposes no revision, sequence number or
snapshot handle, and the `Storage::snapshot` that exists internally is not reachable from the public
API. So `store revision` stays in the attested half of the manifest, where this decision already put
it, and TCB-6 and TCB-7 stay irreducible for this dependency at this version. That is settled, not
deferred; it becomes live again only if Oxigraph exposes one.

**Neither option is good enough, so the dependency is being changed.** The choice the original text
thought it faced was between a copy and nothing. The choice it actually faced is between a copy that
spends 221 bytes a triple and a transaction that stops the store for the length of a reasoning run.
Both are bad, and they are bad for the same reason: Oxigraph 0.5 exposes no way to pin a read state
without also taking a lock, even though `Storage::snapshot` does exactly that internally and is what
`Store::len` and `Store::quads_for_pattern` already call for every single read.

So the fix is a patch to Oxigraph rather than a better sentence here. `Store::snapshot` returns a
read view pinned to one state that takes no lock, measured as:

| | pins the state | writers keep running | memory over 20,000 triples |
|---|---|---|---|
| transaction, in memory | yes | **no, all blocked** | 0 |
| the copy this decision makes | yes | yes | 4,417,076 bytes |
| `Store::snapshot`, patched in | **yes** | **yes** | **0** |

The reader is shared through an `Arc` rather than cloned per call, because
`rocksdb_wrapper::Reader` owns a `*mut rocksdb_readoptions_t` and frees it in `Drop`; a derived
`Clone` would double free. Oxigraph's own store suite passes against the patch.

That work is a separate change and is not in this one, which corrects the record and leaves the copy
in place. When the snapshot lands, the copy goes and this section is what says why it was ever
there.

## Consequences

TCB-8 stops being an informal promise and becomes a testable boundary. The cost is a copy of the
selected assertions per certified run, which is the price of the input being a value, and it is worth
paying because the alternative is a certificate that cannot say what it is about.
