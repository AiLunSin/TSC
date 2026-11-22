//! 门控唯一公开面：4 个纯函数签名在此冻结，判定细节留在子模块。

use std::collections::BTreeMap;

use meeting_copilot_domain::model::card::GapCandidate;
use meeting_copilot_domain::model::gate::{
    DecisionKind, GateBudget, GateCounters, GateDecision, GatePolicy, GateRecord, RelevanceScore,
};

use crate::gate::clock::GateClock;
use crate::gate::input::{GapScanInput, GateInput, RelevanceInput};
use crate::gate::{diagnostics, prefilter, relevance};

pub fn detect_gaps(input: &GapScanInput<'_>) -> Vec<GapCandidate> {
    let mut counters = GateCounters::default();
    prefilter::detect_gaps_with_counters(input, &mut counters)
}

pub fn evaluate_gate(
    input: &GateInput<'_>,
    policy: &GatePolicy,
    clock: &dyn GateClock,
) -> GateDecision {
    diagnostics::evaluate_gate_with_counters(input, policy, clock).decision
}

pub fn score_relevance(input: &RelevanceInput<'_>, policy: &GatePolicy) -> RelevanceScore {
    relevance::compute(input, policy)
}

pub fn record_from(
    input: &GateInput<'_>,
    decision: &GateDecision,
    budget: &GateBudget,
) -> GateRecord {
    let (per_stage, decision_kind, stopped_at, cooldown_until_ms, fallback_ref) = match decision {
        GateDecision::Allow {
            per_stage,
            candidates,
            stopped_at,
            ..
        } => (
            per_stage.clone(),
            DecisionKind::Allow,
            *stopped_at,
            None,
            candidates
                .first()
                .map(|scored| scored.candidate.dedupe_key.clone()),
        ),
        GateDecision::Deny {
            per_stage,
            stopped_at,
            cooldown_until_ms,
            ..
        } => (
            per_stage.clone(),
            DecisionKind::Deny,
            Some(*stopped_at),
            *cooldown_until_ms,
            None,
        ),
    };

    let candidate_ref = input.candidate_ref.cloned().or(fallback_ref);
    let insight_ref = input.insight_ref.cloned();
    let mut unknown = BTreeMap::new();
    if candidate_ref.is_none() && insight_ref.is_none() {
        unknown.insert("gate_record_violation".to_owned(), "refs_empty".into());
    }

    GateRecord {
        schema_version: 1,
        trace_id: input.trace_id.clone(),
        session_id: input.session.clone(),
        candidate_ref,
        insight_ref,
        trigger: input.trigger,
        per_stage,
        decision: decision_kind,
        stopped_at,
        budget_remaining: budget.session_remaining_with(600).min(600),
        cooldown_until_ms,
        authorization_status: input.authorization.status,
        skipped_stages: Vec::new(),
        decided_at_ms: input.now_ms,
        unknown,
    }
}
