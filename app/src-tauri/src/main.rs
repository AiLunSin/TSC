#![windows_subsystem = "windows"]

mod diag;
mod ipc;
mod privacy;

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use meeting_copilot_application::settings::{
    CardSnapshotEntry, CardSnapshotSource, SecureStore, ViewFactsSnapshot, ViewFactsSource,
};
use meeting_copilot_platform_win::secure_store::{
    WindowsCredentialStore, SEARCH_CREDENTIAL_TARGET_BOCHA, SEARCH_CREDENTIAL_TARGET_TAVILY,
};
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

/// 当前会话的视图事实。
///
/// `session_matches` 必须是 **true**：桌面端只有一个进程内会话，卡片就是在这个会话里
/// 造出来的（`build_draft` 的 `candidate.session_id` 与 `current_session` 同源）。
/// 早先这里硬编码 false，等于告诉门控"卡片属于别的会话"，于是**每张卡一生成就是
/// 「引用已过期 / 不可点击」**，用户点不开详情。
fn active_view(privacy: crate::privacy::PrivacyMode) -> ViewFactsSnapshot {
    ViewFactsSnapshot {
        session_matches: true,
        authorization_revoked: false,
        reference_stale: false,
        screen_share: privacy.screen_share,
        emergency_masked: privacy.emergency_masked,
        visible_card_count: 0,
    }
}

fn now_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

fn build_state(
    config_dir: PathBuf,
    presets: Vec<String>,
    cards: Arc<dyn CardSnapshotSource>,
) -> AppState {
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
        cards,
        view_facts: Arc::new(UsingPendingSources {
            view: active_view(crate::privacy::PrivacyMode::from_env()),
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

/// 把 tracing 事件写入 `app_log_dir/desktop.log`；超过 5 MiB 时重写。
fn init_file_logging(dir: &std::path::Path) {
    let _ = std::fs::create_dir_all(dir);
    let path = dir.join("desktop.log");
    let oversized = std::fs::metadata(&path).is_ok_and(|meta| meta.len() > 5 * 1024 * 1024);
    if oversized {
        let _ = std::fs::remove_file(&path);
    }
    let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    else {
        return;
    };
    tracing_subscriber::fmt()
        .with_writer(std::sync::Mutex::new(file))
        .with_ansi(false)
        .init();
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let log_dir = app
                .path()
                .app_log_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            init_file_logging(&log_dir);
            let config_dir = app
                .path()
                .app_config_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            let presets = std::env::var("LLM_MODEL")
                .ok()
                .filter(|model| !model.trim().is_empty())
                .map(|model| vec![model])
                .unwrap_or_else(|| vec![DEFAULT_MODEL_PRESET.to_owned()]);
            // 断网补齐：库密钥在 OS 凭据库；库打不开时补齐停用，检索本身仍可用。
            let backfill = ipc::backfill::BackfillBridge::open(&config_dir).map(Arc::new);
            // 卡片持久化与补齐共用同一单库；启动回灌，避免重启后卡片列表清空。
            let cards = Arc::new(ipc::card_store::CardStore::new_persistent(
                backfill.as_ref().map(|bridge| bridge.store()),
            ));
            cards.restore();
            app.manage(Arc::clone(&cards));
            let transcripts = Arc::new(ipc::transcript::TranscriptLog::new());
            app.manage(Arc::clone(&transcripts));
            let asr_paths = ipc::asr::AsrPaths::resolve(app.path().resource_dir().ok().as_deref());
            app.manage(Arc::new(ipc::asr::AsrPipeline::new(
                Arc::clone(&cards),
                Arc::clone(&transcripts),
                asr_paths,
            )));
            // 检索凭据与模型凭据分开存；博查与 Tavily 也各存一份，切换服务不互相覆盖。
            let search_bocha: Arc<dyn SecureStore> = Arc::new(
                WindowsCredentialStore::with_target_name(SEARCH_CREDENTIAL_TARGET_BOCHA),
            );
            let search_tavily: Arc<dyn SecureStore> = Arc::new(
                WindowsCredentialStore::with_target_name(SEARCH_CREDENTIAL_TARGET_TAVILY),
            );
            // 断网补齐与卡片持久化已在上方装配；这里只构造检索运行态。
            let search_state = Arc::new(ipc::search::SearchState::new(
                config_dir.clone(),
                search_bocha,
                search_tavily,
                Arc::clone(&cards),
                backfill,
            ));
            app.manage(Arc::clone(&search_state));
            let ticker = Arc::clone(&search_state);
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_millis(500));
                ticker.tick(now_unix_ms());
            });
            app.manage(Arc::new(ipc::assist::AssistState::new(config_dir.clone())));
            app.manage(Arc::new(ipc::ui_prefs::UiPrefsState::new(
                config_dir.clone(),
            )));
            app.manage(build_state(config_dir, presets, cards.clone()));
            tracing::info!(
                protocol = IPC_PROTOCOL_VERSION,
                "meeting-copilot desktop bootstrapped"
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ipc_version,
            ipc::log::open_log,
            ipc::meeting::meeting_status,
            ipc::capture::capture_status,
            ipc::card_store::ingest_text,
            ipc::card_store::clear_cards,
            ipc::quality::card_feedback,
            ipc::quality::card_rendered,
            ipc::quality::quality_metrics,
            ipc::asr::start_asr,
            ipc::asr::stop_asr,
            ipc::asr::asr_status,
            ipc::asr::set_auto_cards,
            ipc::settings_snapshot,
            ipc::settings_probe,
            ipc::settings_save,
            ipc::settings_clear,
            ipc::keyword_cards_snapshot,
            ipc::keyword_card_open,
            ipc::transcript::transcript_snapshot,
            ipc::explain::explain_selection,
            ipc::explain::ingest_explanation,
            ipc::search::search_config,
            ipc::search::search_config_save,
            ipc::search::search_clear_key,
            ipc::search::web_search,
            ipc::assist::assist_reply,
            ipc::assist::assist_config,
            ipc::assist::assist_config_save,
            ipc::ui_prefs::ui_prefs,
            ipc::ui_prefs::ui_prefs_save
        ])
        .run(tauri::generate_context!())
        .expect("failed to run meeting-copilot desktop");
}
