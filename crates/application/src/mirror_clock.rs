//! 生产镜像时钟：由装配方推进，门控只读；不读系统时间。

use std::sync::atomic::{AtomicU64, Ordering};

use crate::gate::GateClock;

/// 外部推进的毫秒镜像；`advance_to` 只增不减。
#[derive(Debug, Default)]
pub struct MirrorClock {
    now_ms: AtomicU64,
}

impl MirrorClock {
    pub fn new(anchor_ms: u64) -> Self {
        Self {
            now_ms: AtomicU64::new(anchor_ms),
        }
    }

    /// 推进到 `ms`；小于当前值时保持不变，返回推进后的值。
    pub fn advance_to(&self, ms: u64) -> u64 {
        self.now_ms.fetch_max(ms, Ordering::SeqCst).max(ms)
    }

    pub fn snapshot(&self) -> u64 {
        self.now_ms.load(Ordering::SeqCst)
    }
}

impl GateClock for MirrorClock {
    fn now_ms(&self) -> u64 {
        self.snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirror_clock_is_monotonic() {
        let clock = MirrorClock::new(100);
        assert_eq!(clock.now_ms(), 100);
        assert_eq!(clock.advance_to(150), 150);
        assert_eq!(clock.advance_to(120), 150);
        assert_eq!(clock.now_ms(), 150);
    }
}
