//! 通道领域类型与四状态迁移表（DESIGN 2.1.3(2)、2.2.2.1）。
//!
//! 纪律：
//! - 状态值不承担成因；原因由独立的封闭枚举表达。
//! - 阈值判定只使用 `MonoMillis`，禁止墙钟参与。
//! - 类型中不存在文本、样本、姓名或凭据字段。

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use super::enums::Platform;
use crate::vo::{MonoMillis, WallClock};

/// 两类采集通道。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelKind {
    Subtitle,
    Audio,
}

impl ChannelKind {
    pub const ALL: [Self; 2] = [Self::Subtitle, Self::Audio];

    pub const fn code(self) -> &'static str {
        match self {
            Self::Subtitle => "subtitle",
            Self::Audio => "audio",
        }
    }
}

/// 通道四状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelState {
    Probing,
    Available,
    Degraded,
    Unavailable,
}

impl ChannelState {
    pub const ALL: [Self; 4] = [
        Self::Probing,
        Self::Available,
        Self::Degraded,
        Self::Unavailable,
    ];

    pub const fn is_usable(self) -> bool {
        matches!(self, Self::Available | Self::Degraded)
    }

    pub const fn code(self) -> &'static str {
        match self {
            Self::Probing => "probing",
            Self::Available => "available",
            Self::Degraded => "degraded",
            Self::Unavailable => "unavailable",
        }
    }
}

/// 平台 × 通道的唯一实例键。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelKey {
    pub platform: Platform,
    pub kind: ChannelKind,
}

impl ChannelKey {
    pub const fn new(platform: Platform, kind: ChannelKind) -> Self {
        Self { platform, kind }
    }
}

/// `Platform` 既有契约不允许追加 `Ord`，这里用显式秩实现 `ChannelKey` 的全序。
fn platform_rank(platform: Platform) -> u8 {
    match platform {
        Platform::TencentMeeting => 0,
        Platform::Feishu => 1,
        Platform::Zoom => 2,
    }
}

impl Ord for ChannelKey {
    fn cmp(&self, other: &Self) -> Ordering {
        platform_rank(self.platform)
            .cmp(&platform_rank(other.platform))
            .then_with(|| self.kind.cmp(&other.kind))
    }
}

impl PartialOrd for ChannelKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// 状态成因（14 个，对应 SPEC 6.1-4）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateReason {
    ProbeCompleted,
    ContainerMissing,
    PermissionMissing,
    VersionMismatch,
    OcrOnly,
    ConsecutiveReadFailures,
    WindowRebuilt,
    NoValidText,
    OcrFailureRateAboveThreshold,
    DeviceFallback,
    SuspectedSilence,
    DeviceUnplugged,
    PermissionRevoked,
    UserRetry,
}

impl StateReason {
    pub const ALL: [Self; 14] = [
        Self::ProbeCompleted,
        Self::ContainerMissing,
        Self::PermissionMissing,
        Self::VersionMismatch,
        Self::OcrOnly,
        Self::ConsecutiveReadFailures,
        Self::WindowRebuilt,
        Self::NoValidText,
        Self::OcrFailureRateAboveThreshold,
        Self::DeviceFallback,
        Self::SuspectedSilence,
        Self::DeviceUnplugged,
        Self::PermissionRevoked,
        Self::UserRetry,
    ];
}

/// 生效通道切换原因（9 个）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwitchReason {
    SessionInitial,
    SubtitleNoValidText,
    SubtitleReadFailure,
    SubtitleStableRecovered,
    AudioUnavailable,
    AudioRecovered,
    PreferenceChanged,
    UserSelected,
    CooldownExpired,
}

impl SwitchReason {
    pub const ALL: [Self; 9] = [
        Self::SessionInitial,
        Self::SubtitleNoValidText,
        Self::SubtitleReadFailure,
        Self::SubtitleStableRecovered,
        Self::AudioUnavailable,
        Self::AudioRecovered,
        Self::PreferenceChanged,
        Self::UserSelected,
        Self::CooldownExpired,
    ];
}

/// 采集缺口原因（2 个）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GapReason {
    BothChannelsUnavailable,
    PreferenceRestricted,
}

/// 迁移判定结果。`Rejected` 是返回值，不产生任何副作用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionOutcome {
    Migrated {
        from: ChannelState,
        to: ChannelState,
        reason: StateReason,
    },
    InternalAdjustment {
        state: ChannelState,
        reason: StateReason,
    },
    NoOp,
    Rejected {
        from: ChannelState,
        to: ChannelState,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ChannelStateError {
    #[error("重复实例：{0:?}")]
    DuplicateInstance(ChannelKey),
    #[error("时钟回退：已见最大 {seen_max:?}，本次 {at:?}")]
    ClockRegression {
        seen_max: MonoMillis,
        at: MonoMillis,
    },
    #[error("平台不匹配：绑定 {bound:?}，观测 {observed:?}")]
    ForeignPlatform { bound: Platform, observed: Platform },
}

static PROBING_TARGETS: [ChannelState; 3] = [
    ChannelState::Available,
    ChannelState::Degraded,
    ChannelState::Unavailable,
];
static AVAILABLE_TARGETS: [ChannelState; 2] = [ChannelState::Degraded, ChannelState::Unavailable];
static DEGRADED_TARGETS: [ChannelState; 2] = [ChannelState::Available, ChannelState::Unavailable];
static UNAVAILABLE_TARGETS: [ChannelState; 1] = [ChannelState::Probing];

/// 唯一合法目标集合来源；application 不得硬编码目标状态。
pub fn legal_targets(from: ChannelState) -> &'static [ChannelState] {
    match from {
        ChannelState::Probing => &PROBING_TARGETS,
        ChannelState::Available => &AVAILABLE_TARGETS,
        ChannelState::Degraded => &DEGRADED_TARGETS,
        ChannelState::Unavailable => &UNAVAILABLE_TARGETS,
    }
}

/// 固定优先级：InternalAdjustment > NoOp > Migrated > Rejected。
#[allow(clippy::unnecessary_wraps)]
pub fn transition(
    from: ChannelState,
    to: ChannelState,
    reason: StateReason,
) -> Result<TransitionOutcome, ChannelStateError> {
    if from == to {
        if from == ChannelState::Degraded
            && matches!(reason, StateReason::OcrOnly | StateReason::DeviceFallback)
        {
            return Ok(TransitionOutcome::InternalAdjustment {
                state: from,
                reason,
            });
        }
        return Ok(TransitionOutcome::NoOp);
    }
    if legal_targets(from).contains(&to) {
        return Ok(TransitionOutcome::Migrated { from, to, reason });
    }
    Ok(TransitionOutcome::Rejected { from, to })
}

/// 通道实例的只读快照（阶段 A 仅定义，不做 IO）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelStateSnapshot {
    pub key: ChannelKey,
    pub state: ChannelState,
    pub reason: Option<StateReason>,
    pub entered_at: MonoMillis,
    pub changed_at: MonoMillis,
}

/// 便于 DTO 转换的墙钟视图；不参与阈值判定。
pub fn wall_view(at: MonoMillis) -> WallClock {
    at.to_wall()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_states_and_two_kinds_are_closed() {
        assert_eq!(ChannelState::ALL.len(), 4);
        assert_eq!(ChannelKind::ALL.len(), 2);
        assert!(ChannelState::Available.is_usable());
        assert!(ChannelState::Degraded.is_usable());
        assert!(!ChannelState::Probing.is_usable());
        assert!(!ChannelState::Unavailable.is_usable());
    }

    #[test]
    fn reason_enums_are_closed() {
        assert_eq!(StateReason::ALL.len(), 14);
        assert_eq!(SwitchReason::ALL.len(), 9);
    }

    #[test]
    fn channel_key_ord_is_stable() {
        let keys = [
            ChannelKey::new(Platform::Zoom, ChannelKind::Audio),
            ChannelKey::new(Platform::TencentMeeting, ChannelKind::Subtitle),
            ChannelKey::new(Platform::Feishu, ChannelKind::Audio),
        ];
        let mut sorted = keys;
        sorted.sort();
        assert_eq!(sorted[0].platform, Platform::TencentMeeting);
        assert_eq!(sorted[2].platform, Platform::Zoom);
    }
}
