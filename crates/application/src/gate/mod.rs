//! 门控模块门面：11 个子模块与唯一的 4 函数公开面。

pub mod api;
pub(crate) mod budget;
pub(crate) mod clock;
pub(crate) mod dedupe;
pub(crate) mod diagnostics;
pub mod input;
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
pub use prefilter::detect_gaps_with_counters;
pub use relevance::decay_milli;
