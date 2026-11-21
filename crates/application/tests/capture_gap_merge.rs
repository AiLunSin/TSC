mod support;

use std::sync::{Arc, Mutex};

use meeting_copilot_application::capture::{
    CaptureChannelService, CaptureGapRecord, CapturePolicy, ChannelGapLog, ChannelObservation,
    ChannelSwitchLog, ChannelSwitchRecord, CountingDiagnostics, DiagnosticCounter, NoopAsrDelivery,
    NoopInsight, NoopStatusSink, ObservationPayload, SessionContextView, SwitchLogError,
};
use meeting_copilot_domain::model::{ChannelKind, ChannelPreference, Platform, SessionState};
use meeting_copilot_domain::vo::{MonoMillis, SessionId, WallClock};
use support::ManualClock;

#[derive(Default)]
struct RecordingGap {
    records: Mutex<Vec<CaptureGapRecord>>,
}

impl ChannelGapLog for RecordingGap {
    fn append(
        &self,
        record: &CaptureGapRecord,
    ) -> Result<(), meeting_copilot_application::capture::CapturePortError> {
        self.records.lock().expect("lock").push(record.clone());
        Ok(())
    }
}

struct NoopSwitch;
impl ChannelSwitchLog for NoopSwitch {
    fn append(&self, _record: &ChannelSwitchRecord) -> Result<(), SwitchLogError> {
        Ok(())
    }
}

fn view() -> SessionContextView {
    SessionContextView {
        session: SessionId::new("s1").expect("session"),
        platform: Platform::TencentMeeting,
        channel_preference: ChannelPreference::SubtitleFirst,
        session_state: SessionState::Listening,
    }
}

fn probe(readable: bool) -> ObservationPayload {
    ObservationPayload::Probe {
        readable,
        version_supported: true,
        ocr_only: false,
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

fn service() -> (CaptureChannelService, Arc<RecordingGap>) {
    service_with(CapturePolicy::default())
}

fn service_with(policy: CapturePolicy) -> (CaptureChannelService, Arc<RecordingGap>) {
    let clock = Arc::new(ManualClock::new(WallClock::from_unix_millis(0)));
    let gaps = Arc::new(RecordingGap::default());
    let service = CaptureChannelService::with_offline_ports(
        clock,
        Arc::new(NoopSwitch),
        gaps.clone(),
        Arc::new(NoopStatusSink),
        Arc::new(NoopAsrDelivery),
        Arc::new(NoopInsight),
        Arc::new(CountingDiagnostics::new()),
        policy,
    );
    (service, gaps)
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

/// AC-B2-OC-C01/C03（D6）：保留原用例名与"合并先于落盘"语义，
/// 改为显式 `gap_min_ms=400`，两段各 300ms 合并后 700ms 达到门限。
#[test]
fn adjacent_gaps_merge_before_write() {
    let policy = CapturePolicy {
        gap_min_ms: 400,
        ..Default::default()
    };
    let (service, gaps) = service_with(policy);
    let context = view();
    service.attach_session(context.clone()).expect("attach");
    service.observe(&observation(&context.session, 100, probe(true)));
    produce_gap(&service, &context.session, 200, 500);
    produce_gap(&service, &context.session, 600, 900);
    service.flush_gaps(&context.session);

    let records = gaps.records.lock().expect("lock");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].start.as_millis(), 200);
    assert_eq!(records[0].end.as_millis(), 900);
}

/// AC-B2-OC-C01（D6）：同一序列在默认 1000ms 门限下不得落盘。
#[test]
fn default_policy_drops_short_merged_gap() {
    let (service, gaps) = service();
    let context = view();
    service.attach_session(context.clone()).expect("attach");
    service.observe(&observation(&context.session, 100, probe(true)));
    produce_gap(&service, &context.session, 200, 500);
    produce_gap(&service, &context.session, 600, 900);
    service.flush_gaps(&context.session);

    assert!(gaps.records.lock().expect("lock").is_empty());
    assert_eq!(
        service
            .diagnostics()
            .get(DiagnosticCounter::ShortGapDropped),
        2
    );
}

/// AC-B2-OC-C01（D7）：间隔由 490ms 调整为 1001ms，两段各 400ms 分别落盘。
#[test]
fn non_adjacent_gaps_remain_two_records() {
    let policy = CapturePolicy {
        gap_min_ms: 400,
        ..Default::default()
    };
    let (service, gaps) = service_with(policy);
    let context = view();
    service.attach_session(context.clone()).expect("attach");
    service.observe(&observation(&context.session, 100, probe(true)));
    produce_gap(&service, &context.session, 200, 600);
    produce_gap(&service, &context.session, 1_601, 2_001);
    service.flush_gaps(&context.session);

    let records = gaps.records.lock().expect("lock");
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].start.as_millis(), 200);
    assert_eq!(records[1].start.as_millis(), 1_601);
}

/// AC-B2-OC-C01：短缺口只计数、不落盘。
#[test]
fn short_gap_only_counted_not_persisted() {
    let (service, gaps) = service();
    let context = view();
    service.attach_session(context.clone()).expect("attach");
    service.observe(&observation(&context.session, 100, probe(true)));
    produce_gap(&service, &context.session, 200, 700);
    service.flush_gaps(&context.session);

    assert!(gaps.records.lock().expect("lock").is_empty());
    assert_eq!(
        service
            .diagnostics()
            .get(DiagnosticCounter::ShortGapDropped),
        1
    );
}
