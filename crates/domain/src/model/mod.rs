//! 领域聚合与枚举（DESIGN 2.3.2）。
//!
//! 聚合根：`Session`。其余对象均通过会话归属，禁止跨会话迁移（SPEC 6.2-2）。

pub mod card;
pub mod channel;
pub mod enums;
pub mod gate;
pub mod gate_keyword;
pub mod gate_replay;
pub mod governance;
pub mod insight;
pub mod insight_kind;
pub mod insight_lock;
pub mod insight_policy;
pub mod insight_similarity;
pub mod llm;
pub mod outbox;
pub mod provider;
pub mod rolling_summary;
pub mod session;
pub mod unknown;
pub mod utterance;
pub mod utterance_text;

pub use card::{
    ConflictView, DedupeKey, Freshness, GapCandidate, Reference, SuggestionCard, TraceChain,
};
pub use channel::{
    legal_targets, transition, wall_view, ChannelKey, ChannelKind, ChannelState, ChannelStateError,
    ChannelStateSnapshot, GapReason, StateReason, SwitchReason, TransitionOutcome,
};
pub use enums::*;
pub use gate::*;
pub use gate::{GateRejectReason, Visibility};
pub use gate_keyword::*;
pub use gate_replay::*;
pub use governance::{AuditRecord, EffectivePolicy, LexiconTerm, OutboundGrant};
pub use insight::{
    DecisionFields, Entity, Insight, InsightFields, QuestionFields, RollingSummary, TodoFields,
    Topic,
};
pub use llm::{
    AuditPhase, CardConclusion, CircuitState, CompletionHint, ConclusionNotice, DeadlineMs,
    DegradeCause, FailureClass, InsightExtract, InsightItem, LlmAuditRecord, LlmGenerationRequest,
    LlmLogFieldError, LlmLogRecord, LlmMetric, LlmMetricPoint, LlmModelError, LlmTask,
    MinutesDraft, MinutesInsights, NormalizationEntry, NormalizationSource, OutputBudget,
    PendingItem, PendingReason, RedactedPayload, StreamEvent, StreamEventKind, StreamOutcome,
    StructuredBody, StructuredOutcome, Summary, SummaryPurpose, TaskBudget, TemperatureMilli,
    Termination, Timeline, TimelineEntry,
};
pub use outbox::{OutboxTask, RetryPolicy};
pub use provider::{DomainPattern, ProviderProfile, ProviderTimeout, RateLimit, Weight};
pub use session::{
    ConsentRecord, ParticipantList, PlatformBinding, RetentionWindow, Session, TextRange,
};
pub use unknown::{DegradationWarning, Degraded, UnknownValue};
pub use utterance::{Correction, SpeakerRef, Utterance};
