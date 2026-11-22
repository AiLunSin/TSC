//! 记录结构校验与计数推进（P0 内部违规枚举，P3 升格为 `GateError`）。

use meeting_copilot_domain::model::gate::{GateCounter, GateCounters, GateDecision, GateRecord};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecordViolation {
    RefsEmpty,
    IllegalSkipStage,
}

#[allow(dead_code)]
pub(crate) fn validate_record(record: &GateRecord) -> Option<RecordViolation> {
    if record
        .unknown
        .get("gate_record_violation")
        .and_then(|value| value.as_str())
        == Some("refs_empty")
    {
        return Some(RecordViolation::RefsEmpty);
    }
    if !record.refs_are_valid() {
        return Some(RecordViolation::RefsEmpty);
    }
    if !record.skipped_stages_are_valid() {
        return Some(RecordViolation::IllegalSkipStage);
    }
    None
}

/// `Allow`/`Deny` 总量互斥；分项只在原因码的产生级 +1。
#[allow(dead_code)]
pub(crate) fn bump_by(counters: &mut GateCounters, decision: &GateDecision) {
    match decision {
        GateDecision::Allow { .. } => counters.bump(GateCounter::GateAllowTotal),
        GateDecision::Deny {
            reason, stopped_at, ..
        } => {
            counters.bump(GateCounter::GateDenyTotal);
            if let Some(counter) = reason.counter_at(*stopped_at) {
                counters.bump(counter);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use meeting_copilot_domain::model::card::DedupeKey;
    use meeting_copilot_domain::model::enums::TriggerKind;
    use meeting_copilot_domain::model::gate::{
        AuthorizationStatus, DecisionKind, GateRejectReason, GateStage, PerStageResults,
    };
    use meeting_copilot_domain::vo::{SessionId, TraceId};

    fn sample_record() -> GateRecord {
        GateRecord {
            schema_version: 1,
            trace_id: TraceId::new("trace-1").expect("trace"),
            session_id: SessionId::new("session-1").expect("session"),
            candidate_ref: Some(DedupeKey::new("b".repeat(32)).expect("dedupe")),
            insight_ref: None,
            trigger: TriggerKind::Auto,
            per_stage: PerStageResults::empty(),
            decision: DecisionKind::Deny,
            stopped_at: Some(GateStage::G1),
            budget_remaining: 120,
            cooldown_until_ms: None,
            authorization_status: AuthorizationStatus::Missing,
            skipped_stages: Vec::new(),
            decided_at_ms: 0,
            unknown: BTreeMap::new(),
        }
    }

    fn deny(reason: GateRejectReason, stopped_at: GateStage) -> GateDecision {
        GateDecision::Deny {
            reason,
            stopped_at,
            per_stage: PerStageResults::empty(),
            cooldown_until_ms: None,
        }
    }

    fn allow() -> GateDecision {
        GateDecision::Allow {
            per_stage: PerStageResults::empty(),
            budget_remaining: 120,
            candidates: Vec::new(),
            outbound: None,
            suppressed_notice: false,
            stopped_at: None,
        }
    }

    #[test]
    fn record_rejects_empty_refs() {
        let mut record = sample_record();
        record.candidate_ref = None;
        record.insight_ref = None;
        assert_eq!(validate_record(&record), Some(RecordViolation::RefsEmpty));
        record.candidate_ref = Some(DedupeKey::new("c".repeat(32)).expect("dedupe"));
        assert_eq!(validate_record(&record), None);
    }

    #[test]
    fn record_rejects_illegal_skip_stage() {
        let mut record = sample_record();
        record.skipped_stages = vec![GateStage::G4];
        assert_eq!(
            validate_record(&record),
            Some(RecordViolation::IllegalSkipStage)
        );
        record.skipped_stages = vec![GateStage::G2, GateStage::G3];
        assert_eq!(validate_record(&record), None);
    }

    #[test]
    fn bump_by_reason_matches_static_table() {
        let mut counters = GateCounters::default();
        bump_by(
            &mut counters,
            &deny(GateRejectReason::Duplicate, GateStage::G3),
        );
        assert_eq!(counters.get(GateCounter::GateDenyTotal), 1);
        assert_eq!(counters.get(GateCounter::GateAllowTotal), 0);
        assert_eq!(counters.get(GateCounter::GateL3Duplicate), 1);
        assert_eq!(counters.get(GateCounter::GateL1Duplicate), 0);

        let mut counters = GateCounters::default();
        bump_by(
            &mut counters,
            &deny(GateRejectReason::PolicyUnavailable, GateStage::G1),
        );
        assert_eq!(counters.get(GateCounter::GateDenyTotal), 1);
        let non_total = counters
            .snapshot()
            .into_iter()
            .filter(|(counter, value)| {
                *value != 0
                    && *counter != GateCounter::GateDenyTotal
                    && *counter != GateCounter::GateAllowTotal
            })
            .count();
        assert_eq!(non_total, 0);
    }

    #[test]
    fn allow_and_deny_are_mutually_exclusive() {
        let mut counters = GateCounters::default();
        bump_by(&mut counters, &allow());
        assert_eq!(counters.get(GateCounter::GateAllowTotal), 1);
        assert_eq!(counters.get(GateCounter::GateDenyTotal), 0);

        bump_by(&mut counters, &deny(GateRejectReason::Stale, GateStage::G3));
        assert_eq!(counters.get(GateCounter::GateAllowTotal), 1);
        assert_eq!(counters.get(GateCounter::GateDenyTotal), 1);
        assert_eq!(counters.get(GateCounter::GateL3Stale), 1);
    }
}
