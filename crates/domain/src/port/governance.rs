use serde::{Deserialize, Serialize};

use crate::error::{CopilotError, DenyCode};
use crate::model::enums::{PresentationMode, TriggerKind};
use crate::model::governance::EffectivePolicy;
use crate::vo::{SanitizedQuery, WallClock};

/// 门控判定结果。`Deny` 必须携带可解释的码，便于 UI 与审计。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    Allow,
    Deny { code: DenyCode },
}

impl Decision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow)
    }

    pub fn deny(code: DenyCode) -> Self {
        Self::Deny { code }
    }
}

/// 门控输入。**纯数据**，`Guardrail` 不得在其中做 IO。
#[derive(Debug, Clone)]
pub struct GuardContext {
    pub policy: EffectivePolicy,
    pub trigger: TriggerKind,
    pub target_domain: Option<String>,
    pub query: Option<SanitizedQuery>,
    pub now: WallClock,
    pub silent_period: bool,
    pub screen_share_protection: bool,
    pub cards_in_window: u8,
    pub sensitive_term_hit: bool,
}

pub trait PolicyResolver: Send + Sync {
    fn resolve(&self) -> Result<EffectivePolicy, CopilotError>;
}

pub trait Guardrail: Send + Sync {
    fn evaluate(&self, context: &GuardContext) -> Decision;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensitiveKind {
    Person,
    Phone,
    Email,
    IdNumber,
    Url,
    InternalCode,
}

/// 敏感命中位置。只记录偏移，不携带原文。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensitiveHit {
    pub kind: SensitiveKind,
    pub start: usize,
    pub len: usize,
}

pub trait Sanitizer: Send + Sync {
    /// 返回（脱敏文本，替换次数）。实现方不得把原文写入日志。
    fn sanitize_query(&self, raw: &str) -> Result<(String, u16), CopilotError>;

    fn sanitize_output(&self, raw: &str) -> Result<String, CopilotError>;

    fn scan(&self, raw: &str) -> Vec<SensitiveHit>;

    /// **受控构造入口**：外部实现通过此方法产出不可伪造的 `SanitizedQuery`。
    fn issue_query(&self, raw: &str) -> Result<SanitizedQuery, CopilotError> {
        let (text, count) = self.sanitize_query(raw)?;
        SanitizedQuery::issue(&text, count).map_err(|error| CopilotError::PrivacyViolation {
            detail: error.to_string(),
        })
    }
}

pub trait AuditLog: Send + Sync {
    fn append(&self, action: crate::model::enums::AuditAction, summary: &str) -> Result<(), CopilotError>;

    /// 校验哈希链，返回首个不一致的审计 ID。
    fn verify(&self) -> Result<Option<crate::vo::AuditId>, CopilotError>;
}

pub trait Presenter: Send + Sync {
    fn push_card(&self, card: &crate::model::card::SuggestionCard) -> Result<(), CopilotError>;

    fn set_mode(&self, mode: PresentationMode) -> Result<(), CopilotError>;

    fn emergency_blackout(&self) -> Result<(), CopilotError>;

    fn set_screen_share_protection(&self, enabled: bool) -> Result<(), CopilotError>;

    fn notify(&self, message: &str) -> Result<(), CopilotError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::card::DedupeKey;
    use crate::model::enums::{ProviderKind, ProviderHealth, EnabledState};
    use crate::model::provider::{ProviderTimeout, RateLimit, Weight};
    use crate::vo::ProviderId;

    /// 只用于验证"受控构造无法绕过长度上限"。
    struct FailingSanitizer;

    impl Sanitizer for FailingSanitizer {
        fn sanitize_query(&self, raw: &str) -> Result<(String, u16), CopilotError> {
            Ok((raw.to_owned(), 0))
        }

        fn sanitize_output(&self, raw: &str) -> Result<String, CopilotError> {
            Ok(raw.to_owned())
        }

        fn scan(&self, _raw: &str) -> Vec<SensitiveHit> {
            Vec::new()
        }
    }

    #[test]
    fn sanitizer_cannot_issue_query_over_limit() {
        let sanitizer = FailingSanitizer;
        let too_long = "字".repeat(201);
        assert!(matches!(
            sanitizer.issue_query(&too_long),
            Err(CopilotError::PrivacyViolation { .. })
        ));
    }

    #[test]
    fn provider_profile_blocks_domains() {
        let profile = crate::model::provider::ProviderProfile {
            id: ProviderId::new("p1").expect("id"),
            kind: ProviderKind::Custom,
            priority: 1,
            weight: Weight::new(0.5).expect("weight"),
            enabled: EnabledState::Enabled,
            timeout: ProviderTimeout::from_millis(3_000).expect("timeout"),
            rate_limit: RateLimit {
                per_minute: 10,
                per_day: 1_000,
            },
            blocked_domains: vec![
                crate::model::provider::DomainPattern::new("*.internal.example.com").expect("pattern"),
            ],
            health: ProviderHealth::Healthy,
            endpoint_ref: None,
        };
        assert!(profile.blocks("wiki.internal.example.com"));
        assert!(!profile.blocks("example.org"));
        let _ = DedupeKey::new("a".repeat(32)).expect("key");
    }
}