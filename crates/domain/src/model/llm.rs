//! LLM 网关领域类型（DESIGN 2.2.2-A~D、2.3）。
//!
//! 约束前移到构造期：任务、载荷、预算、截止时间与四类结果均以强类型表达。
//! 本模块不含任何网络、存储或具体服务商细节。

use std::fmt;
use std::time::Instant;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::model::card::ConflictView;
use crate::model::enums::InsightKind;
use crate::model::insight::{DecisionFields, TodoFields};
use crate::model::session::TextRange;
use crate::vo::{
    AudioClock, ConclusionText, Confidence, ReferenceRef, ShortText, TaskId, TitleText,
    UtteranceRef, UtteranceText,
};

/// 四类结构化生成任务（封闭枚举）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LlmTask {
    InsightExtract,
    Summary,
    CardConclusion,
    MinutesDraft,
}

impl LlmTask {
    pub const ALL: [Self; 4] = [
        Self::InsightExtract,
        Self::Summary,
        Self::CardConclusion,
        Self::MinutesDraft,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InsightExtract => "insight_extract",
            Self::Summary => "summary",
            Self::CardConclusion => "card_conclusion",
            Self::MinutesDraft => "minutes_draft",
        }
    }

    pub const fn schema_version(self) -> &'static str {
        match self {
            Self::InsightExtract => "llm.insight.v1",
            Self::Summary => "llm.summary.v1",
            Self::CardConclusion => "llm.conclusion.v1",
            Self::MinutesDraft => "llm.minutes.v1",
        }
    }

    /// 默认输出预算：输出 token 上限与温度（毫决定位）。
    pub const fn default_budget(self) -> TaskBudget {
        match self {
            Self::InsightExtract => TaskBudget {
                output: 512,
                temperature_milli: 200,
            },
            Self::Summary => TaskBudget {
                output: 400,
                temperature_milli: 150,
            },
            Self::CardConclusion => TaskBudget {
                output: 256,
                temperature_milli: 100,
            },
            Self::MinutesDraft => TaskBudget {
                output: 1024,
                temperature_milli: 200,
            },
        }
    }
}

/// 任务默认预算。构造期保证合法（见 [`TaskBudget::validate`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBudget {
    pub output: u16,
    pub temperature_milli: u16,
}

impl TaskBudget {
    pub fn validate(self) -> Result<Self, LlmModelError> {
        OutputBudget::new(self.output)?;
        TemperatureMilli::new(self.temperature_milli)?;
        Ok(self)
    }
}

/// 已脱敏载荷。构造期校验非空与字符数上限（按 `chars()` 计数）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactedPayload {
    text: String,
    task: LlmTask,
    char_limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LlmModelError {
    #[error("载荷为空")]
    EmptyPayload,
    #[error("载荷长度为 {actual}，超出上限 {limit}")]
    PayloadTooLong { actual: usize, limit: usize },
    #[error("输出上限必须落在 1..=4096，实际为 {0}")]
    InvalidOutputBudget(u16),
    #[error("温度必须落在 0..=1000 毫决定位，实际为 {0}")]
    InvalidTemperature(u16),
    #[error("截止时间必须落在 1..=8000 毫秒，实际为 {0}")]
    InvalidDeadline(u64),
    #[error("摘要任务必须且只能携带摘要用途")]
    PurposeMismatch,
    #[error("模型标识不能为空且不超过 128 字符")]
    InvalidModelId,
}

impl RedactedPayload {
    pub fn new(
        raw: impl Into<String>,
        task: LlmTask,
        char_limit: usize,
    ) -> Result<Self, LlmModelError> {
        let text = raw.into();
        let actual = text.chars().count();
        if actual == 0 {
            return Err(LlmModelError::EmptyPayload);
        }
        if char_limit == 0 || actual > char_limit {
            return Err(LlmModelError::PayloadTooLong {
                actual,
                limit: char_limit,
            });
        }
        Ok(Self {
            text,
            task,
            char_limit,
        })
    }

    pub const fn task(&self) -> LlmTask {
        self.task
    }

    pub fn char_count(&self) -> usize {
        self.text.chars().count()
    }

    pub const fn char_limit(&self) -> usize {
        self.char_limit
    }

    /// 仅供网关序列化提示词使用；不得进入日志、审计或错误文本。
    pub fn expose(&self) -> &str {
        &self.text
    }
}

/// 输出 token 上限（1..=4096）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct OutputBudget(u16);

impl OutputBudget {
    pub const MAX: u16 = 4096;

    pub fn new(value: u16) -> Result<Self, LlmModelError> {
        if value == 0 || value > Self::MAX {
            return Err(LlmModelError::InvalidOutputBudget(value));
        }
        Ok(Self(value))
    }

    pub const fn value(self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for OutputBudget {
    type Error = LlmModelError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<OutputBudget> for u16 {
    fn from(value: OutputBudget) -> Self {
        value.0
    }
}

/// 温度（0..=1000 毫决定位，即 0.000~1.000）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct TemperatureMilli(u16);

impl TemperatureMilli {
    pub const MAX: u16 = 1000;

    pub fn new(value: u16) -> Result<Self, LlmModelError> {
        if value > Self::MAX {
            return Err(LlmModelError::InvalidTemperature(value));
        }
        Ok(Self(value))
    }

    pub const fn value(self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for TemperatureMilli {
    type Error = LlmModelError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<TemperatureMilli> for u16 {
    fn from(value: TemperatureMilli) -> Self {
        value.0
    }
}

/// 单次生成的最长等待时间（1..=8000 毫秒）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct DeadlineMs(u64);

impl DeadlineMs {
    pub const MAX: u64 = 8_000;

    pub fn new(value: u64) -> Result<Self, LlmModelError> {
        if value == 0 || value > Self::MAX {
            return Err(LlmModelError::InvalidDeadline(value));
        }
        Ok(Self(value))
    }

    pub const fn as_millis(self) -> u64 {
        self.0
    }
}

impl TryFrom<u64> for DeadlineMs {
    type Error = LlmModelError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<DeadlineMs> for u64 {
    fn from(value: DeadlineMs) -> Self {
        value.0
    }
}

/// 摘要用途。`Summary` 任务必须携带，其余任务不得携带。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SummaryPurpose {
    RollingWindow,
    Minutes,
}

/// 结构化生成请求。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmGenerationRequest {
    pub request_id: TaskId,
    pub task: LlmTask,
    pub payload: RedactedPayload,
    pub purpose: Option<SummaryPurpose>,
    pub output: OutputBudget,
    pub temperature: TemperatureMilli,
    pub deadline: DeadlineMs,
}

impl LlmGenerationRequest {
    pub fn new(
        request_id: TaskId,
        payload: RedactedPayload,
        purpose: Option<SummaryPurpose>,
        output: OutputBudget,
        temperature: TemperatureMilli,
        deadline: DeadlineMs,
    ) -> Result<Self, LlmModelError> {
        let task = payload.task;
        match (task, purpose) {
            (LlmTask::Summary, Some(_)) => {}
            (LlmTask::Summary, None) => return Err(LlmModelError::PurposeMismatch),
            (_, Some(_)) => return Err(LlmModelError::PurposeMismatch),
            (_, None) => {}
        }
        Ok(Self {
            request_id,
            task,
            payload,
            purpose,
            output,
            temperature,
            deadline,
        })
    }
}

/// 要点抽取结果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InsightExtract {
    pub insights: Vec<InsightItem>,
}

impl InsightExtract {
    pub const fn schema_version() -> &'static str {
        LlmTask::InsightExtract.schema_version()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InsightItem {
    pub kind: InsightKind,
    #[schemars(with = "String")]
    pub text: ShortText,
    #[schemars(with = "Vec<String>")]
    pub sources: Vec<UtteranceRef>,
    #[schemars(with = "f64")]
    pub confidence: Confidence,
    #[serde(default)]
    #[schemars(with = "Option<serde_json::Value>")]
    pub decision: Option<DecisionFields>,
    #[serde(default)]
    #[schemars(with = "Option<serde_json::Value>")]
    pub todo: Option<TodoFields>,
}

/// 滚动摘要结果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    pub purpose: SummaryPurpose,
    #[schemars(with = "String")]
    pub text: UtteranceText,
    #[serde(default)]
    #[schemars(with = "Option<TextRangeShape>")]
    pub covered: Option<TextRange>,
    pub version: u32,
    pub carried_over: bool,
    pub produced: bool,
}

impl Summary {
    pub const fn schema_version() -> &'static str {
        LlmTask::Summary.schema_version()
    }
}

/// 提示卡结论结果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CardConclusion {
    #[schemars(with = "String")]
    pub title: TitleText,
    #[serde(default)]
    #[schemars(with = "Option<String>")]
    pub conclusion: Option<ConclusionText>,
    #[schemars(with = "Vec<String>")]
    pub references: Vec<ReferenceRef>,
    #[schemars(with = "f64")]
    pub confidence: Confidence,
    #[serde(default)]
    #[schemars(with = "Option<serde_json::Value>")]
    pub conflict: Option<ConflictView>,
    #[serde(default)]
    pub notice: Option<ConclusionNotice>,
}

impl CardConclusion {
    pub const fn schema_version() -> &'static str {
        LlmTask::CardConclusion.schema_version()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConclusionNotice {
    MissingSource,
    GenerationUnavailable,
}

/// 纪要草稿结果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MinutesDraft {
    #[schemars(with = "String")]
    pub summary: UtteranceText,
    pub insights: MinutesInsights,
    pub timeline: Timeline,
    pub open_questions: Vec<PendingItem>,
    #[schemars(with = "Vec<String>")]
    pub sources: Vec<UtteranceRef>,
    pub unrefined_count: u32,
    #[serde(default)]
    pub completion_hint: Option<CompletionHint>,
}

impl MinutesDraft {
    pub const fn schema_version() -> &'static str {
        LlmTask::MinutesDraft.schema_version()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MinutesInsights {
    #[schemars(with = "Vec<String>")]
    pub key_info: Vec<ShortText>,
    #[schemars(with = "Vec<String>")]
    pub decision: Vec<ShortText>,
    #[schemars(with = "Vec<String>")]
    pub todo: Vec<ShortText>,
    #[schemars(with = "Vec<String>")]
    pub question: Vec<ShortText>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Timeline {
    #[serde(default)]
    #[schemars(with = "Option<u64>")]
    pub start: Option<AudioClock>,
    #[serde(default)]
    #[schemars(with = "Option<u64>")]
    pub end: Option<AudioClock>,
    pub entries: Vec<TimelineEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TimelineEntry {
    #[schemars(with = "u64")]
    pub at: AudioClock,
    #[schemars(with = "String")]
    pub text: ShortText,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PendingItem {
    #[schemars(with = "String")]
    pub text: ShortText,
    pub reason: PendingReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PendingReason {
    AwaitingOwner,
    AwaitingDue,
    Unrefined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CompletionHint {
    MoreUtterances,
    HumanReview,
}

/// 四类结果的统一载体。
#[derive(Debug, Clone, PartialEq)]
pub enum StructuredBody {
    Insight(InsightExtract),
    Summary(Summary),
    Conclusion(CardConclusion),
    Minutes(MinutesDraft),
}

impl StructuredBody {
    pub const fn task(&self) -> LlmTask {
        match self {
            Self::Insight(_) => LlmTask::InsightExtract,
            Self::Summary(_) => LlmTask::Summary,
            Self::Conclusion(_) => LlmTask::CardConclusion,
            Self::Minutes(_) => LlmTask::MinutesDraft,
        }
    }

    pub const fn schema_version(&self) -> &'static str {
        self.task().schema_version()
    }
}

/// 六类封闭故障分类（不含任何自由文本）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureClass {
    Timeout,
    RateLimited,
    InvalidSchema,
    Unavailable,
    ContentFiltered,
    Offline,
}

impl FailureClass {
    pub const ALL: [Self; 6] = [
        Self::Timeout,
        Self::RateLimited,
        Self::InvalidSchema,
        Self::Unavailable,
        Self::ContentFiltered,
        Self::Offline,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Timeout => "timeout",
            Self::RateLimited => "rate_limited",
            Self::InvalidSchema => "invalid_schema",
            Self::Unavailable => "unavailable",
            Self::ContentFiltered => "content_filtered",
            Self::Offline => "offline",
        }
    }
}

/// 降级原因，与 [`FailureClass`] 分离。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DegradeCause {
    Failed(FailureClass),
    Cooling,
    QueueTimeout,
    PayloadTooLarge,
    AuditUnavailable,
    ConfigInvalid,
    BudgetExhausted,
}

/// 归一来源，仅用于可追溯，不含任何文本值。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizationSource {
    Request,
    Context,
    Computed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizationEntry {
    pub field: &'static str,
    pub source: NormalizationSource,
}

/// 一次性生成结果。
#[derive(Debug, Clone, PartialEq)]
pub struct StructuredOutcome {
    pub request_id: TaskId,
    pub task: LlmTask,
    pub body: Option<StructuredBody>,
    pub model: String,
    pub tokens_used: Option<u32>,
    pub degraded: bool,
    pub degrade_cause: Option<DegradeCause>,
    pub failure: Option<FailureClass>,
    pub attempts: u8,
    pub truncated: bool,
    pub schema_version: &'static str,
    /// 真实 HTTP 状态码；mock/离线路径为 `None`。
    pub http_status: Option<u16>,
    /// 服务端归一字段留痕（只有字段名与来源，不含文本）。
    pub normalization: Vec<NormalizationEntry>,
}

impl StructuredOutcome {
    /// 结果不变量：降级必有原因；未降级不得携带原因；`Failed` 原因与故障分类一致。
    pub fn invariants_hold(&self) -> bool {
        match (self.degraded, self.degrade_cause, self.failure) {
            (true, Some(DegradeCause::Failed(class)), Some(failure)) => class == failure,
            (true, Some(DegradeCause::Failed(_)), None) => false,
            (true, Some(_), _) => true,
            (false, None, None) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Termination {
    Completed(Box<StructuredOutcome>),
    Failed {
        failure: FailureClass,
        degraded: bool,
        cause: DegradeCause,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct StreamEvent {
    pub sequence: u32,
    pub kind: StreamEventKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StreamEventKind {
    Delta {
        text: String,
        first_token: bool,
    },
    Completed(Box<StructuredOutcome>),
    Failed {
        failure: FailureClass,
        degraded: bool,
        cause: DegradeCause,
    },
}

impl StreamEventKind {
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed(_) | Self::Failed { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamOutcome {
    pub request_id: TaskId,
    pub terminated: TerminationKind,
    pub discarded_deltas: u32,
    pub cancelled: bool,
    pub first_token_ms: Option<u32>,
}

/// 仅表达终止类型，详情在 [`StreamEventKind`] 中传递。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminationKind {
    Completed,
    Failed,
}

/// LLM 专用度量指标（不改动既有 `Metric` 十项）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmMetric {
    FirstTokenLatencyMs,
    TotalLatencyMs,
    RetryCount,
    FailureCount,
    DegradeCount,
    MetricsDropped,
}

impl LlmMetric {
    pub const ALL: [Self; 6] = [
        Self::FirstTokenLatencyMs,
        Self::TotalLatencyMs,
        Self::RetryCount,
        Self::FailureCount,
        Self::DegradeCount,
        Self::MetricsDropped,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::FirstTokenLatencyMs => "llm_first_token_latency_ms",
            Self::TotalLatencyMs => "llm_total_latency_ms",
            Self::RetryCount => "llm_retry_count",
            Self::FailureCount => "llm_failure_count",
            Self::DegradeCount => "llm_degrade_count",
            Self::MetricsDropped => "llm_metrics_dropped",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LlmMetricPoint {
    pub metric: LlmMetric,
    pub value: f64,
    pub task: Option<LlmTask>,
    pub failure_class: Option<FailureClass>,
}

/// 两阶段审计（意图 → 结果）。仅元数据，禁止载荷与响应正文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmAuditRecord {
    pub request_id: TaskId,
    pub phase: AuditPhase,
    pub task: LlmTask,
    pub chars: usize,
    pub model: String,
    pub endpoint: String,
    pub elapsed_ms: u64,
    pub outcome: String,
    pub degraded: bool,
    pub attempts: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditPhase {
    Intent,
    Result,
}

impl LlmAuditRecord {
    pub const fn phase_str(phase: AuditPhase) -> &'static str {
        match phase {
            AuditPhase::Intent => "intent",
            AuditPhase::Result => "result",
        }
    }

    /// 审计摘要只含元数据；不得出现载荷占位串或响应正文。
    pub fn summary(&self) -> String {
        format!(
            "llm phase={} task={} chars={} model={} endpoint={} elapsed_ms={} outcome={} degraded={} attempts={}",
            Self::phase_str(self.phase),
            self.task.as_str(),
            self.chars,
            self.model,
            self.endpoint,
            self.elapsed_ms,
            self.outcome,
            self.degraded,
            self.attempts
        )
    }
}

/// 日志字段白名单（LLM 子集）。
pub const ALLOWED_LLM_LOG_FIELDS: [&str; 4] =
    ["failure_class", "error_code", "latency_ms", "provider_id"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmLogRecord {
    field: String,
    value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("日志字段 `{0}` 不在 LLM 白名单内")]
pub struct LlmLogFieldError(pub String);

impl LlmLogRecord {
    pub fn new(
        field: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, LlmLogFieldError> {
        let field = field.into();
        if !ALLOWED_LLM_LOG_FIELDS.contains(&field.as_str()) {
            return Err(LlmLogFieldError(field));
        }
        Ok(Self {
            field,
            value: value.into(),
        })
    }

    pub fn field(&self) -> &str {
        &self.field
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for LlmLogRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={}", self.field, self.value)
    }
}

/// 端点冷却状态。`Instant` 单调时钟，不落盘。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open { opened_at: Instant },
}

/// 供 Schema 生成使用的结构形状（不参与反序列化）。
#[derive(Debug, Clone, Copy, JsonSchema)]
#[allow(dead_code)]
struct TextRangeShape {
    start_ms: u64,
    end_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(task: LlmTask) -> RedactedPayload {
        RedactedPayload::new("脱敏后的上下文", task, 8_000).expect("载荷")
    }

    fn request(task: LlmTask) -> LlmGenerationRequest {
        let purpose = (task == LlmTask::Summary).then_some(SummaryPurpose::RollingWindow);
        LlmGenerationRequest::new(
            TaskId::new("req-1").expect("id"),
            payload(task),
            purpose,
            OutputBudget::new(512).expect("output"),
            TemperatureMilli::new(200).expect("temperature"),
            DeadlineMs::new(6_000).expect("deadline"),
        )
        .expect("request")
    }

    #[test]
    fn four_tasks_are_closed_and_distinct() {
        assert_eq!(LlmTask::ALL.len(), 4);
        let names: std::collections::HashSet<_> =
            LlmTask::ALL.iter().map(|task| task.as_str()).collect();
        assert_eq!(names.len(), 4);
        for task in LlmTask::ALL {
            assert!(!task.schema_version().is_empty());
        }
    }

    #[test]
    fn default_budget_matches_design() {
        let expected = [
            (LlmTask::InsightExtract, 512, 200),
            (LlmTask::Summary, 400, 150),
            (LlmTask::CardConclusion, 256, 100),
            (LlmTask::MinutesDraft, 1024, 200),
        ];
        for (task, output, temperature) in expected {
            let budget = task.default_budget();
            assert_eq!(budget.output, output);
            assert_eq!(budget.temperature_milli, temperature);
            assert!(budget.validate().is_ok());
        }
    }

    #[test]
    fn payload_bounds_are_character_based() {
        assert_eq!(
            RedactedPayload::new("", LlmTask::Summary, 8_000),
            Err(LlmModelError::EmptyPayload)
        );
        let over = "字".repeat(8_001);
        assert_eq!(
            RedactedPayload::new(over, LlmTask::Summary, 8_000),
            Err(LlmModelError::PayloadTooLong {
                actual: 8_001,
                limit: 8_000
            })
        );
        let at_limit = "字".repeat(8_000);
        let payload = RedactedPayload::new(at_limit, LlmTask::Summary, 8_000).expect("边界");
        assert_eq!(payload.char_count(), 8_000);
    }

    #[test]
    fn numeric_bounds() {
        assert!(OutputBudget::new(0).is_err());
        assert!(OutputBudget::new(4_097).is_err());
        assert!(OutputBudget::new(1).is_ok());
        assert!(OutputBudget::new(4_096).is_ok());
        assert!(TemperatureMilli::new(1_001).is_err());
        assert!(TemperatureMilli::new(0).is_ok());
        assert!(TemperatureMilli::new(1_000).is_ok());
        assert!(DeadlineMs::new(0).is_err());
        assert!(DeadlineMs::new(8_001).is_err());
        assert!(DeadlineMs::new(1).is_ok());
        assert!(DeadlineMs::new(8_000).is_ok());
    }

    #[test]
    fn purpose_must_match_summary_task() {
        let not_summary = LlmGenerationRequest::new(
            TaskId::new("req-2").expect("id"),
            payload(LlmTask::InsightExtract),
            Some(SummaryPurpose::RollingWindow),
            OutputBudget::new(512).expect("output"),
            TemperatureMilli::new(200).expect("temperature"),
            DeadlineMs::new(6_000).expect("deadline"),
        );
        assert_eq!(not_summary, Err(LlmModelError::PurposeMismatch));

        let summary_without_purpose = LlmGenerationRequest::new(
            TaskId::new("req-3").expect("id"),
            payload(LlmTask::Summary),
            None,
            OutputBudget::new(400).expect("output"),
            TemperatureMilli::new(150).expect("temperature"),
            DeadlineMs::new(6_000).expect("deadline"),
        );
        assert_eq!(summary_without_purpose, Err(LlmModelError::PurposeMismatch));
    }

    #[test]
    fn failure_classes_are_six_and_closed() {
        assert_eq!(FailureClass::ALL.len(), 6);
        assert!(!FailureClass::RateLimited.as_str().is_empty());
    }

    #[test]
    fn outcome_invariants() {
        let normal = StructuredOutcome {
            request_id: TaskId::new("req-4").expect("id"),
            task: LlmTask::Summary,
            body: None,
            model: "test".into(),
            tokens_used: None,
            degraded: false,
            degrade_cause: None,
            failure: None,
            attempts: 1,
            truncated: false,
            schema_version: LlmTask::Summary.schema_version(),
            http_status: None,
            normalization: Vec::new(),
        };
        assert!(normal.invariants_hold());

        let degraded = StructuredOutcome {
            degraded: true,
            degrade_cause: Some(DegradeCause::Failed(FailureClass::Timeout)),
            failure: Some(FailureClass::Timeout),
            ..normal.clone()
        };
        assert!(degraded.invariants_hold());

        let inconsistent = StructuredOutcome {
            degrade_cause: Some(DegradeCause::Failed(FailureClass::Timeout)),
            failure: Some(FailureClass::Offline),
            ..degraded
        };
        assert!(!inconsistent.invariants_hold());
    }

    #[test]
    fn audit_summary_has_no_payload_placeholder() {
        let record = LlmAuditRecord {
            request_id: TaskId::new("req-5").expect("id"),
            phase: AuditPhase::Result,
            task: LlmTask::InsightExtract,
            chars: 123,
            model: "test-model".into(),
            endpoint: "example.invalid".into(),
            elapsed_ms: 456,
            outcome: "ok".into(),
            degraded: false,
            attempts: 1,
        };
        let summary = record.summary();
        assert!(summary.contains("phase=result"));
        assert!(summary.contains("chars=123"));
        assert!(!summary.contains('{'));
    }

    #[test]
    fn log_record_rejects_non_allowlisted_fields() {
        assert!(LlmLogRecord::new("utterance_text", "敏感原文").is_err());
        assert!(LlmLogRecord::new("failure_class", "timeout").is_ok());
    }

    #[test]
    fn structured_body_task_mapping() {
        let empty_summary = Summary {
            purpose: SummaryPurpose::RollingWindow,
            text: UtteranceText::new("摘要").expect("text"),
            covered: None,
            version: 1,
            carried_over: false,
            produced: true,
        };
        let body = StructuredBody::Summary(empty_summary);
        assert_eq!(body.task(), LlmTask::Summary);
        assert_eq!(body.schema_version(), LlmTask::Summary.schema_version());
    }

    #[test]
    fn request_ids_and_budget_survive() {
        let req = request(LlmTask::MinutesDraft);
        assert_eq!(req.task, LlmTask::MinutesDraft);
        assert_eq!(req.output.value(), 512);
        assert_eq!(req.deadline.as_millis(), 6_000);
        assert!(req.purpose.is_none());
    }
}
