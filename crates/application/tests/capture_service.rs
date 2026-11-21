mod support;

use std::sync::{Arc, Mutex};

use meeting_copilot_application::capture::{
    ActiveChannel, CaptureChannelService, CapturePolicy, ChannelObservation, ChannelSwitchLog,
    ChannelSwitchRecord, ObservationPayload, SessionContextView, SwitchLogError,
};
use meeting_copilot_domain::model::{ChannelKind, ChannelPreference, Platform, SessionState};
use meeting_copilot_domain::vo::{MonoMillis, SessionId, WallClock};
use support::ManualClock;

#[derive(Default)]
struct RecordingLog {
    records: Mutex<Vec<ChannelSwitchRecord>>,
}

impl ChannelSwitchLog for RecordingLog {
    fn append(&self, record: &ChannelSwitchRecord) -> Result<(), SwitchLogError> {
        self.records.lock().expect("lock").push(record.clone());
        Ok(())
    }
}

fn view(session: &str, platform: Platform) -> SessionContextView {
    SessionContextView {
        session: SessionId::new(session).expect("session"),
        platform,
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

fn observation(
    session: &SessionId,
    platform: Platform,
    kind: ChannelKind,
    at: u64,
    payload: ObservationPayload,
) -> ChannelObservation {
    ChannelObservation {
        session: Some(session.clone()),
        platform,
        kind,
        at: MonoMillis::from_millis(at),
        payload,
    }
}

fn service() -> (CaptureChannelService, Arc<RecordingLog>, Arc<ManualClock>) {
    let clock = Arc::new(ManualClock::new(WallClock::from_unix_millis(0)));
    let log = Arc::new(RecordingLog::default());
    let service = CaptureChannelService::new(clock.clone(), log.clone(), CapturePolicy::default());
    (service, log, clock)
}

#[test]
fn attach_and_available_selects_subtitle() {
    let (service, log, _clock) = service();
    let context = view("s1", Platform::TencentMeeting);
    service.attach_session(context.clone()).expect("attach");
    let outcome = service.observe(&observation(
        &context.session,
        context.platform,
        ChannelKind::Subtitle,
        100,
        probe(true),
    ));
    assert_eq!(
        outcome,
        meeting_copilot_application::capture::ObservationOutcome::Applied
    );
    assert_eq!(
        service.active_channel(&context.session),
        ActiveChannel::Subtitle
    );
    assert_eq!(service.snapshot().entries.len(), 2);
    let records = log.records.lock().expect("lock");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].from, ActiveChannel::None);
    assert_eq!(records[0].to, ActiveChannel::Subtitle);
}

#[test]
fn subtitle_unavailable_falls_back_to_audio() {
    let (service, log, _clock) = service();
    let context = view("s1", Platform::TencentMeeting);
    service.attach_session(context.clone()).expect("attach");
    service.observe(&observation(
        &context.session,
        context.platform,
        ChannelKind::Subtitle,
        100,
        probe(true),
    ));
    service.observe(&observation(
        &context.session,
        context.platform,
        ChannelKind::Audio,
        110,
        probe(true),
    ));
    assert_eq!(
        service.active_channel(&context.session),
        ActiveChannel::Subtitle
    );

    service.observe(&observation(
        &context.session,
        context.platform,
        ChannelKind::Subtitle,
        120,
        probe(false),
    ));
    assert_eq!(
        service.active_channel(&context.session),
        ActiveChannel::Audio
    );
    let records = log.records.lock().expect("lock");
    assert_eq!(records.last().expect("record").to, ActiveChannel::Audio);
}

#[test]
fn detach_clears_active_and_snapshot() {
    let (service, _log, _clock) = service();
    let context = view("s1", Platform::Feishu);
    service.attach_session(context.clone()).expect("attach");
    service.observe(&observation(
        &context.session,
        context.platform,
        ChannelKind::Subtitle,
        100,
        probe(true),
    ));
    service.detach_session(&context.session);
    assert_eq!(
        service.active_channel(&context.session),
        ActiveChannel::None
    );
    assert!(service.snapshot().entries.is_empty());
}

#[test]
fn two_sessions_are_independent() {
    let (service, _log, _clock) = service();
    let first = view("s1", Platform::TencentMeeting);
    let second = view("s2", Platform::Zoom);
    service.attach_session(first.clone()).expect("attach");
    service.attach_session(second.clone()).expect("attach");

    service.observe(&observation(
        &first.session,
        first.platform,
        ChannelKind::Subtitle,
        100,
        probe(true),
    ));
    service.observe(&observation(
        &second.session,
        second.platform,
        ChannelKind::Subtitle,
        100,
        probe(false),
    ));

    assert_eq!(
        service.active_channel(&first.session),
        ActiveChannel::Subtitle
    );
    assert_eq!(service.active_channel(&second.session), ActiveChannel::None);
    assert_eq!(service.snapshot().entries.len(), 4);
}
#[test]
fn advance_falls_back_after_five_seconds() {
    let (service, _log, _clock) = service();
    let context = view("s1", Platform::TencentMeeting);
    service.attach_session(context.clone()).expect("attach");
    service.observe(&observation(
        &context.session,
        context.platform,
        ChannelKind::Subtitle,
        100,
        probe(true),
    ));
    service.observe(&observation(
        &context.session,
        context.platform,
        ChannelKind::Audio,
        110,
        probe(true),
    ));
    assert_eq!(
        service.active_channel(&context.session),
        ActiveChannel::Subtitle
    );

    assert_eq!(service.advance_to(MonoMillis::from_millis(5_099)), 0);
    assert_eq!(
        service.active_channel(&context.session),
        ActiveChannel::Subtitle
    );
    assert_eq!(service.advance_to(MonoMillis::from_millis(5_100)), 1);
    assert_eq!(
        service.active_channel(&context.session),
        ActiveChannel::Audio
    );
}
#[test]
fn gap_recovery_does_not_redeliver_old_points() {
    let (service, _log, _clock) = service();
    let context = view("s1", Platform::TencentMeeting);
    service.attach_session(context.clone()).expect("attach");
    service.observe(&observation(
        &context.session,
        context.platform,
        ChannelKind::Subtitle,
        100,
        probe(true),
    ));
    assert!(service.record_point(&context.session, "p1", MonoMillis::from_millis(110)));

    service.observe(&observation(
        &context.session,
        context.platform,
        ChannelKind::Subtitle,
        200,
        probe(false),
    ));
    assert_eq!(
        service.active_channel(&context.session),
        ActiveChannel::None
    );

    service.observe(&observation(
        &context.session,
        context.platform,
        ChannelKind::Subtitle,
        300,
        ObservationPayload::UserRetry,
    ));
    service.observe(&observation(
        &context.session,
        context.platform,
        ChannelKind::Subtitle,
        310,
        probe(true),
    ));
    assert_eq!(
        service.active_channel(&context.session),
        ActiveChannel::Subtitle
    );

    assert!(!service.record_point(&context.session, "p1", MonoMillis::from_millis(320)));
    assert!(service.record_point(&context.session, "p2", MonoMillis::from_millis(321)));
    assert_eq!(service.delivered_points(&context.session), 2);
}
#[test]
fn session_state_is_not_changed_by_gap() {
    let (service, _log, _clock) = service();
    let context = view("s1", Platform::TencentMeeting);
    service.attach_session(context.clone()).expect("attach");
    assert_eq!(
        service.session_state(&context.session),
        Some(SessionState::Listening)
    );

    service.observe(&observation(
        &context.session,
        context.platform,
        ChannelKind::Subtitle,
        100,
        probe(true),
    ));
    service.observe(&observation(
        &context.session,
        context.platform,
        ChannelKind::Subtitle,
        200,
        probe(false),
    ));
    assert_eq!(
        service.session_state(&context.session),
        Some(SessionState::Listening)
    );

    service.observe(&observation(
        &context.session,
        context.platform,
        ChannelKind::Subtitle,
        300,
        ObservationPayload::UserRetry,
    ));
    service.observe(&observation(
        &context.session,
        context.platform,
        ChannelKind::Subtitle,
        310,
        probe(true),
    ));
    assert_eq!(
        service.session_state(&context.session),
        Some(SessionState::Listening)
    );
}
