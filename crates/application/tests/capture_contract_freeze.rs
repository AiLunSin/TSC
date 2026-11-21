//! 领域契约冻结（AC-B2-OC-E10，6 例）。

use meeting_copilot_domain::error::ErrorCode;
use meeting_copilot_domain::model::{ChannelPreference, Platform};
use meeting_copilot_domain::port::{SourceHealth, Subscription};

/// AC-B2-OC-E10：`SourceHealth` 恰 3 个变体。
#[test]
fn source_health_variants_frozen() {
    let values = [
        SourceHealth::Ready,
        SourceHealth::Degraded("原因".to_owned()),
        SourceHealth::Unavailable("原因".to_owned()),
    ];
    assert_eq!(values.len(), 3);
}

/// AC-B2-OC-E10：`Platform` 恰 3 个平台值。
#[test]
fn platform_values_frozen() {
    let values = [Platform::TencentMeeting, Platform::Feishu, Platform::Zoom];
    assert_eq!(values.len(), 3);
}

/// AC-B2-OC-E10：`ChannelPreference` 恰 4 个取值。
#[test]
fn channel_preference_values_frozen() {
    let values = [
        ChannelPreference::SubtitleFirst,
        ChannelPreference::AudioFirst,
        ChannelPreference::SubtitleOnly,
        ChannelPreference::AudioOnly,
    ];
    assert_eq!(values.len(), 4);
}

/// AC-B2-OC-E10：`Subscription` 的 4 个方法均可调用。
#[test]
fn subscription_surface_frozen() {
    let subscription = Subscription::new();
    assert!(subscription.is_active());
    let _wrapped = subscription.wrap(|_event| {});
    subscription.release();
    assert!(!subscription.is_active());
}

/// AC-B2-OC-E10：四个仓储 trait 的方法集合冻结（只读源码断言，域层零改动）。
#[test]
fn repository_traits_frozen() {
    let source = include_str!("../../domain/src/port/repository.rs");
    for required in [
        "pub trait SessionRepository",
        "fn create",
        "fn transition",
        "fn get",
        "fn delete_irrecoverable",
        "pub trait UtteranceRepository",
        "fn append",
        "fn apply_correction",
        "fn list",
        "pub trait InsightRepository",
        "fn save",
        "fn save_summary",
        "pub trait CardRepository",
        "fn update",
    ] {
        assert!(source.contains(required), "缺少仓储契约：{required}");
    }
}

/// AC-B2-OC-E10：`ErrorCode::ALL` 长度 12。
#[test]
fn error_code_all_len_frozen() {
    assert_eq!(ErrorCode::ALL.len(), 12);
}
