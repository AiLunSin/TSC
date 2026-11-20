//! 统一错误模型与错误码映射（DESIGN 2.2.1）。
//!
//! 11 个错误码必须全覆盖，且每个变体提供中文用户提示与可重试判定。

use serde::{Deserialize, Serialize};

/// 对外暴露的错误码。任何新增变体都必须同时登记到 [`ErrorCode::ALL`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    AudioDeviceUnavailable,
    AudioCaptureFailed,
    AsrFailed,
    StoreFailure,
    PrivacyViolation,
    SearchFailed,
    MinutesFailed,
    ConsentRequired,
    AuthFailed,
    ProviderConfigInvalid,
    AdapterVersionIncompatible,
    NetworkOffline,
}

impl ErrorCode {
    pub const ALL: [Self; 12] = [
        Self::AudioDeviceUnavailable,
        Self::AudioCaptureFailed,
        Self::AsrFailed,
        Self::SearchFailed,
        Self::StoreFailure,
        Self::PrivacyViolation,
        Self::MinutesFailed,
        Self::ConsentRequired,
        Self::AuthFailed,
        Self::ProviderConfigInvalid,
        Self::AdapterVersionIncompatible,
        Self::NetworkOffline,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AudioDeviceUnavailable => "E-AUDIO-001",
            Self::AudioCaptureFailed => "E-AUDIO-002",
            Self::AsrFailed => "E-ASR-002",
            Self::StoreFailure => "E-STORE-003",
            Self::PrivacyViolation => "E-PRIV-004",
            Self::SearchFailed => "E-SEARCH-005",
            Self::MinutesFailed => "E-MIN-006",
            Self::ConsentRequired => "E-CONSENT-007",
            Self::AuthFailed => "E-AUTH-008",
            Self::ProviderConfigInvalid => "E-PROVIDER-009",
            Self::AdapterVersionIncompatible => "E-ADAPTER-010",
            Self::NetworkOffline => "E-NETWORK-011",
        }
    }
}

/// 策略拒绝码（Guardrail::Deny 使用）。共 11 值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DenyCode {
    ConsentMissing,
    OutboundNotGranted,
    SensitiveTermBlocked,
    ProviderNotAllowed,
    DomainBlocked,
    ScreenShareProtection,
    SilentPeriod,
    RateLimited,
    SanitizeRequired,
    RetentionExpired,
    PolicyUnavailable,
}

impl DenyCode {
    pub const ALL: [Self; 11] = [
        Self::ConsentMissing,
        Self::OutboundNotGranted,
        Self::SensitiveTermBlocked,
        Self::ProviderNotAllowed,
        Self::DomainBlocked,
        Self::ScreenShareProtection,
        Self::SilentPeriod,
        Self::RateLimited,
        Self::SanitizeRequired,
        Self::RetentionExpired,
        Self::PolicyUnavailable,
    ];

    pub const fn user_message(self) -> &'static str {
        match self {
            Self::ConsentMissing => "尚未取得会话采集同意",
            Self::OutboundNotGranted => "尚未授权自动外发检索",
            Self::SensitiveTermBlocked => "命中敏感术语，已阻止外发",
            Self::ProviderNotAllowed => "当前检索源不在允许范围内",
            Self::DomainBlocked => "目标域名已被策略屏蔽",
            Self::ScreenShareProtection => "投屏保护模式中，暂不显示提示",
            Self::SilentPeriod => "处于静默期，暂不显示提示",
            Self::RateLimited => "已达提示频率上限",
            Self::SanitizeRequired => "查询必须脱敏后才能外发",
            Self::RetentionExpired => "数据已超过留存期",
            Self::PolicyUnavailable => "组织策略不可用，已按最严格默认值拦截",
        }
    }
}

/// 业务错误。禁止在错误信息中携带原始对话、URL 全文或凭据。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CopilotError {
    #[error("音频设备不可用：{detail}")]
    AudioDeviceUnavailable { detail: String },
    #[error("音频采集失败：{detail}")]
    AudioCaptureFailed { detail: String },
    #[error("转写失败：{detail}")]
    AsrFailed { detail: String },
    #[error("本地存储失败：{detail}")]
    StoreFailure { detail: String },
    #[error("隐私约束拒绝：{detail}")]
    PrivacyViolation { detail: String },
    #[error("检索失败：{detail}")]
    SearchFailed { detail: String },
    #[error("纪要生成失败：{detail}")]
    MinutesFailed { detail: String },
    #[error("缺少采集同意")]
    ConsentRequired,
    #[error("鉴权失败：{detail}")]
    AuthFailed { detail: String },
    #[error("检索源配置非法：{detail}")]
    ProviderConfigInvalid { detail: String },
    #[error("适配器版本不兼容：要求 {required}，实际 {actual}")]
    AdapterVersionIncompatible { required: String, actual: String },
    #[error("网络不可用")]
    NetworkOffline,
}

impl CopilotError {
    pub const fn code(&self) -> ErrorCode {
        match self {
            Self::AudioDeviceUnavailable { .. } => ErrorCode::AudioDeviceUnavailable,
            Self::AudioCaptureFailed { .. } => ErrorCode::AudioCaptureFailed,
            Self::AsrFailed { .. } => ErrorCode::AsrFailed,
            Self::StoreFailure { .. } => ErrorCode::StoreFailure,
            Self::PrivacyViolation { .. } => ErrorCode::PrivacyViolation,
            Self::SearchFailed { .. } => ErrorCode::SearchFailed,
            Self::MinutesFailed { .. } => ErrorCode::MinutesFailed,
            Self::ConsentRequired => ErrorCode::ConsentRequired,
            Self::AuthFailed { .. } => ErrorCode::AuthFailed,
            Self::ProviderConfigInvalid { .. } => ErrorCode::ProviderConfigInvalid,
            Self::AdapterVersionIncompatible { .. } => ErrorCode::AdapterVersionIncompatible,
            Self::NetworkOffline => ErrorCode::NetworkOffline,
        }
    }

    pub const fn user_message(&self) -> &'static str {
        match self {
            Self::AudioDeviceUnavailable { .. } => "未找到可用的音频设备，请检查系统声音设置",
            Self::AudioCaptureFailed { .. } => "音频采集中断，正在尝试恢复",
            Self::AsrFailed { .. } => "语音转写失败，已切换到字幕通道",
            Self::StoreFailure { .. } => "本地数据写入失败，本次内容可能不会保存",
            Self::PrivacyViolation { .. } => "操作被隐私策略阻止",
            Self::SearchFailed { .. } => "联网检索暂时失败，可稍后重试",
            Self::MinutesFailed { .. } => "纪要生成失败，可稍后重新生成",
            Self::ConsentRequired => "请先确认采集同意后再开始",
            Self::AuthFailed { .. } => "授权已失效，请重新授权",
            Self::ProviderConfigInvalid { .. } => "检索源配置有误，请检查后重试",
            Self::AdapterVersionIncompatible { .. } => "会议软件版本不受支持，请升级或切换通道",
            Self::NetworkOffline => "网络不可用，已切换至本地模式",
        }
    }

    pub const fn is_retryable(&self) -> bool {
        match self {
            Self::AudioDeviceUnavailable { .. }
            | Self::AudioCaptureFailed { .. }
            | Self::AsrFailed { .. }
            | Self::SearchFailed { .. }
            | Self::MinutesFailed { .. }
            | Self::NetworkOffline => true,
            Self::StoreFailure { .. }
            | Self::PrivacyViolation { .. }
            | Self::ConsentRequired
            | Self::AuthFailed { .. }
            | Self::ProviderConfigInvalid { .. }
            | Self::AdapterVersionIncompatible { .. } => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn sample_errors() -> Vec<CopilotError> {
        vec![
            CopilotError::AudioDeviceUnavailable {
                detail: "no device".into(),
            },
            CopilotError::AudioCaptureFailed {
                detail: "device lost".into(),
            },
            CopilotError::AsrFailed {
                detail: "decode".into(),
            },
            CopilotError::StoreFailure {
                detail: "io".into(),
            },
            CopilotError::PrivacyViolation {
                detail: "sensitive".into(),
            },
            CopilotError::SearchFailed {
                detail: "timeout".into(),
            },
            CopilotError::MinutesFailed {
                detail: "llm".into(),
            },
            CopilotError::ConsentRequired,
            CopilotError::AuthFailed {
                detail: "expired".into(),
            },
            CopilotError::ProviderConfigInvalid {
                detail: "no url".into(),
            },
            CopilotError::AdapterVersionIncompatible {
                required: ">=1.2".into(),
                actual: "1.0".into(),
            },
            CopilotError::NetworkOffline,
        ]
    }

    #[test]
    fn all_eleven_codes_are_covered() {
        let errors = sample_errors();
        assert_eq!(errors.len(), 12);

        let mapped: HashSet<_> = errors.iter().map(|error| error.code()).collect();
        assert_eq!(mapped.len(), ErrorCode::ALL.len());
        for code in ErrorCode::ALL {
            assert!(mapped.contains(&code), "缺少错误码 {}", code.as_str());
        }
    }

    #[test]
    fn every_error_has_human_readable_message() {
        for error in sample_errors() {
            assert!(!error.user_message().is_empty());
            assert!(error.code().as_str().starts_with('E'));
        }
    }

    #[test]
    fn deny_codes_are_unique_and_documented() {
        let unique: HashSet<_> = DenyCode::ALL.iter().collect();
        assert_eq!(unique.len(), DenyCode::ALL.len());
        for code in DenyCode::ALL {
            assert!(!code.user_message().is_empty());
        }
    }
}
