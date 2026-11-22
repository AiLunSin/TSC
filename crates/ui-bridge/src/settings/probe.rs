//! 探针状态机与失败人话文案：无网络、无计时器、无系统时间。

use super::input::{ProbeRequest, SettingsInput, SettingsIntent};
use super::view::{SettingsDefaults, SettingsFacts, SettingsFormState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeFailureKind {
    MissingKey,
    KeyRejected,
    EndpointUnreachable,
    Timeout,
    RateLimited,
    QuotaExceeded,
    ServerError,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeEvent {
    UserTest,
    UserCancel,
    ProbeSucceeded,
    ProbeFailed { failure: ProbeFailureKind },
    ClockTick { now_ms: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeOutcome {
    Succeeded { elapsed_ms: u64 },
    Failed { failure: ProbeFailureKind },
}

impl ProbeOutcome {
    pub fn to_event(self) -> ProbeEvent {
        match self {
            Self::Succeeded { .. } => ProbeEvent::ProbeSucceeded,
            Self::Failed { failure } => ProbeEvent::ProbeFailed { failure },
        }
    }
}

pub fn humanize_probe_failure(failure: ProbeFailureKind) -> &'static str {
    match failure {
        ProbeFailureKind::MissingKey => "请先填写访问密钥。",
        ProbeFailureKind::KeyRejected => "密钥无效，请检查后重试。",
        ProbeFailureKind::EndpointUnreachable => "服务地址不可达，请检查网络。",
        ProbeFailureKind::Timeout => "连接超时，请稍后重试。",
        ProbeFailureKind::RateLimited => "请求过于频繁，请稍后再试。",
        ProbeFailureKind::QuotaExceeded => "额度不足，请检查账户。",
        ProbeFailureKind::ServerError => "服务暂时异常，请稍后重试。",
        ProbeFailureKind::Unknown => "未知错误，请重试。",
    }
}

pub fn next_probe_state(
    facts: &SettingsFacts,
    input: &SettingsInput,
    state: SettingsFormState,
    event: ProbeEvent,
    _presets: &[String],
    _endpoints: &[String],
    _defaults: &SettingsDefaults,
) -> (SettingsFormState, Option<SettingsIntent>) {
    match event {
        ProbeEvent::UserTest => {
            if !input.api_key_draft.is_provided() {
                return (SettingsFormState::NotConfigured, None);
            }
            let request = ProbeRequest {
                credential: input.api_key_draft.clone(),
                model: input.model_draft.clone(),
                endpoint_index: match input.endpoint_draft {
                    super::input::EndpointDraft::Index(index) => Some(index),
                    super::input::EndpointDraft::None => None,
                },
                timeout_ms: _defaults.timeout_ms,
            };
            (
                SettingsFormState::Testing,
                Some(SettingsIntent::Probe { request }),
            )
        }
        ProbeEvent::UserCancel => (SettingsFormState::ConfiguredUnverified, None),
        ProbeEvent::ProbeSucceeded => (SettingsFormState::Verified, None),
        ProbeEvent::ProbeFailed { .. } => (SettingsFormState::Failed, None),
        ProbeEvent::ClockTick { now_ms } => {
            if let Some(until) = facts.cooldown_until_ms {
                if until > now_ms {
                    return (
                        SettingsFormState::Cooling {
                            remaining_ms: until - now_ms,
                        },
                        None,
                    );
                }
            }
            (state, None)
        }
    }
}
