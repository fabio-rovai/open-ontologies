use std::io::BufRead;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use tauri::{Emitter, Manager};

pub struct ChatState {
    pub process: Mutex<Option<Child>>,
}

fn resolve_node_binary() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let candidates = [
            PathBuf::from(r"C:\Program Files\nodejs\node.exe"),
            PathBuf::from(r"C:\Program Files (x86)\nodejs\node.exe"),
        ];
        if let Some(path) = candidates.into_iter().find(|path| path.exists()) {
            return path;
        }
        PathBuf::from("node")
    }
    #[cfg(not(target_os = "windows"))]
    {
        let candidates = [
            PathBuf::from("/opt/homebrew/bin/node"),
            PathBuf::from("/usr/local/bin/node"),
            PathBuf::from("/usr/bin/node"),
        ];
        if let Some(path) = candidates.into_iter().find(|path| path.exists()) {
            return path;
        }
        PathBuf::from("node")
    }
}

fn augmented_path() -> String {
    let mut parts: Vec<String> = Vec::new();
    #[cfg(target_os = "windows")]
    {
        parts.push(r"C:\Program Files\nodejs".to_string());
        if let Some(home) = std::env::var_os("USERPROFILE") {
            parts.push(format!(r"{}\.\cargo\bin", home.to_string_lossy()));
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        parts.push("/opt/homebrew/bin".to_string());
        parts.push("/usr/local/bin".to_string());
        parts.push("/usr/bin".to_string());
    }
    if let Ok(existing) = std::env::var("PATH") {
        parts.push(existing);
    }
    let separator = if cfg!(target_os = "windows") { ";" } else { ":" };
    parts.join(separator)
}

fn dev_sidecar_entry() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sidecars/agent/dist/index.js")
}

fn sidecar_entry(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    if let Ok(dir) = app.path().resource_dir() {
        let bundled = dir.join("sidecars/agent/dist/index.js");
        if bundled.exists() {
            return Ok(bundled);
        }
    }
    let dev = dev_sidecar_entry();
    if dev.exists() {
        return Ok(dev);
    }
    Err(format!(
        "Agent sidecar not found. Looked in the app resource directory and at {}. \
         Run `npm run build` in studio/src-tauri/sidecars/agent to produce dist/index.js.",
        dev.display()
    ))
}

pub fn spawn_agent_sidecar(app: &tauri::AppHandle) -> Result<(), String> {
    let entry = sidecar_entry(app)?;
    let node = resolve_node_binary();

    let mut child = Command::new(&node)
        .arg(&entry)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("PATH", augmented_path())
        .spawn()
        .map_err(|e| format!("Failed to spawn agent sidecar: {e}"))?;

    let stdout = child.stdout.take().ok_or("No stdout")?;
    let app_handle = app.clone();

    std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                let _ = app_handle.emit("agent-message", line);
            }
        }
    });

    let stderr = child.stderr.take().ok_or("No stderr")?;
    std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                eprintln!("[agent stderr] {}", line);
            }
        }
    });

    let state = app.state::<ChatState>();
    *state.process.lock().map_err(|e| format!("Lock error: {e}"))? = Some(child);

    Ok(())
}

#[tauri::command]
pub fn send_chat_message(
    message: String,
    mode: String,
    state: tauri::State<ChatState>,
) -> Result<(), String> {
    let mut guard = state.process.lock().map_err(|e| format!("Lock error: {e}"))?;
    let child = guard.as_mut().ok_or("Agent sidecar not running")?;
    let stdin = child.stdin.as_mut().ok_or("No stdin")?;

    let payload = serde_json::json!({ "type": "chat", "message": message, "mode": mode });
    writeln!(stdin, "{}", payload).map_err(|e| format!("Write failed: {e}"))?;
    stdin.flush().map_err(|e| format!("Flush failed: {e}"))?;

    Ok(())
}

#[tauri::command]
pub fn reset_chat(state: tauri::State<ChatState>) -> Result<(), String> {
    let mut guard = state.process.lock().map_err(|e| format!("Lock error: {e}"))?;
    let child = guard.as_mut().ok_or("Agent sidecar not running")?;
    let stdin = child.stdin.as_mut().ok_or("No stdin")?;

    let payload = serde_json::json!({ "type": "reset" });
    writeln!(stdin, "{}", payload).map_err(|e| format!("Write failed: {e}"))?;
    stdin.flush().map_err(|e| format!("Flush failed: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_fallback_points_at_the_source_sidecar() {
        let path = dev_sidecar_entry();
        assert!(
            path.ends_with("sidecars/agent/dist/index.js"),
            "unexpected dev sidecar path: {}",
            path.display()
        );
    }
}
