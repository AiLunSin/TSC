//! 通道采集 DTO 与阈值配置（DESIGN 2.3.2、2.4.3）。
//!
//! 所有载荷只含枚举/数值；不存在字幕文本、音频样本、说话人或参会人字段。

use meeting_copilot_domain::error::ErrorCode;
use meeting_copilot_domain::model::{
    ChannelKey, ChannelKind, ChannelPreference, ChannelState, GapReason, Platform, SessionState,
    StateReason, SwitchReason,
};
use meeting_copilot_domain::vo::{MonoMillis, SessionId, WallClock};

/// 通道观测载荷：封闭 11 类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationPayload {
    Probe {
        readable: bool,
        version_supported: bool,
        ocr_only: bool,
    },
    ValidText {
        estimated: bool,
        confidence_milli: Option<u16>,
    },
    ReadFailure,
    WindowRebuilt,
    Silence {
        blocks: u8,
        vad: bool,
    },
    OcrStats {
        failed: u32,
        total: u32,
    },
    Permission {
        revoked: bool,
    },
    AudioDevice {
        event: AudioDeviceEvent,
    },
    UserRetry,
    OcrRoiChanged,
    AudioFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioDeviceEvent {
    OpenFailed,
    Unplugged,
    Lost,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelObservation {
    pub session: Option<SessionId>,
    pub platform: Platform,
    pub kind: ChannelKind,
    pub at: MonoMillis,
    pub payload: ObservationPayload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelStatusEvent {
    pub key: ChannelKey,
    pub state: ChannelState,
    pub reason: StateReason,
    pub sequence: u64,
    pub determined_at: MonoMillis,
    pub wall_at: WallClock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelSnapshotEntry {
    pub key: ChannelKey,
    pub state: ChannelState,
    pub reason: Option<StateReason>,
    pub entered_at: MonoMillis,
    pub changed_at: MonoMillis,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelSnapshot {
    pub entries: Vec<ChannelSnapshotEntry>,
    pub generated_at: MonoMillis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveChannel {
    Subtitle,
    Audio,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureAvailability {
    Available { channel: ActiveChannel },
    Unavailable { reason: GapReason, code: ErrorCode },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelSwitchRecord {
    pub session: SessionId,
    pub from: ActiveChannel,
    pub to: ActiveChannel,
    pub reason: SwitchReason,
    pub at: MonoMillis,
    pub wall_at: WallClock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureGapRecord {
    pub session: SessionId,
    pub start: MonoMillis,
    pub end: MonoMillis,
    pub reason: GapReason,
    pub backfilled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsrIntentKind {
    Validation,
    GapBackfill,
    Primary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsrIntent {
    pub session: SessionId,
    pub kind: AsrIntentKind,
    pub window_start: MonoMillis,
    pub window_end: MonoMillis,
    pub chunk_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeBoundary {
    pub session: SessionId,
    pub resume_from: MonoMillis,
    pub gap_end: MonoMillis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    ForeignPlatform,
    DuplicateInstance,
    ClockRegression,
    InvalidTransition,
    SessionNotFound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropReason {
    SessionDetached,
    SourceCancelled,
    QueueFull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationOutcome {
    Applied,
    NoOp,
    Rejected { reason: RejectReason },
    Dropped { reason: DropReason },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionContextView {
    pub session: SessionId,
    pub platform: Platform,
    pub channel_preference: ChannelPreference,
    pub session_state: SessionState,
}

/// 阈值配置：全部可注入、可下调；越界在构造期拒绝。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapturePolicy {
    pub advance_interval_ms: u64,
    pub silence_fallback_ms: u64,
    pub stable_switch_back_ms: u64,
    pub read_failure_threshold: u8,
    pub silence_blocks: u8,
    pub ocr_failure_numerator: u32,
    pub ocr_failure_denominator: u32,
    pub cooldown_window_ms: u64,
    pub cooldown_threshold: u32,
    pub cooldown_duration_ms: u64,
    pub gap_min_ms: u64,
    pub gap_tolerance_ms: u64,
    pub backfill_max_ms: u64,
    pub validation_interval_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CapturePolicyError {
    #[error("{field} 越界：{value}")]
    OutOfRange { field: &'static str, value: u64 },
    #[error("OCR 失败比例分母不能为 0")]
    ZeroDenominator,
}

impl Default for CapturePolicy {
    fn default() -> Self {
        Self {
            advance_interval_ms: 100,
            silence_fallback_ms: 5_000,
            stable_switch_back_ms: 10_000,
            read_failure_threshold: 3,
            silence_blocks: 3,
            ocr_failure_numerator: 1,
            ocr_failure_denominator: 2,
            cooldown_window_ms: 60_000,
            cooldown_threshold: 3,
            cooldown_duration_ms: 120_000,
            gap_min_ms: 1_000,
            gap_tolerance_ms: 400,
            backfill_max_ms: 8_000,
            validation_interval_ms: 10_000,
        }
    }
}

impl CapturePolicy {
    pub fn validate(&self) -> Result<(), CapturePolicyError> {
        if !(100..=250).contains(&self.advance_interval_ms) {
            return Err(CapturePolicyError::OutOfRange {
                field: "advance_interval_ms",
                value: self.advance_interval_ms,
            });
        }
        if self.silence_fallback_ms == 0
            || self.stable_switch_back_ms == 0
            || self.read_failure_threshold == 0
            || self.silence_blocks == 0
            || self.cooldown_window_ms == 0
            || self.cooldown_threshold == 0
            || self.cooldown_duration_ms == 0
            || self.gap_min_ms == 0
            || self.backfill_max_ms == 0
        {
            return Err(CapturePolicyError::OutOfRange {
                field: "policy_nonzero",
                value: 0,
            });
        }
        if self.ocr_failure_denominator == 0 {
            return Err(CapturePolicyError::ZeroDenominator);
        }
        if self.ocr_failure_numerator > self.ocr_failure_denominator {
            return Err(CapturePolicyError::OutOfRange {
                field: "ocr_failure_numerator",
                value: u64::from(self.ocr_failure_numerator),
            });
        }
        if !(5_000..=60_000).contains(&self.validation_interval_ms) {
            return Err(CapturePolicyError::OutOfRange {
                field: "validation_interval_ms",
                value: self.validation_interval_ms,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_design() {
        let policy = CapturePolicy::default();
        assert_eq!(policy.advance_interval_ms, 100);
        assert_eq!(policy.silence_fallback_ms, 5_000);
        assert_eq!(policy.stable_switch_back_ms, 10_000);
        assert_eq!(policy.read_failure_threshold, 3);
        assert_eq!(policy.silence_blocks, 3);
        assert_eq!(policy.cooldown_window_ms, 60_000);
        assert_eq!(policy.cooldown_duration_ms, 120_000);
        assert!(policy.validate().is_ok());
    }

    #[test]
    fn interval_bounds_are_enforced() {
        let too_low = CapturePolicy {
            advance_interval_ms: 99,
            ..Default::default()
        };
        assert!(too_low.validate().is_err());
        let lower = CapturePolicy {
            advance_interval_ms: 100,
            ..Default::default()
        };
        assert!(lower.validate().is_ok());
        let upper = CapturePolicy {
            advance_interval_ms: 250,
            ..Default::default()
        };
        assert!(upper.validate().is_ok());
        let too_high = CapturePolicy {
            advance_interval_ms: 251,
            ..Default::default()
        };
        assert!(too_high.validate().is_err());
    }

    #[test]
    fn validation_interval_has_minimum() {
        let too_small = CapturePolicy {
            validation_interval_ms: 4_999,
            ..Default::default()
        };
        assert!(too_small.validate().is_err());
        let minimum = CapturePolicy {
            validation_interval_ms: 5_000,
            ..Default::default()
        };
        assert!(minimum.validate().is_ok());
    }

    #[test]
    fn ocr_ratio_is_bounded() {
        let zero_denominator = CapturePolicy {
            ocr_failure_denominator: 0,
            ..Default::default()
        };
        assert!(zero_denominator.validate().is_err());
        let numerator_too_large = CapturePolicy {
            ocr_failure_numerator: 3,
            ocr_failure_denominator: 2,
            ..Default::default()
        };
        assert!(numerator_too_large.validate().is_err());
    }
}
