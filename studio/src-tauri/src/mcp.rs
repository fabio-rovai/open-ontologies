use std::sync::Mutex;

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
        .post("http://127.0.0.1:8080/mcp")
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
