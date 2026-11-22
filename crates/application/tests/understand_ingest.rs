//! 采集服务接入理解层：合并单元只提炼一次。

mod support;

use std::sync::Arc;

use meeting_copilot_application::capture::capture_ports::NoopDiagnostics;
use meeting_copilot_application::capture::merge_types::{MergedUnit, SingleUtterance};
use meeting_copilot_application::capture::{CaptureChannelService, CapturePolicy};
use meeting_copilot_application::understand::{
    UnderstandFeature, UnderstandOutcome, UnderstandService,
};
use meeting_copilot_domain::model::enums::{Platform, ReadingMode, SessionState, SourceKind};
use meeting_copilot_domain::model::insight_policy::InsightPolicy;
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::model::utterance::{SpeakerRef, Utterance};
use meeting_copilot_domain::vo::{Confidence, SessionId, UtteranceRef, UtteranceText, WallClock};
use support::fakes::{
    CountingAsrDelivery, CountingInsight, MemoryGapLog, MemorySwitchLog, MonotonicSystemClock,
    RecordingStatusSink,
};

fn utterance() -> Utterance {
    Utterance {
        reference: UtteranceRef::new("u1").expect("ref"),
        span: TextRange {
            start_ms: 0,
            end_ms: 1_000,
        },
        wall_clock: WallClock::from_unix_millis(0),
        text: UtteranceText::new("确认下周灰度上线").expect("text"),
        speaker: SpeakerRef::unknown(),
        confidence: Confidence::new(0.9).expect("conf"),
        source: SourceKind::PlatformSubtitle,
        platform: Some(Platform::TencentMeeting),
        dual_channel: false,
        reading_mode: ReadingMode::Subtitle,
    }
}

fn service() -> CaptureChannelService {
    CaptureChannelService::with_offline_ports(
        Arc::new(MonotonicSystemClock::default()),
        Arc::new(MemorySwitchLog::default()),
        Arc::new(MemoryGapLog::default()),
        Arc::new(RecordingStatusSink::default()),
        Arc::new(CountingAsrDelivery::default()),
        Arc::new(CountingInsight::default()),
        Arc::new(NoopDiagnostics),
        CapturePolicy::default(),
    )
    .with_understand(Arc::new(
        UnderstandService::new(InsightPolicy::default(), UnderstandFeature::Enabled)
            .expect("understand"),
    ))
}

#[test]
fn merged_unit_triggers_insight_once() {
    let service = service();
    let session = SessionId::new("s1").expect("session");
    let unit = MergedUnit::Single(SingleUtterance {
        utterance: utterance(),
        overlap_marked: false,
    });

    let first = service.admit_merged(
        &session,
        unit.clone(),
        SessionState::Listening,
        Some(Platform::TencentMeeting),
    );
    assert!(matches!(first, UnderstandOutcome::Admitted { .. }));

    let second = service.admit_merged(
        &session,
        unit,
        SessionState::Listening,
        Some(Platform::TencentMeeting),
    );
    assert_eq!(second, UnderstandOutcome::Duplicate);
}

#[test]
fn without_understand_is_disabled() {
    let service = CaptureChannelService::with_offline_ports(
        Arc::new(MonotonicSystemClock::default()),
        Arc::new(MemorySwitchLog::default()),
        Arc::new(MemoryGapLog::default()),
        Arc::new(RecordingStatusSink::default()),
        Arc::new(CountingAsrDelivery::default()),
        Arc::new(CountingInsight::default()),
        Arc::new(NoopDiagnostics),
        CapturePolicy::default(),
    );
    let session = SessionId::new("s1").expect("session");
    let unit = MergedUnit::Single(SingleUtterance {
        utterance: utterance(),
        overlap_marked: false,
    });
    assert_eq!(
        service.admit_merged(&session, unit, SessionState::Listening, None),
        UnderstandOutcome::Disabled
    );
}
