#![allow(dead_code)]

//! 应用层测试替身：可手动推进的时钟。禁止读取系统时间。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use meeting_copilot_domain::port::CaptureClock;
use meeting_copilot_domain::vo::{ClockError, MonoMillis, WallClock};

pub struct ManualClock {
    mono_ms: Mutex<u64>,
    wall_base: WallClock,
    wall_offset_ms: Mutex<u64>,
    fail_mono: AtomicBool,
}

impl ManualClock {
    pub fn new(wall_base: WallClock) -> Self {
        Self {
            mono_ms: Mutex::new(0),
            wall_base,
            wall_offset_ms: Mutex::new(0),
            fail_mono: AtomicBool::new(false),
        }
    }

    pub fn advance_by(&self, delta_ms: u64) {
        let mut mono = self
            .mono_ms
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        *mono = mono.saturating_add(delta_ms);
        let mut wall = self
            .wall_offset_ms
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        *wall = wall.saturating_add(delta_ms);
    }

    pub fn fail_mono(&self) {
        self.fail_mono.store(true, Ordering::SeqCst);
    }

    pub fn recover_mono(&self) {
        self.fail_mono.store(false, Ordering::SeqCst);
    }
}

impl CaptureClock for ManualClock {
    fn mono_now(&self) -> Result<MonoMillis, ClockError> {
        if self.fail_mono.load(Ordering::SeqCst) {
            return Err(ClockError::Overflow);
        }
        let mono = *self
            .mono_ms
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        Ok(MonoMillis::from_millis(mono))
    }

    fn wall_now(&self) -> Result<WallClock, ClockError> {
        let offset = *self
            .wall_offset_ms
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let offset = i64::try_from(offset).map_err(|_| ClockError::Overflow)?;
        Ok(WallClock::from_unix_millis(
            self.wall_base.as_unix_millis().saturating_add(offset),
        ))
    }
}
