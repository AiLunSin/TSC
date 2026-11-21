//! 合并侧 DTO 与端口：ASR 文本事件、合并单元、计数证据。

use meeting_copilot_domain::merge::MergedUtterance;
use meeting_copilot_domain::model::channel::ChannelKind;
use meeting_copilot_domain::model::enums::Platform;
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::model::utterance::{SpeakerRef, Utterance};
use meeting_copilot_domain::vo::{Confidence, MonoMillis, SessionId, UtteranceRef, UtteranceText};

use super::merge_gate::MergeSpan;
use super::types::AsrIntentKind;

/// ASR 文本事件（含精确区间与来源侧）。
#[derive(Debug, Clone, PartialEq)]
pub struct AsrTextEvent {
    pub session: SessionId,
    pub kind: AsrIntentKind,
    pub span: TextRange,
    pub text: UtteranceText,
    pub speaker: SpeakerRef,
    pub confidence: Confidence,
    pub estimated: bool,
    pub platform: Option<Platform>,
    pub reference: UtteranceRef,
}

/// 双通道文本事件（侧别由 `side` 区分）。
#[derive(Debug, Clone, PartialEq)]
pub struct TextEvent {
    pub session: SessionId,
    pub side: ChannelKind,
    pub span: TextRange,
    pub text: UtteranceText,
    pub speaker: SpeakerRef,
    pub confidence: Confidence,
    pub estimated: bool,
    pub platform: Option<Platform>,
    pub reference: UtteranceRef,
}

impl TextEvent {
    pub fn as_asr_event(&self) -> AsrTextEvent {
        AsrTextEvent {
            session: self.session.clone(),
            kind: AsrIntentKind::GapBackfill,
            span: self.span,
            text: self.text.clone(),
            speaker: self.speaker.clone(),
            confidence: self.confidence,
            estimated: self.estimated,
            platform: self.platform,
            reference: self.reference.clone(),
        }
    }
}

/// 合并摄入端口：只追加、不写状态机。
pub trait MergeIntake: Send + Sync {
    fn push(&self, event: AsrTextEvent);
}

/// 未合并的单路片段。
#[derive(Debug, Clone, PartialEq)]
pub struct SingleUtterance {
    pub utterance: Utterance,
    pub overlap_marked: bool,
}

/// 合并输出单元。
#[derive(Debug, Clone, PartialEq)]
pub enum MergedUnit {
    Single(SingleUtterance),
    Merged(MergedUtterance),
}

impl From<&MergedUnit> for MergeSpan {
    fn from(unit: &MergedUnit) -> Self {
        match unit {
            MergedUnit::Single(single) => MergeSpan {
                start: MonoMillis::from_millis(single.utterance.span.start_ms),
                end: MonoMillis::from_millis(single.utterance.span.end_ms),
                dual_channel: false,
                overlap_marked: single.overlap_marked,
                origin_pair: None,
            },
            MergedUnit::Merged(merged) => MergeSpan {
                start: MonoMillis::from_millis(merged.utterance.span.start_ms),
                end: MonoMillis::from_millis(merged.utterance.span.end_ms),
                dual_channel: true,
                overlap_marked: false,
                origin_pair: Some((
                    merged.pairing.subtitle_ref.clone(),
                    merged.pairing.asr_ref.clone(),
                )),
            },
        }
    }
}

/// 合并过程计数（固定 9 项）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MergeCounters {
    pub pair_candidates: u64,
    pub merge_applied: u64,
    pub merge_rejected: u64,
    pub late_unmerged: u64,
    pub overlap_marked: u64,
    pub non_mergeable_text: u64,
    pub duplicate_suppressed: u64,
    pub budget_stopped: u64,
    pub gap_conflicts: u64,
}

/// 单次合并的计数证据。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DedupeCounters {
    pub insight_trigger_count: u32,
    pub gap_detect_count: u32,
    pub summary_entry_count: u32,
    pub persisted_utterance_count: u32,
    pub mentions_delta: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DedupeEvidence {
    pub merged: DedupeCounters,
    pub folded: DedupeCounters,
}
