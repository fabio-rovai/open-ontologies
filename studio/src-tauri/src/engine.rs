use std::io::BufRead;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};

pub struct EngineState {
    pub running: Arc<AtomicBool>,
    pub child: Mutex<Option<Child>>,
}

pub fn spawn_engine(app: &tauri::AppHandle) -> Result<(), String> {
    let binaries_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries");

    // Find the engine binary (e.g. open-ontologies-aarch64-apple-darwin)
    let binary = std::fs::read_dir(&binaries_dir)
        .map_err(|e| format!("Cannot list binaries dir {}: {e}", binaries_dir.display()))?
        .filter_map(|e| e.ok())
        .find(|e| {
            let name = e.file_name();
            let s = name.to_string_lossy();
            s.starts_with("open-ontologies") && !s.ends_with(".d")
        })
        .map(|e| e.path())
        .ok_or_else(|| "No open-ontologies binary found in binaries/".to_string())?;

    // Kill any stale process on port 8080 (e.g. from a previous hot-reload cycle)
    let _ = Command::new("sh")
        .args(["-c", "lsof -ti:8080 | xargs kill -9 2>/dev/null; true"])
        .output();
    std::thread::sleep(std::time::Duration::from_millis(300));

    eprintln!("[engine] spawning {}", binary.display());

    let mut child = Command::new(&binary)
        .args(["serve-http", "--port", "8080"])
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
                if line.contains("listening") || line.contains("Listening") || line.contains("8080") {
                    let _ = app_handle.emit("engine-ready", true);
                }
            }
        }
        let _ = app_handle.emit("engine-stopped", true);
    });

    let state = app.state::<EngineState>();
    *state.child.lock().map_err(|e| format!("Lock error: {e}"))? = Some(child);

    // Emit ready after 2s as fallback
    let app_handle2 = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(2));
        let _ = app_handle2.emit("engine-ready", true);
    });

    Ok(())
}
