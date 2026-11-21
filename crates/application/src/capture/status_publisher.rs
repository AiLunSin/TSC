//! 状态发布器：去重、先快照后扇出、会话序号、时延统计（AC-B2-OC-D01~D10）。
//!
//! 发布顺序固定为「快照写入 → 会话序号分配 → 入队」；`drain` 不持发布器内部锁。

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use meeting_copilot_domain::port::CaptureClock;
use meeting_copilot_domain::vo::{MonoMillis, SessionId};

use super::capture_ports::{ChannelStatusSink, DiagnosticsSink, NoopDiagnostics};
use super::diagnostics::DiagnosticCounter;
use super::subscriptions::{
    DeliveryQueue, DrainReport, Slot, SlotList, StatusHandler, SubscriptionHandle,
};
use super::types::{
    ActiveChannel, CaptureAvailability, CaptureAvailabilityEvent, CapturePolicy,
    CaptureStatusDelivery, ChannelSnapshot, ChannelSnapshotEntry, ChannelStatusEvent,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishOutcome {
    Published { sequence: u64 },
    NotPushed { reason: NotPushedReason },
    Rejected { reason: PublishRejectReason },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotPushedReason {
    SameState,
    NoSubscriberButSnapshotUpdated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishRejectReason {
    SeqExhausted,
    SnapshotWriteFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LatencyStats {
    pub samples: usize,
    pub p50_ms: u64,
    pub p99_ms: u64,
    pub max_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AvailabilityEdge {
    Available(ActiveChannel),
    Unavailable,
}

impl AvailabilityEdge {
    fn of(availability: CaptureAvailability) -> Self {
        match availability {
            CaptureAvailability::Available { channel } => Self::Available(channel),
            CaptureAvailability::Unavailable { .. } => Self::Unavailable,
        }
    }
}

pub struct StatusPublisher {
    clock: Arc<dyn CaptureClock>,
    sink: Arc<dyn ChannelStatusSink>,
    diagnostics: Arc<dyn DiagnosticsSink>,
    policy: CapturePolicy,
    snapshots: Mutex<HashMap<SessionId, Vec<ChannelSnapshotEntry>>>,
    sequences: Mutex<HashMap<SessionId, u64>>,
    availability_edges: Mutex<HashMap<SessionId, AvailabilityEdge>>,
    latency: Mutex<VecDeque<u64>>,
    slots: SlotList,
    next_subscriber: AtomicU64,
}

impl StatusPublisher {
    pub fn new(
        clock: Arc<dyn CaptureClock>,
        sink: Arc<dyn ChannelStatusSink>,
        policy: CapturePolicy,
    ) -> Self {
        Self::with_diagnostics(clock, sink, policy, Arc::new(NoopDiagnostics))
    }

    pub fn with_diagnostics(
        clock: Arc<dyn CaptureClock>,
        sink: Arc<dyn ChannelStatusSink>,
        policy: CapturePolicy,
        diagnostics: Arc<dyn DiagnosticsSink>,
    ) -> Self {
        Self {
            clock,
            sink,
            diagnostics,
            policy,
            snapshots: Mutex::new(HashMap::new()),
            sequences: Mutex::new(HashMap::new()),
            availability_edges: Mutex::new(HashMap::new()),
            latency: Mutex::new(VecDeque::with_capacity(
                usize::try_from(policy.status_latency_ring).unwrap_or(256),
            )),
            slots: Arc::new(Mutex::new(Vec::new())),
            next_subscriber: AtomicU64::new(0),
        }
    }

    /// 仅用于恢复/测试注入：设定会话当前序号。
    pub fn seed_sequence(&self, session: &SessionId, sequence: u64) {
        self.sequences
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .insert(session.clone(), sequence);
    }

    pub fn publish_channel(
        &self,
        event: ChannelStatusEvent,
        session: &SessionId,
    ) -> PublishOutcome {
        let outcome = {
            let mut snapshots = self
                .snapshots
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            let entries = snapshots.entry(session.clone()).or_default();
            let existing = entries.iter().position(|entry| entry.key == event.key);
            if let Some(index) = existing {
                if entries[index].state == event.state {
                    entries[index].reason = Some(event.reason);
                    entries[index].changed_at = event.determined_at;
                    self.diagnostics.bump(DiagnosticCounter::StatusNotPushed, 1);
                    return PublishOutcome::NotPushed {
                        reason: NotPushedReason::SameState,
                    };
                }
            }
            let Some(sequence) = self.next_sequence(session) else {
                self.diagnostics.bump(DiagnosticCounter::SeqOverflow, 1);
                return PublishOutcome::Rejected {
                    reason: PublishRejectReason::SeqExhausted,
                };
            };
            match existing {
                Some(index) => {
                    entries[index] = ChannelSnapshotEntry {
                        key: event.key,
                        state: event.state,
                        reason: Some(event.reason),
                        entered_at: event.determined_at,
                        changed_at: event.determined_at,
                    };
                }
                None => entries.push(ChannelSnapshotEntry {
                    key: event.key,
                    state: event.state,
                    reason: Some(event.reason),
                    entered_at: event.determined_at,
                    changed_at: event.determined_at,
                }),
            }
            entries.sort_by_key(|entry| entry.key);
            PublishOutcome::Published { sequence }
        };
        let PublishOutcome::Published { sequence } = outcome else {
            return outcome;
        };
        let published = ChannelStatusEvent { sequence, ..event };
        let _ = self.sink.publish(&published);
        self.enqueue(CaptureStatusDelivery {
            session: session.clone(),
            channel: Some(published),
            availability: None,
            determined_at: event.determined_at,
        });
        outcome
    }

    pub fn publish_availability(
        &self,
        event: CaptureAvailabilityEvent,
        session: &SessionId,
    ) -> PublishOutcome {
        let edge = AvailabilityEdge::of(event.availability);
        {
            let mut edges = self
                .availability_edges
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            if edges.get(session) == Some(&edge) {
                self.diagnostics.bump(DiagnosticCounter::StatusNotPushed, 1);
                return PublishOutcome::NotPushed {
                    reason: NotPushedReason::SameState,
                };
            }
            edges.insert(session.clone(), edge);
        }
        let Some(sequence) = self.next_sequence(session) else {
            self.diagnostics.bump(DiagnosticCounter::SeqOverflow, 1);
            return PublishOutcome::Rejected {
                reason: PublishRejectReason::SeqExhausted,
            };
        };
        let determined_at = self.clock.mono_now().unwrap_or(MonoMillis::ZERO);
        let published = CaptureAvailabilityEvent { sequence, ..event };
        self.diagnostics
            .bump(DiagnosticCounter::AvailabilityPushed, 1);
        self.enqueue(CaptureStatusDelivery {
            session: session.clone(),
            channel: None,
            availability: Some(published),
            determined_at,
        });
        PublishOutcome::Published { sequence }
    }

    fn next_sequence(&self, session: &SessionId) -> Option<u64> {
        let mut sequences = self
            .sequences
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let current = sequences.get(session).copied().unwrap_or(0);
        let ceiling = u64::MAX.saturating_sub(self.policy.seq_upper_bound_margin);
        if current >= ceiling {
            return None;
        }
        let next = current.saturating_add(1);
        sequences.insert(session.clone(), next);
        Some(next)
    }

    fn enqueue(&self, delivery: CaptureStatusDelivery) {
        let slots: Vec<Arc<Mutex<Slot>>> = {
            let slots = self
                .slots
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            slots.clone()
        };
        for slot in slots {
            let mut slot = slot.lock().unwrap_or_else(|poison| poison.into_inner());
            if !slot.active {
                continue;
            }
            if slot.queue.push(delivery.clone()) {
                self.diagnostics.bump(DiagnosticCounter::QueueDropped, 1);
            }
        }
    }

    pub fn subscribe(&self, handler: StatusHandler) -> SubscriptionHandle {
        let id = self.next_subscriber.fetch_add(1, Ordering::SeqCst);
        let slot = Arc::new(Mutex::new(Slot {
            id,
            active: true,
            handler,
            queue: DeliveryQueue::new(self.policy.status_queue_capacity),
            failures: 0,
        }));
        self.slots
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .push(Arc::clone(&slot));
        SubscriptionHandle::new(slot, Arc::clone(&self.slots))
    }

    /// 显式推进投递；不持发布器内部锁，慢/抛错订阅者互不阻塞。
    pub fn drain(&self, now: MonoMillis) -> DrainReport {
        let slots: Vec<Arc<Mutex<Slot>>> = {
            let slots = self
                .slots
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            slots.clone()
        };
        let mut report = DrainReport::default();
        for slot in slots {
            let mut slot = slot.lock().unwrap_or_else(|poison| poison.into_inner());
            if !slot.active {
                continue;
            }
            while let Some(delivery) = slot.queue.pop_front() {
                let before = self.clock.mono_now().unwrap_or(now);
                let ok = (slot.handler)(&delivery);
                let after = self.clock.mono_now().unwrap_or(now);
                if after.saturating_sub(before) > self.policy.status_handler_timeout_ms {
                    slot.queue.push_front(delivery);
                    break;
                }
                if ok {
                    report.delivered += 1;
                    let latency = now.saturating_sub(delivery.determined_at);
                    self.record_latency(latency);
                } else {
                    slot.failures = slot.failures.saturating_add(1);
                    report.failures += 1;
                    self.diagnostics
                        .bump(DiagnosticCounter::SubscriberFailure, 1);
                }
            }
            report.pending = report.pending.saturating_add(slot.queue.len());
            report.dropped = report.dropped.saturating_add(slot.queue.dropped());
            report.failures = report.failures.saturating_add(0);
        }
        report
    }

    fn record_latency(&self, sample_ms: u64) {
        let mut latency = self
            .latency
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let capacity = usize::try_from(self.policy.status_latency_ring).unwrap_or(256);
        if latency.len() >= capacity {
            latency.pop_front();
        }
        latency.push_back(sample_ms);
    }

    pub fn latency(&self) -> LatencyStats {
        let latency = self
            .latency
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let mut sorted: Vec<u64> = latency.iter().copied().collect();
        if sorted.is_empty() {
            return LatencyStats::default();
        }
        sorted.sort_unstable();
        let count = sorted.len();
        let p50 = sorted[count.div_ceil(2).saturating_sub(1)];
        let p99 = sorted[(count * 99_usize)
            .div_ceil(100)
            .saturating_sub(1)
            .min(count - 1)];
        let max = sorted[count - 1];
        LatencyStats {
            samples: count,
            p50_ms: p50,
            p99_ms: p99,
            max_ms: max,
        }
    }

    pub fn snapshot(&self) -> ChannelSnapshot {
        let mut entries: Vec<ChannelSnapshotEntry> = {
            let snapshots = self
                .snapshots
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            snapshots
                .values()
                .flat_map(|entries| entries.iter().copied())
                .collect()
        };
        entries.sort_by_key(|entry| entry.key);
        ChannelSnapshot {
            entries,
            generated_at: self.clock.mono_now().unwrap_or(MonoMillis::ZERO),
        }
    }

    pub fn snapshot_for(&self, session: &SessionId) -> ChannelSnapshot {
        let mut entries = self
            .snapshots
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .get(session)
            .cloned()
            .unwrap_or_default();
        entries.sort_by_key(|entry| entry.key);
        ChannelSnapshot {
            entries,
            generated_at: self.clock.mono_now().unwrap_or(MonoMillis::ZERO),
        }
    }

    pub fn detach(&self, session: &SessionId) {
        self.snapshots
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .remove(session);
        self.sequences
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .remove(session);
        self.availability_edges
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .remove(session);
    }

    pub fn key_count(&self, session: &SessionId) -> usize {
        self.snapshots
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .get(session)
            .map_or(0, Vec::len)
    }

    pub fn current_sequence(&self, session: &SessionId) -> u64 {
        self.sequences
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .get(session)
            .copied()
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::capture_ports::NoopStatusSink;
    use meeting_copilot_domain::model::{
        ChannelKey, ChannelKind, ChannelState, Platform, StateReason,
    };
    use meeting_copilot_domain::vo::{ClockError, WallClock};

    struct TestClock;

    impl CaptureClock for TestClock {
        fn mono_now(&self) -> Result<MonoMillis, ClockError> {
            Ok(MonoMillis::ZERO)
        }

        fn wall_now(&self) -> Result<WallClock, ClockError> {
            Ok(WallClock::from_unix_millis(0))
        }
    }

    fn publisher() -> StatusPublisher {
        StatusPublisher::new(
            Arc::new(TestClock),
            Arc::new(NoopStatusSink),
            CapturePolicy::default(),
        )
    }

    fn session() -> SessionId {
        SessionId::new("s1").expect("session")
    }

    fn event(at: u64, state: ChannelState, reason: StateReason) -> ChannelStatusEvent {
        ChannelStatusEvent {
            key: ChannelKey::new(Platform::TencentMeeting, ChannelKind::Subtitle),
            state,
            reason,
            sequence: 0,
            determined_at: MonoMillis::from_millis(at),
            wall_at: WallClock::from_unix_millis(0),
        }
    }

    /// AC-B2-OC-D01：同状态 10 次只推 1 次，快照原因已更新。
    #[test]
    fn same_state_ten_times_pushes_once_and_updates_reason() {
        let publisher = publisher();
        let session = session();
        let first = publisher.publish_channel(
            event(1, ChannelState::Available, StateReason::ProbeCompleted),
            &session,
        );
        assert_eq!(first, PublishOutcome::Published { sequence: 1 });
        for at in 2..=10 {
            let outcome = publisher.publish_channel(
                event(at, ChannelState::Available, StateReason::OcrOnly),
                &session,
            );
            assert_eq!(
                outcome,
                PublishOutcome::NotPushed {
                    reason: NotPushedReason::SameState
                }
            );
        }
        let snapshot = publisher.snapshot_for(&session);
        assert_eq!(snapshot.entries.len(), 1);
        assert_eq!(snapshot.entries[0].reason, Some(StateReason::OcrOnly));
        assert_eq!(snapshot.entries[0].entered_at, MonoMillis::from_millis(1));
    }

    /// AC-B2-OC-D03：100 次变更共用一个严格递增序号流。
    #[test]
    fn hundred_changes_share_one_increasing_sequence() {
        let publisher = publisher();
        let session = session();
        let mut states = Vec::new();
        for index in 0..100 {
            let state = if index % 2 == 0 {
                ChannelState::Available
            } else {
                ChannelState::Unavailable
            };
            let outcome = publisher
                .publish_channel(event(index, state, StateReason::ProbeCompleted), &session);
            if let PublishOutcome::Published { sequence } = outcome {
                states.push(sequence);
            }
        }
        assert_eq!(states.len(), 100);
        assert!(states.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(publisher.key_count(&session), 1);
    }

    /// AC-B2-OC-D04：近上界拒绝且不覆盖快照。
    #[test]
    fn seq_near_upper_bound_is_rejected() {
        let publisher = publisher();
        let session = session();
        publisher.seed_sequence(&session, u64::MAX - 1);
        let outcome = publisher.publish_channel(
            event(1, ChannelState::Available, StateReason::ProbeCompleted),
            &session,
        );
        assert_eq!(
            outcome,
            PublishOutcome::Rejected {
                reason: PublishRejectReason::SeqExhausted
            }
        );
        assert_eq!(publisher.key_count(&session), 0);
    }

    /// AC-B2-OC-D07：200 样本时 p99 与最大值按注入时钟计算。
    #[test]
    fn latency_p99_uses_expected_index() {
        let publisher = publisher();
        let session = session();
        let handle = publisher.subscribe(Box::new(|_| true));
        for index in 0..200_u64 {
            let state = if index % 2 == 0 {
                ChannelState::Available
            } else {
                ChannelState::Unavailable
            };
            publisher.publish_channel(event(index, state, StateReason::ProbeCompleted), &session);
            let report = publisher.drain(MonoMillis::from_millis(index + 10));
            assert_eq!(report.delivered, 1);
        }
        handle.release();
        let stats = publisher.latency();
        assert_eq!(stats.samples, 200);
        assert_eq!(stats.p99_ms, 10);
        assert_eq!(stats.max_ms, 10);
        assert!(publisher.policy.status_latency_ring >= 256);
    }

    /// AC-B2-OC-D01：构造即用 Noop 端口。
    #[test]
    fn publisher_constructs_with_noop_sink() {
        let publisher = publisher();
        assert_eq!(
            publisher.publish_availability(
                CaptureAvailabilityEvent {
                    availability: CaptureAvailability::Unavailable {
                        reason: meeting_copilot_domain::model::GapReason::BothChannelsUnavailable,
                        code: meeting_copilot_domain::error::ErrorCode::AudioCaptureFailed,
                    },
                    gap_reason: Some(
                        meeting_copilot_domain::model::GapReason::BothChannelsUnavailable
                    ),
                    code: Some(meeting_copilot_domain::error::ErrorCode::AudioCaptureFailed),
                    sequence: 0,
                },
                &session(),
            ),
            PublishOutcome::Published { sequence: 1 }
        );
    }
}
