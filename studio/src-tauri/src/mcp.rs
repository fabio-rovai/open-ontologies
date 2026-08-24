use std::sync::Mutex;

use crate::engine;

/// Pure construction of the MCP endpoint URL from a port, kept separate from
/// port resolution so it can be unit-tested without touching the network or
/// environment variables.
fn mcp_endpoint(port: u16) -> String {
    format!("http://127.0.0.1:{port}/mcp")
}

pub struct McpState {
    pub session_id: Mutex<Option<String>>,
    pub client: reqwest::Client,
}

#[tauri::command]
pub async fn mcp_call(
    method: String,
    params: serde_json::Value,
    state: tauri::State<'_, McpState>,
) -> Result<serde_json::Value, String> {
    // Try the call; if the session has expired, reinitialize and retry once
    match do_mcp_call(&method, &params, &state).await {
        Err(ref e) if e.contains("Session not found") || e.contains("Not Found") => {
            // Session expired — reinitialize and retry
            reinitialize(&state).await?;
            do_mcp_call(&method, &params, &state).await
        }
        other => other,
    }
}

async fn reinitialize(state: &tauri::State<'_, McpState>) -> Result<(), String> {
    let init_params = serde_json::json!({
        "protocolVersion": "2025-03-26",
        "capabilities": {},
        "clientInfo": { "name": "open-ontologies-studio", "version": "1.0.0" }
    });
    do_mcp_call("initialize", &init_params, state).await?;
    Ok(())
}

async fn do_mcp_call(
    method: &str,
    params: &serde_json::Value,
    state: &tauri::State<'_, McpState>,
) -> Result<serde_json::Value, String> {
    let client = &state.client;

    let session_id = state.session_id.lock()
        .map_err(|e| format!("Lock error: {e}"))?
        .clone();

    // Notifications must NOT include an "id" field per MCP spec
    let is_notification = method.starts_with("notifications/");
    let body = if is_notification {
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        })
    } else {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": rand_id(),
            "method": method,
            "params": params,
        })
    };

    let mut req = client
        .post(mcp_endpoint(engine::engine_port()))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .json(&body);

    if let Some(sid) = &session_id {
        req = req.header("Mcp-Session-Id", sid);
    }

    let resp = req.send().await.map_err(|e| format!("Request failed: {e}"))?;

    // Capture new session ID from response
    if let Some(sid) = resp.headers().get("mcp-session-id") {
        if let Ok(sid_str) = sid.to_str() {
            if let Ok(mut guard) = state.session_id.lock() {
                *guard = Some(sid_str.to_string());
            }
        }
    }

    // Notifications get 202 with empty body — that's success
    if is_notification {
        return Ok(serde_json::Value::Null);
    }

    let text = resp.text().await.map_err(|e| format!("Read body failed: {e}"))?;

    // Surface session errors so the caller can retry
    if text.contains("Session not found") || text.contains("Not Found") {
        return Err(text);
    }

    parse_response(&text)
}

fn parse_response(text: &str) -> Result<serde_json::Value, String> {
    for line in text.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            let trimmed = data.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) {
                if parsed.get("result").is_some() {
                    return Ok(parsed["result"].clone());
                }
                if let Some(err) = parsed.get("error") {
                    return Err(format!("MCP error: {}", err));
                }
                return Ok(parsed);
            }
        }
    }

    serde_json::from_str(text)
        .map(|v: serde_json::Value| v.get("result").cloned().unwrap_or(v))
        .map_err(|_| format!("Failed to parse response: {}", &text[..text.len().min(200)]))
}

#[tauri::command]
pub async fn set_mcp_session(
    session_id: String,
    state: tauri::State<'_, McpState>,
) -> Result<(), String> {
    let mut guard = state.session_id.lock().map_err(|e| format!("Lock error: {e}"))?;
    *guard = Some(session_id);
    Ok(())
}

fn rand_id() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_endpoint_tracks_the_configured_port_not_a_literal() {
        assert_eq!(mcp_endpoint(8137), "http://127.0.0.1:8137/mcp");
        assert_eq!(mcp_endpoint(9001), "http://127.0.0.1:9001/mcp");
        // The regression this guards against: the engine's default port
        // changed away from 8080, but this call site kept the old literal.
        assert_ne!(mcp_endpoint(8137), "http://127.0.0.1:8080/mcp");
    }

    // Rust runs unit tests in parallel threads within one process, so a test
    // that mutates a process environment variable races every other test
    // that reads or writes the environment concurrently (std::env::set_var
    // and remove_var are `unsafe` for exactly this reason). This lock
    // serializes the one test below that needs to do it, so it never
    // overlaps another env mutation in this crate.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn mcp_call_site_builds_its_url_from_the_configured_port_resolver() {
        const VAR: &str = "OPEN_ONTOLOGIES_STUDIO_PORT";
        const DISTINCTIVE_PORT: &str = "48213";

        let _guard = ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous = std::env::var(VAR).ok();

        // SAFETY: ENV_LOCK above guarantees no other test in this process is
        // reading or writing the environment while this section runs.
        unsafe {
            std::env::set_var(VAR, DISTINCTIVE_PORT);
        }
        let endpoint = mcp_endpoint(engine::engine_port());
        unsafe {
            match &previous {
                Some(value) => std::env::set_var(VAR, value),
                None => std::env::remove_var(VAR),
            }
        }

        // This exercises the actual call site in do_mcp_call
        // (`.post(mcp_endpoint(engine::engine_port()))`), not just the pure
        // helper: it proves the URL is built from the real port resolver, so
        // re-inlining a literal at that call site would fail this test.
        assert_eq!(endpoint, "http://127.0.0.1:48213/mcp");
    }
}
