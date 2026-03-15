# Studio Architecture Redesign

**Date:** 2026-03-14
**Status:** Implemented

## Problem

The Studio was reimplementing logic the engine already handles — including a Claude API client for ontology generation. This duplicated the engine's 42 MCP tools and 5 prompts.

## Decision

Three-layer architecture with clean separation:

| Layer | Role | Responsibilities |
|-------|------|-----------------|
| **Studio** (Flutter) | Visual client | Graph canvas, property inspector, chat panel with thin tool wrappers, auto-refresh polling |
| **Claude** (separate process) | AI orchestrator | Runs via Claude Code/Desktop, connects to engine via MCP, uses engine prompts |
| **Engine** (Rust) | Shared backend | Oxigraph triple store, 42 MCP tools, 5 prompts, full CLI |

## Key Principles

1. **No AI in Studio** — Claude runs separately, connects to the same engine
2. **Thin pass-through** — command processor delegates to engine MCP tools, no logic reimplemented in Dart
3. **Polling auto-refresh** — Studio polls `onto_stats` every 3s; if triple count changes, invalidates providers to pick up external changes (e.g. from Claude)
4. **Generic tool command** — `tool <name> [json]` lets users call any engine tool directly

## Changes Made

- Deleted `claude_client.dart`
- Removed Claude API references from `providers.dart`, `chat_panel.dart`, `command_processor.dart`
- Removed `_generate`, `_looksLikeGenerateRequest`, and `generate`/`build`/`create` switch cases
- Added `_tool()` method — generic pass-through to any engine MCP tool
- Added `_status()` method — shows engine connection status
- Added 3s polling timer in `home_screen.dart` for auto-refresh
- Updated help text and welcome message
