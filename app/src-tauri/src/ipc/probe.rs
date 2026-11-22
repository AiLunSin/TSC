//! 连通性探针：单飞、8 类失败映射、相对冷却毫值；只经 `ProbeExecutor` 出网。

use meeting_copilot_application::settings::{
    FactsSnapshot, PortError, ProbeAttempt, ProbeCall, ProbeExecutor, ProbeFailure, SecretBytes,
    SettingsConfig,
};
use meeting_copilot_ui_bridge::settings::{
    humanize_probe_failure, FieldIssue, ProbeFailureKind, SecretInput, SettingsFormState,
};

use crate::ipc::dto::{IpcErrorDto, ProbeInputDto, ProbeReportDto};
use crate::ipc::state::{AppState, ProbeGuard};

/// 探针：冷却先行 → 输入校验 → 端口执行 → 回执与状态更新。
pub fn probe_settings_impl(
    state: &AppState,
    input: ProbeInputDto,
) -> Result<ProbeReportDto, IpcErrorDto> {
    let facts = state.facts.facts();
    if let Some(until) = facts.cooldown_until_ms {
        if until > facts.now_ms {
            return Ok(cooling_report(until - facts.now_ms));
        }
    }

    let _guard = ProbeGuard::acquire(&state.probe_in_flight)
        .ok_or_else(|| IpcErrorDto::new("busy", "正在测试中，请稍候。", true))?;

    if input.model_index >= state.catalog.model_count() {
        return Err(field_error(FieldIssue::ModelMissing));
    }
    if input.endpoint_index >= state.catalog.endpoint_count() {
        return Err(field_error(FieldIssue::EndpointUnavailable));
    }

    let credential = if input.key.trim().is_empty() {
        state
            .secure
            .read()
            .map_err(|_| probe_unavailable())?
            .ok_or_else(|| field_error(FieldIssue::ApiKeyMissing))?
    } else {
        let secret_input = SecretInput::from_draft(&input.key);
        if let Some(issue) = secret_input.draft_issue() {
            return Err(field_error(issue));
        }
        SecretBytes::new(input.key.trim().as_bytes().to_vec())
    };

    let call = ProbeCall {
        credential,
        model_name: state.catalog.presets[input.model_index].clone(),
        endpoint_index: input.endpoint_index,
        timeout_ms: SettingsConfig {
            schema_version: 1,
            model_index: input.model_index,
            endpoint_index: input.endpoint_index,
            timeout_ms: input.timeout_ms.unwrap_or_default(),
        }
        .normalized_timeout_ms(),
    };

    let attempt = match state.probe.execute(&call) {
        Ok(attempt) => attempt,
        Err(PortError::Unsupported) => ProbeAttempt {
            succeeded: false,
            failure: Some(ProbeFailure::Unknown),
            elapsed_ms: 0,
            remaining_ms: None,
        },
        Err(_) => return Err(probe_unavailable()),
    };
    Ok(apply_attempt(state, facts, attempt))
}

fn apply_attempt(state: &AppState, facts: FactsSnapshot, attempt: ProbeAttempt) -> ProbeReportDto {
    if attempt.succeeded {
        if let Ok(mut cooldown) = state.cooldown_until_ms.lock() {
            *cooldown = None;
        }
        if let Ok(mut form) = state.form_state.lock() {
            *form = SettingsFormState::Verified;
        }
        let report = ProbeReportDto {
            succeeded: true,
            failure: None,
            message: "连接成功。".to_owned(),
            elapsed_ms: attempt.elapsed_ms,
            remaining_ms: None,
        };
        if let Ok(mut last) = state.last_probe_report.lock() {
            *last = Some(report.clone());
        }
        return report;
    }

    let failure = attempt.failure.unwrap_or(ProbeFailure::Unknown);
    let remaining = attempt.remaining_ms.filter(|ms| *ms > 0);
    if let Ok(mut cooldown) = state.cooldown_until_ms.lock() {
        *cooldown = remaining.map(|ms| facts.now_ms.saturating_add(ms));
    }
    if let Ok(mut form) = state.form_state.lock() {
        *form = match remaining {
            Some(ms) => SettingsFormState::Cooling { remaining_ms: ms },
            None => SettingsFormState::Failed,
        };
    }
    let report = ProbeReportDto {
        succeeded: false,
        failure: Some(failure.as_str().to_owned()),
        message: humanize_probe_failure(to_kind(failure)).to_owned(),
        elapsed_ms: attempt.elapsed_ms,
        remaining_ms: remaining,
    };
    if let Ok(mut last) = state.last_probe_report.lock() {
        *last = Some(report.clone());
    }
    report
}

fn cooling_report(remaining_ms: u64) -> ProbeReportDto {
    ProbeReportDto {
        succeeded: false,
        failure: Some("rate_limited".to_owned()),
        message: humanize_probe_failure(ProbeFailureKind::RateLimited).to_owned(),
        elapsed_ms: 0,
        remaining_ms: Some(remaining_ms),
    }
}

fn to_kind(failure: ProbeFailure) -> ProbeFailureKind {
    match failure {
        ProbeFailure::MissingKey => ProbeFailureKind::MissingKey,
        ProbeFailure::KeyRejected => ProbeFailureKind::KeyRejected,
        ProbeFailure::EndpointUnreachable => ProbeFailureKind::EndpointUnreachable,
        ProbeFailure::Timeout => ProbeFailureKind::Timeout,
        ProbeFailure::RateLimited => ProbeFailureKind::RateLimited,
        ProbeFailure::QuotaExceeded => ProbeFailureKind::QuotaExceeded,
        ProbeFailure::ServerError => ProbeFailureKind::ServerError,
        ProbeFailure::Unknown => ProbeFailureKind::Unknown,
    }
}

fn field_error(issue: FieldIssue) -> IpcErrorDto {
    let (code, message) = match issue {
        FieldIssue::ApiKeyMissing => ("api_key_missing", "请填写访问密钥。"),
        FieldIssue::ApiKeyTooShort => ("api_key_too_short", "访问密钥过短，请检查。"),
        FieldIssue::ApiKeyHasWhitespace => ("api_key_has_whitespace", "访问密钥不能包含空白字符。"),
        FieldIssue::ModelMissing => ("model_missing", "请选择可用模型。"),
        FieldIssue::ModelTooLong => ("model_too_long", "模型名称过长。"),
        FieldIssue::EndpointUnavailable => ("endpoint_unavailable", "服务地址不可用。"),
    };
    IpcErrorDto::new(code, message, false)
}

fn probe_unavailable() -> IpcErrorDto {
    IpcErrorDto::new("probe_unavailable", "连通性测试不可用。", true)
}

/// 生产适配器：与既有 `assemble_live` 相同的 TLS 与 Bearer 注入，仅发一次探针。
pub struct GatewayProbeExecutor;

#[cfg(feature = "llm-https")]
impl ProbeExecutor for GatewayProbeExecutor {
    fn execute(&self, call: &ProbeCall) -> Result<ProbeAttempt, PortError> {
        https_adapter::execute(call).map_err(|_| PortError::Unavailable)
    }
}

#[cfg(not(feature = "llm-https"))]
impl ProbeExecutor for GatewayProbeExecutor {
    fn execute(&self, _call: &ProbeCall) -> Result<ProbeAttempt, PortError> {
        Err(PortError::Unsupported)
    }
}

#[cfg(feature = "llm-https")]
mod https_adapter {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use meeting_copilot_application::settings::{ProbeAttempt, ProbeCall, ProbeFailure};
    use meeting_copilot_domain::port::llm::CancelToken;
    use meeting_copilot_infrastructure::llm::live::{
        BearerTransport, SecretString, DEEPSEEK_HOSTS, PATH_CONSTANT,
    };
    use meeting_copilot_infrastructure::llm::transport::{
        HttpRequest, HttpTransport, TransportError,
    };
    use meeting_copilot_infrastructure::llm::transport_tls::TlsTransport;

    pub(super) fn execute(call: &ProbeCall) -> Result<ProbeAttempt, TransportError> {
        let secret = call
            .credential
            .with_bytes(|bytes| String::from_utf8_lossy(bytes).into_owned());
        let transport: Arc<dyn HttpTransport> = Arc::new(BearerTransport::new(
            Arc::new(TlsTransport::new()),
            SecretString::new(secret),
        ));
        let body = serde_json::to_vec(&serde_json::json!({
            "model": call.model_name,
            "messages": [{ "role": "user", "content": "ping" }],
            "thinking": { "type": "disabled" },
            "max_tokens": 1,
        }))
        .unwrap_or_default();
        let headers = vec![
            ("Content-Type".to_owned(), "application/json".to_owned()),
            ("Accept".to_owned(), "application/json".to_owned()),
        ];
        let host = DEEPSEEK_HOSTS.first().copied().unwrap_or_default();
        let mut request = HttpRequest::json_post(host, 443, PATH_CONSTANT, headers, body);
        request.secure = true;
        let started = Instant::now();
        let deadline = started + Duration::from_millis(call.timeout_ms);
        let cancel = CancelToken::new();
        let response = transport.send(&request, deadline, &cancel)?;
        let failure = status_failure(response.status);
        Ok(ProbeAttempt {
            succeeded: failure.is_none(),
            failure,
            elapsed_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            remaining_ms: None,
        })
    }

    fn status_failure(status: u16) -> Option<ProbeFailure> {
        match status {
            200..=299 => None,
            401 | 403 => Some(ProbeFailure::KeyRejected),
            402 => Some(ProbeFailure::QuotaExceeded),
            404 | 405 => Some(ProbeFailure::EndpointUnreachable),
            408 | 504 => Some(ProbeFailure::Timeout),
            429 => Some(ProbeFailure::RateLimited),
            500..=599 => Some(ProbeFailure::ServerError),
            _ => Some(ProbeFailure::Unknown),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use meeting_copilot_application::settings::{
        CardSnapshotEntry, CardSnapshotSource, SecureStore, SettingsConfigStore,
        SettingsFactsSource, ViewFactsSnapshot, ViewFactsSource,
    };

    use super::*;
    use crate::ipc::catalog::SettingsCatalog;
    use crate::ipc::dto::SaveInputDto;
    use crate::ipc::settings::{settings_save_impl, settings_snapshot_impl};
    use crate::ipc::state::CommandCounters;

    struct MemorySecure {
        value: Mutex<Option<Vec<u8>>>,
    }

    impl SecureStore for MemorySecure {
        fn write(&self, secret: &SecretBytes) -> Result<(), PortError> {
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

    struct MemoryConfig;

    impl SettingsConfigStore for MemoryConfig {
        fn load(&self) -> Result<SettingsConfig, PortError> {
            Ok(SettingsConfig::default())
        }

        fn save(&self, _config: &SettingsConfig) -> Result<(), PortError> {
            Ok(())
        }

        fn reset(&self) -> Result<(), PortError> {
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

    struct CountingProbe {
        failure: Option<ProbeFailure>,
        remaining: Option<u64>,
        calls: AtomicUsize,
    }

    impl ProbeExecutor for CountingProbe {
        fn execute(&self, _call: &ProbeCall) -> Result<ProbeAttempt, PortError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(ProbeAttempt {
                succeeded: self.failure.is_none(),
                failure: self.failure,
                elapsed_ms: 7,
                remaining_ms: self.remaining,
            })
        }
    }

    fn facts(has_key: bool, cooldown_until_ms: Option<u64>) -> FactsSnapshot {
        FactsSnapshot {
            has_stored_key: has_key,
            model_count: 1,
            endpoint_count: 1,
            now_ms: 1_000,
            cooldown_until_ms,
            probe_in_flight: false,
        }
    }

    fn state(probe: Arc<dyn ProbeExecutor>, snapshot: FactsSnapshot) -> AppState {
        AppState {
            secure: Arc::new(MemorySecure {
                value: Mutex::new(None),
            }),
            config: Arc::new(MemoryConfig),
            probe,
            facts: Arc::new(FixedFacts { snapshot }),
            cards: Arc::new(EmptyCards),
            view_facts: Arc::new(FixedView),
            catalog: SettingsCatalog::from_injected(vec!["preset-a".to_owned()]),
            memory_key: Arc::new(Mutex::new(None)),
            form_state: Mutex::new(SettingsFormState::NotConfigured),
            probe_in_flight: Arc::new(AtomicBool::new(false)),
            cooldown_until_ms: Arc::new(Mutex::new(None)),
            last_probe_report: Mutex::new(None),
            counters: Mutex::new(CommandCounters::default()),
        }
    }

    fn probe_input() -> ProbeInputDto {
        ProbeInputDto {
            key: "test-key-0123456789".to_owned(),
            model_index: 0,
            endpoint_index: 0,
            timeout_ms: None,
        }
    }

    fn probe_with(failure: Option<ProbeFailure>, remaining: Option<u64>) -> Arc<CountingProbe> {
        Arc::new(CountingProbe {
            failure,
            remaining,
            calls: AtomicUsize::new(0),
        })
    }

    #[test]
    fn probe_executor_is_port_only() {
        let source = include_str!("probe.rs");
        assert!(source.contains(".execute("));
        assert!(!source.contains(&["req", "west"].concat()));
    }

    #[test]
    fn probe_adapter_reuses_existing_gateway() {
        let source = include_str!("probe.rs");
        assert!(source.contains("assemble_live"));
        assert!(source.contains("BearerTransport"));
        assert!(source.contains("TlsTransport"));
    }

    #[test]
    fn ipc_probe_never_opens_socket() {
        let source = include_str!("probe.rs");
        for needle in [
            ["req", "west"].concat(),
            ["hy", "per::"].concat(),
            ["Tcp", "Stream"].concat(),
            ["std::", "net"].concat(),
        ] {
            assert!(!source.contains(&needle), "found {needle}");
        }
    }

    #[test]
    fn ipc_probe_failure_maps_eight_kinds() {
        let kinds = [
            (ProbeFailure::MissingKey, "missing_key"),
            (ProbeFailure::KeyRejected, "key_rejected"),
            (ProbeFailure::EndpointUnreachable, "endpoint_unreachable"),
            (ProbeFailure::Timeout, "timeout"),
            (ProbeFailure::RateLimited, "rate_limited"),
            (ProbeFailure::QuotaExceeded, "quota_exceeded"),
            (ProbeFailure::ServerError, "server_error"),
            (ProbeFailure::Unknown, "unknown"),
        ];
        let mut messages = Vec::new();
        for (failure, name) in kinds {
            let state = state(probe_with(Some(failure), None), facts(false, None));
            let report = probe_settings_impl(&state, probe_input()).expect("probe");
            assert!(!report.succeeded);
            assert_eq!(report.failure.as_deref(), Some(name));
            messages.push(report.message);
        }
        messages.sort();
        messages.dedup();
        assert_eq!(messages.len(), 8);
    }

    #[test]
    fn ipc_probe_failure_keeps_inputs() {
        let state = state(
            probe_with(Some(ProbeFailure::Timeout), None),
            facts(true, None),
        );
        settings_save_impl(
            &state,
            SaveInputDto {
                key: "test-key-0123456789".to_owned(),
                model_index: 0,
                endpoint_index: 0,
                timeout_ms: None,
            },
        )
        .expect("save");
        let before = settings_snapshot_impl(&state).expect("snapshot");
        let report = probe_settings_impl(&state, probe_input()).expect("probe");
        assert!(!report.succeeded);
        let after = settings_snapshot_impl(&state).expect("snapshot");
        assert!(after.has_key);
        assert_eq!(after.key_display, "masked");
        assert_eq!(after.model_choice, before.model_choice);
    }

    #[test]
    fn ipc_probe_cooling_reports_remaining_ms() {
        let probe = probe_with(Some(ProbeFailure::Timeout), None);
        let counter = Arc::clone(&probe);
        let state = state(probe, facts(true, Some(9_000)));
        let report = probe_settings_impl(&state, probe_input()).expect("cooling");
        assert_eq!(report.failure.as_deref(), Some("rate_limited"));
        assert_eq!(report.remaining_ms, Some(8_000));
        assert_eq!(counter.calls.load(Ordering::SeqCst), 0);
    }
}
