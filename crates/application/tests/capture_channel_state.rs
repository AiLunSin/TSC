mod support;

use std::sync::{Arc, Mutex};

use meeting_copilot_application::capture::channel_state::CaptureAuditSink;
use meeting_copilot_application::capture::{
    ChannelObservation, ChannelStateRegistry, ObservationOutcome, ObservationPayload, RejectReason,
    SessionContextView,
};
use meeting_copilot_domain::model::{
    ChannelKind, ChannelPreference, ChannelState, Platform, SessionState,
};
use meeting_copilot_domain::vo::{MonoMillis, SessionId, WallClock};
use support::ManualClock;

fn view(session: &str, platform: Platform) -> SessionContextView {
    SessionContextView {
        session: SessionId::new(session).expect("session"),
        platform,
        channel_preference: ChannelPreference::SubtitleFirst,
        session_state: SessionState::Listening,
    }
}

fn probe_for(state: ChannelState) -> ObservationPayload {
    match state {
        ChannelState::Available => ObservationPayload::Probe {
            readable: true,
            version_supported: true,
            ocr_only: false,
        },
        ChannelState::Degraded => ObservationPayload::Probe {
            readable: true,
            version_supported: true,
            ocr_only: true,
        },
        ChannelState::Unavailable | ChannelState::Probing => ObservationPayload::Probe {
            readable: false,
            version_supported: false,
            ocr_only: false,
        },
    }
}

fn observation(
    session: &SessionId,
    platform: Platform,
    kind: ChannelKind,
    at: u64,
    state: ChannelState,
) -> ChannelObservation {
    ChannelObservation {
        session: Some(session.clone()),
        platform,
        kind,
        at: MonoMillis::from_millis(at),
        payload: probe_for(state),
    }
}

#[test]
fn attach_creates_two_probings_at_attach_time() {
    let clock = Arc::new(ManualClock::new(WallClock::from_unix_millis(0)));
    let mut registry = ChannelStateRegistry::new(clock.clone());
    let context = view("s1", Platform::TencentMeeting);
    registry.attach_session(&context).expect("attach");

    for kind in [ChannelKind::Subtitle, ChannelKind::Audio] {
        let instance = registry.instance(&context.session, kind).expect("instance");
        assert_eq!(instance.state, ChannelState::Probing);
        assert_eq!(instance.entered_at, MonoMillis::ZERO);
        assert_eq!(instance.changed_at, MonoMillis::ZERO);
        assert_eq!(instance.reason, None);
    }
    assert_eq!(registry.snapshot().entries.len(), 2);
}

#[test]
fn duplicate_attach_reuses_instances_and_counts() {
    let clock = Arc::new(ManualClock::new(WallClock::from_unix_millis(0)));
    let mut registry = ChannelStateRegistry::new(clock.clone());
    let context = view("s1", Platform::Feishu);
    registry.attach_session(&context).expect("attach");
    clock.advance_by(500);
    registry.attach_session(&context).expect("duplicate");

    assert_eq!(registry.duplicate_instance(), 1);
    let instance = registry
        .instance(&context.session, ChannelKind::Subtitle)
        .expect("instance");
    assert_eq!(instance.entered_at, MonoMillis::ZERO);
    assert_eq!(instance.changed_at, MonoMillis::ZERO);
}

#[test]
fn six_keys_are_independent() {
    let clock = Arc::new(ManualClock::new(WallClock::from_unix_millis(0)));
    let mut registry = ChannelStateRegistry::new(clock.clone());
    let contexts = [
        view("s1", Platform::TencentMeeting),
        view("s2", Platform::Feishu),
        view("s3", Platform::Zoom),
    ];
    for context in &contexts {
        registry.attach_session(context).expect("attach");
    }

    let states = [
        ChannelState::Available,
        ChannelState::Degraded,
        ChannelState::Unavailable,
        ChannelState::Available,
        ChannelState::Degraded,
        ChannelState::Available,
    ];
    let mut index = 0;
    for context in &contexts {
        for kind in [ChannelKind::Subtitle, ChannelKind::Audio] {
            let outcome = registry.observe(&observation(
                &context.session,
                context.platform,
                kind,
                100,
                states[index],
            ));
            assert_eq!(outcome, ObservationOutcome::Applied);
            index += 1;
        }
    }

    index = 0;
    for context in &contexts {
        for kind in [ChannelKind::Subtitle, ChannelKind::Audio] {
            assert_eq!(registry.state(&context.session, kind), Some(states[index]));
            index += 1;
        }
    }
}

#[test]
fn foreign_platform_and_clock_regression_are_rejected_without_state_change() {
    let clock = Arc::new(ManualClock::new(WallClock::from_unix_millis(0)));
    let mut registry = ChannelStateRegistry::new(clock.clone());
    let context = view("s1", Platform::TencentMeeting);
    registry.attach_session(&context).expect("attach");

    let foreign = observation(
        &context.session,
        Platform::Feishu,
        ChannelKind::Subtitle,
        100,
        ChannelState::Available,
    );
    assert_eq!(
        registry.observe(&foreign),
        ObservationOutcome::Rejected {
            reason: RejectReason::ForeignPlatform
        }
    );
    assert_eq!(registry.cross_platform_rejected(), 1);
    assert_eq!(
        registry.state(&context.session, ChannelKind::Subtitle),
        Some(ChannelState::Probing)
    );

    let applied = observation(
        &context.session,
        Platform::TencentMeeting,
        ChannelKind::Subtitle,
        100,
        ChannelState::Available,
    );
    assert_eq!(registry.observe(&applied), ObservationOutcome::Applied);
    let regression = observation(
        &context.session,
        Platform::TencentMeeting,
        ChannelKind::Subtitle,
        50,
        ChannelState::Available,
    );
    assert_eq!(
        registry.observe(&regression),
        ObservationOutcome::Rejected {
            reason: RejectReason::ClockRegression
        }
    );
    assert_eq!(registry.clock_regression(), 1);
    assert_eq!(
        registry.state(&context.session, ChannelKind::Subtitle),
        Some(ChannelState::Available)
    );
}
#[derive(Default)]
struct RecordingAudit {
    events: Mutex<Vec<(&'static str, u64)>>,
}

impl CaptureAuditSink for RecordingAudit {
    fn record_rejection(&self, event_kind: &'static str, count: u64) {
        self.events.lock().expect("lock").push((event_kind, count));
    }
}

fn payload_observation(
    session: Option<SessionId>,
    platform: Platform,
    kind: ChannelKind,
    at: u64,
    payload: ObservationPayload,
) -> ChannelObservation {
    ChannelObservation {
        session,
        platform,
        kind,
        at: MonoMillis::from_millis(at),
        payload,
    }
}

#[test]
fn probe_mapping_four_branches() {
    let clock = Arc::new(ManualClock::new(WallClock::from_unix_millis(0)));
    let mut registry = ChannelStateRegistry::new(clock);
    let cases = [
        (
            "s1",
            ObservationPayload::Probe {
                readable: true,
                version_supported: true,
                ocr_only: false,
            },
            ChannelState::Available,
        ),
        (
            "s2",
            ObservationPayload::Probe {
                readable: true,
                version_supported: true,
                ocr_only: true,
            },
            ChannelState::Degraded,
        ),
        (
            "s3",
            ObservationPayload::Probe {
                readable: true,
                version_supported: false,
                ocr_only: false,
            },
            ChannelState::Degraded,
        ),
        (
            "s4",
            ObservationPayload::Probe {
                readable: false,
                version_supported: false,
                ocr_only: false,
            },
            ChannelState::Unavailable,
        ),
    ];
    for (name, payload, expected) in cases {
        let context = view(name, Platform::TencentMeeting);
        registry.attach_session(&context).expect("attach");
        let outcome = registry.observe(&payload_observation(
            Some(context.session.clone()),
            context.platform,
            ChannelKind::Subtitle,
            100,
            payload,
        ));
        assert_eq!(outcome, ObservationOutcome::Applied, "{name}");
        assert_eq!(
            registry.state(&context.session, ChannelKind::Subtitle),
            Some(expected),
            "{name}"
        );
    }
}

#[test]
fn read_failure_three_times_degrades() {
    let clock = Arc::new(ManualClock::new(WallClock::from_unix_millis(0)));
    let mut registry = ChannelStateRegistry::new(clock);
    let context = view("s1", Platform::TencentMeeting);
    registry.attach_session(&context).expect("attach");
    registry.observe(&payload_observation(
        Some(context.session.clone()),
        context.platform,
        ChannelKind::Subtitle,
        100,
        probe_for(ChannelState::Available),
    ));

    for at in [200, 300] {
        let outcome = registry.observe(&payload_observation(
            Some(context.session.clone()),
            context.platform,
            ChannelKind::Subtitle,
            at,
            ObservationPayload::ReadFailure,
        ));
        assert_eq!(outcome, ObservationOutcome::NoOp);
    }
    let outcome = registry.observe(&payload_observation(
        Some(context.session.clone()),
        context.platform,
        ChannelKind::Subtitle,
        400,
        ObservationPayload::ReadFailure,
    ));
    assert_eq!(outcome, ObservationOutcome::Applied);
    assert_eq!(
        registry.state(&context.session, ChannelKind::Subtitle),
        Some(ChannelState::Degraded)
    );
}

#[test]
fn silence_requires_three_blocks_and_no_vad() {
    let clock = Arc::new(ManualClock::new(WallClock::from_unix_millis(0)));
    let mut registry = ChannelStateRegistry::new(clock);
    let context = view("s1", Platform::TencentMeeting);
    registry.attach_session(&context).expect("attach");
    registry.observe(&payload_observation(
        Some(context.session.clone()),
        context.platform,
        ChannelKind::Subtitle,
        100,
        probe_for(ChannelState::Available),
    ));
    assert_eq!(
        registry.observe(&payload_observation(
            Some(context.session.clone()),
            context.platform,
            ChannelKind::Subtitle,
            110,
            ObservationPayload::Silence {
                blocks: 2,
                vad: false
            },
        )),
        ObservationOutcome::NoOp
    );
    assert_eq!(
        registry.observe(&payload_observation(
            Some(context.session.clone()),
            context.platform,
            ChannelKind::Subtitle,
            120,
            ObservationPayload::Silence {
                blocks: 3,
                vad: true
            },
        )),
        ObservationOutcome::NoOp
    );
    assert_eq!(
        registry.observe(&payload_observation(
            Some(context.session.clone()),
            context.platform,
            ChannelKind::Subtitle,
            130,
            ObservationPayload::Silence {
                blocks: 3,
                vad: false
            },
        )),
        ObservationOutcome::Applied
    );
}

#[test]
fn ocr_strict_majority_triggers_unavailable() {
    let clock = Arc::new(ManualClock::new(WallClock::from_unix_millis(0)));
    let mut registry = ChannelStateRegistry::new(clock);
    let context = view("s1", Platform::TencentMeeting);
    registry.attach_session(&context).expect("attach");
    registry.observe(&payload_observation(
        Some(context.session.clone()),
        context.platform,
        ChannelKind::Subtitle,
        100,
        probe_for(ChannelState::Degraded),
    ));
    assert_eq!(
        registry.observe(&payload_observation(
            Some(context.session.clone()),
            context.platform,
            ChannelKind::Subtitle,
            110,
            ObservationPayload::OcrStats {
                failed: 1,
                total: 2
            },
        )),
        ObservationOutcome::NoOp
    );
    assert_eq!(
        registry.observe(&payload_observation(
            Some(context.session.clone()),
            context.platform,
            ChannelKind::Subtitle,
            120,
            ObservationPayload::OcrStats {
                failed: 2,
                total: 3
            },
        )),
        ObservationOutcome::Applied
    );
    assert_eq!(
        registry.state(&context.session, ChannelKind::Subtitle),
        Some(ChannelState::Unavailable)
    );
}

#[test]
fn audit_records_rejections_without_state_change() {
    let clock = Arc::new(ManualClock::new(WallClock::from_unix_millis(0)));
    let audit = Arc::new(RecordingAudit::default());
    let mut registry = ChannelStateRegistry::with_audit(clock, audit.clone());
    let missing = payload_observation(
        None,
        Platform::TencentMeeting,
        ChannelKind::Subtitle,
        0,
        probe_for(ChannelState::Available),
    );
    assert_eq!(
        registry.observe(&missing),
        ObservationOutcome::Dropped {
            reason: meeting_copilot_application::capture::DropReason::SessionDetached
        }
    );

    let context = view("s1", Platform::TencentMeeting);
    registry.attach_session(&context).expect("attach");
    let foreign = payload_observation(
        Some(context.session.clone()),
        Platform::Feishu,
        ChannelKind::Subtitle,
        100,
        probe_for(ChannelState::Available),
    );
    assert_eq!(
        registry.observe(&foreign),
        ObservationOutcome::Rejected {
            reason: RejectReason::ForeignPlatform
        }
    );
    assert_eq!(
        registry.state(&context.session, ChannelKind::Subtitle),
        Some(ChannelState::Probing)
    );
    let events = audit.events.lock().expect("lock").clone();
    assert!(events.contains(&("missing_session", 1)));
    assert!(events.contains(&("cross_platform", 1)));
}

#[test]
fn snapshot_is_sorted_and_detach_removes_session() {
    let clock = Arc::new(ManualClock::new(WallClock::from_unix_millis(0)));
    let mut registry = ChannelStateRegistry::new(clock);
    let first = view("s1", Platform::TencentMeeting);
    let second = view("s2", Platform::Feishu);
    registry.attach_session(&first).expect("attach");
    registry.attach_session(&second).expect("attach");
    let snapshot = registry.snapshot();
    assert_eq!(snapshot.entries.len(), 4);
    let mut keys: Vec<_> = snapshot.entries.iter().map(|entry| entry.key).collect();
    let sorted = {
        keys.sort();
        keys
    };
    assert!(sorted.windows(2).all(|pair| pair[0] <= pair[1]));
    registry.detach_session(&first.session);
    assert_eq!(registry.snapshot().entries.len(), 2);
}
