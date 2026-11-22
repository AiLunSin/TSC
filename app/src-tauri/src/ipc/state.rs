//! 运行态：6 端口/事实源、内存降级 Key、单飞与诊断计数。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use meeting_copilot_application::settings::{
    CardSnapshotSource, ProbeExecutor, SecureStore, SettingsConfigStore, SettingsFactsSource,
    ViewFactsSource,
};
use meeting_copilot_ui_bridge::settings::SettingsFormState;

use crate::ipc::catalog::SettingsCatalog;
use crate::ipc::dto::ProbeReportDto;

/// 6 组命令的成功/拒绝计数（12 项）。
#[derive(Debug, Default)]
pub struct CommandCounters {
    pub settings_snapshot_ok: u64,
    pub settings_snapshot_rejected: u64,
    pub settings_probe_ok: u64,
    pub settings_probe_rejected: u64,
    pub settings_save_ok: u64,
    pub settings_save_rejected: u64,
    pub settings_clear_ok: u64,
    pub settings_clear_rejected: u64,
    pub keyword_cards_snapshot_ok: u64,
    pub keyword_cards_snapshot_rejected: u64,
    pub keyword_card_open_ok: u64,
    pub keyword_card_open_rejected: u64,
}

/// 装配层运行态。
pub struct AppState {
    pub secure: Arc<dyn SecureStore>,
    pub config: Arc<dyn SettingsConfigStore>,
    pub probe: Arc<dyn ProbeExecutor>,
    pub facts: Arc<dyn SettingsFactsSource>,
    pub cards: Arc<dyn CardSnapshotSource>,
    pub view_facts: Arc<dyn ViewFactsSource>,
    pub catalog: SettingsCatalog,
    pub memory_key: Mutex<Option<String>>,
    pub form_state: Mutex<SettingsFormState>,
    pub probe_in_flight: AtomicBool,
    pub cooldown_until_ms: Mutex<Option<u64>>,
    pub last_probe_report: Mutex<Option<ProbeReportDto>>,
    pub counters: Mutex<CommandCounters>,
}

/// 单飞守卫：离开作用域必复位在途标记。
pub struct ProbeGuard {
    flag: Arc<AtomicBool>,
}

impl ProbeGuard {
    /// 取得单飞权；已在途时返回 `None`。
    pub fn acquire(flag: &Arc<AtomicBool>) -> Option<Self> {
        flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Self {
                flag: Arc::clone(flag),
            })
    }
}

impl Drop for ProbeGuard {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::Release);
    }
}
