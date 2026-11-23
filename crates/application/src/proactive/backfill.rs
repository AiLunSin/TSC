//! 断网恢复后的补齐计划（SPEC 7.2.3、DESIGN 2.10.2）。

use meeting_copilot_domain::model::proactive::{
    OutboxEntry, BACKFILL_CONCURRENCY, BACKFILL_DEDUPE_WINDOW_MS, OUTBOX_MAX_AGE_MS,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackfillPlan {
    pub batch: Vec<OutboxEntry>,
    pub skipped_recent: usize,
    pub skipped_not_due: usize,
    pub skipped_expired: usize,
}

/// 选取至多 3 个到期任务；30 分钟内已展示过的去重键不再补全。
pub fn plan_backfill(
    tasks: &[OutboxEntry],
    recently_shown: &[(String, i64)],
    now_ms: i64,
) -> BackfillPlan {
    let mut batch = Vec::new();
    let mut skipped_recent = 0;
    let mut skipped_not_due = 0;
    let mut skipped_expired = 0;

    for task in tasks {
        if !task.state.is_active() {
            continue;
        }
        if now_ms.saturating_sub(task.created_at_ms) > OUTBOX_MAX_AGE_MS {
            skipped_expired += 1;
            continue;
        }
        if task.next_attempt_ms.is_some_and(|next| next > now_ms) {
            skipped_not_due += 1;
            continue;
        }
        let recently = recently_shown.iter().any(|(key, shown_at)| {
            key == task.dedupe_key.as_str()
                && now_ms.saturating_sub(*shown_at) < BACKFILL_DEDUPE_WINDOW_MS
        });
        if recently {
            skipped_recent += 1;
            continue;
        }
        if batch.len() >= BACKFILL_CONCURRENCY {
            skipped_not_due += 1;
            continue;
        }
        batch.push(task.clone());
    }

    BackfillPlan {
        batch,
        skipped_recent,
        skipped_not_due,
        skipped_expired,
    }
}
