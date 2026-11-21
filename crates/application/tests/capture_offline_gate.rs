//! 离线门禁用例（AC-B2-OC-E06/E07/E09，3 例）。

mod support;

use meeting_copilot_application::capture::{CapturePolicy, ChannelObservation, ObservationPayload};
use meeting_copilot_domain::model::ChannelKind;
use meeting_copilot_domain::vo::MonoMillis;
use support::{build_offline_capture, offline_view};

/// AC-B2-OC-E06：装配可跨 4 个测试文件复用，最小路径可走通。
#[test]
fn offline_harness_is_reusable_across_tests() {
    let harness = build_offline_capture(CapturePolicy::default());
    let context = offline_view("s1");
    harness
        .service
        .attach_session(context.clone())
        .expect("attach");
    harness.service.observe(&ChannelObservation {
        session: Some(context.session.clone()),
        platform: context.platform,
        kind: ChannelKind::Subtitle,
        at: MonoMillis::from_millis(100),
        payload: ObservationPayload::Probe {
            readable: true,
            version_supported: true,
            ocr_only: false,
        },
    });
    assert_eq!(harness.status_sink.len(), 1);

    let files = [
        include_str!("capture_degraded_guard.rs"),
        include_str!("capture_status_push.rs"),
        include_str!("capture_fakes.rs"),
        include_str!("capture_offline_gate.rs"),
    ];
    let reusable = files
        .iter()
        .filter(|source| source.contains("build_offline_capture"))
        .count();
    assert!(reusable >= 4);
}

/// AC-B2-OC-E07：最小路径建连计数为 0。
#[test]
fn offline_min_path_has_zero_connections() {
    let harness = build_offline_capture(CapturePolicy::default());
    let context = offline_view("s1");
    harness
        .service
        .attach_session(context.clone())
        .expect("attach");
    harness.service.observe(&ChannelObservation {
        session: Some(context.session),
        platform: context.platform,
        kind: ChannelKind::Subtitle,
        at: MonoMillis::from_millis(100),
        payload: ObservationPayload::Probe {
            readable: true,
            version_supported: true,
            ocr_only: false,
        },
    });
    assert_eq!(harness.connections.connections(), 0);
}

/// AC-B2-OC-E06/E09：装配函数不含禁用调用。
#[test]
fn offline_harness_forbidden_calls_scan() {
    let source = include_str!("support/mod.rs");
    for forbidden in ["thread::sleep", "Instant::now", "SystemTime::now"] {
        assert!(!source.contains(forbidden), "装配函数不应包含 {forbidden}");
    }
}
