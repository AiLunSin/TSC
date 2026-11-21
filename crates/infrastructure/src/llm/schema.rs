//! Schema 派生与严格解码（DESIGN 2.2.2-F、2.4.2 AC-01/AC-07/AC-13）。
//!
//! 类型是唯一真源：Schema 由领域类型派生，解码走 serde 严格模式。

use std::sync::OnceLock;

use meeting_copilot_domain::model::llm::{
    CardConclusion, InsightExtract, LlmTask, MinutesDraft, StructuredBody, Summary,
};
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaRule {
    NotJson,
    UnknownField,
    TypeMismatch,
    RangeViolation,
    CrossField,
}

impl SchemaRule {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotJson => "NotJson",
            Self::UnknownField => "UnknownField",
            Self::TypeMismatch => "TypeMismatch",
            Self::RangeViolation => "RangeViolation",
            Self::CrossField => "CrossField",
        }
    }
}

/// 结构化解码错误：只含规则与字段路径，绝不含载荷片段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaError {
    pub rule: SchemaRule,
    pub path: String,
}

impl SchemaError {
    fn new(rule: SchemaRule, path: impl Into<String>) -> Self {
        Self {
            rule,
            path: path.into(),
        }
    }
}

/// 四类 Schema 的只读注册表。
pub struct SchemaRegistry;

static SCHEMAS: OnceLock<[Value; 4]> = OnceLock::new();

fn schemas() -> &'static [Value; 4] {
    SCHEMAS.get_or_init(|| {
        [
            to_schema::<InsightExtract>(),
            to_schema::<Summary>(),
            to_schema::<CardConclusion>(),
            to_schema::<MinutesDraft>(),
        ]
    })
}

fn to_schema<T: schemars::JsonSchema>() -> Value {
    serde_json::to_value(schemars::schema_for!(T)).unwrap_or(Value::Null)
}

impl SchemaRegistry {
    pub fn json_schema(task: LlmTask) -> &'static Value {
        match task {
            LlmTask::InsightExtract => &schemas()[0],
            LlmTask::Summary => &schemas()[1],
            LlmTask::CardConclusion => &schemas()[2],
            LlmTask::MinutesDraft => &schemas()[3],
        }
    }

    /// 严格解码：未知字段、类型、范围与跨字段规则均拒绝。
    pub fn decode(task: LlmTask, raw: &str) -> Result<StructuredBody, SchemaError> {
        let value: Value =
            serde_json::from_str(raw).map_err(|_| SchemaError::new(SchemaRule::NotJson, "body"))?;
        check_unknown(task, &value)?;
        match task {
            LlmTask::InsightExtract => {
                let body: InsightExtract = from_value(value)?;
                Ok(StructuredBody::Insight(body))
            }
            LlmTask::Summary => {
                let body: Summary = from_value(value)?;
                Ok(StructuredBody::Summary(body))
            }
            LlmTask::CardConclusion => {
                let body: CardConclusion = from_value(value)?;
                if body.conclusion.is_some() && body.references.is_empty() {
                    return Err(SchemaError::new(SchemaRule::CrossField, "references"));
                }
                Ok(StructuredBody::Conclusion(body))
            }
            LlmTask::MinutesDraft => {
                let body: MinutesDraft = from_value(value)?;
                if contains_pending(&body.insights) {
                    return Err(SchemaError::new(SchemaRule::CrossField, "insights.待确认"));
                }
                Ok(StructuredBody::Minutes(body))
            }
        }
    }

    /// 修复提示只输出 `path` 与 `rule` 两字段（AC-25）。
    pub fn repair_hint(error: &SchemaError) -> String {
        serde_json::to_string(&json!({
            "path": error.path,
            "rule": error.rule.as_str(),
        }))
        .unwrap_or_else(|_| String::from("{\"path\":\"body\",\"rule\":\"CrossField\"}"))
    }
}

fn from_value<T>(value: Value) -> Result<T, SchemaError>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(value).map_err(|error| {
        let message = error.to_string();
        let rule = if message.contains("invalid type") {
            SchemaRule::TypeMismatch
        } else if message.contains("长度为") || message.contains("置信度") {
            SchemaRule::RangeViolation
        } else {
            SchemaRule::CrossField
        };
        SchemaError::new(rule, "body")
    })
}

fn contains_pending(insights: &meeting_copilot_domain::model::llm::MinutesInsights) -> bool {
    let marked = |texts: &[meeting_copilot_domain::vo::ShortText]| {
        texts.iter().any(|text| text.as_str().contains("待确认"))
    };
    marked(&insights.key_info) || marked(&insights.decision) || marked(&insights.todo)
}

fn allowed_keys(task: LlmTask) -> &'static [&'static str] {
    match task {
        LlmTask::InsightExtract => &["insights"],
        LlmTask::Summary => &[
            "purpose",
            "text",
            "covered",
            "version",
            "carried_over",
            "produced",
        ],
        LlmTask::CardConclusion => &[
            "title",
            "conclusion",
            "references",
            "confidence",
            "conflict",
            "notice",
        ],
        LlmTask::MinutesDraft => &[
            "summary",
            "insights",
            "timeline",
            "open_questions",
            "sources",
            "unrefined_count",
            "completion_hint",
        ],
    }
}

fn check_unknown(task: LlmTask, value: &Value) -> Result<(), SchemaError> {
    let object = value
        .as_object()
        .ok_or_else(|| SchemaError::new(SchemaRule::TypeMismatch, "body"))?;
    for key in object.keys() {
        if !allowed_keys(task).contains(&key.as_str()) {
            return Err(SchemaError::new(SchemaRule::UnknownField, key.clone()));
        }
    }
    match task {
        LlmTask::InsightExtract => {
            if let Some(items) = value.get("insights").and_then(Value::as_array) {
                for (index, item) in items.iter().enumerate() {
                    check_keys(
                        item,
                        &["kind", "text", "sources", "confidence", "decision", "todo"],
                        &format!("insights[{index}]"),
                    )?;
                }
            }
        }
        LlmTask::MinutesDraft => {
            if let Some(insights) = value.get("insights") {
                check_keys(
                    insights,
                    &["key_info", "decision", "todo", "question"],
                    "insights",
                )?;
            }
            if let Some(timeline) = value.get("timeline") {
                check_keys(timeline, &["start", "end", "entries"], "timeline")?;
                if let Some(entries) = timeline.get("entries").and_then(Value::as_array) {
                    for (index, entry) in entries.iter().enumerate() {
                        check_keys(
                            entry,
                            &["at", "text"],
                            &format!("timeline.entries[{index}]"),
                        )?;
                    }
                }
            }
            if let Some(items) = value.get("open_questions").and_then(Value::as_array) {
                for (index, item) in items.iter().enumerate() {
                    check_keys(
                        item,
                        &["text", "reason"],
                        &format!("open_questions[{index}]"),
                    )?;
                }
            }
        }
        LlmTask::Summary => {
            if let Some(covered) = value.get("covered") {
                if !covered.is_null() {
                    check_keys(covered, &["start_ms", "end_ms"], "covered")?;
                }
            }
        }
        LlmTask::CardConclusion => {}
    }
    Ok(())
}

fn check_keys(value: &Value, allowed: &[&str], path: &str) -> Result<(), SchemaError> {
    let object = value
        .as_object()
        .ok_or_else(|| SchemaError::new(SchemaRule::TypeMismatch, path))?;
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(SchemaError::new(
                SchemaRule::UnknownField,
                format!("{path}.{key}"),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENSITIVE: &str = "SENSITIVE-TOKEN";

    fn valid_insight() -> String {
        json!({
            "insights": [{
                "kind": "key_info",
                "text": "确认下周灰度上线",
                "sources": ["utt-1"],
                "confidence": 0.8,
                "decision": null,
                "todo": null
            }]
        })
        .to_string()
    }

    #[test]
    fn decodes_four_valid_shapes() {
        assert!(SchemaRegistry::decode(LlmTask::InsightExtract, &valid_insight()).is_ok());
        assert!(SchemaRegistry::decode(
            LlmTask::Summary,
            &json!({
                "purpose": "rolling_window",
                "text": "已讨论灰度计划",
                "covered": null,
                "version": 2,
                "carried_over": false,
                "produced": true
            })
            .to_string()
        )
        .is_ok());
        assert!(SchemaRegistry::decode(
            LlmTask::CardConclusion,
            &json!({
                "title": "灰度策略",
                "conclusion": null,
                "references": [],
                "confidence": 0.4,
                "conflict": null,
                "notice": "generation_unavailable"
            })
            .to_string()
        )
        .is_ok());
        assert!(SchemaRegistry::decode(
            LlmTask::MinutesDraft,
            &json!({
                "summary": "本次会议讨论了灰度",
                "insights": {"key_info": [], "decision": [], "todo": [], "question": []},
                "timeline": {"start": null, "end": null, "entries": []},
                "open_questions": [{"text": "上线时间待定", "reason": "awaiting_due"}],
                "sources": ["utt-1"],
                "unrefined_count": 1,
                "completion_hint": "more_utterances"
            })
            .to_string()
        )
        .is_ok());
    }

    #[test]
    fn rejects_non_json() {
        let error = SchemaRegistry::decode(LlmTask::Summary, "不是 JSON").expect_err("必须拒绝");
        assert_eq!(error.rule, SchemaRule::NotJson);
    }

    #[test]
    fn rejects_unknown_field_with_path() {
        let raw = json!({
            "purpose": "rolling_window",
            "text": "摘要",
            "covered": null,
            "version": 1,
            "carried_over": false,
            "produced": true,
            "hallucinated": "x"
        })
        .to_string();
        let error = SchemaRegistry::decode(LlmTask::Summary, &raw).expect_err("必须拒绝");
        assert_eq!(error.rule, SchemaRule::UnknownField);
        assert_eq!(error.path, "hallucinated");
    }

    #[test]
    fn rejects_type_mismatch() {
        let raw = json!({
            "insights": [{
                "kind": "key_info",
                "text": "要点",
                "sources": ["utt-1"],
                "confidence": "很高"
            }]
        })
        .to_string();
        let error = SchemaRegistry::decode(LlmTask::InsightExtract, &raw).expect_err("必须拒绝");
        assert_eq!(error.rule, SchemaRule::TypeMismatch);
    }

    #[test]
    fn rejects_range_violation() {
        let raw = json!({
            "insights": [{
                "kind": "key_info",
                "text": "字".repeat(61),
                "sources": ["utt-1"],
                "confidence": 0.5
            }]
        })
        .to_string();
        let error = SchemaRegistry::decode(LlmTask::InsightExtract, &raw).expect_err("必须拒绝");
        assert_eq!(error.rule, SchemaRule::RangeViolation);
    }

    #[test]
    fn rejects_conclusion_without_reference() {
        let raw = json!({
            "title": "结论标题",
            "conclusion": "这是一句结论",
            "references": [],
            "confidence": 0.5
        })
        .to_string();
        let error = SchemaRegistry::decode(LlmTask::CardConclusion, &raw).expect_err("必须拒绝");
        assert_eq!(error.rule, SchemaRule::CrossField);
    }

    #[test]
    fn rejects_pending_in_minutes_insights() {
        let raw = json!({
            "summary": "摘要",
            "insights": {"key_info": ["待确认"], "decision": [], "todo": [], "question": []},
            "timeline": {"start": null, "end": null, "entries": []},
            "open_questions": [{"text": "待确认", "reason": "unrefined"}],
            "sources": ["utt-1"],
            "unrefined_count": 1
        })
        .to_string();
        let error = SchemaRegistry::decode(LlmTask::MinutesDraft, &raw).expect_err("必须拒绝");
        assert_eq!(error.rule, SchemaRule::CrossField);
    }

    #[test]
    fn error_path_never_contains_payload() {
        let raw = json!({
            "purpose": "rolling_window",
            "text": SENSITIVE,
            "covered": null,
            "version": 1,
            "carried_over": false,
            "produced": true,
            "extra": SENSITIVE
        })
        .to_string();
        let error = SchemaRegistry::decode(LlmTask::Summary, &raw).expect_err("必须拒绝");
        assert!(!error.path.contains(SENSITIVE));
        let hint = SchemaRegistry::repair_hint(&error);
        assert!(!hint.contains(SENSITIVE));
        assert!(hint.contains("\"path\""));
        assert!(hint.contains("\"rule\""));
    }

    #[test]
    fn schemas_are_stable_and_closed() {
        for task in LlmTask::ALL {
            let schema = SchemaRegistry::json_schema(task);
            assert_eq!(schema["additionalProperties"], json!(false));
            assert!(schema.get("properties").is_some());
        }
    }
}
