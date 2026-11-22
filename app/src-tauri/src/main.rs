#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ipc;

use meeting_copilot_ui_bridge::IPC_PROTOCOL_VERSION;

#[tauri::command]
const fn ipc_version() -> u16 {
    IPC_PROTOCOL_VERSION
}

fn main() {
    tauri::Builder::default()
        .setup(|_app| {
            tracing::info!(
                protocol = IPC_PROTOCOL_VERSION,
                "meeting-copilot desktop bootstrapped"
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![ipc_version])
        .run(tauri::generate_context!())
        .expect("failed to run meeting-copilot desktop");
}
