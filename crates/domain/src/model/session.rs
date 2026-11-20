use serde::{Deserialize, Serialize};

use crate::error::CopilotError;
use crate::model::enums::{
    ChannelPreference, ConsentMode, Platform, SessionState,
};
use crate::vo::{SessionId, WallClock};

/// 会话聚合根。状态只能通过 [`Session::transition`] 迁移。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub state: SessionState,
    pub binding: PlatformBinding,
    pub channel: ChannelPreference,
    pub started_at: WallClock,
    pub ended_at: Option<WallClock>,
    pub retention: RetentionWindow,
    pub consent: ConsentRecord,
    pub participants: Option<ParticipantList>,
    pub screen_share_protection: bool,
}

/// 平台绑定：一个会话只能绑定单一平台与接入通道（SPEC 6.1）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformBinding {
    pub platform: Platform,
    pub version: String,
    pub preference: ChannelPreference,
    pub subtitle_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionWindow {
    pub days: u16,
}

impl RetentionWindow {
    pub const MIN_DAYS: u16 = 1;
    pub const MAX_DAYS: u16 = 365;

    pub fn new(days: u16) -> Result<Self, RetentionError> {
        if !(Self::MIN_DAYS..=Self::MAX_DAYS).contains(&days) {
            return Err(RetentionError(days));
        }
        Ok(Self { days })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("留存期 {0} 天超出允许范围 1–365")]
pub struct RetentionError(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentRecord {
    pub mode: ConsentMode,
    pub granted_at: WallClock,
    /// 采集告知的版本号；告知文案变更后需重新征得同意。
    pub notice_version: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipantList {
    pub known_count: u16,
}

/// 文本区间，绑定音频时间轴，用于要点回溯（SPEC 6.3-2）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextRange {
    pub start_ms: u64,
    pub end_ms: u64,
}

impl TextRange {
    pub fn new(start_ms: u64, end_ms: u64) -> Option<Self> {
        (start_ms < end_ms).then_some(Self { start_ms, end_ms })
    }
}

fn allowed_transitions(from: SessionState) -> &'static [SessionState] {
    match from {
        SessionState::NotStarted => &[SessionState::Listening, SessionState::Deleted],
        SessionState::Listening => &[
            SessionState::Paused,
            SessionState::Silent,
            SessionState::Ended,
            SessionState::Deleted,
        ],
        SessionState::Paused => &[SessionState::Listening, SessionState::Ended, SessionState::Deleted],
        SessionState::Silent => &[SessionState::Listening, SessionState::Ended, SessionState::Deleted],
        SessionState::Ended => &[SessionState::Deleted],
        SessionState::Deleted => &[],
    }
}

impl Session {
    pub fn transition(mut self, next: SessionState) -> Result<Self, CopilotError> {
        if !allowed_transitions(self.state).contains(&next) {
            return Err(CopilotError::StoreFailure {
                detail: format!("非法会话状态迁移 {:?} -> {next:?}", self.state),
            });
        }
        self.state = next;
        Ok(self)
    }

    pub fn is_eligible_for_capture(&self) -> bool {
        self.state.allows_capture() && self.consent.notice_version > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> Session {
        Session {
            id: SessionId::new("sess-1").expect("id"),
            state: SessionState::NotStarted,
            binding: PlatformBinding {
                platform: Platform::TencentMeeting,
                version: "3.28".into(),
                preference: ChannelPreference::SubtitleFirst,
                subtitle_enabled: true,
            },
            channel: ChannelPreference::SubtitleFirst,
            started_at: WallClock::from_unix_millis(1_000),
            ended_at: None,
            retention: RetentionWindow::new(30).expect("retention"),
            consent: ConsentRecord {
                mode: ConsentMode::Explicit,
                granted_at: WallClock::from_unix_millis(1_000),
                notice_version: 1,
            },
            participants: None,
            screen_share_protection: true,
        }
    }

    #[test]
    fn legal_transition_succeeds() {
        let listening = session()
            .transition(SessionState::Listening)
            .expect("合法");
        assert!(listening.is_eligible_for_capture());
    }

    #[test]
    fn illegal_transition_is_rejected() {
        assert!(session().transition(SessionState::Ended).is_err());
    }

    #[test]
    fn deleted_is_terminal() {
        let deleted = session()
            .transition(SessionState::Deleted)
            .expect("合法");
        assert!(deleted.transition(SessionState::Listening).is_err());
    }

    #[test]
    fn retention_bounds() {
        assert!(RetentionWindow::new(0).is_err());
        assert!(RetentionWindow::new(366).is_err());
        assert!(RetentionWindow::new(365).is_ok());
    }
}