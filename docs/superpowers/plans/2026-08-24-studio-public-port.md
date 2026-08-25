# Studio Public Port and Interface Rebuild Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring the Studio frontend, Tauri shell additions, agent sidecar and
document-to-ontology pipeline into this repository, rebuild the interface over a data source
abstraction, and ship a precomputed demonstration built from the DCAT-US corpus that runs with
no model call, no API key and no network.

**Architecture:** One React codebase behind a `DemoSource` interface with two
implementations. `LiveSource` speaks MCP to the bundled engine and drives the provider
sidecar; the Tauri desktop application boots it. `ReplaySource` reads committed JSON
artifacts; the static web build boots it. Components consume `DemoSource` and nothing else,
and the build target selects the implementation at a single construction site. The Python
pipeline produces the artifacts `ReplaySource` reads, and a manifest hash checked in
continuous integration keeps the two from drifting apart.

**Tech Stack:** Rust 2021 with Oxigraph, Tauri 2, React 19, TypeScript, Vite, Tailwind,
zustand, 3d-force-graph, Node 20 or later for the sidecar, Python 3.11 or later with pytest
for the pipeline, vitest for the frontend.

## PIVOT, 25 August 2026: the demonstration changes, the port does not

Task 8 ran the document-to-ontology pipeline over the DCAT-US corpus and produced **zero
contradictions**, confirmed three independent ways. The diagnosis is a shape mismatch in the
corpus choice, not a defect in the pipeline or the model.

The contradiction scanner detects provenance-split typing conflicts: two documents typing the
same individual incompatibly. DCAT-US's disagreement is not that shape. It is a claim against
evidence, a README asserting conformance that the published artifacts do not exhibit. Measured
extraction per document makes this concrete: the three JSON Schemas yielded 11, 17 and 19
classes, while `profile-readme.md`, `w3c-dcat-conformance.md`, `pr-120-record.md` and
`recovered-shapes.ttl` each yielded zero. The disagreement lives entirely in the documents that
extracted nothing.

**The demonstration is therefore no longer the contradiction scan.** It is the validator finding
already established and filed against the profile: 115 published examples expanding to 76
triples with one predicate and no DCAT at all, and 287 SHACL violations once the binding the
schema already implies is actually applied. That finding needs no model, is reproducible from
committed inputs, and states the thesis more sharply than the pipeline ever would: a standard
that cannot demonstrate the conformance it claims.

What this changes:

- **Task 8** is superseded. `demo/precomputed/findings.json` is empty and must be regenerated
  from the validator run rather than the pipeline. The `corpus`, `graph` and `compare` artifacts
  remain useful and stay.
- **Tasks 9, 10, 11** are unaffected. The `DemoSource` shape does not change; only what fills
  `findings()` changes.
- **Task 12 and 12A** are unaffected in structure. The findings panel now cites shapes and
  example files rather than disagreeing prose documents.
- **The pipeline port itself stands.** Tasks 5, 6, 6A and 6B remain correct and committed. The
  pipeline is a real capability of this repository; it simply is not what this particular
  demonstration shows.
- **The comparison from Task 8 is retained and must stay honest.** On one of its five questions
  the grounded path hallucinated a publisher name while the plain baseline answered correctly
  from source. That result is not to be quietly dropped. Either show it, or drop the comparison
  surface entirely; do not curate it into a clean win.

## Global Constraints

- The source branch is referenced throughout as `$INTERNAL`. Set it once per shell:
  `export INTERNAL=/Users/fabio/projects/<internal-branch-checkout>`. Never write the literal
  path into any file in this repository.
- No file, comment, commit message, test fixture, artifact, README, article, video or
  submission field may identify the organisation the internal branch was prepared for, or any
  person, product or competitor named in its documentation. This is a hard gate on every task.
- Because this plan is itself committed to the public repository, it never spells out the
  terms being guarded against. Before starting, create an untracked pattern file at the
  repository root, one extended-regex term per line, covering the organisation, the client
  contacts, the competitor, the hosted gateway product, the model alias and the internal
  namespace and document prefixes:

  ```bash
  cd /Users/fabio/projects/open-ontologies
  printf '%s\n' <term> <term> ... > .identifiers-guard
  grep -qxF '.identifiers-guard' .gitignore || echo '.identifiers-guard' >> .gitignore
  ```

  Every task that screens for leaked identifiers runs `grep -rniEf .identifiers-guard`. Never
  commit `.identifiers-guard`, and never inline its contents into a tracked file.
- `docs/client/` from the internal branch does not move under any circumstances.
- The hosted gateway provider module under `providers/` does not move. Its filename is in the
  guard file, so the leak screens catch it if it is copied by mistake.
- No file under the internal branch's `demo/corpus/`, `demo/corpus_bio/`, `demo/biology/`,
  `demo/documents/`, `demo/documents_extra/`, `demo/derived/`, `demo/extracted/`,
  `demo/corpus_extracted/`, `demo/pdfs/`, `demo/data/` or `demo/ontology/` moves. Every
  artifact in this repository is regenerated from DCAT-US sources.
- Licence stays MIT, copyright Fabio Rovai. Every ported file keeps that.
- Prose in documentation and user-facing copy uses no em dashes.
- Never run `git add -A` or `git add .`. Stage explicit paths in every commit step.
- `main` is behind `origin/main` at plan time. Task 1 resolves this before anything else.

---

## Task 1: Rebase, then smoke-build Studio from this tree

The Tauri shell has never been compiled from this repository. Proving it builds is worth more
on day one than any feature, because every later task depends on it and the failure mode is
unbounded.

**Files:**
- Modify: none
- Test: manual build

**Interfaces:**
- Consumes: nothing
- Produces: a verified `studio/src-tauri/target/` and a working `npm run tauri build`

- [ ] **Step 1: Rebase local work onto origin**

```bash
cd /Users/fabio/projects/open-ontologies
git stash push -m "wip heritage-aerial" case-studies/heritage-aerial
git pull --rebase origin main
git stash pop
git log --oneline -3
```

Expected: the spec commit sits on top of the five upstream commits, and the heritage aerial
modifications return to the working tree unstaged.

- [ ] **Step 2: Install frontend dependencies**

```bash
cd /Users/fabio/projects/open-ontologies/studio
npm ci
```

Expected: completes without error. If `npm ci` fails on a lockfile mismatch, use
`npm install` and commit the updated `package-lock.json` in Step 5.

- [ ] **Step 3: Build the engine binary the bundler expects**

```bash
cd /Users/fabio/projects/open-ontologies
cargo build --release
node studio/scripts/prepare-engine.mjs
ls -la studio/src-tauri/binaries/
```

Expected: a target-triple-suffixed `open-ontologies` binary in `studio/src-tauri/binaries/`.
Tauri requires the triple suffix; if `prepare-engine.mjs` did not add one, note the exact
filename it produced and fix the script before continuing.

- [ ] **Step 4: Build the desktop application**

```bash
cd /Users/fabio/projects/open-ontologies/studio
npm run tauri build 2>&1 | tail -30
```

Expected: a bundle under `studio/src-tauri/target/release/bundle/`. This is the step most
likely to fail. Record every error and its fix in the commit message; do not work around a
failure by copying artifacts from `$INTERNAL`.

- [ ] **Step 5: Commit whatever the build required**

```bash
cd /Users/fabio/projects/open-ontologies
git add studio/package-lock.json studio/scripts/prepare-engine.mjs
git commit -m "build(studio): make the desktop bundle reproducible from this tree"
```

If the build needed no changes, skip the commit and record that fact in the task notes.

---

## Task 2: Add a frontend test runner

The frontend has no test configuration. Later tasks are test-driven and need one.

**Files:**
- Create: `studio/vitest.config.ts`
- Create: `studio/src/lib/__tests__/smoke.test.ts`
- Modify: `studio/package.json`

**Interfaces:**
- Consumes: nothing
- Produces: `npm test` runs vitest in `studio/`

- [ ] **Step 1: Install vitest**

```bash
cd /Users/fabio/projects/open-ontologies/studio
npm install -D vitest@^2 @vitest/coverage-v8@^2
```

- [ ] **Step 2: Write the config**

Create `studio/vitest.config.ts`:

```ts
import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts', 'src/**/*.test.tsx'],
  },
})
```

- [ ] **Step 3: Write a failing smoke test**

Create `studio/src/lib/__tests__/smoke.test.ts`:

```ts
import { describe, it, expect } from 'vitest'
import { runnerIsWired } from '../runner-check'

describe('test runner', () => {
  it('is wired up', () => {
    expect(runnerIsWired()).toBe(true)
  })
})
```

- [ ] **Step 4: Run it to verify it fails**

Add to `studio/package.json` scripts: `"test": "vitest run"`, then:

```bash
cd /Users/fabio/projects/open-ontologies/studio && npm test
```

Expected: FAIL, cannot resolve `../runner-check`.

- [ ] **Step 5: Write the minimal implementation**

Create `studio/src/lib/runner-check.ts`:

```ts
export function runnerIsWired(): boolean {
  return true
}
```

- [ ] **Step 6: Run it to verify it passes**

```bash
cd /Users/fabio/projects/open-ontologies/studio && npm test
```

Expected: PASS, 1 test.

- [ ] **Step 7: Commit**

```bash
cd /Users/fabio/projects/open-ontologies
git add studio/vitest.config.ts studio/package.json studio/package-lock.json \
        studio/src/lib/runner-check.ts studio/src/lib/__tests__/smoke.test.ts
git commit -m "test(studio): add vitest and a runner smoke test"
```

---

## Task 3: Resolve the sidecar from the bundle instead of a compile-time path

`studio/src-tauri/src/chat.rs:61` builds the sidecar path from `env!("CARGO_MANIFEST_DIR")`,
which bakes the compiling machine's checkout into the binary. Any installed copy looks for the
sidecar at a path that does not exist on the installing machine, so the chat feature can only
ever work for whoever compiled it. `bundle.resources` in `tauri.conf.json` is unset, so the
sidecar is not packaged at all.

**Files:**
- Modify: `studio/src-tauri/src/chat.rs:60-70`
- Modify: `studio/src-tauri/tauri.conf.json`
- Test: `studio/src-tauri/src/chat.rs` unit test

**Interfaces:**
- Consumes: nothing
- Produces: `fn sidecar_entry(app: &tauri::AppHandle) -> Result<PathBuf, String>`

- [ ] **Step 1: Write the failing test**

Append to `studio/src-tauri/src/chat.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_fallback_points_at_the_source_sidecar() {
        let path = dev_sidecar_entry();
        assert!(
            path.ends_with("sidecars/agent/dist/index.js"),
            "unexpected dev sidecar path: {}",
            path.display()
        );
    }
}
```

- [ ] **Step 2: Run it to verify it fails**

```bash
cd /Users/fabio/projects/open-ontologies/studio/src-tauri
cargo test dev_fallback_points_at_the_source_sidecar 2>&1 | tail -20
```

Expected: FAIL, `cannot find function dev_sidecar_entry`.

- [ ] **Step 3: Implement resource-dir resolution with a dev fallback**

Replace the first two lines of `spawn_agent_sidecar` in `studio/src-tauri/src/chat.rs`:

```rust
fn dev_sidecar_entry() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sidecars/agent/dist/index.js")
}

fn sidecar_entry(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    if let Ok(dir) = app.path().resource_dir() {
        let bundled = dir.join("sidecars/agent/dist/index.js");
        if bundled.exists() {
            return Ok(bundled);
        }
    }
    let dev = dev_sidecar_entry();
    if dev.exists() {
        return Ok(dev);
    }
    Err(format!(
        "Agent sidecar not found. Looked in the app resource directory and at {}. \
         Run `npm run build` in studio/src-tauri/sidecars/agent to produce dist/index.js.",
        dev.display()
    ))
}

pub fn spawn_agent_sidecar(app: &tauri::AppHandle) -> Result<(), String> {
    let entry = sidecar_entry(app)?;
    let node = resolve_node_binary();

    let mut child = Command::new(&node)
        .arg(&entry)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("PATH", augmented_path())
        .spawn()
        .map_err(|e| {
            format!(
                "Failed to spawn agent sidecar with {}: {e}. Node 20 or later must be installed.",
                node.display()
            )
        })?;
```

Leave the remainder of `spawn_agent_sidecar` unchanged.

- [ ] **Step 4: Run it to verify it passes**

```bash
cd /Users/fabio/projects/open-ontologies/studio/src-tauri
cargo test dev_fallback_points_at_the_source_sidecar 2>&1 | tail -10
```

Expected: PASS.

- [ ] **Step 5: Package the sidecar**

In `studio/src-tauri/tauri.conf.json`, add to the `bundle` object, as a sibling of
`externalBin`:

```json
"resources": {
  "sidecars/agent/dist": "sidecars/agent/dist",
  "sidecars/agent/package.json": "sidecars/agent/package.json"
}
```

- [ ] **Step 6: Verify the sidecar ships**

```bash
cd /Users/fabio/projects/open-ontologies/studio
(cd src-tauri/sidecars/agent && npm ci && npx tsc)
npm run tauri build 2>&1 | tail -5
find src-tauri/target/release/bundle -path '*Resources/sidecars/agent/dist/index.js'
```

Expected: the find prints a path. An empty result means the resource mapping is wrong.

- [ ] **Step 7: Commit**

```bash
cd /Users/fabio/projects/open-ontologies
git add studio/src-tauri/src/chat.rs studio/src-tauri/tauri.conf.json
git commit -m "fix(studio): resolve the agent sidecar from the bundle, and package it

The sidecar path came from CARGO_MANIFEST_DIR, so an installed build looked
for it inside the compiling machine's checkout and never found it. Resolve
from the Tauri resource directory, fall back to the source tree for local
development, and add the built sidecar to bundle.resources so it ships."
```

---

## Task 4: Make the engine port configurable

`studio/src-tauri/src/engine.rs:123` calls `clear_stale_port(8080)` on startup, which kills any
process already listening on 8080. A local model server is the common case. The port is also
hardcoded at lines 129 and 142.

**Files:**
- Modify: `studio/src-tauri/src/engine.rs:115-145`
- Test: `studio/src-tauri/src/engine.rs` unit test

**Interfaces:**
- Consumes: nothing
- Produces: `fn engine_port() -> u16`, honouring `OPEN_ONTOLOGIES_STUDIO_PORT`, default 8137

- [ ] **Step 1: Write the failing test**

Append to `studio/src-tauri/src/engine.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_defaults_to_8137_and_never_to_8080() {
        assert_eq!(parse_port(None), 8137);
        assert_eq!(parse_port(Some("9001")), 9001);
        assert_eq!(parse_port(Some("not-a-port")), 8137);
    }
}
```

- [ ] **Step 2: Run it to verify it fails**

```bash
cd /Users/fabio/projects/open-ontologies/studio/src-tauri
cargo test port_defaults_to_8137 2>&1 | tail -20
```

Expected: FAIL, `cannot find function parse_port`.

- [ ] **Step 3: Implement**

Add to `studio/src-tauri/src/engine.rs`:

```rust
const DEFAULT_ENGINE_PORT: u16 = 8137;

fn parse_port(raw: Option<&str>) -> u16 {
    raw.and_then(|value| value.trim().parse::<u16>().ok())
        .filter(|port| *port != 0)
        .unwrap_or(DEFAULT_ENGINE_PORT)
}

fn engine_port() -> u16 {
    let raw = std::env::var("OPEN_ONTOLOGIES_STUDIO_PORT").ok();
    parse_port(raw.as_deref())
}
```

Then replace the three hardcoded uses. At line 123, `clear_stale_port(8080)` becomes:

```rust
let port = engine_port();
clear_stale_port(port);
```

At line 129, `.args(["serve-http", "--port", "8080"])` becomes:

```rust
.args(["serve-http", "--port", &port.to_string()])
```

At line 142, the readiness check becomes:

```rust
if line.contains("listening") || line.contains("Listening") || line.contains(&port.to_string())
```

- [ ] **Step 4: Run it to verify it passes**

```bash
cd /Users/fabio/projects/open-ontologies/studio/src-tauri
cargo test port_defaults_to_8137 2>&1 | tail -10
```

Expected: PASS, 3 assertions.

- [ ] **Step 5: Update the frontend's engine URL**

In `studio/src/lib/mcp-client.ts`, replace any hardcoded `8080` with a value read from
`import.meta.env.VITE_ENGINE_PORT ?? '8137'`. Add `VITE_ENGINE_PORT=8137` to `studio/.env`.

- [ ] **Step 6: Commit**

```bash
cd /Users/fabio/projects/open-ontologies
git add studio/src-tauri/src/engine.rs studio/src/lib/mcp-client.ts studio/.env
git commit -m "fix(studio): make the engine port configurable, default 8137

Startup called clear_stale_port(8080), which kills a local model server
listening on the same port. Read OPEN_ONTOLOGIES_STUDIO_PORT instead."
```

---

## Task 4A: Tighten the alignment classifier

The one engine change carried across. `classify_relation` currently returns `skos:exactMatch`
for any pair above 0.6 label similarity even when no structural evidence supports it, which
produces a wall of near-identical proposals and overstates what a bare name resemblance
justifies. Mid similarity on names alone warrants `skos:closeMatch`; only structural agreement
upgrades the claim.

**Files:**
- Modify: `src/align.rs:784-797`
- Test: `src/align.rs` unit test

**Interfaces:**
- Consumes: nothing
- Produces: unchanged signature
  `fn classify_relation(label_sim: f64, prop_overlap: f64, parent_overlap: f64) -> &'static str`

- [ ] **Step 1: Write the failing test**

Append to the test module in `src/align.rs`:

```rust
#[test]
fn claim_strength_tracks_evidence_strength() {
    // Strong name plus structural agreement is the only equivalence.
    assert_eq!(
        AlignmentEngine::classify_relation(0.9, 0.6, 0.0),
        "owl:equivalentClass"
    );
    // A strong name alone is an exact match.
    assert_eq!(AlignmentEngine::classify_relation(0.9, 0.1, 0.0), "skos:exactMatch");
    // Shared parents without a strong name is a subclass claim.
    assert_eq!(AlignmentEngine::classify_relation(0.3, 0.1, 0.7), "rdfs:subClassOf");
    // A middling name with no structural support must not claim exactness.
    assert_eq!(AlignmentEngine::classify_relation(0.7, 0.1, 0.1), "skos:closeMatch");
    assert_eq!(AlignmentEngine::classify_relation(0.61, 0.0, 0.0), "skos:closeMatch");
}
```

- [ ] **Step 2: Run it to verify it fails**

```bash
cd /Users/fabio/projects/open-ontologies
cargo test claim_strength_tracks_evidence_strength 2>&1 | tail -15
```

Expected: FAIL on the last two assertions, `assertion failed: left "skos:exactMatch", right
"skos:closeMatch"`.

- [ ] **Step 3: Remove the unsupported branch**

In `src/align.rs`, delete these two lines from `classify_relation`:

```rust
        } else if label_sim > 0.6 {
            "skos:exactMatch"
```

The chain then falls through to `skos:closeMatch`. Add the reasoning as a comment above the
function body so the next reader does not restore the branch:

```rust
        // The strength of the claim must track the strength of the evidence.
        // Handing exactMatch to every mid-similarity pair overstated what a bare
        // name resemblance justifies. Names alone at mid similarity warrant
        // closeMatch, and only structural agreement upgrades the claim.
```

- [ ] **Step 4: Run it to verify it passes**

```bash
cd /Users/fabio/projects/open-ontologies
cargo test claim_strength_tracks_evidence_strength 2>&1 | tail -10
```

Expected: PASS.

- [ ] **Step 5: Check nothing else depended on the old behaviour**

```bash
cd /Users/fabio/projects/open-ontologies && cargo test 2>&1 | tail -15
```

Expected: the full suite passes. If an alignment test now fails, read it before changing it.
A test asserting `exactMatch` at mid similarity was encoding the defect.

- [ ] **Step 6: Commit**

```bash
cd /Users/fabio/projects/open-ontologies
git add src/align.rs
git commit -m "fix(align): stop claiming exactMatch on name similarity alone

A pair above 0.6 label similarity was classified as skos:exactMatch with no
structural evidence, which overstated the claim and buried reviewers in
near-identical proposals. Mid similarity now yields skos:closeMatch, and only
property or parent agreement upgrades it."
```

---

## Task 5: Port the pipeline modules

Ten Python modules move. No corpus, no derived artifact and no ontology file moves with them.

**Files:**
- Create: `demo/chunker.py`, `demo/tokenisation.py`, `demo/extract.py`,
  `demo/corpus_pipeline.py`, `demo/corpus_text.py`, `demo/ontology_from_docs.py`,
  `demo/contradiction_scan.py`, `demo/verify.py`, `demo/kpi_context_graph.py`,
  `demo/cq/run-cross-doc.py`
- Create: `demo/README.md`
- Create: `demo/tests/test_reconcile.py`
- Create: `demo/requirements.txt`

**Interfaces:**
- Consumes: nothing
- Produces: `reconcile(graph) -> Graph`, `refine(graph, partitions) -> Graph`,
  `scan_contradictions(store) -> list[Contradiction]` where
  `Contradiction = {id, subject, claims: [{document, predicate, object}], kind}`

- [ ] **Step 1: Copy the modules and nothing else**

```bash
cd /Users/fabio/projects/open-ontologies
mkdir -p demo/cq demo/tests
for f in chunker.py tokenisation.py extract.py corpus_pipeline.py corpus_text.py \
         ontology_from_docs.py contradiction_scan.py verify.py kpi_context_graph.py; do
  cp "$INTERNAL/demo/$f" "demo/$f"
done
cp "$INTERNAL/demo/cq/run-cross-doc.py" demo/cq/run-cross-doc.py
ls demo
```

Expected: exactly the ten files plus the directories you created. If anything else appears,
delete it.

- [ ] **Step 2: Strip identifying content**

```bash
cd /Users/fabio/projects/open-ontologies
grep -rniEf .identifiers-guard demo/ || echo CLEAN
```

Every hit must be removed or renamed before proceeding. Replace any default corpus path with
`demo/corpus/dcat-us` and any default namespace with `https://w3id.org/dcat-us-demo#`. Re-run
until the grep prints `CLEAN`.

- [ ] **Step 3: Write a failing test for the reconciliation rule**

The rule that earns its place: when independent per-document derivation produces both a
subclass partition and a competing attribute class for the same notion, the attribute class
must be removed, except for Status and State, which are genuinely attributes.

Create `demo/tests/test_reconcile.py`:

```python
from rdflib import Graph, Namespace, RDFS, OWL, RDF

from demo.contradiction_scan import reconcile

EX = Namespace("https://example.org/t#")


def _graph(pairs):
    g = Graph()
    for sub, parent in pairs:
        g.add((sub, RDF.type, OWL.Class))
        g.add((sub, RDFS.subClassOf, parent))
    return g


def test_attribute_class_is_removed_when_a_partition_exists():
    g = _graph([(EX.PublishedDataset, EX.Dataset), (EX.DraftDataset, EX.Dataset)])
    g.add((EX.DatasetType, RDF.type, OWL.Class))
    out = reconcile(g)
    assert (EX.DatasetType, RDF.type, OWL.Class) not in out
    assert (EX.PublishedDataset, RDFS.subClassOf, EX.Dataset) in out


def test_status_is_spared_because_states_are_attributes():
    g = _graph([(EX.ActiveThing, EX.Thing), (EX.RetiredThing, EX.Thing)])
    g.add((EX.ThingStatus, RDF.type, OWL.Class))
    out = reconcile(g)
    assert (EX.ThingStatus, RDF.type, OWL.Class) in out
```

- [ ] **Step 4: Run it to verify it fails or passes**

```bash
cd /Users/fabio/projects/open-ontologies
python -m pip install -r demo/requirements.txt
python -m pytest demo/tests/test_reconcile.py -v
```

Expected: PASS if `reconcile` ported correctly. If it FAILS, the port lost behaviour and must
be fixed before continuing. This test exists to prove the port preserved the rule, so a failure
here is a real signal, not a formality.

- [ ] **Step 5: Write demo/requirements.txt**

```
rdflib>=7.0
presidio-analyzer>=2.2
pytest>=8.0
```

- [ ] **Step 6: Write demo/README.md**

One page: what the pipeline stages are, how to run `make demo`, what each derived artifact is,
and the statement that the corpus is public DCAT-US material with its provenance recorded. No
em dashes.

- [ ] **Step 7: Commit**

```bash
cd /Users/fabio/projects/open-ontologies
git add demo/chunker.py demo/tokenisation.py demo/extract.py demo/corpus_pipeline.py \
        demo/corpus_text.py demo/ontology_from_docs.py demo/contradiction_scan.py \
        demo/verify.py demo/kpi_context_graph.py demo/cq/run-cross-doc.py demo/README.md \
        demo/requirements.txt demo/tests/test_reconcile.py
git commit -m "feat(demo): add the document-to-ontology pipeline

Read, tokenise, derive per-document fragments, merge, reconcile competing
modelling patterns, refine individual typings, verify, then scan for
contradictions that split along provenance. Corpus and artifacts are
generated separately and are not part of this commit."
```

---

## Task 6: Port the agent sidecar split

**Files:**
- Create: `studio/src-tauri/sidecars/agent/mcp.ts`, `workflows.ts`, `graphrag.ts`
- Create: `studio/src-tauri/sidecars/agent/providers/{types,anthropic,openai,claudecli,alias}.ts`
- Modify: `studio/src-tauri/sidecars/agent/index.ts`
- Modify: `studio/src-tauri/sidecars/agent/package.json`

**Interfaces:**
- Consumes: nothing
- Produces: `interface Provider { name: string; chat(messages, tools): AsyncIterable<Chunk> }`
  from `providers/types.ts`, and `selectProvider(env): Provider` from `providers/alias.ts`

- [ ] **Step 1: Copy, excluding the gateway provider**

```bash
cd /Users/fabio/projects/open-ontologies/studio/src-tauri/sidecars/agent
mkdir -p providers
for f in mcp.ts workflows.ts graphrag.ts index.ts; do cp "$INTERNAL/studio/src-tauri/sidecars/agent/$f" .; done
for f in types.ts anthropic.ts openai.ts claudecli.ts alias.ts; do
  cp "$INTERNAL/studio/src-tauri/sidecars/agent/providers/$f" providers/
done
ls providers
```

Expected: five files. The hosted gateway provider must not be present. `acl.ts` and `baseline.ts` are not
copied; they are not referenced by the demonstration path.

- [ ] **Step 2: Remove every reference to the excluded provider**

```bash
cd /Users/fabio/projects/open-ontologies/studio/src-tauri/sidecars/agent
grep -rniEf ../../../../.identifiers-guard . --exclude-dir=node_modules --exclude-dir=dist || echo CLEAN
```

Remove the import and the switch arm in `providers/alias.ts`. Re-run until `CLEAN`. The alias
indirection that hides a model behind a pseudonym is removed entirely: `selectProvider` returns
the provider named by `ONTO_PROVIDER`, defaulting to `anthropic`.

- [ ] **Step 3: Compile**

```bash
cd /Users/fabio/projects/open-ontologies/studio/src-tauri/sidecars/agent
npm ci && npx tsc --noEmit
```

Expected: no errors. Type errors here mean the copy dropped a dependency; add it to
`package.json` rather than deleting the code that needs it.

- [ ] **Step 4: Build the bundle the Tauri resource mapping expects**

```bash
cd /Users/fabio/projects/open-ontologies/studio/src-tauri/sidecars/agent
npx tsc && ls dist
```

Expected: `index.js` plus the compiled modules.

- [ ] **Step 5: Commit**

```bash
cd /Users/fabio/projects/open-ontologies
git add studio/src-tauri/sidecars/agent/mcp.ts studio/src-tauri/sidecars/agent/workflows.ts \
        studio/src-tauri/sidecars/agent/graphrag.ts studio/src-tauri/sidecars/agent/index.ts \
        studio/src-tauri/sidecars/agent/providers studio/src-tauri/sidecars/agent/package.json \
        studio/src-tauri/sidecars/agent/package-lock.json
git commit -m "feat(studio): split the agent sidecar into engine, workflow and provider layers

Two providers behind one interface, selected by ONTO_PROVIDER: the Anthropic
SDK with native tool use, and any OpenAI-compatible endpoint including a
local server. No model client lives in the engine."
```

---

## Task 6A: Port the corpus Tauri commands

`corpus.rs` is the shell-side plumbing the interface panels in Task 12 depend on. It does not
exist in this tree.

**Files:**
- Create: `studio/src-tauri/src/corpus.rs`
- Modify: `studio/src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: nothing
- Produces: eight Tauri commands, invoked from the frontend by name:
  `corpus_presets() -> Vec<(String, String, usize)>`,
  `ingest_corpus(...) -> Result<String, String>`,
  `read_store() -> Result<String, String>`,
  `list_graphs() -> Vec<String>`,
  `read_decisions() -> String`,
  `revert_type(doc, subject, from, to) -> Result<(), String>`,
  `list_saved() -> Vec<String>`,
  `pick_ontology_file() -> Option<String>`

- [ ] **Step 1: Copy the module**

```bash
cd /Users/fabio/projects/open-ontologies
cp "$INTERNAL/studio/src-tauri/src/corpus.rs" studio/src-tauri/src/corpus.rs
```

- [ ] **Step 2: Replace the corpus presets**

`corpus_presets` returns the demonstration corpora offered in the interface, and its current
entries name the internal corpus. Replace the returned vector with the public one:

```rust
#[tauri::command]
pub fn corpus_presets() -> Vec<(String, String, usize)> {
    vec![(
        "dcat-us".to_string(),
        "DCAT-US 3.0 profile documents and the W3C DCAT conformance clause".to_string(),
        6,
    )]
}
```

The third element is the document count and must match what Task 7 actually produces. If Task
7 yields a different count, update this number rather than leaving it wrong.

- [ ] **Step 3: Strip every remaining identifier**

```bash
cd /Users/fabio/projects/open-ontologies
grep -niEf .identifiers-guard studio/src-tauri/src/corpus.rs || echo CLEAN
```

Every hit is a reference to the internal corpus. Remove or replace each one. Re-run until the
grep prints `CLEAN`.

- [ ] **Step 4: Register the commands**

In `studio/src-tauri/src/lib.rs`, add `mod corpus;` and extend the existing
`tauri::generate_handler!` list with the eight command names above, keeping the existing
entries.

- [ ] **Step 5: Compile**

```bash
cd /Users/fabio/projects/open-ontologies/studio/src-tauri && cargo build 2>&1 | tail -15
```

Expected: builds clean. A missing-path error means `repo_root()` resolves against a layout this
tree does not have; fix it to resolve from the Tauri resource directory as Task 3 did, not by
recreating the internal layout.

- [ ] **Step 6: Commit**

```bash
cd /Users/fabio/projects/open-ontologies
git add studio/src-tauri/src/corpus.rs studio/src-tauri/src/lib.rs
git commit -m "feat(studio): add corpus ingest, store and decision-ledger commands

Shell-side commands the corpus and resolution panels call: list presets,
ingest a corpus, read the store and the decisions ledger, revert a typing
decision, and pick an ontology file."
```

---

## Task 6B: Port the access control, roles and governance layer

Four files implementing document-level access control, role-based views of the corpus, and the
governance panel that surfaces both. Added to the plan on 24 August at the owner's request, so
that every feature the internal branch introduced reaches the public tree.

**Files:**
- Create: `studio/src-tauri/sidecars/agent/acl.ts`
- Create: `demo/acl_normalise.py`
- Create: `studio/src/lib/roles.ts`
- Create: `studio/src/components/GovernancePanel.tsx`
- Create: `studio/src/lib/__tests__/roles.test.ts`

**Interfaces:**
- Consumes: the sidecar layering from Task 6, the pipeline from Task 5
- Produces: whatever `roles.ts` exports for resolving a role to the documents it may see.
  Record the exact exported signatures in your report, because Task 12 wires the panel and
  needs them.

- [ ] **Step 1: Copy the four files**

```bash
cd /Users/fabio/projects/open-ontologies
cp "$INTERNAL/studio/src-tauri/sidecars/agent/acl.ts" studio/src-tauri/sidecars/agent/acl.ts
cp "$INTERNAL/demo/acl_normalise.py" demo/acl_normalise.py
cp "$INTERNAL/studio/src/lib/roles.ts" studio/src/lib/roles.ts
cp "$INTERNAL/studio/src/components/GovernancePanel.tsx" studio/src/components/GovernancePanel.tsx
```

- [ ] **Step 2: Strip every identifier**

```bash
cd /Users/fabio/projects/open-ontologies
grep -niEf .identifiers-guard studio/src-tauri/sidecars/agent/acl.ts demo/acl_normalise.py \
  studio/src/lib/roles.ts studio/src/components/GovernancePanel.tsx || echo CLEAN
```

Role names, document identifiers and example users in these files come from the internal
corpus. Replace them with roles and documents that make sense for a public standards corpus,
for example an editor who sees every document and a reader who sees only published ones.
Re-run until the grep prints `CLEAN`.

- [ ] **Step 3: Write the failing test**

Read the actual exports of `roles.ts` first, then write `studio/src/lib/__tests__/roles.test.ts`
against them. The test must cover the security-relevant behaviour, not the happy path alone:

- a role granted access to a subset of documents sees exactly that subset
- a role with no grants sees nothing, rather than everything
- an unknown role name is denied, rather than defaulting to permitted

The second and third cases are the ones that matter. An access control layer that fails open is
worse than none, because it looks like it is working.

- [ ] **Step 4: Run it to verify it fails**

```bash
cd /Users/fabio/projects/open-ontologies/studio && npm test -- roles
```

Expected: FAIL. If it passes immediately, confirm you are actually importing the real module and
not asserting something trivially true.

- [ ] **Step 5: Make it pass**

If the ported code already satisfies the test, say so in your report and move on. If it fails
open on an unknown role or an empty grant list, fix it. That is a real defect, not a porting
artifact.

- [ ] **Step 6: Compile everything**

```bash
cd /Users/fabio/projects/open-ontologies/studio && npx tsc --noEmit
cd src-tauri/sidecars/agent && npx tsc --noEmit
cd /Users/fabio/projects/open-ontologies && python -m pytest demo/tests -q
```

Expected: all three clean.

- [ ] **Step 7: Commit**

```bash
cd /Users/fabio/projects/open-ontologies
git add studio/src-tauri/sidecars/agent/acl.ts demo/acl_normalise.py studio/src/lib/roles.ts \
        studio/src/components/GovernancePanel.tsx studio/src/lib/__tests__/roles.test.ts
git commit -m "feat(studio): add document access control, roles and the governance panel

Documents carry access grants, roles resolve to the set a viewer may see, and
the governance panel surfaces both. Access is denied by default: an unknown
role or an empty grant list sees nothing."
```

---

## Task 7: Assemble the DCAT-US corpus

This is the task most likely to surprise, which is why it runs first on the calendar. The
corpus is prose and schema documents, not RDF. The disagreement the pipeline must find is
between what the profile claims and what its own artifacts do.

**Files:**
- Create: `demo/corpus/dcat-us/*.md`, `demo/corpus/dcat-us/*.json`
- Create: `demo/corpus/dcat-us/MANIFEST.json`
- Create: `demo/fetch_corpus.py`

**Interfaces:**
- Consumes: nothing
- Produces: `demo/corpus/dcat-us/` containing at least six documents and a manifest recording
  the source URL, retrieval date and SHA-256 of each

- [ ] **Step 1: Fetch upstream**

```bash
cd /Users/fabio/projects/dcat-us-binding && make upstream && ls
```

Inspect what arrived. The documents wanted are the profile README with its W3C DCAT
conformance claim, the JSON Schema, a representative sample of the published examples, the
recovered SHACL shapes at `vendor/dcat-us_3.0_shacl_shapes.recovered.ttl`, the record of the
pull request that deleted the shapes, and the relevant section of the W3C DCAT v3
specification defining conformance.

- [ ] **Step 2: Write the fetcher**

Create `demo/fetch_corpus.py`. The retrieval date is an argument, not `datetime.now()`, so
reruns are byte reproducible:

```python
"""Assemble the public DCAT-US document corpus with a provenance manifest."""
import argparse
import hashlib
import json
import shutil
from pathlib import Path

# (destination filename, source path relative to the upstream checkout, source URL)
SOURCES = [
    ("profile-readme.md", "README.md",
     "https://github.com/GSA/dcat-us/blob/main/README.md"),
    ("dataset-schema.json", "schemas/dataset.json",
     "https://github.com/GSA/dcat-us/blob/main/schemas/dataset.json"),
    ("catalog-schema.json", "schemas/catalog.json",
     "https://github.com/GSA/dcat-us/blob/main/schemas/catalog.json"),
    ("examples.json", "examples/catalog.json",
     "https://github.com/GSA/dcat-us/blob/main/examples/catalog.json"),
    ("recovered-shapes.ttl", None,
     "https://github.com/GSA/dcat-us/pull/120"),
    ("w3c-dcat-conformance.md", None,
     "https://www.w3.org/TR/vocab-dcat-3/#conformance"),
]


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--upstream", type=Path,
                    default=Path("/Users/fabio/projects/dcat-us-binding"))
    ap.add_argument("--out", type=Path, default=Path("demo/corpus/dcat-us"))
    ap.add_argument("--retrieved", required=True, help="ISO date, e.g. 2026-08-25")
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    manifest = []
    for name, rel, url in SOURCES:
        dest = args.out / name
        if rel is not None:
            src = args.upstream / "upstream" / rel
            if not src.exists():
                raise SystemExit(f"missing upstream file: {src}. Run `make upstream` first.")
            shutil.copyfile(src, dest)
        elif name == "recovered-shapes.ttl":
            shutil.copyfile(
                args.upstream / "vendor" / "dcat-us_3.0_shacl_shapes.recovered.ttl", dest
            )
        elif not dest.exists():
            raise SystemExit(
                f"{dest} must be saved by hand from {url} before running this script"
            )
        manifest.append({
            "file": name,
            "source_url": url,
            "retrieved": args.retrieved,
            "sha256": sha256(dest),
        })

    (args.out / "MANIFEST.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"{len(manifest)} documents recorded in {args.out / 'MANIFEST.json'}")


if __name__ == "__main__":
    main()
```

The upstream paths in `SOURCES` are the expected layout. Verify them against what `make
upstream` actually produced in Step 1 and correct the tuples before running, rather than
letting the script fail one file at a time.

- [ ] **Step 3: Run it**

```bash
cd /Users/fabio/projects/open-ontologies
python demo/fetch_corpus.py --retrieved 2026-08-25
ls demo/corpus/dcat-us && python -c "import json;print(len(json.load(open('demo/corpus/dcat-us/MANIFEST.json'))))"
```

Expected: at least six documents and a matching manifest length.

- [ ] **Step 4: Verify the disagreement is actually present**

```bash
cd /Users/fabio/projects/open-ontologies
grep -ril "implementation of the World Wide Web Consortium" demo/corpus/dcat-us/
grep -l "shacl" demo/corpus/dcat-us/* 2>/dev/null
```

Expected: the conformance claim appears in at least one document, and the shapes are present
as a separate document. If the claim is not in the corpus, the demonstration has no
contradiction to find and the corpus selection must be revised before continuing.

- [ ] **Step 5: Commit**

```bash
cd /Users/fabio/projects/open-ontologies
git add demo/fetch_corpus.py demo/corpus/dcat-us
git commit -m "feat(demo): add the DCAT-US document corpus with provenance manifest

Public material from the profile repository and the W3C specification, each
file recorded with its source URL, retrieval date and checksum."
```

---

## Task 8: Precompute, with a manifest hash checked in CI

**Files:**
- Create: `demo/precomputed/*.json`
- Create: `demo/precomputed/MANIFEST.sha256`
- Create: `demo/bundle_fixtures.py`
- Modify: `Makefile`
- Create: `.github/workflows/demo-artifacts.yml`

**Interfaces:**
- Consumes: `demo/corpus/dcat-us/`
- Produces: `demo/precomputed/{corpus,graph,findings,chat,compare}.json` and the combined
  `demo/precomputed/bundle.json`, whose shape is exactly the `ReplayFixtures` type defined in
  Task 9

- [ ] **Step 0: Write the fixture bundler**

`ReplaySource` loads one object, so the four artifacts are combined into one file. Create
`demo/bundle_fixtures.py`:

```python
"""Combine the pipeline's four artifacts into the single object ReplaySource loads."""
import argparse
import json
from pathlib import Path

PARTS = ("corpus", "graph", "findings", "chat", "compare")


def bundle(indir: Path) -> dict:
    out = {}
    for part in PARTS:
        path = indir / f"{part}.json"
        if not path.exists():
            raise SystemExit(f"missing artifact: {path}")
        out[part] = json.loads(path.read_text(encoding="utf-8"))
    return out


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--in", dest="indir", required=True, type=Path)
    ap.add_argument("--out", dest="outfile", required=True, type=Path)
    args = ap.parse_args()
    payload = bundle(args.indir)
    args.outfile.write_text(
        json.dumps(payload, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(f"bundled {len(payload['findings'])} findings into {args.outfile}")


if __name__ == "__main__":
    main()
```

`sort_keys` and a fixed indent keep the output byte stable across runs, which is what makes the
manifest hash meaningful.

- [ ] **Step 1: Add the make target**

Append to `Makefile`:

```make
.PHONY: demo demo-verify
demo:
	python demo/corpus_pipeline.py --corpus demo/corpus/dcat-us --out demo/precomputed
	python demo/contradiction_scan.py --in demo/precomputed --out demo/precomputed/findings.json
	python demo/bundle_fixtures.py --in demo/precomputed --out demo/precomputed/bundle.json
	cd demo/precomputed && shasum -a 256 *.json | sort > MANIFEST.sha256

demo-verify:
	python demo/verify.py --in demo/precomputed
	cd demo/precomputed && shasum -a 256 -c MANIFEST.sha256
```

- [ ] **Step 2: Run the pipeline**

```bash
cd /Users/fabio/projects/open-ontologies && make demo && ls demo/precomputed
```

Expected: six JSON files including `bundle.json`, plus `MANIFEST.sha256`. This step calls a
model. It is the only step in the entire plan that does, and its output is committed so that
nothing downstream ever repeats it.

- [ ] **Step 3: Run the verifier**

```bash
cd /Users/fabio/projects/open-ontologies && make demo-verify; echo "exit=$?"
```

Expected: `exit=0`. A non-zero exit equals the number of failed checks and must be resolved
before anything is recorded.

- [ ] **Step 4: Confirm the contradiction was found**

```bash
python -c "
import json
f = json.load(open('demo/precomputed/findings.json'))
print(len(f), 'findings')
for x in f[:3]:
    print(x['subject'], '->', [c['document'] for c in x['claims']])
"
```

Expected: at least one finding whose claims cite two or more distinct documents. Zero findings
means the pipeline ran but the demonstration has no content, and Task 7's corpus selection
must be revisited.

- [ ] **Step 5: Add the CI check**

Create `.github/workflows/demo-artifacts.yml`:

```yaml
name: demo artifacts
on: [push, pull_request]
jobs:
  manifest:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Verify committed artifacts match their manifest
        run: cd demo/precomputed && shasum -a 256 -c MANIFEST.sha256
```

- [ ] **Step 6: Commit**

```bash
cd /Users/fabio/projects/open-ontologies
git add Makefile demo/precomputed .github/workflows/demo-artifacts.yml
git commit -m "feat(demo): precompute the demonstration artifacts and pin them by hash

The replay path reads these files, so no model call sits anywhere on the
demonstration path. CI recomputes the manifest so the artifacts cannot
silently drift from the pipeline that produced them."
```

---

## Task 9: DemoSource interface and ReplaySource

**Files:**
- Create: `studio/src/lib/demo-source.ts`
- Create: `studio/src/lib/replay-source.ts`
- Create: `studio/src/lib/__tests__/replay-source.test.ts`

**Interfaces:**
- Consumes: `demo/precomputed/*.json` from Task 8
- Produces: the `DemoSource` interface and `createReplaySource(fixtures): DemoSource`

- [ ] **Step 1: Define the interface**

Create `studio/src/lib/demo-source.ts`:

```ts
export interface Document { id: string; title: string; text: string }
export interface Claim { document: string; predicate: string; object: string }
export interface Contradiction {
  id: string
  subject: string
  kind: 'provenance-split' | 'disjointness' | 'typing'
  claims: Claim[]
}
export interface GraphView {
  classes: { iri: string; label?: string }[]
  properties: { iri: string; label?: string }[]
  edges: { source: string; target: string }[]
}
export type Decision = { kind: 'accept' | 'reject'; note?: string }
export interface Chunk { type: 'text' | 'tool_call'; value: string }

export interface DemoSource {
  corpus(): Promise<Document[]>
  graph(): Promise<GraphView>
  findings(): Promise<Contradiction[]>
  resolve(id: string, decision: Decision): Promise<void>
  ask(question: string): AsyncIterable<Chunk>
}
```

- [ ] **Step 2: Write the failing test**

Create `studio/src/lib/__tests__/replay-source.test.ts`:

```ts
import { describe, it, expect } from 'vitest'
import { createReplaySource } from '../replay-source'

const fixtures = {
  corpus: [{ id: 'README', title: 'README', text: 'an implementation of the W3C DCAT standard' }],
  graph: { classes: [{ iri: 'ex:Dataset' }], properties: [], edges: [] },
  findings: [
    {
      id: 'f1',
      subject: 'ex:conformance',
      kind: 'provenance-split' as const,
      claims: [
        { document: 'README', predicate: 'claims', object: 'dcat-conformant' },
        { document: 'examples', predicate: 'yields', object: 'zero-dcat-triples' },
      ],
    },
  ],
  chat: { 'what disagrees?': [{ type: 'text' as const, value: 'README and examples disagree.' }] },
}

describe('ReplaySource', () => {
  it('returns the committed findings', async () => {
    const src = createReplaySource(fixtures)
    const found = await src.findings()
    expect(found).toHaveLength(1)
    expect(found[0].claims.map((c) => c.document)).toEqual(['README', 'examples'])
  })

  it('records a resolution in session state without mutating fixtures', async () => {
    const src = createReplaySource(fixtures)
    await src.resolve('f1', { kind: 'accept' })
    expect(await src.findings()).toHaveLength(1)
    expect(fixtures.findings).toHaveLength(1)
  })

  it('streams a scripted answer', async () => {
    const src = createReplaySource(fixtures)
    const out: string[] = []
    for await (const chunk of src.ask('what disagrees?')) out.push(chunk.value)
    expect(out.join('')).toContain('disagree')
  })

  it('answers unknown questions without throwing', async () => {
    const src = createReplaySource(fixtures)
    const out: string[] = []
    for await (const chunk of src.ask('unscripted question')) out.push(chunk.value)
    expect(out.join('')).not.toHaveLength(0)
  })
})
```

- [ ] **Step 3: Run it to verify it fails**

```bash
cd /Users/fabio/projects/open-ontologies/studio && npm test -- replay-source
```

Expected: FAIL, cannot resolve `../replay-source`.

- [ ] **Step 4: Implement**

Create `studio/src/lib/replay-source.ts`:

```ts
import type { Chunk, Contradiction, Decision, DemoSource, Document, GraphView } from './demo-source'

export interface ReplayFixtures {
  corpus: Document[]
  graph: GraphView
  findings: Contradiction[]
  chat: Record<string, Chunk[]>
}

const UNSCRIPTED: Chunk[] = [
  {
    type: 'text',
    value:
      'This is the offline replay of the demonstration. Only the scripted questions are answered here. Run the desktop application against the engine for a live session.',
  },
]

export function createReplaySource(fixtures: ReplayFixtures): DemoSource {
  const findings = fixtures.findings.map((f) => ({ ...f, claims: [...f.claims] }))
  const ledger: { id: string; decision: Decision }[] = []

  return {
    async corpus() {
      return fixtures.corpus
    },
    async graph() {
      return fixtures.graph
    },
    async findings() {
      return findings
    },
    async resolve(id, decision) {
      ledger.push({ id, decision })
    },
    async *ask(question) {
      const scripted = fixtures.chat[question.trim().toLowerCase()] ?? fixtures.chat[question]
      for (const chunk of scripted ?? UNSCRIPTED) yield chunk
    },
  }
}
```

- [ ] **Step 5: Run it to verify it passes**

```bash
cd /Users/fabio/projects/open-ontologies/studio && npm test -- replay-source
```

Expected: PASS, 4 tests.

- [ ] **Step 6: Commit**

```bash
cd /Users/fabio/projects/open-ontologies
git add studio/src/lib/demo-source.ts studio/src/lib/replay-source.ts \
        studio/src/lib/__tests__/replay-source.test.ts
git commit -m "feat(studio): add the DemoSource interface and its replay implementation"
```

---

## Task 10: LiveSource over the existing MCP client

**Files:**
- Create: `studio/src/lib/live-source.ts`
- Create: `studio/src/lib/__tests__/live-source.test.ts`
- Modify: `studio/src/lib/mcp-client.ts`

**Interfaces:**
- Consumes: `DemoSource` from Task 9. `mcp-client.ts` exports free functions, not a client
  object: `sparqlQuery(query: string): Promise<string>` returning raw SPARQL JSON results, and
  `callTool(name: string, args?: Record<string, unknown>): Promise<string>`.
- Produces: `createLiveSource(deps: LiveDeps): DemoSource` where
  `LiveDeps = { sparqlQuery: typeof sparqlQuery; callTool: typeof callTool }`

Dependencies are injected rather than imported so the implementation is testable without a
running engine.

- [ ] **Step 1: Write the failing test with injected fakes**

Create `studio/src/lib/__tests__/live-source.test.ts`:

```ts
import { describe, it, expect, vi } from 'vitest'
import { createLiveSource } from '../live-source'

function sparqlJson(bindings: Record<string, string>[]): string {
  return JSON.stringify({
    results: {
      bindings: bindings.map((row) =>
        Object.fromEntries(Object.entries(row).map(([k, v]) => [k, { value: v }])),
      ),
    },
  })
}

describe('LiveSource', () => {
  it('maps the class and subclass queries into a GraphView', async () => {
    const sparqlQuery = vi.fn(async (q: string) =>
      q.includes('rdfs:subClassOf ?b')
        ? sparqlJson([{ a: 'ex:Dataset', b: 'ex:Resource' }])
        : sparqlJson([{ c: 'ex:Dataset', l: 'Dataset' }]),
    )
    const callTool = vi.fn(async () => '[]')
    const src = createLiveSource({ sparqlQuery, callTool })

    const g = await src.graph()
    expect(g.classes).toContainEqual({ iri: 'ex:Dataset', label: 'Dataset' })
    expect(g.edges).toContainEqual({ source: 'ex:Dataset', target: 'ex:Resource' })
  })

  it('surfaces an engine error rather than returning an empty graph', async () => {
    const sparqlQuery = vi.fn(async () => {
      throw new Error('engine not listening')
    })
    const callTool = vi.fn(async () => '[]')
    const src = createLiveSource({ sparqlQuery, callTool })

    await expect(src.graph()).rejects.toThrow('engine not listening')
  })

  it('parses findings returned by the contradiction tool', async () => {
    const sparqlQuery = vi.fn(async () => sparqlJson([]))
    const callTool = vi.fn(async () =>
      JSON.stringify([
        {
          id: 'f1',
          subject: 'ex:conformance',
          kind: 'provenance-split',
          claims: [
            { document: 'README', predicate: 'claims', object: 'conformant' },
            { document: 'examples', predicate: 'yields', object: 'zero-triples' },
          ],
        },
      ]),
    )
    const src = createLiveSource({ sparqlQuery, callTool })

    const found = await src.findings()
    expect(found).toHaveLength(1)
    expect(found[0].claims.map((c) => c.document)).toEqual(['README', 'examples'])
  })
})
```

The second test matters most. An engine failure that silently renders an empty graph is
indistinguishable on stage from an ontology that genuinely has nothing in it.

- [ ] **Step 2: Run it to verify it fails**

```bash
cd /Users/fabio/projects/open-ontologies/studio && npm test -- live-source
```

Expected: FAIL, cannot resolve `../live-source`.

- [ ] **Step 3: Implement**

Create `studio/src/lib/live-source.ts`. The two SPARQL queries are the ones `Graph3D` already
issues at `studio/src/components/Graph3D.tsx:89-90`, moved here so the component stops talking
to the engine directly:

```ts
import type { Chunk, Contradiction, Decision, DemoSource, Document, GraphView } from './demo-source'
import { sparqlQuery, callTool } from './mcp-client'

export interface LiveDeps {
  sparqlQuery: typeof sparqlQuery
  callTool: typeof callTool
}

const PREFIXES = `PREFIX owl: <http://www.w3.org/2002/07/owl#>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>`

const CLASSES = `${PREFIXES}
SELECT ?c ?l WHERE {
  { ?c a owl:Class } UNION { ?c rdfs:subClassOf ?x }
  OPTIONAL { ?c rdfs:label ?l }
  FILTER(!isBlank(?c))
} LIMIT 300`

const SUBCLASSES = `${PREFIXES}
SELECT ?a ?b WHERE {
  ?a rdfs:subClassOf ?b .
  FILTER(!isBlank(?a) && !isBlank(?b))
}`

function rows(raw: string): Record<string, string>[] {
  const parsed = JSON.parse(raw)
  const bindings = parsed?.results?.bindings ?? []
  return bindings.map((b: Record<string, { value: string }>) =>
    Object.fromEntries(Object.entries(b).map(([k, v]) => [k, v.value])),
  )
}

export function createLiveSource(deps: LiveDeps): DemoSource {
  return {
    async corpus(): Promise<Document[]> {
      return JSON.parse(await deps.callTool('corpus_documents'))
    },

    async graph(): Promise<GraphView> {
      const [classRows, subRows] = await Promise.all([
        deps.sparqlQuery(CLASSES).then(rows),
        deps.sparqlQuery(SUBCLASSES).then(rows),
      ])
      return {
        classes: classRows.map((r) => (r.l ? { iri: r.c, label: r.l } : { iri: r.c })),
        properties: [],
        edges: subRows.map((r) => ({ source: r.a, target: r.b })),
      }
    },

    async findings(): Promise<Contradiction[]> {
      return JSON.parse(await deps.callTool('onto_contradiction_scan'))
    },

    async resolve(id: string, decision: Decision): Promise<void> {
      await deps.callTool('onto_apply', { finding: id, decision: decision.kind })
      await deps.callTool('onto_save')
    },

    async *ask(question: string): AsyncIterable<Chunk> {
      const answer = await deps.callTool('agent_ask', { question })
      yield { type: 'text', value: answer }
    },
  }
}

export const liveSource = () => createLiveSource({ sparqlQuery, callTool })
```

No error is caught here. Every failure propagates to the caller, which is the point of the
second test.

- [ ] **Step 4: Run it to verify it passes**

```bash
cd /Users/fabio/projects/open-ontologies/studio && npm test -- live-source
```

Expected: PASS, 2 tests.

- [ ] **Step 5: Commit**

```bash
cd /Users/fabio/projects/open-ontologies
git add studio/src/lib/live-source.ts studio/src/lib/__tests__/live-source.test.ts \
        studio/src/lib/mcp-client.ts
git commit -m "feat(studio): add the live DemoSource implementation over MCP"
```

---

## Task 11: Static web build target

**Files:**
- Modify: `studio/vite.config.ts`
- Create: `studio/src/lib/source-factory.ts`
- Create: `studio/src/lib/__tests__/source-factory.test.ts`
- Modify: `studio/package.json`

**Interfaces:**
- Consumes: Tasks 9 and 10
- Produces: `getDemoSource(): Promise<DemoSource>`, and `npm run build:web`

- [ ] **Step 1: Write the failing test**

```ts
import { describe, it, expect } from 'vitest'
import { chooseSourceKind } from '../source-factory'

describe('source selection', () => {
  it('replays when the build target says web', () => {
    expect(chooseSourceKind({ VITE_DEMO_MODE: 'replay' })).toBe('replay')
  })
  it('goes live by default', () => {
    expect(chooseSourceKind({})).toBe('live')
  })
})
```

- [ ] **Step 2: Run it to verify it fails**

```bash
cd /Users/fabio/projects/open-ontologies/studio && npm test -- source-factory
```

Expected: FAIL.

- [ ] **Step 3: Implement**

```ts
import type { DemoSource } from './demo-source'
import { createReplaySource, type ReplayFixtures } from './replay-source'
import { liveSource } from './live-source'

export function chooseSourceKind(env: Record<string, string | undefined>): 'live' | 'replay' {
  return env.VITE_DEMO_MODE === 'replay' ? 'replay' : 'live'
}

export async function getDemoSource(): Promise<DemoSource> {
  if (chooseSourceKind(import.meta.env as never) === 'replay') {
    const response = await fetch('./precomputed/bundle.json')
    if (!response.ok) {
      throw new Error(`Could not load the precomputed demonstration: ${response.status}`)
    }
    const fixtures = (await response.json()) as ReplayFixtures
    return createReplaySource(fixtures)
  }
  return liveSource()
}
```

- [ ] **Step 4: Run it to verify it passes**

```bash
cd /Users/fabio/projects/open-ontologies/studio && npm test -- source-factory
```

Expected: PASS, 2 tests.

- [ ] **Step 5: Add the web build**

Add to `studio/package.json` scripts:

```json
"build:web": "VITE_DEMO_MODE=replay tsc && VITE_DEMO_MODE=replay vite build --base ./ --outDir dist-web"
```

Add a step that copies `demo/precomputed/*.json` into `dist-web/precomputed/` as a single
`bundle.json`.

- [ ] **Step 6: Verify it serves with no engine running**

```bash
cd /Users/fabio/projects/open-ontologies/studio
npm run build:web && npx vite preview --outDir dist-web --port 4173
```

Open the preview with no engine process running. The corpus, graph and findings must all
render. If anything is empty, `ReplaySource` is not wired at the construction site.

- [ ] **Step 7: Commit**

```bash
cd /Users/fabio/projects/open-ontologies
git add studio/vite.config.ts studio/package.json studio/src/lib/source-factory.ts \
        studio/src/lib/__tests__/source-factory.test.ts
git commit -m "feat(studio): add a static web build that replays the precomputed demonstration

Anyone can open the result without installing the desktop application, an
API key, a model or a Node runtime."
```

---

## Task 12: Rebuild the interface shell

**Files:**
- Create: `studio/src/AppShell.tsx`
- Create: `studio/src/components/CorpusPanel.tsx`, `FindingsPanel.tsx`, `ResolutionPanel.tsx`
- Modify: `studio/src/App.tsx`, `studio/src/components/Graph3D.tsx`
- Create: `studio/src/state/demo-store.ts`

**Interfaces:**
- Consumes: `getDemoSource()` from Task 11
- Produces: a shell whose components take data as props and never import a source directly

- [ ] **Step 1: Write the store**

This is the only module in the frontend that touches a source. Create
`studio/src/state/demo-store.ts`:

```ts
import { create } from 'zustand'
import type { Contradiction, Decision, Document, GraphView } from '../lib/demo-source'
import { getDemoSource } from '../lib/source-factory'

interface DemoState {
  documents: Document[]
  graph: GraphView
  findings: Contradiction[]
  selectedFinding: string | null
  ledger: { id: string; decision: Decision }[]
  error: string | null
  loading: boolean
  load: () => Promise<void>
  select: (id: string | null) => void
  resolve: (id: string, decision: Decision) => Promise<void>
}

const EMPTY_GRAPH: GraphView = { classes: [], properties: [], edges: [] }

export const useDemoStore = create<DemoState>((set, get) => ({
  documents: [],
  graph: EMPTY_GRAPH,
  findings: [],
  selectedFinding: null,
  ledger: [],
  error: null,
  loading: false,

  async load() {
    set({ loading: true, error: null })
    try {
      const source = await getDemoSource()
      const [documents, graph, findings] = await Promise.all([
        source.corpus(),
        source.graph(),
        source.findings(),
      ])
      set({ documents, graph, findings, loading: false })
    } catch (e) {
      // Surfaced, never swallowed. An empty graph and a dead engine must not look alike.
      set({ error: e instanceof Error ? e.message : String(e), loading: false })
    }
  },

  select(id) {
    set({ selectedFinding: id })
  },

  async resolve(id, decision) {
    try {
      const source = await getDemoSource()
      await source.resolve(id, decision)
      set({ ledger: [...get().ledger, { id, decision }] })
    } catch (e) {
      set({ error: e instanceof Error ? e.message : String(e) })
    }
  },
}))
```

- [ ] **Step 2: Build the findings panel**

Panels take data and callbacks as props and import no source, so each is testable without an
engine. Create `studio/src/components/FindingsPanel.tsx`:

```tsx
import type { Contradiction } from '../lib/demo-source'

export interface FindingsPanelProps {
  findings: Contradiction[]
  selected: string | null
  onSelect: (id: string) => void
}

export function FindingsPanel({ findings, selected, onSelect }: FindingsPanelProps) {
  if (findings.length === 0) {
    return <p className="p-4 text-sm opacity-70">No contradictions in this corpus.</p>
  }
  return (
    <ul className="divide-y">
      {findings.map((f) => (
        <li
          key={f.id}
          onClick={() => onSelect(f.id)}
          className={`cursor-pointer p-3 ${selected === f.id ? 'bg-amber-100/10' : ''}`}
        >
          <div className="font-mono text-sm">{f.subject}</div>
          <div className="text-xs opacity-70">{f.kind}</div>
          <ul className="mt-2 space-y-1 text-xs">
            {f.claims.map((c, i) => (
              <li key={i}>
                <span className="font-semibold">{c.document}</span> {c.predicate} {c.object}
              </li>
            ))}
          </ul>
        </li>
      ))}
    </ul>
  )
}
```

Naming the disagreeing documents on every finding is the whole point of the panel. A count
without citations is not evidence.

- [ ] **Step 3: Build the corpus and resolution panels**

`studio/src/components/CorpusPanel.tsx` takes
`{ documents: Document[]; onOpen: (id: string) => void }` and renders one row per document
showing its id and title, with the provenance from `MANIFEST.json` in a title attribute.

`studio/src/components/ResolutionPanel.tsx` takes
`{ finding: Contradiction | null; ledger: { id: string; decision: Decision }[]; onResolve:
(id: string, decision: Decision) => void }`. When a finding is selected it renders the two
competing claims as `A =? B` with an accept button and a reject button, both calling
`onResolve`. Below that it lists the ledger entries in order.

- [ ] **Step 4: Wire Graph3D to the store**

`Graph3D` issues its own SPARQL at `studio/src/components/Graph3D.tsx:89-90`. Those two
queries now live in `live-source.ts` from Task 10, so delete them here and read `graph` from
the store instead:

```tsx
const graph = useDemoStore((s) => s.graph)
```

Its rendering, camera behaviour and interaction handling stay exactly as they are. This is the
change that makes the same component work in the replay build, where there is no engine to
query.

- [ ] **Step 5: Verify both targets still render**

```bash
cd /Users/fabio/projects/open-ontologies/studio
npm test && npm run build:web && npm run tauri build 2>&1 | tail -5
```

Expected: tests pass, both builds succeed. Then open the web preview with no engine running
and confirm the findings panel still lists citations. If it is empty there but populated in the
desktop application, a component is still reaching the engine directly.

- [ ] **Step 6: Commit**

```bash
cd /Users/fabio/projects/open-ontologies
git add studio/src/AppShell.tsx studio/src/App.tsx studio/src/state/demo-store.ts \
        studio/src/components/CorpusPanel.tsx studio/src/components/FindingsPanel.tsx \
        studio/src/components/ResolutionPanel.tsx studio/src/components/Graph3D.tsx
git commit -m "feat(studio): rebuild the interface over the DemoSource abstraction

Components take data as props and no longer reach the engine directly, so the
same tree renders live against the engine and offline from the precomputed
artifacts."
```

---

## Task 12A: Port the baseline comparison

Added on 24 August at the owner's request. This is the strongest demonstration asset in the
whole port: it answers the same question twice, once grounded in the ontology and once by a
plain baseline, and shows them side by side. A room full of people who already believe
retrieval is enough is exactly the audience for that comparison, and it directly addresses the
published counterargument that pre-built graphs are unnecessary for retrieval.

**Files:**
- Create: `studio/src-tauri/sidecars/agent/baseline.ts`
- Create: `studio/src/components/ComparePanel.tsx`
- Create: `studio/src/lib/compare-source.ts`
- Create: `studio/src/lib/__tests__/compare-source.test.ts`

**Interfaces:**
- Consumes: `ReplayFixtures` from Task 9, the sidecar from Task 6, the `compare` artifact
  written into `bundle.json` by Task 8
- Produces:

```ts
export interface CompareResult {
  question: string
  grounded: { answer: string; citations: string[] }
  baseline: { answer: string; citations: string[] }
  divergence: string | null
}
export interface CompareSource {
  compare(question: string): Promise<CompareResult>
}
```

`CompareSource` is deliberately a separate interface rather than a fifth method on
`DemoSource`. It keeps Tasks 9 through 11 untouched, and the comparison is an optional demo
surface rather than part of the core loop.

- [ ] **Step 1: Copy the baseline runner**

```bash
cd /Users/fabio/projects/open-ontologies
cp "$INTERNAL/studio/src-tauri/sidecars/agent/baseline.ts" studio/src-tauri/sidecars/agent/baseline.ts
cp "$INTERNAL/studio/src/components/ComparePanel.tsx" studio/src/components/ComparePanel.tsx
grep -niEf .identifiers-guard studio/src-tauri/sidecars/agent/baseline.ts \
  studio/src/components/ComparePanel.tsx || echo CLEAN
```

Re-run the grep until it prints `CLEAN`.

- [ ] **Step 2: Write the failing test**

Create `studio/src/lib/__tests__/compare-source.test.ts`:

```ts
import { describe, it, expect } from 'vitest'
import { createReplayCompareSource } from '../compare-source'

const fixture = {
  'does this profile implement W3C DCAT?': {
    question: 'does this profile implement W3C DCAT?',
    grounded: {
      answer: 'No. The published examples expand to 76 triples and no DCAT predicates.',
      citations: ['examples.json', 'w3c-dcat-conformance.md'],
    },
    baseline: {
      answer: 'Yes. The README states it is an implementation of the W3C DCAT standard.',
      citations: ['profile-readme.md'],
    },
    divergence: 'The baseline repeats the claim. The grounded answer checks it against the artifacts.',
  },
}

describe('ReplayCompareSource', () => {
  it('returns both answers with their citations', async () => {
    const src = createReplayCompareSource(fixture)
    const r = await src.compare('does this profile implement W3C DCAT?')
    expect(r.grounded.citations).toContain('examples.json')
    expect(r.baseline.citations).toEqual(['profile-readme.md'])
    expect(r.divergence).not.toBeNull()
  })

  it('reports an unscripted question rather than fabricating a comparison', async () => {
    const src = createReplayCompareSource(fixture)
    const r = await src.compare('something nobody scripted')
    expect(r.divergence).toBeNull()
    expect(r.grounded.answer).toMatch(/not scripted|offline replay/i)
  })
})
```

The second test is the one that matters. A comparison panel that invents a difference when it
has no data would be exactly the failure mode the demonstration argues against.

- [ ] **Step 3: Run it to verify it fails**

```bash
cd /Users/fabio/projects/open-ontologies/studio && npm test -- compare-source
```

Expected: FAIL, cannot resolve `../compare-source`.

- [ ] **Step 4: Implement**

Create `studio/src/lib/compare-source.ts`:

```ts
export interface CompareResult {
  question: string
  grounded: { answer: string; citations: string[] }
  baseline: { answer: string; citations: string[] }
  divergence: string | null
}

export interface CompareSource {
  compare(question: string): Promise<CompareResult>
}

export type CompareFixtures = Record<string, CompareResult>

export function createReplayCompareSource(fixtures: CompareFixtures): CompareSource {
  return {
    async compare(question: string): Promise<CompareResult> {
      const hit = fixtures[question] ?? fixtures[question.trim().toLowerCase()]
      if (hit) return hit
      return {
        question,
        grounded: {
          answer: 'This question is not scripted in the offline replay.',
          citations: [],
        },
        baseline: { answer: '', citations: [] },
        divergence: null,
      }
    },
  }
}
```

- [ ] **Step 5: Run it to verify it passes**

```bash
cd /Users/fabio/projects/open-ontologies/studio && npm test -- compare-source
```

Expected: PASS, 2 tests.

- [ ] **Step 6: Wire the panel**

`ComparePanel` takes `{ result: CompareResult | null; onAsk: (q: string) => void }` and renders
the two answers in adjacent columns with their citations listed beneath each, and the divergence
note between them. It imports no source. Add it to the shell from Task 12, reading its data
from the store.

- [ ] **Step 7: Verify the replay build still works offline**

```bash
cd /Users/fabio/projects/open-ontologies/studio && npm test && npm run build:web
```

Expected: all tests pass and the web build succeeds. Open the preview with no engine running and
confirm the comparison renders from `bundle.json`.

- [ ] **Step 8: Commit**

```bash
cd /Users/fabio/projects/open-ontologies
git add studio/src-tauri/sidecars/agent/baseline.ts studio/src/components/ComparePanel.tsx \
        studio/src/lib/compare-source.ts studio/src/lib/__tests__/compare-source.test.ts
git commit -m "feat(studio): answer the same question twice and show the difference

One answer grounded in the ontology with citations into the corpus, one from a
plain baseline. Where they diverge is the point of the demonstration. The
replay reports an unscripted question rather than inventing a comparison."
```

---

## Task 13: Ship

**Files:**
- Modify: `studio/README.md`
- Modify: `README.md`

**Interfaces:**
- Consumes: everything above
- Produces: a published web replay, a desktop bundle, a recorded video

- [ ] **Step 1: Run every gate**

```bash
cd /Users/fabio/projects/open-ontologies
cargo test 2>&1 | tail -5
(cd studio && npm test)
make demo-verify; echo "exit=$?"
grep -rniEf .identifiers-guard \
  --exclude-dir=node_modules --exclude-dir=target --exclude-dir=.git . || echo CLEAN
```

Expected: Rust tests pass, frontend tests pass, `exit=0`, and `CLEAN`. The last check is a
release gate, not a formality.

- [ ] **Step 2: Publish the web replay**

Deploy `studio/dist-web` to a static host and confirm the URL renders with no engine.

- [ ] **Step 3: Update both READMEs**

State what the demonstration shows, that the corpus is public DCAT-US material, how to run the
pipeline, and where the hosted replay lives. No em dashes.

- [ ] **Step 4: Record the video**

One minute, screen recording, no voiceover required, hosted unlisted. Show the corpus, the
finding that cites two disagreeing documents, and the engine refusing a proposed change. The
refusal is the point of the recording, so it gets the most seconds.

- [ ] **Step 5: Commit and push**

```bash
cd /Users/fabio/projects/open-ontologies
git add README.md studio/README.md
git commit -m "docs: describe the demonstration and the hosted replay"
git push origin main
```

- [ ] **Step 6: Fill the submission**

EasyChair, `semantics2026`, Practitioners Track. The eight required fields are title, authors
with affiliation and contact, abstract, keywords, user experience section, system availability
statement, maturity indicator, and data sources used. Availability statement: public
repository plus the hosted replay URL. Maturity: engine at v1.2.0 with 110 MCP tools, PyPI
package, Docker image, external contributors, archived releases with DOIs; the interface is the
reference client. Data sources: DCAT-US 3.0 and W3C DCAT v3.

---

## Notes on sequencing

Tasks 1, 3 and 4 are independent of everything else and can run first in any order. Task 7 is
the highest-variance item and is scheduled for 25 August so that a corpus problem surfaces
with four days left rather than one. Tasks 9, 10 and 11 have no dependency on Task 12, so if
the interface rebuild slips, the previous panels still render over `DemoSource` and the video
can be recorded from them.

---

## Task 14 (optional, after submission): evaluate on an external benchmark

Added 24 August from a literature sweep. This is the only item from that sweep placed in the
plan, because it is the only one that needs no new architecture and produces a maturity claim
the submission can honestly make. It is explicitly optional and must not be started before the
29 August submission is in.

**Files:**
- Create: `demo/bench/wikiconflict_eval.py`
- Create: `demo/bench/RESULTS.md`

**Interfaces:**
- Consumes: the contradiction scanner from Task 5
- Produces: a measured precision and recall for provenance-split contradiction detection against
  a public reference set

- [ ] **Step 1: Confirm the benchmark is usable**

WikiConflict and TrustFuse (K-CAP 2025, code at `Orange-OpenSource/trustfuse`) supply
documents that genuinely disagree, with the disagreements labelled. Fetch it and confirm the
label format can be mapped onto the scanner's `Contradiction` shape. If it cannot be mapped
without distorting either side, stop and record why. A benchmark bent to fit is worth nothing.

- [ ] **Step 2: Run and report honestly**

Report precision, recall and the count of disagreements the scanner missed entirely, with at
least three missed cases quoted in full. The misses are the useful part. A results file that
reports only the score is marketing.

- [ ] **Step 3: Commit**

```bash
cd /Users/fabio/projects/open-ontologies
git add demo/bench/wikiconflict_eval.py demo/bench/RESULTS.md
git commit -m "test(demo): measure contradiction detection against a public reference set"
```

---

## Backlog from the literature sweep, not scheduled

Recorded so they are not lost. None of these are in scope before 29 August. The full sweep is at
`.superpowers/sdd/graphrag-sweep.md`.

1. **Ontology-grounded hyperedge retrieval** (OG-RAG, EMNLP 2025, code at `microsoft/ograg2`).
   Schema-constrained minimal-cover retrieval, genuinely distinct from the existing similarity
   search and community-based global search.
2. **A `filter_candidate_context` primitive** combining the existing SHACL validation and
   closed-world vocabulary check into one pre-model cleaning step (GraphRAG-FI, EMNLP 2025).
3. **A SHACL violation-explanation cache** keyed on a canonical violation signature (xpSHACL,
   VLDB LLM and graph workshop 2025, code at `gcpdev/xpshacl`).
4. **GraphRAG-Bench** (arXiv:2506.05690) to test whether community-based global search earns its
   place on this system's own data.

**The counterargument, which matters more than any of the above.** Three independent 2025
results converge on graph-topology retrieval, community-based global search included, losing to
plain embedding retrieval outside genuine whole-corpus sensemaking questions. One study measured
plain retrieval at 99.4% top-10 accuracy while the graph method pulled 47,000 tokens of context
against 3,700, with 63.3% of its top-one failures being same-chapter noise. This is a direct
challenge to the existing `onto_communities` primitive rather than a general remark about
retrieval.

It bears on the demonstration, not just the roadmap. The comparison panel from Task 12A shows a
grounded answer beside an ungrounded one, and on questions where retrieval alone is sufficient
the ungrounded answer will be just as good. Choosing demonstration questions where the grounded
answer wins for a stateable reason, and being willing to show one where it does not, is more
persuasive to this audience than a panel that always favours the home side.
