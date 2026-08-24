# Studio public port and UI rebuild

Design, 24 August 2026.

## Context

Open Ontologies Studio exists today only as a build produced from an internal branch. The
binary installed at `/Applications/Open Ontologies Studio.app` is v0.1.0, ad hoc signed,
arm64 thin, with the engine bundled as an external binary. It was not compiled from this
repository, and the public tree has never produced a Studio build.

That internal branch also carries a document-to-ontology pipeline that this repository does
not have: read a document corpus, derive per-document ontology fragments, merge them,
reconcile competing modelling patterns, refine individual typings, verify, then scan for
contradictions that split along provenance. The pipeline works end to end and its findings
are already recorded as reusable results.

The immediate occasion is the SEMANTiCS 2026 Practitioners Track, which takes contributions
until 29 August 2026, notifies on 3 September, and runs on site in Ghent from 15 to 17
September. The track is non archival. It scores quality of description and demo video,
originality and relevance, maturity and demonstrated functionality, and logistics for on
site presentation. It asks explicitly whether reviewers and attendees can test the tool
themselves.

Today they cannot, and that is a defect independent of any conference.

## Goals

1. Bring the Studio frontend, the Tauri shell additions, the agent sidecar and the
   document-to-ontology pipeline into this repository under its existing MIT licence.
2. Rebuild the user interface over a data source abstraction rather than reskinning the
   current panels.
3. Ship a precomputed demonstration built entirely from open data, so the demo path makes no
   model call and requires no API key, no network and no Node runtime.
4. Make the tool genuinely installable and testable by someone who is not the author.

## Non goals

Compiling the engine to WebAssembly. Changing the engine beyond the five lines noted below.
Any refactor not required by the four goals above.

## What moves

| Component | Location | Note |
| --- | --- | --- |
| Pipeline | `demo/` | chunker, tokenisation, extraction, corpus pipeline, `reconcile()`, `refine()`, contradiction scan, verifier |
| Agent sidecar | `studio/src-tauri/sidecars/agent/` | split into `mcp.ts`, `workflows.ts`, `graphrag.ts` and `providers/` |
| Providers | `providers/` | `types.ts`, `anthropic.ts`, `openai.ts`, `claudecli.ts`, `alias.ts` |
| Corpus commands | `studio/src-tauri/src/corpus.rs` | new file in this tree |
| Configurable engine port | `studio/src-tauri/src/engine.rs`, `src/mcp.rs` | see Fixes |
| Alignment change | `src/align.rs` | five insertions |

Measured delta from the branch point: 46 files and 4,474 insertions across the Studio tree
and sidecar, 43 commits, plus 123 files in the pipeline directory. The engine sees only two
changes: the five insertions in `align.rs` and the port configurability in `src/mcp.rs`
described under Fixes.

## What does not move

The internal documentation set, in full. The hosted gateway provider. The existing corpus and
every artifact derived from it. The model alias indirection. Any string identifying where the
internal branch was deployed or for whom, in code, commit messages, README, article, video or
submission form.

## Fixes carried out during the port

These are defects in the current code, not consequences of the move. They are listed here
because the port is the moment they get fixed.

1. `studio/src-tauri/src/chat.rs:61` resolves the sidecar as
   `env!("CARGO_MANIFEST_DIR").join("sidecars/agent")`, a compile time absolute path. It must
   resolve from Tauri's resource directory instead, and the sidecar must be added to
   `bundle.resources`, which is currently unset. Until this changes, the chat feature works
   only on a machine holding the exact source checkout the binary was compiled from, which
   means it can never work for anyone who installs the app.
2. `studio/src-tauri/src/chat.rs:27` hardcodes `/opt/homebrew/bin/node`. Node must be
   discovered, and its absence must produce a clear message rather than a silent failure.
3. The engine port is hardcoded to 8080 upstream and `clear_stale_port(8080)` runs on
   startup, which kills any local model server listening there. The port becomes
   configurable through `OPEN_ONTOLOGIES_STUDIO_PORT`, defaulting to 8137.

## Architecture

The rebuild is organised around one interface with two implementations.

```ts
interface DemoSource {
  corpus():   Document[]
  graph():    { classes, properties, edges }
  findings(): Contradiction[]
  resolve(id: string, decision: Decision): void
  ask(question: string): AsyncIterable<Chunk>
}
```

`LiveSource` speaks MCP to the bundled engine and drives the provider sidecar. It is what the
Tauri desktop application boots.

`ReplaySource` reads committed JSON artifacts. It is what the static web build boots.
`resolve` applies its effect to in memory state and appends to a session ledger that is
discarded on reload. `ask` returns scripted responses keyed by question.

Every component consumes `DemoSource` and nothing else. Neither implementation is aware of
the other. The build target selects which one is constructed, at exactly one place in the
application entry point.

This is the reason to rebuild rather than reskin. The current panels reach the engine
directly, so no amount of restyling produces a version anyone else can run.

## Corpus and narrative

The demonstration corpus is DCAT-US, assembled from the vendored sources already present and
already byte reproducible in the `dcat-us-binding` repository.

The story the pipeline tells is a disagreement between documents that all claim authority.
The project README states the profile is an implementation of the W3C DCAT standard. The 115
published examples expand to 76 triples, one predicate, and no DCAT at all. PR #120 deleted
the SHACL definition, leaving a JSON Schema with no published JSON-LD context and therefore
no defined RDF interpretation.

The contradiction scan splits those claims by provenance and reconstructs that sequence. This
is the same shape as the change record beat the pipeline already handles, so the pipeline is
being pointed at a new corpus rather than asked to do a new thing.

The demonstration on stage is a refusal. The agent proposes something plausible, the engine
declines it, and the refusal concerns a national metadata standard the audience publishes
against. This matches the conference theme of curated versus induced semantics directly.

## Precompute and verification

`make demo` regenerates every derived artifact from the vendored sources, writes a manifest
hash, and the artifacts are committed. Continuous integration recomputes the hash and fails
on mismatch, so the replay can never silently drift from the pipeline that produced it.

The pipeline verifier ports across unchanged as the gate: six checks, exit code equal to the
number of failures. It runs before anything is recorded or demonstrated. The engine's own
vocabulary check runs inside the pipeline as it does today.

No model call sits anywhere on the demonstration path.

## Schedule

| Date | Work |
| --- | --- |
| 25 August | Corpus assembly, pipeline port, headless run, artifacts committed |
| 26 August | `DemoSource` abstraction, `ReplaySource`, static web build passing |
| 27 August | New interface shell over both sources |
| 28 August | Sidecar and Node fixes, Tauri build from this tree, record the video |
| 29 August | Submit |

## Risks

The interface rebuild sits on the critical path by explicit decision, so 27 August carries no
slack. If it slips, the fallback is to ship the ported panels unstyled, because the video
needs only the refusal to be legible.

The Tauri build has never been run from this tree. The toolchain is proven, since a build
exists, but this specific checkout is unverified. It is scheduled for 28 August and should be
attempted once on 25 August as a smoke test.

Corpus assembly is the item most likely to surprise, which is why it is first.

Local `main` is currently five commits behind `origin/main` with unrelated modifications in
the heritage aerial case study. Those are not part of this work and are not to be staged with
it.
