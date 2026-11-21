//! 应用层离线替身集（AC-B2-OC-E01~E05、E13）。
//!
//! 只保留追加 / 合并 / 计数 / 单调时钟语义；不提供更新、删除、清空或通用发送入口。

#![allow(dead_code)]

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

use meeting_copilot_application::capture::{
    merge_gaps, merge_same_millisecond, AsrDeliverySink, AsrIntent, AsrIntentKind,
    CaptureGapRecord, CapturePortError, ChannelGapLog, ChannelStatusEvent, ChannelStatusSink,
    ChannelSwitchLog, ChannelSwitchRecord, CountingDiagnostics, InsightInvariantPort,
    InsightReadError, SwitchLogError,
};
use meeting_copilot_domain::port::CaptureClock;
use meeting_copilot_domain::vo::{ClockError, MonoMillis, SessionId, WallClock};

/// 只追加缺口日志；`query` 时按 1000ms 窗口合并相邻/重叠记录。
#[derive(Debug, Default)]
pub struct MemoryGapLog {
    records: Mutex<Vec<CaptureGapRecord>>,
}

impl MemoryGapLog {
    pub fn query(&self, session: &SessionId) -> Vec<CaptureGapRecord> {
        let records = self
            .records
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .iter()
            .filter(|record| record.session == *session)
            .cloned()
            .collect();
        merge_gaps(records, 1_000)
    }

    pub fn raw_len(&self) -> usize {
        self.records
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .len()
    }
}

impl ChannelGapLog for MemoryGapLog {
    fn append(&self, record: &CaptureGapRecord) -> Result<(), CapturePortError> {
        self.records
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .push(record.clone());
        Ok(())
    }
}

/// 只追加切换日志；同会话同毫秒合并为 1 条。
#[derive(Debug, Default)]
pub struct MemorySwitchLog {
    records: Mutex<Vec<ChannelSwitchRecord>>,
}

impl MemorySwitchLog {
    pub fn records(&self) -> Vec<ChannelSwitchRecord> {
        let merged = merge_same_millisecond(
            self.records
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .clone(),
        );
        merged
    }

    pub fn raw_len(&self) -> usize {
        self.records
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .len()
    }
}

impl ChannelSwitchLog for MemorySwitchLog {
    fn append(&self, record: &ChannelSwitchRecord) -> Result<(), SwitchLogError> {
        self.records
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .push(record.clone());
        Ok(())
    }
}

/// 三类 ASR 意图分别计数。
#[derive(Debug, Default)]
pub struct CountingAsrDelivery {
    validation: AtomicU64,
    backfill: AtomicU64,
    primary: AtomicU64,
    last: Mutex<Option<AsrIntent>>,
}

impl CountingAsrDelivery {
    pub fn count(&self, kind: AsrIntentKind) -> u64 {
        match kind {
            AsrIntentKind::Validation => self.validation.load(Ordering::SeqCst),
            AsrIntentKind::GapBackfill => self.backfill.load(Ordering::SeqCst),
            AsrIntentKind::Primary => self.primary.load(Ordering::SeqCst),
        }
    }

    pub fn last(&self) -> Option<AsrIntent> {
        self.last
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }
}

impl AsrDeliverySink for CountingAsrDelivery {
    fn deliver(&self, intent: &AsrIntent) -> Result<(), CapturePortError> {
        match intent.kind {
            AsrIntentKind::Validation => &self.validation,
            AsrIntentKind::GapBackfill => &self.backfill,
            AsrIntentKind::Primary => &self.primary,
        }
        .fetch_add(1, Ordering::SeqCst);
        *self
            .last
            .lock()
            .unwrap_or_else(|poison| poison.into_inner()) = Some(intent.clone());
        Ok(())
    }

    fn current_task(&self) -> Option<meeting_copilot_domain::model::llm::LlmTask> {
        None
    }
}

/// 审计条目：仅类别、时刻与计数，不含载荷。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEntry {
    pub kind: &'static str,
    pub at_millis: u64,
    pub count: u64,
}

#[derive(Debug, Default)]
pub struct CountingAudit {
    entries: Mutex<Vec<AuditEntry>>,
}

impl CountingAudit {
    pub fn record(&self, kind: &'static str, at_millis: u64, count: u64) {
        self.entries
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .push(AuditEntry {
                kind,
                at_millis,
                count,
            });
    }

    pub fn entries(&self) -> Vec<AuditEntry> {
        self.entries
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }
}

/// 只读要点替身：可注入失败，便于验证"失败不降级为删除"。
#[derive(Debug, Default)]
pub struct CountingInsight {
    ids: Mutex<Vec<String>>,
    fail: AtomicBool,
}

impl CountingInsight {
    pub fn seed(&self, ids: Vec<String>) {
        *self.ids.lock().unwrap_or_else(|poison| poison.into_inner()) = ids;
    }

    pub fn set_failure(&self, fail: bool) {
        self.fail.store(fail, Ordering::SeqCst);
    }

    pub fn ids(&self) -> Vec<String> {
        self.ids
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }
}

impl InsightInvariantPort for CountingInsight {
    fn read_insight_ids(&self, _session: &SessionId) -> Result<Vec<String>, InsightReadError> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(InsightReadError::Unavailable);
        }
        Ok(self.ids())
    }
}

/// 写/删/迁移哨兵：守卫期间应恒为 0。
#[derive(Debug, Default)]
pub struct SentinelRepos {
    writes: AtomicU64,
    deletes: AtomicU64,
    migrates: AtomicU64,
    insight_writes: AtomicU64,
    summary_writes: AtomicU64,
}

impl SentinelRepos {
    pub fn write(&self) {
        self.writes.fetch_add(1, Ordering::SeqCst);
    }

    pub fn delete(&self) {
        self.deletes.fetch_add(1, Ordering::SeqCst);
    }

    pub fn migrate(&self) {
        self.migrates.fetch_add(1, Ordering::SeqCst);
    }

    pub fn write_insight(&self) {
        self.insight_writes.fetch_add(1, Ordering::SeqCst);
    }

    pub fn write_summary(&self) {
        self.summary_writes.fetch_add(1, Ordering::SeqCst);
    }

    pub fn counts(&self) -> (u64, u64, u64, u64, u64) {
        (
            self.writes.load(Ordering::SeqCst),
            self.deletes.load(Ordering::SeqCst),
            self.migrates.load(Ordering::SeqCst),
            self.insight_writes.load(Ordering::SeqCst),
            self.summary_writes.load(Ordering::SeqCst),
        )
    }
}

/// 建连计数：离线装配的最小路径恒为 0。
#[derive(Debug, Default)]
pub struct ConnectCounter {
    attempts: AtomicU64,
}

impl ConnectCounter {
    pub fn record_attempt(&self) {
        self.attempts.fetch_add(1, Ordering::SeqCst);
    }

    pub fn connections(&self) -> u64 {
        self.attempts.load(Ordering::SeqCst)
    }
}

/// 通道状态事件记录器。
#[derive(Debug, Default)]
pub struct RecordingStatusSink {
    events: Mutex<Vec<ChannelStatusEvent>>,
}

impl RecordingStatusSink {
    pub fn events(&self) -> Vec<ChannelStatusEvent> {
        self.events
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }

    pub fn len(&self) -> usize {
        self.events
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl ChannelStatusSink for RecordingStatusSink {
    fn publish(&self, event: &ChannelStatusEvent) -> Result<(), CapturePortError> {
        self.events
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .push(*event);
        Ok(())
    }
}

/// 诊断替身直接复用生产实现，不复制逻辑。
pub type DiagnosticsFake = CountingDiagnostics;
/// 单调系统时钟替身：单进程内单调不减，不读真实时间。
#[derive(Debug)]
pub struct MonotonicSystemClock {
    mono_ms: AtomicU64,
    wall_base: WallClock,
}

impl Default for MonotonicSystemClock {
    fn default() -> Self {
        Self {
            mono_ms: AtomicU64::new(0),
            wall_base: WallClock::from_unix_millis(0),
        }
    }
}

impl MonotonicSystemClock {
    pub fn new(wall_base: WallClock) -> Self {
        Self {
            mono_ms: AtomicU64::new(0),
            wall_base,
        }
    }
}

impl CaptureClock for MonotonicSystemClock {
    fn mono_now(&self) -> Result<MonoMillis, ClockError> {
        Ok(MonoMillis::from_millis(
            self.mono_ms.fetch_add(1, Ordering::SeqCst),
        ))
    }

    fn wall_now(&self) -> Result<WallClock, ClockError> {
        Ok(self.wall_base)
    }
}
