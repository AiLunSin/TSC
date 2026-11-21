//! ASR 投递预算与抖动抑制（B2.23.8.5、8.6）。
//!
//! 纯状态机；不依赖墙钟、网络或 ASR 实现。

use std::collections::VecDeque;

use meeting_copilot_domain::vo::MonoMillis;

use super::types::{ActiveChannel, AsrIntentKind};

pub struct AsrBudget {
    interval_ms: u64,
    last_issued_at: Option<MonoMillis>,
}

impl AsrBudget {
    pub const fn new(interval_ms: u64) -> Self {
        Self {
            interval_ms,
            last_issued_at: None,
        }
    }

    pub const fn with_default_interval() -> Self {
        Self::new(10_000)
    }

    /// 字幕生效期只允许 Validation/GapBackfill，且共用 10s 预算；
    /// 音频主路径不受预算限制。
    pub fn try_issue(
        &mut self,
        active: ActiveChannel,
        kind: AsrIntentKind,
        at: MonoMillis,
    ) -> bool {
        match active {
            ActiveChannel::Audio => true,
            ActiveChannel::None => false,
            ActiveChannel::Subtitle => {
                if kind == AsrIntentKind::Primary {
                    return false;
                }
                let allowed = self
                    .last_issued_at
                    .is_none_or(|last| at.saturating_sub(last) >= self.interval_ms);
                if allowed {
                    self.last_issued_at = Some(at);
                }
                allowed
            }
        }
    }

    pub fn reset(&mut self) {
        self.last_issued_at = None;
    }
}

pub struct JitterSuppressor {
    window_ms: u64,
    threshold: usize,
    duration_ms: u64,
    fallback_times: VecDeque<MonoMillis>,
    cooldown_until: Option<MonoMillis>,
    trigger_count: u64,
}

impl Default for JitterSuppressor {
    fn default() -> Self {
        Self::new(60_000, 3, 120_000)
    }
}

impl JitterSuppressor {
    pub fn new(window_ms: u64, threshold: usize, duration_ms: u64) -> Self {
        Self {
            window_ms,
            threshold: threshold.max(1),
            duration_ms,
            fallback_times: VecDeque::new(),
            cooldown_until: None,
            trigger_count: 0,
        }
    }

    /// 记录一次“字幕→音频”回退；达到阈值则进入冷却。
    pub fn note_fallback(&mut self, at: MonoMillis) -> bool {
        while let Some(oldest) = self.fallback_times.front() {
            if at.saturating_sub(*oldest) > self.window_ms {
                self.fallback_times.pop_front();
            } else {
                break;
            }
        }
        self.fallback_times.push_back(at);
        if self.fallback_times.len() >= self.threshold {
            self.cooldown_until = Some(MonoMillis::from_millis(
                at.as_millis().saturating_add(self.duration_ms),
            ));
            self.trigger_count = self.trigger_count.saturating_add(1);
            self.fallback_times.clear();
            return true;
        }
        false
    }

    pub fn can_switch_back(&self, now: MonoMillis) -> bool {
        self.cooldown_until
            .is_none_or(|until| now.as_millis() >= until.as_millis())
    }

    /// 用户显式改偏好可立即解除暂缓。
    pub fn release_by_user(&mut self) {
        self.cooldown_until = None;
        self.fallback_times.clear();
    }

    pub const fn trigger_count(&self) -> u64 {
        self.trigger_count
    }

    pub const fn cooldown_until(&self) -> Option<MonoMillis> {
        self.cooldown_until
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtitle_validation_budget_is_shared_per_ten_seconds() {
        let mut budget = AsrBudget::with_default_interval();
        assert!(budget.try_issue(
            ActiveChannel::Subtitle,
            AsrIntentKind::Validation,
            MonoMillis::from_millis(0)
        ));
        assert!(!budget.try_issue(
            ActiveChannel::Subtitle,
            AsrIntentKind::GapBackfill,
            MonoMillis::from_millis(5_000)
        ));
        assert!(budget.try_issue(
            ActiveChannel::Subtitle,
            AsrIntentKind::GapBackfill,
            MonoMillis::from_millis(10_000)
        ));
    }

    #[test]
    fn subtitle_primary_is_rejected() {
        let mut budget = AsrBudget::with_default_interval();
        assert!(!budget.try_issue(
            ActiveChannel::Subtitle,
            AsrIntentKind::Primary,
            MonoMillis::ZERO
        ));
    }

    #[test]
    fn audio_primary_is_unbudgeted() {
        let mut budget = AsrBudget::with_default_interval();
        for _ in 0..20 {
            assert!(budget.try_issue(
                ActiveChannel::Audio,
                AsrIntentKind::Primary,
                MonoMillis::ZERO
            ));
        }
    }

    #[test]
    fn jitter_triggers_after_three_fallbacks_in_window() {
        let mut jitter = JitterSuppressor::default();
        assert!(!jitter.note_fallback(MonoMillis::from_millis(0)));
        assert!(!jitter.note_fallback(MonoMillis::from_millis(1_000)));
        assert!(jitter.note_fallback(MonoMillis::from_millis(2_000)));
        assert_eq!(jitter.trigger_count(), 1);
        assert!(!jitter.can_switch_back(MonoMillis::from_millis(2_000)));
        assert!(!jitter.can_switch_back(MonoMillis::from_millis(121_999)));
        assert!(jitter.can_switch_back(MonoMillis::from_millis(122_000)));
    }

    #[test]
    fn old_fallbacks_are_pruned_and_user_release_clears() {
        let mut jitter = JitterSuppressor::default();
        jitter.note_fallback(MonoMillis::from_millis(0));
        jitter.note_fallback(MonoMillis::from_millis(1_000));
        assert!(!jitter.note_fallback(MonoMillis::from_millis(61_001)));
        jitter.release_by_user();
        assert!(jitter.can_switch_back(MonoMillis::ZERO));
    }
}
