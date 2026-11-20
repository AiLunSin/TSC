use serde::{Deserialize, Serialize};

use crate::model::enums::{Platform, ReadingMode, SourceKind};
use crate::model::session::TextRange;
use crate::vo::{AudioClock, Confidence, UtteranceRef, UtteranceText, WallClock};

/// 说话人引用。无法识别时使用匿名兜底，而非丢弃片段（SPEC 5.1）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpeakerRef {
    pub label: String,
    pub identified: bool,
}

impl SpeakerRef {
    pub fn anonymous(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            identified: false,
        }
    }

    pub fn unknown() -> Self {
        Self::anonymous("未知说话人")
    }
}

/// 对话片段。所有要点必须能回溯到至少一条片段。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Utterance {
    pub reference: UtteranceRef,
    pub span: TextRange,
    pub wall_clock: WallClock,
    pub text: UtteranceText,
    pub speaker: SpeakerRef,
    pub confidence: Confidence,
    pub source: SourceKind,
    pub platform: Option<Platform>,
    pub dual_channel: bool,
    pub reading_mode: ReadingMode,
}

impl Utterance {
    /// 静音、超低置信度与占位文本不参与要点提炼。
    pub const MIN_INSIGHT_CONFIDENCE: f64 = 0.35;

    pub fn is_eligible_for_insight(&self) -> bool {
        !self.confidence.is_speculative(Self::MIN_INSIGHT_CONFIDENCE)
            && self.text.as_str().trim() != "…"
    }

    pub fn audio_span(&self) -> AudioClock {
        AudioClock::from_millis(self.span.start_ms)
    }
}

/// 对已产出片段的修订（ASR 二次校准或人工更正）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Correction {
    pub replaces: UtteranceRef,
    pub revised_text: UtteranceText,
    pub confidence: Confidence,
    pub applied_at: WallClock,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utterance(confidence: f64, text: &str) -> Utterance {
        Utterance {
            reference: UtteranceRef::new("utt-1").expect("ref"),
            span: TextRange::new(0, 1_500).expect("range"),
            wall_clock: WallClock::from_unix_millis(1_000),
            text: UtteranceText::new(text).expect("text"),
            speaker: SpeakerRef::unknown(),
            confidence: Confidence::new(confidence).expect("confidence"),
            source: SourceKind::PlatformSubtitle,
            platform: Some(Platform::Feishu),
            dual_channel: false,
            reading_mode: ReadingMode::Subtitle,
        }
    }

    #[test]
    fn low_confidence_is_filtered() {
        assert!(!utterance(0.1, "嗯").is_eligible_for_insight());
        assert!(utterance(0.9, "确认下周灰度上线").is_eligible_for_insight());
    }

    #[test]
    fn placeholder_text_is_filtered() {
        assert!(!utterance(0.9, "…").is_eligible_for_insight());
    }

    #[test]
    fn speaker_fallback_is_explicitly_unidentified() {
        let speaker = SpeakerRef::unknown();
        assert!(!speaker.identified);
        assert_eq!(speaker.label, "未知说话人");
    }
}
