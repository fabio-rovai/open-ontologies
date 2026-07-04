//! Background daemon management — start/stop/status for the persistent HTTP store.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonInfo {
    pub pid: u32,
    pub url: String,
    pub token: Option<String>,
}

pub fn daemon_file_path(data_dir: &str) -> PathBuf {
    let data_dir = crate::config::expand_tilde(data_dir);
    PathBuf::from(data_dir).join("daemon.json")
}

pub fn read_daemon_info(data_dir: &str) -> Option<DaemonInfo> {
    let path = daemon_file_path(data_dir);
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

fn write_daemon_info(data_dir: &str, info: &DaemonInfo) -> anyhow::Result<()> {
    let path = daemon_file_path(data_dir);
    std::fs::write(path, serde_json::to_string_pretty(info)?)?;
    Ok(())
}

pub fn remove_daemon_info(data_dir: &str) {
    let _ = std::fs::remove_file(daemon_file_path(data_dir));
}

/// Returns true if the process with the given PID is alive.
pub fn is_daemon_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // `kill -0 <pid>` exits 0 when the process exists and we have permission.
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/NH"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }
}

/// Start `serve-http` in the background and write daemon.json.
/// Returns the `DaemonInfo` on success.
pub fn start_daemon(
    data_dir: &str,
    host: &str,
    port: u16,
    token: Option<String>,
) -> anyhow::Result<DaemonInfo> {
    let exe = std::env::current_exe()
        .map_err(|e| anyhow::anyhow!("cannot find current binary: {}", e))?;

    let data_dir_expanded = crate::config::expand_tilde(data_dir);

    let mut cmd = std::process::Command::new(&exe);
    cmd.arg("--data-dir").arg(&data_dir_expanded);
    cmd.arg("serve-http");
    cmd.arg("--host").arg(host);
    cmd.arg("--port").arg(port.to_string());
    if let Some(ref t) = token {
        cmd.arg("--token").arg(t);
    }
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());

    let child = cmd.spawn()
        .map_err(|e| anyhow::anyhow!("failed to spawn daemon: {}", e))?;

    let pid = child.id();
    // Detach: forget the Child so it becomes an orphan adopted by init.
    std::mem::forget(child);

    // Give the server a moment to bind the port before returning.
    std::thread::sleep(std::time::Duration::from_millis(600));

    let url = format!("http://{}:{}", host, port);
    let info = DaemonInfo { pid, url, token };
    write_daemon_info(data_dir, &info)?;
    Ok(info)
}

/// Kill the daemon and remove daemon.json.
pub fn stop_daemon(data_dir: &str) -> anyhow::Result<()> {
    let info = read_daemon_info(data_dir)
        .ok_or_else(|| anyhow::anyhow!("no daemon.json found — is the daemon running?"))?;

    if !is_daemon_alive(info.pid) {
        remove_daemon_info(data_dir);
        anyhow::bail!("daemon PID {} is not running (stale daemon.json removed)", info.pid);
    }

    #[cfg(unix)]
    {
        let status = std::process::Command::new("kill")
            .args([&info.pid.to_string()])
            .status()
            .map_err(|e| anyhow::anyhow!("kill failed: {}", e))?;
        if !status.success() {
            anyhow::bail!("kill {} failed with status {}", info.pid, status);
        }
    }
    #[cfg(not(unix))]
    {
        std::process::Command::new("taskkill")
            .args(["/PID", &info.pid.to_string(), "/F"])
            .status()
            .map_err(|e| anyhow::anyhow!("taskkill failed: {}", e))?;
    }

    remove_daemon_info(data_dir);
    Ok(())
}
