mod support;

use meeting_copilot_domain::model::llm::LlmTask;
use meeting_copilot_infrastructure::llm::schema::SchemaRegistry;
use serde_json::Value;

fn properties(task: LlmTask) -> Vec<String> {
    let mut keys = SchemaRegistry::json_schema(task)["properties"]
        .as_object()
        .expect("properties")
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    keys.sort();
    keys
}

fn expected(mut keys: Vec<&str>) -> Vec<String> {
    keys.sort_unstable();
    keys.into_iter().map(str::to_owned).collect()
}

#[test]
fn four_schemas_are_derived_from_types() {
    for task in LlmTask::ALL {
        let body = match task {
            LlmTask::InsightExtract => support::insight_body(),
            LlmTask::Summary => support::summary_body(),
            LlmTask::CardConclusion => support::conclusion_body(),
            LlmTask::MinutesDraft => support::minutes_body(),
        };
        let decoded = SchemaRegistry::decode(task, &body).expect("合法样例");
        assert_eq!(decoded.task(), task);
        assert_eq!(decoded.schema_version(), task.schema_version());
    }
}

#[test]
fn schema_property_keys_match_domain_types() {
    assert_eq!(
        properties(LlmTask::InsightExtract),
        expected(vec!["insights"])
    );
    assert_eq!(
        properties(LlmTask::Summary),
        expected(vec![
            "purpose",
            "text",
            "covered",
            "version",
            "carried_over",
            "produced"
        ])
    );
    assert_eq!(
        properties(LlmTask::CardConclusion),
        expected(vec![
            "title",
            "conclusion",
            "references",
            "confidence",
            "conflict",
            "notice"
        ])
    );
    assert_eq!(
        properties(LlmTask::MinutesDraft),
        expected(vec![
            "summary",
            "insights",
            "timeline",
            "open_questions",
            "sources",
            "unrefined_count",
            "completion_hint"
        ])
    );
}

#[test]
fn schemas_are_closed_and_have_expected_metadata() {
    for task in LlmTask::ALL {
        let schema = SchemaRegistry::json_schema(task);
        assert_eq!(schema["additionalProperties"], Value::Bool(false));
        assert!(schema["properties"].is_object());
        let normalized = serde_json::to_string(schema).expect("序列化");
        assert!(normalized.len() > 32);
        assert!(!normalized.contains("SENSITIVE-TOKEN"));
    }
}

#[test]
fn illegal_samples_are_rejected_per_task() {
    let cases = [
        (LlmTask::InsightExtract, "{\"insights\":[{\"text\":1}]}"),
        (LlmTask::Summary, "{\"purpose\":\"rolling_window\"}"),
        (
            LlmTask::CardConclusion,
            "{\"title\":\"t\",\"conclusion\":\"c\",\"references\":[],\"confidence\":0.5}",
        ),
        (LlmTask::MinutesDraft, "{\"summary\":\"s\"}"),
    ];
    for (task, raw) in cases {
        assert!(SchemaRegistry::decode(task, raw).is_err(), "{task:?}");
    }
}

#[test]
fn schema_versions_are_distinct_and_stable() {
    let versions: std::collections::HashSet<_> = LlmTask::ALL
        .iter()
        .map(|task| task.schema_version())
        .collect();
    assert_eq!(versions.len(), 4);
    for task in LlmTask::ALL {
        assert_eq!(task.schema_version(), task.schema_version());
    }
}
