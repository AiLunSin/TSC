//! 网关编排（DESIGN 2.1.3-（1）（2））。
//!
//! 唯一编排点：校验 → 审计意图 → 冷却 → 并发准入 → 传输/分类/重试 → 解码 → 兜底。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::llm::{
    AuditPhase, CompletionHint, ConclusionNotice, DegradeCause, FailureClass, LlmAuditRecord,
    LlmGenerationRequest, LlmLogRecord, LlmMetric, LlmMetricPoint, LlmTask, NormalizationEntry,
    NormalizationSource, PendingReason, StreamEvent, StreamEventKind, StructuredBody,
    StructuredOutcome, SummaryPurpose,
};
use meeting_copilot_domain::port::llm::{
    CancelToken, LlmCapability, LlmObserver, StreamOutcomeSummary, StructuredLlmGateway,
};
use serde_json::{json, Value};

use crate::llm::degrade::{
    classify, classify_http, classify_status, decide_retry, parse_retry_after, Clock,
    ConcurrencyGate, EndpointCircuit, RetryDecision,
};
use crate::llm::fallback::{FallbackInput, RuleFallback};
use crate::llm::schema::SchemaRegistry;
use crate::llm::transport::{HttpRequest, HttpTransport};
use crate::llm::wire::decode_structured;
use crate::llm::LlmConfig;

/// 思考模式字段名（唯一来源）。
pub const THINKING_FIELD: &str = "thinking";
/// 思考模式必须固定的取值。
pub const THINKING_DISABLED_VALUE: &str = "disabled";
/// 唯一标注标识；不进入 `ErrorCode`。
pub const THINKING_NOT_DISABLED: &str = "thinking_not_disabled";

/// 请求体单点构造：mock 与真实共用；不接受运行期 thinking 参数。
pub fn request_body(model: &str, request: &LlmGenerationRequest, stream: bool) -> Value {
    let schema = SchemaRegistry::json_schema(request.task).to_string();
    let mut body = json!({
        "model": model,
        "temperature": f64::from(request.temperature.value()) / 1000.0,
        "max_tokens": request.output.value(),
        "stream": stream,
        "messages": [
            {"role": "system", "content": format!("task={} schema={}。只输出符合以下 JSON Schema 的 JSON：{schema}", request.task.as_str(), request.task.schema_version())},
            {"role": "user", "content": request.payload.expose()}
        ],
        "response_format": {"type": "json_object"}
    });
    body[THINKING_FIELD] = json!({"type": THINKING_DISABLED_VALUE});
    body
}

/// 缺失、空对象或非 disabled 均返回固定标识。
pub fn thinking_verdict(body: &Value) -> Option<&'static str> {
    let disabled = body
        .get(THINKING_FIELD)
        .and_then(Value::as_object)
        .and_then(|thinking| thinking.get("type"))
        .and_then(Value::as_str)
        == Some(THINKING_DISABLED_VALUE);
    if disabled {
        None
    } else {
        Some(THINKING_NOT_DISABLED)
    }
}

pub const JSON_INSTRUCTION_MISSING: &str = "json_instruction_missing";

/// 检查全部 system message 是否显式包含 JSON 指令（大小写不敏感）。
pub fn json_instruction_verdict(body: &Value) -> Option<&'static str> {
    let has_json = body
        .get("messages")
        .and_then(Value::as_array)
        .map(|messages| {
            messages.iter().any(|message| {
                message.get("role").and_then(Value::as_str) == Some("system")
                    && message
                        .get("content")
                        .and_then(Value::as_str)
                        .is_some_and(|content| content.to_ascii_lowercase().contains("json"))
            })
        })
        .unwrap_or(false);
    if has_json {
        None
    } else {
        Some(JSON_INSTRUCTION_MISSING)
    }
}

pub struct StructuredGateway {
    config: Arc<RwLock<LlmConfig>>,
    transport: Arc<dyn HttpTransport>,
    observer: Arc<dyn LlmObserver>,
    circuit: Mutex<EndpointCircuit>,
    gate: ConcurrencyGate,
    clock: Arc<dyn Clock>,
    audit_blocked: AtomicBool,
}

impl StructuredGateway {
    pub fn new(
        config: Arc<RwLock<LlmConfig>>,
        transport: Arc<dyn HttpTransport>,
        observer: Arc<dyn LlmObserver>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        let (threshold, cooling, limit) = {
            let snapshot = config.read().unwrap_or_else(|poison| poison.into_inner());
            (
                snapshot.cooling_threshold,
                Duration::from_millis(snapshot.cooling_ms),
                snapshot.max_concurrency,
            )
        };
        Self {
            config,
            transport,
            observer,
            circuit: Mutex::new(EndpointCircuit::new(threshold, cooling)),
            gate: ConcurrencyGate::new(limit),
            clock,
            audit_blocked: AtomicBool::new(false),
        }
    }

    fn snapshot(&self) -> LlmConfig {
        self.config
            .read()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }

    fn deadline(&self, config: &LlmConfig, request: &LlmGenerationRequest) -> Instant {
        let budget = request
            .deadline
            .as_millis()
            .min(config.request_timeout_ms)
            .max(1);
        self.clock.now() + Duration::from_millis(budget)
    }

    fn build_http(
        &self,
        config: &LlmConfig,
        request: &LlmGenerationRequest,
        stream: bool,
    ) -> Result<HttpRequest, CopilotError> {
        let (secure, host, port, path, _) =
            config
                .endpoint_parts()
                .ok_or_else(|| CopilotError::ProviderConfigInvalid {
                    detail: "endpoint: 必须为 http(s) 绝对地址".into(),
                })?;
        let body = request_body(&config.model, request, stream);
        if let Some(code) = json_instruction_verdict(&body) {
            return Err(CopilotError::ProviderConfigInvalid {
                detail: format!("request: {code}"),
            });
        }
        let bytes = serde_json::to_vec(&body).map_err(|_| CopilotError::ProviderConfigInvalid {
            detail: "request: 序列化失败".into(),
        })?;
        if bytes.len() > config.max_request_bytes {
            return Err(CopilotError::ProviderConfigInvalid {
                detail: "request: payload_too_large".into(),
            });
        }
        let mut http = HttpRequest::json_post(
            host,
            port,
            path,
            vec![
                ("Content-Type".into(), "application/json".into()),
                ("Accept".into(), "application/json".into()),
            ],
            bytes,
        );
        http.secure = secure;
        Ok(http)
    }

    #[allow(clippy::too_many_arguments)]
    fn audit(
        &self,
        config: &LlmConfig,
        request: &LlmGenerationRequest,
        phase: AuditPhase,
        outcome: &str,
        degraded: bool,
        attempts: u8,
        elapsed_ms: u64,
    ) -> Result<(), CopilotError> {
        let record = LlmAuditRecord {
            request_id: request.request_id.clone(),
            phase,
            task: request.task,
            chars: request.payload.char_count(),
            model: config.model.clone(),
            endpoint: config
                .endpoint_parts()
                .map(|(_, host, _, _, _)| host)
                .unwrap_or_else(|| "unknown".into()),
            elapsed_ms,
            outcome: outcome.to_owned(),
            degraded,
            attempts,
        };
        self.observer.append_audit(record)
    }

    fn record_result(
        &self,
        request: &LlmGenerationRequest,
        outcome: &StructuredOutcome,
        elapsed_ms: u64,
    ) {
        let _ = self.observer.record_metric(LlmMetricPoint {
            metric: LlmMetric::TotalLatencyMs,
            value: elapsed_ms as f64,
            task: Some(request.task),
            failure_class: None,
        });
        if outcome.degraded {
            let _ = self.observer.record_metric(LlmMetricPoint {
                metric: LlmMetric::DegradeCount,
                value: 1.0,
                task: Some(request.task),
                failure_class: outcome.failure,
            });
        }
        if let Some(class) = outcome.failure {
            let _ = self.observer.record_metric(LlmMetricPoint {
                metric: LlmMetric::FailureCount,
                value: 1.0,
                task: Some(request.task),
                failure_class: Some(class),
            });
            if let Ok(record) = LlmLogRecord::new("failure_class", class.as_str()) {
                self.observer.log(record);
            }
        }
        if let Ok(record) = LlmLogRecord::new("latency_ms", elapsed_ms.to_string()) {
            self.observer.log(record);
        }
    }

    fn fallback_input(request: &LlmGenerationRequest) -> FallbackInput {
        FallbackInput {
            sources: Vec::new(),
            window_text: Some(request.payload.expose().to_owned()),
            last_summary: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn degraded_outcome(
        &self,
        config: &LlmConfig,
        request: &LlmGenerationRequest,
        cause: DegradeCause,
        failure: Option<FailureClass>,
        attempts: u8,
        elapsed_ms: u64,
        http_status: Option<u16>,
    ) -> StructuredOutcome {
        let body = RuleFallback::produce(request.task, &Self::fallback_input(request));
        let (normalized_body, normalization) = normalize_body(request, body);
        let outcome = StructuredOutcome {
            request_id: request.request_id.clone(),
            task: request.task,
            body: Some(normalized_body),
            model: config.model.clone(),
            tokens_used: None,
            degraded: true,
            degrade_cause: Some(cause),
            failure,
            attempts,
            truncated: false,
            schema_version: request.task.schema_version(),
            http_status,
            normalization,
        };
        let outcome_text = if failure.is_some() {
            "degraded"
        } else {
            "fallback"
        };
        let _ = self.audit(
            config,
            request,
            AuditPhase::Result,
            outcome_text,
            true,
            attempts,
            elapsed_ms,
        );
        self.record_result(request, &outcome, elapsed_ms);
        outcome
    }
}

impl StructuredLlmGateway for StructuredGateway {
    fn complete_structured(
        &self,
        request: &LlmGenerationRequest,
    ) -> Result<StructuredOutcome, CopilotError> {
        let started = self.clock.now();
        let config = self.snapshot();
        config.validate()?;

        if request.payload.char_count() > config.payload_char_limit {
            return Ok(self.degraded_outcome(
                &config,
                request,
                DegradeCause::PayloadTooLarge,
                None,
                0,
                elapsed(started, self.clock.now()),
                None,
            ));
        }

        if self.audit_blocked.load(Ordering::SeqCst) {
            return Ok(self.degraded_outcome(
                &config,
                request,
                DegradeCause::AuditUnavailable,
                None,
                0,
                elapsed(started, self.clock.now()),
                None,
            ));
        }
        if let Err(error) = self.audit(&config, request, AuditPhase::Intent, "intent", false, 0, 0)
        {
            if matches!(error, CopilotError::StoreFailure { .. }) {
                self.audit_blocked.store(true, Ordering::SeqCst);
                return Ok(self.degraded_outcome(
                    &config,
                    request,
                    DegradeCause::AuditUnavailable,
                    None,
                    0,
                    elapsed(started, self.clock.now()),
                    None,
                ));
            }
            return Err(error);
        }

        let now = self.clock.now();
        let cooling = {
            let mut circuit = self
                .circuit
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            !circuit.is_closed(now)
        };
        if cooling {
            return Ok(self.degraded_outcome(
                &config,
                request,
                DegradeCause::Cooling,
                None,
                0,
                elapsed(started, self.clock.now()),
                None,
            ));
        }

        let deadline = self.deadline(&config, request);
        let permit = match self.gate.acquire(deadline, &CancelToken::new()) {
            Ok(permit) => permit,
            Err(cause) => {
                return Ok(self.degraded_outcome(
                    &config,
                    request,
                    cause,
                    None,
                    0,
                    elapsed(started, self.clock.now()),
                    None,
                ));
            }
        };

        let mut attempt = 1_u8;
        let mut last_failure: Option<FailureClass>;
        let mut last_http_status: Option<u16> = None;
        loop {
            if self.clock.now() >= deadline {
                last_failure = Some(FailureClass::Timeout);
                break;
            }
            let http = self.build_http(&config, request, false)?;
            match self.transport.send(&http, deadline, &CancelToken::new()) {
                Err(error) => {
                    if error == crate::llm::transport::TransportError::BudgetExhausted {
                        drop(permit);
                        return Ok(self.degraded_outcome(
                            &config,
                            request,
                            DegradeCause::BudgetExhausted,
                            None,
                            attempt,
                            elapsed(started, self.clock.now()),
                            None,
                        ));
                    }
                    let Some(class) = classify(&error) else {
                        last_failure = Some(FailureClass::Timeout);
                        break;
                    };
                    last_failure = Some(class);
                    if !self.maybe_retry(class, attempt, deadline, None, request, &config) {
                        break;
                    }
                    attempt = attempt.saturating_add(1);
                }
                Ok(response) => {
                    last_http_status = Some(response.status);
                    let response_body = response.body_text();
                    if let Some(class) = classify_http(response.status, &response_body) {
                        last_failure = Some(class);
                        if (401..=403).contains(&response.status) {
                            break;
                        }
                        let retry_after = parse_retry_after(response.header("retry-after"));
                        if !self.maybe_retry(
                            class,
                            attempt,
                            deadline,
                            retry_after,
                            request,
                            &config,
                        ) {
                            break;
                        }
                        attempt = attempt.saturating_add(1);
                        continue;
                    }
                    let payload = extract_content(&response_body);
                    match decode_structured(request.task, &payload) {
                        Ok(body) => {
                            let model = extract_model(&response.body_text())
                                .unwrap_or_else(|| config.model.clone());
                            let (normalized_body, normalization) = normalize_body(request, body);
                            let outcome = StructuredOutcome {
                                request_id: request.request_id.clone(),
                                task: request.task,
                                body: Some(normalized_body),
                                model,
                                tokens_used: None,
                                degraded: false,
                                degrade_cause: None,
                                failure: None,
                                attempts: attempt,
                                truncated: false,
                                schema_version: request.task.schema_version(),
                                http_status: Some(response.status),
                                normalization,
                            };
                            {
                                let mut circuit = self
                                    .circuit
                                    .lock()
                                    .unwrap_or_else(|poison| poison.into_inner());
                                circuit.on_success();
                            }
                            let elapsed_ms = elapsed(started, self.clock.now());
                            let _ = self.audit(
                                &config,
                                request,
                                AuditPhase::Result,
                                "ok",
                                false,
                                attempt,
                                elapsed_ms,
                            );
                            self.record_result(request, &outcome, elapsed_ms);
                            drop(permit);
                            return Ok(outcome);
                        }
                        Err(_schema_error) => {
                            last_failure = Some(FailureClass::InvalidSchema);
                            if !self.maybe_retry(
                                FailureClass::InvalidSchema,
                                attempt,
                                deadline,
                                None,
                                request,
                                &config,
                            ) {
                                break;
                            }
                            attempt = attempt.saturating_add(1);
                        }
                    }
                }
            }
        }

        {
            let mut circuit = self
                .circuit
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            circuit.on_final_failure(self.clock.now());
        }
        drop(permit);
        let failure = last_failure.unwrap_or(FailureClass::Unavailable);
        Ok(self.degraded_outcome(
            &config,
            request,
            DegradeCause::Failed(failure),
            Some(failure),
            attempt,
            elapsed(started, self.clock.now()),
            last_http_status,
        ))
    }

    fn stream_structured(
        &self,
        request: &LlmGenerationRequest,
        cancel: &CancelToken,
        sink: &mut (dyn FnMut(StreamEvent) + Send),
    ) -> Result<StreamOutcomeSummary, CopilotError> {
        let started = self.clock.now();
        let config = self.snapshot();
        config.validate()?;
        let deadline = self.deadline(&config, request);
        let http = self.build_http(&config, request, true)?;

        let mut stream = match self.transport.open_stream(&http, deadline, cancel) {
            Ok(stream) => stream,
            Err(error) => {
                let class = classify(&error).unwrap_or(FailureClass::Unavailable);
                let cause = DegradeCause::Failed(class);
                sink(StreamEvent {
                    sequence: 1,
                    kind: StreamEventKind::Failed {
                        failure: class,
                        degraded: true,
                        cause,
                    },
                });
                self.record_failure(request, class);
                return Ok(StreamOutcomeSummary {
                    request_id: request.request_id.clone(),
                    terminated: true,
                    cancelled: error == crate::llm::transport::TransportError::Cancelled,
                    discarded_deltas: 0,
                    first_token_ms: None,
                });
            }
        };

        let mut sequence = 0_u32;
        let mut discarded = 0_u32;
        let mut collected = String::new();
        let mut buffer = String::new();
        let mut first_token_ms: Option<u32> = None;
        let mut last_delta = String::new();
        let mut failed = false;
        let mut done = false;
        let mut saw_sse = false;

        loop {
            if cancel.is_cancelled() {
                break;
            }
            match stream.next_chunk(deadline, cancel) {
                Ok(None) => break,
                Ok(Some(chunk)) => {
                    let text = String::from_utf8_lossy(&chunk).into_owned();
                    if text.starts_with("HTTPSTATUS:") {
                        let status = text
                            .lines()
                            .next()
                            .and_then(|line| line.strip_prefix("HTTPSTATUS:"))
                            .and_then(|value| value.trim().parse::<u16>().ok())
                            .unwrap_or(500);
                        let class = classify_status(status).unwrap_or(FailureClass::Unavailable);
                        let cause = DegradeCause::Failed(class);
                        sequence = sequence.saturating_add(1);
                        sink(StreamEvent {
                            sequence,
                            kind: StreamEventKind::Failed {
                                failure: class,
                                degraded: true,
                                cause,
                            },
                        });
                        failed = true;
                        self.record_failure(request, class);
                        break;
                    }
                    buffer.push_str(&text);
                    while let Some(index) = buffer.find('\n') {
                        let line = buffer[..index].trim_end_matches('\r').to_owned();
                        buffer = buffer[index + 1..].to_owned();
                        let Some(data) = line.strip_prefix("data:") else {
                            continue;
                        };
                        saw_sse = true;
                        let data = data.trim();
                        if data == "[DONE]" {
                            done = true;
                            break;
                        }
                        if let Ok(value) = serde_json::from_str::<Value>(data) {
                            if let Some(content) = value
                                .pointer("/choices/0/delta/content")
                                .and_then(Value::as_str)
                            {
                                if content.is_empty() || content == last_delta {
                                    discarded = discarded.saturating_add(1);
                                    continue;
                                }
                                last_delta = content.to_owned();
                                collected.push_str(content);
                                sequence = sequence.saturating_add(1);
                                let first_token = first_token_ms.is_none();
                                let elapsed_ms = elapsed(started, self.clock.now()) as u32;
                                if first_token {
                                    first_token_ms = Some(elapsed_ms);
                                    let _ = self.observer.record_metric(LlmMetricPoint {
                                        metric: LlmMetric::FirstTokenLatencyMs,
                                        value: f64::from(elapsed_ms),
                                        task: Some(request.task),
                                        failure_class: None,
                                    });
                                }
                                sink(StreamEvent {
                                    sequence,
                                    kind: StreamEventKind::Delta {
                                        text: content.to_owned(),
                                        first_token,
                                    },
                                });
                            }
                        }
                    }
                    if done {
                        break;
                    }
                    if !saw_sse {
                        collected.push_str(&text);
                    }
                }
                Err(error) => {
                    let class = classify(&error).unwrap_or(FailureClass::Timeout);
                    let cause = DegradeCause::Failed(class);
                    sequence = sequence.saturating_add(1);
                    sink(StreamEvent {
                        sequence,
                        kind: StreamEventKind::Failed {
                            failure: class,
                            degraded: true,
                            cause,
                        },
                    });
                    failed = true;
                    self.record_failure(request, class);
                    break;
                }
            }
        }

        // 部分服务端在最后一条 `data:` 后不发换行；把它补处理，避免丢掉 JSON 结尾。
        if !failed && !done {
            let tail = buffer.trim();
            if let Some(data) = tail.strip_prefix("data:") {
                let data = data.trim();
                if data != "[DONE]" {
                    if let Ok(value) = serde_json::from_str::<Value>(data) {
                        if let Some(content) = value
                            .pointer("/choices/0/delta/content")
                            .and_then(Value::as_str)
                        {
                            collected.push_str(content);
                        }
                    }
                }
            }
        }

        if !failed {
            match decode_structured(request.task, &collected) {
                Ok(body) => {
                    let (normalized_body, normalization) = normalize_body(request, body);
                    let outcome = StructuredOutcome {
                        request_id: request.request_id.clone(),
                        task: request.task,
                        body: Some(normalized_body),
                        model: config.model.clone(),
                        tokens_used: None,
                        degraded: false,
                        degrade_cause: None,
                        failure: None,
                        attempts: 1,
                        truncated: false,
                        schema_version: request.task.schema_version(),
                        http_status: None,
                        normalization,
                    };
                    sequence = sequence.saturating_add(1);
                    sink(StreamEvent {
                        sequence,
                        kind: StreamEventKind::Completed(Box::new(outcome)),
                    });
                }
                Err(_) => {
                    let class = FailureClass::InvalidSchema;
                    let cause = DegradeCause::Failed(class);
                    sequence = sequence.saturating_add(1);
                    sink(StreamEvent {
                        sequence,
                        kind: StreamEventKind::Failed {
                            failure: class,
                            degraded: true,
                            cause,
                        },
                    });
                    self.record_failure(request, class);
                }
            }
        }

        let elapsed_ms = elapsed(started, self.clock.now());
        let _ = self.audit(
            &config,
            request,
            AuditPhase::Result,
            if failed { "failed" } else { "completed" },
            failed,
            1,
            elapsed_ms,
        );
        Ok(StreamOutcomeSummary {
            request_id: request.request_id.clone(),
            terminated: true,
            cancelled: cancel.is_cancelled(),
            discarded_deltas: discarded,
            first_token_ms,
        })
    }

    fn capability(&self) -> LlmCapability {
        let config = self.snapshot();
        let endpoint_configured = config.endpoint_parts().is_some();
        let model_configured = !config.model.trim().is_empty();
        let cooling = {
            let mut circuit = self
                .circuit
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            !circuit.is_closed(self.clock.now())
        };
        LlmCapability {
            task: LlmTask::Summary,
            endpoint_configured,
            model_configured,
            cooling,
        }
    }
}

impl StructuredGateway {
    fn maybe_retry(
        &self,
        class: FailureClass,
        attempt: u8,
        deadline: Instant,
        retry_after: Option<Duration>,
        request: &LlmGenerationRequest,
        _config: &LlmConfig,
    ) -> bool {
        let remaining = deadline.saturating_duration_since(self.clock.now());
        match decide_retry(
            class,
            attempt,
            remaining,
            retry_after,
            request.request_id.as_str(),
        ) {
            RetryDecision::Retry { delay } => {
                let _ = self.observer.record_metric(LlmMetricPoint {
                    metric: LlmMetric::RetryCount,
                    value: 1.0,
                    task: Some(request.task),
                    failure_class: Some(class),
                });
                self.clock.sleep(delay);
                true
            }
            RetryDecision::Degrade { .. } => false,
        }
    }

    fn record_failure(&self, request: &LlmGenerationRequest, class: FailureClass) {
        let _ = self.observer.record_metric(LlmMetricPoint {
            metric: LlmMetric::FailureCount,
            value: 1.0,
            task: Some(request.task),
            failure_class: Some(class),
        });
        if let Ok(record) = LlmLogRecord::new("failure_class", class.as_str()) {
            self.observer.log(record);
        }
    }
}

fn elapsed(started: Instant, now: Instant) -> u64 {
    u64::try_from(now.saturating_duration_since(started).as_millis()).unwrap_or(u64::MAX)
}

fn sanitize_json_text(raw: &str) -> String {
    let trimmed = raw.trim();
    let stripped = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed)
        .trim();
    let stripped = stripped.strip_suffix("```").unwrap_or(stripped).trim();
    if let (Some(start), Some(end)) = (stripped.find('{'), stripped.rfind('}')) {
        if start <= end {
            return stripped[start..=end].to_owned();
        }
    }
    stripped.to_owned()
}

fn extract_content(raw: &str) -> String {
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        if let Some(content) = value
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
        {
            return sanitize_json_text(content);
        }
        if let Some(content) = value.pointer("/choices/0/text").and_then(Value::as_str) {
            return sanitize_json_text(content);
        }
        if value.get("insights").is_some()
            || value.get("open_questions").is_some()
            || value.get("references").is_some()
            || (value.get("purpose").is_some() && value.get("text").is_some())
        {
            return sanitize_json_text(raw);
        }
    }
    sanitize_json_text(raw)
}

fn extract_model(raw: &str) -> Option<String> {
    serde_json::from_str::<Value>(raw)
        .ok()?
        .get("model")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

/// 服务端归一：模型只负责内容字段，元数据由网关按请求与上下文填充。
fn normalize_body(
    request: &LlmGenerationRequest,
    body: StructuredBody,
) -> (StructuredBody, Vec<NormalizationEntry>) {
    let mut trace = Vec::new();
    let entry =
        |field: &'static str, source: NormalizationSource| NormalizationEntry { field, source };
    let normalized = match body {
        StructuredBody::Summary(mut summary) => {
            summary.purpose = request.purpose.unwrap_or(SummaryPurpose::RollingWindow);
            summary.version = 1;
            summary.carried_over = false;
            summary.produced = true;
            trace.push(entry("purpose", NormalizationSource::Request));
            trace.push(entry("version", NormalizationSource::Computed));
            trace.push(entry("carried_over", NormalizationSource::Computed));
            trace.push(entry("produced", NormalizationSource::Computed));
            StructuredBody::Summary(summary)
        }
        StructuredBody::Conclusion(mut conclusion) => {
            if conclusion.notice.is_none() {
                conclusion.notice = Some(if conclusion.conclusion.is_none() {
                    ConclusionNotice::GenerationUnavailable
                } else {
                    ConclusionNotice::MissingSource
                });
                trace.push(entry("notice", NormalizationSource::Computed));
            }
            StructuredBody::Conclusion(conclusion)
        }
        StructuredBody::Minutes(mut minutes) => {
            minutes.unrefined_count = minutes
                .open_questions
                .iter()
                .filter(|item| item.reason == PendingReason::Unrefined)
                .count() as u32;
            minutes.completion_hint =
                (minutes.unrefined_count > 0).then_some(CompletionHint::HumanReview);
            trace.push(entry("unrefined_count", NormalizationSource::Computed));
            trace.push(entry("completion_hint", NormalizationSource::Computed));
            StructuredBody::Minutes(minutes)
        }
        other => other,
    };
    (normalized, trace)
}

/// 暴露给组件的类型别名，便于装配处书写。
pub type BoxedGateway = Arc<dyn StructuredLlmGateway>;

#[allow(dead_code)]
fn assert_body_type(body: &StructuredBody, task: LlmTask) -> bool {
    body.task() == task
}
#[cfg(test)]
mod tests {
    use super::*;
    use meeting_copilot_domain::model::llm::{
        DeadlineMs, OutputBudget, RedactedPayload, SummaryPurpose, TemperatureMilli,
    };
    use meeting_copilot_domain::vo::TaskId;

    fn request() -> LlmGenerationRequest {
        LlmGenerationRequest::new(
            TaskId::new("req-thinking").expect("id"),
            RedactedPayload::new("上下文", LlmTask::Summary, 8_000).expect("payload"),
            Some(SummaryPurpose::RollingWindow),
            OutputBudget::new(512).expect("output"),
            TemperatureMilli::new(200).expect("temperature"),
            DeadlineMs::new(8_000).expect("deadline"),
        )
        .expect("request")
    }

    #[test]
    fn request_body_thinking_field_is_fixed() {
        for stream in [false, true] {
            let body = request_body("test-model", &request(), stream);
            let system = body["messages"][0]["content"].as_str().expect("system");
            assert!(system.contains("\"properties\""));
            assert!(system.contains("\"required\""));
            assert_eq!(body[THINKING_FIELD]["type"], THINKING_DISABLED_VALUE);
            assert!(thinking_verdict(&body).is_none());

            let mut missing = body.clone();
            missing
                .as_object_mut()
                .expect("对象")
                .remove(THINKING_FIELD);
            assert_eq!(thinking_verdict(&missing), Some(THINKING_NOT_DISABLED));

            let mut wrong = body.clone();
            wrong[THINKING_FIELD]["type"] = json!("enabled");
            assert_eq!(thinking_verdict(&wrong), Some(THINKING_NOT_DISABLED));

            let mut empty = body.clone();
            empty[THINKING_FIELD] = json!({});
            assert_eq!(thinking_verdict(&empty), Some(THINKING_NOT_DISABLED));
        }
    }
}
