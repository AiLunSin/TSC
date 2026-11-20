use std::collections::BTreeSet;
use std::path::Path;

use meeting_copilot_ui_bridge::dto::{
    ChannelStatusDto, IpcErrorDto, SessionSummaryDto, SuggestionCardDto,
};
use serde_json::Value;

fn key_set(value: &Value) -> BTreeSet<String> {
    value
        .as_object()
        .expect("DTO 必须序列化为对象")
        .keys()
        .cloned()
        .collect()
}

fn schema_entry(schema: &Value, name: &str) -> BTreeSet<String> {
    schema
        .get(name)
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("schema.json 缺少 {name}"))
        .iter()
        .map(|item| item.as_str().expect("字段名必须为字符串").to_owned())
        .collect()
}

#[test]
fn dto_fields_match_schema_contract() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../app/src/bridge/schema.json");
    let raw = std::fs::read_to_string(&path).expect("读取 schema.json");
    let schema: Value = serde_json::from_str(&raw).expect("解析 schema.json");

    let session = serde_json::to_value(SessionSummaryDto {
        session_id: "sess-1".into(),
        platform: "zoom".into(),
        state: "listening".into(),
        started_at_ms: 1_000,
        channel: "subtitle".into(),
    })
    .expect("序列化 SessionSummaryDto");

    let channel = serde_json::to_value(ChannelStatusDto {
        name: "字幕文本通道".into(),
        state: "ready".into(),
        detail: "已连接".into(),
    })
    .expect("序列化 ChannelStatusDto");

    let card = serde_json::to_value(SuggestionCardDto {
        card_id: "card-1".into(),
        kind: "fact".into(),
        title: "SLA 条款".into(),
        conclusion: None,
        trust: "high".into(),
        source_label: "合同 §7.2".into(),
        shown_at_ms: 1_000,
    })
    .expect("序列化 SuggestionCardDto");

    let error = serde_json::to_value(IpcErrorDto {
        code: "E-SEARCH-005".into(),
        message: "检索失败".into(),
        retryable: true,
    })
    .expect("序列化 IpcErrorDto");

    for (name, value) in [
        ("SessionSummaryDto", session),
        ("ChannelStatusDto", channel),
        ("SuggestionCardDto", card),
        ("IpcErrorDto", error),
    ] {
        assert_eq!(
            key_set(&value),
            schema_entry(&schema, name),
            "{name} 的序列化字段与 schema.json 不一致"
        );
    }
}
