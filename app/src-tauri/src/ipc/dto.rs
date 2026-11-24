//! 6 组只读 DTO 与 4 个输入 DTO：全部 camelCase、拒绝未知字段。
//!
//! 字段数固定为 11/5/4/3/10/4/5；新增字段视为结构违规。

use serde::{Deserialize, Serialize};

/// 设置快照：恰 11 项。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingsSnapshotDto {
    pub form_state: String,
    pub has_key: bool,
    pub key_display: String,
    pub model_choice: ModelChoiceDto,
    pub endpoint_choice: EndpointChoiceDto,
    pub inline_errors: Vec<String>,
    pub can_test: bool,
    pub can_save: bool,
    pub sections_order: Vec<String>,
    pub advanced_collapsed: bool,
    pub probe_report: Option<ProbeReportDto>,
}

/// 探针报告：恰 5 项。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProbeReportDto {
    pub succeeded: bool,
    pub failure: Option<String>,
    pub message: String,
    pub elapsed_ms: u64,
    pub remaining_ms: Option<u64>,
}

/// 保存回执：恰 4 项。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingsSaveReceiptDto {
    pub persisted: bool,
    pub fallback: Option<String>,
    pub has_key: bool,
    pub notice: Option<String>,
}

/// 清除回执：恰 3 项。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingsClearReceiptDto {
    pub cleared: bool,
    pub has_key: bool,
    pub notice: Option<String>,
}

/// 卡片快照：恰 10 项。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KeywordCardViewDto {
    pub card_id: String,
    pub keyword_kind: String,
    pub trigger: String,
    pub triggered_at_ms: u64,
    pub card_status: String,
    pub body_visible: bool,
    pub body: Option<CardBodyDto>,
    pub click_state: String,
    pub disabled_reason: Option<String>,
    pub badge_count: u32,
}

/// 卡片正文：恰 4 项。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CardBodyDto {
    pub title: String,
    pub conclusion: ConclusionDto,
    pub confidence: f64,
    pub reference_ids: Vec<String>,
}

/// 卡片详情：恰 5 项。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CardDetailDto {
    pub card_id: String,
    pub title: String,
    pub conclusion: ConclusionDto,
    pub reference_ids: Vec<String>,
    pub citation_alignment: String,
}

/// 模型选择：预设显示名或自定义名。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ModelChoiceDto {
    Preset { display_name: String },
    Custom { name: String },
}

/// 端点选择：目录内显示名或不可用。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum EndpointChoiceDto {
    Option { display_name: String },
    Unavailable,
}

/// 结论：可用文本、无可靠来源或缺失。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum ConclusionDto {
    Available { text: String },
    NoReliableSource,
    Absent,
}

/// 探针输入。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProbeInputDto {
    pub key: String,
    pub model_index: usize,
    pub endpoint_index: usize,
    pub timeout_ms: Option<u64>,
}

/// 保存输入。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveInputDto {
    pub key: String,
    pub model_index: usize,
    pub endpoint_index: usize,
    pub timeout_ms: Option<u64>,
}

/// 清除输入。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClearInputDto {
    pub confirm: bool,
}

/// 打开详情输入。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OpenInputDto {
    pub card_id: String,
}

/// 转写全文的一行；仅本次会话保留在内存。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TranscriptLineDto {
    pub index: u64,
    pub text: String,
    pub at_ms: i64,
}

/// 选中文字的释义请求；`context` 只带选中处前后各一句。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExplainInputDto {
    pub text: String,
    pub context: String,
}

/// 释义结果；`source` 为 `llm` 或 `local`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExplainResultDto {
    pub text: String,
    pub explanation: String,
    pub source: String,
}

/// 把释义落成卡片的请求。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IngestExplanationInputDto {
    pub word: String,
    pub explanation: String,
}

/// IPC 错误：与前端既有 3 项一致。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IpcErrorDto {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

impl IpcErrorDto {
    /// 固定短句错误；不得携带凭据、端点或内部码。
    pub fn new(code: &str, message: &str, retryable: bool) -> Self {
        Self {
            code: code.to_owned(),
            message: message.to_owned(),
            retryable,
        }
    }
}
