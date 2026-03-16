# Webhook Actions for Monitor Watchers

## Summary

Extend the existing monitor watcher system to fire HTTP webhooks when alerts trigger. No new tools -- the existing `onto_monitor` accepts watcher JSON with two new optional fields: `webhook_url` and `webhook_headers`.

## Motivation

The monitor system already evaluates SPARQL conditions and produces alerts, but the Notify action is a no-op. Adding webhook delivery turns the platform from passive (alerts returned to caller) to active (alerts pushed to external systems -- Slack, PagerDuty, custom APIs).

## Changes

### 1. Watcher struct (monitor.rs)

Add two optional fields:
- `webhook_url: Option<String>` -- POST target URL
- `webhook_headers: Option<String>` -- JSON object of extra headers (e.g. `{"Authorization": "Bearer ..."}`)

### 2. SQLite schema (state.rs)

Add columns to `monitor_watchers`:
```sql
ALTER TABLE monitor_watchers ADD COLUMN webhook_url TEXT;
ALTER TABLE monitor_watchers ADD COLUMN webhook_headers TEXT;
```

Use IF NOT EXISTS pattern for safe migration.

### 3. Webhook delivery (monitor.rs)

After `run_watchers()` collects alerts, for each alert whose watcher has a `webhook_url`:
- Build JSON payload with alert fields + timestamp
- POST to webhook_url with Content-Type: application/json
- Include any custom headers from webhook_headers
- 10 second timeout, fire-and-forget (tokio::spawn)
- Log delivery result to lineage

### 4. Payload format

```json
{
  "source": "open-ontologies",
  "watcher_id": "unlabeled-classes",
  "severity": "warning",
  "value": 12,
  "threshold": 5,
  "message": "12 classes without labels",
  "timestamp": "2026-03-16T12:00:00Z"
}
```

### What this does NOT include

- No retries or dead-letter queue (YAGNI)
- No new tools (onto_monitor already handles watcher management)
- No webhook registration UI
- No payload templating
