//! 领域端口（DESIGN 2.2.2）。所有端口均为同步 trait，实现层负责调度。
//!
//! 端口是"开放可配置"与"平台插件化"的结构保证：领域层只描述能力，
//! 不出现任何具体服务商、平台 API 或存储细节。

pub mod capture;
pub mod capture_clock;
pub mod governance;
pub mod llm;
pub mod proactive;
pub mod repository;
pub mod search;
pub mod understand;

pub use capture::{
    AudioChunk, AudioSource, SourceHealth, Subscription, SubtitleCapability, SubtitleEvent,
    SubtitleSource,
};
pub use capture_clock::CaptureClock;
pub use governance::{AuditLog, Decision, Guardrail, PolicyResolver, Presenter, Sanitizer};
pub use llm::{
    CancelToken, LlmCapability, LlmObserver, StreamCollector, StreamOutcomeSummary,
    StructuredLlmGateway,
};
pub use proactive::{FeedbackStore, OnDemandSampleStore, OutboxStore, QueryCacheStore};
pub use repository::{CardRepository, InsightRepository, SessionRepository, UtteranceRepository};
pub use search::{
    FetchOutcome, PageFetcher, ProviderRequest, ProviderResponse, SearchBroker, SearchHit,
    SearchProvider,
};
pub use understand::{
    CardComposer, CardDraft, Deduplicator, ExtractionInput, GapDetector, InsightDraft,
    InsightExtractor, LlmGateway, LlmRequest, LlmResponse, RollingSummarizer, Transcriber,
    TranscriptEvent,
};
