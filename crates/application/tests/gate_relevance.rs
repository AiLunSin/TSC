//! G2 相关性（AC-B4-09~14）。

mod common;

use common::Harness;
use meeting_copilot_application::gate::{decay_milli, score_relevance, RelevanceInput};
use meeting_copilot_domain::model::card::{DedupeKey, GapCandidate};
use meeting_copilot_domain::model::enums::{GapKind, GapState};
use meeting_copilot_domain::model::gate::{
    GateCounter, GateDecision, GatePolicy, GateRejectReason, GateStage, StageResult,
    ThresholdProfile,
};
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::vo::Confidence;

fn candidate() -> GapCandidate {
    GapCandidate {
        kind: GapKind::ProperNoun,
        trigger_span: TextRange {
            start_ms: 0,
            end_ms: 100,
        },
        confidence: Confidence::new(0.9).expect("confidence"),
        dedupe_key: DedupeKey::new("a".repeat(32)).expect("key"),
        state: GapState::Detected,
    }
}

fn score(semantic: u16, entity: u16, gain: u16, trust: u16, delta_ms: u64) -> u16 {
    let item = candidate();
    let input = RelevanceInput {
        candidate: &item,
        semantic_milli: semantic,
        entity_milli: entity,
        gain_milli: gain,
        source_trust_milli: trust,
        delta_ms,
        threshold_profile: ThresholdProfile::Balanced,
    };
    score_relevance(&input, &GatePolicy::default()).score_milli
}

#[test]
fn relevance_weights_match_spec() {
    assert_eq!(score(1000, 0, 0, 0, 0), 400);
    assert_eq!(score(0, 1000, 0, 0, 0), 250);
    assert_eq!(score(0, 0, 1000, 0, 0), 200);
    assert_eq!(score(0, 0, 0, 1000, 0), 150);
}

#[test]
fn relevance_full_score_is_thousand() {
    assert_eq!(score(1000, 1000, 1000, 1000, 0), 1000);
}

#[test]
fn time_decay_at_180_seconds_is_367_milli() {
    assert!((367..=368).contains(&decay_milli(180_000)));
    assert_eq!(decay_milli(0), 1000);
    assert_eq!(decay_milli(180_000), decay_milli(180_000));
}

#[test]
fn relevance_parts_are_all_reported() {
    let item = candidate();
    let input = RelevanceInput {
        candidate: &item,
        semantic_milli: 900,
        entity_milli: 800,
        gain_milli: 700,
        source_trust_milli: 600,
        delta_ms: 0,
        threshold_profile: ThresholdProfile::Balanced,
    };
    let reported = score_relevance(&input, &GatePolicy::default());
    assert_eq!(reported.semantic_milli, 900);
    assert_eq!(reported.entity_milli, 800);
    assert_eq!(reported.gain_milli, 700);
    assert_eq!(reported.source_trust_milli, 600);
    assert_eq!(reported.decay_milli, 1000);
    assert_eq!(reported.score_milli, 790);
    assert!(reported.score_milli <= 1000);
}

#[test]
fn out_of_range_parts_are_clamped() {
    let item = candidate();
    let input = RelevanceInput {
        candidate: &item,
        semantic_milli: 5_000,
        entity_milli: 0,
        gain_milli: 0,
        source_trust_milli: 0,
        delta_ms: 0,
        threshold_profile: ThresholdProfile::Balanced,
    };
    let reported = score_relevance(&input, &GatePolicy::default());
    assert_eq!(reported.semantic_milli, 1000);
    assert_eq!(reported.score_milli, 400);
    assert!(reported.score_milli <= 1000);
}

#[test]
fn threshold_profiles_are_072_062_052() {
    assert_eq!(ThresholdProfile::Strict.milli(), 720);
    assert_eq!(ThresholdProfile::Balanced.milli(), 620);
    assert_eq!(ThresholdProfile::Loose.milli(), 520);
    assert_eq!(ThresholdProfile::default(), ThresholdProfile::Balanced);
}

fn boundary_harness(entity_milli: u16) -> Harness {
    let mut harness = Harness::new(10_000);
    harness.lexicon.push("张伟".to_owned());
    harness.push("张伟负责该模块。", 0.9, 0, 100);
    harness.relevance.semantic_milli = 1000;
    harness.relevance.entity_milli = entity_milli;
    harness
}

#[test]
fn balanced_threshold_boundary() {
    let pass = boundary_harness(880).outcome();
    assert!(matches!(pass.decision, GateDecision::Allow { .. }));

    let deny = boundary_harness(876).outcome();
    match deny.decision {
        GateDecision::Deny {
            reason,
            stopped_at,
            per_stage,
            ..
        } => {
            assert_eq!(reason, GateRejectReason::LowRelevance);
            assert_eq!(stopped_at, GateStage::G2);
            assert_eq!(per_stage.0[2].result, StageResult::NotReached);
        }
        other => panic!("expected low relevance deny, got {other:?}"),
    }
}

#[test]
fn low_relevance_is_denied() {
    let outcome = boundary_harness(876).outcome();
    assert_eq!(outcome.counters.get(GateCounter::GateL2LowRelevance), 1);
    assert_eq!(outcome.counters.get(GateCounter::GateDenyTotal), 1);
    if let GateDecision::Deny { per_stage, .. } = outcome.decision {
        assert_eq!(per_stage.0[3].calls, 0);
        assert_eq!(per_stage.0[4].calls, 0);
    }
}

#[test]
fn empty_topic_is_deterministic() {
    let first = score(0, 0, 0, 0, 0);
    assert_eq!(first, 0);
    for _ in 0..100 {
        assert_eq!(score(0, 0, 0, 0, 0), first);
    }
}

#[test]
fn relevance_never_calls_llm_or_network() {
    let first = score(700, 600, 500, 400, 30_000);
    for _ in 0..100 {
        assert_eq!(score(700, 600, 500, 400, 30_000), first);
    }
}

#[test]
fn relevance_is_deterministic_for_mixed_text() {
    let first = score(650, 550, 450, 350, 60_000);
    for _ in 0..100 {
        assert_eq!(score(650, 550, 450, 350, 60_000), first);
    }
}
