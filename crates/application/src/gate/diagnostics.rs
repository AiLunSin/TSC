//! 诊断入口：五级编排 + 32 项计数快照；4 个公开纯函数仍在 `api.rs`。

use meeting_copilot_domain::model::gate::{
    ClusterSource, EntityKey, GapRule, GateContentType, GateCounter, GateCounters, GateDecision,
    GatePolicy, GateRejectReason, GateStage, OutboundIntent, PerStageResults, ScoredCandidate,
    StageResult, StageTrace,
};

use crate::gate::clock::GateClock;
use crate::gate::input::{GateInput, RelevanceInput};
use crate::gate::{budget, dedupe, prefilter, relevance};

/// 判定 + 32 项计数快照（诊断入口，不进入回放面）。
#[derive(Debug, Clone)]
pub struct GateOutcome {
    pub decision: GateDecision,
    pub counters: GateCounters,
}

pub fn evaluate_gate_with_counters(
    input: &GateInput<'_>,
    policy: &GatePolicy,
    clock: &dyn GateClock,
) -> GateOutcome {
    let mut counters = GateCounters::default();
    if policy.validate().is_err() {
        counters.bump(GateCounter::GateDenyTotal);
        return GateOutcome {
            decision: GateDecision::Deny {
                reason: GateRejectReason::PolicyUnavailable,
                stopped_at: GateStage::G1,
                per_stage: PerStageResults::empty(),
                cooldown_until_ms: None,
            },
            counters,
        };
    }

    let mut per_stage = PerStageResults::empty();

    let started = clock.now_ms();
    let g1 = prefilter::run(input, policy);
    let elapsed = elapsed_ms(started, clock.now_ms());
    counters = g1.counters;

    if let Some(reason) = g1.deny {
        per_stage.0[0] = trace(StageResult::Deny, Some(reason), elapsed);
        counters.bump(GateCounter::GateDenyTotal);
        return deny_outcome(reason, GateStage::G1, None, per_stage, counters);
    }
    if g1.candidates.is_empty() {
        per_stage.0[0] = trace(StageResult::NoCandidate, None, elapsed);
        counters.bump(GateCounter::GateAllowTotal);
        return GateOutcome {
            decision: allow(
                per_stage,
                input
                    .budget
                    .session_remaining_with(policy.session_outbound_cap),
                Vec::new(),
                None,
                false,
                Some(GateStage::G1),
            ),
            counters,
        };
    }
    per_stage.0[0] = trace(StageResult::Allow, None, elapsed);

    let threshold = policy.threshold_milli();
    let g2_started = clock.now_ms();
    let mut scored = Vec::new();
    for candidate in g1.candidates {
        let relevance_input = RelevanceInput {
            candidate: &candidate,
            semantic_milli: input.signals.relevance.semantic_milli,
            entity_milli: input.signals.relevance.entity_milli,
            gain_milli: input.signals.relevance.gain_milli,
            source_trust_milli: input.signals.relevance.source_trust_milli,
            delta_ms: input.signals.relevance.delta_ms,
            threshold_profile: policy.threshold_profile,
        };
        let score = relevance::compute(&relevance_input, policy);
        if score.score_milli < threshold {
            counters.bump(GateCounter::GateL2LowRelevance);
            counters.bump(GateCounter::GateDenyTotal);
            per_stage.0[1] = trace(
                StageResult::Deny,
                Some(GateRejectReason::LowRelevance),
                elapsed_ms(g2_started, clock.now_ms()),
            );
            return deny_outcome(
                GateRejectReason::LowRelevance,
                GateStage::G2,
                None,
                per_stage,
                counters,
            );
        }
        scored.push(ScoredCandidate {
            rule: rule_of(candidate.kind),
            topic_cluster_id: input.signals.cluster_id,
            entity_keys: parse_entities(input.signals.entity_keys),
            topic_cluster_source: if input.signals.similarity_milli.is_some() {
                ClusterSource::Injected
            } else {
                ClusterSource::Fallback
            },
            candidate,
            relevance: score,
        });
    }
    per_stage.0[1] = StageTrace {
        calls: u16::try_from(scored.len()).unwrap_or(u16::MAX),
        result: StageResult::Allow,
        reason: None,
        elapsed_ms: elapsed_ms(g2_started, clock.now_ms()),
    };

    let g3_started = clock.now_ms();
    let g3 = dedupe::apply(input, policy, scored);
    let g3_elapsed = elapsed_ms(g3_started, clock.now_ms());
    if let Some((reason, cooldown)) = g3.deny {
        if let Some(counter) = reason.counter_at(GateStage::G3) {
            counters.bump(counter);
        }
        counters.bump(GateCounter::GateDenyTotal);
        per_stage.0[2] = trace(StageResult::Deny, Some(reason), g3_elapsed);
        return deny_outcome(reason, GateStage::G3, cooldown, per_stage, counters);
    }
    per_stage.0[2] = trace(StageResult::Allow, None, g3_elapsed);
    if g3.skipped {
        counters.bump(GateCounter::GateStageSkippedTotal);
    }
    for _ in 0..g3.dropped {
        counters.bump(GateCounter::GateL3Quota);
    }

    let g4_started = clock.now_ms();
    let g4 = budget::check(input, policy);
    let g4_elapsed = elapsed_ms(g4_started, clock.now_ms());
    if let Some((reason, cooldown)) = g4.deny {
        if let Some(counter) = reason.counter_at(GateStage::G4) {
            counters.bump(counter);
        }
        counters.bump(GateCounter::GateDenyTotal);
        per_stage.0[3] = trace(StageResult::Deny, Some(reason), g4_elapsed);
        return deny_outcome(reason, GateStage::G4, cooldown, per_stage, counters);
    }
    per_stage.0[3] = trace(StageResult::Allow, None, g4_elapsed);

    counters.bump(GateCounter::GateAllowTotal);
    let outbound = if g4.queued {
        Some(OutboundIntent {
            sanitized_query_len: 0,
            content_type: GateContentType::TextQuery,
            provider_ids: input.provider.provider_ids.clone(),
            authorization_status: input.authorization.status,
            truncated: false,
            queued: true,
            cooldown_until_ms: None,
        })
    } else {
        None
    };
    GateOutcome {
        decision: allow(
            per_stage,
            g4.remaining,
            g3.selected,
            outbound,
            g3.suppressed_notice,
            None,
        ),
        counters,
    }
}

fn trace(result: StageResult, reason: Option<GateRejectReason>, elapsed_ms: u32) -> StageTrace {
    StageTrace {
        calls: 1,
        result,
        reason,
        elapsed_ms,
    }
}

fn elapsed_ms(started: u64, ended: u64) -> u32 {
    ended.saturating_sub(started).min(u64::from(u32::MAX)) as u32
}

fn deny_outcome(
    reason: GateRejectReason,
    stopped_at: GateStage,
    cooldown_until_ms: Option<u64>,
    per_stage: PerStageResults,
    counters: GateCounters,
) -> GateOutcome {
    GateOutcome {
        decision: GateDecision::Deny {
            reason,
            stopped_at,
            per_stage,
            cooldown_until_ms,
        },
        counters,
    }
}

fn allow(
    per_stage: PerStageResults,
    budget_remaining: u16,
    candidates: Vec<ScoredCandidate>,
    outbound: Option<OutboundIntent>,
    suppressed_notice: bool,
    stopped_at: Option<GateStage>,
) -> GateDecision {
    GateDecision::Allow {
        per_stage,
        budget_remaining,
        candidates,
        outbound,
        suppressed_notice,
        stopped_at,
    }
}

fn rule_of(kind: meeting_copilot_domain::model::enums::GapKind) -> GapRule {
    use meeting_copilot_domain::model::enums::GapKind;
    match kind {
        GapKind::ProperNoun => GapRule::ProperNoun,
        GapKind::Abbreviation => GapRule::Abbreviation,
        GapKind::PolicyStandard => GapRule::PolicyStandard,
        GapKind::Metric => GapRule::Metric,
        GapKind::UnansweredQuestion => GapRule::UnansweredQuestion,
        GapKind::Acronym => GapRule::Acronym,
    }
}

fn parse_entities(keys: &[String]) -> Vec<EntityKey> {
    keys.iter()
        .filter_map(|key| EntityKey::parse(key))
        .collect()
}
