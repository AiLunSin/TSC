//! 通道仲裁决策表与 5s/10s 阈值纯函数（B2.23.8.1~8.3）。

use meeting_copilot_domain::model::{ChannelPreference, ChannelState, GapReason};
use meeting_copilot_domain::port::SourceHealth;

use super::types::{ActiveChannel, CapturePolicy};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decision {
    pub active: ActiveChannel,
    pub gap: Option<GapReason>,
}

fn usable(state: ChannelState) -> bool {
    state.is_usable()
}

/// 16 行决策表：4 偏好 × 字幕可用 × 音频可用。
pub fn decide(
    preference: ChannelPreference,
    subtitle: ChannelState,
    audio: ChannelState,
) -> Decision {
    let subtitle_usable = usable(subtitle);
    let audio_usable = usable(audio);
    match preference {
        ChannelPreference::SubtitleOnly => {
            if subtitle_usable {
                Decision {
                    active: ActiveChannel::Subtitle,
                    gap: None,
                }
            } else {
                Decision {
                    active: ActiveChannel::None,
                    gap: Some(GapReason::PreferenceRestricted),
                }
            }
        }
        ChannelPreference::AudioOnly => {
            if audio_usable {
                Decision {
                    active: ActiveChannel::Audio,
                    gap: None,
                }
            } else {
                Decision {
                    active: ActiveChannel::None,
                    gap: Some(GapReason::PreferenceRestricted),
                }
            }
        }
        ChannelPreference::SubtitleFirst => {
            if subtitle_usable {
                Decision {
                    active: ActiveChannel::Subtitle,
                    gap: None,
                }
            } else if audio_usable {
                Decision {
                    active: ActiveChannel::Audio,
                    gap: None,
                }
            } else {
                Decision {
                    active: ActiveChannel::None,
                    gap: Some(GapReason::BothChannelsUnavailable),
                }
            }
        }
        ChannelPreference::AudioFirst => {
            if audio_usable {
                Decision {
                    active: ActiveChannel::Audio,
                    gap: None,
                }
            } else if subtitle_usable {
                Decision {
                    active: ActiveChannel::Subtitle,
                    gap: None,
                }
            } else {
                Decision {
                    active: ActiveChannel::None,
                    gap: Some(GapReason::BothChannelsUnavailable),
                }
            }
        }
    }
}

/// 回退判定：`Available` 且 `now - last_valid_text_at >= 5000`。
pub fn subtitle_fallback_due(
    now: meeting_copilot_domain::vo::MonoMillis,
    last_valid_text_at: Option<meeting_copilot_domain::vo::MonoMillis>,
    policy: &CapturePolicy,
    subtitle: ChannelState,
) -> bool {
    subtitle == ChannelState::Available
        && last_valid_text_at
            .is_some_and(|last| now.saturating_sub(last) >= policy.silence_fallback_ms)
}

/// 切回判定：字幕 `Available` 且稳定满 10s。
pub fn stable_switch_back_due(
    now: meeting_copilot_domain::vo::MonoMillis,
    stable_since: Option<meeting_copilot_domain::vo::MonoMillis>,
    policy: &CapturePolicy,
    subtitle: ChannelState,
) -> bool {
    subtitle == ChannelState::Available
        && stable_since
            .is_some_and(|since| now.saturating_sub(since) >= policy.stable_switch_back_ms)
}

/// 只读候选映射：`SourceHealth` 的 String 原因在入口处丢弃，不产生 Probing。
pub fn source_health_candidate(health: &SourceHealth) -> ChannelState {
    match health {
        SourceHealth::Ready => ChannelState::Available,
        SourceHealth::Degraded(_) => ChannelState::Degraded,
        SourceHealth::Unavailable(_) => ChannelState::Unavailable,
    }
}

/// 偏好变更后立即用当前快照重算，不等待新证据。
pub fn preference_change_decision(
    next: ChannelPreference,
    subtitle: ChannelState,
    audio: ChannelState,
) -> Decision {
    decide(next, subtitle, audio)
}

#[cfg(test)]
mod tests {
    use super::*;
    use meeting_copilot_domain::vo::MonoMillis;

    const PREFS: [ChannelPreference; 4] = [
        ChannelPreference::SubtitleFirst,
        ChannelPreference::AudioFirst,
        ChannelPreference::SubtitleOnly,
        ChannelPreference::AudioOnly,
    ];

    #[test]
    fn source_health_mapping_never_probes() {
        assert_eq!(
            source_health_candidate(&SourceHealth::Ready),
            ChannelState::Available
        );
        assert_eq!(
            source_health_candidate(&SourceHealth::Degraded("legacy".into())),
            ChannelState::Degraded
        );
        assert_eq!(
            source_health_candidate(&SourceHealth::Unavailable("reason".into())),
            ChannelState::Unavailable
        );
    }

    #[test]
    fn preference_change_recomputes_immediately() {
        let before = decide(
            ChannelPreference::SubtitleFirst,
            ChannelState::Available,
            ChannelState::Available,
        );
        assert_eq!(before.active, ActiveChannel::Subtitle);
        let after = preference_change_decision(
            ChannelPreference::AudioFirst,
            ChannelState::Available,
            ChannelState::Available,
        );
        assert_eq!(after.active, ActiveChannel::Audio);
    }

    #[test]
    fn decision_table_16_rows() {
        let expected = |preference, subtitle_ok, audio_ok| match preference {
            ChannelPreference::SubtitleFirst => {
                if subtitle_ok {
                    (ActiveChannel::Subtitle, None)
                } else if audio_ok {
                    (ActiveChannel::Audio, None)
                } else {
                    (
                        ActiveChannel::None,
                        Some(GapReason::BothChannelsUnavailable),
                    )
                }
            }
            ChannelPreference::AudioFirst => {
                if audio_ok {
                    (ActiveChannel::Audio, None)
                } else if subtitle_ok {
                    (ActiveChannel::Subtitle, None)
                } else {
                    (
                        ActiveChannel::None,
                        Some(GapReason::BothChannelsUnavailable),
                    )
                }
            }
            ChannelPreference::SubtitleOnly => {
                if subtitle_ok {
                    (ActiveChannel::Subtitle, None)
                } else {
                    (ActiveChannel::None, Some(GapReason::PreferenceRestricted))
                }
            }
            ChannelPreference::AudioOnly => {
                if audio_ok {
                    (ActiveChannel::Audio, None)
                } else {
                    (ActiveChannel::None, Some(GapReason::PreferenceRestricted))
                }
            }
        };

        for preference in PREFS {
            for subtitle_ok in [true, false] {
                for audio_ok in [true, false] {
                    let subtitle = if subtitle_ok {
                        ChannelState::Available
                    } else {
                        ChannelState::Unavailable
                    };
                    let audio = if audio_ok {
                        ChannelState::Degraded
                    } else {
                        ChannelState::Probing
                    };
                    let decision = decide(preference, subtitle, audio);
                    let (active, gap) = expected(preference, subtitle_ok, audio_ok);
                    assert_eq!(decision.active, active, "{preference:?}");
                    assert_eq!(decision.gap, gap, "{preference:?}");
                }
            }
        }
    }

    #[test]
    fn fallback_boundaries_are_strict() {
        let policy = CapturePolicy::default();
        let last = MonoMillis::from_millis(1_000);
        assert!(!subtitle_fallback_due(
            MonoMillis::from_millis(5_999),
            Some(last),
            &policy,
            ChannelState::Available
        ));
        assert!(subtitle_fallback_due(
            MonoMillis::from_millis(6_000),
            Some(last),
            &policy,
            ChannelState::Available
        ));
    }

    #[test]
    fn switch_back_boundaries_are_strict() {
        let policy = CapturePolicy::default();
        let since = MonoMillis::from_millis(1_000);
        assert!(!stable_switch_back_due(
            MonoMillis::from_millis(10_999),
            Some(since),
            &policy,
            ChannelState::Available
        ));
        assert!(stable_switch_back_due(
            MonoMillis::from_millis(11_000),
            Some(since),
            &policy,
            ChannelState::Available
        ));
        assert!(!stable_switch_back_due(
            MonoMillis::from_millis(11_000),
            Some(since),
            &policy,
            ChannelState::Degraded
        ));
    }

    #[test]
    fn preference_restriction_never_crosses() {
        let subtitle_none = decide(
            ChannelPreference::SubtitleOnly,
            ChannelState::Unavailable,
            ChannelState::Available,
        );
        assert_eq!(subtitle_none.active, ActiveChannel::None);
        assert_eq!(subtitle_none.gap, Some(GapReason::PreferenceRestricted));

        let audio_none = decide(
            ChannelPreference::AudioOnly,
            ChannelState::Available,
            ChannelState::Unavailable,
        );
        assert_eq!(audio_none.active, ActiveChannel::None);
        assert_eq!(audio_none.gap, Some(GapReason::PreferenceRestricted));
    }
}
