//! 网络状态识别（SPEC 7.2.1-7、7.2.3-4；DESIGN 2.10.2）。
//!
//! 只以真实请求结果为准，不读取系统网络标志。网络恢复的识别延迟由探测间隔决定：
//! 默认每 1 秒探测一次，且间隔被夹在 `(0, NET_RECOVERY_RECOGNIZE_MS]`，因此满足
//! 「网络恢复 ≤3 秒识别」。

/// 网络恢复的识别上限（DESIGN 2.10.2：以真实请求成功为准，≤3 秒识别）。
pub const NET_RECOVERY_RECOGNIZE_MS: i64 = 3_000;
/// 网络探测间隔；小于识别上限，保证识别延迟不超过 3 秒。
pub const NET_PROBE_INTERVAL_MS: i64 = 1_000;

/// 网络状态。`Unknown` 表示还没有过一次真实请求结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkState {
    Unknown,
    Online,
    Offline,
}

impl NetworkState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Online => "online",
            Self::Offline => "offline",
        }
    }
}

/// 由真实请求结果驱动的网络状态机。
#[derive(Debug)]
pub struct NetProbe {
    state: NetworkState,
    poll_interval_ms: i64,
    last_failure_ms: Option<i64>,
    last_success_ms: Option<i64>,
    recovered_at_ms: Option<i64>,
    next_probe_ms: Option<i64>,
}

impl Default for NetProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl NetProbe {
    pub fn new() -> Self {
        Self::with_interval(NET_PROBE_INTERVAL_MS)
    }

    /// 探测间隔会被夹到 `[1, NET_RECOVERY_RECOGNIZE_MS]`，保证识别延迟上限不被配置破坏。
    pub fn with_interval(interval_ms: i64) -> Self {
        let interval = interval_ms.clamp(1, NET_RECOVERY_RECOGNIZE_MS);
        Self {
            state: NetworkState::Unknown,
            poll_interval_ms: interval,
            last_failure_ms: None,
            last_success_ms: None,
            recovered_at_ms: None,
            next_probe_ms: None,
        }
    }

    pub fn state(&self) -> NetworkState {
        self.state
    }

    pub const fn poll_interval_ms(&self) -> i64 {
        self.poll_interval_ms
    }

    pub const fn last_failure_ms(&self) -> Option<i64> {
        self.last_failure_ms
    }

    pub const fn last_success_ms(&self) -> Option<i64> {
        self.last_success_ms
    }

    pub const fn recovered_at_ms(&self) -> Option<i64> {
        self.recovered_at_ms
    }

    /// 上报一次真实请求结果；返回状态是否发生变化。
    pub fn on_result(&mut self, success: bool, now_ms: i64) -> bool {
        let before = self.state;
        if success {
            self.last_success_ms = Some(now_ms);
            self.next_probe_ms = None;
            self.state = NetworkState::Online;
            if before == NetworkState::Offline {
                self.recovered_at_ms = Some(now_ms);
            }
        } else {
            self.last_failure_ms = Some(now_ms);
            self.next_probe_ms = Some(now_ms.saturating_add(self.poll_interval_ms));
            if before != NetworkState::Offline {
                self.state = NetworkState::Offline;
                self.recovered_at_ms = None;
            }
        }
        before != self.state
    }

    /// 离线时是否应发出一次探测请求（由调用方用真实检索或健康检查执行）。
    pub fn should_probe(&self, now_ms: i64) -> bool {
        self.state == NetworkState::Offline && self.next_probe_ms.is_none_or(|next| now_ms >= next)
    }

    /// 探测发出后推进节拍，避免同一 tick 内重复探测。
    pub fn mark_probed(&mut self, now_ms: i64) {
        if self.state == NetworkState::Offline {
            self.next_probe_ms = Some(now_ms.saturating_add(self.poll_interval_ms));
        }
    }

    /// 最近一次「离线 → 在线」的识别延迟；没有恢复记录时为 `None`。
    pub fn recovery_latency_ms(&self) -> Option<i64> {
        match (self.last_failure_ms, self.recovered_at_ms) {
            (Some(failed), Some(recovered)) if recovered >= failed => Some(recovered - failed),
            _ => None,
        }
    }

    /// 是否满足识别上限；尚未恢复时为 `None`。
    pub fn recognized_within_budget(&self) -> Option<bool> {
        self.recovery_latency_ms()
            .map(|latency| latency <= NET_RECOVERY_RECOGNIZE_MS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_result_decides_state() {
        let mut probe = NetProbe::new();
        assert_eq!(probe.state(), NetworkState::Unknown);

        assert!(probe.on_result(true, 1_000));
        assert_eq!(probe.state(), NetworkState::Online);

        let mut probe = NetProbe::new();
        assert!(probe.on_result(false, 1_000));
        assert_eq!(probe.state(), NetworkState::Offline);
    }

    #[test]
    fn recovery_latency_is_measured_from_last_failure() {
        let mut probe = NetProbe::with_interval(1_000);
        probe.on_result(false, 1_000);
        probe.on_result(false, 2_000);
        assert!(probe.on_result(true, 2_500));
        assert_eq!(probe.state(), NetworkState::Online);
        assert_eq!(probe.recovery_latency_ms(), Some(500));
        assert_eq!(probe.recognized_within_budget(), Some(true));
    }

    #[test]
    fn probe_interval_is_clamped_within_budget() {
        assert_eq!(NetProbe::with_interval(0).poll_interval_ms(), 1);
        assert_eq!(
            NetProbe::with_interval(10_000).poll_interval_ms(),
            NET_RECOVERY_RECOGNIZE_MS
        );
        assert_eq!(NetProbe::new().poll_interval_ms(), NET_PROBE_INTERVAL_MS);
    }

    #[test]
    fn should_probe_only_when_offline_and_due() {
        let mut probe = NetProbe::with_interval(1_000);
        assert!(!probe.should_probe(0));

        probe.on_result(false, 1_000);
        assert!(!probe.should_probe(1_000));
        assert!(probe.should_probe(2_000));

        probe.mark_probed(2_000);
        assert!(!probe.should_probe(2_500));
        assert!(probe.should_probe(3_000));

        probe.on_result(true, 3_100);
        assert!(!probe.should_probe(9_999));
    }

    #[test]
    fn repeated_success_does_not_reset_recovery_point() {
        let mut probe = NetProbe::with_interval(1_000);
        probe.on_result(false, 1_000);
        probe.on_result(true, 2_000);
        probe.on_result(true, 5_000);
        assert_eq!(probe.recovered_at_ms(), Some(2_000));
        assert_eq!(probe.recovery_latency_ms(), Some(1_000));
    }
}
