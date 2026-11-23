//! 主动提示的持久化端口（SPEC 7.1~7.4、DESIGN 2.10）。
//!
//! 端口只收发标识、枚举、时间与计数，不接收发言原文、卡片结论或查询原文。

use crate::error::CopilotError;
use crate::model::feedback::{FeedbackEvent, TopicWeight};
use crate::model::proactive::{CachedQuery, OnDemandSample, OutboxEntry};
use crate::vo::SessionId;

/// 反馈事件与派生权重。事件只追加；权重可重建。
pub trait FeedbackStore: Send + Sync {
    fn append_event(&self, event: &FeedbackEvent) -> Result<(), CopilotError>;

    fn weight(
        &self,
        topic_cluster_id: &str,
        source_ref_id: &str,
    ) -> Result<Option<TopicWeight>, CopilotError>;

    fn save_weight(&self, weight: &TopicWeight) -> Result<(), CopilotError>;

    /// 按追加顺序返回全部事件，用于重建派生权重。
    fn events(&self) -> Result<Vec<FeedbackEvent>, CopilotError>;
}

/// 检索结果缓存。`put` 返回因容量上限被淘汰的条数。
pub trait QueryCacheStore: Send + Sync {
    fn get(&self, query_hash: &str) -> Result<Option<CachedQuery>, CopilotError>;

    fn put(&self, entry: &CachedQuery) -> Result<usize, CopilotError>;

    fn touch(&self, query_hash: &str, at_ms: i64) -> Result<(), CopilotError>;

    fn sweep_expired(&self, now_ms: i64) -> Result<usize, CopilotError>;

    fn delete_session(&self, session_id: &SessionId) -> Result<usize, CopilotError>;
}

/// 先落库、后外发的检索任务队列。
pub trait OutboxStore: Send + Sync {
    fn insert(&self, entry: &OutboxEntry) -> Result<(), CopilotError>;

    fn find_active_by_dedupe(&self, dedupe_key: &str) -> Result<Option<OutboxEntry>, CopilotError>;

    /// 启动恢复：返回全部 `pending` 与 `failed`。
    fn recoverable(&self) -> Result<Vec<OutboxEntry>, CopilotError>;

    fn active_count(&self) -> Result<usize, CopilotError>;

    fn update(&self, entry: &OutboxEntry) -> Result<(), CopilotError>;

    fn delete_session(&self, session_id: &SessionId) -> Result<usize, CopilotError>;
}

/// 按需补全时延样本。
pub trait OnDemandSampleStore: Send + Sync {
    fn append(&self, sample: &OnDemandSample) -> Result<(), CopilotError>;

    fn load(&self) -> Result<Vec<OnDemandSample>, CopilotError>;
}
