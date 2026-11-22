//! C-03 外发租约：固定 3 槽 + 单排队位，无锁、无 await、无定时器。

/// 单调递增租约标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LeaseId(u64);

impl LeaseId {
    pub const fn value(self) -> u64 {
        self.0
    }

    pub const fn from_value(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseOutcome {
    Acquired { lease_id: LeaseId, inflight: u8 },
    Queued { position: u8, queue_len: u8 },
}

/// 固定 3 槽 + 单排队位。
#[derive(Debug, Default)]
pub struct OutboundLease {
    slots: [Option<LeaseId>; 3],
    queue: Vec<LeaseId>,
    next_lease: u64,
}

/// 对外别名（不扩散 `OutboundLease` 到回放面）。
pub type LeaseTable = OutboundLease;

impl OutboundLease {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inflight(&self) -> u8 {
        u8::try_from(self.slots.iter().filter(|slot| slot.is_some()).count()).unwrap_or(3)
    }

    pub fn queue_len(&self) -> u8 {
        u8::try_from(self.queue.len()).unwrap_or(1)
    }

    pub fn try_acquire(&mut self) -> LeaseOutcome {
        let id = LeaseId(self.next_lease);
        self.next_lease = self.next_lease.saturating_add(1);
        if let Some(index) = self.slots.iter().position(Option::is_none) {
            self.slots[index] = Some(id);
            return LeaseOutcome::Acquired {
                lease_id: id,
                inflight: self.inflight(),
            };
        }
        if self.queue.is_empty() {
            self.queue.push(id);
        }
        LeaseOutcome::Queued {
            position: 1,
            queue_len: self.queue_len(),
        }
    }

    /// 释放指定租约；队首提升到最小空槽并返回其 `LeaseId`。
    pub fn release(&mut self, lease_id: LeaseId) -> Option<LeaseId> {
        let index = self.slots.iter().position(|slot| *slot == Some(lease_id))?;
        self.slots[index] = None;
        let promoted = self.queue.first().copied()?;
        let free = self.slots.iter().position(Option::is_none)?;
        self.slots[free] = Some(promoted);
        self.queue.remove(0);
        Some(promoted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inflight_never_exceeds_three() {
        let mut lease = OutboundLease::new();
        for _ in 0..3 {
            assert!(matches!(lease.try_acquire(), LeaseOutcome::Acquired { .. }));
        }
        assert_eq!(lease.inflight(), 3);
        assert_eq!(
            lease.try_acquire(),
            LeaseOutcome::Queued {
                position: 1,
                queue_len: 1
            }
        );
    }
}
