//! 跨 IPC 的 DTO。业务 DTO 严格拒绝未知字段。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SessionSummaryDto {
    pub session_id: String,
    pub platform: String,
    pub state: String,
    pub started_at_ms: i64,
    pub channel: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChannelStatusDto {
    pub name: String,
    pub state: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SuggestionCardDto {
    pub card_id: String,
    pub kind: String,
    pub title: String,
    pub conclusion: Option<String>,
    pub trust: String,
    pub source_label: String,
    pub shown_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct IpcErrorDto {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn business_dto_rejects_unknown_fields() {
        let json = r#"{"cardId":"c1","kind":"fact","title":"t","conclusion":null,"trust":"high","sourceLabel":"s","shownAtMs":1,"unexpected":true}"#;
        assert!(serde_json::from_str::<SuggestionCardDto>(json).is_err());
    }

    #[test]
    fn camel_case_roundtrip() {
        let dto = ChannelStatusDto {
            name: "字幕文本通道".into(),
            state: "ready".into(),
            detail: "已连接".into(),
        };
        let json = serde_json::to_string(&dto).expect("序列化");
        assert!(json.contains("sourceLabel") || json.contains("state"));
        let back: ChannelStatusDto = serde_json::from_str(&json).expect("反序列化");
        assert_eq!(back, dto);
    }
}
