//! G3 时效/去重：聚类、去重键、配额、冷却、抑制、缓存、收敛（只读预算快照）。

use meeting_copilot_domain::model::enums::TriggerKind;
use meeting_copilot_domain::model::gate::{
    ClusterSource, EntityKey, GatePolicy, GateRejectReason, ScoredCandidate,
};

use crate::gate::input::GateInput;

pub(crate) struct DedupeOutcome {
    pub deny: Option<(GateRejectReason, Option<u64>)>,
    pub selected: Vec<ScoredCandidate>,
    pub suppressed_notice: bool,
    pub dropped: u64,
    pub skipped: bool,
}

const CLUSTER_THRESHOLD_MILLI: u16 = 880;
const DUPLICATE_WINDOW_MS: u64 = 600_000;
const TOPIC_QUOTA: u8 = 2;
const GLOBAL_QUOTA: u8 = 3;
const ENTITY_COOLDOWN_MS: u64 = 300_000;
const CONVERGENCE_LIMIT: usize = 10;
const CONVERGENCE_KEEP: usize = 3;

pub(crate) fn apply(
    input: &GateInput<'_>,
    policy: &GatePolicy,
    mut candidates: Vec<ScoredCandidate>,
) -> DedupeOutcome {
    let cluster_id = match input.signals.similarity_milli {
        Some(similarity) if similarity >= CLUSTER_THRESHOLD_MILLI => input.signals.cluster_id,
        Some(_) => input.signals.cluster_id.saturating_add(1),
        None => input.signals.cluster_id,
    };
    let source = if input.signals.similarity_milli.is_some() {
        ClusterSource::Injected
    } else {
        ClusterSource::Fallback
    };
    for candidate in &mut candidates {
        candidate.topic_cluster_id = cluster_id;
        candidate.topic_cluster_source = source;
    }

    if !policy.dedupe_enabled {
        let outcome = suppression_and_cache(input, policy, candidates, cluster_id);
        return DedupeOutcome {
            skipped: outcome.deny.is_none(),
            ..outcome
        };
    }

    if let Some(last) = input.signals.duplicate_last_seen_ms {
        if input.now_ms.saturating_sub(last) < DUPLICATE_WINDOW_MS {
            return deny(GateRejectReason::Duplicate, None, candidates);
        }
    }

    if input
        .budget
        .topic_window_used
        .get(&cluster_id)
        .copied()
        .unwrap_or(0)
        >= TOPIC_QUOTA
    {
        return deny(GateRejectReason::QuotaExhausted, None, candidates);
    }
    if input.budget.global_window_used >= GLOBAL_QUOTA {
        return deny(GateRejectReason::QuotaExhausted, None, candidates);
    }

    let mut cooldown_until = None;
    for key in input.signals.entity_keys {
        if let Some(entity) = EntityKey::parse(key) {
            if let Some(last) = input.budget.entity_last_seen_ms.get(&entity) {
                if input.now_ms.saturating_sub(*last) < ENTITY_COOLDOWN_MS {
                    cooldown_until = Some(last.saturating_add(ENTITY_COOLDOWN_MS));
                }
            }
        }
    }
    if cooldown_until.is_some() {
        return deny(GateRejectReason::Duplicate, cooldown_until, candidates);
    }

    let mut outcome = suppression_and_cache(input, policy, candidates, cluster_id);
    if outcome.deny.is_some() {
        return outcome;
    }

    if outcome.selected.len() > CONVERGENCE_LIMIT {
        outcome.selected.sort_by(|left, right| {
            right
                .relevance
                .score_milli
                .cmp(&left.relevance.score_milli)
                .then(
                    left.candidate
                        .dedupe_key
                        .as_str()
                        .cmp(right.candidate.dedupe_key.as_str()),
                )
        });
        outcome.dropped = u64::try_from(outcome.selected.len() - CONVERGENCE_KEEP).unwrap_or(0);
        outcome.selected.truncate(CONVERGENCE_KEEP);
    }
    outcome
}

fn suppression_and_cache(
    input: &GateInput<'_>,
    policy: &GatePolicy,
    mut candidates: Vec<ScoredCandidate>,
    cluster_id: u32,
) -> DedupeOutcome {
    let suppressed = input.signals.suppression_topics.contains(&cluster_id)
        || !input.signals.suppression_providers.is_empty()
        || input
            .signals
            .entity_keys
            .iter()
            .any(|key| input.signals.suppression_entities.contains(key));
    let mut suppressed_notice = false;
    if suppressed {
        if input.trigger == TriggerKind::Auto {
            return deny(GateRejectReason::Suppressed, None, candidates);
        }
        suppressed_notice = true;
    }

    if input.signals.cache_age_ms.unwrap_or(0) > policy.max_cache_age_ms
        && !input.signals.refresh_available
    {
        return deny(GateRejectReason::Stale, None, candidates);
    }

    DedupeOutcome {
        deny: None,
        selected: std::mem::take(&mut candidates),
        suppressed_notice,
        dropped: 0,
        skipped: false,
    }
}

fn deny(
    reason: GateRejectReason,
    cooldown_until_ms: Option<u64>,
    selected: Vec<ScoredCandidate>,
) -> DedupeOutcome {
    DedupeOutcome {
        deny: Some((reason, cooldown_until_ms)),
        selected,
        suppressed_notice: false,
        dropped: 0,
        skipped: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_match_spec() {
        assert_eq!(CLUSTER_THRESHOLD_MILLI, 880);
        assert_eq!(DUPLICATE_WINDOW_MS, 600_000);
        assert_eq!(ENTITY_COOLDOWN_MS, 300_000);
    }
}
