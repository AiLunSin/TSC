//! 门控时钟端口与回放时钟；P0 不读系统时间。

use std::cell::Cell;

/// 时钟端口：只暴露毫秒，不读 OS 计时。
pub trait GateClock: Send + Sync {
    fn now_ms(&self) -> u64;
}

/// 回放时钟：第 n 次取值 = `anchor_ms + n * tick_ms`。
#[derive(Debug, Default)]
pub struct FixedClock {
    anchor_ms: u64,
    tick_ms: u64,
    calls: Cell<u64>,
}

impl FixedClock {
    pub fn new(anchor_ms: u64, tick_ms: u64) -> Self {
        Self {
            anchor_ms,
            tick_ms,
            calls: Cell::new(0),
        }
    }

    pub fn now_ms(&self) -> u64 {
        let index = self.calls.get();
        self.calls.set(index.saturating_add(1));
        self.anchor_ms
            .saturating_add(index.saturating_mul(self.tick_ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_clock_is_deterministic_sequence() {
        let clock = FixedClock::new(1_000, 250);
        let sequence: Vec<u64> = (0..5).map(|_| clock.now_ms()).collect();
        assert_eq!(sequence, vec![1_000, 1_250, 1_500, 1_750, 2_000]);
    }
}
