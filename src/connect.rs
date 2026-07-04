//! HTTP client that proxies CLI batch commands to a running daemon.

use crate::daemon::DaemonInfo;

/// POST batch input to the daemon's `/api/batch` endpoint and stream results to stdout.
/// Output matches local CLI format: human-readable by default, JSON when `json` is true.
/// Returns the process exit code (0 = all OK, 1 = at least one error).
pub async fn proxy_batch(
    info: &DaemonInfo,
    input: &str,
    bail: bool,
    json: bool,
    pretty: bool,
) -> anyhow::Result<i32> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/batch{}", info.url, if bail { "?bail=true" } else { "" });

    let mut req = client
        .post(&url)
        .header("Content-Type", "text/plain")
        .body(input.to_string());

    if let Some(ref token) = info.token {
        req = req.header("Authorization", format!("Bearer {}", token));
    }

    let response = req.send().await
        .map_err(|e| anyhow::anyhow!("daemon request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("daemon returned {}: {}", status, body);
    }

    let items: Vec<serde_json::Value> = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("failed to parse daemon response: {}", e))?;

    let mut exit_code = 0;
    for item in &items {
        // Unwrap the batch envelope — print only the result, matching local output.
        let result = item.get("result").unwrap_or(item);
        if json || pretty {
            if pretty {
                println!("{}", serde_json::to_string_pretty(result).unwrap());
            } else {
                println!("{}", result);
            }
        } else {
            println!("{}", crate::output::render_human(result));
        }
        if result.get("error").is_some() {
            exit_code = 1;
        }
    }

    Ok(exit_code)
}
