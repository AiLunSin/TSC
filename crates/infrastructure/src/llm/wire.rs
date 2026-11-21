//! 模型输出域（wire）解码：只要求模型可决定的字段，服务端元数据由网关归一。
//!
//! 严格性保留：未知字段拒绝；不用 `serde(default)` 掩盖模型域必填字段。

use meeting_copilot_domain::model::card::ConflictView;
use meeting_copilot_domain::model::enums::InsightKind;
use meeting_copilot_domain::model::insight::{DecisionFields, TodoFields};
use meeting_copilot_domain::model::llm::{
    CardConclusion, CompletionHint, ConclusionNotice, InsightExtract, InsightItem, LlmTask,
    MinutesDraft, MinutesInsights, PendingItem, PendingReason, StructuredBody, Summary,
    SummaryPurpose, Timeline, TimelineEntry,
};
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::vo::{
    AudioClock, ConclusionText, Confidence, ReferenceRef, ShortText, TitleText, UtteranceRef,
    UtteranceText,
};
use serde::Deserialize;
use serde_json::Value;

use super::schema::{SchemaError, SchemaRule};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SummaryWire {
    pub text: UtteranceText,
    #[serde(default)]
    pub covered: Option<TextRange>,
    #[serde(default)]
    pub purpose: Option<SummaryPurpose>,
    #[serde(default)]
    pub version: Option<u32>,
    #[serde(default)]
    pub carried_over: Option<bool>,
    #[serde(default)]
    pub produced: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MinutesWire {
    pub summary: String,
    #[serde(default)]
    pub insights: Option<MinutesInsightsWire>,
    #[serde(default)]
    pub timeline: Option<TimelineWire>,
    #[serde(default)]
    pub open_questions: Vec<PendingItemWire>,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub unrefined_count: Option<u32>,
    #[serde(default)]
    pub completion_hint: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MinutesInsightsWire {
    #[serde(default)]
    pub key_info: Vec<String>,
    #[serde(default)]
    pub decision: Vec<String>,
    #[serde(default)]
    pub todo: Vec<String>,
    #[serde(default)]
    pub question: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineWire {
    #[serde(default)]
    pub start: Option<u64>,
    #[serde(default)]
    pub end: Option<u64>,
    #[serde(default)]
    pub entries: Vec<TimelineEntryWire>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineEntryWire {
    #[serde(default)]
    pub at: Option<u64>,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingItemWire {
    pub text: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InsightExtractWire {
    pub insights: Vec<InsightItemWire>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InsightItemWire {
    pub kind: String,
    pub text: String,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub decision: Option<Value>,
    #[serde(default)]
    pub todo: Option<Value>,
}

fn kind_from_wire(raw: &str) -> InsightKind {
    match raw {
        "decision" => InsightKind::Decision,
        "todo" => InsightKind::Todo,
        "question" => InsightKind::Question,
        _ => InsightKind::KeyInfo,
    }
}

fn insight_from_wire(wire: InsightItemWire) -> InsightItem {
    let sources = wire
        .sources
        .iter()
        .filter_map(|raw| UtteranceRef::new(raw.clone()).ok())
        .collect();
    let confidence = Confidence::new(wire.confidence.unwrap_or(0.35).clamp(0.0, 1.0))
        .unwrap_or(Confidence::ZERO);
    InsightItem {
        kind: kind_from_wire(&wire.kind),
        text: ShortText::truncated(&wire.text),
        sources,
        confidence,
        decision: wire
            .decision
            .and_then(|value| serde_json::from_value::<DecisionFields>(value).ok()),
        todo: wire
            .todo
            .and_then(|value| serde_json::from_value::<TodoFields>(value).ok()),
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardConclusionWire {
    pub title: String,
    #[serde(default)]
    pub conclusion: Option<String>,
    #[serde(default)]
    pub references: Vec<String>,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub conflict: Option<Value>,
    #[serde(default)]
    pub notice: Option<ConclusionNotice>,
}

fn truncate_chars(value: &str, limit: usize) -> String {
    value.trim().chars().take(limit).collect()
}

fn card_from_wire(wire: CardConclusionWire) -> CardConclusion {
    let title_raw = truncate_chars(&wire.title, TitleText::LIMIT);
    let title = TitleText::new(title_raw)
        .unwrap_or_else(|_| TitleText::new("未生成结论").expect("常量合法"));
    let conclusion = wire.conclusion.and_then(|raw| {
        let text = truncate_chars(&raw, ConclusionText::LIMIT);
        if text.is_empty() {
            None
        } else {
            ConclusionText::new(text).ok()
        }
    });
    let references = wire
        .references
        .iter()
        .filter_map(|raw| ReferenceRef::new(raw.clone()).ok())
        .collect();
    let confidence = Confidence::new(wire.confidence.unwrap_or(0.35).clamp(0.0, 1.0))
        .unwrap_or(Confidence::ZERO);
    CardConclusion {
        title,
        conclusion,
        references,
        confidence,
        conflict: wire
            .conflict
            .and_then(|value| serde_json::from_value::<ConflictView>(value).ok()),
        notice: wire.notice,
    }
}

fn empty_minutes_insights() -> MinutesInsights {
    MinutesInsights {
        key_info: Vec::new(),
        decision: Vec::new(),
        todo: Vec::new(),
        question: Vec::new(),
    }
}

fn empty_timeline() -> Timeline {
    Timeline {
        start: None,
        end: None,
        entries: Vec::new(),
    }
}

fn normalize_minutes_summary(raw: String) -> UtteranceText {
    let text = truncate_chars(&raw, UtteranceText::LIMIT);
    if text.is_empty() {
        return UtteranceText::new("未生成纪要")
            .unwrap_or_else(|_| UtteranceText::new("空").expect("常量合法"));
    }
    UtteranceText::new(text).unwrap_or_else(|_| UtteranceText::new("未生成纪要").expect("常量合法"))
}

fn text_vec(values: Vec<String>) -> Vec<ShortText> {
    values
        .iter()
        .map(|value| ShortText::truncated(value))
        .collect()
}

fn reason_from_wire(raw: Option<&str>) -> PendingReason {
    match raw.unwrap_or("unrefined") {
        "awaiting_owner" => PendingReason::AwaitingOwner,
        "awaiting_due" => PendingReason::AwaitingDue,
        _ => PendingReason::Unrefined,
    }
}

fn minutes_from_wire(wire: MinutesWire) -> MinutesDraft {
    let insights = wire
        .insights
        .map(|value| MinutesInsights {
            key_info: text_vec(value.key_info),
            decision: text_vec(value.decision),
            todo: text_vec(value.todo),
            question: text_vec(value.question),
        })
        .unwrap_or_else(empty_minutes_insights);
    let timeline = wire
        .timeline
        .map(|value| Timeline {
            start: value.start.map(AudioClock::from_millis),
            end: value.end.map(AudioClock::from_millis),
            entries: value
                .entries
                .into_iter()
                .map(|entry| TimelineEntry {
                    at: AudioClock::from_millis(entry.at.unwrap_or(0)),
                    text: ShortText::truncated(&entry.text),
                })
                .collect(),
        })
        .unwrap_or_else(empty_timeline);
    let open_questions = wire
        .open_questions
        .into_iter()
        .map(|item| PendingItem {
            text: ShortText::truncated(&item.text),
            reason: reason_from_wire(item.reason.as_deref()),
        })
        .collect::<Vec<_>>();
    let sources = wire
        .sources
        .iter()
        .filter_map(|raw| UtteranceRef::new(raw.clone()).ok())
        .collect();
    let mut minutes = MinutesDraft {
        summary: normalize_minutes_summary(wire.summary),
        insights,
        timeline,
        open_questions,
        sources,
        unrefined_count: 0,
        completion_hint: None,
    };
    minutes.unrefined_count = minutes
        .open_questions
        .iter()
        .filter(|item| item.reason == PendingReason::Unrefined)
        .count() as u32;
    minutes.completion_hint = (minutes.unrefined_count > 0).then_some(CompletionHint::HumanReview);
    minutes
}

fn map_error(error: serde_json::Error) -> SchemaError {
    let message = error.to_string();
    let rule = if message.contains("unknown field") {
        SchemaRule::UnknownField
    } else if message.contains("invalid type") {
        SchemaRule::TypeMismatch
    } else if message.contains("长度为") || message.contains("置信度") {
        SchemaRule::RangeViolation
    } else {
        SchemaRule::CrossField
    };
    SchemaError {
        rule,
        path: "body".into(),
    }
}

pub fn decode_structured(task: LlmTask, raw: &str) -> Result<StructuredBody, SchemaError> {
    match task {
        LlmTask::Summary => {
            let value: Value = serde_json::from_str(raw).map_err(|_| SchemaError {
                rule: SchemaRule::NotJson,
                path: "body".into(),
            })?;
            let wire: SummaryWire = serde_json::from_value(value).map_err(map_error)?;
            Ok(StructuredBody::Summary(Summary {
                purpose: wire.purpose.unwrap_or(SummaryPurpose::RollingWindow),
                text: wire.text,
                covered: wire.covered,
                version: wire.version.unwrap_or(0),
                carried_over: wire.carried_over.unwrap_or(false),
                produced: wire.produced.unwrap_or(true),
            }))
        }
        LlmTask::MinutesDraft => {
            let value: Value = serde_json::from_str(raw).map_err(|_| SchemaError {
                rule: SchemaRule::NotJson,
                path: "body".into(),
            })?;
            let wire: MinutesWire = serde_json::from_value(value).map_err(map_error)?;
            Ok(StructuredBody::Minutes(minutes_from_wire(wire)))
        }
        LlmTask::InsightExtract => {
            let value: Value = serde_json::from_str(raw).map_err(|_| SchemaError {
                rule: SchemaRule::NotJson,
                path: "body".into(),
            })?;
            let wire: InsightExtractWire = serde_json::from_value(value).map_err(map_error)?;
            Ok(StructuredBody::Insight(InsightExtract {
                insights: wire.insights.into_iter().map(insight_from_wire).collect(),
            }))
        }
        LlmTask::CardConclusion => {
            let value: Value = serde_json::from_str(raw).map_err(|_| SchemaError {
                rule: SchemaRule::NotJson,
                path: "body".into(),
            })?;
            let wire: CardConclusionWire = serde_json::from_value(value).map_err(map_error)?;
            Ok(StructuredBody::Conclusion(card_from_wire(wire)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_wire_allows_missing_metadata() {
        let body = decode_structured(
            LlmTask::Summary,
            r#"{"text":"本次会议讨论了灰度","covered":null}"#,
        )
        .expect("wire");
        match body {
            StructuredBody::Summary(summary) => {
                assert_eq!(summary.text.as_str(), "本次会议讨论了灰度");
                assert_eq!(summary.version, 0);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn summary_wire_rejects_unknown() {
        assert!(decode_structured(LlmTask::Summary, r#"{"text":"摘要","extra":1}"#).is_err());
    }

    #[test]
    fn minutes_wire_derives_unrefined() {
        let body = decode_structured(
            LlmTask::MinutesDraft,
            r#"{
                "summary":"摘要",
                "insights":{"key_info":[],"decision":[],"todo":[],"question":[]},
                "timeline":{"start":null,"end":null,"entries":[]},
                "open_questions":[{"text":"待确认","reason":"unrefined"}]
            }"#,
        )
        .expect("wire");
        match body {
            StructuredBody::Minutes(minutes) => {
                assert_eq!(minutes.unrefined_count, 1);
                assert_eq!(minutes.completion_hint, Some(CompletionHint::HumanReview));
            }
            other => panic!("{other:?}"),
        }
    }
}
