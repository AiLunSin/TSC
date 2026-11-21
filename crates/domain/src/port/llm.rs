//! 结构化 LLM 端口（与既有 `LlmGateway` 并存）。
//!
//! 端口均为同步 trait，实现层负责调度；调用方线程不得执行 socket 读。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::error::CopilotError;
use crate::model::llm::{
    LlmAuditRecord, LlmGenerationRequest, LlmLogRecord, LlmMetricPoint, LlmTask, StreamEvent,
    StreamEventKind, StructuredOutcome,
};
use crate::vo::TaskId;

/// 取消令牌：克隆后共享同一标志。
#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
}

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

/// 流式事件收集器：保证终止唯一，并在终止后只累计丢弃数。
pub struct StreamCollector<F>
where
    F: FnMut(StreamEvent) + Send,
{
    sink: F,
    terminated: bool,
    discarded: u32,
}

impl<F> StreamCollector<F>
where
    F: FnMut(StreamEvent) + Send,
{
    pub fn new(sink: F) -> Self {
        Self {
            sink,
            terminated: false,
            discarded: 0,
        }
    }

    pub fn emit(&mut self, event: StreamEvent) {
        if self.terminated {
            self.discarded = self.discarded.saturating_add(1);
            return;
        }
        if event.kind.is_terminal() {
            self.terminated = true;
        }
        (self.sink)(event);
    }

    pub const fn discarded(&self) -> u32 {
        self.discarded
    }

    pub const fn is_terminated(&self) -> bool {
        self.terminated
    }
}

/// 能力查询结果，供界面展示，不暴露端点细节。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LlmCapability {
    pub task: LlmTask,
    pub endpoint_configured: bool,
    pub model_configured: bool,
    pub cooling: bool,
}

/// 结构化生成端口。
pub trait StructuredLlmGateway: Send + Sync {
    fn complete_structured(
        &self,
        request: &LlmGenerationRequest,
    ) -> Result<StructuredOutcome, CopilotError>;

    fn stream_structured(
        &self,
        request: &LlmGenerationRequest,
        cancel: &CancelToken,
        sink: &mut (dyn FnMut(StreamEvent) + Send),
    ) -> Result<StreamOutcomeSummary, CopilotError>;

    fn capability(&self) -> LlmCapability;
}

/// 流式调用返回的摘要。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamOutcomeSummary {
    pub request_id: TaskId,
    pub terminated: bool,
    pub cancelled: bool,
    pub discarded_deltas: u32,
    pub first_token_ms: Option<u32>,
}

/// 观测端口：度量、审计与白名单日志。
pub trait LlmObserver: Send + Sync {
    fn record_metric(&self, point: LlmMetricPoint) -> Result<(), CopilotError>;

    fn append_audit(&self, record: LlmAuditRecord) -> Result<(), CopilotError>;

    fn log(&self, record: LlmLogRecord);
}

/// 终止事件的便捷判断（供实现层与测试共用）。
pub fn terminal_kind(kind: &StreamEventKind) -> Option<&'static str> {
    match kind {
        StreamEventKind::Completed(_) => Some("completed"),
        StreamEventKind::Failed { .. } => Some("failed"),
        StreamEventKind::Delta { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CopilotError;
    use crate::model::llm::{FailureClass, LlmMetric, LlmTask};

    struct NullGateway;

    impl StructuredLlmGateway for NullGateway {
        fn complete_structured(
            &self,
            request: &LlmGenerationRequest,
        ) -> Result<StructuredOutcome, CopilotError> {
            Err(CopilotError::ProviderConfigInvalid {
                detail: format!("未实现：{}", request.task.as_str()),
            })
        }

        fn stream_structured(
            &self,
            _request: &LlmGenerationRequest,
            _cancel: &CancelToken,
            _sink: &mut (dyn FnMut(StreamEvent) + Send),
        ) -> Result<StreamOutcomeSummary, CopilotError> {
            Err(CopilotError::ProviderConfigInvalid {
                detail: "未实现".into(),
            })
        }

        fn capability(&self) -> LlmCapability {
            LlmCapability {
                task: LlmTask::Summary,
                endpoint_configured: false,
                model_configured: false,
                cooling: false,
            }
        }
    }

    #[test]
    fn gateway_is_object_safe() {
        let gateway: Arc<dyn StructuredLlmGateway> = Arc::new(NullGateway);
        assert!(!gateway.capability().endpoint_configured);
    }

    #[test]
    fn cancel_token_is_shared_across_clones() {
        let token = CancelToken::new();
        let clone = token.clone();
        assert!(!clone.is_cancelled());
        token.cancel();
        assert!(clone.is_cancelled());
    }

    #[test]
    fn collector_discards_after_terminal() {
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink_events = events.clone();
        let mut collector = StreamCollector::new(move |event: StreamEvent| {
            sink_events.lock().expect("锁").push(event.sequence);
        });
        collector.emit(StreamEvent {
            sequence: 1,
            kind: StreamEventKind::Delta {
                text: "第一段".into(),
                first_token: true,
            },
        });
        collector.emit(StreamEvent {
            sequence: 2,
            kind: StreamEventKind::Failed {
                failure: FailureClass::Timeout,
                degraded: true,
                cause: crate::model::llm::DegradeCause::Failed(FailureClass::Timeout),
            },
        });
        collector.emit(StreamEvent {
            sequence: 3,
            kind: StreamEventKind::Delta {
                text: "迟到".into(),
                first_token: false,
            },
        });
        assert_eq!(collector.discarded(), 1);
        assert_eq!(collector.discarded() + 2, 3);
        assert_eq!(events.lock().expect("锁").len(), 2);
    }

    #[test]
    fn observer_is_object_safe() {
        struct NullObserver;
        impl LlmObserver for NullObserver {
            fn record_metric(&self, _point: LlmMetricPoint) -> Result<(), CopilotError> {
                Ok(())
            }
            fn append_audit(&self, _record: LlmAuditRecord) -> Result<(), CopilotError> {
                Ok(())
            }
            fn log(&self, _record: LlmLogRecord) {}
        }
        let observer: Arc<dyn LlmObserver> = Arc::new(NullObserver);
        assert!(observer
            .record_metric(LlmMetricPoint {
                metric: LlmMetric::TotalLatencyMs,
                value: 1.0,
                task: Some(LlmTask::Summary),
                failure_class: None,
            })
            .is_ok());
    }

    #[test]
    fn terminal_kind_is_explicit() {
        assert!(terminal_kind(&StreamEventKind::Delta {
            text: "x".into(),
            first_token: false
        })
        .is_none());
    }
}
