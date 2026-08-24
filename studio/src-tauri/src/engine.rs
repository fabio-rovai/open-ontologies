use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};

pub struct EngineState {
    pub running: Arc<AtomicBool>,
    pub child: Mutex<Option<Child>>,
}

fn engine_binary_names() -> &'static [&'static str] {
    #[cfg(target_os = "windows")]
    {
        &["open-ontologies-x86_64-pc-windows-msvc.exe", "open-ontologies.exe"]
    }
    #[cfg(target_os = "macos")]
    {
        &[
            "open-ontologies-aarch64-apple-darwin",
            "open-ontologies-x86_64-apple-darwin",
            "open-ontologies",
        ]
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        &[
            "open-ontologies-x86_64-unknown-linux-gnu",
            "open-ontologies-x86_64-unknown-linux-musl",
            "open-ontologies",
        ]
    }
}

fn find_existing_binary(dir: &Path) -> Option<PathBuf> {
    engine_binary_names()
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.exists())
}

fn resolve_engine_binary() -> Result<PathBuf, String> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bundled_dir = manifest_dir.join("binaries");
    if let Some(path) = find_existing_binary(&bundled_dir) {
        return Ok(path);
    }

    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or("Cannot resolve workspace root from Tauri manifest directory")?;
    for profile in ["release", "debug"] {
        let candidate_dir = workspace_root.join("target").join(profile);
        if let Some(path) = find_existing_binary(&candidate_dir) {
            return Ok(path);
        }
    }

    Err(format!(
        "No open-ontologies binary found. Checked {} and workspace target/{{release,debug}}.",
        bundled_dir.display()
    ))
}

fn stale_pids_on_port(port: u16) -> Vec<u32> {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("netstat")
            .args(["-ano", "-p", "tcp"])
            .output();
        let Ok(output) = output else {
            return Vec::new();
        };
        let needle = format!(":{}", port);
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| line.contains("LISTENING") && line.contains(&needle))
            .filter_map(|line| line.split_whitespace().last())
            .filter_map(|pid| pid.parse::<u32>().ok())
            .collect()
    }
    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("lsof")
            .args(["-ti", &format!("tcp:{}", port)])
            .output();
        let Ok(output) = output else {
            return Vec::new();
        };
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| line.trim().parse::<u32>().ok())
            .collect()
    }
}

fn kill_process(pid: u32) {
    #[cfg(target_os = "windows")]
    let _ = Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    #[cfg(not(target_os = "windows"))]
    let _ = Command::new("kill")
        .args(["-9", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn clear_stale_port(port: u16) {
    for pid in stale_pids_on_port(port) {
        kill_process(pid);
    }
}

const DEFAULT_ENGINE_PORT: u16 = 8137;

fn parse_port(raw: Option<&str>) -> u16 {
    raw.and_then(|value| value.trim().parse::<u16>().ok())
        .filter(|port| *port != 0)
        .unwrap_or(DEFAULT_ENGINE_PORT)
}

/// Single source of truth for the engine's HTTP port. Every consumer of the
/// port (the REST base URL handed to the frontend, the MCP endpoint the
/// backend calls, and the environment variable passed to the agent sidecar)
/// derives its value from this function rather than resolving it again.
pub(crate) fn engine_port() -> u16 {
    let raw = std::env::var("OPEN_ONTOLOGIES_STUDIO_PORT").ok();
    parse_port(raw.as_deref())
}

/// Exposes the resolved engine port to the frontend so it does not need its
/// own copy of the port-resolution logic (and cannot drift from Rust's view
/// of it).
#[tauri::command]
pub fn get_engine_port() -> u16 {
    engine_port()
}

pub fn spawn_engine(app: &tauri::AppHandle) -> Result<(), String> {
    let binary = resolve_engine_binary()?;
    let port = engine_port();
    clear_stale_port(port);
    std::thread::sleep(std::time::Duration::from_millis(300));

    eprintln!("[engine] spawning {}", binary.display());

    let mut child = Command::new(&binary)
        .args(["serve-http", "--port", &port.to_string()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn engine {}: {e}", binary.display()))?;

    let stderr = child.stderr.take().ok_or("No stderr")?;
    let app_handle = app.clone();
    std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                eprintln!("[engine] {}", line);
                if line.contains("listening") || line.contains("Listening") || line.contains(&port.to_string())
                {
                    let _ = app_handle.emit("engine-ready", true);
                }
            }
        }
        let _ = app_handle.emit("engine-stopped", true);
    });

    let state = app.state::<EngineState>();
    *state.child.lock().map_err(|e| format!("Lock error: {e}"))? = Some(child);

    let app_handle2 = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(2));
        let _ = app_handle2.emit("engine-ready", true);
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_defaults_to_8137_and_never_to_8080() {
        assert_eq!(parse_port(None), 8137);
        assert_eq!(parse_port(Some("9001")), 9001);
        assert_eq!(parse_port(Some("not-a-port")), 8137);
    }
}
