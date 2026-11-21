//! 替身契约用例（AC-B2-OC-E01~E05、E13，7 例）。

mod support;

use meeting_copilot_application::capture::{
    AsrDeliverySink, AsrIntent, AsrIntentKind, CaptureGapRecord, CapturePolicy, ChannelGapLog,
    ChannelSwitchLog,
};
use meeting_copilot_domain::model::{ChannelKind, GapReason, Platform, SwitchReason};
use meeting_copilot_domain::port::CaptureClock;
use meeting_copilot_domain::vo::{MonoMillis, SessionId, WallClock};
use support::{
    build_offline_capture, CountingAsrDelivery, CountingAudit, MemoryGapLog, MemorySwitchLog,
    MonotonicSystemClock,
};

fn session() -> SessionId {
    SessionId::new("s1").expect("session")
}

fn gap(start: u64, end: u64) -> CaptureGapRecord {
    CaptureGapRecord {
        session: session(),
        start: MonoMillis::from_millis(start),
        end: MonoMillis::from_millis(end),
        reason: GapReason::BothChannelsUnavailable,
        backfilled: false,
    }
}

/// AC-B2-OC-E01：替身公开面仅追加/查询/计数，无更新、删除、清空或通用发送。
#[test]
fn fakes_expose_minimal_method_surface() {
    let source = include_str!("support/fakes.rs");
    for forbidden in [
        "fn update(",
        "fn delete(&self,",
        "fn remove(",
        "fn clear(",
        "fn send<",
    ] {
        assert!(!source.contains(forbidden), "替身不应暴露 {forbidden}");
    }
    let _harness = build_offline_capture(CapturePolicy::default());
}

/// AC-B2-OC-E02：切换日志只追加，同毫秒合并为 1 条。
#[test]
fn switch_log_append_is_monotonic_and_same_ms_idempotent() {
    let log = MemorySwitchLog::default();
    let record = meeting_copilot_application::capture::ChannelSwitchRecord {
        session: session(),
        from: meeting_copilot_application::capture::ActiveChannel::None,
        to: meeting_copilot_application::capture::ActiveChannel::Subtitle,
        reason: SwitchReason::SessionInitial,
        at: MonoMillis::from_millis(100),
        wall_at: WallClock::from_unix_millis(0),
    };
    log.append(&record).expect("append");
    assert_eq!(log.raw_len(), 1);
    log.append(&record).expect("append");
    assert_eq!(log.raw_len(), 2);
    assert_eq!(log.records().len(), 1);
}

/// AC-B2-OC-E03：缺口日志查询按 1000ms 合并，保留最早起点与最晚终点。
#[test]
fn gap_log_query_merges_adjacent_within_1s() {
    let log = MemoryGapLog::default();
    log.append(&gap(0, 600)).expect("append");
    log.append(&gap(1_000, 1_600)).expect("append");
    let merged = log.query(&session());
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].start.as_millis(), 0);
    assert_eq!(merged[0].end.as_millis(), 1_600);
    assert!(merged.windows(2).all(|pair| pair[0].end <= pair[1].start));
}

/// AC-B2-OC-E04：三类 ASR 意图分别计数。
#[test]
fn asr_delivery_counts_by_kind() {
    let asr = CountingAsrDelivery::default();
    for kind in [
        AsrIntentKind::Validation,
        AsrIntentKind::GapBackfill,
        AsrIntentKind::Primary,
    ] {
        for offset in 0..2 {
            asr.deliver(&AsrIntent {
                session: session(),
                kind,
                window_start: MonoMillis::from_millis(offset),
                window_end: MonoMillis::from_millis(offset + 100),
                chunk_count: 1,
            })
            .expect("deliver");
        }
    }
    assert_eq!(asr.count(AsrIntentKind::Validation), 2);
    assert_eq!(asr.count(AsrIntentKind::GapBackfill), 2);
    assert_eq!(asr.count(AsrIntentKind::Primary), 2);
}

/// AC-B2-OC-E04/D08：审计条目无载荷字段。
#[test]
fn audit_event_has_no_payload() {
    let audit = CountingAudit::default();
    audit.record("capture_gap", 10, 1);
    let entries = audit.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].kind, "capture_gap");
    let rendered = format!("{:?}", entries[0]);
    for forbidden in ["text", "sample", "speaker", "participant"] {
        assert!(!rendered.contains(forbidden));
    }
}

/// AC-B2-OC-E05：时钟替身单调不减且不 panic。
#[test]
fn system_clock_fake_is_monotonic_non_decreasing() {
    let clock = MonotonicSystemClock::default();
    let first = clock.mono_now().expect("first");
    let second = clock.mono_now().expect("second");
    assert!(second >= first);
    let _ = clock.wall_now().expect("wall");
}

/// AC-B2-OC-E13：替身无网络/设备/凭据调用。
#[test]
fn fakes_have_no_network_device_or_credential_calls() {
    let source = include_str!("support/fakes.rs");
    for forbidden in [
        "reqwest",
        "TcpStream",
        "native_tls",
        "keychain",
        "password",
        "credential",
        "wasapi",
        "ScreenCaptureKit",
    ] {
        assert!(!source.contains(forbidden), "替身不应包含 {forbidden}");
    }
    let _ = Platform::TencentMeeting;
    let _ = ChannelKind::Subtitle;
}
