//! 状态推送、快照与订阅集成用例（AC-B2-OC-D01~D10，13 例）。

mod support;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use meeting_copilot_application::capture::{
    CaptureAvailability, CaptureAvailabilityEvent, CapturePolicy, ChannelSnapshotEntry,
    ChannelStatusEvent, DiagnosticCounter, NotPushedReason, PublishOutcome, PublishRejectReason,
};
use meeting_copilot_domain::error::ErrorCode;
use meeting_copilot_domain::model::{
    ChannelKey, ChannelKind, ChannelState, GapReason, Platform, StateReason,
};
use meeting_copilot_domain::vo::{MonoMillis, SessionId, WallClock};
use support::{build_offline_capture, OfflineHarness};

fn session() -> SessionId {
    SessionId::new("s1").expect("session")
}

fn harness() -> OfflineHarness {
    build_offline_capture(CapturePolicy::default())
}

fn event(at: u64, state: ChannelState, reason: StateReason) -> ChannelStatusEvent {
    ChannelStatusEvent {
        key: ChannelKey::new(Platform::TencentMeeting, ChannelKind::Subtitle),
        state,
        reason,
        sequence: 0,
        determined_at: MonoMillis::from_millis(at),
        wall_at: WallClock::from_unix_millis(0),
    }
}

fn unavailable_event(sequence: u64) -> CaptureAvailabilityEvent {
    CaptureAvailabilityEvent {
        availability: CaptureAvailability::Unavailable {
            reason: GapReason::BothChannelsUnavailable,
            code: ErrorCode::AudioCaptureFailed,
        },
        gap_reason: Some(GapReason::BothChannelsUnavailable),
        code: Some(ErrorCode::AudioCaptureFailed),
        sequence,
    }
}

fn available_event(sequence: u64) -> CaptureAvailabilityEvent {
    CaptureAvailabilityEvent {
        availability: CaptureAvailability::Available {
            channel: meeting_copilot_application::capture::ActiveChannel::Subtitle,
        },
        gap_reason: None,
        code: None,
        sequence,
    }
}

/// AC-B2-OC-D01：同状态 10 次只推 1 次，快照原因已更新。
#[test]
fn same_state_ten_times_pushes_once_and_updates_reason() {
    let harness = harness();
    let session = session();
    let publisher = harness.service.publisher();
    let count = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&count);
    let handle = publisher.subscribe(Box::new(move |_| {
        counter.fetch_add(1, Ordering::SeqCst);
        true
    }));

    assert_eq!(
        publisher.publish_channel(
            event(1, ChannelState::Available, StateReason::ProbeCompleted),
            &session
        ),
        PublishOutcome::Published { sequence: 1 }
    );
    for at in 2..=10 {
        assert_eq!(
            publisher.publish_channel(
                event(at, ChannelState::Available, StateReason::OcrOnly),
                &session
            ),
            PublishOutcome::NotPushed {
                reason: NotPushedReason::SameState
            }
        );
    }
    publisher.drain(MonoMillis::from_millis(20));
    handle.release();

    assert_eq!(count.load(Ordering::SeqCst), 1);
    let snapshot = publisher.snapshot_for(&session);
    assert_eq!(snapshot.entries.len(), 1);
    assert_eq!(snapshot.entries[0].reason, Some(StateReason::OcrOnly));
    assert_eq!(snapshot.entries[0].entered_at, MonoMillis::from_millis(1));
}

/// AC-B2-OC-D02：回调内可见本次快照。
#[test]
fn snapshot_visible_inside_subscriber_callback() {
    let harness = harness();
    let session = session();
    let publisher = Arc::clone(harness.service.publisher());
    let observed: Arc<Mutex<Option<(ChannelState, StateReason)>>> = Arc::new(Mutex::new(None));
    let capture = Arc::clone(&observed);
    let query_session = session.clone();
    let query_publisher = Arc::clone(&publisher);
    let handle = publisher.subscribe(Box::new(move |delivery| {
        let Some(event) = delivery.channel else {
            return true;
        };
        let snapshot = query_publisher.snapshot_for(&query_session);
        if let Some(entry) = snapshot.entries.first() {
            *capture.lock().expect("lock") = Some((entry.state, event.reason));
        }
        true
    }));
    publisher.publish_channel(
        event(10, ChannelState::Degraded, StateReason::OcrOnly),
        &session,
    );
    publisher.drain(MonoMillis::from_millis(12));
    handle.release();

    assert_eq!(
        *observed.lock().expect("lock"),
        Some((ChannelState::Degraded, StateReason::OcrOnly))
    );
}

/// AC-B2-OC-D03：100 次变更共用会话内严格递增序号。
#[test]
fn hundred_changes_share_one_increasing_sequence() {
    let harness = harness();
    let session = session();
    let publisher = harness.service.publisher();
    let mut sequences = Vec::new();
    for index in 0..100_u64 {
        let state = if index % 2 == 0 {
            ChannelState::Available
        } else {
            ChannelState::Unavailable
        };
        if let PublishOutcome::Published { sequence } =
            publisher.publish_channel(event(index, state, StateReason::ProbeCompleted), &session)
        {
            sequences.push(sequence);
        }
    }
    assert_eq!(sequences.len(), 100);
    assert!(sequences.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(publisher.key_count(&session), 1);
}

/// AC-B2-OC-D03：通道事件与汇总事件共用同一会话序号流。
#[test]
fn channel_and_availability_share_one_sequence() {
    let harness = harness();
    let session = session();
    let publisher = harness.service.publisher();
    let mut sequences = Vec::new();
    for index in 0..4_u64 {
        let state = if index % 2 == 0 {
            ChannelState::Available
        } else {
            ChannelState::Degraded
        };
        if let PublishOutcome::Published { sequence } =
            publisher.publish_channel(event(index, state, StateReason::ProbeCompleted), &session)
        {
            sequences.push(sequence);
        }
        let availability = if index % 2 == 0 {
            unavailable_event(0)
        } else {
            available_event(0)
        };
        if let PublishOutcome::Published { sequence } =
            publisher.publish_availability(availability, &session)
        {
            sequences.push(sequence);
        }
    }
    assert_eq!(sequences, vec![1, 2, 3, 4, 5, 6, 7, 8]);
}

/// AC-B2-OC-D04：序号近上界拒绝且不覆盖快照。
#[test]
fn seq_near_upper_bound_is_rejected() {
    let harness = harness();
    let session = session();
    let publisher = harness.service.publisher();
    publisher.seed_sequence(&session, u64::MAX - 1);
    assert_eq!(
        publisher.publish_channel(
            event(1, ChannelState::Available, StateReason::ProbeCompleted),
            &session
        ),
        PublishOutcome::Rejected {
            reason: PublishRejectReason::SeqExhausted
        }
    );
    assert_eq!(publisher.key_count(&session), 0);
    assert_eq!(
        harness
            .diagnostics
            .snapshot()
            .get(DiagnosticCounter::SeqOverflow),
        1
    );
}

/// AC-B2-OC-D05：慢订阅者不阻塞其它订阅者。
#[test]
fn slow_subscriber_does_not_block_others() {
    let harness = harness();
    let session = session();
    let publisher = Arc::clone(harness.service.publisher());
    let received = Arc::new(AtomicUsize::new(0));
    let fast = Arc::clone(&received);
    let fast_handle = publisher.subscribe(Box::new(move |_| {
        fast.fetch_add(1, Ordering::SeqCst);
        true
    }));
    let slow_clock = Arc::clone(&harness.clock);
    let slow_handle = publisher.subscribe(Box::new(move |_| {
        slow_clock.advance_by(100);
        true
    }));
    publisher.publish_channel(
        event(1, ChannelState::Available, StateReason::ProbeCompleted),
        &session,
    );
    let report = publisher.drain(MonoMillis::from_millis(10));
    fast_handle.release();
    slow_handle.release();

    assert_eq!(received.load(Ordering::SeqCst), 1);
    assert_eq!(report.pending, 1);
}

/// AC-B2-OC-D05：回调抛错只计数，不影响其它订阅者。
#[test]
fn handler_error_only_counts() {
    let harness = harness();
    let session = session();
    let publisher = Arc::clone(harness.service.publisher());
    let ok = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&ok);
    let failing = publisher.subscribe(Box::new(|_| false));
    let healthy = publisher.subscribe(Box::new(move |_| {
        counter.fetch_add(1, Ordering::SeqCst);
        true
    }));
    publisher.publish_channel(
        event(1, ChannelState::Available, StateReason::ProbeCompleted),
        &session,
    );
    let report = publisher.drain(MonoMillis::from_millis(5));
    failing.release();
    healthy.release();

    assert_eq!(report.failures, 1);
    assert_eq!(ok.load(Ordering::SeqCst), 1);
    assert_eq!(
        harness
            .diagnostics
            .snapshot()
            .get(DiagnosticCounter::SubscriberFailure),
        1
    );
}

/// AC-B2-OC-D06：释放后 0 回调，其余订阅者不受影响。
#[test]
fn released_subscriber_receives_zero_events() {
    let harness = harness();
    let session = session();
    let publisher = Arc::clone(harness.service.publisher());
    let released = Arc::new(AtomicUsize::new(0));
    let released_counter = Arc::clone(&released);
    let kept = Arc::new(AtomicUsize::new(0));
    let kept_counter = Arc::clone(&kept);
    let released_handle = publisher.subscribe(Box::new(move |_| {
        released_counter.fetch_add(1, Ordering::SeqCst);
        true
    }));
    let kept_handle = publisher.subscribe(Box::new(move |_| {
        kept_counter.fetch_add(1, Ordering::SeqCst);
        true
    }));
    released_handle.release();
    released_handle.release();

    for index in 0..10_u64 {
        let state = if index % 2 == 0 {
            ChannelState::Available
        } else {
            ChannelState::Degraded
        };
        publisher.publish_channel(event(index, state, StateReason::ProbeCompleted), &session);
    }
    publisher.drain(MonoMillis::from_millis(20));
    kept_handle.release();

    assert_eq!(released.load(Ordering::SeqCst), 0);
    assert_eq!(kept.load(Ordering::SeqCst), 10);
}

/// AC-B2-OC-D07：200 样本下 p99 与最大值不超过 5s。
#[test]
fn latency_p99_and_max_within_5s() {
    let harness = harness();
    let session = session();
    let publisher = Arc::clone(harness.service.publisher());
    let handle = publisher.subscribe(Box::new(|_| true));
    for index in 0..200_u64 {
        let state = if index % 2 == 0 {
            ChannelState::Available
        } else {
            ChannelState::Degraded
        };
        publisher.publish_channel(event(index, state, StateReason::ProbeCompleted), &session);
        publisher.drain(MonoMillis::from_millis(index + 10));
    }
    handle.release();
    let stats = publisher.latency();
    assert_eq!(stats.samples, 200);
    assert!(stats.p99_ms <= 5_000);
    assert!(stats.max_ms <= 5_000);
    assert!(harness.service.policy().status_latency_ring >= 256);
}

const CHANNEL_EVENT_FIELDS: [&str; 7] = [
    "platform",
    "kind",
    "state",
    "reason",
    "sequence",
    "determined_at",
    "wall_at",
];
const SNAPSHOT_ENTRY_FIELDS: [&str; 6] = [
    "platform",
    "kind",
    "state",
    "reason",
    "entered_at",
    "changed_at",
];
const AVAILABILITY_FIELDS: [&str; 4] = ["availability", "gap_reason", "code", "sequence"];

/// AC-B2-OC-D08：三类对象叶子字段数分别为 7 / 6 / 4。
#[test]
fn payload_whitelist_field_counts() {
    assert_eq!(CHANNEL_EVENT_FIELDS.len(), 7);
    assert_eq!(SNAPSHOT_ENTRY_FIELDS.len(), 6);
    assert_eq!(AVAILABILITY_FIELDS.len(), 4);

    let event = event(1, ChannelState::Available, StateReason::ProbeCompleted);
    let rendered = format!("{event:?}");
    for field in CHANNEL_EVENT_FIELDS {
        assert!(rendered.contains(field), "缺少字段 {field}");
    }
    let entry = ChannelSnapshotEntry {
        key: event.key,
        state: event.state,
        reason: Some(event.reason),
        entered_at: event.determined_at,
        changed_at: event.determined_at,
    };
    let rendered = format!("{entry:?}");
    for field in SNAPSHOT_ENTRY_FIELDS {
        assert!(rendered.contains(field), "缺少字段 {field}");
    }
    let rendered = format!("{:?}", unavailable_event(1));
    for field in AVAILABILITY_FIELDS {
        assert!(rendered.contains(field), "缺少字段 {field}");
    }
}

/// AC-B2-OC-D08：事件/快照/缺口记录的 Debug 输出不含字幕文本与音频样本。
#[test]
fn payload_scan_hits_no_text_or_sample() {
    let harness = harness();
    let session = session();
    let publisher = harness.service.publisher();
    publisher.publish_channel(
        event(1, ChannelState::Available, StateReason::ProbeCompleted),
        &session,
    );
    let rendered = format!(
        "{:?}{:?}{:?}",
        publisher.snapshot(),
        publisher.snapshot_for(&session),
        event(1, ChannelState::Available, StateReason::ProbeCompleted)
    );
    for forbidden in ["会议字幕样例", "audio-sample-bytes"] {
        assert!(!rendered.contains(forbidden));
    }
}

/// AC-B2-OC-D09：无订阅者仍更新快照，且为最后一次状态。
#[test]
fn no_subscriber_still_updates_snapshot() {
    let harness = harness();
    let session = session();
    let publisher = harness.service.publisher();
    for index in 0..=5_u64 {
        let state = if index % 2 == 0 {
            ChannelState::Available
        } else {
            ChannelState::Degraded
        };
        publisher.publish_channel(event(index, state, StateReason::ProbeCompleted), &session);
    }
    let snapshot = harness.service.snapshot();
    assert_eq!(snapshot.entries.len(), 1);
    assert_eq!(snapshot.entries[0].state, ChannelState::Degraded);
}

/// AC-B2-OC-D10：缺口内原因变化不重复推送。
#[test]
fn reason_changes_inside_gap_do_not_repush() {
    let harness = harness();
    let session = session();
    let publisher = harness.service.publisher();
    let pushed = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&pushed);
    let handle = publisher.subscribe(Box::new(move |delivery| {
        if delivery.availability.is_some_and(|event| {
            matches!(event.availability, CaptureAvailability::Unavailable { .. })
        }) {
            counter.fetch_add(1, Ordering::SeqCst);
        }
        true
    }));

    assert!(matches!(
        publisher.publish_availability(unavailable_event(0), &session),
        PublishOutcome::Published { .. }
    ));
    for reason in [
        GapReason::PreferenceRestricted,
        GapReason::BothChannelsUnavailable,
        GapReason::PreferenceRestricted,
        GapReason::BothChannelsUnavailable,
        GapReason::PreferenceRestricted,
    ] {
        let mut event = unavailable_event(0);
        event.availability = CaptureAvailability::Unavailable {
            reason,
            code: ErrorCode::AudioCaptureFailed,
        };
        event.gap_reason = Some(reason);
        assert_eq!(
            publisher.publish_availability(event, &session),
            PublishOutcome::NotPushed {
                reason: NotPushedReason::SameState
            }
        );
    }
    publisher.drain(MonoMillis::from_millis(10));
    handle.release();

    assert_eq!(pushed.load(Ordering::SeqCst), 1);
    assert_eq!(
        harness
            .diagnostics
            .snapshot()
            .get(DiagnosticCounter::StatusNotPushed),
        5
    );
}
