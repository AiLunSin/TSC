use serde::{Deserialize, Serialize};

/// 会话状态（SPEC 6.1-4）。唯一写入点是 `Session::transition`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    NotStarted,
    Listening,
    Paused,
    Silent,
    Ended,
    Deleted,
}

impl SessionState {
    pub const ALL: [Self; 6] = [
        Self::NotStarted,
        Self::Listening,
        Self::Paused,
        Self::Silent,
        Self::Ended,
        Self::Deleted,
    ];

    pub const fn allows_capture(self) -> bool {
        matches!(self, Self::Listening)
    }
}

/// 要点类型（SPEC 6.3-3）。无法判定时降级为 `KeyInfo` 并降低置信度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InsightKind {
    #[default]
    KeyInfo,
    Decision,
    Todo,
    Question,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InsightStatus {
    #[default]
    Auto,
    Edited,
    Deleted,
}

/// 片段来源（SPEC 6.2-7）。任何要点都必须能回溯到来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Microphone,
    PlatformAudio,
    PlatformSubtitle,
    ExternalTranscription,
}

impl SourceKind {
    pub const ALL: [Self; 4] = [
        Self::Microphone,
        Self::PlatformAudio,
        Self::PlatformSubtitle,
        Self::ExternalTranscription,
    ];

    pub const fn is_subtitle(self) -> bool {
        matches!(self, Self::PlatformSubtitle)
    }
}

/// 提示卡触发方式（SPEC 6.4-2）。**默认自动主动**。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TriggerKind {
    #[default]
    Auto,
    OnDemand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CardStatus {
    #[default]
    Shown,
    Collapsed,
    Withdrawn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TrustTier {
    Official,
    AuthoritativeMedia,
    #[default]
    GeneralWeb,
    UserGenerated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadingMode {
    Subtitle,
    Audio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresentationMode {
    SidePanel,
    Floating,
    Badge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackStatus {
    #[default]
    None,
    Adopted,
    Dismissed,
    Reported,
}

/// 信息缺口类型（DESIGN 2.6.2 的六类触发识别）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GapKind {
    ProperNoun,
    Abbreviation,
    PolicyStandard,
    Metric,
    UnansweredQuestion,
    Acronym,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GapState {
    #[default]
    Detected,
    Scoring,
    Retrieving,
    Composed,
    Pushed,
    Dropped,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChannelPreference {
    #[default]
    SubtitleFirst,
    AudioFirst,
    SubtitleOnly,
    AudioOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    TencentMeeting,
    Feishu,
    Zoom,
}

impl Platform {
    pub const ALL: [Self; 3] = [Self::TencentMeeting, Self::Feishu, Self::Zoom];

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::TencentMeeting => "腾讯会议",
            Self::Feishu => "飞书",
            Self::Zoom => "Zoom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    SearxngHttp,
    Tavily,
    Brave,
    Bing,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProviderHealth {
    #[default]
    Healthy,
    Degraded,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    #[default]
    Pending,
    Running,
    Succeeded,
    Failed,
    Dead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConsentMode {
    #[default]
    Explicit,
    OrganizationImplicit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PolicyScope {
    #[default]
    Personal,
    Organization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AutoSuggestionSwitch {
    #[default]
    Enabled,
    DisabledByUser,
    DisabledByPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TermKind {
    Product,
    Acronym,
    Policy,
    Metric,
    Person,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EnabledState {
    #[default]
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WithdrawReason {
    PolicyDenied,
    LowConfidence,
    UserDismissed,
    SourceUnavailable,
    Conflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessStatus {
    Ok,
    LoginWall,
    RobotsDenied,
    Timeout,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    SessionStart,
    SessionEnd,
    ConsentGranted,
    ConsentRevoked,
    OutboundQuery,
    CardShown,
    Export,
    Delete,
    ConfigChange,
    PolicyApplied,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_defaults_to_auto() {
        assert_eq!(TriggerKind::default(), TriggerKind::Auto);
    }

    #[test]
    fn session_state_only_allows_capture_when_listening() {
        for state in SessionState::ALL {
            assert_eq!(state.allows_capture(), state == SessionState::Listening);
        }
    }

    #[test]
    fn all_three_platforms_present() {
        assert_eq!(Platform::ALL.len(), 3);
    }

    #[test]
    fn enums_roundtrip_through_json() {
        for platform in Platform::ALL {
            let json = serde_json::to_string(&platform).expect("序列化");
            let back: Platform = serde_json::from_str(&json).expect("反序列化");
            assert_eq!(platform, back);
        }
    }
}
