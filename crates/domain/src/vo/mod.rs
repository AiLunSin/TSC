//! 约束型值对象：把约束固化在类型里，越界在构造期即失败。
//!
//! 与 DESIGN 2.2.2-C 对应：`ShortText` / `TitleText` / `ConclusionText` /
//! `UtteranceText` / `Confidence` / `SanitizedQuery`。

mod confidence;
mod ids;
mod sanitized;
mod text;
mod time;

pub use confidence::Confidence;
pub use ids::{
    ActorRef, AuditId, CardRef, InsightRef, KeywordRef, ProviderId, ReferenceRef, ResourceRef,
    SessionId, TaskId, TraceId, UtteranceRef,
};
pub use sanitized::{SanitizedQuery, URL_LIMIT};
pub use text::{
    ConclusionText, NormalizedTerm, ShortText, TextBoundError, TitleText, UtteranceText,
};
pub use time::{AudioClock, ClockError, MonoMillis, WallClock};
