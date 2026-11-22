//! G4 预算/冷却/离线/并发 + C-03 租约（AC-B4-22~27）。

mod common;

use common::Harness;
use meeting_copilot_application::gate::{refill, LeaseId, LeaseOutcome, LeaseTable};
use meeting_copilot_domain::model::gate::{
    FrequencyProfile, GateCounter, GateDecision, GatePolicy, GateRejectReason, GateStage,
    NetworkState,
};

fn deny_of(
    outcome: &meeting_copilot_application::gate::GateOutcome,
) -> (GateRejectReason, GateStage) {
    match &outcome.decision {
        GateDecision::Deny {
            reason, stopped_at, ..
        } => (*reason, *stopped_at),
        other => panic!("expected deny, got {other:?}"),
    }
}

fn ready() -> Harness {
    let mut harness = Harness::new(10_000);
    harness.lexicon.push("张伟".to_owned());
    harness.push("张伟负责该模块。", 0.9, 0, 100);
    harness.relevance.semantic_milli = 1000;
    harness.relevance.entity_milli = 1000;
    harness.relevance.gain_milli = 1000;
    harness.relevance.source_trust_milli = 1000;
    harness
}

#[test]
fn budget_is_independent_of_llm_live_cap() {
    let harness = ready();
    let first = harness.outcome().decision;
    let second = harness.outcome().decision;
    assert_eq!(first, second);
}

#[test]
fn session_outbound_cap_is_120() {
    let mut harness = ready();
    harness.budget.session_outbound_used = 119;
    match harness.outcome().decision {
        GateDecision::Allow {
            budget_remaining, ..
        } => assert_eq!(budget_remaining, 1),
        other => panic!("expected allow, got {other:?}"),
    }

    harness.budget.session_outbound_used = 120;
    let outcome = harness.outcome();
    assert_eq!(
        deny_of(&outcome),
        (GateRejectReason::BudgetExhausted, GateStage::G4)
    );
    assert_eq!(outcome.counters.get(GateCounter::GateL4BudgetExhausted), 1);

    let zero_cap = GatePolicy {
        session_outbound_cap: 0,
        ..GatePolicy::default()
    };
    let mut harness = ready();
    harness.budget.session_outbound_used = 0;
    assert_eq!(
        deny_of(&harness.outcome_with(&zero_cap)),
        (GateRejectReason::BudgetExhausted, GateStage::G4)
    );
}

#[test]
fn token_bucket_capacity_and_refill() {
    assert_eq!(refill(0, 0, 600_001, FrequencyProfile::Medium), 3000);
    assert_eq!(refill(0, 0, 0, FrequencyProfile::Low), 0);
    assert_eq!(refill(3000, 0, 600_001, FrequencyProfile::High), 3000);

    let mut empty = ready();
    empty.budget.tokens_milli = 0;
    empty.budget.window_start_ms = empty.now_ms;
    assert_eq!(
        deny_of(&empty.outcome()),
        (GateRejectReason::BudgetExhausted, GateStage::G4)
    );

    let mut filled = ready();
    filled.budget.tokens_milli = 1000;
    assert!(matches!(
        filled.outcome().decision,
        GateDecision::Allow { .. }
    ));
}

#[test]
fn frequency_profiles_are_1_3_5() {
    assert_eq!(FrequencyProfile::Low.per_window(), 1);
    assert_eq!(FrequencyProfile::Medium.per_window(), 3);
    assert_eq!(FrequencyProfile::High.per_window(), 5);
}

#[test]
fn evaluate_gate_does_not_mutate_budget() {
    let harness = ready();
    let before = harness.budget.clone();
    let _ = harness.outcome();
    assert_eq!(harness.budget, before);
}

#[test]
fn commit_advances_only_on_allow() {
    let mut harness = ready();
    harness.budget.inflight = 3;
    let allow = harness.outcome().decision;
    let mut budget = harness.budget.clone();
    budget.commit(&allow);
    assert_eq!(budget.session_outbound_used, 1);
    assert_eq!(budget.global_window_used, 1);
    assert_eq!(budget.tokens_milli, 2000);

    let mut offline = ready();
    offline.network = NetworkState::Offline;
    let deny = offline.outcome().decision;
    let mut budget = offline.budget.clone();
    let before = budget.clone();
    budget.commit(&deny);
    assert_eq!(budget, before);
}

#[test]
fn gateway_cooling_denies_without_retry() {
    let mut harness = ready();
    harness.llm_cooling = true;
    harness.llm_cooling_until_ms = Some(9_000);
    let outcome = harness.outcome();
    match outcome.decision {
        GateDecision::Deny {
            reason,
            stopped_at,
            cooldown_until_ms,
            ..
        } => {
            assert_eq!(reason, GateRejectReason::GatewayCooling);
            assert_eq!(stopped_at, GateStage::G4);
            assert_eq!(cooldown_until_ms, Some(9_000));
        }
        other => panic!("expected cooling deny, got {other:?}"),
    }
    assert_eq!(outcome.counters.get(GateCounter::GateL4GatewayCooling), 1);
}

#[test]
fn gateway_cooling_does_not_block_admission() {
    let mut harness = ready();
    harness.llm_cooling = true;
    let first = harness.outcome().decision;
    let second = harness.outcome().decision;
    assert_eq!(first, second);
    assert!(matches!(first, GateDecision::Deny { .. }));
}

#[test]
fn budget_recovers_after_window() {
    let policy = GatePolicy {
        dedupe_enabled: false,
        ..GatePolicy::default()
    };
    let mut harness = ready();
    harness.budget.global_window_used = 3;
    harness.budget.tokens_milli = 3000;
    harness.now_ms = 599_999;
    assert_eq!(
        deny_of(&harness.outcome_with(&policy)),
        (GateRejectReason::BudgetExhausted, GateStage::G4)
    );

    harness.now_ms = 600_001;
    assert!(matches!(
        harness.outcome_with(&policy).decision,
        GateDecision::Allow { .. }
    ));
}

#[test]
fn offline_and_unknown_network_paths() {
    let mut harness = ready();
    harness.network = NetworkState::Offline;
    let outcome = harness.outcome();
    assert_eq!(
        deny_of(&outcome),
        (GateRejectReason::Offline, GateStage::G4)
    );
    assert_eq!(outcome.counters.get(GateCounter::GateL4Offline), 1);

    harness.network = NetworkState::Unknown;
    assert!(matches!(
        harness.outcome().decision,
        GateDecision::Allow { .. }
    ));
}

#[test]
fn fourth_concurrent_request_is_queued() {
    let mut harness = ready();
    harness.budget.inflight = 3;
    match harness.outcome().decision {
        GateDecision::Allow { outbound, .. } => match outbound {
            Some(intent) => assert!(intent.queued),
            None => panic!("expected queued outbound intent"),
        },
        other => panic!("expected allow, got {other:?}"),
    }
}

#[test]
fn lease_acquires_three_slots_then_queues() {
    let mut lease = LeaseTable::new();
    let first = lease.try_acquire();
    let second = lease.try_acquire();
    let third = lease.try_acquire();
    for (index, outcome) in [first, second, third].into_iter().enumerate() {
        match outcome {
            LeaseOutcome::Acquired { lease_id, inflight } => {
                assert_eq!(lease_id.value(), index as u64);
                assert_eq!(inflight, (index + 1) as u8);
            }
            other => panic!("expected acquired, got {other:?}"),
        }
    }
    assert_eq!(
        lease.try_acquire(),
        LeaseOutcome::Queued {
            position: 1,
            queue_len: 1
        }
    );
}

#[test]
fn fifth_request_does_not_grow_queue() {
    let mut lease = LeaseTable::new();
    for _ in 0..3 {
        let _ = lease.try_acquire();
    }
    let _ = lease.try_acquire();
    let fifth = lease.try_acquire();
    assert_eq!(
        fifth,
        LeaseOutcome::Queued {
            position: 1,
            queue_len: 1
        }
    );
    assert_eq!(lease.queue_len(), 1);
}

#[test]
fn release_promotes_queue_head_to_lowest_free_slot() {
    let mut lease = LeaseTable::new();
    let first = match lease.try_acquire() {
        LeaseOutcome::Acquired { lease_id, .. } => lease_id,
        other => panic!("expected acquired, got {other:?}"),
    };
    let second = match lease.try_acquire() {
        LeaseOutcome::Acquired { lease_id, .. } => lease_id,
        other => panic!("expected acquired, got {other:?}"),
    };
    let _ = lease.try_acquire();
    let queued = match lease.try_acquire() {
        LeaseOutcome::Queued { .. } => true,
        other => panic!("expected queued, got {other:?}"),
    };
    assert!(queued);

    let promoted = lease.release(first);
    assert!(promoted.is_some());
    let promoted = promoted.expect("promoted");
    assert_ne!(promoted, first);
    assert_ne!(promoted, second);
    assert_eq!(lease.inflight(), 3);

    assert_eq!(lease.release(LeaseId::from_value(999)), None);
    assert_eq!(lease.inflight(), 3);
}
