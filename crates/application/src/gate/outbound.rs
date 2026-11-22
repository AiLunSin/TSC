//! G5 外发授权与审计：唯一出口，顺序 `状态 → 范围 → 截断 → 脱敏 → 审计 → 放行`。

use meeting_copilot_domain::model::gate::{
    AuditResult, AuthorizationStatus, GateAuditEvent, GateContentType, GatePolicy,
    GateRejectReason, OutboundIntent, QueryLimit,
};

use crate::gate::input::GateInput;
use crate::gate::{AuditError, AuditSink};

/// 回放与关闭态的默认实现。
pub struct NoopAuditSink;

impl AuditSink for NoopAuditSink {
    fn append(&self, _event: &GateAuditEvent) -> Result<(), AuditError> {
        Ok(())
    }
}

/// 回放与关闭态默认脱敏器：原样返回，不落盘、不记录原文。
pub struct PassthroughSanitizer;

impl meeting_copilot_domain::port::governance::Sanitizer for PassthroughSanitizer {
    fn sanitize_query(
        &self,
        raw: &str,
    ) -> Result<(String, u16), meeting_copilot_domain::error::CopilotError> {
        Ok((raw.to_owned(), 0))
    }

    fn sanitize_output(
        &self,
        raw: &str,
    ) -> Result<String, meeting_copilot_domain::error::CopilotError> {
        Ok(raw.to_owned())
    }

    fn scan(&self, _raw: &str) -> Vec<meeting_copilot_domain::port::governance::SensitiveHit> {
        Vec::new()
    }
}

pub(crate) struct AuthorizeOutcome {
    pub deny: Option<GateRejectReason>,
    pub outbound: Option<OutboundIntent>,
    #[allow(dead_code)]
    pub event: Option<GateAuditEvent>,
    #[allow(dead_code)]
    pub buffered: Vec<GateAuditEvent>,
}

/// 按字符数截断；返回（截断文本, 是否截断）。
pub fn truncate_chars(raw: &str, limit: usize) -> (String, bool) {
    let count = raw.chars().count();
    if count <= limit {
        return (raw.to_owned(), false);
    }
    (raw.chars().take(limit).collect(), true)
}

fn content_type_valid(input: &GateInput<'_>) -> bool {
    input.authorization.content_types == [GateContentType::TextQuery]
}

fn provider_scope_valid(input: &GateInput<'_>) -> bool {
    input.authorization.provider_ids.is_empty()
        || input
            .authorization
            .provider_ids
            .iter()
            .all(|id| input.provider.provider_ids.contains(id))
}

fn build_event(
    input: &GateInput<'_>,
    query_len: usize,
    result: AuditResult,
    reason: Option<GateRejectReason>,
) -> GateAuditEvent {
    GateAuditEvent {
        authorization_status: input.authorization.status,
        granted_at_ms: input.authorization.granted_at_ms,
        content_type: GateContentType::TextQuery,
        query_len,
        provider_ids: input.authorization.provider_ids.clone(),
        result,
        reason,
        occurred_at_ms: input.now_ms,
    }
}

pub(crate) fn authorize(input: &GateInput<'_>, policy: &GatePolicy) -> AuthorizeOutcome {
    let _ = policy;
    let deny = |reason: GateRejectReason, event: GateAuditEvent| AuthorizeOutcome {
        deny: Some(reason),
        outbound: None,
        event: Some(event),
        buffered: Vec::new(),
    };

    if !input.authorization.is_complete()
        || input.authorization.status != AuthorizationStatus::Granted
        || !input.authorization.is_grantable()
        || !content_type_valid(input)
    {
        return deny(
            GateRejectReason::OutboundNotGranted,
            build_event(
                input,
                0,
                AuditResult::Deny,
                Some(GateRejectReason::OutboundNotGranted),
            ),
        );
    }
    if !provider_scope_valid(input) {
        return deny(
            GateRejectReason::ProviderNotAllowed,
            build_event(
                input,
                0,
                AuditResult::Deny,
                Some(GateRejectReason::ProviderNotAllowed),
            ),
        );
    }

    let limit = match input.trigger {
        meeting_copilot_domain::model::enums::TriggerKind::Auto => QueryLimit::Auto,
        meeting_copilot_domain::model::enums::TriggerKind::OnDemand => QueryLimit::OnDemand,
    };
    if let Some(max) = input.authorization.max_query_chars {
        if usize::from(max) < limit.chars() {
            return deny(
                GateRejectReason::OutboundNotGranted,
                build_event(
                    input,
                    0,
                    AuditResult::Deny,
                    Some(GateRejectReason::OutboundNotGranted),
                ),
            );
        }
    }

    let (truncated, was_truncated) = truncate_chars(input.query_seed, limit.chars());
    let sanitized = match input.sanitizer.issue_query(&truncated) {
        Ok(query) => query,
        Err(_) => {
            let event = build_event(
                input,
                0,
                AuditResult::Deny,
                Some(GateRejectReason::SanitizeFailed),
            );
            if input.audit_available {
                let _ = input.audit_sink.append(&event);
                return AuthorizeOutcome {
                    deny: Some(GateRejectReason::SanitizeFailed),
                    outbound: None,
                    event: None,
                    buffered: Vec::new(),
                };
            }
            return AuthorizeOutcome {
                deny: Some(GateRejectReason::SanitizeFailed),
                outbound: None,
                event: None,
                buffered: vec![event],
            };
        }
    };

    let event = build_event(input, sanitized.char_count(), AuditResult::Allow, None);
    if !input.audit_available {
        return AuthorizeOutcome {
            deny: Some(GateRejectReason::AuditUnavailable),
            outbound: None,
            event: None,
            buffered: vec![event],
        };
    }
    if input.audit_sink.append(&event).is_err() {
        return AuthorizeOutcome {
            deny: Some(GateRejectReason::AuditUnavailable),
            outbound: None,
            event: None,
            buffered: vec![event],
        };
    }

    AuthorizeOutcome {
        deny: None,
        outbound: Some(OutboundIntent {
            sanitized_query_len: sanitized.char_count(),
            content_type: GateContentType::TextQuery,
            provider_ids: input.authorization.provider_ids.clone(),
            authorization_status: input.authorization.status,
            truncated: was_truncated,
            queued: false,
            cooldown_until_ms: None,
        }),
        event: Some(event),
        buffered: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncation_counts_chars() {
        let (text, cut) = truncate_chars(&"字".repeat(81), 80);
        assert_eq!(text.chars().count(), 80);
        assert!(cut);
        let (same, cut) = truncate_chars("短", 80);
        assert_eq!(same, "短");
        assert!(!cut);
    }
}
