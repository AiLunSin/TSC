//! 采集通道编排（B2.23）。
//!
//! 本模块只承载离线可判定的 DTO 与阈值配置；通道实例/仲裁/推送将在后续任务补齐。

pub mod capture_budget;
pub mod capture_ports;
pub mod channel_arbiter;
pub mod channel_state;
pub mod service;
pub mod switch_log;
pub mod types;

pub use capture_budget::{AsrBudget, JitterSuppressor};
pub use capture_ports::{
    AsrDeliverySink, CapturePortError, ChannelGapLog, ChannelStatusSink, NoopAsrDelivery,
    NoopGapLog, NoopStatusSink,
};
pub use channel_arbiter::{decide, stable_switch_back_due, subtitle_fallback_due, Decision};
pub use channel_state::{ChannelInstance, ChannelStateRegistry};
pub use service::CaptureChannelService;
pub use switch_log::{merge_same_millisecond, ChannelSwitchLog, SwitchLogError, SwitchTracker};
pub use types::*;
