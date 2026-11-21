mod support;

use std::sync::{Arc, Mutex};

use meeting_copilot_application::capture::{
    AsrDeliverySink, AsrIntent, AsrIntentKind, CaptureChannelService, CapturePolicy,
    CapturePortError, ChannelObservation, ChannelSwitchLog, ChannelSwitchRecord, NoopGapLog,
    NoopStatusSink, ObservationPayload, SessionContextView, SwitchLogError,
};
use meeting_copilot_domain::model::{ChannelKind, ChannelPreference, Platform, SessionState};
use meeting_copilot_domain::vo::{MonoMillis, SessionId, WallClock};
use support::ManualClock;

#[derive(Default)]
struct RecordingAsr {
    intents: Mutex<Vec<AsrIntent>>,
}

impl AsrDeliverySink for RecordingAsr {
    fn deliver(&self, intent: &AsrIntent) -> Result<(), CapturePortError> {
        self.intents.lock().expect("lock").push(intent.clone());
        Ok(())
    }

    fn current_task(&self) -> Option<meeting_copilot_domain::model::llm::LlmTask> {
        None
    }
}

struct NoopSwitch;

impl ChannelSwitchLog for NoopSwitch {
    fn append(&self, _record: &ChannelSwitchRecord) -> Result<(), SwitchLogError> {
        Ok(())
    }
}

fn view(session: &str) -> SessionContextView {
    SessionContextView {
        session: SessionId::new(session).expect("session"),
        platform: Platform::TencentMeeting,
        channel_preference: ChannelPreference::SubtitleFirst,
        session_state: SessionState::Listening,
    }
}

fn observation(session: &SessionId, at: u64, payload: ObservationPayload) -> ChannelObservation {
    ChannelObservation {
        session: Some(session.clone()),
        platform: Platform::TencentMeeting,
        kind: ChannelKind::Subtitle,
        at: MonoMillis::from_millis(at),
        payload,
    }
}

fn probe(readable: bool) -> ObservationPayload {
    ObservationPayload::Probe {
        readable,
        version_supported: true,
        ocr_only: false,
    }
}

fn service() -> (CaptureChannelService, Arc<RecordingAsr>) {
    let clock = Arc::new(ManualClock::new(WallClock::from_unix_millis(0)));
    let asr = Arc::new(RecordingAsr::default());
    let service = CaptureChannelService::with_ports(
        clock,
        Arc::new(NoopSwitch),
        Arc::new(NoopGapLog),
        Arc::new(NoopStatusSink),
        asr.clone(),
        CapturePolicy::default(),
    );
    (service, asr)
}

fn produce_gap(service: &CaptureChannelService, session: &SessionId, start: u64, end: u64) {
    service.observe(&observation(session, start, probe(false)));
    service.observe(&observation(
        session,
        start + 100,
        ObservationPayload::UserRetry,
    ));
    service.observe(&observation(session, end, probe(true)));
}

/// 门限口径收紧（D12）：缺口需 >=1000ms 才投递回补意图。
#[test]
fn gap_end_delivers_gap_backfill_intent() {
    let (service, asr) = service();
    let context = view("s1");
    service.attach_session(context.clone()).expect("attach");
    service.observe(&observation(&context.session, 100, probe(true)));
    produce_gap(&service, &context.session, 200, 1_310);

    let intents = asr.intents.lock().expect("lock");
    assert_eq!(intents.len(), 1);
    assert_eq!(intents[0].kind, AsrIntentKind::GapBackfill);
    assert_eq!(intents[0].window_start.as_millis(), 200);
    assert_eq!(intents[0].window_end.as_millis(), 1_310);
    assert_eq!(intents[0].chunk_count, 3);
}

#[test]
fn second_gap_within_ten_seconds_is_budget_suppressed() {
    let (service, asr) = service();
    let context = view("s1");
    service.attach_session(context.clone()).expect("attach");
    service.observe(&observation(&context.session, 100, probe(true)));
    produce_gap(&service, &context.session, 200, 1_310);
    produce_gap(&service, &context.session, 1_400, 2_510);

    let intents = asr.intents.lock().expect("lock");
    assert_eq!(intents.len(), 1);
}
