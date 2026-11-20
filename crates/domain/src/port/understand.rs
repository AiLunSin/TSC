use crate::error::CopilotError;
use crate::model::card::{DedupeKey, GapCandidate};
use crate::model::enums::{InsightKind, TrustTier};
use crate::model::governance::EffectivePolicy;
use crate::model::insight::{InsightFields, RollingSummary};
use crate::model::utterance::Utterance;
use crate::vo::{CardRef, ConclusionText, Confidence, InsightRef, SessionId, ShortText, TitleText, UtteranceRef};

/// 转写事件：部分结果、最终结果、修订。
#[derive(Debug, Clone, PartialEq)]
pub enum TranscriptEvent {
    Partial(Utterance),
    Final(Utterance),
    Correction(crate::model::utterance::Correction),
}

/// 理解层输入快照。所有字段均为 owned，避免跨线程生命周期耦合。
#[derive(Debug, Clone)]
pub struct ExtractionInput {
    pub session: SessionId,
    pub summary: RollingSummary,
    pub utterances: Vec<Utterance>,
    pub policy: EffectivePolicy,
}

/// 要点草稿（尚未分配 ID，由应用层落库时补全）。
#[derive(Debug, Clone, PartialEq)]
pub struct InsightDraft {
    pub kind: InsightKind,
    pub text: ShortText,
    pub confidence: Confidence,
    pub sources: Vec<UtteranceRef>,
    pub fields: InsightFields,
}

/// 提示卡草稿。
#[derive(Debug, Clone, PartialEq)]
pub struct CardDraft {
    pub title: TitleText,
    pub conclusion: Option<ConclusionText>,
    pub confidence: Confidence,
    pub tier: TrustTier,
    pub dedupe_key: DedupeKey,
    pub trigger: crate::model::enums::TriggerKind,
    pub reference_ids: Vec<crate::vo::ReferenceRef>,
}

/// LLM 请求。调用方必须在构造前完成脱敏（SPEC 4.3）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmRequest {
    pub system: String,
    pub user: String,
    pub json_schema: String,
    pub max_tokens: u32,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmResponse {
    pub json: String,
    pub model: String,
    pub tokens_used: u32,
}

pub trait Transcriber: Send + Sync {
    fn transcribe(&self, chunk: &crate::port::capture::AudioChunk)
        -> Result<Vec<TranscriptEvent>, CopilotError>;
}

pub trait RollingSummarizer: Send + Sync {
    fn summarize(&self, input: &ExtractionInput) -> Result<RollingSummary, CopilotError>;
}

pub trait InsightExtractor: Send + Sync {
    fn extract(&self, input: &ExtractionInput) -> Result<Vec<InsightDraft>, CopilotError>;
}

pub trait GapDetector: Send + Sync {
    fn detect(
        &self,
        summary: &RollingSummary,
        insights: &[InsightRef],
    ) -> Result<Vec<GapCandidate>, CopilotError>;
}

pub trait Deduplicator: Send + Sync {
    /// 10 分钟窗口内同键即视为重复（SPEC 6.4-9）。
    fn is_duplicate(&self, key: &str, at_ms: i64) -> bool;
}

pub trait LlmGateway: Send + Sync {
    fn complete_json(&self, request: LlmRequest) -> Result<LlmResponse, CopilotError>;
}

pub trait CardComposer: Send + Sync {
    fn compose(
        &self,
        candidate: &GapCandidate,
        evidence: &[crate::port::search::SearchHit],
        card: CardRef,
    ) -> Result<CardDraft, CopilotError>;
}