//! 采集服务运行期端口（gap / status / asr）。
//!
//! 本轮只提供契约与 Noop 实现；真实实现留给平台/UI/ASR 装配。

use meeting_copilot_domain::model::llm::LlmTask;

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
