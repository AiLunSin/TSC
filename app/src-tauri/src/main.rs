#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ipc;

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use meeting_copilot_application::settings::{
    CardSnapshotEntry, CardSnapshotSource, SecureStore, ViewFactsSnapshot, ViewFactsSource,
};
use meeting_copilot_platform_win::secure_store::WindowsCredentialStore;
use meeting_copilot_ui_bridge::settings::SettingsFormState;
use meeting_copilot_ui_bridge::IPC_PROTOCOL_VERSION;
use tauri::Manager;

use ipc::catalog::SettingsCatalog;
use ipc::dto::ProbeReportDto;
use ipc::probe::GatewayProbeExecutor;
use ipc::settings::AppFactsSource;
use ipc::settings_store::FileSettingsConfigStore;
use ipc::state::{AppState, CommandCounters};

/// 无环境注入时的默认模型预设；只含显示名，不含端点或凭据。
const DEFAULT_MODEL_PRESET: &str = "deepseek-flash";

struct UsingPendingSources {
    view: ViewFactsSnapshot,
}

impl CardSnapshotSource for UsingPendingSources {
    fn cards(&self) -> Vec<CardSnapshotEntry> {
        Vec::new()
    }
}

impl ViewFactsSource for UsingPendingSources {
    fn view_facts(&self) -> ViewFactsSnapshot {
        self.view
    }
}

fn pending_view() -> ViewFactsSnapshot {
    ViewFactsSnapshot {
        session_matches: false,
        authorization_revoked: false,
        reference_stale: false,
        screen_share: false,
        emergency_masked: false,
        visible_card_count: 0,
    }
}

fn build_state(config_dir: PathBuf, presets: Vec<String>) -> AppState {
    let secure: Arc<dyn SecureStore> = Arc::new(WindowsCredentialStore::new());
    let memory_key = Arc::new(Mutex::new(None));
    let cooldown_until_ms = Arc::new(Mutex::new(None));
    let probe_in_flight = Arc::new(AtomicBool::new(false));
    let catalog = SettingsCatalog::from_injected(presets);
    let facts = Arc::new(AppFactsSource {
        secure: Arc::clone(&secure),
        memory_key: Arc::clone(&memory_key),
        cooldown_until_ms: Arc::clone(&cooldown_until_ms),
        probe_in_flight: Arc::clone(&probe_in_flight),
        model_count: catalog.model_count(),
        endpoint_count: catalog.endpoint_count(),
    });
    AppState {
        secure,
        config: Arc::new(FileSettingsConfigStore::new(config_dir)),
        probe: Arc::new(GatewayProbeExecutor),
        facts,
        cards: Arc::new(UsingPendingSources {
            view: pending_view(),
        }),
        view_facts: Arc::new(UsingPendingSources {
            view: pending_view(),
        }),
        catalog,
        memory_key,
        form_state: Mutex::new(SettingsFormState::NotConfigured),
        probe_in_flight,
        cooldown_until_ms,
        last_probe_report: Mutex::new(None::<ProbeReportDto>),
        counters: Mutex::new(CommandCounters::default()),
    }
}

#[tauri::command]
const fn ipc_version() -> u16 {
    IPC_PROTOCOL_VERSION
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let config_dir = app
                .path()
                .app_config_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            let presets = std::env::var("LLM_MODEL")
                .ok()
                .filter(|model| !model.trim().is_empty())
                .map(|model| vec![model])
                .unwrap_or_else(|| vec![DEFAULT_MODEL_PRESET.to_owned()]);
            app.manage(build_state(config_dir, presets));
            tracing::info!(
                protocol = IPC_PROTOCOL_VERSION,
                "meeting-copilot desktop bootstrapped"
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ipc_version,
            ipc::settings_snapshot,
            ipc::settings_probe,
            ipc::settings_save,
            ipc::settings_clear,
            ipc::keyword_cards_snapshot,
            ipc::keyword_card_open
        ])
        .run(tauri::generate_context!())
        .expect("failed to run meeting-copilot desktop");
}
