//! 四类任务的规则兜底：纯同步、零 IO、单次 ≤50 ms（DESIGN 2.1.3-（8））。
//!
//! 兜底结果与正常结果同型；不得以历史结果冒充新生成。

use meeting_copilot_domain::model::enums::InsightKind;
use meeting_copilot_domain::model::llm::{
    CardConclusion, CompletionHint, ConclusionNotice, InsightExtract, InsightItem, LlmTask,
    MinutesDraft, MinutesInsights, PendingItem, PendingReason, StructuredBody, Summary,
    SummaryPurpose, Timeline,
};
use meeting_copilot_domain::vo::{Confidence, ShortText, UtteranceRef, UtteranceText};

/// 兜底输入只含本次请求快照；历史入口仅对 `Summary` 开放。
#[derive(Debug, Clone, Default)]
pub struct FallbackInput {
    pub sources: Vec<UtteranceRef>,
    pub window_text: Option<String>,
    pub last_summary: Option<Summary>,
}

pub struct RuleFallback;

impl RuleFallback {
    pub fn produce(task: LlmTask, input: &FallbackInput) -> StructuredBody {
        match task {
            LlmTask::InsightExtract => StructuredBody::Insight(insight(input)),
            LlmTask::Summary => StructuredBody::Summary(summary(input)),
            LlmTask::CardConclusion => StructuredBody::Conclusion(conclusion(input)),
            LlmTask::MinutesDraft => StructuredBody::Minutes(minutes(input)),
        }
    }
}

fn text_or_default(input: &FallbackInput) -> ShortText {
    input
        .window_text
        .as_deref()
        .map(ShortText::truncated)
        .unwrap_or_else(|| ShortText::truncated("待确认的要点"))
}

fn confidence() -> Confidence {
    Confidence::new(0.35).unwrap_or(Confidence::ZERO)
}

fn insight(input: &FallbackInput) -> InsightExtract {
    InsightExtract {
        insights: vec![InsightItem {
            kind: InsightKind::KeyInfo,
            text: text_or_default(input),
            sources: input.sources.clone(),
            confidence: confidence(),
            decision: None,
            todo: None,
        }],
    }
}

fn summary(input: &FallbackInput) -> Summary {
    match input.last_summary.as_ref() {
        Some(previous) => Summary {
            carried_over: true,
            produced: true,
            ..previous.clone()
        },
        None => Summary {
            purpose: SummaryPurpose::RollingWindow,
            text: UtteranceText::new("未生成")
                .unwrap_or_else(|_| UtteranceText::new("空").expect("常量合法")),
            covered: None,
            version: 0,
            carried_over: false,
            produced: false,
        },
    }
}

fn conclusion(input: &FallbackInput) -> CardConclusion {
    let notice = if input.sources.is_empty() {
        ConclusionNotice::GenerationUnavailable
    } else {
        ConclusionNotice::MissingSource
    };
    CardConclusion {
        title: meeting_copilot_domain::vo::TitleText::new("未生成结论").unwrap_or_else(|_| {
            meeting_copilot_domain::vo::TitleText::new("待确认").expect("常量合法")
        }),
        conclusion: None,
        references: Vec::new(),
        confidence: confidence(),
        conflict: None,
        notice: Some(notice),
    }
}

fn minutes(input: &FallbackInput) -> MinutesDraft {
    MinutesDraft {
        summary: UtteranceText::new("未生成纪要")
            .unwrap_or_else(|_| UtteranceText::new("空").expect("常量合法")),
        insights: MinutesInsights {
            key_info: Vec::new(),
            decision: Vec::new(),
            todo: Vec::new(),
            question: Vec::new(),
        },
        timeline: Timeline {
            start: None,
            end: None,
            entries: Vec::new(),
        },
        open_questions: vec![PendingItem {
            text: ShortText::truncated("待确认"),
            reason: PendingReason::Unrefined,
        }],
        sources: input.sources.clone(),
        unrefined_count: 1,
        completion_hint: Some(CompletionHint::MoreUtterances),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn input() -> FallbackInput {
        FallbackInput {
            sources: vec![UtteranceRef::new("utt-1").expect("ref")],
            window_text: Some("下周三开始灰度上线".into()),
            last_summary: None,
        }
    }

    #[test]
    fn insight_has_source_and_low_confidence() {
        let body = RuleFallback::produce(LlmTask::InsightExtract, &input());
        match body {
            StructuredBody::Insight(result) => {
                assert!(!result.insights.is_empty());
                assert!(!result.insights[0].sources.is_empty());
                assert!(result.insights[0].confidence.value() <= 0.5);
                assert_eq!(result.insights[0].sources[0].as_str(), "utt-1");
            }
            other => panic!("类型错误：{other:?}"),
        }
    }

    #[test]
    fn summary_without_history_is_not_produced() {
        match RuleFallback::produce(LlmTask::Summary, &input()) {
            StructuredBody::Summary(result) => {
                assert!(!result.produced);
                assert!(!result.carried_over);
            }
            other => panic!("类型错误：{other:?}"),
        }
    }

    #[test]
    fn summary_with_history_is_carried_over() {
        let previous = Summary {
            purpose: SummaryPurpose::RollingWindow,
            text: UtteranceText::new("上一版摘要").expect("text"),
            covered: None,
            version: 7,
            carried_over: false,
            produced: true,
        };
        let input = FallbackInput {
            last_summary: Some(previous.clone()),
            ..input()
        };
        match RuleFallback::produce(LlmTask::Summary, &input) {
            StructuredBody::Summary(result) => {
                assert!(result.carried_over);
                assert_eq!(result.version, previous.version);
                assert_eq!(result.text, previous.text);
            }
            other => panic!("类型错误：{other:?}"),
        }
    }

    #[test]
    fn conclusion_is_always_empty() {
        match RuleFallback::produce(LlmTask::CardConclusion, &input()) {
            StructuredBody::Conclusion(result) => {
                assert!(result.conclusion.is_none());
                assert!(result.notice.is_some());
            }
            other => panic!("类型错误：{other:?}"),
        }
    }

    #[test]
    fn minutes_have_skeleton_and_open_questions() {
        match RuleFallback::produce(LlmTask::MinutesDraft, &input()) {
            StructuredBody::Minutes(result) => {
                assert!(!result.open_questions.is_empty());
                assert!(result.unrefined_count > 0);
                assert!(result.completion_hint.is_some());
            }
            other => panic!("类型错误：{other:?}"),
        }
    }

    #[test]
    fn empty_input_never_panics() {
        let empty = FallbackInput::default();
        for task in LlmTask::ALL {
            let body = RuleFallback::produce(task, &empty);
            assert_eq!(body.task(), task);
        }
    }

    #[test]
    fn repeated_calls_are_deterministic_for_stateless_tasks() {
        let first = RuleFallback::produce(LlmTask::InsightExtract, &input());
        let second = RuleFallback::produce(LlmTask::InsightExtract, &input());
        assert_eq!(first, second);
    }

    #[test]
    fn fallback_runs_within_fifty_millis_p95() {
        let input = FallbackInput {
            sources: (0..200)
                .map(|index| UtteranceRef::new(format!("utt-{index}")).expect("ref"))
                .collect(),
            window_text: Some("字".repeat(2_000)),
            last_summary: None,
        };
        for task in LlmTask::ALL {
            for _ in 0..20 {
                let start = Instant::now();
                let _ = RuleFallback::produce(task, &input);
                assert!(start.elapsed().as_millis() <= 50);
            }
        }
    }
}
