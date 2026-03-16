# Governance Webhook — OpenCheir Integration

## Summary

Add an optional governance webhook to open-ontologies so that every lineage event (save, apply, push, monitor, enforce, etc.) is automatically POSTed to an external endpoint. When pointed at OpenCheir's new `/api/enforcer/event` endpoint, this makes the existing enforcer rules (`onto_validate_after_save`, `onto_version_before_push`) fire automatically — no Claude orchestration required.

## Motivation

OpenCheir already has enforcer rules for ontology workflows, but they only work if Claude remembers to call `enforcer_check`. By pushing lineage events from open-ontologies to OpenCheir via webhook, governance becomes automatic and works even in headless/CI pipelines.

## Changes

### 1. Open-ontologies: lineage.rs

Add an optional webhook URL to `LineageLog`. When set, every `record()` call also spawns a fire-and-forget POST.

- New field: `governance_webhook: Option<String>`
- Constructor accepts the URL (from env var or CLI arg)
- Reuse the `deliver_webhook` function from monitor.rs (extract to a shared `webhook` module)

### 2. Open-ontologies: shared webhook module

Extract `deliver_webhook` from monitor.rs into `src/webhook.rs` so both monitor and lineage can use it.

### 3. Open-ontologies: CLI / server config

- New env var: `GOVERNANCE_WEBHOOK` (URL)
- New CLI flag: `--governance-webhook <URL>` on `serve` and `serve-http` subcommands
- Passed through to `LineageLog::new()`

### 4. Payload format

```json
{
  "source": "open-ontologies",
  "session_id": "abc123def456",
  "seq": 5,
  "event_type": "A",
  "operation": "apply",
  "details": "safe",
  "timestamp": "2026-03-16T14:00:00Z"
}
```

### 5. OpenCheir: HTTP enforcer endpoint

Add a small Axum route at `/api/enforcer/event`:

- Receives the lineage event JSON
- Calls `enforcer.post_check(&event.operation)` to record in sliding window
- Calls `enforcer.pre_check(&event.operation)` to evaluate rules
- Logs the verdict to the enforcement table
- Returns the verdict as JSON (for observability, not blocking)

### 6. OpenCheir: start HTTP listener

OpenCheir already uses Axum for its lineage API. Add the enforcer endpoint to the same router, listening on a configurable port (default 9900).

## What this does NOT include

- No blocking — verdicts are fire-and-forget, logged but don't halt open-ontologies
- No new MCP tools on either side
- No changes to the enforcer rule engine logic
- No retries or dead-letter queue
