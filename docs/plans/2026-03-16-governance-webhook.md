# Governance Webhook Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire open-ontologies lineage events to OpenCheir's enforcer via webhook, making governance rules fire automatically.

**Architecture:** Extract `deliver_webhook` into a shared module, add a `governance_webhook` option to `LineageLog` that POSTs every lineage event. On the OpenCheir side, add an Axum endpoint that feeds events into the existing enforcer engine.

**Tech Stack:** Rust, reqwest, Axum, tokio, SQLite (both projects already use all of these)

---

### Task 1: Extract `deliver_webhook` into shared module (open-ontologies)

**Files:**
- Create: `src/webhook.rs`
- Modify: `src/lib.rs`
- Modify: `src/monitor.rs`

**Step 1: Create `src/webhook.rs`**

```rust
use std::time::Duration;

/// Fire-and-forget webhook delivery with 10s timeout.
pub async fn deliver_webhook(
    url: &str,
    headers_json: Option<&str>,
    payload: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    let mut req = client.post(url).json(payload);
    if let Some(hdr_json) = headers_json {
        if let Ok(map) =
            serde_json::from_str::<std::collections::HashMap<String, String>>(hdr_json)
        {
            for (k, v) in map {
                req = req.header(&k, &v);
            }
        }
    }
    let resp = req.send().await?;
    let status = resp.status();
    if !status.is_success() {
        eprintln!("Webhook to {} returned {}", url, status);
    }
    Ok(())
}
```

**Step 2: Register module in `src/lib.rs`**

Add `pub mod webhook;` to the module list.

**Step 3: Update `src/monitor.rs` to use shared function**

Remove the local `deliver_webhook` function and its `use std::time::Duration;` import. Change the call site from `deliver_webhook(...)` to `crate::webhook::deliver_webhook(...)`.

**Step 4: Build**

Run: `cargo build --release`
Expected: compiles with no errors

**Step 5: Commit**

```bash
git add src/webhook.rs src/lib.rs src/monitor.rs
git commit -m "refactor: extract deliver_webhook into shared webhook module"
```

---

### Task 2: Add governance webhook to LineageLog (open-ontologies)

**Files:**
- Modify: `src/lineage.rs`

**Step 1: Add webhook field and update constructor**

Change `LineageLog` to:

```rust
use crate::state::StateDb;
use chrono::Utc;

pub struct LineageLog {
    db: StateDb,
    governance_webhook: Option<String>,
}

impl LineageLog {
    pub fn new(db: StateDb) -> Self {
        Self { db, governance_webhook: None }
    }

    pub fn with_governance_webhook(db: StateDb, webhook_url: Option<String>) -> Self {
        Self { db, governance_webhook: webhook_url }
    }
```

**Step 2: Add webhook POST to `record()`**

After the existing `INSERT INTO lineage_events` block, add:

```rust
        // Fire governance webhook if configured
        if let Some(ref url) = self.governance_webhook {
            let url = url.clone();
            let payload = serde_json::json!({
                "source": "open-ontologies",
                "session_id": session_id,
                "seq": seq,
                "event_type": event_type,
                "operation": operation,
                "details": details,
                "timestamp": Utc::now().to_rfc3339(),
            });
            tokio::spawn(async move {
                let _ = crate::webhook::deliver_webhook(&url, None, &payload).await;
            });
        }
```

**Step 3: Build**

Run: `cargo build --release`
Expected: compiles — existing callers use `LineageLog::new()` which still works (webhook = None)

**Step 4: Commit**

```bash
git add src/lineage.rs
git commit -m "feat: add optional governance webhook to lineage events"
```

---

### Task 3: Wire governance webhook through server and CLI (open-ontologies)

**Files:**
- Modify: `src/server.rs` (lines 22-88)
- Modify: `src/main.rs` (lines 29-53, ~395-405)

**Step 1: Add `governance_webhook` field to `OpenOntologiesServer`**

In `src/server.rs`, add a field after `session_id`:

```rust
    governance_webhook: Option<String>,
```

**Step 2: Accept webhook in constructors**

Update `new()` and `new_with_graph()`:

```rust
    pub fn new(db: StateDb) -> Self {
        Self::new_with_options(db, Arc::new(GraphStore::new()), None)
    }

    pub fn new_with_graph(db: StateDb, graph: Arc<GraphStore>) -> Self {
        Self::new_with_options(db, graph, None)
    }

    pub fn new_with_options(db: StateDb, graph: Arc<GraphStore>, governance_webhook: Option<String>) -> Self {
        let lineage = crate::lineage::LineageLog::with_governance_webhook(db.clone(), governance_webhook.clone());
        let session_id = lineage.new_session();
        // ... rest of constructor, storing governance_webhook field
    }
```

**Step 3: Update `lineage()` helper to pass webhook**

```rust
    fn lineage(&self) -> crate::lineage::LineageLog {
        crate::lineage::LineageLog::with_governance_webhook(self.db.clone(), self.governance_webhook.clone())
    }
```

**Step 4: Add CLI args in `src/main.rs`**

Add to both `Serve` and `ServeHttp` variants:

```rust
        /// Optional governance webhook URL (fires on every lineage event)
        #[arg(long, env = "GOVERNANCE_WEBHOOK")]
        governance_webhook: Option<String>,
```

**Step 5: Pass webhook to server constructors in `src/main.rs`**

In the `Serve` match arm (~line 401):
```rust
let server = OpenOntologiesServer::new_with_options(db, Arc::new(GraphStore::new()), governance_webhook);
```

In the `ServeHttp` match arm (where `new_with_graph` is called):
```rust
let server = OpenOntologiesServer::new_with_options(db, graph, governance_webhook);
```

**Step 6: Build**

Run: `cargo build --release`
Expected: compiles

**Step 7: Commit**

```bash
git add src/server.rs src/main.rs
git commit -m "feat: wire governance webhook through CLI and server"
```

---

### Task 4: Add enforcer HTTP endpoint to OpenCheir

**Files:**
- Create: `src/orchestration/enforcer_api.rs` (in `/Users/fabio/projects/opencheir/`)
- Modify: `src/orchestration/mod.rs` (in `/Users/fabio/projects/opencheir/`)
- Modify: `src/main.rs` (in `/Users/fabio/projects/opencheir/`)

**Step 1: Create `src/orchestration/enforcer_api.rs`**

```rust
use axum::extract::State;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use super::enforcer::Enforcer;
use crate::store::state::StateDb;

#[derive(Deserialize)]
pub struct LineageEvent {
    pub source: Option<String>,
    pub session_id: Option<String>,
    pub seq: Option<i64>,
    pub event_type: Option<String>,
    pub operation: String,
    pub details: Option<String>,
    pub timestamp: Option<String>,
}

#[derive(Serialize)]
pub struct VerdictResponse {
    pub action: String,
    pub rule: Option<String>,
    pub reason: Option<String>,
}

pub struct EnforcerApiState {
    pub enforcer: Arc<Mutex<Enforcer>>,
    pub db: StateDb,
}

async fn handle_event(
    State(state): State<Arc<EnforcerApiState>>,
    Json(event): Json<LineageEvent>,
) -> Json<VerdictResponse> {
    let mut enforcer = state.enforcer.lock().unwrap();
    // Record the operation in the sliding window
    enforcer.post_check(&event.operation);
    // Evaluate rules against this operation
    let verdict = enforcer.pre_check(&event.operation);
    // Log to DB
    let session_id = event.session_id.as_deref().unwrap_or("external");
    let _ = Enforcer::log_verdict(&state.db, session_id, &verdict, &event.operation);
    let action_str = match verdict.action {
        super::enforcer::Action::Block => "block",
        super::enforcer::Action::Warn => "warn",
        super::enforcer::Action::Allow => "allow",
    };
    Json(VerdictResponse {
        action: action_str.to_string(),
        rule: verdict.rule,
        reason: verdict.reason,
    })
}

pub fn enforcer_router(state: Arc<EnforcerApiState>) -> Router {
    Router::new()
        .route("/api/enforcer/event", axum::routing::post(handle_event))
        .with_state(state)
}
```

**Step 2: Register module in `src/orchestration/mod.rs`**

Add `pub mod enforcer_api;`

**Step 3: Start the HTTP listener in `src/main.rs`**

In OpenCheir's `main.rs`, after MCP server setup, spawn the enforcer HTTP listener alongside the existing lineage API router. Use a configurable port (env `OPENCHEIR_HTTP_PORT`, default 9900):

```rust
// Enforcer HTTP API
let enforcer_state = Arc::new(orchestration::enforcer_api::EnforcerApiState {
    enforcer: enforcer_arc.clone(), // the Arc<Mutex<Enforcer>> already used by the MCP server
    db: db.clone(),
});
let enforcer_router = orchestration::enforcer_api::enforcer_router(enforcer_state);
let lineage_router = orchestration::lineage::lineage_router(db.clone());
let http_router = enforcer_router.merge(lineage_router);
let http_port: u16 = std::env::var("OPENCHEIR_HTTP_PORT")
    .ok()
    .and_then(|p| p.parse().ok())
    .unwrap_or(9900);
let http_addr = format!("127.0.0.1:{http_port}");
let http_listener = tokio::net::TcpListener::bind(&http_addr).await?;
eprintln!("OpenCheir HTTP API on http://{http_addr}");
tokio::spawn(async move {
    axum::serve(http_listener, http_router).await.ok();
});
```

**Step 4: Build OpenCheir**

Run: `cd /Users/fabio/projects/opencheir && cargo build --release`
Expected: compiles

**Step 5: Commit**

```bash
cd /Users/fabio/projects/opencheir
git add src/orchestration/enforcer_api.rs src/orchestration/mod.rs src/main.rs
git commit -m "feat: add HTTP enforcer endpoint for governance webhook integration"
```

---

### Task 5: Update docs (both projects)

**Files:**
- Modify: `CLAUDE.md` (open-ontologies)
- Modify: `README.md` (open-ontologies)
- Modify: `README.md` (opencheir)

**Step 1: Update open-ontologies CLAUDE.md**

In the "Enforcer Rules (Optional)" section, add:

```markdown
To enable automatic governance, start the server with:
```
GOVERNANCE_WEBHOOK=http://localhost:9900/api/enforcer/event open-ontologies serve
```

**Step 2: Update open-ontologies README.md**

Add a line to the Lifecycle row mentioning governance webhook support.

**Step 3: Update OpenCheir README.md**

Add a section explaining the `/api/enforcer/event` endpoint and how to connect it to open-ontologies.

**Step 4: Commit both repos**

```bash
cd /Users/fabio/projects/open-ontologies
git add CLAUDE.md README.md
git commit -m "docs: document governance webhook for OpenCheir integration"

cd /Users/fabio/projects/opencheir
git add README.md
git commit -m "docs: document enforcer HTTP endpoint"
```

---

### Task 6: Integration test

**Step 1: Build both projects**

```bash
cd /Users/fabio/projects/open-ontologies && cargo build --release
cd /Users/fabio/projects/opencheir && cargo build --release
```

**Step 2: Start OpenCheir**

```bash
cd /Users/fabio/projects/opencheir
./target/release/opencheir serve &
```

This starts the MCP server on stdio AND the HTTP API on port 9900.

**Step 3: Test the enforcer endpoint directly**

```bash
curl -X POST http://localhost:9900/api/enforcer/event \
  -H "Content-Type: application/json" \
  -d '{"operation": "onto_save", "session_id": "test-1"}'
```

Expected: `{"action":"allow","rule":null,"reason":null}` (first save, no rule fires)

**Step 4: Test rule firing**

Send 3 save events without validate:

```bash
for i in 1 2 3; do
  curl -s -X POST http://localhost:9900/api/enforcer/event \
    -H "Content-Type: application/json" \
    -d '{"operation": "onto_save", "session_id": "test-1"}'
done
```

The 3rd response should return: `{"action":"warn","rule":"onto_validate_after_save","reason":"Warn if ontology is saved 3+ times without validation"}`

**Step 5: Push both repos**

```bash
cd /Users/fabio/projects/open-ontologies && git push
cd /Users/fabio/projects/opencheir && git push
```
