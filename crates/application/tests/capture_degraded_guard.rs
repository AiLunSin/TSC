//! 双通道失效保护集成用例（AC-B2-OC-C01~C09，12 例）。

mod support;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use meeting_copilot_application::capture::{
    validate_intervals, CaptureGapRecord, CapturePolicy, ChannelObservation, DiagnosticCounter,
    IntervalViolation, ObservationPayload,
};
use meeting_copilot_domain::model::{ChannelKind, Platform};
use meeting_copilot_domain::vo::{MonoMillis, SessionId};
use support::{build_offline_capture, offline_view, OfflineHarness};

fn obs(
    session: &SessionId,
    at: u64,
    kind: ChannelKind,
    payload: ObservationPayload,
) -> ChannelObservation {
    ChannelObservation {
        session: Some(session.clone()),
        platform: Platform::TencentMeeting,
        kind,
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

fn subtitle_readable(harness: &OfflineHarness, session: &SessionId, at: u64, readable: bool) {
    harness
        .service
        .observe(&obs(session, at, ChannelKind::Subtitle, probe(readable)));
}

fn subtitle_retry(harness: &OfflineHarness, session: &SessionId, at: u64) {
    harness.service.observe(&obs(
        session,
        at,
        ChannelKind::Subtitle,
        ObservationPayload::UserRetry,
    ));
}

fn audio_readable(harness: &OfflineHarness, session: &SessionId, at: u64, readable: bool) {
    harness
        .service
        .observe(&obs(session, at, ChannelKind::Audio, probe(readable)));
}

fn audio_retry(harness: &OfflineHarness, session: &SessionId, at: u64) {
    harness.service.observe(&obs(
        session,
        at,
        ChannelKind::Audio,
        ObservationPayload::UserRetry,
    ));
}

fn recover_subtitle(harness: &OfflineHarness, session: &SessionId, at: u64) {
    subtitle_retry(harness, session, at);
    subtitle_readable(harness, session, at, true);
}

fn attach(harness: &OfflineHarness, session: &SessionId) {
    let _ = session;
    harness
        .service
        .attach_session(offline_view("s1"))
        .expect("attach");
}

/// AC-B2-OC-C01：500ms 失效不落盘。
#[test]
fn short_gap_500ms_is_not_persisted() {
    let harness = build_offline_capture(CapturePolicy::default());
    let session = SessionId::new("s1").expect("session");
    attach(&harness, &session);
    subtitle_readable(&harness, &session, 100, true);
    subtitle_readable(&harness, &session, 200, false);
    recover_subtitle(&harness, &session, 700);

    assert!(harness.gaps.query(&session).is_empty());
    assert_eq!(
        harness
            .diagnostics
            .snapshot()
            .get(DiagnosticCounter::ShortGapDropped),
        1
    );
}

/// AC-B2-OC-C02：30s 缺口只落 1 条半开区间记录。
#[test]
fn long_gap_30s_persists_single_half_open_record() {
    let harness = build_offline_capture(CapturePolicy::default());
    let session = SessionId::new("s1").expect("session");
    attach(&harness, &session);
    subtitle_readable(&harness, &session, 100, true);
    subtitle_readable(&harness, &session, 200, false);
    recover_subtitle(&harness, &session, 30_200);

    let records = harness.gaps.query(&session);
    assert_eq!(records.len(), 1);
    assert!(records[0].end > records[0].start);
    let duration = records[0].end.saturating_sub(records[0].start);
    assert!((29_600..=30_400).contains(&duration), "duration={duration}");
}

/// AC-B2-OC-C03：两段短缺口先合并再判门限，落 1 条。
#[test]
fn two_short_gaps_merge_then_persist() {
    let policy = CapturePolicy {
        gap_min_ms: 400,
        ..Default::default()
    };
    let harness = build_offline_capture(policy);
    let session = SessionId::new("s1").expect("session");
    attach(&harness, &session);
    subtitle_readable(&harness, &session, 100, true);
    subtitle_readable(&harness, &session, 200, false);
    recover_subtitle(&harness, &session, 500);
    subtitle_readable(&harness, &session, 600, false);
    recover_subtitle(&harness, &session, 900);

    let records = harness.gaps.query(&session);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].start.as_millis(), 200);
    assert_eq!(records[0].end.as_millis(), 900);
}

/// AC-B2-OC-C03：合并后时长达到门限才落盘。
#[test]
fn merged_short_gaps_pass_gate() {
    let policy = CapturePolicy {
        gap_min_ms: 400,
        ..Default::default()
    };
    let harness = build_offline_capture(policy);
    let session = SessionId::new("s1").expect("session");
    attach(&harness, &session);
    subtitle_readable(&harness, &session, 100, true);
    subtitle_readable(&harness, &session, 200, false);
    recover_subtitle(&harness, &session, 500);
    subtitle_readable(&harness, &session, 600, false);
    recover_subtitle(&harness, &session, 900);

    let records = harness.gaps.query(&session);
    assert_eq!(records.len(), 1);
    assert!(records[0].end > records[0].start);
    assert!(records[0].end.saturating_sub(records[0].start) >= policy.gap_min_ms);
}

/// AC-B2-OC-C04：回补窗口起点为 close - min(8000, duration)。
#[test]
fn resume_from_is_close_minus_capped_duration() {
    let harness = build_offline_capture(CapturePolicy::default());
    let session = SessionId::new("s1").expect("session");
    attach(&harness, &session);
    subtitle_readable(&harness, &session, 100, true);
    subtitle_readable(&harness, &session, 200, false);
    recover_subtitle(&harness, &session, 30_200);

    let intent = harness.asr.last().expect("intent");
    assert_eq!(intent.window_start.as_millis(), 22_200);
    assert_eq!(intent.window_end.as_millis(), 30_200);
}

/// AC-B2-OC-C04：更早区间不产生任何片段。
#[test]
fn earlier_than_resume_window_produces_no_segment() {
    let harness = build_offline_capture(CapturePolicy::default());
    let session = SessionId::new("s1").expect("session");
    attach(&harness, &session);
    subtitle_readable(&harness, &session, 100, true);
    subtitle_readable(&harness, &session, 200, false);
    recover_subtitle(&harness, &session, 30_200);

    let intent = harness.asr.last().expect("intent");
    assert!(intent.window_start.as_millis() > 200);
    assert_eq!(intent.chunk_count, 20);
}

/// AC-B2-OC-C05：300s 失效期间会话与要点零写入、零删除。
#[test]
fn gap_300s_keeps_session_and_insight_ids() {
    let harness = build_offline_capture(CapturePolicy::default());
    let session = SessionId::new("s1").expect("session");
    attach(&harness, &session);
    harness.insight.seed(vec!["i1".to_owned(), "i2".to_owned()]);
    subtitle_readable(&harness, &session, 100, true);
    subtitle_readable(&harness, &session, 200, false);
    recover_subtitle(&harness, &session, 300_200);

    assert_eq!(
        harness.insight.ids(),
        vec!["i1".to_owned(), "i2".to_owned()]
    );
    assert_eq!(harness.sentinel.counts(), (0, 0, 0, 0, 0));
    assert_eq!(harness.gaps.query(&session).len(), 1);
}

/// AC-B2-OC-C06：要点读取失败只跳过校验，不降级为删除。
#[test]
fn insight_read_failure_skips_check_without_delete() {
    let harness = build_offline_capture(CapturePolicy::default());
    let session = SessionId::new("s1").expect("session");
    attach(&harness, &session);
    harness.insight.set_failure(true);
    subtitle_readable(&harness, &session, 100, true);
    subtitle_readable(&harness, &session, 200, false);

    assert_eq!(
        harness
            .diagnostics
            .snapshot()
            .get(DiagnosticCounter::InsightCheckSkipped),
        1
    );
    let (_, deletes, _, _, _) = harness.sentinel.counts();
    assert_eq!(deletes, 0);
}

/// AC-B2-OC-C07：单通道恢复 5s 内关闭缺口并切到可用通道。
#[test]
fn single_channel_recovery_within_5s() {
    let harness = build_offline_capture(CapturePolicy::default());
    let session = SessionId::new("s1").expect("session");
    attach(&harness, &session);
    audio_readable(&harness, &session, 100, true);
    subtitle_readable(&harness, &session, 110, true);
    subtitle_readable(&harness, &session, 200, false);
    audio_readable(&harness, &session, 210, false);
    audio_retry(&harness, &session, 5_210);
    audio_readable(&harness, &session, 5_210, true);

    assert_eq!(5_210_u64 - 210, 5_000);
    assert!(matches!(
        harness.service.availability(&session),
        meeting_copilot_application::capture::CaptureAvailability::Available {
            channel: meeting_copilot_application::capture::ActiveChannel::Audio
        }
    ));
    assert_eq!(harness.gaps.query(&session).len(), 1);
}

/// AC-B2-OC-C08：反复失效只推送 1 次不可用。
#[test]
fn repeated_flapping_pushes_unavailable_once() {
    let harness = build_offline_capture(CapturePolicy::default());
    let session = SessionId::new("s1").expect("session");
    attach(&harness, &session);
    let unavailable = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&unavailable);
    let handle = harness.service.subscribe_status(Box::new(move |delivery| {
        if let Some(event) = delivery.availability {
            if matches!(
                event.availability,
                meeting_copilot_application::capture::CaptureAvailability::Unavailable { .. }
            ) {
                counter.fetch_add(1, Ordering::SeqCst);
            }
        }
        true
    }));

    subtitle_readable(&harness, &session, 100, true);
    subtitle_readable(&harness, &session, 200, false);
    for at in [300, 400, 500, 600] {
        subtitle_retry(&harness, &session, at);
    }
    recover_subtitle(&harness, &session, 1_300);
    harness.service.drain_status(MonoMillis::from_millis(1_400));
    handle.release();

    assert_eq!(unavailable.load(Ordering::SeqCst), 1);
    assert_eq!(harness.gaps.query(&session).len(), 1);
}

/// AC-B2-OC-C09：片段与缺口仅允许 400ms 边界接触。
#[test]
fn intervals_do_not_overlap_and_touch_within_boundary() {
    let session = SessionId::new("s1").expect("session");
    let gaps = vec![CaptureGapRecord {
        session: session.clone(),
        start: MonoMillis::from_millis(0),
        end: MonoMillis::from_millis(1_000),
        reason: meeting_copilot_domain::model::GapReason::BothChannelsUnavailable,
        backfilled: false,
    }];
    let segments = [
        (MonoMillis::from_millis(600), MonoMillis::from_millis(2_000)),
        (
            MonoMillis::from_millis(2_000),
            MonoMillis::from_millis(2_500),
        ),
    ];
    assert!(validate_intervals(&gaps, &segments, 400).is_ok());
}

/// AC-B2-OC-C09：重叠超过边界接触上限被拒。
#[test]
fn interval_violation_is_reported_for_overlap() {
    let session = SessionId::new("s1").expect("session");
    let gaps = vec![CaptureGapRecord {
        session,
        start: MonoMillis::from_millis(0),
        end: MonoMillis::from_millis(1_000),
        reason: meeting_copilot_domain::model::GapReason::BothChannelsUnavailable,
        backfilled: false,
    }];
    let segments = [(MonoMillis::from_millis(500), MonoMillis::from_millis(2_000))];
    assert_eq!(
        validate_intervals(&gaps, &segments, 400),
        Err(IntervalViolation::OverlapWithGap {
            segment: 0,
            gap: 0,
            overlap_ms: 500
        })
    );
}
