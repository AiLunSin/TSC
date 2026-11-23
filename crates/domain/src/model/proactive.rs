//! 主动提示的缓存、outbox 与按需补全共享类型（SPEC 7.2、7.3，DESIGN 2.10.2）。
//!
//! 这些类型只承载标识、枚举、时间、计数与引用元信息；禁止放入发言原文、
//! 卡片标题/结论与检索查询原文。`candidate_json` 另做键名白名单校验。

use serde::{Deserialize, Serialize};

use crate::model::card::{DedupeKey, Reference};
use crate::model::enums::TriggerKind;
use crate::vo::{ProviderId, SessionId, TaskId};

/// 检索结果缓存有效期。
pub const CACHE_TTL_MS: i64 = 24 * 60 * 60 * 1_000;
/// 本机时钟回拨超过该阈值时不使用缓存。
pub const CACHE_CLOCK_SKEW_TOLERANCE_MS: i64 = 5 * 60 * 1_000;
/// 缓存条数上限。
pub const CACHE_MAX_ENTRIES: usize = 2_000;
/// 缓存字节上限（20 MiB）。
pub const CACHE_MAX_BYTES: usize = 20 * 1024 * 1024;
/// 单条参考摘录字符上限。
pub const CACHE_EXCERPT_LIMIT: usize = 500;

/// outbox 待处理（pending + failed）上限。
pub const OUTBOX_MAX_ACTIVE: usize = 500;
/// outbox 任务最长保留时长。
pub const OUTBOX_MAX_AGE_MS: i64 = 7 * 24 * 60 * 60 * 1_000;
/// 补全并发上限。
pub const BACKFILL_CONCURRENCY: usize = 3;
/// 同一去重键 30 分钟内不重复补全。
pub const BACKFILL_DEDUPE_WINDOW_MS: i64 = 30 * 60 * 1_000;
/// 全部检索源不可用时的降级重试间隔。
pub const DEGRADE_RETRY_AFTER_MS: i64 = 20 * 1_000;
/// 按需补全进入统计所需的最少真实样本数。
pub const ON_DEMAND_REQUIRED_SAMPLES: usize = 200;
/// 按需补全首卡 p95 预算。
pub const ON_DEMAND_P95_BUDGET_MS: u64 = 8_000;
/// 按需补全超时率上限。
pub const ON_DEMAND_TIMEOUT_RATE_MAX: f64 = 0.02;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProactiveError {
    #[error("缓存条目非法：{0}")]
    InvalidCacheEntry(&'static str),
    #[error("outbox 条目非法：{0}")]
    InvalidOutboxEntry(&'static str),
    #[error("补全样本非法：{0}")]
    InvalidSample(&'static str),
}

/// 可回放的检索结果缓存条目。`references` 只含引用元信息。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CachedQuery {
    /// SHA-256(provider_profile_id ‖ normalized_query)，64 位小写十六进制。
    pub query_hash: String,
    pub provider_profile_id: ProviderId,
    pub session_id: Option<SessionId>,
    pub fetched_at_ms: i64,
    pub expires_at_ms: i64,
    pub references: Vec<Reference>,
    pub byte_size: usize,
    pub last_access_ms: i64,
}

impl CachedQuery {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        query_hash: impl Into<String>,
        provider_profile_id: ProviderId,
        session_id: Option<SessionId>,
        fetched_at_ms: i64,
        references: Vec<Reference>,
        byte_size: usize,
        last_access_ms: i64,
    ) -> Result<Self, ProactiveError> {
        let entry = Self {
            query_hash: query_hash.into(),
            provider_profile_id,
            session_id,
            fetched_at_ms,
            expires_at_ms: fetched_at_ms.saturating_add(CACHE_TTL_MS),
            references,
            byte_size,
            last_access_ms,
        };
        entry.validate()?;
        Ok(entry)
    }

    /// 是否可直接复用：24 小时内，且本机时钟未大幅回拨。
    pub fn is_usable(&self, now_ms: i64) -> bool {
        if now_ms.saturating_add(CACHE_CLOCK_SKEW_TOLERANCE_MS) < self.fetched_at_ms {
            return false;
        }
        now_ms < self.expires_at_ms
    }

    pub fn age_hours(&self, now_ms: i64) -> u64 {
        u64::try_from(now_ms.saturating_sub(self.fetched_at_ms).max(0) / 3_600_000)
            .unwrap_or(u64::MAX)
    }

    pub fn validate(&self) -> Result<(), ProactiveError> {
        if self.query_hash.len() != 64 || !self.query_hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ProactiveError::InvalidCacheEntry("query_hash"));
        }
        if self.fetched_at_ms < 0 || self.last_access_ms < 0 {
            return Err(ProactiveError::InvalidCacheEntry("timestamp"));
        }
        if self.expires_at_ms != self.fetched_at_ms.saturating_add(CACHE_TTL_MS) {
            return Err(ProactiveError::InvalidCacheEntry("expires_at_ms"));
        }
        if self.byte_size == 0 || self.byte_size > CACHE_MAX_BYTES {
            return Err(ProactiveError::InvalidCacheEntry("byte_size"));
        }
        for reference in &self.references {
            if reference
                .excerpt
                .as_ref()
                .is_some_and(|excerpt| excerpt.chars().count() > CACHE_EXCERPT_LIMIT)
            {
                return Err(ProactiveError::InvalidCacheEntry("excerpt"));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OutboxState {
    #[default]
    Pending,
    Failed,
    Dead,
    Done,
}

impl OutboxState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Failed => "failed",
            Self::Dead => "dead",
            Self::Done => "done",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "pending" => Some(Self::Pending),
            "failed" => Some(Self::Failed),
            "dead" => Some(Self::Dead),
            "done" => Some(Self::Done),
            _ => None,
        }
    }

    /// pending / failed 需要继续处理。
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Pending | Self::Failed)
    }
}

/// 先落库、后外发的检索任务。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxEntry {
    pub task_id: TaskId,
    pub session_id: Option<SessionId>,
    pub trigger_kind: TriggerKind,
    pub dedupe_key: DedupeKey,
    pub topic_cluster_id: String,
    /// 已脱敏的候选序列化；键名受白名单约束。
    pub candidate_json: String,
    pub state: OutboxState,
    pub attempts: u8,
    pub next_attempt_ms: Option<i64>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

impl OutboxEntry {
    pub fn new(
        task_id: TaskId,
        session_id: Option<SessionId>,
        trigger_kind: TriggerKind,
        dedupe_key: DedupeKey,
        topic_cluster_id: impl Into<String>,
        candidate_json: impl Into<String>,
        created_at_ms: i64,
    ) -> Result<Self, ProactiveError> {
        let entry = Self {
            task_id,
            session_id,
            trigger_kind,
            dedupe_key,
            topic_cluster_id: topic_cluster_id.into(),
            candidate_json: candidate_json.into(),
            state: OutboxState::Pending,
            attempts: 0,
            next_attempt_ms: None,
            created_at_ms,
            updated_at_ms: created_at_ms,
        };
        entry.validate()?;
        Ok(entry)
    }

    pub fn is_expired(&self, now_ms: i64) -> bool {
        now_ms.saturating_sub(self.created_at_ms) > OUTBOX_MAX_AGE_MS
    }

    pub fn validate(&self) -> Result<(), ProactiveError> {
        if self.topic_cluster_id.trim().is_empty() {
            return Err(ProactiveError::InvalidOutboxEntry("topic_cluster_id"));
        }
        if self.attempts > 3 {
            return Err(ProactiveError::InvalidOutboxEntry("attempts"));
        }
        if self.created_at_ms < 0 || self.updated_at_ms < 0 {
            return Err(ProactiveError::InvalidOutboxEntry("timestamp"));
        }
        validate_candidate_json(&self.candidate_json)
    }
}

/// 候选 JSON 禁止携带内容字段，只允许 ID / 枚举 / 时间 / 长度 / 计数 / 哈希。
pub fn validate_candidate_json(raw: &str) -> Result<(), ProactiveError> {
    const FORBIDDEN_KEYS: [&str; 13] = [
        "title",
        "conclusion",
        "query",
        "utterance",
        "text",
        "summary",
        "excerpt",
        "snippet",
        "speaker",
        "participant",
        "body",
        "url",
        "page",
    ];
    let value: serde_json::Value = serde_json::from_str(raw)
        .map_err(|_| ProactiveError::InvalidOutboxEntry("candidate_json"))?;
    walk_json_keys(&value, &FORBIDDEN_KEYS)
}

fn walk_json_keys(value: &serde_json::Value, forbidden: &[&str]) -> Result<(), ProactiveError> {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                if forbidden
                    .iter()
                    .any(|denied| key.eq_ignore_ascii_case(denied))
                {
                    return Err(ProactiveError::InvalidOutboxEntry("candidate_json_key"));
                }
                walk_json_keys(child, forbidden)?;
            }
            Ok(())
        }
        serde_json::Value::Array(items) => {
            for item in items {
                walk_json_keys(item, forbidden)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnDemandOutcome {
    Success,
    Timeout,
    Failed,
    Canceled,
}

impl OnDemandOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Timeout => "timeout",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "success" => Some(Self::Success),
            "timeout" => Some(Self::Timeout),
            "failed" => Some(Self::Failed),
            "canceled" => Some(Self::Canceled),
            _ => None,
        }
    }
}

/// 按需补全的时延样本。只记录时间与结果，不记录查询与卡片内容。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnDemandSample {
    pub sample_id: String,
    pub provider_profile_id: ProviderId,
    pub started_at_ms: i64,
    pub latency_ms: u64,
    pub outcome: OnDemandOutcome,
    pub cache_hit: bool,
}

impl OnDemandSample {
    pub fn validate(&self) -> Result<(), ProactiveError> {
        if self.sample_id.trim().is_empty() || self.sample_id.chars().count() > 128 {
            return Err(ProactiveError::InvalidSample("sample_id"));
        }
        if self.started_at_ms < 0 {
            return Err(ProactiveError::InvalidSample("started_at_ms"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider() -> ProviderId {
        ProviderId::new("provider-1").expect("provider")
    }

    #[test]
    fn cache_entry_requires_exact_ttl() {
        let entry = CachedQuery::new(
            "a".repeat(64),
            provider(),
            None,
            1_000,
            Vec::new(),
            128,
            1_000,
        )
        .expect("entry");
        assert_eq!(entry.expires_at_ms, 1_000 + CACHE_TTL_MS);
        assert!(entry.is_usable(1_000 + CACHE_TTL_MS - 1));
        assert!(!entry.is_usable(1_000 + CACHE_TTL_MS));

        let mut broken = entry;
        broken.expires_at_ms += 1;
        assert!(broken.validate().is_err());
    }

    #[test]
    fn cache_bypasses_on_clock_rollback() {
        let entry = CachedQuery::new(
            "b".repeat(64),
            provider(),
            None,
            10_000_000,
            Vec::new(),
            64,
            10_000_000,
        )
        .expect("entry");
        assert!(!entry.is_usable(10_000_000 - CACHE_CLOCK_SKEW_TOLERANCE_MS - 1));
        assert!(entry.is_usable(10_000_000 - CACHE_CLOCK_SKEW_TOLERANCE_MS));
        assert_eq!(entry.age_hours(10_000_000 + 7_200_000), 2);
    }

    #[test]
    fn candidate_json_rejects_content_keys_recursively() {
        assert!(validate_candidate_json(r#"{"kind":"metric","confidence":700}"#).is_ok());
        for raw in [
            r#"{"title":"x"}"#,
            r#"{"nested":[{"conclusion":"x"}]}"#,
            r#"{"Query":"x"}"#,
        ] {
            assert!(validate_candidate_json(raw).is_err(), "{raw}");
        }
    }

    #[test]
    fn outbox_state_roundtrips() {
        for state in [
            OutboxState::Pending,
            OutboxState::Failed,
            OutboxState::Dead,
            OutboxState::Done,
        ] {
            assert_eq!(OutboxState::parse(state.as_str()), Some(state));
        }
        assert!(OutboxState::Failed.is_active());
        assert!(!OutboxState::Dead.is_active());
    }

    #[test]
    fn outbox_entry_expires_at_seven_days() {
        let entry = OutboxEntry::new(
            TaskId::new("task-1").expect("task"),
            None,
            TriggerKind::Auto,
            DedupeKey::new("c".repeat(32)).expect("key"),
            "topic-1",
            r#"{"kind":"metric"}"#,
            0,
        )
        .expect("entry");
        assert!(!entry.is_expired(OUTBOX_MAX_AGE_MS));
        assert!(entry.is_expired(OUTBOX_MAX_AGE_MS + 1));
    }

    #[test]
    fn on_demand_outcome_roundtrips() {
        for outcome in [
            OnDemandOutcome::Success,
            OnDemandOutcome::Timeout,
            OnDemandOutcome::Failed,
            OnDemandOutcome::Canceled,
        ] {
            assert_eq!(OnDemandOutcome::parse(outcome.as_str()), Some(outcome));
        }
    }
}
