//! 阶段 G 收口用例：wire 边界、请求体契约、预算与归一相关纯函数。

use meeting_copilot_domain::model::enums::InsightKind;
use meeting_copilot_domain::model::llm::{
    DeadlineMs, LlmGenerationRequest, LlmTask, OutputBudget, PendingReason, RedactedPayload,
    StructuredBody, SummaryPurpose, TemperatureMilli,
};
use meeting_copilot_domain::vo::{TaskId, UtteranceText};
use meeting_copilot_infrastructure::llm::gateway::{
    json_instruction_verdict, request_body, thinking_verdict, THINKING_DISABLED_VALUE,
    THINKING_FIELD,
};
use meeting_copilot_infrastructure::llm::wire::decode_structured;
use meeting_copilot_infrastructure::llm::LlmConfig;
use serde_json::json;

fn request(task: LlmTask) -> LlmGenerationRequest {
    let purpose = (task == LlmTask::Summary).then_some(SummaryPurpose::RollingWindow);
    LlmGenerationRequest::new(
        TaskId::new(format!("strict-{}", task.as_str())).expect("id"),
        RedactedPayload::new("会议上下文", task, 8_000).expect("payload"),
        purpose,
        OutputBudget::new(512).expect("output"),
        TemperatureMilli::new(200).expect("temperature"),
        DeadlineMs::new(8_000).expect("deadline"),
    )
    .expect("request")
}

#[test]
fn summary_wire_missing_metadata_ok() {
    let body = decode_structured(LlmTask::Summary, r#"{"text":"摘要内容","covered":null}"#)
        .expect("decode");
    assert!(matches!(body, StructuredBody::Summary(_)));
}

#[test]
fn summary_wire_unknown_field_err() {
    assert!(decode_structured(LlmTask::Summary, r#"{"text":"摘要","extra":1}"#).is_err());
}

#[test]
fn summary_wire_long_text_err() {
    let long = "字".repeat(UtteranceText::LIMIT + 1);
    let raw = json!({"text": long}).to_string();
    assert!(decode_structured(LlmTask::Summary, &raw).is_err());
}

#[test]
fn minutes_wire_missing_containers_ok() {
    let body = decode_structured(
        LlmTask::MinutesDraft,
        r#"{"summary":"纪要内容","open_questions":[]}"#,
    )
    .expect("decode");
    match body {
        StructuredBody::Minutes(minutes) => {
            assert!(minutes.insights.key_info.is_empty());
            assert!(minutes.timeline.entries.is_empty());
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn minutes_wire_nested_long_text_truncated() {
    let long = "字".repeat(61);
    let raw = json!({
        "summary":"纪要",
        "insights":{"key_info":[long],"decision":[],"todo":[],"question":[]}
    })
    .to_string();
    match decode_structured(LlmTask::MinutesDraft, &raw).expect("decode") {
        StructuredBody::Minutes(minutes) => {
            assert_eq!(minutes.insights.key_info[0].char_count(), 60);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn minutes_wire_invalid_sources_filtered() {
    let raw = json!({
        "summary":"纪要",
        "sources":["bad source with space"]
    })
    .to_string();
    match decode_structured(LlmTask::MinutesDraft, &raw).expect("decode") {
        StructuredBody::Minutes(minutes) => assert!(minutes.sources.is_empty()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn minutes_wire_unknown_reason_maps_unrefined() {
    let raw = json!({
        "summary":"纪要",
        "open_questions":[{"text":"待确认","reason":"weird"}]
    })
    .to_string();
    match decode_structured(LlmTask::MinutesDraft, &raw).expect("decode") {
        StructuredBody::Minutes(minutes) => {
            assert_eq!(minutes.open_questions[0].reason, PendingReason::Unrefined);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn card_wire_long_title_truncated() {
    let raw = json!({
        "title":"标".repeat(50),
        "conclusion":null,
        "references":[],
        "confidence":0.5
    })
    .to_string();
    match decode_structured(LlmTask::CardConclusion, &raw).expect("decode") {
        StructuredBody::Conclusion(card) => {
            assert_eq!(card.title.as_str().chars().count(), 40);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn card_wire_long_conclusion_truncated() {
    let raw = json!({
        "title":"标题",
        "conclusion":"结".repeat(100),
        "references":[],
        "confidence":0.5
    })
    .to_string();
    match decode_structured(LlmTask::CardConclusion, &raw).expect("decode") {
        StructuredBody::Conclusion(card) => {
            assert_eq!(
                card.conclusion
                    .expect("conclusion")
                    .as_str()
                    .chars()
                    .count(),
                80
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn card_wire_invalid_references_filtered() {
    let raw = json!({
        "title":"标题",
        "conclusion":null,
        "references":["bad ref with space"],
        "confidence":0.5
    })
    .to_string();
    match decode_structured(LlmTask::CardConclusion, &raw).expect("decode") {
        StructuredBody::Conclusion(card) => assert!(card.references.is_empty()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn card_wire_confidence_clamped() {
    let raw = json!({
        "title":"标题",
        "conclusion":null,
        "references":[],
        "confidence":9.0
    })
    .to_string();
    match decode_structured(LlmTask::CardConclusion, &raw).expect("decode") {
        StructuredBody::Conclusion(card) => assert_eq!(card.confidence.value(), 1.0),
        other => panic!("{other:?}"),
    }
}

#[test]
fn insight_wire_unknown_kind_maps_key_info() {
    let raw = json!({
        "insights":[{"kind":"diagnostic","text":"要点","sources":[],"confidence":0.8}]
    })
    .to_string();
    match decode_structured(LlmTask::InsightExtract, &raw).expect("decode") {
        StructuredBody::Insight(insight) => {
            assert_eq!(insight.insights[0].kind, InsightKind::KeyInfo);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn insight_wire_long_text_truncated() {
    let raw = json!({
        "insights":[{"kind":"key_info","text":"字".repeat(100),"sources":[],"confidence":0.8}]
    })
    .to_string();
    match decode_structured(LlmTask::InsightExtract, &raw).expect("decode") {
        StructuredBody::Insight(insight) => {
            assert_eq!(insight.insights[0].text.char_count(), 60);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn insight_wire_invalid_sources_filtered() {
    let raw = json!({
        "insights":[{"kind":"key_info","text":"要点","sources":["bad source"],"confidence":0.8}]
    })
    .to_string();
    match decode_structured(LlmTask::InsightExtract, &raw).expect("decode") {
        StructuredBody::Insight(insight) => assert!(insight.insights[0].sources.is_empty()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn insight_wire_confidence_clamped() {
    let raw = json!({
        "insights":[{"kind":"key_info","text":"要点","sources":[],"confidence":9.0}]
    })
    .to_string();
    match decode_structured(LlmTask::InsightExtract, &raw).expect("decode") {
        StructuredBody::Insight(insight) => {
            assert_eq!(insight.insights[0].confidence.value(), 1.0);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn request_body_contains_schema_and_json() {
    let body = request_body("test-model", &request(LlmTask::Summary), false);
    let system = body["messages"][0]["content"].as_str().expect("system");
    assert!(system.contains("\"properties\""));
    assert!(system.contains("\"required\""));
    assert!(system.contains("JSON"));
    assert!(json_instruction_verdict(&body).is_none());
}

#[test]
fn request_body_thinking_disabled() {
    let body = request_body("test-model", &request(LlmTask::InsightExtract), true);
    assert_eq!(body[THINKING_FIELD]["type"], THINKING_DISABLED_VALUE);
    assert!(thinking_verdict(&body).is_none());
}

#[test]
fn json_instruction_missing_is_detected() {
    let mut body = request_body("test-model", &request(LlmTask::Summary), false);
    body["messages"][0]["content"] = json!("只输出结构化内容");
    assert_eq!(
        json_instruction_verdict(&body),
        Some("json_instruction_missing")
    );
}

#[test]
fn config_default_max_request_bytes() {
    assert_eq!(LlmConfig::default().max_request_bytes, 65_536);
}

#[test]
fn config_max_request_bytes_rejects_out_of_range() {
    let low = LlmConfig {
        max_request_bytes: 512,
        ..Default::default()
    };
    assert!(low.validate().is_err());
    let ok = LlmConfig {
        max_request_bytes: 1_024,
        ..Default::default()
    };
    assert!(ok.validate().is_ok());
}

#[test]
fn minutes_wire_unknown_top_field_err() {
    assert!(decode_structured(LlmTask::MinutesDraft, r#"{"summary":"纪要","unknown":1}"#).is_err());
}
