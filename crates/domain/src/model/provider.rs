use serde::{Deserialize, Serialize};

use crate::model::enums::{EnabledState, ProviderHealth, ProviderKind};
use crate::vo::ProviderId;

/// 检索源权重，`(0, 1]`。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct Weight(f64);

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("权重必须落在 (0, 1]，实际为 {0}")]
pub struct WeightError(f64);

impl Weight {
    pub fn new(value: f64) -> Result<Self, WeightError> {
        if value.is_finite() && value > 0.0 && value <= 1.0 {
            Ok(Self(value))
        } else {
            Err(WeightError(value))
        }
    }

    pub fn value(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for Weight {
    type Error = WeightError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Weight> for f64 {
    fn from(value: Weight) -> Self {
        value.0
    }
}

/// 单次检索超时上限 8 秒（SPEC 5.3.1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct ProviderTimeout(u64);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("检索超时必须落在 1–8000 毫秒，实际为 {0}")]
pub struct TimeoutError(u64);

impl ProviderTimeout {
    pub const MAX_MS: u64 = 8_000;

    pub fn from_millis(millis: u64) -> Result<Self, TimeoutError> {
        if millis == 0 || millis > Self::MAX_MS {
            return Err(TimeoutError(millis));
        }
        Ok(Self(millis))
    }

    pub const fn as_millis(self) -> u64 {
        self.0
    }
}

impl TryFrom<u64> for ProviderTimeout {
    type Error = TimeoutError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::from_millis(value)
    }
}

impl From<ProviderTimeout> for u64 {
    fn from(value: ProviderTimeout) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimit {
    pub per_minute: u16,
    pub per_day: u32,
}

/// 域名匹配模式，例如 `*.internal.example.com`。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DomainPattern(String);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("域名模式非法：{0}")]
pub struct DomainPatternError(String);

impl DomainPattern {
    pub fn new(raw: impl Into<String>) -> Result<Self, DomainPatternError> {
        let value = raw.into().trim().to_ascii_lowercase();
        if value.is_empty()
            || value.contains(char::is_whitespace)
            || !value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '*' | '_'))
        {
            return Err(DomainPatternError(value));
        }
        Ok(Self(value))
    }

    pub fn matches(&self, domain: &str) -> bool {
        let domain = domain.to_ascii_lowercase();
        match self.0.strip_prefix("*.") {
            Some(suffix) => domain == suffix || domain.ends_with(&format!(".{suffix}")),
            None => domain == self.0 || domain.ends_with(&format!(".{}", self.0)),
        }
    }
}

impl TryFrom<String> for DomainPattern {
    type Error = DomainPatternError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<DomainPattern> for String {
    fn from(value: DomainPattern) -> Self {
        value.0
    }
}

/// 检索源配置。**不含任何具体服务商常量**，由用户配置驱动。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderProfile {
    pub id: ProviderId,
    pub kind: ProviderKind,
    pub priority: i32,
    pub weight: Weight,
    pub enabled: EnabledState,
    pub timeout: ProviderTimeout,
    pub rate_limit: RateLimit,
    pub blocked_domains: Vec<DomainPattern>,
    pub health: ProviderHealth,
    /// 连接地址等非敏感配置；凭据以引用形式保存，禁止明文。
    pub endpoint_ref: Option<String>,
}

impl ProviderProfile {
    pub fn is_usable(&self) -> bool {
        self.enabled == EnabledState::Enabled && self.health != ProviderHealth::Unavailable
    }

    pub fn blocks(&self, domain: &str) -> bool {
        self.blocked_domains.iter().any(|p| p.matches(domain))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weight_bounds() {
        assert!(Weight::new(0.0).is_err());
        assert!(Weight::new(1.0).is_ok());
        assert!(Weight::new(1.01).is_err());
    }

    #[test]
    fn timeout_capped_at_eight_seconds() {
        assert!(ProviderTimeout::from_millis(0).is_err());
        assert!(ProviderTimeout::from_millis(8_000).is_ok());
        assert!(ProviderTimeout::from_millis(8_001).is_err());
    }

    #[test]
    fn domain_wildcards() {
        let pattern = DomainPattern::new("*.internal.example.com").expect("pattern");
        assert!(pattern.matches("wiki.internal.example.com"));
        assert!(pattern.matches("internal.example.com"));
        assert!(!pattern.matches("example.com"));

        let exact = DomainPattern::new("example.com").expect("pattern");
        assert!(exact.matches("docs.example.com"));
        assert!(!exact.matches("notexample.com"));
    }
}
