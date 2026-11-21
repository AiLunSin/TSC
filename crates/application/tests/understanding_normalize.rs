//! 28 准入的集成用例：开关、会话状态、静音、平台一致性。

use meeting_copilot_application::capture::merge_types::{MergedUnit, SingleUtterance};
use meeting_copilot_application::understand::{
    RejectReason, SilenceSignal, UnderstandFeature, UnderstandInput, UnderstandOutcome,
    UnderstandService,
};
use meeting_copilot_domain::model::enums::{Platform, ReadingMode, SessionState, SourceKind};
use meeting_copilot_domain::model::insight_policy::InsightPolicy;
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::model::utterance::{SpeakerRef, Utterance};
use meeting_copilot_domain::vo::{Confidence, SessionId, UtteranceRef, UtteranceText, WallClock};

fn utterance(reference: &str, source: SourceKind, platform: Option<Platform>) -> Utterance {
    Utterance {
        reference: UtteranceRef::new(reference).expect("ref"),
        span: TextRange {
            start_ms: 0,
            end_ms: 1_000,
        },
        wall_clock: WallClock::from_unix_millis(0),
        text: UtteranceText::new("确认下周灰度上线").expect("text"),
        speaker: SpeakerRef::unknown(),
        confidence: Confidence::new(0.9).expect("conf"),
        source,
        platform,
        dual_channel: false,
        reading_mode: ReadingMode::Subtitle,
    }
}

fn input(state: SessionState, utterance: Utterance) -> UnderstandInput {
    UnderstandInput {
        session: SessionId::new("s1").expect("session"),
        unit: MergedUnit::Single(SingleUtterance {
            utterance,
            overlap_marked: false,
        }),
        state,
        silence: SilenceSignal::none(),
        session_platform: Some(Platform::TencentMeeting),
    }
}

fn service(feature: UnderstandFeature) -> UnderstandService {
    UnderstandService::new(InsightPolicy::default(), feature).expect("service")
}

#[test]
fn disabled_feature_is_zero_effect() {
    let service = service(UnderstandFeature::Disabled);
    let outcome = service.observe(&input(
        SessionState::Listening,
        utterance(
            "u1",
            SourceKind::PlatformSubtitle,
            Some(Platform::TencentMeeting),
        ),
    ));
    assert_eq!(outcome, UnderstandOutcome::Disabled);
    assert_eq!(service.counters().admitted, 0);
    assert_eq!(service.counters().gateway_calls, 0);
}

#[test]
fn paused_session_drops_all_fragments() {
    let service = service(UnderstandFeature::Enabled);
    for index in 0..3 {
        let outcome = service.observe(&input(
            SessionState::Paused,
            utterance(
                &format!("u{index}"),
                SourceKind::PlatformSubtitle,
                Some(Platform::TencentMeeting),
            ),
        ));
        assert_eq!(outcome, UnderstandOutcome::Rejected(RejectReason::Paused));
    }
    assert_eq!(service.counters().paused_drop, 3);
    assert_eq!(service.counters().admitted, 0);
}

#[test]
fn silent_period_still_records_without_dispatch() {
    let service = service(UnderstandFeature::Enabled);
    let outcome = service.observe(&input(
        SessionState::Silent,
        utterance(
            "u1",
            SourceKind::PlatformSubtitle,
            Some(Platform::TencentMeeting),
        ),
    ));
    assert!(matches!(outcome, UnderstandOutcome::Admitted { .. }));
    assert_eq!(service.counters().admitted, 1);
    assert_eq!(service.counters().dispatch_candidates, 0);
}

#[test]
fn platform_subtitle_requires_matching_platform() {
    let service = service(UnderstandFeature::Enabled);
    let outcome = service.observe(&input(
        SessionState::Listening,
        utterance("u1", SourceKind::PlatformSubtitle, None),
    ));
    assert_eq!(
        outcome,
        UnderstandOutcome::Rejected(RejectReason::SourcePlatformMismatch)
    );
    assert_eq!(service.counters().source_platform_mismatch, 1);
}
#[test]
fn duplicate_delivery_is_ignored() {
    let service = service(UnderstandFeature::Enabled);
    let item = input(
        SessionState::Listening,
        utterance(
            "dup",
            SourceKind::PlatformSubtitle,
            Some(Platform::TencentMeeting),
        ),
    );
    assert!(matches!(
        service.observe(&item),
        UnderstandOutcome::Admitted { .. }
    ));
    assert_eq!(service.observe(&item), UnderstandOutcome::Duplicate);
    assert_eq!(service.counters().duplicate_delivery_ignored, 1);
    assert_eq!(service.counters().admitted, 1);
}

#[test]
fn partial_silence_keeps_fragment() {
    let service = service(UnderstandFeature::Enabled);
    let mut item = input(
        SessionState::Listening,
        utterance(
            "u1",
            SourceKind::PlatformSubtitle,
            Some(Platform::TencentMeeting),
        ),
    );
    item.silence = SilenceSignal::from_intervals(vec![TextRange {
        start_ms: 500,
        end_ms: 6_000,
    }]);
    let outcome = service.observe(&item);
    assert!(matches!(outcome, UnderstandOutcome::Admitted { .. }));
    assert_eq!(service.counters().silence_partial_overlap, 1);
    assert_eq!(service.counters().silence_dropped, 0);
}

#[test]
fn speaker_fallback_is_explicitly_unidentified() {
    let service = service(UnderstandFeature::Enabled);
    let mut fragment = utterance(
        "u1",
        SourceKind::PlatformSubtitle,
        Some(Platform::TencentMeeting),
    );
    fragment.speaker = SpeakerRef::anonymous("");
    let outcome = service.observe(&input(SessionState::Listening, fragment));
    match outcome {
        UnderstandOutcome::Admitted { fragments } => {
            assert!(!fragments[0].speaker.identified);
            assert_eq!(fragments[0].speaker.label, "未知说话人");
        }
        other => panic!("expected admitted, got {other:?}"),
    }
}
