//! LLM 网关装配与可观测适配（DESIGN 2.2.2-G、2.1.2）。
//!
//! 网关对"真实联调"零感知；生产 HTTPS 传输由 `llm-https` feature 挂起。

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::enums::AuditAction;
use meeting_copilot_domain::model::llm::{
    FailureClass, LlmAuditRecord, LlmLogRecord, LlmMetric, LlmMetricPoint, LlmTask, TaskBudget,
};
use meeting_copilot_domain::model::provider::DomainPattern;
use meeting_copilot_domain::port::llm::{LlmObserver, StructuredLlmGateway};
use meeting_copilot_domain::vo::{ActorRef, AuditId, ResourceRef, WallClock};

use crate::audit::AuditChain;
use crate::llm::degrade::{Clock, EndpointCircuit, SystemClock};
use crate::llm::gateway::StructuredGateway;
use crate::llm::transport::HttpTransport;

pub mod degrade;
pub mod fallback;
pub mod gateway;
pub mod live;
pub mod schema;
pub mod transport;
#[cfg(feature = "llm-https")]
pub mod transport_tls;
pub mod wire;

/// 配置中的凭据引用：只保存引用名，永不保存凭据值。
#[derive(Clone, PartialEq, Eq)]
pub struct CredentialRef(String);

impl CredentialRef {
    pub fn new(reference: impl Into<String>) -> Self {
        Self(reference.into())
    }

    pub fn reference(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for CredentialRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

impl fmt::Display for CredentialRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

/// 网关配置快照。`Arc<RwLock<_>>` 整体替换，在途请求持有开始时的快照。
#[derive(Debug, Clone, PartialEq)]
pub struct LlmConfig {
    pub schema_version: String,
    pub endpoint_id: String,
    pub endpoint: String,
    pub model: String,
    pub domain_allowlist: Vec<DomainPattern>,
    pub request_timeout_ms: u64,
    pub max_retries: u8,
    pub cooling_threshold: u32,
    pub cooling_ms: u64,
    pub max_concurrency: usize,
    pub payload_char_limit: usize,
    pub max_response_bytes: usize,
    pub max_request_bytes: usize,
    pub max_stream_chars: usize,
    pub defaults: [TaskBudget; 4],
    pub credential_ref: CredentialRef,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            schema_version: "llm.config.v1".into(),
            endpoint_id: "default".into(),
            endpoint: "http://127.0.0.1:8080/v1/chat/completions".into(),
            model: "test-model".into(),
            domain_allowlist: vec![DomainPattern::new("127.0.0.1").expect("常量合法")],
            request_timeout_ms: 6_000,
            max_retries: 2,
            cooling_threshold: 3,
            cooling_ms: 60_000,
            max_concurrency: 3,
            payload_char_limit: 8_000,
            max_response_bytes: 1_048_576,
            max_request_bytes: 65_536,
            max_stream_chars: 65_536,
            defaults: [
                LlmTask::InsightExtract.default_budget(),
                LlmTask::Summary.default_budget(),
                LlmTask::CardConclusion.default_budget(),
                LlmTask::MinutesDraft.default_budget(),
            ],
            credential_ref: CredentialRef::new("env:LLM_API_KEY"),
        }
    }
}

impl LlmConfig {
    pub fn validate(&self) -> Result<(), CopilotError> {
        if self.domain_allowlist.is_empty() {
            return Err(config_error("domain_allowlist", "不能为空"));
        }
        let (_, _, _, _, host) = self
            .endpoint_parts()
            .ok_or_else(|| config_error("endpoint", "必须为 http(s) 绝对地址"))?;
        if !self
            .domain_allowlist
            .iter()
            .any(|pattern| pattern.matches(&host))
        {
            return Err(config_error("endpoint.host", "未命中域名白名单"));
        }
        if self.model.trim().is_empty() || self.model.chars().count() > 128 {
            return Err(config_error("model", "不能为空且不超过 128 字符"));
        }
        if !(100..=8_000).contains(&self.request_timeout_ms) {
            return Err(config_error("request_timeout_ms", "必须落在 100..=8000"));
        }
        if !(1..=8).contains(&self.max_concurrency) {
            return Err(config_error("max_concurrency", "必须落在 1..=8"));
        }
        if !(1..=10).contains(&self.cooling_threshold) {
            return Err(config_error("cooling_threshold", "必须落在 1..=10"));
        }
        if !(1_000..=600_000).contains(&self.cooling_ms) {
            return Err(config_error("cooling_ms", "必须落在 1000..=600000"));
        }
        if !(1_024..=1_048_576).contains(&self.max_request_bytes) {
            return Err(config_error("max_request_bytes", "必须落在 1024..=1048576"));
        }
        if self.payload_char_limit == 0
            || self.max_response_bytes == 0
            || self.max_stream_chars == 0
        {
            return Err(config_error("limits", "上限必须大于 0"));
        }
        for budget in self.defaults {
            budget
                .validate()
                .map_err(|_| config_error("defaults", "预算越界"))?;
        }
        Ok(())
    }

    /// 返回 `(secure, host, port, path, host_only)`。
    pub fn endpoint_parts(&self) -> Option<(bool, String, u16, String, String)> {
        let (scheme, rest) = self.endpoint.split_once("://")?;
        let secure = match scheme.to_ascii_lowercase().as_str() {
            "http" => false,
            "https" => true,
            _ => return None,
        };
        let (authority, path) = match rest.split_once('/') {
            Some((authority, path)) => (authority, format!("/{path}")),
            None => (rest, "/".to_string()),
        };
        if authority.is_empty() || authority.contains(char::is_whitespace) {
            return None;
        }
        let (host, port) = match authority.rsplit_once(':') {
            Some((host, port)) => (host.to_owned(), port.parse::<u16>().ok()?),
            None => (authority.to_owned(), if secure { 443 } else { 80 }),
        };
        if host.is_empty() {
            return None;
        }
        Some((secure, host.clone(), port, path, host))
    }

    pub fn budget_for(&self, task: LlmTask) -> TaskBudget {
        match task {
            LlmTask::InsightExtract => self.defaults[0],
            LlmTask::Summary => self.defaults[1],
            LlmTask::CardConclusion => self.defaults[2],
            LlmTask::MinutesDraft => self.defaults[3],
        }
    }
}

fn config_error(field: &str, rule: &str) -> CopilotError {
    CopilotError::ProviderConfigInvalid {
        detail: format!("{field}: {rule}"),
    }
}

/// 有界环形度量缓冲（每指标最近 1024 点）。
#[derive(Debug, Default)]
pub struct LlmMetricsBuffer {
    inner: Mutex<MetricsInner>,
}

#[derive(Debug, Default)]
struct MetricsInner {
    points: HashMap<LlmMetric, VecDeque<f64>>,
    failures: VecDeque<FailureClass>,
    dropped: u64,
}

impl LlmMetricsBuffer {
    pub const CAPACITY: usize = 1_024;

    pub fn record(&self, point: &LlmMetricPoint) {
        let mut inner = self
            .inner
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if point.metric == LlmMetric::MetricsDropped {
            inner.dropped = inner.dropped.saturating_add(1);
            return;
        }
        let queue = inner.points.entry(point.metric).or_default();
        let overflow = queue.len() >= Self::CAPACITY;
        if overflow {
            queue.pop_front();
        }
        queue.push_back(point.value);
        if overflow {
            let dropped = inner.dropped;
            inner.dropped = dropped.saturating_add(1);
        }
        if point.metric == LlmMetric::FailureCount {
            if let Some(class) = point.failure_class {
                if inner.failures.len() >= Self::CAPACITY {
                    inner.failures.pop_front();
                }
                inner.failures.push_back(class);
            }
        }
    }

    pub fn p95(&self, metric: LlmMetric) -> Option<f64> {
        let inner = self
            .inner
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let values = inner.points.get(&metric)?;
        if values.is_empty() {
            return None;
        }
        let mut sorted: Vec<f64> = values.iter().copied().collect();
        sorted.sort_by(f64::total_cmp);
        let rank = ((0.95 * sorted.len() as f64).ceil() as usize).max(1);
        sorted.get(rank - 1).copied()
    }

    pub fn failure_rate(&self, class: FailureClass) -> f64 {
        let inner = self
            .inner
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if inner.failures.is_empty() {
            return 0.0;
        }
        let hits = inner.failures.iter().filter(|item| **item == class).count();
        hits as f64 / inner.failures.len() as f64
    }

    pub fn dropped(&self) -> u64 {
        self.inner
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .dropped
    }
}

/// 进程内观测适配：度量缓冲 + 只追加审计 + 白名单日志。
#[derive(Debug, Default)]
pub struct LocalObserver {
    pub metrics: LlmMetricsBuffer,
    audit: Mutex<AuditChain>,
    logs: Mutex<Vec<LlmLogRecord>>,
}

impl LocalObserver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn audit_records(&self) -> Vec<meeting_copilot_domain::model::governance::AuditRecord> {
        self.audit
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .records()
            .to_vec()
    }

    pub fn logs(&self) -> Vec<LlmLogRecord> {
        self.logs
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }
}

impl LlmObserver for LocalObserver {
    fn record_metric(&self, point: LlmMetricPoint) -> Result<(), CopilotError> {
        self.metrics.record(&point);
        Ok(())
    }

    fn append_audit(&self, record: LlmAuditRecord) -> Result<(), CopilotError> {
        let id = format!(
            "{}-{}",
            record.request_id.as_str(),
            LlmAuditRecord::phase_str(record.phase)
        );
        let audit_id = AuditId::new(id).map_err(|_| CopilotError::StoreFailure {
            detail: "审计 ID 非法".into(),
        })?;
        let actor = ActorRef::new("llm-gateway").map_err(|_| CopilotError::StoreFailure {
            detail: "审计操作者非法".into(),
        })?;
        let target =
            ResourceRef::new(record.task.as_str()).map_err(|_| CopilotError::StoreFailure {
                detail: "审计对象非法".into(),
            })?;
        let occurred_at = WallClock::now().unwrap_or(WallClock::from_unix_millis(0));
        let summary = record.summary();
        self.audit
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .append(
                audit_id,
                AuditAction::OutboundQuery,
                actor,
                occurred_at,
                target,
                summary,
            );
        Ok(())
    }

    fn log(&self, record: LlmLogRecord) {
        self.logs
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .push(record);
    }
}

/// 装配入口：不读取任何环境变量。
pub fn assemble(
    config: Arc<RwLock<LlmConfig>>,
    transport: Arc<dyn HttpTransport>,
    observer: Arc<dyn LlmObserver>,
) -> Result<Arc<dyn StructuredLlmGateway>, CopilotError> {
    assemble_with_clock(config, transport, observer, Arc::new(SystemClock))
}

pub fn assemble_with_clock(
    config: Arc<RwLock<LlmConfig>>,
    transport: Arc<dyn HttpTransport>,
    observer: Arc<dyn LlmObserver>,
    clock: Arc<dyn Clock>,
) -> Result<Arc<dyn StructuredLlmGateway>, CopilotError> {
    {
        let snapshot = config.read().unwrap_or_else(|poison| poison.into_inner());
        snapshot.validate()?;
    }
    let gateway = StructuredGateway::new(config, transport, observer, clock);
    Ok(Arc::new(gateway))
}

/// 供能力查询与测试使用的冷却快照。
pub fn cooling_state(circuit: &Mutex<EndpointCircuit>, now: Instant) -> bool {
    matches!(
        circuit
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .state(now),
        meeting_copilot_domain::model::llm::CircuitState::Open { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use meeting_copilot_domain::model::llm::AuditPhase;
    use meeting_copilot_domain::vo::TaskId;

    #[test]
    fn config_validation_rejects_bad_values() {
        let mut config = LlmConfig::default();
        assert!(config.validate().is_ok());

        config.domain_allowlist.clear();
        assert!(config.validate().is_err());

        let config = LlmConfig {
            request_timeout_ms: 99,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        let config = LlmConfig {
            max_concurrency: 9,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        let config = LlmConfig {
            model: String::new(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn endpoint_parts_are_parsed_without_query() {
        let config = LlmConfig {
            endpoint: "https://example.invalid:8443/v1/chat?token=x".into(),
            ..Default::default()
        };
        let (secure, host, port, path, host_only) = config.endpoint_parts().expect("parts");
        assert!(secure);
        assert_eq!(host, "example.invalid");
        assert_eq!(port, 8443);
        assert_eq!(path, "/v1/chat?token=x");
        assert_eq!(host_only, "example.invalid");
    }

    #[test]
    fn credential_ref_is_masked() {
        let reference = CredentialRef::new("env:LLM_API_KEY");
        assert_eq!(format!("{reference:?}"), "***");
        assert_eq!(format!("{reference}"), "***");
    }

    #[test]
    fn metrics_buffer_is_bounded_and_computes_p95() {
        let buffer = LlmMetricsBuffer::default();
        for value in 1..=100 {
            buffer.record(&LlmMetricPoint {
                metric: LlmMetric::TotalLatencyMs,
                value: f64::from(value),
                task: None,
                failure_class: None,
            });
        }
        assert_eq!(buffer.p95(LlmMetric::TotalLatencyMs), Some(95.0));
        assert_eq!(buffer.dropped(), 0);
    }

    #[test]
    fn failure_rate_uses_class_dimension() {
        let buffer = LlmMetricsBuffer::default();
        for class in [
            FailureClass::Timeout,
            FailureClass::Timeout,
            FailureClass::Offline,
        ] {
            buffer.record(&LlmMetricPoint {
                metric: LlmMetric::FailureCount,
                value: 1.0,
                task: None,
                failure_class: Some(class),
            });
        }
        let rate = buffer.failure_rate(FailureClass::Timeout);
        assert!((rate - 2.0 / 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn observer_appends_two_phase_audit() {
        let observer = LocalObserver::new();
        for phase in [AuditPhase::Intent, AuditPhase::Result] {
            observer
                .append_audit(LlmAuditRecord {
                    request_id: TaskId::new("req-audit").expect("id"),
                    phase,
                    task: LlmTask::Summary,
                    chars: 10,
                    model: "test".into(),
                    endpoint: "example.invalid".into(),
                    elapsed_ms: 1,
                    outcome: "ok".into(),
                    degraded: false,
                    attempts: 1,
                })
                .expect("审计");
        }
        let records = observer.audit_records();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].summary, records[0].summary);
        assert!(records[0].summary.contains("phase=intent"));
        assert!(records[1].summary.contains("phase=result"));
    }
}
