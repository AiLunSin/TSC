//! 合并单元构造：正文取字幕、区间取 ASR、置信度封顶、说话人取舍。

use crate::model::enums::SourceKind;
use crate::model::session::TextRange;
use crate::model::utterance::{SpeakerRef, Utterance};
use crate::vo::{Confidence, UtteranceRef};

use super::equivalence::PairBasis;
use super::policy::MergePolicy;

/// 配对依据与双方引用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pairing {
    pub basis: PairBasis,
    pub subtitle_ref: UtteranceRef,
    pub asr_ref: UtteranceRef,
    pub subtitle_span: TextRange,
    pub subtitle_estimated: bool,
}

/// 合并后的片段与其配对证据。
#[derive(Debug, Clone, PartialEq)]
pub struct MergedUtterance {
    pub utterance: Utterance,
    pub pairing: Pairing,
}

/// 双方均已识别且标签不同时返回 `None`（不合并）。
pub fn build_merged(
    subtitle: &Utterance,
    asr: &Utterance,
    policy: &MergePolicy,
    basis: PairBasis,
) -> Option<MergedUtterance> {
    if subtitle.reference == asr.reference {
        return None;
    }
    let speaker = if subtitle.speaker.identified && asr.speaker.identified {
        if subtitle.speaker == asr.speaker {
            subtitle.speaker.clone()
        } else {
            return None;
        }
    } else if subtitle.speaker.identified {
        subtitle.speaker.clone()
    } else if asr.speaker.identified {
        asr.speaker.clone()
    } else {
        SpeakerRef::unknown()
    };

    let cap = f64::from(policy.confidence_cap_milli) / 1000.0;
    let raw = subtitle.confidence.value().max(asr.confidence.value()).min(cap);
    let confidence = Confidence::new(raw).ok()?;

    let utterance = Utterance {
        reference: subtitle.reference.clone(),
        span: asr.span.clone(),
        wall_clock: subtitle.wall_clock.clone(),
        text: subtitle.text.clone(),
        speaker,
        confidence,
        source: SourceKind::PlatformSubtitle,
        platform: subtitle.platform,
        dual_channel: true,
        reading_mode: subtitle.reading_mode,
    };

    Some(MergedUtterance {
        utterance,
        pairing: Pairing {
            basis,
            subtitle_ref: subtitle.reference.clone(),
            asr_ref: asr.reference.clone(),
            subtitle_span: subtitle.span.clone(),
            subtitle_estimated: false,
        },
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::enums::{Platform, ReadingMode};
    use crate::vo::{UtteranceText, WallClock};

    fn utt(
        reference: &str,
        start: u64,
        end: u64,
        text: &str,
        speaker: SpeakerRef,
        confidence: f64,
    ) -> Utterance {
        Utterance {
            reference: UtteranceRef::new(reference).expect("ref"),
            span: TextRange {
                start_ms: start,
                end_ms: end,
            },
            wall_clock: WallClock::from_unix_millis(0),
            text: UtteranceText::new(text).expect("text"),
            speaker,
            confidence: Confidence::new(confidence).expect("conf"),
            source: SourceKind::PlatformSubtitle,
            platform: Some(Platform::TencentMeeting),
            dual_channel: false,
            reading_mode: ReadingMode::Subtitle,
        }
    }

    #[test]
    fn merged_span_uses_asr_precise_bounds() {
        let subtitle = utt("sub-1", 1_000, 2_000, "确认下周灰度上线", SpeakerRef::unknown(), 0.9);
        let asr = utt("asr-1", 1_200, 1_900, "确认下周灰度上线", SpeakerRef::unknown(), 0.9);
        let merged = build_merged(&subtitle, &asr, &MergePolicy::default(), PairBasis::Identical)
            .expect("merged");
        assert_eq!(merged.utterance.span, asr.span);
        assert!(merged.utterance.dual_channel);
    }

    #[test]
    fn confidence_is_capped() {
        let subtitle = utt("sub-1", 1_000, 2_000, "确认下周灰度上线", SpeakerRef::unknown(), 0.95);
        let asr = utt("asr-1", 1_000, 2_000, "确认下周灰度上线", SpeakerRef::unknown(), 0.99);
        let merged = build_merged(&subtitle, &asr, &MergePolicy::default(), PairBasis::Identical)
            .expect("merged");
        assert!(merged.utterance.confidence.value() <= 0.950);

        let mut capped = MergePolicy::default();
        capped.confidence_cap_milli = 800;
        let merged = build_merged(&subtitle, &asr, &capped, PairBasis::Identical).expect("merged");
        assert!(merged.utterance.confidence.value() <= 0.800);
    }

    #[test]
    fn identified_speaker_wins() {
        let muted = SpeakerRef::anonymous("说话人1");
        let named = SpeakerRef {
            label: "张三".into(),
            identified: true,
        };
        let subtitle = utt("sub-1", 1_000, 2_000, "确认下周灰度上线", muted.clone(), 0.9);
        let asr = utt("asr-1", 1_000, 2_000, "确认下周灰度上线", named.clone(), 0.9);
        let merged = build_merged(&subtitle, &asr, &MergePolicy::default(), PairBasis::Identical)
            .expect("merged");
        assert_eq!(merged.utterance.speaker, named);
    }

    #[test]
    fn conflicting_identified_speakers_are_not_merged() {
        let first = SpeakerRef {
            label: "张三".into(),
            identified: true,
        };
        let second = SpeakerRef {
            label: "李四".into(),
            identified: true,
        };
        let subtitle = utt("sub-1", 1_000, 2_000, "确认下周灰度上线", first, 0.9);
        let asr = utt("asr-1", 1_000, 2_000, "确认下周灰度上线", second, 0.9);
        assert!(
            build_merged(&subtitle, &asr, &MergePolicy::default(), PairBasis::Identical).is_none()
        );
    }
}