use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::io::BufRead;
use tauri::{Emitter, Manager};

pub struct ChatState {
    pub process: Mutex<Option<Child>>,
}

pub fn spawn_agent_sidecar(app: &tauri::AppHandle) -> Result<(), String> {
    let sidecar_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sidecars/agent");

    let mut child = Command::new("node")
        .arg(sidecar_dir.join("dist/index.js"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("PATH", format!("/opt/homebrew/bin:/usr/local/bin:/usr/bin:{}", std::env::var("PATH").unwrap_or_default()))
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
pub fn send_chat_message(message: String, state: tauri::State<ChatState>) -> Result<(), String> {
    let mut guard = state.process.lock().map_err(|e| format!("Lock error: {e}"))?;
    let child = guard.as_mut().ok_or("Agent sidecar not running")?;
    let stdin = child.stdin.as_mut().ok_or("No stdin")?;

    let payload = serde_json::json!({ "type": "chat", "message": message });
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
