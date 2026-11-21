//! 切换记录跟踪（B2.23.8.4 的同步写语义）。
//!
//! 切换记录一经生效即写入；写失败只计数，不回滚 active。
//! 同毫秒合并与 tick 级 flush 留待服务门面装配时接入。

use meeting_copilot_domain::model::SwitchReason;
use meeting_copilot_domain::vo::{MonoMillis, SessionId, WallClock};

use super::channel_arbiter::Decision;
use super::types::{ActiveChannel, ChannelSwitchRecord};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("switch log unavailable")]
pub struct SwitchLogError;

pub trait ChannelSwitchLog {
    fn append(&self, record: &ChannelSwitchRecord) -> Result<(), SwitchLogError>;
}

pub struct SwitchTracker {
    active: ActiveChannel,
    record_missing: u64,
}

impl Default for SwitchTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl SwitchTracker {
    pub const fn new() -> Self {
        Self {
            active: ActiveChannel::None,
            record_missing: 0,
        }
    }

    pub const fn active(&self) -> ActiveChannel {
        self.active
    }

    pub const fn record_missing(&self) -> u64 {
        self.record_missing
    }

    /// 初选由“无 → 字幕/音频”写 `SessionInitial`；无变化不写。
    pub fn on_decision(
        &mut self,
        session: &SessionId,
        decision: Decision,
        reason: SwitchReason,
        at: MonoMillis,
        wall_at: WallClock,
        log: &dyn ChannelSwitchLog,
    ) -> Option<ChannelSwitchRecord> {
        if decision.active == self.active {
            return None;
        }
        let from = self.active;
        let to = decision.active;
        let reason = if from == ActiveChannel::None {
            SwitchReason::SessionInitial
        } else {
            reason
        };
        let record = ChannelSwitchRecord {
            session: session.clone(),
            from,
            to,
            reason,
            at,
            wall_at,
        };
        self.active = to;
        if log.append(&record).is_err() {
            self.record_missing = self.record_missing.saturating_add(1);
        }
        Some(record)
    }
}

/// 同毫秒、同会话的连续变化合并：前值取首条、后值取末条、原因取首条。
pub fn merge_same_millisecond(records: Vec<ChannelSwitchRecord>) -> Vec<ChannelSwitchRecord> {
    let mut merged: Vec<ChannelSwitchRecord> = Vec::new();
    for record in records {
        if let Some(last) = merged.last_mut() {
            if last.session == record.session && last.at == record.at {
                last.to = record.to;
                continue;
            }
        }
        merged.push(record);
    }
    merged
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    fn record(
        session: &str,
        from: ActiveChannel,
        to: ActiveChannel,
        at: u64,
        reason: SwitchReason,
    ) -> ChannelSwitchRecord {
        ChannelSwitchRecord {
            session: SessionId::new(session).expect("session"),
            from,
            to,
            reason,
            at: MonoMillis::from_millis(at),
            wall_at: WallClock::from_unix_millis(0),
        }
    }

    #[test]
    fn same_millisecond_merge_keeps_first_from_and_reason() {
        let merged = merge_same_millisecond(vec![
            record(
                "s1",
                ActiveChannel::None,
                ActiveChannel::Subtitle,
                0,
                SwitchReason::SessionInitial,
            ),
            record(
                "s1",
                ActiveChannel::Subtitle,
                ActiveChannel::Audio,
                0,
                SwitchReason::SubtitleNoValidText,
            ),
        ]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].from, ActiveChannel::None);
        assert_eq!(merged[0].to, ActiveChannel::Audio);
        assert_eq!(merged[0].reason, SwitchReason::SessionInitial);
    }

    #[test]
    fn different_milliseconds_are_not_merged() {
        let merged = merge_same_millisecond(vec![
            record(
                "s1",
                ActiveChannel::None,
                ActiveChannel::Subtitle,
                0,
                SwitchReason::SessionInitial,
            ),
            record(
                "s1",
                ActiveChannel::Subtitle,
                ActiveChannel::Audio,
                1,
                SwitchReason::SubtitleNoValidText,
            ),
        ]);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn different_sessions_are_not_merged() {
        let merged = merge_same_millisecond(vec![
            record(
                "s1",
                ActiveChannel::None,
                ActiveChannel::Subtitle,
                0,
                SwitchReason::SessionInitial,
            ),
            record(
                "s2",
                ActiveChannel::None,
                ActiveChannel::Audio,
                0,
                SwitchReason::SessionInitial,
            ),
        ]);
        assert_eq!(merged.len(), 2);
    }

    struct RecordingLog {
        records: Mutex<Vec<ChannelSwitchRecord>>,
        fail: bool,
    }

    impl RecordingLog {
        fn new(fail: bool) -> Self {
            Self {
                records: Mutex::new(Vec::new()),
                fail,
            }
        }
    }

    impl ChannelSwitchLog for RecordingLog {
        fn append(&self, record: &ChannelSwitchRecord) -> Result<(), SwitchLogError> {
            if self.fail {
                return Err(SwitchLogError);
            }
            self.records.lock().expect("lock").push(record.clone());
            Ok(())
        }
    }

    fn decision(active: ActiveChannel) -> Decision {
        Decision { active, gap: None }
    }

    #[test]
    fn initial_none_to_subtitle_writes_session_initial() {
        let mut tracker = SwitchTracker::new();
        let log = RecordingLog::new(false);
        let session = SessionId::new("s1").expect("session");
        let record = tracker
            .on_decision(
                &session,
                decision(ActiveChannel::Subtitle),
                SwitchReason::SubtitleNoValidText,
                MonoMillis::from_millis(100),
                WallClock::from_unix_millis(0),
                &log,
            )
            .expect("record");
        assert_eq!(record.from, ActiveChannel::None);
        assert_eq!(record.to, ActiveChannel::Subtitle);
        assert_eq!(record.reason, SwitchReason::SessionInitial);
        assert_eq!(tracker.active(), ActiveChannel::Subtitle);
        assert_eq!(log.records.lock().expect("lock").len(), 1);
    }

    #[test]
    fn no_change_writes_nothing() {
        let mut tracker = SwitchTracker::new();
        let log = RecordingLog::new(false);
        let session = SessionId::new("s1").expect("session");
        tracker.on_decision(
            &session,
            decision(ActiveChannel::None),
            SwitchReason::SubtitleNoValidText,
            MonoMillis::from_millis(0),
            WallClock::from_unix_millis(0),
            &log,
        );
        assert_eq!(tracker.active(), ActiveChannel::None);
        assert!(log.records.lock().expect("lock").is_empty());
    }

    #[test]
    fn append_failure_increments_without_rollback() {
        let mut tracker = SwitchTracker::new();
        let log = RecordingLog::new(true);
        let session = SessionId::new("s1").expect("session");
        let record = tracker.on_decision(
            &session,
            decision(ActiveChannel::Audio),
            SwitchReason::AudioRecovered,
            MonoMillis::from_millis(10),
            WallClock::from_unix_millis(0),
            &log,
        );
        assert!(record.is_some());
        assert_eq!(tracker.active(), ActiveChannel::Audio);
        assert_eq!(tracker.record_missing(), 1);
        assert!(log.records.lock().expect("lock").is_empty());
    }

    #[test]
    fn preference_change_reason_is_preserved_after_initial() {
        let mut tracker = SwitchTracker::new();
        let log = RecordingLog::new(false);
        let session = SessionId::new("s1").expect("session");
        tracker.on_decision(
            &session,
            decision(ActiveChannel::Subtitle),
            SwitchReason::SessionInitial,
            MonoMillis::from_millis(0),
            WallClock::from_unix_millis(0),
            &log,
        );
        let record = tracker
            .on_decision(
                &session,
                decision(ActiveChannel::Audio),
                SwitchReason::PreferenceChanged,
                MonoMillis::from_millis(10),
                WallClock::from_unix_millis(0),
                &log,
            )
            .expect("record");
        assert_eq!(record.reason, SwitchReason::PreferenceChanged);
    }
}
