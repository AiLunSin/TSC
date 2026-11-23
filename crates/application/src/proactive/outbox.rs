//! 断网 outbox 用例（SPEC 7.2.2~7.2.3、DESIGN 2.10.2）。
//!
//! 顺序固定为"先落库、后外发"；容量、去重、退避与超期规则在这里集中执行。

use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::proactive::{
    OutboxEntry, OutboxState, BACKFILL_CONCURRENCY, OUTBOX_MAX_ACTIVE, OUTBOX_MAX_AGE_MS,
};
use meeting_copilot_domain::model::RetryPolicy;
use meeting_copilot_domain::port::proactive::OutboxStore;
use meeting_copilot_domain::vo::TaskId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnqueueOutcome {
    Enqueued(TaskId),
    /// 同去重键已有活跃任务，合并到既有任务。
    Merged(TaskId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoverReport {
    pub recovered: Vec<OutboxEntry>,
    pub expired: Vec<TaskId>,
}

pub struct OutboxService<'a, Q: OutboxStore> {
    store: &'a Q,
    retry: RetryPolicy,
}

impl<'a, Q: OutboxStore> OutboxService<'a, Q> {
    pub fn new(store: &'a Q) -> Self {
        Self {
            store,
            retry: RetryPolicy::DEFAULT,
        }
    }

    pub fn with_retry(store: &'a Q, retry: RetryPolicy) -> Self {
        Self { store, retry }
    }

    /// 先写本地队列，再允许调用方外发。超过去重或容量时明确返回结果/错误。
    pub fn enqueue(&self, entry: OutboxEntry) -> Result<EnqueueOutcome, CopilotError> {
        entry
            .validate()
            .map_err(|error| CopilotError::StoreFailure {
                detail: error.to_string(),
            })?;
        if let Some(existing) = self
            .store
            .find_active_by_dedupe(entry.dedupe_key.as_str())?
        {
            return Ok(EnqueueOutcome::Merged(existing.task_id));
        }
        if self.store.active_count()? >= OUTBOX_MAX_ACTIVE {
            return Err(CopilotError::StoreFailure {
                detail: format!("待补任务已达 {OUTBOX_MAX_ACTIVE} 上限"),
            });
        }
        self.store.insert(&entry)?;
        Ok(EnqueueOutcome::Enqueued(entry.task_id))
    }

    /// 启动恢复：返回仍需处理的 pending/failed；超 7 天的任务转 dead。
    pub fn recover(&self, now_ms: i64) -> Result<RecoverReport, CopilotError> {
        let mut recovered = Vec::new();
        let mut expired = Vec::new();
        for mut entry in self.store.recoverable()? {
            if entry.is_expired(now_ms) {
                entry.state = OutboxState::Dead;
                entry.next_attempt_ms = None;
                entry.updated_at_ms = now_ms;
                self.store.update(&entry)?;
                expired.push(entry.task_id);
            } else {
                recovered.push(entry);
            }
        }
        Ok(RecoverReport { recovered, expired })
    }

    /// 到期可执行的前 3 个任务。
    pub fn due(&self, now_ms: i64) -> Result<Vec<OutboxEntry>, CopilotError> {
        Ok(self
            .store
            .recoverable()?
            .into_iter()
            .filter(|entry| entry.next_attempt_ms.is_none_or(|next| next <= now_ms))
            .take(BACKFILL_CONCURRENCY)
            .collect())
    }

    /// 记录一次失败：指数退避；达到 3 次转 dead。
    pub fn on_failure(&self, entry: &mut OutboxEntry, now_ms: i64) -> Result<(), CopilotError> {
        entry.attempts = entry.attempts.saturating_add(1);
        entry.updated_at_ms = now_ms;
        if self.retry.exhausted(entry.attempts) {
            entry.state = OutboxState::Dead;
            entry.next_attempt_ms = None;
        } else {
            let delay = self.retry.delay_for(entry.attempts.saturating_sub(1));
            entry.state = OutboxState::Failed;
            entry.next_attempt_ms =
                Some(now_ms.saturating_add(i64::try_from(delay).unwrap_or(i64::MAX)));
        }
        self.store.update(entry)
    }

    pub fn on_success(&self, entry: &mut OutboxEntry, now_ms: i64) -> Result<(), CopilotError> {
        entry.state = OutboxState::Done;
        entry.next_attempt_ms = None;
        entry.updated_at_ms = now_ms;
        self.store.update(entry)
    }

    pub fn max_age_ms(&self) -> i64 {
        OUTBOX_MAX_AGE_MS
    }
}
