//! C-01 码表：23 项 `GateRejectReason` 为唯一对外标识，12 项投影到既有 11 项 `DenyCode`。

use serde::{Deserialize, Serialize};

use crate::error::DenyCode;

/// 对外可见性。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    Internal,
}

/// 门控拒绝原因（唯一对外码，23 项，顺序即断言口径）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateRejectReason {
    PreFilterLowConfidence,
    ConsentMissing,
    AutoDisabled,
    Paused,
    SilentPeriod,
    ScreenShareProtection,
    SensitiveTerm,
    Duplicate,
    Suppressed,
    QuotaExhausted,
    NoProviderEnabled,
    LowRelevance,
    Stale,
    BudgetExhausted,
    GatewayCooling,
    Offline,
    OutboundNotGranted,
    SanitizeFailed,
    AuditUnavailable,
    ProviderNotAllowed,
    DomainBlocked,
    PolicyUnavailable,
    RetentionExpired,
}

impl GateRejectReason {
    /// 顺序与 design §2.2.3 表 #1..#23 完全一致。
    pub const ALL: [Self; 23] = [
        Self::PreFilterLowConfidence,
        Self::ConsentMissing,
        Self::AutoDisabled,
        Self::Paused,
        Self::SilentPeriod,
        Self::ScreenShareProtection,
        Self::SensitiveTerm,
        Self::Duplicate,
        Self::Suppressed,
        Self::QuotaExhausted,
        Self::NoProviderEnabled,
        Self::LowRelevance,
        Self::Stale,
        Self::BudgetExhausted,
        Self::GatewayCooling,
        Self::Offline,
        Self::OutboundNotGranted,
        Self::SanitizeFailed,
        Self::AuditUnavailable,
        Self::ProviderNotAllowed,
        Self::DomainBlocked,
        Self::PolicyUnavailable,
        Self::RetentionExpired,
    ];

    /// serde 值，同时用于 `ui_message_key` 与审计 `reason`。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PreFilterLowConfidence => "pre_filter_low_confidence",
            Self::ConsentMissing => "consent_missing",
            Self::AutoDisabled => "auto_disabled",
            Self::Paused => "paused",
            Self::SilentPeriod => "silent_period",
            Self::ScreenShareProtection => "screen_share_protection",
            Self::SensitiveTerm => "sensitive_term",
            Self::Duplicate => "duplicate",
            Self::Suppressed => "suppressed",
            Self::QuotaExhausted => "quota_exhausted",
            Self::NoProviderEnabled => "no_provider_enabled",
            Self::LowRelevance => "low_relevance",
            Self::Stale => "stale",
            Self::BudgetExhausted => "budget_exhausted",
            Self::GatewayCooling => "gateway_cooling",
            Self::Offline => "offline",
            Self::OutboundNotGranted => "outbound_not_granted",
            Self::SanitizeFailed => "sanitize_failed",
            Self::AuditUnavailable => "audit_unavailable",
            Self::ProviderNotAllowed => "provider_not_allowed",
            Self::DomainBlocked => "domain_blocked",
            Self::PolicyUnavailable => "policy_unavailable",
            Self::RetentionExpired => "retention_expired",
        }
    }

    /// 12 项投影到既有 `DenyCode`；11 项为纯内部码。
    pub const fn to_deny_code(self) -> Option<DenyCode> {
        match self {
            Self::ConsentMissing => Some(DenyCode::ConsentMissing),
            Self::SilentPeriod => Some(DenyCode::SilentPeriod),
            Self::ScreenShareProtection => Some(DenyCode::ScreenShareProtection),
            Self::SensitiveTerm => Some(DenyCode::SensitiveTermBlocked),
            Self::QuotaExhausted | Self::BudgetExhausted => Some(DenyCode::RateLimited),
            Self::OutboundNotGranted => Some(DenyCode::OutboundNotGranted),
            Self::SanitizeFailed => Some(DenyCode::SanitizeRequired),
            Self::ProviderNotAllowed => Some(DenyCode::ProviderNotAllowed),
            Self::DomainBlocked => Some(DenyCode::DomainBlocked),
            Self::PolicyUnavailable => Some(DenyCode::PolicyUnavailable),
            Self::RetentionExpired => Some(DenyCode::RetentionExpired),
            Self::PreFilterLowConfidence
            | Self::AutoDisabled
            | Self::Paused
            | Self::Duplicate
            | Self::Suppressed
            | Self::NoProviderEnabled
            | Self::LowRelevance
            | Self::Stale
            | Self::GatewayCooling
            | Self::Offline
            | Self::AuditUnavailable => None,
        }
    }

    /// UI 文案键统一为 `gate.reason.<serde 值>`。
    pub fn ui_message_key(self) -> String {
        format!("gate.reason.{}", self.as_str())
    }

    pub const fn visibility(self) -> Visibility {
        match self {
            Self::ConsentMissing
            | Self::SilentPeriod
            | Self::ScreenShareProtection
            | Self::SensitiveTerm
            | Self::QuotaExhausted
            | Self::BudgetExhausted
            | Self::OutboundNotGranted
            | Self::SanitizeFailed
            | Self::ProviderNotAllowed
            | Self::DomainBlocked
            | Self::PolicyUnavailable
            | Self::RetentionExpired => Visibility::Public,
            Self::PreFilterLowConfidence
            | Self::AutoDisabled
            | Self::Paused
            | Self::Duplicate
            | Self::Suppressed
            | Self::NoProviderEnabled
            | Self::LowRelevance
            | Self::Stale
            | Self::GatewayCooling
            | Self::Offline
            | Self::AuditUnavailable => Visibility::Internal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_reject_reason_all_is_23_in_table_order() {
        assert_eq!(GateRejectReason::ALL.len(), 23);
        assert_eq!(
            GateRejectReason::ALL[0],
            GateRejectReason::PreFilterLowConfidence
        );
        assert_eq!(
            GateRejectReason::ALL[22],
            GateRejectReason::RetentionExpired
        );
    }

    #[test]
    fn reject_reason_rejects_unknown_string() {
        assert!(serde_json::from_str::<GateRejectReason>("\"not_a_reason\"").is_err());
        assert_eq!(
            serde_json::from_str::<GateRejectReason>("\"consent_missing\"").expect("known"),
            GateRejectReason::ConsentMissing
        );
    }

    #[test]
    fn deny_code_mapping_has_exactly_12_some() {
        let mapped = GateRejectReason::ALL
            .into_iter()
            .filter(|reason| reason.to_deny_code().is_some())
            .count();
        assert_eq!(mapped, 12);
    }

    #[test]
    fn deny_code_mapping_covers_11_with_rate_limited_twice() {
        let mut covered = Vec::new();
        let mut rate_limited = 0;
        for reason in GateRejectReason::ALL {
            if let Some(code) = reason.to_deny_code() {
                if code == DenyCode::RateLimited {
                    rate_limited += 1;
                }
                if !covered.contains(&code) {
                    covered.push(code);
                }
            }
        }
        assert_eq!(covered.len(), 11);
        assert_eq!(rate_limited, 2);
    }

    #[test]
    fn ui_message_key_is_23_and_matches_serde_value() {
        let mut keys = Vec::new();
        for reason in GateRejectReason::ALL {
            let key = reason.ui_message_key();
            assert_eq!(key, format!("gate.reason.{}", reason.as_str()));
            assert!(!keys.contains(&key));
            keys.push(key);
        }
        assert_eq!(keys.len(), 23);
    }

    #[test]
    fn visibility_is_12_public_11_internal() {
        let public = GateRejectReason::ALL
            .into_iter()
            .filter(|reason| reason.visibility() == Visibility::Public)
            .count();
        let internal = GateRejectReason::ALL
            .into_iter()
            .filter(|reason| reason.visibility() == Visibility::Internal)
            .count();
        assert_eq!(public, 12);
        assert_eq!(internal, 11);
    }
}
