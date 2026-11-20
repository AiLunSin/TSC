//! 脱敏日志与诊断包（SPEC 4.4-2/4-4-5、DESIGN 2.7.1）。

/// 允许写入结构化日志的字段白名单。原文、URL 全文、查询全文、凭据一律禁止。
pub const ALLOWED_FIELDS: [&str; 12] = [
    "platform",
    "channel",
    "client_version",
    "session_state",
    "error_code",
    "latency_ms",
    "provider_id",
    "trust_tier",
    "card_status",
    "trigger_kind",
    "failure_class",
    "redaction_count",
];

pub const FORBIDDEN_FIELDS: [&str; 6] = [
    "utterance_text",
    "transcript",
    "url",
    "query",
    "credential",
    "audio_samples",
];

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("日志字段 `{0}` 不在白名单内")]
pub struct FieldNotAllowed(pub String);

pub fn validate_field(field: &str) -> Result<(), FieldNotAllowed> {
    if ALLOWED_FIELDS.contains(&field) {
        Ok(())
    } else {
        Err(FieldNotAllowed(field.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forbidden_fields_are_never_allowed() {
        for field in FORBIDDEN_FIELDS {
            assert!(validate_field(field).is_err(), "{field} 必须被拒绝");
        }
    }

    #[test]
    fn allowlist_accepts_diagnostic_fields() {
        for field in ALLOWED_FIELDS {
            assert!(validate_field(field).is_ok());
        }
    }
}