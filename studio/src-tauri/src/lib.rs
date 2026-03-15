mod chat;
mod engine;
mod mcp;

use chat::ChatState;
use engine::EngineState;
use mcp::McpState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::Manager;

#[tauri::command]
fn engine_status(state: tauri::State<EngineState>) -> bool {
    state.running.load(Ordering::Relaxed)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(EngineState {
            running: Arc::new(AtomicBool::new(false)),
            child: Mutex::new(None),
        })
        .manage(ChatState {
            process: Mutex::new(None),
        })
        .manage(McpState {
            session_id: Mutex::new(None),
            client: reqwest::Client::new(),
        })
        .setup(|app| {
            let handle = app.handle().clone();
            if let Err(e) = engine::spawn_engine(&handle) {
                eprintln!("Failed to start engine: {e}");
            } else {
                app.state::<EngineState>().running.store(true, Ordering::Relaxed);
            }

            // Spawn agent sidecar after a delay to let the engine start first
            let agent_handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(3));
                if let Err(e) = chat::spawn_agent_sidecar(&agent_handle) {
                    eprintln!("Failed to start agent sidecar: {e}");
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            engine_status,
            mcp::mcp_call,
            mcp::set_mcp_session,
            chat::send_chat_message,
            chat::reset_chat
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                // Kill agent sidecar
                if let Some(mut child) = app_handle
                    .state::<ChatState>()
                    .process
                    .lock()
                    .ok()
                    .and_then(|mut g| g.take())
                {
                    let _ = child.kill();
                }
                // Kill engine
                if let Some(mut child) = app_handle
                    .state::<EngineState>()
                    .child
                    .lock()
                    .ok()
                    .and_then(|mut g| g.take())
                {
                    let _ = child.kill();
                }
            }
        });
}
