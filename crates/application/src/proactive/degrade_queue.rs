//! 全部检索源不可用时的降级队列（SPEC 7.2.3-3、DESIGN 2.10.2）。

use meeting_copilot_domain::model::proactive::DEGRADE_RETRY_AFTER_MS;
use meeting_copilot_domain::vo::TaskId;

#[derive(Debug, Clone, PartialEq, Eq)]
struct DegradeEntry {
    task_id: TaskId,
    entered_at_ms: i64,
    retried: bool,
}

/// 降级任务在 20 秒后统一重试一次；仍失败则保持待处理，等源恢复后补齐。
#[derive(Debug, Default)]
pub struct DegradeQueue {
    entries: Vec<DegradeEntry>,
}

impl DegradeQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enter(&mut self, task_id: TaskId, now_ms: i64) {
        if self.entries.iter().any(|entry| entry.task_id == task_id) {
            return;
        }
        self.entries.push(DegradeEntry {
            task_id,
            entered_at_ms: now_ms,
            retried: false,
        });
    }

    /// 取出已到 20 秒且尚未重试的任务，并标记为"已重试"。
    pub fn due(&mut self, now_ms: i64) -> Vec<TaskId> {
        let mut due = Vec::new();
        for entry in &mut self.entries {
            if !entry.retried
                && now_ms.saturating_sub(entry.entered_at_ms) >= DEGRADE_RETRY_AFTER_MS
            {
                entry.retried = true;
                due.push(entry.task_id.clone());
            }
        }
        due
    }

    pub fn resolve(&mut self, task_id: &TaskId) {
        self.entries.retain(|entry| &entry.task_id != task_id);
    }

    pub fn pending(&self) -> usize {
        self.entries.len()
    }
}
