//! 订阅槽、有界队列与释放语义（AC-B2-OC-D05/D06）。

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use super::types::CaptureStatusDelivery;

/// 回调返回 `true` 表示处理完成；`false` 视作处理失败（只计数）。
pub type StatusHandler = Box<dyn Fn(&CaptureStatusDelivery) -> bool + Send + Sync + 'static>;

/// 每订阅者固定容量的有界队列：满时丢最旧，入队不阻塞。
pub struct DeliveryQueue {
    capacity: usize,
    items: VecDeque<CaptureStatusDelivery>,
    dropped: u64,
}

impl DeliveryQueue {
    pub fn new(capacity: u64) -> Self {
        let capacity = usize::try_from(capacity).unwrap_or(usize::MAX).max(1);
        Self {
            capacity,
            items: VecDeque::with_capacity(capacity.min(1024)),
            dropped: 0,
        }
    }

    /// 返回是否发生丢弃。
    pub fn push(&mut self, delivery: CaptureStatusDelivery) -> bool {
        if self.items.len() >= self.capacity {
            self.items.pop_front();
            self.dropped = self.dropped.saturating_add(1);
            self.items.push_back(delivery);
            return true;
        }
        self.items.push_back(delivery);
        false
    }

    pub fn pop_front(&mut self) -> Option<CaptureStatusDelivery> {
        self.items.pop_front()
    }

    /// 处理超时的事件回到队首，不丢失。
    pub fn push_front(&mut self, delivery: CaptureStatusDelivery) {
        if self.items.len() >= self.capacity {
            self.items.pop_back();
            self.dropped = self.dropped.saturating_add(1);
        }
        self.items.push_front(delivery);
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    pub const fn dropped(&self) -> u64 {
        self.dropped
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }
}

pub(crate) struct Slot {
    #[allow(dead_code)]
    pub(crate) id: u64,
    pub(crate) active: bool,
    pub(crate) handler: StatusHandler,
    pub(crate) queue: DeliveryQueue,
    pub(crate) failures: u64,
}

pub(crate) type SlotList = Arc<Mutex<Vec<Arc<Mutex<Slot>>>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DrainReport {
    pub delivered: usize,
    pub pending: usize,
    pub dropped: u64,
    pub failures: u64,
}

/// 释放幂等：`release` 之后该订阅者回调恒为 0。
pub struct SubscriptionHandle {
    slot: Arc<Mutex<Slot>>,
    slots: SlotList,
    released: AtomicBool,
}

impl SubscriptionHandle {
    pub(crate) fn new(slot: Arc<Mutex<Slot>>, slots: SlotList) -> Self {
        Self {
            slot,
            slots,
            released: AtomicBool::new(false),
        }
    }

    pub fn release(&self) {
        if self.released.swap(true, Ordering::AcqRel) {
            return;
        }
        {
            let mut slot = self
                .slot
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            slot.active = false;
            slot.queue.clear();
        }
        let mut slots = self
            .slots
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        slots.retain(|candidate| !Arc::ptr_eq(candidate, &self.slot));
    }

    pub fn is_active(&self) -> bool {
        !self.released.load(Ordering::Acquire)
            && self
                .slot
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .active
    }
}

impl std::fmt::Debug for SubscriptionHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SubscriptionHandle")
            .field("released", &self.released.load(Ordering::Acquire))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meeting_copilot_domain::model::{ChannelKey, ChannelKind, Platform, StateReason};
    use meeting_copilot_domain::vo::{MonoMillis, SessionId, WallClock};

    fn delivery(sequence: u64) -> CaptureStatusDelivery {
        CaptureStatusDelivery {
            session: SessionId::new("s1").expect("session"),
            channel: Some(super::super::types::ChannelStatusEvent {
                key: ChannelKey::new(Platform::TencentMeeting, ChannelKind::Subtitle),
                state: meeting_copilot_domain::model::ChannelState::Available,
                reason: StateReason::ProbeCompleted,
                sequence,
                determined_at: MonoMillis::from_millis(sequence),
                wall_at: WallClock::from_unix_millis(0),
            }),
            availability: None,
            determined_at: MonoMillis::from_millis(sequence),
        }
    }

    /// AC-B2-OC-D05：容量 64 满时丢最旧并计数。
    #[test]
    fn full_queue_drops_oldest_and_counts() {
        let mut queue = DeliveryQueue::new(64);
        for sequence in 0..64 {
            assert!(!queue.push(delivery(sequence)));
        }
        assert!(queue.push(delivery(64)));
        assert_eq!(queue.len(), 64);
        assert_eq!(queue.dropped(), 1);
        assert_eq!(
            queue.pop_front().map(|item| item.determined_at.as_millis()),
            Some(1)
        );
    }

    /// AC-B2-OC-D06：释放幂等且队列被清空。
    #[test]
    fn released_subscriber_slot_is_removed_once() {
        let slot = Arc::new(Mutex::new(Slot {
            id: 1,
            active: true,
            handler: Box::new(|_| true),
            queue: DeliveryQueue::new(64),
            failures: 0,
        }));
        let slots: SlotList = Arc::new(Mutex::new(vec![Arc::clone(&slot)]));
        let handle = SubscriptionHandle::new(Arc::clone(&slot), Arc::clone(&slots));
        handle.release();
        handle.release();
        assert!(slots.lock().expect("lock").is_empty());
        assert!(!handle.is_active());
    }
}
