//! 断网降级与恢复补齐的协调与取证（SPEC 7.2.1-7/8、DESIGN 2.10.2）。
//!
//! 这里只做「节拍 + 计数 + 阈值判定」：实际的落库、重试与出卡由调用方注入，
//! 这样应用层和测试可以共用同一套口径：
//! - 全部检索源不可用 → 进入降级队列，20 秒后单次重试；
//! - 网络恢复 → 开启 30 秒补齐窗口；
//! - 补齐率 ≥98%、窗口内完成、并发 ≤3 三项同时满足才算达标。

use meeting_copilot_domain::model::proactive::BACKFILL_CONCURRENCY;
use meeting_copilot_domain::vo::TaskId;

use crate::proactive::degrade_queue::DegradeQueue;

/// 恢复后的补齐窗口（SPEC 7.2.1-8a：网络恢复后 30 秒内补齐）。
pub const BACKFILL_WINDOW_MS: i64 = 30 * 1_000;
/// 恢复后的补齐率下限（SPEC 7.2.1-8a：≥98%）。
pub const BACKFILL_RATIO_MIN: f64 = 0.98;

/// 一次恢复窗口的取证结果。
#[derive(Debug, Clone, PartialEq)]
pub struct BackfillOutcome {
    /// 恢复时需要补齐的任务总数。
    pub total: usize,
    /// 窗口内完成的任务数。
    pub completed: usize,
    /// 完成比例；`total == 0` 记为 1.0。
    pub ratio: f64,
    /// 从网络恢复识别到窗口结束的耗时。
    pub elapsed_ms: i64,
    /// 窗口内观测到的最大并发。
    pub max_inflight: usize,
    /// 补齐率 ≥98%、落在 30 秒内、并发 ≤3 三项同时满足。
    pub meets_budget: bool,
}

impl BackfillOutcome {
    /// 供调用方与 SPEC 7.2.1-8a 对齐的判定入口。
    pub const fn meets_spec(&self) -> bool {
        self.meets_budget
    }
}

/// 断网 → 降级 → 恢复 → 补齐的协调器。
#[derive(Debug)]
pub struct RecoveryCoordinator {
    degrade: DegradeQueue,
    window_start_ms: Option<i64>,
    window_total: usize,
    window_completed: usize,
    inflight: usize,
    max_inflight: usize,
    last_outcome: Option<BackfillOutcome>,
}

impl Default for RecoveryCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl RecoveryCoordinator {
    pub fn new() -> Self {
        Self {
            degrade: DegradeQueue::new(),
            window_start_ms: None,
            window_total: 0,
            window_completed: 0,
            inflight: 0,
            max_inflight: 0,
            last_outcome: None,
        }
    }

    /// 仍在降级队列中等待重试的任务数。
    pub fn degrade_pending(&self) -> usize {
        self.degrade.pending()
    }

    /// 全部检索源不可用：任务进入降级队列（20 秒后单次重试）。
    pub fn on_sources_unavailable(&mut self, task_id: TaskId, now_ms: i64) {
        self.degrade.enter(task_id, now_ms);
    }

    /// 取出已到 20 秒的单次重试任务。
    pub fn due_retries(&mut self, now_ms: i64) -> Vec<TaskId> {
        self.degrade.due(now_ms)
    }

    /// 单次重试成功后移出降级队列。
    pub fn on_retry_resolved(&mut self, task_id: &TaskId) {
        self.degrade.resolve(task_id);
    }

    /// 网络恢复：开启补齐窗口。`pending_total` 为恢复时需要补齐的任务数。
    pub fn on_network_recovered(&mut self, now_ms: i64, pending_total: usize) {
        self.window_start_ms = Some(now_ms);
        self.window_total = pending_total;
        self.window_completed = 0;
        self.inflight = 0;
        self.max_inflight = 0;
    }

    /// 是否还有未结束的恢复窗口。
    pub const fn window_open(&self) -> bool {
        self.window_start_ms.is_some()
    }

    /// 开始一条补齐；返回 `false` 表示已达并发上限，本条需要排队。
    pub fn on_backfill_started(&mut self) -> bool {
        if self.inflight >= BACKFILL_CONCURRENCY {
            return false;
        }
        self.inflight += 1;
        self.max_inflight = self.max_inflight.max(self.inflight);
        true
    }

    /// 一条补齐结束（成功或最终失败都算已处理）。
    pub fn on_backfill_finished(&mut self) {
        self.inflight = self.inflight.saturating_sub(1);
        if self.window_start_ms.is_some() {
            self.window_completed = self.window_completed.saturating_add(1);
        }
    }

    /// 窗口内已完成 / 总数，供界面显示进度。
    pub const fn window_progress(&self) -> (usize, usize) {
        (self.window_completed, self.window_total)
    }

    /// 窗口完成或到期时返回取证结果并清理窗口；否则返回 `None`。
    pub fn poll_outcome(&mut self, now_ms: i64) -> Option<BackfillOutcome> {
        let start = self.window_start_ms?;
        let elapsed = now_ms.saturating_sub(start);
        let finished = self.window_completed >= self.window_total;
        if !finished && elapsed < BACKFILL_WINDOW_MS {
            return None;
        }

        let total = self.window_total;
        let completed = self.window_completed.min(total);
        let ratio = if total == 0 {
            1.0
        } else {
            completed as f64 / total as f64
        };
        let outcome = BackfillOutcome {
            total,
            completed,
            ratio,
            elapsed_ms: elapsed,
            max_inflight: self.max_inflight,
            meets_budget: ratio >= BACKFILL_RATIO_MIN
                && elapsed <= BACKFILL_WINDOW_MS
                && self.max_inflight <= BACKFILL_CONCURRENCY,
        };

        self.window_start_ms = None;
        self.window_total = 0;
        self.window_completed = 0;
        self.inflight = 0;
        self.max_inflight = 0;
        self.last_outcome = Some(outcome.clone());
        Some(outcome)
    }

    pub fn last_outcome(&self) -> Option<&BackfillOutcome> {
        self.last_outcome.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str) -> TaskId {
        TaskId::new(id).expect("task id")
    }

    #[test]
    fn degrade_retry_fires_after_twenty_seconds() {
        let mut coordinator = RecoveryCoordinator::new();
        coordinator.on_sources_unavailable(task("task-1"), 1_000);
        assert_eq!(coordinator.degrade_pending(), 1);

        assert!(coordinator.due_retries(20_999).is_empty());
        let due = coordinator.due_retries(21_000);
        assert_eq!(due, vec![task("task-1")]);

        coordinator.on_retry_resolved(&task("task-1"));
        assert_eq!(coordinator.degrade_pending(), 0);
    }

    #[test]
    fn full_backfill_meets_spec() {
        let mut coordinator = RecoveryCoordinator::new();
        coordinator.on_network_recovered(10_000, 2);

        assert!(coordinator.on_backfill_started());
        assert!(coordinator.on_backfill_started());
        coordinator.on_backfill_finished();
        coordinator.on_backfill_finished();

        let outcome = coordinator.poll_outcome(11_000).expect("outcome");
        assert_eq!(outcome.total, 2);
        assert_eq!(outcome.completed, 2);
        assert!((outcome.ratio - 1.0).abs() < f64::EPSILON);
        assert!(outcome.meets_spec());
        assert!(!coordinator.window_open());
        assert!(coordinator.last_outcome().is_some());
    }

    #[test]
    fn concurrency_cap_blocks_fourth_task() {
        let mut coordinator = RecoveryCoordinator::new();
        coordinator.on_network_recovered(0, 10);
        assert!(coordinator.on_backfill_started());
        assert!(coordinator.on_backfill_started());
        assert!(coordinator.on_backfill_started());
        assert!(!coordinator.on_backfill_started());

        coordinator.on_backfill_finished();
        assert!(coordinator.on_backfill_started());
    }

    #[test]
    fn partial_backfill_fails_budget_after_window() {
        let mut coordinator = RecoveryCoordinator::new();
        coordinator.on_network_recovered(0, 100);
        for _ in 0..97 {
            assert!(coordinator.on_backfill_started());
            coordinator.on_backfill_finished();
        }

        assert!(coordinator.poll_outcome(29_000).is_none());
        let outcome = coordinator.poll_outcome(30_000).expect("outcome");
        assert_eq!(outcome.completed, 97);
        assert!((outcome.ratio - 0.97).abs() < 1e-9);
        assert!(!outcome.meets_spec(), "97% < 98% 不得判达标");
    }

    #[test]
    fn empty_window_is_treated_as_complete() {
        let mut coordinator = RecoveryCoordinator::new();
        coordinator.on_network_recovered(0, 0);
        let outcome = coordinator.poll_outcome(1).expect("outcome");
        assert_eq!(outcome.ratio, 1.0);
        assert!(outcome.meets_spec());
    }
}
