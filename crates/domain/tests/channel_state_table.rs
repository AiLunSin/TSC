//! 阶段 A 穷举：四状态、9 条边、16 组合、同状态与降级自环。

use std::collections::HashSet;

use meeting_copilot_domain::model::{
    transition, ChannelKey, ChannelKind, ChannelState, Platform, StateReason, TransitionOutcome,
};
use meeting_copilot_domain::vo::{MonoMillis, WallClock};

fn migrated(from: ChannelState, to: ChannelState, reason: StateReason) {
    assert_eq!(
        transition(from, to, reason).expect("transition"),
        TransitionOutcome::Migrated { from, to, reason },
        "{from:?}->{to:?}"
    );
}

#[test]
fn nine_legal_edges_covered() {
    let edges = [
        (
            ChannelState::Probing,
            ChannelState::Available,
            StateReason::ProbeCompleted,
        ),
        (
            ChannelState::Probing,
            ChannelState::Degraded,
            StateReason::SuspectedSilence,
        ),
        (
            ChannelState::Probing,
            ChannelState::Unavailable,
            StateReason::ContainerMissing,
        ),
        (
            ChannelState::Available,
            ChannelState::Degraded,
            StateReason::ConsecutiveReadFailures,
        ),
        (
            ChannelState::Available,
            ChannelState::Unavailable,
            StateReason::PermissionRevoked,
        ),
        (
            ChannelState::Degraded,
            ChannelState::Available,
            StateReason::ProbeCompleted,
        ),
        (
            ChannelState::Degraded,
            ChannelState::Unavailable,
            StateReason::OcrFailureRateAboveThreshold,
        ),
        (
            ChannelState::Unavailable,
            ChannelState::Probing,
            StateReason::UserRetry,
        ),
    ];
    for (from, to, reason) in edges {
        migrated(from, to, reason);
    }
    assert_eq!(
        transition(
            ChannelState::Degraded,
            ChannelState::Degraded,
            StateReason::OcrOnly
        )
        .expect("transition"),
        TransitionOutcome::InternalAdjustment {
            state: ChannelState::Degraded,
            reason: StateReason::OcrOnly
        }
    );
}

#[test]
fn sixteen_combinations_exhaustive() {
    let mut migrated_count = 0;
    let mut internal_count = 0;
    let mut noop_count = 0;
    let mut rejected_count = 0;
    let mut rejected = Vec::new();

    for from in ChannelState::ALL {
        for to in ChannelState::ALL {
            let reason = if from == ChannelState::Degraded && to == ChannelState::Degraded {
                StateReason::OcrOnly
            } else {
                StateReason::ProbeCompleted
            };
            match transition(from, to, reason).expect("transition") {
                TransitionOutcome::Migrated { .. } => migrated_count += 1,
                TransitionOutcome::InternalAdjustment { .. } => internal_count += 1,
                TransitionOutcome::NoOp => noop_count += 1,
                TransitionOutcome::Rejected { from, to } => {
                    rejected_count += 1;
                    rejected.push((from, to));
                }
            }
        }
    }

    assert_eq!(migrated_count, 8);
    assert_eq!(internal_count, 1);
    assert_eq!(noop_count, 3);
    assert_eq!(rejected_count, 4);
    assert_eq!(
        rejected,
        vec![
            (ChannelState::Available, ChannelState::Probing),
            (ChannelState::Degraded, ChannelState::Probing),
            (ChannelState::Unavailable, ChannelState::Available),
            (ChannelState::Unavailable, ChannelState::Degraded),
        ]
    );
}

#[test]
fn same_state_is_noop() {
    let mut noop = 0;
    for state in [
        ChannelState::Probing,
        ChannelState::Available,
        ChannelState::Unavailable,
    ] {
        for _ in 0..10 {
            let outcome =
                transition(state, state, StateReason::ProbeCompleted).expect("transition");
            assert_eq!(outcome, TransitionOutcome::NoOp);
            noop += 1;
        }
    }
    assert_eq!(noop, 30);
}

#[test]
fn degraded_self_loop_internal_adjustment() {
    for reason in [StateReason::OcrOnly, StateReason::DeviceFallback] {
        assert_eq!(
            transition(ChannelState::Degraded, ChannelState::Degraded, reason).expect("transition"),
            TransitionOutcome::InternalAdjustment {
                state: ChannelState::Degraded,
                reason
            }
        );
    }
    assert_eq!(
        transition(
            ChannelState::Degraded,
            ChannelState::Degraded,
            StateReason::SuspectedSilence
        )
        .expect("transition"),
        TransitionOutcome::NoOp
    );
}

#[test]
fn channel_key_is_ordered_and_serializable() {
    let keys = [
        ChannelKey::new(Platform::Zoom, ChannelKind::Audio),
        ChannelKey::new(Platform::TencentMeeting, ChannelKind::Subtitle),
        ChannelKey::new(Platform::Feishu, ChannelKind::Audio),
    ];
    let mut sorted = keys;
    sorted.sort();
    assert_eq!(sorted[0].platform, Platform::TencentMeeting);
    assert_eq!(sorted[2].platform, Platform::Zoom);

    let set: HashSet<ChannelKey> = keys.into_iter().collect();
    assert_eq!(set.len(), 3);
    let encoded = serde_json::to_string(&keys[0]).expect("serialize");
    let decoded: ChannelKey = serde_json::from_str(&encoded).expect("deserialize");
    assert_eq!(decoded, keys[0]);
}

#[test]
fn mono_millis_contract() {
    assert_eq!(MonoMillis::ZERO.as_millis(), 0);
    assert_eq!(MonoMillis::from_millis(1_500).as_millis(), 1_500);
    assert_eq!(
        MonoMillis::from_millis(5_000).saturating_sub(MonoMillis::from_millis(2_000)),
        3_000
    );
    assert_eq!(MonoMillis::from_millis(u64::MAX).checked_add(1), None);
    assert_eq!(
        MonoMillis::from_millis(1_000).to_wall(),
        WallClock::from_unix_millis(1_000)
    );
}
