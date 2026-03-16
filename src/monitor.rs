use crate::graph::GraphStore;
use crate::state::StateDb;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WatcherAction {
    #[serde(rename = "notify")]
    Notify,
    #[serde(rename = "block_next_apply")]
    BlockNextApply,
    #[serde(rename = "auto_rollback")]
    AutoRollback,
    #[serde(rename = "log")]
    Log,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Watcher {
    pub id: String,
    pub check_type: String,
    pub threshold: f64,
    pub severity: String,
    pub action: WatcherAction,
    pub query: Option<String>,
    pub message: Option<String>,
    #[serde(default)]
    pub webhook_url: Option<String>,
    #[serde(default)]
    pub webhook_headers: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Alert {
    pub watcher: String,
    pub severity: String,
    pub value: f64,
    pub threshold: f64,
    pub action: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MonitorResult {
    pub status: String,
    pub alerts: Vec<Alert>,
    pub passed: Vec<String>,
}

pub struct Monitor {
    db: StateDb,
    graph: Arc<GraphStore>,
}

impl Monitor {
    pub fn new(db: StateDb, graph: Arc<GraphStore>) -> Self {
        Self { db, graph }
    }

    pub fn add_watcher(&self, watcher: Watcher) {
        let conn = self.db.conn();
        let action_str = serde_json::to_string(&watcher.action).unwrap_or_default();
        let action_str = action_str.trim_matches('"');
        let _ = conn.execute(
            "INSERT OR REPLACE INTO monitor_watchers (id, check_type, threshold, severity, action, query, message, webhook_url, webhook_headers, enabled)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1)",
            rusqlite::params![
                watcher.id, watcher.check_type, watcher.threshold,
                watcher.severity, action_str, watcher.query, watcher.message,
                watcher.webhook_url, watcher.webhook_headers,
            ],
        );
    }

    pub fn run_watchers(&self) -> MonitorResult {
        let watchers = self.load_watchers();
        let mut alerts = Vec::new();
        let mut passed = Vec::new();
        let mut blocked = false;

        for w in &watchers {
            let value = self.evaluate_watcher(w);
            if value > w.threshold {
                let action_str = serde_json::to_string(&w.action).unwrap_or_default();
                let action_str = action_str.trim_matches('"').to_string();
                if matches!(w.action, WatcherAction::BlockNextApply) {
                    blocked = true;
                    self.set_blocked(true);
                }
                // Fire webhook for Notify actions
                if matches!(w.action, WatcherAction::Notify) {
                    if let Some(ref url) = w.webhook_url {
                        let url = url.clone();
                        let headers = w.webhook_headers.clone();
                        let payload = serde_json::json!({
                            "source": "open-ontologies",
                            "watcher_id": w.id,
                            "severity": w.severity,
                            "value": value,
                            "threshold": w.threshold,
                            "message": w.message.clone().unwrap_or_default(),
                            "timestamp": chrono::Utc::now().to_rfc3339(),
                        });
                        tokio::spawn(async move {
                            let _ = deliver_webhook(&url, headers.as_deref(), &payload).await;
                        });
                    }
                }
                alerts.push(Alert {
                    watcher: w.id.clone(),
                    severity: w.severity.clone(),
                    value,
                    threshold: w.threshold,
                    action: action_str,
                    detail: w.message.clone().unwrap_or_default(),
                });
            } else {
                passed.push(w.id.clone());
            }
        }

        let status = if blocked {
            "blocked".to_string()
        } else if !alerts.is_empty() {
            "alert".to_string()
        } else {
            "ok".to_string()
        };

        MonitorResult { status, alerts, passed }
    }

    pub fn is_blocked(&self) -> bool {
        let conn = self.db.conn();
        let result: Option<String> = conn
            .query_row(
                "SELECT value FROM monitor_state WHERE key = 'blocked'",
                [],
                |r| r.get(0),
            )
            .ok();
        result.as_deref() == Some("true")
    }

    pub fn set_blocked(&self, blocked: bool) {
        let conn = self.db.conn();
        let _ = conn.execute(
            "INSERT OR REPLACE INTO monitor_state (key, value) VALUES ('blocked', ?1)",
            rusqlite::params![if blocked { "true" } else { "false" }],
        );
    }

    pub fn clear_blocked(&self) {
        self.set_blocked(false);
    }

    fn load_watchers(&self) -> Vec<Watcher> {
        let conn = self.db.conn();
        let mut stmt = conn
            .prepare("SELECT id, check_type, threshold, severity, action, query, message, webhook_url, webhook_headers FROM monitor_watchers WHERE enabled = 1")
            .unwrap();
        stmt.query_map([], |row| {
            let action_str: String = row.get(4)?;
            let action = match action_str.as_str() {
                "block_next_apply" => WatcherAction::BlockNextApply,
                "auto_rollback" => WatcherAction::AutoRollback,
                "log" => WatcherAction::Log,
                _ => WatcherAction::Notify,
            };
            Ok(Watcher {
                id: row.get(0)?,
                check_type: row.get(1)?,
                threshold: row.get(2)?,
                severity: row.get(3)?,
                action,
                query: row.get(5)?,
                message: row.get(6)?,
                webhook_url: row.get(7)?,
                webhook_headers: row.get(8)?,
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
    }

    fn evaluate_watcher(&self, watcher: &Watcher) -> f64 {
        match watcher.check_type.as_str() {
            "sparql" => self.eval_sparql_watcher(watcher),
            _ => 0.0,
        }
    }

    fn eval_sparql_watcher(&self, watcher: &Watcher) -> f64 {
        let query = match &watcher.query {
            Some(q) => q,
            None => return 0.0,
        };
        // Expect a SELECT query returning a ?count binding
        match self.graph.sparql_select(query) {
            Ok(json) => {
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&json)
                    && let Some(results) = parsed["results"].as_array()
                        && let Some(first) = results.first()
                            && let Some(count_str) = first["count"].as_str() {
                                // Oxigraph returns literal like "\"1\"^^<http://...>"
                                let cleaned = count_str
                                    .trim_matches('"')
                                    .split("^^")
                                    .next()
                                    .unwrap_or("0")
                                    .trim_matches('"');
                                return cleaned.parse().unwrap_or(0.0);
                            }
                0.0
            }
            Err(_) => 0.0,
        }
    }
}

/// Fire-and-forget webhook delivery with 10s timeout.
async fn deliver_webhook(
    url: &str,
    headers_json: Option<&str>,
    payload: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    let mut req = client.post(url).json(payload);
    if let Some(hdr_json) = headers_json {
        if let Ok(map) = serde_json::from_str::<std::collections::HashMap<String, String>>(hdr_json) {
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
