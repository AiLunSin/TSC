//! G3 时效/去重（AC-B4-15~21）。

mod common;

use common::Harness;
use meeting_copilot_domain::model::enums::TriggerKind;
use meeting_copilot_domain::model::gate::GateDecision as Decision;
use meeting_copilot_domain::model::gate::{
    ClusterSource, EntityKey, GateCounter, GateDecision, GatePolicy, GateRejectReason, GateStage,
};

fn deny_of(
    outcome: &meeting_copilot_application::gate::GateOutcome,
) -> (GateRejectReason, GateStage) {
    match &outcome.decision {
        Decision::Deny {
            reason, stopped_at, ..
        } => (*reason, *stopped_at),
        other => panic!("expected deny, got {other:?}"),
    }
}

fn allow_of(outcome: &meeting_copilot_application::gate::GateOutcome) -> &GateDecision {
    match &outcome.decision {
        Decision::Allow { .. } => &outcome.decision,
        other => panic!("expected allow, got {other:?}"),
    }
}

fn ready_harness(similarity: Option<u16>) -> Harness {
    let mut harness = Harness::new(1_000_000);
    harness.lexicon.push("张伟".to_owned());
    harness.push("张伟负责该模块。", 0.9, 0, 100);
    harness.relevance.semantic_milli = 1000;
    harness.relevance.entity_milli = 1000;
    harness.relevance.gain_milli = 1000;
    harness.relevance.source_trust_milli = 1000;
    harness.similarity_milli = similarity;
    harness
}

#[test]
fn semantic_cluster_threshold_is_088() {
    let same = ready_harness(Some(880)).outcome();
    match allow_of(&same) {
        Decision::Allow { candidates, .. } => {
            assert_eq!(candidates[0].topic_cluster_id, 1);
            assert_eq!(candidates[0].topic_cluster_source, ClusterSource::Injected);
        }
        _ => unreachable!(),
    }
}

#[test]
fn cluster_boundary_08799_08800() {
    let below = ready_harness(Some(879)).outcome();
    match allow_of(&below) {
        Decision::Allow { candidates, .. } => {
            assert_eq!(candidates[0].topic_cluster_id, 2);
        }
        _ => unreachable!(),
    }
    let at = ready_harness(Some(880)).outcome();
    match allow_of(&at) {
        Decision::Allow { candidates, .. } => assert_eq!(candidates[0].topic_cluster_id, 1),
        _ => unreachable!(),
    }
}

#[test]
fn cluster_source_is_reported_for_fallback() {
    let outcome = ready_harness(None).outcome();
    match allow_of(&outcome) {
        Decision::Allow { candidates, .. } => {
            assert_eq!(candidates[0].topic_cluster_source, ClusterSource::Fallback);
        }
        _ => unreachable!(),
    }
}

#[test]
fn duplicate_key_blocks_within_ten_minutes() {
    let mut harness = ready_harness(None);
    harness.duplicate_last_seen_ms = Some(harness.now_ms - 599_999);
    let outcome = harness.outcome();
    assert_eq!(
        deny_of(&outcome),
        (GateRejectReason::Duplicate, GateStage::G3)
    );
    assert_eq!(outcome.counters.get(GateCounter::GateL3Duplicate), 1);
}

#[test]
fn duplicate_key_expires_at_ten_minutes() {
    let mut harness = ready_harness(None);
    harness.duplicate_last_seen_ms = Some(harness.now_ms - 600_000);
    assert!(matches!(
        harness.outcome().decision,
        GateDecision::Allow { .. }
    ));
}

#[test]
fn topic_quota_is_two_per_ten_minutes() {
    let mut harness = ready_harness(None);
    harness.budget.topic_window_used.insert(1, 2);
    let outcome = harness.outcome();
    assert_eq!(
        deny_of(&outcome),
        (GateRejectReason::QuotaExhausted, GateStage::G3)
    );
    assert_eq!(outcome.counters.get(GateCounter::GateL3Quota), 1);
}

#[test]
fn global_quota_is_three_per_ten_minutes() {
    let mut harness = ready_harness(None);
    harness.budget.global_window_used = 3;
    let outcome = harness.outcome();
    assert_eq!(
        deny_of(&outcome),
        (GateRejectReason::QuotaExhausted, GateStage::G3)
    );
    assert_eq!(outcome.counters.get(GateCounter::GateL3Quota), 1);
}

fn entity_key() -> String {
    "b".repeat(32)
}

#[test]
fn entity_cooldown_is_five_minutes() {
    let mut harness = ready_harness(None);
    let key = entity_key();
    harness.entity_keys.push(key.clone());
    harness.budget.entity_last_seen_ms.insert(
        EntityKey::parse(&key).expect("entity"),
        harness.now_ms - 299_999,
    );
    let outcome = harness.outcome();
    match outcome.decision {
        GateDecision::Deny {
            reason,
            stopped_at,
            cooldown_until_ms,
            ..
        } => {
            assert_eq!(reason, GateRejectReason::Duplicate);
            assert_eq!(stopped_at, GateStage::G3);
            assert_eq!(cooldown_until_ms, Some(harness.now_ms - 299_999 + 300_000));
        }
        other => panic!("expected deny, got {other:?}"),
    }
    assert_eq!(outcome.counters.get(GateCounter::GateL3Duplicate), 1);
}

#[test]
fn entity_cooldown_expires_at_five_minutes() {
    let mut harness = ready_harness(None);
    let key = entity_key();
    harness.entity_keys.push(key.clone());
    harness.budget.entity_last_seen_ms.insert(
        EntityKey::parse(&key).expect("entity"),
        harness.now_ms - 300_000,
    );
    assert!(matches!(
        harness.outcome().decision,
        GateDecision::Allow { .. }
    ));
}

#[test]
fn suppression_blocks_auto_allows_on_demand() {
    let mut harness = ready_harness(None);
    harness.suppression_topics.push(1);
    let auto = harness.outcome();
    assert_eq!(
        deny_of(&auto),
        (GateRejectReason::Suppressed, GateStage::G3)
    );
    assert_eq!(auto.counters.get(GateCounter::GateL3Suppressed), 1);

    harness.trigger = TriggerKind::OnDemand;
    let on_demand = harness.outcome();
    match on_demand.decision {
        GateDecision::Allow {
            suppressed_notice, ..
        } => assert!(suppressed_notice),
        other => panic!("expected allow, got {other:?}"),
    }
}

#[test]
fn fresh_cache_is_allowed() {
    let mut harness = ready_harness(None);
    harness.cache_age_ms = Some(86_400_000);
    assert!(matches!(
        harness.outcome().decision,
        GateDecision::Allow { .. }
    ));
}

#[test]
fn stale_cache_is_denied() {
    let mut harness = ready_harness(None);
    harness.cache_age_ms = Some(86_400_001);
    let outcome = harness.outcome();
    assert_eq!(deny_of(&outcome), (GateRejectReason::Stale, GateStage::G3));
    assert_eq!(outcome.counters.get(GateCounter::GateL3Stale), 1);
}

#[test]
fn candidate_convergence_keeps_top_three() {
    let mut harness = Harness::new(10_000);
    for index in 1..=11 {
        let term = format!("term{index:02}");
        harness.lexicon.push(term.clone());
        harness.push(&format!("{term} 内容"), 0.9, index * 10, index * 10 + 5);
    }
    harness.relevance.semantic_milli = 1000;
    harness.relevance.entity_milli = 1000;
    harness.relevance.gain_milli = 1000;
    harness.relevance.source_trust_milli = 1000;
    let outcome = harness.outcome();
    match &outcome.decision {
        GateDecision::Allow { candidates, .. } => assert_eq!(candidates.len(), 3),
        other => panic!("expected allow, got {other:?}"),
    }
    assert_eq!(outcome.counters.get(GateCounter::GateL3Quota), 8);
}

#[test]
fn dedupe_switch_keeps_suppression_and_cache() {
    let policy = GatePolicy {
        dedupe_enabled: false,
        ..GatePolicy::default()
    };

    let mut suppressed = ready_harness(None);
    suppressed.suppression_topics.push(1);
    assert_eq!(
        deny_of(&suppressed.outcome_with(&policy)),
        (GateRejectReason::Suppressed, GateStage::G3)
    );

    let mut stale = ready_harness(None);
    stale.cache_age_ms = Some(86_400_001);
    assert_eq!(
        deny_of(&stale.outcome_with(&policy)),
        (GateRejectReason::Stale, GateStage::G3)
    );

    let skipped = ready_harness(None).outcome_with(&policy);
    assert!(matches!(skipped.decision, GateDecision::Allow { .. }));
    assert_eq!(skipped.counters.get(GateCounter::GateStageSkippedTotal), 1);
}

#[test]
fn single_window_drives_both_stages() {
    let mut harness = ready_harness(None);
    harness.budget.window_start_ms = harness.now_ms;
    harness.budget.topic_window_used.insert(1, 2);
    assert_eq!(
        deny_of(&harness.outcome()),
        (GateRejectReason::QuotaExhausted, GateStage::G3)
    );

    let no_dedupe = GatePolicy {
        dedupe_enabled: false,
        ..GatePolicy::default()
    };
    assert_eq!(
        deny_of(&harness.outcome_with(&no_dedupe)),
        (GateRejectReason::BudgetExhausted, GateStage::G4)
    );

    harness.budget.topic_window_used.insert(1, 0);
    assert!(matches!(
        harness.outcome_with(&no_dedupe).decision,
        GateDecision::Allow { .. }
    ));
}
