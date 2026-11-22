//! 门控模块门面：11 个子模块与唯一的 4 函数公开面。

/// B4 总开关；默认关闭，关闭态零副作用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GateFeature {
    #[default]
    Disabled,
    Enabled,
}

/// 审计追加失败；与"不可用"归一为 fail-closed。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AuditError;

/// 审计追加端口；失败与不可用统一 fail-closed。
pub trait AuditSink: Send + Sync {
    fn append(
        &self,
        event: &meeting_copilot_domain::model::gate::GateAuditEvent,
    ) -> Result<(), AuditError>;
}

pub mod api;
pub(crate) mod budget;
pub(crate) mod clock;
pub(crate) mod dedupe;
pub(crate) mod diagnostics;
pub mod input;
pub mod keyword;
pub(crate) mod lease;
pub(crate) mod outbound;
pub(crate) mod prefilter;
pub(crate) mod record;
pub(crate) mod relevance;
pub(crate) mod replay;

pub use api::{detect_gaps, evaluate_gate, record_from, score_relevance};
pub use budget::refill;
pub use clock::{FixedClock, GateClock};
pub use diagnostics::{evaluate_gate_with_counters, GateOutcome};
pub use input::{
    GapScanInput, GateInput, GateSignals, GateSnapshot, RelevanceInput, RelevanceSnapshot,
};
pub use lease::{LeaseId, LeaseOutcome, LeaseTable};
pub use outbound::{NoopAuditSink as NoopSink, PassthroughSanitizer};
pub use prefilter::detect_gaps_with_counters;
pub use relevance::decay_milli;
