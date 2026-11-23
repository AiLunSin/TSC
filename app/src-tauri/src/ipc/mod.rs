//! IPC 装配层：只读 DTO、运行态、目录清单与 13 个固定命令。

pub mod asr;
pub mod capture;
mod card_gate;
pub mod card_store;
pub mod cards;
pub mod catalog;
pub mod dto;
pub mod log;
pub mod meeting;
pub mod probe;
pub mod settings;
pub mod settings_store;
pub mod state;

use tauri::State;

use crate::ipc::dto::{
    CardDetailDto, ClearInputDto, IpcErrorDto, KeywordCardViewDto, OpenInputDto, ProbeInputDto,
    ProbeReportDto, SaveInputDto, SettingsClearReceiptDto, SettingsSaveReceiptDto,
    SettingsSnapshotDto,
};
use crate::ipc::state::AppState;

macro_rules! with_counter {
    ($state:expr, $ok:ident, $rejected:ident, $body:expr) => {{
        let result = $body;
        if let Ok(mut counters) = $state.counters.lock() {
            if result.is_ok() {
                counters.$ok = counters.$ok.saturating_add(1);
            } else {
                counters.$rejected = counters.$rejected.saturating_add(1);
            }
        }
        result
    }};
}

#[tauri::command]
pub fn settings_snapshot(state: State<'_, AppState>) -> Result<SettingsSnapshotDto, IpcErrorDto> {
    with_counter!(
        state.inner(),
        settings_snapshot_ok,
        settings_snapshot_rejected,
        settings::settings_snapshot_impl(state.inner())
    )
}

#[tauri::command]
pub fn settings_probe(
    state: State<'_, AppState>,
    input: ProbeInputDto,
) -> Result<ProbeReportDto, IpcErrorDto> {
    with_counter!(
        state.inner(),
        settings_probe_ok,
        settings_probe_rejected,
        probe::probe_settings_impl(state.inner(), input)
    )
}

#[tauri::command]
pub fn settings_save(
    state: State<'_, AppState>,
    input: SaveInputDto,
) -> Result<SettingsSaveReceiptDto, IpcErrorDto> {
    with_counter!(
        state.inner(),
        settings_save_ok,
        settings_save_rejected,
        settings::settings_save_impl(state.inner(), input)
    )
}

#[tauri::command]
pub fn settings_clear(
    state: State<'_, AppState>,
    input: ClearInputDto,
) -> Result<SettingsClearReceiptDto, IpcErrorDto> {
    with_counter!(
        state.inner(),
        settings_clear_ok,
        settings_clear_rejected,
        settings::settings_clear_impl(state.inner(), input)
    )
}

#[tauri::command]
pub fn keyword_cards_snapshot(
    state: State<'_, AppState>,
) -> Result<Vec<KeywordCardViewDto>, IpcErrorDto> {
    with_counter!(
        state.inner(),
        keyword_cards_snapshot_ok,
        keyword_cards_snapshot_rejected,
        cards::keyword_cards_snapshot_impl(state.inner())
    )
}

#[tauri::command]
pub fn keyword_card_open(
    state: State<'_, AppState>,
    input: OpenInputDto,
) -> Result<CardDetailDto, IpcErrorDto> {
    with_counter!(
        state.inner(),
        keyword_card_open_ok,
        keyword_card_open_rejected,
        cards::keyword_card_open_impl(state.inner(), input)
    )
}
