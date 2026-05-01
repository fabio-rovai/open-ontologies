use std::time::Duration;

/// Fire-and-forget webhook delivery. Timeout is configurable via
/// `[webhook] request_timeout_secs` (default 10s).
pub async fn deliver_webhook(
    url: &str,
    headers_json: Option<&str>,
    payload: &serde_json::Value,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let timeout_secs = crate::runtime::webhook_request_timeout_secs();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()?;
    let mut req = client.post(url).json(payload);
    if let Some(hdr_json) = headers_json
        && let Ok(map) = serde_json::from_str::<std::collections::HashMap<String, String>>(hdr_json) {
            for (k, v) in map {
                req = req.header(&k, &v);
            }
    }
    let resp = req.send().await?;
    let status = resp.status();
    if !status.is_success() {
        eprintln!("Webhook to {} returned {}", url, status);
    }
    Ok(())
}
