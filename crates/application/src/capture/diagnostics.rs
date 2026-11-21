//! 采集诊断计数（AC-B2-OC-C01/C06/C08/D04/D05/E01）。
//!
//! 只接收封闭枚举与增量；快照仅含数值，不含文本/样本/凭据。

use std::sync::Mutex;

/// 13 项封闭计数（禁止字符串事件名）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticCounter {
    ShortGapDropped,
    GapOpened,
    GapPersisted,
    GapReopened,
    InsightCheckSkipped,
    ClockRegression,
    BackfillBlocked,
    AvailabilityPushed,
    StatusNotPushed,
    SubscriberFailure,
    QueueDropped,
    SeqOverflow,
    ObservationRejected,
}

impl DiagnosticCounter {
    pub const ALL: [Self; 13] = [
        Self::ShortGapDropped,
        Self::GapOpened,
        Self::GapPersisted,
        Self::GapReopened,
        Self::InsightCheckSkipped,
        Self::ClockRegression,
        Self::BackfillBlocked,
        Self::AvailabilityPushed,
        Self::StatusNotPushed,
        Self::SubscriberFailure,
        Self::QueueDropped,
        Self::SeqOverflow,
        Self::ObservationRejected,
    ];

    pub const fn index(self) -> usize {
        match self {
            Self::ShortGapDropped => 0,
            Self::GapOpened => 1,
            Self::GapPersisted => 2,
            Self::GapReopened => 3,
            Self::InsightCheckSkipped => 4,
            Self::ClockRegression => 5,
            Self::BackfillBlocked => 6,
            Self::AvailabilityPushed => 7,
            Self::StatusNotPushed => 8,
            Self::SubscriberFailure => 9,
            Self::QueueDropped => 10,
            Self::SeqOverflow => 11,
            Self::ObservationRejected => 12,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::ShortGapDropped => "short_gap_dropped",
            Self::GapOpened => "gap_opened",
            Self::GapPersisted => "gap_persisted",
            Self::GapReopened => "gap_reopened",
            Self::InsightCheckSkipped => "insight_check_skipped",
            Self::ClockRegression => "clock_regression",
            Self::BackfillBlocked => "backfill_blocked",
            Self::AvailabilityPushed => "availability_pushed",
            Self::StatusNotPushed => "status_not_pushed",
            Self::SubscriberFailure => "subscriber_failure",
            Self::QueueDropped => "queue_dropped",
            Self::SeqOverflow => "seq_overflow",
            Self::ObservationRejected => "observation_rejected",
        }
    }
}

/// 计数值快照：唯一字段为 13 项数值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DiagnosticsSnapshot {
    counts: [u64; DiagnosticCounter::ALL.len()],
}

impl DiagnosticsSnapshot {
    pub const ZERO: Self = Self {
        counts: [0; DiagnosticCounter::ALL.len()],
    };

    pub fn get(&self, counter: DiagnosticCounter) -> u64 {
        self.counts[counter.index()]
    }

    pub const fn counts(&self) -> &[u64; DiagnosticCounter::ALL.len()] {
        &self.counts
    }
}

/// 生产实现：只累加，不携带任何载荷字段。
#[derive(Debug, Default)]
pub struct CountingDiagnostics {
    counts: Mutex<[u64; DiagnosticCounter::ALL.len()]>,
}

impl CountingDiagnostics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bump(&self, counter: DiagnosticCounter, delta: u64) {
        let mut counts = self
            .counts
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        counts[counter.index()] = counts[counter.index()].saturating_add(delta);
    }

    pub fn snapshot(&self) -> DiagnosticsSnapshot {
        let counts = self
            .counts
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        DiagnosticsSnapshot { counts: *counts }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC-B2-OC-C01/C06：按枚举项独立累加。
    #[test]
    fn diagnostics_bump_accumulates_per_counter() {
        let diagnostics = CountingDiagnostics::new();
        diagnostics.bump(DiagnosticCounter::ShortGapDropped, 1);
        diagnostics.bump(DiagnosticCounter::ShortGapDropped, 2);
        diagnostics.bump(DiagnosticCounter::GapOpened, 5);
        let snapshot = diagnostics.snapshot();
        assert_eq!(snapshot.get(DiagnosticCounter::ShortGapDropped), 3);
        assert_eq!(snapshot.get(DiagnosticCounter::GapOpened), 5);
        assert_eq!(snapshot.get(DiagnosticCounter::GapPersisted), 0);
        assert_eq!(DiagnosticCounter::ALL.len(), 13);
    }

    /// AC-B2-OC-D05/E01：快照只含数值，不含文本/样本载荷。
    #[test]
    fn diagnostics_snapshot_has_no_payload_fields() {
        let diagnostics = CountingDiagnostics::new();
        diagnostics.bump(DiagnosticCounter::QueueDropped, 7);
        let rendered = format!("{:?}", diagnostics.snapshot());
        for forbidden in ["text", "sample", "speaker", "participant", "caption"] {
            assert!(!rendered.contains(forbidden), "快照不应含 {forbidden}");
        }
        assert!(rendered.contains("counts"));
    }
}
