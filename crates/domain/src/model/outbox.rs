use serde::{Deserialize, Serialize};

use crate::model::card::GapCandidate;
use crate::model::enums::TaskState;
use crate::vo::TaskId;

/// 重试策略（指数退避，带上限）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u8,
    pub base_backoff_ms: u64,
    pub max_backoff_ms: u64,
}

impl RetryPolicy {
    pub const DEFAULT: Self = Self {
        max_attempts: 3,
        base_backoff_ms: 500,
        max_backoff_ms: 8_000,
    };

    pub fn delay_for(&self, attempt: u8) -> u64 {
        let exponent = u32::from(attempt.min(16));
        let factor = 1_u64 << exponent;
        self.base_backoff_ms
            .saturating_mul(factor)
            .min(self.max_backoff_ms)
    }

    pub fn exhausted(&self, attempt: u8) -> bool {
        attempt >= self.max_attempts
    }
}

/// 外发检索任务（outbox，保证断网/失败不丢）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutboxTask {
    pub id: TaskId,
    pub candidate: GapCandidate,
    pub retry: RetryPolicy,
    pub state: TaskState,
    pub attempts: u8,
}

impl OutboxTask {
    pub fn record_failure(&mut self) {
        self.attempts = self.attempts.saturating_add(1);
        self.state = if self.retry.exhausted(self.attempts) {
            TaskState::Dead
        } else {
            TaskState::Failed
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_is_capped() {
        let policy = RetryPolicy::DEFAULT;
        assert_eq!(policy.delay_for(0), 500);
        assert_eq!(policy.delay_for(1), 1_000);
        assert_eq!(policy.delay_for(10), 8_000);
    }

    #[test]
    fn max_attempts_marks_dead() {
        let mut task = OutboxTask {
            id: TaskId::new("task-1").expect("id"),
            candidate: super::GapCandidate {
                kind: crate::model::enums::GapKind::Metric,
                trigger_span: crate::model::session::TextRange::new(0, 100).expect("span"),
                confidence: crate::vo::Confidence::new(0.7).expect("conf"),
                dedupe_key: crate::model::card::DedupeKey::new("a".repeat(32)).expect("key"),
                state: crate::model::enums::GapState::Detected,
            },
            retry: RetryPolicy::DEFAULT,
            state: TaskState::Pending,
            attempts: 0,
        };
        task.record_failure();
        task.record_failure();
        assert_eq!(task.state, TaskState::Failed);
        task.record_failure();
        assert_eq!(task.state, TaskState::Dead);
    }
}
