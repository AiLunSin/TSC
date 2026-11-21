//! 采集通道编排（B2.23 + B2 离线闭环）。
//!
//! 本模块只承载离线可判定的 DTO 与阈值配置。

pub mod capture_budget;
pub mod capture_gap;
pub mod capture_gap_gate;
pub mod capture_ports;
pub mod capture_resume;
pub mod channel_arbiter;
pub mod channel_state;
pub mod dedupe_guard;
pub mod degraded_guard;
pub mod diagnostics;
pub mod merge_gate;
pub mod merge_types;
pub mod service;
pub mod status_publisher;
pub mod subscriptions;
pub mod switch_log;
pub mod types;

pub use capture_budget::{AsrBudget, JitterSuppressor};
pub use capture_gap::merge_gaps;
pub use capture_gap_gate::{
    merge_for_persist, persist_gate, resume_boundary, validate_intervals, GateDecision,
    IntervalViolation,
};
pub use capture_ports::{
    AsrDeliverySink, CapturePortError, ChannelGapLog, ChannelStatusSink, DiagnosticsSink,
    InsightInvariantPort, InsightReadError, NoopAsrDelivery, NoopDiagnostics, NoopGapLog,
    NoopInsight, NoopStatusSink,
};
pub use capture_resume::CaptureResumeLedger;
pub use channel_arbiter::{decide, stable_switch_back_due, subtitle_fallback_due, Decision};
pub use channel_state::{ChannelInstance, ChannelStateRegistry};
pub use degraded_guard::{DegradedGuard, GapCloseOutcome, GuardError, GuardTransition};
pub use diagnostics::{CountingDiagnostics, DiagnosticCounter, DiagnosticsSnapshot};
pub use merge_gate::{validate_merged_set, MergeSetViolation, MergeSpan};
pub use service::CaptureChannelService;
pub use status_publisher::{
    LatencyStats, NotPushedReason, PublishOutcome, PublishRejectReason, StatusPublisher,
};
pub use subscriptions::{DeliveryQueue, DrainReport, StatusHandler, SubscriptionHandle};
pub use switch_log::{merge_same_millisecond, ChannelSwitchLog, SwitchLogError, SwitchTracker};
pub use types::*;
