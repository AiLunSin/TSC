//! 联调配置：凭据只经环境变量读取，Debug/Display 恒掩码。

use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

use meeting_copilot_domain::error::CopilotError;

use super::gate::EnvView;

static SECRET_CONSTRUCTIONS: AtomicUsize = AtomicUsize::new(0);
static SECRET_EXPOSURES: AtomicUsize = AtomicUsize::new(0);

/// 供离线探针读取"凭据是否进入内存"；不暴露凭据内容。
pub fn secret_read_counters() -> (usize, usize) {
    (
        SECRET_CONSTRUCTIONS.load(Ordering::SeqCst),
        SECRET_EXPOSURES.load(Ordering::SeqCst),
    )
}

/// 凭据包装：不实现 `Serialize`/`Deserialize`，避免进入产物与记录。
#[derive(Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Self {
        SECRET_CONSTRUCTIONS.fetch_add(1, Ordering::SeqCst);
        Self(value.into())
    }

    pub fn expose(&self) -> &str {
        SECRET_EXPOSURES.fetch_add(1, Ordering::SeqCst);
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct LiveRunConfig {
    pub endpoint: String,
    pub endpoint_host: String,
    pub model: String,
    pub samples_per_task: usize,
    pub deadline_ms: u64,
    pub concurrency: usize,
    pub call_budget: usize,
    pub path_confirmed: bool,
    pub secret: SecretString,
}

impl fmt::Debug for LiveRunConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LiveRunConfig")
            .field("endpoint_host", &self.endpoint_host)
            .field("model", &self.model)
            .field("samples_per_task", &self.samples_per_task)
            .field("deadline_ms", &self.deadline_ms)
            .field("concurrency", &self.concurrency)
            .field("call_budget", &self.call_budget)
            .field("path_confirmed", &self.path_confirmed)
            .field("secret", &"***")
            .finish()
    }
}

fn invalid(field: &str, rule: &str) -> CopilotError {
    CopilotError::ProviderConfigInvalid {
        detail: format!("{field}: {rule}"),
    }
}

pub fn host_from_url(url: &str) -> Option<String> {
    let rest = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))?;
    let authority = rest.split(['/', '?', '#']).next()?;
    let host = authority.split(':').next()?;
    (!host.is_empty()).then(|| host.to_owned())
}

/// 只取 path（含前导 `/`），并去掉 query/fragment；不拼接任何默认段。
pub fn path_from_url(url: &str) -> Option<String> {
    let rest = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))?;
    let (_, path) = rest.split_once('/')?;
    let path = path.split(['?', '#']).next().unwrap_or("");
    Some(format!("/{path}"))
}

fn parse_call_cap(env: &dyn EnvView) -> Option<u64> {
    let cap = env.get("LLM_LIVE_CALL_CAP")?.trim().parse::<u64>().ok()?;
    (cap > 0).then_some(cap)
}

impl LiveRunConfig {
    /// 参数顺序固定：截止时间、样本数、并发。
    pub fn try_new(
        env: &dyn EnvView,
        deadline_override: Option<u64>,
        samples_override: Option<usize>,
        concurrency_override: Option<usize>,
    ) -> Result<Self, CopilotError> {
        let endpoint = env
            .get("LLM_BASE_URL")
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| invalid("LLM_BASE_URL", "不能为空"))?;
        let endpoint_host = host_from_url(&endpoint)
            .ok_or_else(|| invalid("LLM_BASE_URL", "必须为 http(s) 地址"))?;
        let path = path_from_url(&endpoint)
            .filter(|path| !path.is_empty() && path != "/")
            .ok_or_else(|| invalid("LLM_BASE_URL", "endpoint_path_unconfirmed"))?;
        let path_confirmed = crate::llm::live::policy::path_confirmed(&path);
        let model = env
            .get("LLM_MODEL")
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| invalid("LLM_MODEL", "不能为空"))?;
        if model.chars().count() > 128 {
            return Err(invalid("LLM_MODEL", "不超过 128 字符"));
        }
        let secret = env
            .get("LLM_API_KEY")
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| invalid("LLM_API_KEY", "不能为空"))?;
        let samples = samples_override.unwrap_or(30);
        if !(1..=200).contains(&samples) {
            return Err(invalid("LLM_LIVE_SAMPLES", "必须落在 1..=200"));
        }
        let deadline_ms = deadline_override.unwrap_or(6_000);
        if !(100..=8_000).contains(&deadline_ms) {
            return Err(invalid("LLM_LIVE_DEADLINE_MS", "必须落在 100..=8000"));
        }
        let concurrency = concurrency_override.unwrap_or(1);
        if concurrency != 1 {
            return Err(invalid("LLM_LIVE_CONCURRENCY", "真实联调必须串行"));
        }
        let cap = parse_call_cap(env)
            .ok_or_else(|| invalid("LLM_LIVE_CALL_CAP", "budget_not_confirmed"))?;
        let call_budget = u64::try_from(samples)
            .unwrap_or(u64::MAX)
            .saturating_mul(3)
            .saturating_mul(4)
            .min(cap);
        let call_budget = usize::try_from(call_budget).unwrap_or(usize::MAX);

        Ok(Self {
            endpoint,
            endpoint_host,
            model,
            samples_per_task: samples,
            deadline_ms,
            concurrency,
            call_budget,
            path_confirmed,
            secret: SecretString::new(secret),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::live::gate::MapEnv;

    fn env() -> MapEnv {
        MapEnv::new()
            .with(
                "LLM_BASE_URL",
                "https://example.invalid:8443/v1/chat?token=x",
            )
            .with("LLM_MODEL", "test-model")
            .with("LLM_API_KEY", "sk-test-0000")
            .with("LLM_LIVE_CALL_CAP", "2400")
            .with("LLM_ALLOWLIST", "example.invalid")
    }

    #[test]
    fn url_collapses_to_host_only() {
        let config = LiveRunConfig::try_new(&env(), None, None, None).expect("配置");
        assert_eq!(config.endpoint_host, "example.invalid");
        assert!(!config.endpoint_host.contains('/'));
        assert!(!config.endpoint_host.contains('?'));
    }

    #[test]
    fn overrides_are_bounded() {
        assert!(LiveRunConfig::try_new(&env(), Some(99), None, None).is_err());
        assert!(LiveRunConfig::try_new(&env(), None, Some(0), None).is_err());
        assert!(LiveRunConfig::try_new(&env(), None, Some(201), None).is_err());
        assert!(LiveRunConfig::try_new(&env(), None, None, Some(4)).is_err());
        let config = LiveRunConfig::try_new(&env(), Some(8_000), Some(200), Some(1)).expect("边界");
        assert_eq!(config.call_budget, 2400);
        assert!(LiveRunConfig::try_new(&env(), None, None, Some(2)).is_err());
    }

    #[test]
    fn path_is_required_and_never_defaulted() {
        let base = env();
        for endpoint in [
            "https://example.invalid",
            "https://example.invalid/",
            "https://example.invalid?x=1",
        ] {
            let env = base.clone().with("LLM_BASE_URL", endpoint);
            let error = LiveRunConfig::try_new(&env, None, None, None).expect_err("必须拒绝");
            assert!(format!("{error}").contains("endpoint_path_unconfirmed"));
        }
        let env = base.with("LLM_BASE_URL", "https://example.invalid/v1/chat");
        assert!(LiveRunConfig::try_new(&env, None, None, None).is_ok());
    }

    #[test]
    fn call_cap_is_required_and_caps_budget() {
        let without = env().with("LLM_LIVE_CALL_CAP", "");
        let error = LiveRunConfig::try_new(&without, None, None, None).expect_err("必须拒绝");
        assert!(format!("{error}").contains("budget_not_confirmed"));

        let capped = env().with("LLM_LIVE_CALL_CAP", "200");
        let config = LiveRunConfig::try_new(&capped, None, None, None).expect("配置");
        assert_eq!(config.call_budget, 200);

        let high = env().with("LLM_LIVE_CALL_CAP", "1000");
        let config = LiveRunConfig::try_new(&high, None, None, None).expect("配置");
        assert_eq!(config.call_budget, 360);
    }

    #[test]
    fn secret_is_masked_everywhere() {
        let config = LiveRunConfig::try_new(&env(), None, None, None).expect("配置");
        assert_eq!(format!("{:?}", config.secret), "***");
        assert_eq!(format!("{}", config.secret), "***");
        let debug = format!("{config:?}");
        assert!(!debug.contains("sk-test-0000"));
    }

    #[test]
    fn missing_credentials_are_rejected() {
        let env = MapEnv::new()
            .with("LLM_BASE_URL", "http://127.0.0.1/v1")
            .with("LLM_MODEL", "test-model");
        assert!(LiveRunConfig::try_new(&env, None, None, None).is_err());
    }

    #[test]
    fn path_confirmed_true_for_constant() {
        let env = env().with("LLM_BASE_URL", "https://example.invalid/chat/completions");
        let config = LiveRunConfig::try_new(&env, None, None, None).expect("配置");
        assert!(config.path_confirmed);
    }

    #[test]
    fn path_present_but_not_constant_is_not_rejected() {
        let env = env().with(
            "LLM_BASE_URL",
            "https://example.invalid/v1/chat/completions",
        );
        let config = LiveRunConfig::try_new(&env, None, None, None).expect("配置");
        assert!(!config.path_confirmed);
    }
}
