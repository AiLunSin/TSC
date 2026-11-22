//! 28 片段准入：状态 → 来源/平台 → 空段 → 置信度 → 静音 → 切分。

use meeting_copilot_domain::model::enums::{SessionState, SourceKind};
use meeting_copilot_domain::model::insight_policy::InsightPolicy;
use meeting_copilot_domain::model::utterance::SpeakerRef;
use meeting_copilot_domain::model::utterance_text::split_utterance;
use meeting_copilot_domain::vo::UtteranceRef;

use super::counters::{Counter, UnderstandingCounters};
use super::types::{NormalizedFragment, RejectReason, UnderstandInput, UnderstandOutcome};

fn fragment(
    input: &UnderstandInput,
    reference: UtteranceRef,
    parent: UtteranceRef,
    span: meeting_copilot_domain::model::TextRange,
    text: meeting_copilot_domain::vo::UtteranceText,
) -> NormalizedFragment {
    let utterance = match &input.unit {
        crate::capture::merge_types::MergedUnit::Single(single) => &single.utterance,
        crate::capture::merge_types::MergedUnit::Merged(merged) => &merged.utterance,
    };
    let mut speaker = utterance.speaker.clone();
    if speaker.label.trim().is_empty() {
        speaker = SpeakerRef::unknown();
    }
    NormalizedFragment {
        session: input.session.clone(),
        reference,
        parent_reference: parent,
        span,
        text,
        speaker,
        confidence: utterance.confidence,
        dual_channel: utterance.dual_channel,
        origin_pair: None,
    }
}

/// 串行准入；每个拒绝分支先计一次数再返回。
pub(crate) fn admit(
    input: &UnderstandInput,
    policy: &InsightPolicy,
    counters: &UnderstandingCounters,
) -> UnderstandOutcome {
    if matches!(input.state, SessionState::Paused | SessionState::Deleted) {
        counters.bump(Counter::PausedDrop);
        return UnderstandOutcome::Rejected(RejectReason::Paused);
    }

    let utterance = match &input.unit {
        crate::capture::merge_types::MergedUnit::Single(single) => &single.utterance,
        crate::capture::merge_types::MergedUnit::Merged(merged) => &merged.utterance,
    };

    match utterance.source {
        SourceKind::PlatformSubtitle | SourceKind::PlatformAudio => {
            if utterance.platform.is_none() || utterance.platform != input.session_platform {
                counters.bump(Counter::SourcePlatformMismatch);
                return UnderstandOutcome::Rejected(RejectReason::SourcePlatformMismatch);
            }
        }
        SourceKind::Microphone | SourceKind::ExternalTranscription => {
            if utterance.platform.is_some() {
                counters.bump(Counter::SourcePlatformMismatch);
                return UnderstandOutcome::Rejected(RejectReason::SourcePlatformMismatch);
            }
        }
    }

    let raw = utterance.text.as_str();
    if raw.trim().is_empty() || raw.trim() == "…" {
        counters.bump(Counter::EmptyChunkDropped);
        return UnderstandOutcome::Rejected(RejectReason::Empty);
    }

    let confidence_milli = (utterance.confidence.value() * 1000.0).round() as u16;
    match super::low_confidence::classify(confidence_milli, policy.insight_confidence_min_milli) {
        super::low_confidence::ConfidenceVerdict::Speculative => {
            counters.bump(Counter::SpeculativeSkip);
            return UnderstandOutcome::Rejected(RejectReason::Speculative);
        }
        super::low_confidence::ConfidenceVerdict::StoredOnly => {
            counters.bump(Counter::SkippedLowConfidence);
            return UnderstandOutcome::StoredOnly {
                reason: RejectReason::LowConfidence,
            };
        }
        super::low_confidence::ConfidenceVerdict::Admitted => {}
    }

    let span_ms = utterance
        .span
        .end_ms
        .saturating_sub(utterance.span.start_ms)
        .max(1);
    let silence_ms = input.silence.overlap_ms(utterance.span);
    match super::silence_filter::classify(
        silence_ms,
        span_ms,
        policy.silence_overlap_ratio_min_milli,
    ) {
        super::silence_filter::SilenceVerdict::Dropped => {
            counters.bump(Counter::SilenceDropped);
            return UnderstandOutcome::Rejected(RejectReason::Silence);
        }
        super::silence_filter::SilenceVerdict::Partial => {
            counters.bump(Counter::SilencePartialOverlap);
        }
        super::silence_filter::SilenceVerdict::None => {}
    }

    if raw.chars().count() > usize::from(policy.split_char_limit) {
        let chunks =
            match split_utterance(raw, utterance.span, usize::from(policy.split_char_limit)) {
                Ok(chunks) => chunks,
                Err(_) => {
                    counters.bump(Counter::SplitSpanDegraded);
                    return UnderstandOutcome::Rejected(RejectReason::Empty);
                }
            };
        let mut fragments = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            let Ok(reference) =
                UtteranceRef::new(format!("{}-{}", utterance.reference.as_str(), chunk.index))
            else {
                counters.bump(Counter::SplitSpanDegraded);
                continue;
            };
            fragments.push(fragment(
                input,
                reference,
                utterance.reference.clone(),
                chunk.span,
                chunk.text,
            ));
        }
        counters.bump(Counter::Admitted);
        return UnderstandOutcome::Admitted { fragments };
    }

    counters.bump(Counter::Admitted);
    UnderstandOutcome::Admitted {
        fragments: vec![fragment(
            input,
            utterance.reference.clone(),
            utterance.reference.clone(),
            utterance.span,
            utterance.text.clone(),
        )],
    }
}
