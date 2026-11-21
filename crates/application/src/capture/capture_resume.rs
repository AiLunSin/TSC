//! 恢复账本（B2.23 阶段 C）：记录已交付要点、缺口边界与恢复起点。
//!
//! 纯逻辑；不使用墙钟、不写日志、不持有文本。

use std::collections::BTreeSet;

use meeting_copilot_domain::vo::MonoMillis;

pub struct CaptureResumeLedger {
    delivered: BTreeSet<String>,
    gap_start: Option<MonoMillis>,
    last_delivered_at: Option<MonoMillis>,
}

impl Default for CaptureResumeLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl CaptureResumeLedger {
    pub fn new() -> Self {
        Self {
            delivered: BTreeSet::new(),
            gap_start: None,
            last_delivered_at: None,
        }
    }

    /// 重复要点返回 false，不重复计数。
    pub fn record_point(&mut self, point_id: &str, at: MonoMillis) -> bool {
        if !self.delivered.insert(point_id.to_owned()) {
            return false;
        }
        self.last_delivered_at = Some(at);
        true
    }

    pub fn has_point(&self, point_id: &str) -> bool {
        self.delivered.contains(point_id)
    }

    pub fn delivered_count(&self) -> usize {
        self.delivered.len()
    }

    pub fn begin_gap(&mut self, at: MonoMillis) {
        self.gap_start.get_or_insert(at);
    }

    pub fn in_gap(&self) -> bool {
        self.gap_start.is_some()
    }

    /// 返回恢复起点（缺口开始时刻）；无缺口返回 None。
    pub fn end_gap(&mut self) -> Option<MonoMillis> {
        self.gap_start.take()
    }

    pub const fn last_delivered_at(&self) -> Option<MonoMillis> {
        self.last_delivered_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_points_are_rejected() {
        let mut ledger = CaptureResumeLedger::new();
        assert!(ledger.record_point("p1", MonoMillis::from_millis(10)));
        assert!(!ledger.record_point("p1", MonoMillis::from_millis(20)));
        assert_eq!(ledger.delivered_count(), 1);
    }

    #[test]
    fn gap_start_and_end_are_explicit() {
        let mut ledger = CaptureResumeLedger::new();
        ledger.begin_gap(MonoMillis::from_millis(100));
        assert!(ledger.in_gap());
        assert_eq!(ledger.end_gap(), Some(MonoMillis::from_millis(100)));
        assert!(!ledger.in_gap());
        assert_eq!(ledger.end_gap(), None);
    }

    #[test]
    fn delivered_points_survive_gap() {
        let mut ledger = CaptureResumeLedger::new();
        ledger.record_point("p1", MonoMillis::from_millis(10));
        ledger.begin_gap(MonoMillis::from_millis(100));
        assert!(ledger.has_point("p1"));
        ledger.end_gap();
        assert!(ledger.has_point("p1"));
        assert_eq!(ledger.delivered_count(), 1);
    }

    #[test]
    fn resume_does_not_redeliver_old_points() {
        let mut ledger = CaptureResumeLedger::new();
        ledger.record_point("p1", MonoMillis::from_millis(10));
        ledger.begin_gap(MonoMillis::from_millis(100));
        ledger.end_gap();
        assert!(!ledger.record_point("p1", MonoMillis::from_millis(200)));
        assert!(ledger.record_point("p2", MonoMillis::from_millis(201)));
        assert_eq!(ledger.delivered_count(), 2);
    }
}
