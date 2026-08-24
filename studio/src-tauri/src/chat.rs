use std::io::BufRead;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use tauri::{Emitter, Manager};

use crate::engine;

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

/// Where the sidecar lives inside a directory, whether that is a Tauri
/// resource directory or the source checkout used during development.
fn sidecar_relative_path() -> &'static str {
    "sidecars/agent/dist/index.js"
}

fn dev_sidecar_entry_under(manifest_dir: &Path) -> PathBuf {
    manifest_dir.join(sidecar_relative_path())
}

// Only exercised directly by tests; production code goes through
// `resolve_sidecar_entry`, which takes the manifest dir as a parameter so
// it can be swapped for a temp directory in tests.
#[cfg(test)]
fn dev_sidecar_entry() -> PathBuf {
    dev_sidecar_entry_under(Path::new(env!("CARGO_MANIFEST_DIR")))
}

/// Pure resolution logic, unit-testable without a `tauri::AppHandle`.
///
/// `bundled_resource_dir` is the app's resource directory when one is
/// available (the packaged case); `dev_manifest_dir` is the crate root to
/// fall back to for local development, and is parameterised so tests can
/// control both branches with temp directories instead of the real
/// `CARGO_MANIFEST_DIR` checkout.
fn resolve_sidecar_entry(
    bundled_resource_dir: Option<&Path>,
    dev_manifest_dir: &Path,
) -> Result<PathBuf, String> {
    if let Some(dir) = bundled_resource_dir {
        let bundled = dir.join(sidecar_relative_path());
        if bundled.exists() {
            return Ok(bundled);
        }
    }
    let dev = dev_sidecar_entry_under(dev_manifest_dir);
    if dev.exists() {
        return Ok(dev);
    }
    let bundled_display = bundled_resource_dir
        .map(|dir| dir.join(sidecar_relative_path()).display().to_string())
        .unwrap_or_else(|| "<no app resource directory available>".to_string());
    Err(format!(
        "Agent sidecar not found. Looked in the app resource directory ({}) and at {}. \
         Run `npm run build` in studio/src-tauri/sidecars/agent to produce dist/index.js.",
        bundled_display,
        dev.display()
    ))
}

fn sidecar_entry(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    resolve_sidecar_entry(
        app.path().resource_dir().ok().as_deref(),
        Path::new(env!("CARGO_MANIFEST_DIR")),
    )
}

pub fn spawn_agent_sidecar(app: &tauri::AppHandle) -> Result<(), String> {
    let entry = sidecar_entry(app)?;
    let node = resolve_node_binary();

    // Same port the engine itself was spawned with (see engine::engine_port,
    // the single place that resolves it). Passed through the environment
    // rather than re-resolved here so the sidecar can't drift from it.
    let port = engine::engine_port();

    let mut child = Command::new(&node)
        .arg(&entry)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("PATH", augmented_path())
        .env("OPEN_ONTOLOGIES_STUDIO_PORT", port.to_string())
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
    use std::fs;

    #[test]
    fn dev_fallback_points_at_the_source_sidecar() {
        let path = dev_sidecar_entry();
        assert!(
            path.ends_with("sidecars/agent/dist/index.js"),
            "unexpected dev sidecar path: {}",
            path.display()
        );
    }

    /// Creates `<dir>/sidecars/agent/dist/index.js` under a fresh temp
    /// directory and returns the temp directory root.
    fn make_sidecar_tree() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().expect("create temp dir");
        let dist = tmp.path().join("sidecars/agent/dist");
        fs::create_dir_all(&dist).expect("create dist dir");
        fs::write(dist.join("index.js"), b"// stub sidecar\n").expect("write stub sidecar");
        tmp
    }

    #[test]
    fn bundled_path_wins_when_it_exists() {
        let bundled_root = make_sidecar_tree();
        let dev_root = tempfile::tempdir().expect("create temp dir"); // no sidecar under here

        let resolved = resolve_sidecar_entry(Some(bundled_root.path()), dev_root.path())
            .expect("should resolve to the bundled sidecar");

        assert_eq!(
            resolved,
            bundled_root.path().join("sidecars/agent/dist/index.js")
        );
    }

    #[test]
    fn dev_path_is_used_when_bundled_is_missing() {
        let bundled_root = tempfile::tempdir().expect("create temp dir"); // no sidecar under here
        let dev_root = make_sidecar_tree();

        let resolved = resolve_sidecar_entry(Some(bundled_root.path()), dev_root.path())
            .expect("should fall back to the dev sidecar");

        assert_eq!(
            resolved,
            dev_root.path().join("sidecars/agent/dist/index.js")
        );
    }

    #[test]
    fn dev_path_is_used_when_no_resource_dir_is_available() {
        let dev_root = make_sidecar_tree();

        let resolved =
            resolve_sidecar_entry(None, dev_root.path()).expect("should fall back to dev sidecar");

        assert_eq!(
            resolved,
            dev_root.path().join("sidecars/agent/dist/index.js")
        );
    }

    #[test]
    fn error_names_both_locations_when_neither_exists() {
        let bundled_root = tempfile::tempdir().expect("create temp dir");
        let dev_root = tempfile::tempdir().expect("create temp dir");

        let err = resolve_sidecar_entry(Some(bundled_root.path()), dev_root.path())
            .expect_err("neither location has a sidecar, so this must fail");

        let expected_bundled = bundled_root
            .path()
            .join("sidecars/agent/dist/index.js")
            .display()
            .to_string();
        let expected_dev = dev_root
            .path()
            .join("sidecars/agent/dist/index.js")
            .display()
            .to_string();

        assert!(
            err.contains(&expected_bundled),
            "error does not name the bundled location: {err}"
        );
        assert!(
            err.contains(&expected_dev),
            "error does not name the dev location: {err}"
        );
    }
}
