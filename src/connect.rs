//! HTTP client that proxies CLI batch commands to a running daemon.

use crate::daemon::DaemonInfo;

/// POST batch input to the daemon's `/api/batch` endpoint and stream results to
/// stdout. Output matches local CLI format: JSON unless `json` is false.
///
/// `Ok(Some(code))` means the daemon ran the commands and this is the exit code.
/// `Ok(None)` means the daemon could not be used and the caller should run
/// locally; the reason has already been written to stderr. That distinction is
/// the difference between a working CLI and a bricked one: a daemon started
/// while `[http] token` was set in config enforces bearer auth, and a client
/// that treats the resulting 401 as fatal turns every proxy-able command into an
/// error until `daemon stop`. Falling back is announced rather than silent,
/// because the local store is not necessarily the daemon's.
pub async fn proxy_batch(
    info: &DaemonInfo,
    input: &str,
    bail: bool,
    json: bool,
    pretty: bool,
) -> anyhow::Result<Option<i32>> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/batch{}", info.url, if bail { "?bail=true" } else { "" });

    // The endpoint sniffs the body rather than the header, but saying which form
    // is being sent costs nothing and keeps the request honest.
    let content_type = if input.trim_start().starts_with('[') {
        "application/json"
    } else {
        "text/plain"
    };

    let mut req = client
        .post(&url)
        .header("Content-Type", content_type)
        .body(input.to_string());

    if let Some(ref token) = info.token {
        req = req.header("Authorization", format!("Bearer {}", token));
    }

    let response = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            // The liveness check passed a moment ago, so this is a daemon that
            // died in between, or one bound somewhere we cannot reach.
            eprintln!("warning: daemon at {} is unreachable ({e}); running locally", info.url);
            return Ok(None);
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::FORBIDDEN
        {
            eprintln!(
                "warning: daemon at {} rejected authentication ({status}); running locally",
                info.url
            );
            return Ok(None);
        }
        anyhow::bail!("daemon returned {}: {}", status, body);
    }

    let items: Vec<serde_json::Value> = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("failed to parse daemon response: {}", e))?;

    let mut exit_code = 0;
    for item in &items {
        // Unwrap the batch envelope — print only the result, matching local
        // output — but keep the command name, which is the discriminator the
        // human renderer needs to dispatch exactly instead of guessing from
        // which keys a payload happens to carry.
        let command = item.get("command").and_then(|c| c.as_str());
        let result = item.get("result").unwrap_or(item);
        if json || pretty {
            if pretty {
                println!("{}", serde_json::to_string_pretty(result).unwrap());
            } else {
                println!("{}", result);
            }
        } else {
            println!("{}", crate::output::render_human_for(command, result));
        }
        if result.get("error").is_some() {
            exit_code = 1;
        }
    }

    Ok(Some(exit_code))
}
