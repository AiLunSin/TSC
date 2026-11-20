use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// 音频时间轴（毫秒）。与墙钟解耦，避免时钟漂移污染字幕对齐。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AudioClock(u64);

impl AudioClock {
    pub const ZERO: Self = Self(0);

    pub const fn from_millis(millis: u64) -> Self {
        Self(millis)
    }

    pub const fn as_millis(self) -> u64 {
        self.0
    }

    pub fn checked_add(self, delta_ms: u64) -> Option<Self> {
        self.0.checked_add(delta_ms).map(Self)
    }

    pub fn saturating_sub(self, other: Self) -> u64 {
        self.0.saturating_sub(other.0)
    }
}

/// 墙钟时间（Unix 毫秒）。用于审计、留存与展示时间。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WallClock(i64);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ClockError {
    #[error("系统时间早于 Unix 纪元")]
    BeforeEpoch,
    #[error("系统时间超出 i64 毫秒表示范围")]
    Overflow,
}

impl WallClock {
    pub const fn from_unix_millis(millis: i64) -> Self {
        Self(millis)
    }

    pub const fn as_unix_millis(self) -> i64 {
        self.0
    }

    pub fn now() -> Result<Self, ClockError> {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ClockError::BeforeEpoch)?;
        i64::try_from(elapsed.as_millis())
            .map(Self)
            .map_err(|_| ClockError::Overflow)
    }

    pub fn elapsed_since(self, earlier: Self) -> Option<u64> {
        u64::try_from(self.0 - earlier.0).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_clock_add_overflow_is_none() {
        assert_eq!(AudioClock::from_millis(u64::MAX).checked_add(1), None);
        assert_eq!(
            AudioClock::ZERO
                .checked_add(1500)
                .map(AudioClock::as_millis),
            Some(1500)
        );
    }

    #[test]
    fn wall_clock_elapsed() {
        let a = WallClock::from_unix_millis(1_000);
        let b = WallClock::from_unix_millis(4_500);
        assert_eq!(b.elapsed_since(a), Some(3_500));
        assert_eq!(a.elapsed_since(b), None);
    }
}
