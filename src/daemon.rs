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
///
/// On unix this is a bare `kill(pid, 0)` rather than fork+exec of `/bin/kill`:
/// it runs on every proxy-able invocation, which is the exact path the daemon
/// exists to make fast, and spawning a process to ask whether a process exists
/// is most of that path's cost. `EPERM` counts as alive — the process is there,
/// this user simply may not signal it — and a non-positive PID is rejected
/// outright, because `kill(0, ...)` addresses the whole process group.
pub fn is_daemon_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        let rc = unsafe { libc::kill(pid as libc::pid_t, 0) };
        if rc == 0 {
            return true;
        }
        std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
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

/// Whether the process with `pid` looks like an open-ontologies daemon, by
/// inspecting its command line for the `serve-http` subcommand every daemon is
/// started with (see `start_daemon`). This is the identity check that keeps a
/// recycled PID in a stale daemon.json from being signalled as if it were ours.
#[cfg(unix)]
fn pid_is_open_ontologies_daemon(pid: u32) -> bool {
    std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "command="])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("serve-http"))
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn pid_is_open_ontologies_daemon(pid: u32) -> bool {
    std::process::Command::new("wmic")
        .args([
            "process",
            "where",
            &format!("ProcessId={pid}"),
            "get",
            "CommandLine",
        ])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("serve-http"))
        .unwrap_or(false)
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

    // Name the config file explicitly instead of letting the child fall back to
    // its own default. Both sides then read one file, which is what makes the
    // token below the token the child will actually enforce, and what keeps a
    // non-default --data-dir from starting a daemon configured by a different
    // directory's config.
    let config_path = PathBuf::from(&data_dir_expanded).join("config.toml");

    // `serve-http` falls back to `[http] token` in config when no flag or env
    // var is given, and then enforces bearer auth. Recording `None` here because
    // no flag was passed left every proxied command sending no Authorization
    // header to a daemon demanding one: 401 on everything until `daemon stop`.
    // Resolve it the same way the child will, and record what it resolves to.
    let token = token.or_else(|| {
        crate::config::Config::load(&config_path)
            .ok()
            .and_then(|c| crate::config::resolve_http_token(&c.http))
    });

    let mut cmd = std::process::Command::new(&exe);
    cmd.arg("--data-dir").arg(&data_dir_expanded);
    cmd.arg("serve-http");
    cmd.arg("--config").arg(&config_path);
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

    // Wait for the port to accept a connection rather than sleeping a fixed
    // 600ms. The fixed sleep was wrong in both directions: it burned the whole
    // 600ms when the bind took 20, and it reported success when the child had
    // already died or never bound at all, so `daemon start` printed a PID and
    // URL for something that was not listening and every later command then
    // failed against it.
    let probe_host = if host == "0.0.0.0" || host == "::" { "127.0.0.1" } else { host };
    let addr = format!("{}:{}", probe_host, port);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut bound = false;
    while std::time::Instant::now() < deadline {
        if std::net::TcpStream::connect(&addr).is_ok() {
            bound = true;
            break;
        }
        if !is_daemon_alive(pid) {
            anyhow::bail!("daemon exited before binding {} (pid {})", addr, pid);
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    if !bound {
        anyhow::bail!("daemon did not bind {} within 10s (pid {})", addr, pid);
    }

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

    // A live PID is not proof it is OUR daemon. daemon.json survives a crash or
    // reboot, and the OS recycles PIDs, so a stale record can point at an unrelated
    // process. Verify the process is actually an open-ontologies daemon before
    // signalling it, or `daemon stop` becomes a way to kill an arbitrary process.
    if !pid_is_open_ontologies_daemon(info.pid) {
        remove_daemon_info(data_dir);
        anyhow::bail!(
            "PID {} is running but is not an open-ontologies daemon; refusing to \
             signal an unrelated process (stale daemon.json removed)",
            info.pid
        );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_current_process_is_alive_and_pid_zero_is_not() {
        assert!(is_daemon_alive(std::process::id()));
        // `kill(0, sig)` addresses the caller's whole process group, so a zero
        // PID must be rejected before it reaches the syscall rather than
        // reported as a living daemon.
        assert!(!is_daemon_alive(0));
    }

    #[test]
    fn a_pid_that_cannot_exist_is_not_alive() {
        // Above every platform's pid_max, so this is never a live process.
        assert!(!is_daemon_alive(u32::MAX - 1));
    }

    #[test]
    fn an_unrelated_live_process_is_not_taken_for_the_daemon() {
        // This test binary is alive but is not a `serve-http` daemon, so the
        // identity check must reject it. This is exactly the recycled-PID case
        // stop_daemon must refuse to kill.
        assert!(is_daemon_alive(std::process::id()));
        assert!(!pid_is_open_ontologies_daemon(std::process::id()));
    }

    #[test]
    fn daemon_info_round_trips_through_the_data_dir() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_string_lossy().to_string();
        assert!(read_daemon_info(&path).is_none());

        let info = DaemonInfo {
            pid: 4242,
            url: "http://127.0.0.1:8080".into(),
            token: Some("s3cret".into()),
        };
        write_daemon_info(&path, &info).unwrap();

        let read = read_daemon_info(&path).expect("daemon.json is readable");
        assert_eq!(read.pid, 4242);
        assert_eq!(read.url, "http://127.0.0.1:8080");
        // The token has to survive the round trip: it is what the proxy sends as
        // its bearer credential, and dropping it is a 401 on every command.
        assert_eq!(read.token.as_deref(), Some("s3cret"));

        remove_daemon_info(&path);
        assert!(read_daemon_info(&path).is_none());
    }

    #[test]
    fn a_stale_daemon_file_is_detectable_as_stale() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_string_lossy().to_string();
        write_daemon_info(
            &path,
            &DaemonInfo { pid: u32::MAX - 1, url: "http://127.0.0.1:8080".into(), token: None },
        )
        .unwrap();

        let info = read_daemon_info(&path).unwrap();
        assert!(!is_daemon_alive(info.pid), "a dead PID must read as dead");

        // Which is what lets the caller clear it and fall through to local
        // execution instead of proxying into nothing.
        remove_daemon_info(&path);
        assert!(read_daemon_info(&path).is_none());
    }
}
