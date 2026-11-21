//! 理解层计数：28 项单调累加快照与派生值。

use std::sync::Mutex;

/// 28 项计数快照（`Copy`，只读返回）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CounterSnapshot {
    pub admitted: u64,
    pub speculative_skip: u64,
    pub skipped_low_confidence: u64,
    pub silence_dropped: u64,
    pub silence_partial_overlap: u64,
    pub paused_drop: u64,
    pub source_platform_mismatch: u64,
    pub duplicate_delivery_ignored: u64,
    pub empty_chunk_dropped: u64,
    pub split_grapheme_boundary: u64,
    pub dispatch_candidates: u64,
    pub prefilter_miss: u64,
    pub gateway_calls: u64,
    pub gateway_cooling: u64,
    pub rule_fallback_count: u64,
    pub schema_violation: u64,
    pub extract_empty: u64,
    pub duplicate_text_draft: u64,
    pub oversize_draft_blocked: u64,
    pub summary_refresh_ok: u64,
    pub summary_timeout: u64,
    pub summary_failed_gateway_unavailable: u64,
    pub summary_publish_blocked: u64,
    pub summary_out_of_order: u64,
    pub source_missing: u64,
    pub cross_session_ref: u64,
    pub short_text_merge_skipped: u64,
    pub split_span_degraded: u64,
}

impl CounterSnapshot {
    /// `rule_fallback_count / max(gateway_calls + rule_fallback_count, 1)`。
    pub fn rule_fallback_rate(&self) -> f64 {
        let numerator = u32::try_from(self.rule_fallback_count).unwrap_or(u32::MAX) as f64;
        let denominator = u32::try_from(self.gateway_calls.saturating_add(self.rule_fallback_count))
            .unwrap_or(u32::MAX)
            .max(1) as f64;
        numerator / denominator
    }
}

/// 封闭计数键：禁止字符串键。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Counter {
    Admitted,
    SpeculativeSkip,
    SkippedLowConfidence,
    SilenceDropped,
    SilencePartialOverlap,
    PausedDrop,
    SourcePlatformMismatch,
    DuplicateDeliveryIgnored,
    EmptyChunkDropped,
    SplitGraphemeBoundary,
    DispatchCandidates,
    PrefilterMiss,
    GatewayCalls,
    GatewayCooling,
    RuleFallbackCount,
    SchemaViolation,
    ExtractEmpty,
    DuplicateTextDraft,
    OversizeDraftBlocked,
    SummaryRefreshOk,
    SummaryTimeout,
    SummaryFailedGatewayUnavailable,
    SummaryPublishBlocked,
    SummaryOutOfOrder,
    SourceMissing,
    CrossSessionRef,
    ShortTextMergeSkipped,
    SplitSpanDegraded,
}

/// 唯一写入入口；`bump` 饱和累加。
#[derive(Debug, Default)]
pub struct UnderstandingCounters {
    inner: Mutex<CounterSnapshot>,
}

impl UnderstandingCounters {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bump(&self, counter: Counter) {
        let mut snapshot = self
            .inner
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let slot = match counter {
            Counter::Admitted => &mut snapshot.admitted,
            Counter::SpeculativeSkip => &mut snapshot.speculative_skip,
            Counter::SkippedLowConfidence => &mut snapshot.skipped_low_confidence,
            Counter::SilenceDropped => &mut snapshot.silence_dropped,
            Counter::SilencePartialOverlap => &mut snapshot.silence_partial_overlap,
            Counter::PausedDrop => &mut snapshot.paused_drop,
            Counter::SourcePlatformMismatch => &mut snapshot.source_platform_mismatch,
            Counter::DuplicateDeliveryIgnored => &mut snapshot.duplicate_delivery_ignored,
            Counter::EmptyChunkDropped => &mut snapshot.empty_chunk_dropped,
            Counter::SplitGraphemeBoundary => &mut snapshot.split_grapheme_boundary,
            Counter::DispatchCandidates => &mut snapshot.dispatch_candidates,
            Counter::PrefilterMiss => &mut snapshot.prefilter_miss,
            Counter::GatewayCalls => &mut snapshot.gateway_calls,
            Counter::GatewayCooling => &mut snapshot.gateway_cooling,
            Counter::RuleFallbackCount => &mut snapshot.rule_fallback_count,
            Counter::SchemaViolation => &mut snapshot.schema_violation,
            Counter::ExtractEmpty => &mut snapshot.extract_empty,
            Counter::DuplicateTextDraft => &mut snapshot.duplicate_text_draft,
            Counter::OversizeDraftBlocked => &mut snapshot.oversize_draft_blocked,
            Counter::SummaryRefreshOk => &mut snapshot.summary_refresh_ok,
            Counter::SummaryTimeout => &mut snapshot.summary_timeout,
            Counter::SummaryFailedGatewayUnavailable => {
                &mut snapshot.summary_failed_gateway_unavailable
            }
            Counter::SummaryPublishBlocked => &mut snapshot.summary_publish_blocked,
            Counter::SummaryOutOfOrder => &mut snapshot.summary_out_of_order,
            Counter::SourceMissing => &mut snapshot.source_missing,
            Counter::CrossSessionRef => &mut snapshot.cross_session_ref,
            Counter::ShortTextMergeSkipped => &mut snapshot.short_text_merge_skipped,
            Counter::SplitSpanDegraded => &mut snapshot.split_span_degraded,
        };
        *slot = slot.saturating_add(1);
    }

    pub fn snapshot(&self) -> CounterSnapshot {
        *self
            .inner
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_are_monotonic_and_copyable() {
        let counters = UnderstandingCounters::new();
        counters.bump(Counter::Admitted);
        counters.bump(Counter::Admitted);
        let first = counters.snapshot();
        assert_eq!(first.admitted, 2);
        let second = first;
        assert_eq!(second, first);
        counters.bump(Counter::SplitSpanDegraded);
        assert_eq!(counters.snapshot().split_span_degraded, 1);
    }

    #[test]
    fn rule_fallback_rate_formula() {
        let counters = UnderstandingCounters::new();
        for _ in 0..3 {
            counters.bump(Counter::GatewayCalls);
        }
        counters.bump(Counter::RuleFallbackCount);
        let rate = counters.snapshot().rule_fallback_rate();
        assert!((rate - 0.25).abs() < 0.001);

        let empty = CounterSnapshot::default();
        assert_eq!(empty.rule_fallback_rate(), 0.0);
    }
}
