# 0017. A pack carries its own proof, and the receiver re-runs it

**Status:** accepted, 28 September 2026.

## The failure this prevents

A receiver trusting the sender's engine because the sender's engine said it was fine.

`onto_pack` wrote a graph, a `sha256` over it, and an `evidence` block holding the lint and enforce
results recorded at pack time. `onto_unpack` compared that one digest and loaded. Every word in
`evidence` was an ASSERTION by the sender's engine, and nothing on the receiving side re-derived any
of it. The receiver verified integrity, which is a statement about bytes, and read soundness off a
report, which is a statement about a machine they were not running.

That is the shape this project exists to refuse everywhere else. `onto_reason` already writes a
certificate the Lean checker in `lean/` verifies, and `verdict.rs` already makes a certified word
unspellable without an exit code behind it. The artefact meant for promotion between environments
was the one place where the evidence was still a claim.

## The decision

A pack may carry the derivation certificate for the run that produced its graph. `onto_unpack` writes
those files out and runs THIS machine's checker over them. The verdict is the receiver's, produced by
`projection_entailment::run_checker`, which is the only function in the crate that can mint a
`verdict::Certified`. The sender's engine is out of the trusted set for that one claim.

Seven words, and none of them collapses into another:

| verdict | what happened | loads? |
| --- | --- | --- |
| `certificate_accepted` | `oo-cert` accepted it under `OOCert.certificate_sound` | yes |
| `certificate_accepted_under_supplied_rules` | `oo-horn` accepted it over a table the SENDER supplied | yes |
| `certificate_refused` | the checker named a step it would not accept | **no** |
| `certificate_unreadable` | exit 2, an unknown file name, or files that form no certificate | **no** |
| `certificate_not_about_this_pack` | the checker accepted it and its premises are not all in the graph | **no** |
| `checker_absent_nothing_was_checked` | no checker on this machine, or the caller turned the check off | yes |
| `no_certificate_in_pack` | the pack carries no proof, which every pack written before this does | yes |

The four outcomes a receiver has to tell apart are the first, the third, the sixth and the seventh,
and the sixth is the one that matters. "We did not check" must never render as "it passed", so the
absent case has its own word, carries no theorem, and is asserted in
`tests/pack_certificate_test.rs` by looking for the accepted word in the WHOLE serialised report and
requiring it to be absent. The wording follows `web/try/api/index.py`, which says the release carries
no checker rather than implying a check happened.

## Five choices, and why each went the way it did

**A refused certificate blocks the load.** A graph whose own proof does not check is a defect, not a
graph to promote. `load_even_if_refused` is the escape, so a receiver can still open the artefact and
look at it. This changes `onto_unpack` behaviour only for packs that carry a certificate, so no pack
written before this exists is affected.

**A pack carries ONE certificate, and the field is singular in the format.** A real promotion may
involve several runs, an RDFS pass and then a supplied Horn table, and making this a list is cheap now
and expensive later. It stays singular because a second certificate raises a question nobody can
answer honestly yet: what the COMBINED verdict is when one is accepted and the other refused. Until
there is an answer to that, a pack that needs two proofs is two packs.

**`pack` refuses a certificate that is not about the graph.** The sender learns at pack time rather
than the auditor learning at unpack time. The receiver refuses it too, because the receiver must not
depend on the sender having checked. The cost is that `onto_pack` can now fail where it used to
succeed, on a mistake that is easy to make: reason over one store, load another, pack.

**The CLI asymmetry is intended.** `unpack` reaches the CLI and the batch executor; `pack` stays
MCP-only. The receiver is the party this feature exists for, and requiring them to run an MCP client
to audit a pack would defeat it. The sender is already in a session.

**The certificate files travel as text, not base64.** A pack is meant to be diffable, which is the
same reason the graph is sorted N-Triples. The cost is escaped tabs and a larger file.

## The binding, stated precisely

`sha256` still covers the graph text and nothing else, so a reader who checked packs before this
existed is checking the same thing. It cannot see a swapped certificate, and a swapped certificate is
the whole attack: a certificate that is internally sound over SOMEBODY ELSE'S assertions checks
green, so a receiver verifying only the graph checksum would accept a true proof about premises that
are not in the pack.

`content_sha256` is the digest of, in order: the tag `oo-pack-content/1\n`, the length and bytes of
the graph text, a presence byte for the certificate and, when one is present, the length-prefixed
`format`, the length-prefixed `profile_claimed_by_sender`, the `asserted` and `derivations` counts,
the number of files, then every (name, body) pair in `BTreeMap` order with both halves
length-prefixed. Every part is length-prefixed so no two different (graph, certificate) pairs can
frame to the same bytes by running together at a seam. It is written out explicitly rather than taken
from `serde_json::to_string`, so a serde_json release cannot change what a security digest covers.

A pack that carries a certificate and no `content_sha256` is REFUSED rather than checked. Without
that rule the downgrade is free: strip the field, paste in a certificate sound over another graph,
and the old, weaker verification passes it.

## What an acceptance does not settle

A certificate proves that the materialised triples follow from the asserted ones under the rules that
RAN. It proves nothing about whether the asserted triples are true, that they describe the world,
that they came from the system the sender says they came from, or that the sender left nothing out.
It is also silent about axioms outside the fragment: a profile run evaluates part of OWL 2 RL, a
supplied table is whatever `rules.tsv` holds, and an axiom no rule fires on contributed nothing and
is invisible. `onto_dlp_boundary` over the loaded graph is what answers that second question.

Those sentences are in the RECEIVER'S REPORT, under `does_not_mean`, and not only in this file. A
limit stated in documentation is a limit the person reading the verdict does not see.

## Two limits that are named rather than closed

**The receiver's checker is still a binary somebody chose.** Residual hole 1 in `src/verdict.rs`
applies unchanged: `$OO_CERT` pointing at a script that exits zero and prints the right theorem name
earns the accepted word. That is now the RECEIVER'S own foot rather than the sender's, which is a
real improvement and not a closure. The report prints the `checker` block the binary says about
itself, so a reader can match its SHA-256 against a release's `SHASUMS.txt`, and a hostile binary can
print the same block with any numbers in it.

**Pack size.** The certificate travels as JSON strings and the whole pack is built in memory, with no
streaming path. Measured on 28 September 2026: `benchmark/reference/ies4.ttl`, 249 KB of Turtle and
3,976 asserted triples, reasoned under `owl-rl-ext` with materialisation, packs to 4,514,931 bytes,
of which 3,015,113 are the certificate and 1,415,791 the graph.
`benchmark/reference/pizza-reference.owl`, 241 KB and 2,332 triples, packs to 1,422,749 bytes with a
843,870-byte certificate. That is roughly 490 bytes of pack JSON per packed triple on the first, so a
1.9M-triple store is of the order of a gigabyte held in memory twice. `onto_pack` reports
`graph_bytes`, the certificate's `bytes` and `pack_json_bytes` so the cost is visible before a
receiver discovers it.

## What a pack is still not

`serialize("ntriples")` drops the graph name, so a pack of a store that reasoned with
`inference_graph: true` loses the asserted and inferred separation that decision 0001 is about. The
certificate is then the only thing in the pack that says which triples were derived. That was already
true of packs and the certificate makes it legible rather than making it worse, but a pack is not a
faithful copy of a quad store. N-Quads is the follow-up.
