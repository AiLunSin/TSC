//! 真实联调运行门（SPEC 11.4、DESIGN 2.7.2）。
//!
//! 纯函数判定；`LLM_LIVE` 严格等于 `"1"` 才可能放行。

use std::collections::BTreeMap;

use meeting_copilot_domain::error::CopilotError;

use super::config::path_from_url;
use super::policy::{
    host_is_locked, locked_host, model_confirmed, path_confirmed, scheme_verdict, HostVerdict,
    Scheme, SchemeVerdict, DEEPSEEK_HOSTS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    LiveDisabled,
    CredentialMissing,
    EndpointMissing,
    ModelMissing,
    AllowlistMissing,
    TransportUnavailable,
}

impl SkipReason {
    pub const fn code(self) -> &'static str {
        match self {
            Self::LiveDisabled => "live_disabled",
            Self::CredentialMissing => "credential_missing",
            Self::EndpointMissing => "endpoint_missing",
            Self::ModelMissing => "model_missing",
            Self::AllowlistMissing => "allowlist_missing",
            Self::TransportUnavailable => "transport_unavailable",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::LiveDisabled => "未实测（未开启真实联调）",
            Self::CredentialMissing => "未实测（凭据缺失）",
            Self::EndpointMissing => "未实测（端点未配置）",
            Self::ModelMissing => "未实测（模型未配置）",
            Self::AllowlistMissing => "未实测（外发白名单缺失）",
            Self::TransportUnavailable => "未实测（传输未启用）",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveDecision {
    Run,
    Skip(SkipReason),
}

/// 环境变量视图。测试用假实现，禁止 `set_var`。
pub trait EnvView {
    fn get(&self, key: &str) -> Option<String>;
}

#[derive(Debug, Clone, Default)]
pub struct MapEnv {
    values: BTreeMap<String, String>,
}

impl MapEnv {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, key: &str, value: &str) -> Self {
        self.values.insert(key.to_owned(), value.to_owned());
        self
    }
}

impl EnvView for MapEnv {
    fn get(&self, key: &str) -> Option<String> {
        self.values.get(key).cloned()
    }
}

/// 生产实现：启动时读取一次快照，之后与进程环境解耦。
#[derive(Debug, Clone, Default)]
pub struct ProcessEnv {
    values: BTreeMap<String, String>,
}

impl ProcessEnv {
    pub fn snapshot() -> Self {
        let mut values = BTreeMap::new();
        let keys = [
            "LLM_LIVE",
            "LLM_API_KEY",
            "LLM_BASE_URL",
            "LLM_MODEL",
            "LLM_MODEL_CONFIRMED",
            "LLM_LIVE_CALL_CAP",
            "LLM_LIVE_SAMPLES",
            "LLM_ALLOWLIST",
        ];
        for key in keys {
            if let Ok(value) = std::env::var(key) {
                values.insert(key.to_owned(), value);
            }
        }
        Self { values }
    }
}

impl EnvView for ProcessEnv {
    fn get(&self, key: &str) -> Option<String> {
        self.values.get(key).cloned()
    }
}

fn non_empty(env: &dyn EnvView, key: &str) -> bool {
    env.get(key).is_some_and(|value| !value.trim().is_empty())
}

/// 判定顺序固定：开关 → 凭据 → 端点 → 模型 → 白名单 → 传输。
pub fn evaluate(env: &dyn EnvView, transport_available: bool) -> LiveDecision {
    if env.get("LLM_LIVE").as_deref() != Some("1") {
        return LiveDecision::Skip(SkipReason::LiveDisabled);
    }
    if !non_empty(env, "LLM_API_KEY") {
        return LiveDecision::Skip(SkipReason::CredentialMissing);
    }
    if !non_empty(env, "LLM_BASE_URL") {
        return LiveDecision::Skip(SkipReason::EndpointMissing);
    }
    if !non_empty(env, "LLM_MODEL") {
        return LiveDecision::Skip(SkipReason::ModelMissing);
    }
    if !non_empty(env, "LLM_ALLOWLIST") {
        return LiveDecision::Skip(SkipReason::AllowlistMissing);
    }
    if !transport_available {
        return LiveDecision::Skip(SkipReason::TransportUnavailable);
    }
    LiveDecision::Run
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateReject {
    EndpointNotAllowed,
    ModelNotConfirmed,
    InsecureTransportRejected,
    BudgetNotConfirmed,
    BudgetExhausted,
}

impl GateReject {
    pub const fn code(self) -> &'static str {
        match self {
            Self::EndpointNotAllowed => "endpoint_not_allowed",
            Self::ModelNotConfirmed => "model_not_confirmed",
            Self::InsecureTransportRejected => "insecure_transport_rejected",
            Self::BudgetNotConfirmed => "budget_not_confirmed",
            Self::BudgetExhausted => "budget_exhausted",
        }
    }

    /// 只含字段名与标识，不含任何配置值。
    pub fn to_error(self) -> CopilotError {
        let field = match self {
            Self::EndpointNotAllowed => "LLM_BASE_URL",
            Self::ModelNotConfirmed => "LLM_MODEL",
            Self::InsecureTransportRejected => "LLM_BASE_URL",
            Self::BudgetNotConfirmed | Self::BudgetExhausted => "LLM_LIVE_CALL_CAP",
        };
        CopilotError::ProviderConfigInvalid {
            detail: format!("{field}: {}", self.code()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionTicket {
    pub endpoint_host: String,
    pub scheme: Scheme,
    pub call_limit: u64,
    pub path_confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Admission {
    Admit(AdmissionTicket),
    Skip(SkipReason),
    Reject(GateReject),
}

fn call_limit(env: &dyn EnvView) -> Option<u64> {
    let cap = env.get("LLM_LIVE_CALL_CAP")?.parse::<u64>().ok()?;
    if cap == 0 {
        return None;
    }
    let samples = env
        .get("LLM_LIVE_SAMPLES")
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| (1..=200).contains(value))
        .unwrap_or(30);
    Some(samples.saturating_mul(3).saturating_mul(4).min(cap))
}

/// 固定 11 步：开关 → 端点 → host → scheme → 凭据 → 模型 → 模型确认
/// → 白名单 → 传输 → 预算 → 放行。host/scheme 均前置于凭据读取。
pub fn admit(env: &dyn EnvView, transport_available: bool) -> Admission {
    if env.get("LLM_LIVE").as_deref() != Some("1") {
        return Admission::Skip(SkipReason::LiveDisabled);
    }
    let Some(base_url) = env
        .get("LLM_BASE_URL")
        .filter(|value| !value.trim().is_empty())
    else {
        return Admission::Skip(SkipReason::EndpointMissing);
    };
    if host_is_locked(&base_url, DEEPSEEK_HOSTS) != HostVerdict::Allowed {
        return Admission::Reject(GateReject::EndpointNotAllowed);
    }
    match scheme_verdict(&base_url) {
        SchemeVerdict::Https => {}
        SchemeVerdict::InsecureRejected => {
            return Admission::Reject(GateReject::InsecureTransportRejected)
        }
        SchemeVerdict::Unparsable => return Admission::Reject(GateReject::EndpointNotAllowed),
    }
    if !non_empty(env, "LLM_API_KEY") {
        return Admission::Skip(SkipReason::CredentialMissing);
    }
    let model = env.get("LLM_MODEL").unwrap_or_default();
    if model.trim().is_empty() {
        return Admission::Reject(GateReject::ModelNotConfirmed);
    }
    if !model_confirmed(&model, env.get("LLM_MODEL_CONFIRMED").as_deref()) {
        return Admission::Reject(GateReject::ModelNotConfirmed);
    }
    if !non_empty(env, "LLM_ALLOWLIST") {
        return Admission::Skip(SkipReason::AllowlistMissing);
    }
    if !transport_available {
        return Admission::Skip(SkipReason::TransportUnavailable);
    }
    let Some(limit) = call_limit(env) else {
        return Admission::Reject(GateReject::BudgetNotConfirmed);
    };
    let Some(endpoint_host) = locked_host(&base_url) else {
        return Admission::Reject(GateReject::EndpointNotAllowed);
    };
    let path = path_from_url(&base_url).unwrap_or_default();
    Admission::Admit(AdmissionTicket {
        endpoint_host,
        scheme: Scheme::Https,
        call_limit: limit,
        path_confirmed: path_confirmed(&path),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full() -> MapEnv {
        MapEnv::new()
            .with("LLM_LIVE", "1")
            .with("LLM_API_KEY", "sk-test-0000")
            .with("LLM_BASE_URL", "http://127.0.0.1:8080/v1/chat")
            .with("LLM_MODEL", "test-model")
            .with("LLM_ALLOWLIST", "127.0.0.1")
    }

    #[test]
    fn switch_must_be_strictly_one() {
        for value in ["", "0", "true", "yes", " 1", "1 ", "01"] {
            let env = full().with("LLM_LIVE", value);
            assert_eq!(
                evaluate(&env, true),
                LiveDecision::Skip(SkipReason::LiveDisabled),
                "{value:?}"
            );
        }
        assert_eq!(evaluate(&full(), true), LiveDecision::Run);
    }

    #[test]
    fn missing_inputs_have_deterministic_precedence() {
        let env = MapEnv::new();
        assert_eq!(
            evaluate(&env, true),
            LiveDecision::Skip(SkipReason::LiveDisabled)
        );
        let env = MapEnv::new().with("LLM_LIVE", "1");
        assert_eq!(
            evaluate(&env, true),
            LiveDecision::Skip(SkipReason::CredentialMissing)
        );
        let env = env.with("LLM_API_KEY", "sk-test-0000");
        assert_eq!(
            evaluate(&env, true),
            LiveDecision::Skip(SkipReason::EndpointMissing)
        );
        let env = env.with("LLM_BASE_URL", "http://127.0.0.1/v1");
        assert_eq!(
            evaluate(&env, true),
            LiveDecision::Skip(SkipReason::ModelMissing)
        );
        let env = env.with("LLM_MODEL", "test-model");
        assert_eq!(
            evaluate(&env, true),
            LiveDecision::Skip(SkipReason::AllowlistMissing)
        );
        let env = env.with("LLM_ALLOWLIST", "127.0.0.1");
        assert_eq!(
            evaluate(&env, false),
            LiveDecision::Skip(SkipReason::TransportUnavailable)
        );
    }

    #[test]
    fn skip_labels_are_explicit() {
        for reason in [
            SkipReason::LiveDisabled,
            SkipReason::CredentialMissing,
            SkipReason::EndpointMissing,
            SkipReason::ModelMissing,
            SkipReason::AllowlistMissing,
            SkipReason::TransportUnavailable,
        ] {
            assert!(reason.label().contains("未实测"));
            assert!(!reason.code().is_empty());
        }
    }

    fn locked() -> MapEnv {
        MapEnv::new()
            .with("LLM_LIVE", "1")
            .with(
                "LLM_BASE_URL",
                "https://api.deepseek.com/v1/chat/completions",
            )
            .with("LLM_API_KEY", "sk-test-0000")
            .with("LLM_MODEL", "test-model")
            .with("LLM_MODEL_CONFIRMED", "test-model")
            .with("LLM_LIVE_CALL_CAP", "360")
            .with("LLM_ALLOWLIST", "api.deepseek.com")
    }

    #[test]
    fn admit_accepts_only_locked_https() {
        match admit(&locked(), true) {
            Admission::Admit(ticket) => {
                assert_eq!(ticket.endpoint_host, "api.deepseek.com");
                assert_eq!(ticket.scheme, Scheme::Https);
                assert_eq!(ticket.call_limit, 360);
            }
            other => panic!("应放行：{other:?}"),
        }
    }

    #[test]
    fn admit_order_is_deterministic_inline() {
        let env = locked();
        let first = admit(&env, true);
        let second = admit(&env, true);
        let third = admit(&env, true);
        assert_eq!(first, second);
        assert_eq!(second, third);
        assert!(matches!(first, Admission::Admit(_)));
    }

    #[test]
    fn host_reject_precedes_credential() {
        let env = MapEnv::new()
            .with("LLM_LIVE", "1")
            .with("LLM_BASE_URL", "https://evil.api.deepseek.com/v1");
        assert_eq!(
            admit(&env, true),
            Admission::Reject(GateReject::EndpointNotAllowed)
        );
    }

    #[test]
    fn scheme_reject_precedes_credential() {
        let env = MapEnv::new()
            .with("LLM_LIVE", "1")
            .with("LLM_BASE_URL", "http://api.deepseek.com/v1");
        assert_eq!(
            admit(&env, true),
            Admission::Reject(GateReject::InsecureTransportRejected)
        );
    }

    #[test]
    fn reject_has_no_ticket() {
        let env = locked().with("LLM_MODEL", "other-model");
        match admit(&env, true) {
            Admission::Reject(GateReject::ModelNotConfirmed) => {}
            other => panic!("应拒绝：{other:?}"),
        }
        let error = format!("{}", GateReject::ModelNotConfirmed.to_error());
        assert!(!error.contains("test-model"));
    }

    #[test]
    fn budget_cap_is_min_of_theoretical_and_confirmed() {
        let ticket = |cap: &str| match admit(&locked().with("LLM_LIVE_CALL_CAP", cap), true) {
            Admission::Admit(ticket) => ticket.call_limit,
            other => panic!("应放行：{other:?}"),
        };
        assert_eq!(ticket("1000"), 360);
        assert_eq!(ticket("200"), 200);
        assert_eq!(
            admit(&locked().with("LLM_LIVE_CALL_CAP", "0"), true),
            Admission::Reject(GateReject::BudgetNotConfirmed)
        );
    }
}
