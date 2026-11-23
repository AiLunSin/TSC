//! 设置命令实现：快照投影、先安全存储后配置、确认后清除。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use meeting_copilot_application::settings::{
    FactsSnapshot, SecretBytes, SecureStore, SettingsConfig, SettingsFactsSource,
};
use meeting_copilot_ui_bridge::settings::{
    apply_defaults, project_settings_view, EndpointSelectionView, FieldIssue, MaskedKey,
    ModelSelectionView, SecretInput, SettingsDefaults, SettingsFacts, SettingsFormState,
    SettingsView, PERSIST_FALLBACK_NOTICE, SECTION_ORDER,
};

use crate::ipc::dto::{
    ClearInputDto, EndpointChoiceDto, IpcErrorDto, ModelChoiceDto, SaveInputDto,
    SettingsClearReceiptDto, SettingsSaveReceiptDto, SettingsSnapshotDto,
};
use crate::ipc::state::AppState;

/// 设置快照：读取事实与配置，投影为 11 项只读 DTO。
pub fn settings_snapshot_impl(state: &AppState) -> Result<SettingsSnapshotDto, IpcErrorDto> {
    let snapshot = state.facts.facts();
    let config = state
        .config
        .load()
        .unwrap_or_default()
        .normalized(snapshot.model_count, snapshot.endpoint_count);
    let facts = SettingsFacts {
        has_stored_key: snapshot.has_stored_key,
        model_count: snapshot.model_count,
        endpoint_count: snapshot.endpoint_count,
        now_ms: snapshot.now_ms,
        cooldown_until_ms: snapshot.cooldown_until_ms,
        probe_in_flight: snapshot.probe_in_flight,
    };
    let form_state = *state.form_state.lock().map_err(|_| internal_error())?;
    let endpoints: Vec<String> = state
        .catalog
        .endpoints
        .iter()
        .map(|endpoint| endpoint.display_name.clone())
        .collect();
    let defaults = SettingsDefaults {
        model_index: config.model_index,
        endpoint_index: config.endpoint_index,
        timeout_ms: config.timeout_ms,
        auto_save_on_verified: true,
    };
    let input = apply_defaults(&facts, &defaults);
    let view = project_settings_view(
        &facts,
        &input,
        form_state,
        &state.catalog.presets,
        &endpoints,
    );
    Ok(to_snapshot_dto(state, &view))
}

/// 保存：先写安全存储，失败降级内存并如实回执。
pub fn settings_save_impl(
    state: &AppState,
    input: SaveInputDto,
) -> Result<SettingsSaveReceiptDto, IpcErrorDto> {
    let secret_input = SecretInput::from_draft(&input.key);
    if let Some(issue) = secret_input.draft_issue() {
        return Err(field_error(issue));
    }
    if input.model_index >= state.catalog.model_count() {
        return Err(field_error(FieldIssue::ModelMissing));
    }
    if input.endpoint_index >= state.catalog.endpoint_count() {
        return Err(field_error(FieldIssue::EndpointUnavailable));
    }

    let raw_key = input.key.trim().to_owned();
    let timeout_ms = SettingsConfig {
        schema_version: 1,
        model_index: input.model_index,
        endpoint_index: input.endpoint_index,
        timeout_ms: input.timeout_ms.unwrap_or_default(),
    }
    .normalized_timeout_ms();
    let config = SettingsConfig {
        schema_version: 1,
        model_index: input.model_index,
        endpoint_index: input.endpoint_index,
        timeout_ms,
    };
    let secret = SecretBytes::new(raw_key.as_bytes().to_vec());
    let persisted = state.secure.write(&secret).is_ok();
    if persisted {
        *state.memory_key.lock().map_err(|_| internal_error())? = None;
    } else {
        *state.memory_key.lock().map_err(|_| internal_error())? = Some(raw_key);
    }
    state
        .config
        .save(&config)
        .map_err(|_| IpcErrorDto::new("save_unavailable", "设置未保存，请稍后重试。", true))?;
    *state.form_state.lock().map_err(|_| internal_error())? =
        SettingsFormState::ConfiguredUnverified;

    if persisted {
        Ok(SettingsSaveReceiptDto {
            persisted: true,
            fallback: None,
            has_key: true,
            notice: None,
        })
    } else {
        Ok(SettingsSaveReceiptDto {
            persisted: false,
            fallback: Some("memory".to_owned()),
            has_key: true,
            notice: Some(PERSIST_FALLBACK_NOTICE.to_owned()),
        })
    }
}

/// 清除：未确认拒绝且无副作用；确认后删除目标名并重置配置。
pub fn settings_clear_impl(
    state: &AppState,
    input: ClearInputDto,
) -> Result<SettingsClearReceiptDto, IpcErrorDto> {
    if !input.confirm {
        return Err(IpcErrorDto::new(
            "confirmation_required",
            "请确认后再清除设置。",
            false,
        ));
    }
    let delete_ok = state.secure.delete().is_ok();
    let reset_ok = state.config.reset().is_ok();
    if !delete_ok || !reset_ok {
        return Err(IpcErrorDto::new(
            "clear_failed",
            "清除未完成，请稍后重试。",
            true,
        ));
    }
    *state.memory_key.lock().map_err(|_| internal_error())? = None;
    *state
        .last_probe_report
        .lock()
        .map_err(|_| internal_error())? = None;
    *state
        .cooldown_until_ms
        .lock()
        .map_err(|_| internal_error())? = None;
    *state.form_state.lock().map_err(|_| internal_error())? = SettingsFormState::NotConfigured;
    Ok(SettingsClearReceiptDto {
        cleared: true,
        has_key: false,
        notice: None,
    })
}

/// 装配层事实源：与命令共享内存 Key、冷却与在途标记。
pub struct AppFactsSource {
    pub secure: Arc<dyn SecureStore>,
    pub memory_key: Arc<Mutex<Option<String>>>,
    pub cooldown_until_ms: Arc<Mutex<Option<u64>>>,
    pub probe_in_flight: Arc<AtomicBool>,
    pub model_count: usize,
    pub endpoint_count: usize,
}

impl SettingsFactsSource for AppFactsSource {
    fn facts(&self) -> FactsSnapshot {
        let memory_has_key = self.memory_key.lock().is_ok_and(|key| key.is_some());
        let stored_has_key = self.secure.read().is_ok_and(|value| value.is_some());
        FactsSnapshot {
            has_stored_key: memory_has_key || stored_has_key,
            model_count: self.model_count,
            endpoint_count: self.endpoint_count,
            now_ms: now_ms(),
            cooldown_until_ms: self.cooldown_until_ms.lock().ok().and_then(|value| *value),
            probe_in_flight: self.probe_in_flight.load(Ordering::Acquire),
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

fn to_snapshot_dto(state: &AppState, view: &SettingsView) -> SettingsSnapshotDto {
    let probe_report = match view.form_state {
        SettingsFormState::Failed | SettingsFormState::Cooling { .. } => state
            .last_probe_report
            .lock()
            .ok()
            .and_then(|report| report.clone()),
        _ => None,
    };
    SettingsSnapshotDto {
        form_state: form_state_str(view.form_state).to_owned(),
        has_key: view.has_key,
        key_display: masked_key_str(view.key_display).to_owned(),
        model_choice: match &view.model_choice {
            ModelSelectionView::Preset { display_name } => ModelChoiceDto::Preset {
                display_name: display_name.clone(),
            },
            ModelSelectionView::Custom { name } => ModelChoiceDto::Custom { name: name.clone() },
        },
        endpoint_choice: match &view.endpoint_choice {
            EndpointSelectionView::Option { display_name } => EndpointChoiceDto::Option {
                display_name: display_name.clone(),
            },
            EndpointSelectionView::Unavailable => EndpointChoiceDto::Unavailable,
        },
        inline_errors: view
            .inline_errors
            .iter()
            .map(|issue| field_issue_str(*issue).to_owned())
            .collect(),
        can_test: view.can_test,
        can_save: view.can_save,
        sections_order: SECTION_ORDER
            .iter()
            .map(|name| (*name).to_owned())
            .collect(),
        advanced_collapsed: true,
        probe_report,
    }
}

fn form_state_str(state: SettingsFormState) -> &'static str {
    match state {
        SettingsFormState::NotConfigured => "not_configured",
        SettingsFormState::ConfiguredUnverified => "configured_unverified",
        SettingsFormState::Testing => "testing",
        SettingsFormState::Verified => "verified",
        SettingsFormState::Failed => "failed",
        SettingsFormState::Cooling { .. } => "cooling",
    }
}

fn masked_key_str(masked: MaskedKey) -> &'static str {
    match masked {
        MaskedKey::Masked => "masked",
        MaskedKey::None => "none",
    }
}

fn field_issue_str(issue: FieldIssue) -> &'static str {
    match issue {
        FieldIssue::ApiKeyMissing => "api_key_missing",
        FieldIssue::ApiKeyTooShort => "api_key_too_short",
        FieldIssue::ApiKeyHasWhitespace => "api_key_has_whitespace",
        FieldIssue::ModelMissing => "model_missing",
        FieldIssue::ModelTooLong => "model_too_long",
        FieldIssue::EndpointUnavailable => "endpoint_unavailable",
    }
}

fn field_error(issue: FieldIssue) -> IpcErrorDto {
    let message = match issue {
        FieldIssue::ApiKeyMissing => "请填写访问密钥。",
        FieldIssue::ApiKeyTooShort => "访问密钥过短，请检查。",
        FieldIssue::ApiKeyHasWhitespace => "访问密钥不能包含空白字符。",
        FieldIssue::ModelMissing => "请选择可用模型。",
        FieldIssue::ModelTooLong => "模型名称过长。",
        FieldIssue::EndpointUnavailable => "服务地址不可用。",
    };
    IpcErrorDto::new(field_issue_str(issue), message, false)
}

fn internal_error() -> IpcErrorDto {
    IpcErrorDto::new("internal", "内部状态不可用。", true)
}
#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicBool;
    use std::sync::{Arc, Mutex};

    use meeting_copilot_application::settings::{
        CardSnapshotEntry, CardSnapshotSource, FactsSnapshot, PortError, ProbeAttempt, ProbeCall,
        ProbeExecutor, SecureStore, SettingsConfigStore, SettingsFactsSource, ViewFactsSnapshot,
        ViewFactsSource,
    };
    use meeting_copilot_ui_bridge::settings::mask_api_key;

    use super::*;
    use crate::ipc::catalog::SettingsCatalog;
    use crate::ipc::dto::{
        CardBodyDto, CardDetailDto, ConclusionDto, EndpointChoiceDto, KeywordCardViewDto,
        ModelChoiceDto, ProbeReportDto,
    };
    use crate::ipc::state::CommandCounters;

    struct MemorySecure {
        value: Mutex<Option<Vec<u8>>>,
        deny_write: bool,
    }

    impl SecureStore for MemorySecure {
        fn write(&self, secret: &SecretBytes) -> Result<(), PortError> {
            if self.deny_write {
                return Err(PortError::Denied);
            }
            secret.with_bytes(|bytes| *self.value.lock().expect("lock") = Some(bytes.to_vec()));
            Ok(())
        }

        fn read(&self) -> Result<Option<SecretBytes>, PortError> {
            Ok(self
                .value
                .lock()
                .expect("lock")
                .clone()
                .map(SecretBytes::new))
        }

        fn delete(&self) -> Result<(), PortError> {
            *self.value.lock().expect("lock") = None;
            Ok(())
        }
    }

    struct MemoryConfig {
        config: Mutex<SettingsConfig>,
    }

    impl SettingsConfigStore for MemoryConfig {
        fn load(&self) -> Result<SettingsConfig, PortError> {
            Ok(*self.config.lock().expect("lock"))
        }

        fn save(&self, config: &SettingsConfig) -> Result<(), PortError> {
            *self.config.lock().expect("lock") = *config;
            Ok(())
        }

        fn reset(&self) -> Result<(), PortError> {
            *self.config.lock().expect("lock") = SettingsConfig::default();
            Ok(())
        }
    }

    struct FixedFacts {
        snapshot: FactsSnapshot,
    }

    impl SettingsFactsSource for FixedFacts {
        fn facts(&self) -> FactsSnapshot {
            self.snapshot
        }
    }

    struct EmptyCards;

    impl CardSnapshotSource for EmptyCards {
        fn cards(&self) -> Vec<CardSnapshotEntry> {
            Vec::new()
        }
    }

    struct FixedView;

    impl ViewFactsSource for FixedView {
        fn view_facts(&self) -> ViewFactsSnapshot {
            ViewFactsSnapshot {
                session_matches: true,
                authorization_revoked: false,
                reference_stale: false,
                screen_share: false,
                emergency_masked: false,
                visible_card_count: 0,
            }
        }
    }

    struct NoProbe;

    impl ProbeExecutor for NoProbe {
        fn execute(&self, _call: &ProbeCall) -> Result<ProbeAttempt, PortError> {
            Err(PortError::Unsupported)
        }
    }

    fn memory_secure(deny_write: bool) -> Arc<MemorySecure> {
        Arc::new(MemorySecure {
            value: Mutex::new(None),
            deny_write,
        })
    }

    fn memory_config() -> Arc<dyn SettingsConfigStore> {
        Arc::new(MemoryConfig {
            config: Mutex::new(SettingsConfig::default()),
        })
    }

    fn app_state(
        secure: Arc<dyn SecureStore>,
        config: Arc<dyn SettingsConfigStore>,
        has_key: bool,
    ) -> AppState {
        let presets = vec!["preset-a".to_owned()];
        AppState {
            secure,
            config,
            probe: Arc::new(NoProbe),
            facts: Arc::new(FixedFacts {
                snapshot: FactsSnapshot {
                    has_stored_key: has_key,
                    model_count: presets.len(),
                    endpoint_count: 1,
                    now_ms: 1_000,
                    cooldown_until_ms: None,
                    probe_in_flight: false,
                },
            }),
            cards: Arc::new(EmptyCards),
            view_facts: Arc::new(FixedView),
            catalog: SettingsCatalog::from_injected(presets),
            memory_key: Arc::new(Mutex::new(None)),
            form_state: Mutex::new(SettingsFormState::NotConfigured),
            probe_in_flight: Arc::new(AtomicBool::new(false)),
            cooldown_until_ms: Arc::new(Mutex::new(None)),
            last_probe_report: Mutex::new(None),
            counters: Mutex::new(CommandCounters::default()),
        }
    }

    fn save_input(key: &str) -> SaveInputDto {
        SaveInputDto {
            key: key.to_owned(),
            model_index: 0,
            endpoint_index: 0,
            timeout_ms: None,
        }
    }

    #[test]
    fn ipc_settings_snapshot_is_closed_and_keyless() {
        let state = app_state(memory_secure(false), memory_config(), false);
        let dto = settings_snapshot_impl(&state).expect("snapshot");
        let json = serde_json::to_value(&dto).expect("json");
        assert_eq!(json.as_object().expect("object").len(), 11);
        let text = json.to_string();
        for banned in [
            "apiKey",
            "keyLen",
            "keyPrefix",
            "keySuffix",
            "endpointRaw",
            "url",
            "host",
            "modelRaw",
            "requestBody",
            "requestId",
        ] {
            assert!(!text.contains(banned), "banned {banned}");
        }
        assert_eq!(dto.sections_order.len(), 4);
        assert!(dto.advanced_collapsed);
    }

    #[test]
    fn ipc_settings_snapshot_without_key_is_not_configured() {
        let state = app_state(memory_secure(false), memory_config(), false);
        let dto = settings_snapshot_impl(&state).expect("snapshot");
        assert!(!dto.has_key);
        assert_eq!(dto.key_display, "none");
        assert!(!dto.can_test);
        assert_eq!(dto.form_state, "not_configured");
    }

    #[test]
    fn save_receipt_reports_memory_fallback_without_plaintext() {
        let state = app_state(memory_secure(true), memory_config(), false);
        let receipt = settings_save_impl(&state, save_input("test-key-0123456789")).expect("save");
        assert!(!receipt.persisted);
        assert_eq!(receipt.fallback.as_deref(), Some("memory"));
        assert!(receipt.notice.is_some());
        let text = serde_json::to_string(&receipt).expect("json");
        assert!(!text.contains("test-key-0123456789"));
    }

    #[test]
    fn clear_requires_confirmation_and_deletes_target() {
        let secure = memory_secure(false);
        let state = app_state(
            Arc::clone(&secure) as Arc<dyn SecureStore>,
            memory_config(),
            false,
        );
        settings_save_impl(&state, save_input("test-key-0123456789")).expect("save");
        let rejected = settings_clear_impl(&state, ClearInputDto { confirm: false })
            .expect_err("confirm required");
        assert_eq!(rejected.code, "confirmation_required");
        assert!(secure.value.lock().expect("lock").is_some());

        let receipt = settings_clear_impl(&state, ClearInputDto { confirm: true }).expect("clear");
        assert!(receipt.cleared);
        assert!(!receipt.has_key);
        assert!(secure.value.lock().expect("lock").is_none());
    }

    #[test]
    fn snapshot_projects_injected_model_and_endpoint_labels() {
        let state = app_state(memory_secure(false), memory_config(), false);
        let dto = settings_snapshot_impl(&state).expect("snapshot");
        assert!(matches!(
            dto.model_choice,
            ModelChoiceDto::Preset { ref display_name } if display_name == "preset-a"
        ));
        assert!(matches!(
            dto.endpoint_choice,
            EndpointChoiceDto::Option { ref display_name } if !display_name.is_empty()
        ));
    }

    #[test]
    fn snapshot_json_uses_camel_case_variant_fields() {
        let state = app_state(memory_secure(false), memory_config(), false);
        let dto = settings_snapshot_impl(&state).expect("snapshot");
        let json = serde_json::to_value(&dto).expect("json");
        assert_eq!(json["modelChoice"]["preset"]["displayName"], "preset-a");
        assert!(json["endpointChoice"]["option"]["displayName"].is_string());
    }

    #[test]
    fn clear_is_idempotent() {
        let state = app_state(memory_secure(false), memory_config(), false);
        settings_clear_impl(&state, ClearInputDto { confirm: true }).expect("first");
        settings_clear_impl(&state, ClearInputDto { confirm: true }).expect("second");
    }

    struct FailingResetConfig;

    impl SettingsConfigStore for FailingResetConfig {
        fn load(&self) -> Result<SettingsConfig, PortError> {
            Ok(SettingsConfig::default())
        }

        fn save(&self, _config: &SettingsConfig) -> Result<(), PortError> {
            Err(PortError::Unavailable)
        }

        fn reset(&self) -> Result<(), PortError> {
            Err(PortError::Unavailable)
        }
    }

    #[test]
    fn clear_reports_failure_when_reset_fails() {
        let secure = memory_secure(false);
        let state = app_state(
            Arc::clone(&secure) as Arc<dyn SecureStore>,
            Arc::new(FailingResetConfig),
            true,
        );
        let error = settings_clear_impl(&state, ClearInputDto { confirm: true })
            .expect_err("reset failure must be reported");
        assert_eq!(error.code, "clear_failed");
        assert!(secure.value.lock().expect("lock").is_none());
    }

    #[test]
    fn ipc_key_is_absent_from_responses_and_logs() {
        let state = app_state(memory_secure(false), memory_config(), false);
        let snapshot = settings_snapshot_impl(&state).expect("snapshot");
        let report = ProbeReportDto {
            succeeded: false,
            failure: Some("timeout".to_owned()),
            message: "超时".to_owned(),
            elapsed_ms: 1,
            remaining_ms: None,
        };
        let save = SettingsSaveReceiptDto {
            persisted: false,
            fallback: Some("memory".to_owned()),
            has_key: true,
            notice: Some("notice".to_owned()),
        };
        let clear = SettingsClearReceiptDto {
            cleared: true,
            has_key: false,
            notice: None,
        };
        let card = KeywordCardViewDto {
            card_id: "card-x".to_owned(),
            keyword_kind: "metric".to_owned(),
            trigger: "auto".to_owned(),
            triggered_at_ms: 1,
            card_status: "displayed".to_owned(),
            body_visible: true,
            body: Some(CardBodyDto {
                title: "title".to_owned(),
                conclusion: ConclusionDto::Absent,
                confidence: 0.5,
                reference_ids: Vec::new(),
            }),
            click_state: "disabled".to_owned(),
            disabled_reason: Some("not_allowed".to_owned()),
            badge_count: 1,
        };
        let detail = CardDetailDto {
            card_id: "card-x".to_owned(),
            title: "title".to_owned(),
            conclusion: ConclusionDto::Absent,
            reference_ids: Vec::new(),
            citation_alignment: "unknown".to_owned(),
        };
        let payload =
            serde_json::to_string(&(snapshot, report, save, clear, card, detail)).expect("json");
        assert!(!payload.contains("test-key-"));

        let source = include_str!("settings.rs");
        for needle in [
            ["print", "ln!"].concat(),
            ["eprint", "ln!"].concat(),
            ["dbg", "!"].concat(),
        ] {
            assert!(!source.contains(&needle), "found {needle}");
        }
    }

    #[test]
    fn settings_api_key_is_masked_without_payload() {
        for len in [1usize, 16, 64, 256] {
            let raw = "k".repeat(len);
            assert_eq!(
                mask_api_key(&SecretInput::from_draft(&raw)),
                MaskedKey::Masked
            );
        }
        let state = app_state(memory_secure(false), memory_config(), true);
        settings_save_impl(&state, save_input("test-key-0123456789")).expect("save");
        let dto = settings_snapshot_impl(&state).expect("snapshot");
        assert_eq!(dto.key_display, "masked");
        assert!(!serde_json::to_string(&dto)
            .expect("json")
            .contains("test-key-0123456789"));
    }

    #[test]
    fn existing_closed_faces_are_unchanged() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
        let status = std::process::Command::new("git")
            .args([
                "diff",
                "--quiet",
                "HEAD",
                "--",
                "crates/ui-bridge",
                "crates/application/src/gate",
                "crates/application/src/cards",
                "crates/domain/src/model",
            ])
            .current_dir(root)
            .status()
            .expect("git");
        assert!(status.success());
    }

    #[test]
    fn platform_scope_is_windows_only() {
        let source = include_str!("settings.rs");
        let forbidden = ["target_os = \"ma", "cos\""].concat();
        assert!(!source.contains(&forbidden));
        assert_eq!(cfg!(windows), cfg!(target_os = "windows"));
    }
}
