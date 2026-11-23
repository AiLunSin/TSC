//! 断网 outbox、补齐计划与降级队列用例（SPEC 7.2、DESIGN 2.10.2）。

use std::sync::Mutex;

use meeting_copilot_application::proactive::backfill::plan_backfill;
use meeting_copilot_application::proactive::degrade_queue::DegradeQueue;
use meeting_copilot_application::proactive::outbox::{EnqueueOutcome, OutboxService};
use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::card::DedupeKey;
use meeting_copilot_domain::model::enums::TriggerKind;
use meeting_copilot_domain::model::proactive::{
    OutboxEntry, OutboxState, BACKFILL_CONCURRENCY, BACKFILL_DEDUPE_WINDOW_MS,
    DEGRADE_RETRY_AFTER_MS, OUTBOX_MAX_ACTIVE, OUTBOX_MAX_AGE_MS,
};
use meeting_copilot_domain::port::proactive::OutboxStore;
use meeting_copilot_domain::vo::{SessionId, TaskId};

#[derive(Default)]
struct FakeOutboxStore {
    entries: Mutex<Vec<OutboxEntry>>,
}

impl OutboxStore for FakeOutboxStore {
    fn insert(&self, entry: &OutboxEntry) -> Result<(), CopilotError> {
        self.entries.lock().expect("entries").push(entry.clone());
        Ok(())
    }

    fn find_active_by_dedupe(&self, dedupe_key: &str) -> Result<Option<OutboxEntry>, CopilotError> {
        Ok(self
            .entries
            .lock()
            .expect("entries")
            .iter()
            .filter(|entry| entry.state.is_active())
            .find(|entry| entry.dedupe_key.as_str() == dedupe_key)
            .cloned())
    }

    fn recoverable(&self) -> Result<Vec<OutboxEntry>, CopilotError> {
        let mut entries: Vec<OutboxEntry> = self
            .entries
            .lock()
            .expect("entries")
            .iter()
            .filter(|entry| entry.state.is_active())
            .cloned()
            .collect();
        entries.sort_by_key(|entry| (entry.next_attempt_ms.unwrap_or(0), entry.created_at_ms));
        Ok(entries)
    }

    fn active_count(&self) -> Result<usize, CopilotError> {
        Ok(self
            .entries
            .lock()
            .expect("entries")
            .iter()
            .filter(|entry| entry.state.is_active())
            .count())
    }

    fn update(&self, entry: &OutboxEntry) -> Result<(), CopilotError> {
        let mut entries = self.entries.lock().expect("entries");
        if let Some(slot) = entries
            .iter_mut()
            .find(|existing| existing.task_id == entry.task_id)
        {
            *slot = entry.clone();
        }
        Ok(())
    }

    fn delete_session(&self, session_id: &SessionId) -> Result<usize, CopilotError> {
        let mut entries = self.entries.lock().expect("entries");
        let before = entries.len();
        entries.retain(|entry| entry.session_id.as_ref() != Some(session_id));
        Ok(before - entries.len())
    }
}

fn dedupe(seed: usize) -> DedupeKey {
    DedupeKey::new(format!("{seed:032x}")).expect("dedupe")
}

fn task(seed: usize, created_at_ms: i64) -> OutboxEntry {
    OutboxEntry::new(
        TaskId::new(format!("task-{seed}")).expect("task"),
        None,
        TriggerKind::Auto,
        dedupe(seed),
        "topic-1",
        r#"{"kind":"metric"}"#,
        created_at_ms,
    )
    .expect("entry")
}

#[test]
fn enqueue_persists_before_any_send() {
    let store = FakeOutboxStore::default();
    let service = OutboxService::new(&store);

    let outcome = service.enqueue(task(1, 0)).expect("enqueue");
    assert_eq!(
        outcome,
        EnqueueOutcome::Enqueued(TaskId::new("task-1").expect("task"))
    );
    assert_eq!(store.entries.lock().expect("entries").len(), 1);
}

#[test]
fn same_dedupe_key_merges_into_existing_task() {
    let store = FakeOutboxStore::default();
    let service = OutboxService::new(&store);

    service.enqueue(task(1, 0)).expect("first");
    let mut duplicate = task(2, 10);
    duplicate.dedupe_key = dedupe(1);
    let outcome = service.enqueue(duplicate).expect("second");
    assert_eq!(
        outcome,
        EnqueueOutcome::Merged(TaskId::new("task-1").expect("task"))
    );
    assert_eq!(store.entries.lock().expect("entries").len(), 1);
}

#[test]
fn rejects_when_active_queue_is_full() {
    let store = FakeOutboxStore::default();
    let service = OutboxService::new(&store);
    for seed in 0..OUTBOX_MAX_ACTIVE {
        service.enqueue(task(seed, 0)).expect("fill");
    }

    let error = service
        .enqueue(task(OUTBOX_MAX_ACTIVE, 0))
        .expect_err("full");
    assert!(matches!(error, CopilotError::StoreFailure { .. }));
    assert_eq!(store.active_count().expect("count"), OUTBOX_MAX_ACTIVE);
}

#[test]
fn three_failures_move_task_to_dead() {
    let store = FakeOutboxStore::default();
    let service = OutboxService::new(&store);
    service.enqueue(task(1, 0)).expect("enqueue");
    let mut entry = task(1, 0);

    service.on_failure(&mut entry, 1_000).expect("first");
    assert_eq!(entry.state, OutboxState::Failed);
    assert_eq!(entry.attempts, 1);
    assert_eq!(entry.next_attempt_ms, Some(1_500));

    service.on_failure(&mut entry, 2_000).expect("second");
    assert_eq!(entry.next_attempt_ms, Some(3_000));

    service.on_failure(&mut entry, 4_000).expect("third");
    assert_eq!(entry.state, OutboxState::Dead);
    assert_eq!(entry.next_attempt_ms, None);
    assert_eq!(store.active_count().expect("count"), 0);
}

#[test]
fn recover_keeps_active_and_expires_stale() {
    let store = FakeOutboxStore::default();
    let service = OutboxService::new(&store);
    let now = OUTBOX_MAX_AGE_MS + 2;
    service.enqueue(task(1, now)).expect("fresh");
    service.enqueue(task(2, 0)).expect("stale");

    let report = service.recover(now).expect("recover");
    assert_eq!(report.recovered.len(), 1);
    assert_eq!(report.recovered[0].task_id.as_str(), "task-1");
    assert_eq!(report.expired.len(), 1);
    assert_eq!(report.expired[0].as_str(), "task-2");
    assert_eq!(store.active_count().expect("count"), 1);
}

#[test]
fn due_returns_at_most_three_tasks() {
    let store = FakeOutboxStore::default();
    let service = OutboxService::new(&store);
    for seed in 0..5 {
        service.enqueue(task(seed, 0)).expect("enqueue");
    }

    let due = service.due(1_000).expect("due");
    assert_eq!(due.len(), BACKFILL_CONCURRENCY);
}

#[test]
fn backfill_skips_recent_not_due_and_expired() {
    let now = OUTBOX_MAX_AGE_MS + 10;
    let due = task(1, now);
    let recently = task(2, now);
    let mut not_due = task(3, now);
    not_due.next_attempt_ms = Some(now + 1);
    let expired = task(4, 0);

    let plan = plan_backfill(
        &[due, recently, not_due, expired],
        &[(
            dedupe(2).as_str().to_owned(),
            now - BACKFILL_DEDUPE_WINDOW_MS + 1,
        )],
        now,
    );

    assert_eq!(plan.batch.len(), 1);
    assert_eq!(plan.batch[0].task_id.as_str(), "task-1");
    assert_eq!(plan.skipped_recent, 1);
    assert_eq!(plan.skipped_not_due, 1);
    assert_eq!(plan.skipped_expired, 1);
}

#[test]
fn degrade_queue_retries_once_after_twenty_seconds() {
    let mut queue = DegradeQueue::new();
    let task_id = TaskId::new("task-1").expect("task");
    queue.enter(task_id.clone(), 0);
    queue.enter(task_id.clone(), 0);

    assert!(queue.due(DEGRADE_RETRY_AFTER_MS - 1).is_empty());
    assert_eq!(queue.due(DEGRADE_RETRY_AFTER_MS), vec![task_id.clone()]);
    assert!(queue.due(DEGRADE_RETRY_AFTER_MS + 1).is_empty());
    assert_eq!(queue.pending(), 1);

    queue.resolve(&task_id);
    assert_eq!(queue.pending(), 0);
}
