use serde::{Deserialize, Serialize};

use crate::model::enums::{
    AuditAction, AutoSuggestionSwitch, ConsentMode, EnabledState, PolicyScope, TermKind,
};
use crate::vo::{ActorRef, AuditId, ProviderId, ResourceRef, WallClock};

/// 术语表条目。敏感术语一律禁止外发（SPEC 6.6-4）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LexiconTerm {
    pub name: String,
    pub aliases: Vec<String>,
    pub kind: TermKind,
    pub sensitive: bool,
    pub enabled: EnabledState,
}

impl LexiconTerm {
    pub fn is_outbound_allowed(&self) -> bool {
        !self.sensitive && self.enabled == EnabledState::Enabled
    }
}

/// 自动外发的一次性授权（SPEC 4.3-5）。会话结束即失效。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundGrant {
    pub granted_at: WallClock,
    pub notice_version: u16,
    pub provider_scope: Vec<ProviderId>,
}

/// 生效策略。组织策略优先于个人设置，且不得被个人绕过。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectivePolicy {
    pub scope: PolicyScope,
    pub consent: ConsentMode,
    pub mandatory_sanitize: bool,
    pub retention_days: u16,
    pub auto_suggestion: AutoSuggestionSwitch,
    pub outbound_grant: Option<OutboundGrant>,
    pub audit_retention_days: u16,
    pub screen_share_protection_default: bool,
}

impl EffectivePolicy {
    pub const MIN_AUDIT_RETENTION_DAYS: u16 = 180;

    /// 组织策略缺失时按最严格默认值执行（fail-closed）。
    pub fn fail_closed() -> Self {
        Self {
            scope: PolicyScope::Organization,
            consent: ConsentMode::Explicit,
            mandatory_sanitize: true,
            retention_days: 1,
            auto_suggestion: AutoSuggestionSwitch::DisabledByPolicy,
            outbound_grant: None,
            audit_retention_days: Self::MIN_AUDIT_RETENTION_DAYS,
            screen_share_protection_default: true,
        }
    }

    pub fn auto_outbound_allowed(&self) -> bool {
        self.auto_suggestion == AutoSuggestionSwitch::Enabled
            && self.outbound_grant.is_some()
            && self.mandatory_sanitize
    }
}

/// 只追加审计记录。`self_hash = H(prev_hash ‖ canonical(fields))`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditRecord {
    pub id: AuditId,
    pub action: AuditAction,
    pub actor: ActorRef,
    pub occurred_at: WallClock,
    pub target: ResourceRef,
    pub summary: String,
    pub prev_hash: String,
    pub self_hash: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_term_blocks_outbound() {
        let term = LexiconTerm {
            name: "内部代号 X".into(),
            aliases: vec![],
            kind: TermKind::Product,
            sensitive: true,
            enabled: EnabledState::Enabled,
        };
        assert!(!term.is_outbound_allowed());
    }

    #[test]
    fn fail_closed_policy_disables_auto_suggestion() {
        let policy = EffectivePolicy::fail_closed();
        assert_eq!(policy.auto_suggestion, AutoSuggestionSwitch::DisabledByPolicy);
        assert!(!policy.auto_outbound_allowed());
        assert_eq!(policy.audit_retention_days, 180);
    }
}