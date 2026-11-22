//! 双通道合并语料回归集（24.5）：等价判定与合并构造。
#![allow(clippy::field_reassign_with_default)]

use meeting_copilot_domain::merge::{
    build_merged, classify_equivalence, MergePolicy, MergedUtterance, PairBasis,
};
use meeting_copilot_domain::model::enums::{Platform, ReadingMode, SourceKind};
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::model::utterance::{SpeakerRef, Utterance};
use meeting_copilot_domain::vo::{Confidence, UtteranceRef, UtteranceText, WallClock};

fn utterance(reference: &str, start: u64, end: u64, text: &str, speaker: SpeakerRef) -> Utterance {
    Utterance {
        reference: UtteranceRef::new(reference).expect("ref"),
        span: TextRange {
            start_ms: start,
            end_ms: end,
        },
        wall_clock: WallClock::from_unix_millis(0),
        text: UtteranceText::new(text).expect("text"),
        speaker,
        confidence: Confidence::new(0.9).expect("conf"),
        source: SourceKind::PlatformSubtitle,
        platform: Some(Platform::TencentMeeting),
        dual_channel: false,
        reading_mode: ReadingMode::Subtitle,
    }
}

#[test]
fn corpus_identical_after_normalization() {
    let policy = MergePolicy::default();
    assert_eq!(
        classify_equivalence("确认下周灰度上线。", "確認下週灰度上線", &policy),
        Some(PairBasis::Identical)
    );
}

#[test]
fn corpus_cosine_detected_for_long_text() {
    let mut policy = MergePolicy::default();
    policy.simhash_max_distance = 0;
    let left = "会议决定下周三开始灰度上线并完成监控告警演练由张三负责回滚方案确认";
    let right = "会议决定下周三开始灰度上线并完成监控告警演练由张四负责回滚方案确认";
    assert!(matches!(
        classify_equivalence(left, right, &policy),
        Some(PairBasis::Cosine { .. })
    ));
}

#[test]
fn corpus_short_text_skipped_unless_identical() {
    let policy = MergePolicy::default();
    assert_eq!(classify_equivalence("灰度", "回滚", &policy), None);
    assert_eq!(
        classify_equivalence("灰度", "灰度", &policy),
        Some(PairBasis::Identical)
    );
}

#[test]
fn corpus_merged_uses_asr_span_and_subtitle_text() {
    let subtitle = utterance(
        "sub-1",
        1_000,
        2_000,
        "确认下周灰度上线",
        SpeakerRef::unknown(),
    );
    let asr = utterance(
        "asr-1",
        1_200,
        1_900,
        "确认下周灰度上线",
        SpeakerRef::unknown(),
    );
    let MergedUtterance {
        utterance: merged,
        pairing,
    } = build_merged(
        &subtitle,
        &asr,
        &MergePolicy::default(),
        PairBasis::Identical,
    )
    .expect("merged");
    assert_eq!(merged.span, asr.span);
    assert_eq!(merged.text.as_str(), subtitle.text.as_str());
    assert!(merged.dual_channel);
    assert_eq!(pairing.subtitle_ref, subtitle.reference);
    assert_eq!(pairing.asr_ref, asr.reference);
}

#[test]
fn corpus_conflicting_speakers_not_merged() {
    let named = SpeakerRef {
        label: "张三".into(),
        identified: true,
    };
    let other = SpeakerRef {
        label: "李四".into(),
        identified: true,
    };
    let subtitle = utterance("sub-1", 0, 1_000, "确认下周上线", named);
    let asr = utterance("asr-1", 0, 1_000, "确认下周上线", other);
    assert!(build_merged(
        &subtitle,
        &asr,
        &MergePolicy::default(),
        PairBasis::Identical
    )
    .is_none());
}
