//! 可注入时钟端口（DESIGN 2.2.2.2、D2/D3）。
//!
//! 阈值判定只用 `MonoMillis`；墙钟用于展示/审计，不参与超时比较。

use crate::vo::{ClockError, MonoMillis, WallClock};

pub trait CaptureClock: Send + Sync {
    fn mono_now(&self) -> Result<MonoMillis, ClockError>;
    fn wall_now(&self) -> Result<WallClock, ClockError>;
}
