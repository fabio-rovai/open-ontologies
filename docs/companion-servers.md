# Companion Servers

Open Ontologies is extensible without touching its binary: because it is an MCP server, **any other MCP server connected to the same session composes with it** — the orchestrating agent chains tools across servers in one conversation. [OpenCheir](https://github.com/fabio-rovai/opencheir) already works this way: it watches Open Ontologies' lineage events and enforces workflow rules, with zero coupling beyond a webhook.

This document names that pattern — the **companion server** — and defines the contract that makes a companion feel like part of one product instead of two tools that happen to coexist.

## The contract

A companion server that follows these five rules gets listed in [ECOSYSTEM.md](../ECOSYSTEM.md).

### 1. MCP-native: no embedded LLM

The connected orchestrator is already an LLM. A companion provides validation primitives, scaffolding outputs, and feedback channels — it never embeds its own LLM client, requires an API key, or hides judgment inside a server function. This is the same convention the core server holds itself to (see "Architecture Convention: MCP-Native Tool Design" in [CLAUDE.md](../CLAUDE.md)).

### 2. Tool naming: never squat `onto_*`

The `onto_` prefix belongs to the core server. A companion picks its own short prefix (`oo_<name>_*` or a product name like `opencheir_*`) so a session with both servers connected has an unambiguous tool namespace, and skills can reference tools without qualification.

### 3. Interchange: packs and files, not shared memory

Companions never reach into the core server's in-memory store. The interchange artifacts are:

- **Ontology files** — Turtle/N-Triples in directories both servers can be pointed at (`[general] ontology_dirs`).
- **Packs** — `onto_pack` output is the promotion artifact: sorted N-Triples + manifest + checksums + recorded lint/enforce evidence. A companion that consumes graphs should accept packs and verify checksums (`onto_unpack --verify_only` semantics) rather than trusting bare files.
- **SPARQL endpoints** — the core can `onto_push`/`onto_pull` against any endpoint a companion exposes or consumes.

### 4. Events: the lineage webhook

The core POSTs every lineage event (plan, apply, save, push, …) to `GOVERNANCE_WEBHOOK` if set. A companion that wants to observe the engineering workflow subscribes there — it does not poll, and it does not require the core to know it exists:

```bash
your-companion serve &   # listens on :9900
GOVERNANCE_WEBHOOK=http://localhost:9900/api/enforcer/event open-ontologies serve
```

### 5. Degrade gracefully

The core must work fully with the companion absent, and vice versa. Optional integration, never a hard dependency — the OpenCheir enforcer rules are the reference implementation of this posture.

## What makes a good companion

The distinguishing question is the same one used for core tools: *what primitive does the orchestrator need that it can't compute itself?* Good companion territory: domain rule packs behind a validation surface (regulatory, clinical, defence), storage/deployment backends (pack registries, triple-store sync), observability (dashboards over lineage events), and editor/UI surfaces (the Obsidian plugin is a companion in this sense — it brings vault notes to the same engine).

## Getting listed

Open a PR adding a row to [ECOSYSTEM.md](../ECOSYSTEM.md) with: name, repo link, one-line description, and which of the five contract rules your server exercises (webhook consumer, pack consumer, endpoint provider, …). Listing criteria: the repo is public, the README shows it composing with Open Ontologies in a real session, and it holds rule 1.
