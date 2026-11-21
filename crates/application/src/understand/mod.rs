//! 理解层（B3）：片段准入、滚动摘要、要点抽取与去重。

pub mod counters;
pub mod deduplicator;
pub mod field_filler;
pub mod insight_extractor;
pub mod insight_gateway;
pub mod insight_ops;
pub mod insight_splitter;
pub mod normalize;
pub mod ports;
pub mod rolling_summary;
pub mod rule_fallback;
pub mod service;
pub mod stable_prefix;
pub mod store;
pub mod summary_guard;
pub mod summary_parser;
pub mod types;

pub use counters::{Counter, CounterSnapshot, UnderstandingCounters};
pub use deduplicator::{
    is_visible, merge_drafted, project_sources, similarity, DedupeOutcome, InsightSimilarity,
};
pub use field_filler::{apply_kind_penalty, fill_fields};
pub use insight_extractor::{classify_signals, prefilter_hit};
pub use insight_gateway::{
    build_request, decide_extraction, drafts_from_extract, run_extraction, ExtractDecision,
};
pub use insight_ops::{delete, filter, flag_important};
pub use insight_splitter::split_insight_text;
pub use ports::{LexiconView, SilenceSignal, UnderstandClock, UnderstandFeature};
pub use rolling_summary::{covered_span, SessionSummaryState, SummaryWindow};
pub use service::UnderstandService;
pub use stable_prefix::StablePrefix;
pub use store::InsightStore;
pub use summary_guard::SummaryGuard;
pub use types::{
    NormalizedFragment, RejectReason, UnderstandInitError, UnderstandInput, UnderstandOutcome,
};
