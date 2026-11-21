//! 采集服务运行期端口（gap / status / asr）。
//!
//! 本轮只提供契约与 Noop 实现；真实实现留给平台/UI/ASR 装配。

use meeting_copilot_domain::model::llm::LlmTask;
use meeting_copilot_domain::vo::SessionId;

use super::diagnostics::{CountingDiagnostics, DiagnosticCounter, DiagnosticsSnapshot};
use super::types::{AsrIntent, CaptureGapRecord, ChannelStatusEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("capture port unavailable")]
pub struct CapturePortError;

pub trait ChannelGapLog: Send + Sync {
    fn append(&self, record: &CaptureGapRecord) -> Result<(), CapturePortError>;
}

pub trait ChannelStatusSink: Send + Sync {
    fn publish(&self, event: &ChannelStatusEvent) -> Result<(), CapturePortError>;
}

pub trait AsrDeliverySink: Send + Sync {
    fn deliver(&self, intent: &AsrIntent) -> Result<(), CapturePortError>;
    fn current_task(&self) -> Option<LlmTask>;
}

#[derive(Debug, Default)]
pub struct NoopGapLog;

impl ChannelGapLog for NoopGapLog {
    fn append(&self, _record: &CaptureGapRecord) -> Result<(), CapturePortError> {
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct NoopStatusSink;

impl ChannelStatusSink for NoopStatusSink {
    fn publish(&self, _event: &ChannelStatusEvent) -> Result<(), CapturePortError> {
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct NoopAsrDelivery;

impl AsrDeliverySink for NoopAsrDelivery {
    fn deliver(&self, _intent: &AsrIntent) -> Result<(), CapturePortError> {
        Ok(())
    }

    fn current_task(&self) -> Option<LlmTask> {
        None
    }
}
/// 要点只读校验失败：不外抛，只转为 `InsightCheckSkipped` 计数。
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum InsightReadError {
    #[error("要点存储不可用")]
    Unavailable,
    #[error("要点数据损坏")]
    Corrupted,
}

/// 只读要点端口：仅允许读取 ID 集合，不提供任何写/删/清空方法。
pub trait InsightInvariantPort: Send + Sync {
    fn read_insight_ids(&self, session: &SessionId) -> Result<Vec<String>, InsightReadError>;
}

/// 诊断计数端口：只接收封闭枚举与增量。
pub trait DiagnosticsSink: Send + Sync {
    fn bump(&self, counter: DiagnosticCounter, delta: u64);
    fn snapshot(&self) -> DiagnosticsSnapshot;
}

/// 只读替身：无要点时返回空集合。
#[derive(Debug, Default)]
pub struct NoopInsight;

impl InsightInvariantPort for NoopInsight {
    fn read_insight_ids(&self, _session: &SessionId) -> Result<Vec<String>, InsightReadError> {
        Ok(Vec::new())
    }
}

/// 诊断替身：不记录、不 panic。
#[derive(Debug, Default)]
pub struct NoopDiagnostics;

impl DiagnosticsSink for NoopDiagnostics {
    fn bump(&self, _counter: DiagnosticCounter, _delta: u64) {}

    fn snapshot(&self) -> DiagnosticsSnapshot {
        DiagnosticsSnapshot::ZERO
    }
}

impl DiagnosticsSink for CountingDiagnostics {
    fn bump(&self, counter: DiagnosticCounter, delta: u64) {
        CountingDiagnostics::bump(self, counter, delta);
    }

    fn snapshot(&self) -> DiagnosticsSnapshot {
        CountingDiagnostics::snapshot(self)
    }
}
