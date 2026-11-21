//! B2.24 双通道去重合并的离线门禁用例（P0 守门面）。
//!
//! 本 target 的用例只校验合并输出集合的区间不变量，
//! 不依赖设备、时钟或网络。

mod support;

use meeting_copilot_application::capture::dedupe_guard::DedupeMergeGuard;
use meeting_copilot_application::capture::merge_types::{AsrTextEvent, DedupeCounters, MergedUnit};
use meeting_copilot_application::capture::{
    validate_merged_set, AsrIntentKind, CaptureGapRecord, MergeSetViolation, MergeSpan,
};
use meeting_copilot_domain::merge::MergePolicy;
use meeting_copilot_domain::model::channel::ChannelKind;
use meeting_copilot_domain::model::enums::Platform;
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::model::utterance::SpeakerRef;
use meeting_copilot_domain::model::GapReason;
use meeting_copilot_domain::vo::{Confidence, MonoMillis, SessionId, UtteranceRef, UtteranceText};

fn event(reference: &str, start: u64, end: u64, text: &str) -> AsrTextEvent {
    AsrTextEvent {
        session: SessionId::new("s1").expect("session"),
        kind: AsrIntentKind::GapBackfill,
        span: TextRange {
            start_ms: start,
            end_ms: end,
        },
        text: UtteranceText::new(text).expect("text"),
        speaker: SpeakerRef::unknown(),
        confidence: Confidence::new(0.9).expect("conf"),
        estimated: false,
        platform: Some(Platform::TencentMeeting),
        reference: UtteranceRef::new(reference).expect("ref"),
    }
}

fn at(millis: u64) -> MonoMillis {
    MonoMillis::from_millis(millis)
}

fn span(start: u64, end: u64, dual: bool, marked: bool) -> MergeSpan {
    MergeSpan {
        start: at(start),
        end: at(end),
        dual_channel: dual,
        overlap_marked: marked,
        origin_pair: None,
    }
}

fn gap(start: u64, end: u64) -> CaptureGapRecord {
    CaptureGapRecord {
        session: SessionId::new("s1").expect("session"),
        start: at(start),
        end: at(end),
        reason: GapReason::BothChannelsUnavailable,
        backfilled: false,
    }
}

#[test]
fn merged_set_spans_are_pairwise_disjoint() {
    let valid = [
        span(0, 1_000, true, false),
        span(1_000, 2_000, true, false),
        span(2_000, 3_000, true, false),
    ];
    assert!(validate_merged_set(&valid, &[]).is_ok());

    let overlapped = [
        span(0, 1_000, true, false),
        span(999, 2_000, true, false),
        span(2_000, 3_000, true, false),
    ];
    assert_eq!(
        validate_merged_set(&overlapped, &[]),
        Err(MergeSetViolation::MergedOverlap {
            left: 0,
            right: 1,
            overlap_ms: 1
        })
    );
}

#[test]
fn merged_span_does_not_overlap_gap_interval() {
    let units = [span(0, 1_000, true, false)];

    assert!(validate_merged_set(&units, &[gap(1_000, 2_000)]).is_ok());

    assert_eq!(
        validate_merged_set(&units, &[gap(999, 2_000)]),
        Err(MergeSetViolation::MergedOverlapsGap {
            unit: 0,
            gap: 0,
            overlap_ms: 1
        })
    );
}

#[test]
fn late_unmerged_pair_requires_overlap_mark() {
    let marked = [span(0, 1_000, false, true), span(900, 2_000, false, true)];
    assert!(validate_merged_set(&marked, &[]).is_ok());

    let unmarked = [span(0, 1_000, false, false), span(900, 2_000, false, false)];
    assert_eq!(
        validate_merged_set(&unmarked, &[]),
        Err(MergeSetViolation::CrossOriginOverlapNotMarked {
            left: 0,
            right: 1,
            overlap_ms: 100
        })
    );
}
#[test]
fn pair_within_default_window() {
    let guard = DedupeMergeGuard::new(MergePolicy::default()).expect("guard");
    assert!(guard
        .push(
            ChannelKind::Subtitle,
            event("sub-1", 1_000, 2_000, "确认下周灰度上线")
        )
        .is_empty());

    let units = guard.push(
        ChannelKind::Audio,
        event("asr-1", 1_100, 1_900, "确认下周灰度上线"),
    );
    assert_eq!(units.len(), 1);
    assert!(matches!(units[0], MergedUnit::Merged(_)));
    assert_eq!(guard.counters().merge_applied, 1);
    assert_eq!(guard.dedupe_evidence().merged.insight_trigger_count, 1);
}

#[test]
fn pair_outside_window_is_none() {
    let guard = DedupeMergeGuard::new(MergePolicy::default()).expect("guard");
    guard.push(
        ChannelKind::Subtitle,
        event("sub-1", 0, 800, "确认下周灰度上线"),
    );
    let units = guard.push(
        ChannelKind::Audio,
        event("asr-1", 3_500, 4_500, "确认下周灰度上线"),
    );
    assert_eq!(units.len(), 2);
    assert_eq!(guard.counters().late_unmerged, 1);
}

#[test]
fn merge_disabled_is_passthrough() {
    let policy = MergePolicy {
        merge_enabled: false,
        ..MergePolicy::default()
    };
    let guard = DedupeMergeGuard::new(policy).expect("guard");
    let units = guard.push(
        ChannelKind::Subtitle,
        event("sub-1", 1_000, 2_000, "确认下周灰度上线"),
    );
    assert_eq!(units.len(), 1);
    match &units[0] {
        MergedUnit::Single(single) => {
            assert!(!single.utterance.dual_channel);
            assert_eq!(single.utterance.span.start_ms, 1_000);
        }
        MergedUnit::Merged(_) => panic!("禁用时不得合并"),
    }
}

#[test]
fn duplicate_pair_applied_once() {
    let guard = DedupeMergeGuard::new(MergePolicy::default()).expect("guard");
    guard.push(
        ChannelKind::Subtitle,
        event("sub-1", 1_000, 2_000, "确认下周灰度上线"),
    );
    let first = guard.push(
        ChannelKind::Audio,
        event("asr-1", 1_100, 1_900, "确认下周灰度上线"),
    );
    let second = guard.push(
        ChannelKind::Audio,
        event("asr-1", 1_100, 1_900, "确认下周灰度上线"),
    );
    assert_eq!(first.len(), 1);
    assert!(second.is_empty());
    assert_eq!(guard.counters().merge_applied, 1);
    assert_eq!(guard.counters().duplicate_suppressed, 1);
}
#[test]
fn merged_utterance_triggers_count_once() {
    let guard = DedupeMergeGuard::new(MergePolicy::default()).expect("guard");
    guard.push(
        ChannelKind::Subtitle,
        event("sub-1", 1_000, 2_000, "确认下周灰度上线"),
    );
    let units = guard.push(
        ChannelKind::Audio,
        event("asr-1", 1_100, 1_900, "确认下周灰度上线"),
    );
    assert_eq!(units.len(), 1);

    let evidence = guard.dedupe_evidence();
    assert_eq!(evidence.merged.insight_trigger_count, 1);
    assert_eq!(evidence.merged.gap_detect_count, 1);
    assert_eq!(evidence.merged.summary_entry_count, 1);
    assert_eq!(evidence.merged.persisted_utterance_count, 1);
    assert_eq!(evidence.merged.mentions_delta, 0);
    assert_eq!(evidence.folded, DedupeCounters::default());
}

#[test]
fn duplicate_side_triggers_zero_insight() {
    let guard = DedupeMergeGuard::new(MergePolicy::default()).expect("guard");
    guard.push(
        ChannelKind::Subtitle,
        event("sub-1", 1_000, 2_000, "确认下周灰度上线"),
    );
    guard.push(
        ChannelKind::Audio,
        event("asr-1", 1_100, 1_900, "确认下周灰度上线"),
    );
    let again = guard.push(
        ChannelKind::Audio,
        event("asr-1", 1_100, 1_900, "确认下周灰度上线"),
    );
    assert!(again.is_empty());
    assert_eq!(guard.dedupe_evidence().merged.insight_trigger_count, 1);
}
#[test]
fn merged_unit_projects_to_disjoint_spans() {
    let guard = DedupeMergeGuard::new(MergePolicy::default()).expect("guard");
    guard.push(
        ChannelKind::Subtitle,
        event("sub-1", 1_000, 2_000, "确认下周灰度上线"),
    );
    let units = guard.push(
        ChannelKind::Audio,
        event("asr-1", 1_100, 1_900, "确认下周灰度上线"),
    );
    let spans: Vec<MergeSpan> = units.iter().map(MergeSpan::from).collect();
    assert_eq!(spans.len(), 1);
    assert!(spans[0].dual_channel);
    assert_eq!(spans[0].start.as_millis(), 1_100);
    assert!(spans[0].origin_pair.is_some());
    assert!(validate_merged_set(&spans, &[]).is_ok());
}

#[test]
fn single_channel_output_unchanged() {
    let guard = DedupeMergeGuard::new(MergePolicy::default()).expect("guard");
    assert!(guard
        .push(
            ChannelKind::Subtitle,
            event("sub-1", 1_000, 2_000, "确认下周灰度上线")
        )
        .is_empty());

    let released = guard.advance_to(MonoMillis::from_millis(10_000));
    assert_eq!(released.len(), 1);
    match &released[0] {
        MergedUnit::Single(single) => {
            assert_eq!(single.utterance.span.start_ms, 1_000);
            assert_eq!(single.utterance.span.end_ms, 2_000);
            assert!(!single.utterance.dual_channel);
            assert!(!single.overlap_marked);
        }
        MergedUnit::Merged(_) => panic!("单路不应合并"),
    }
}
