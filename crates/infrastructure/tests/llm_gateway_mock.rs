mod support;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use meeting_copilot_domain::model::llm::{LlmTask, StreamEvent, StreamEventKind, StructuredBody};
use meeting_copilot_domain::port::llm::CancelToken;
use meeting_copilot_infrastructure::llm::degrade::FakeClock;
use meeting_copilot_infrastructure::llm::transport::{LoopbackTransport, TransportError};
use meeting_copilot_infrastructure::llm::LocalObserver;
use support::Step;

fn insight_request() -> meeting_copilot_domain::model::llm::LlmGenerationRequest {
    support::request(LlmTask::InsightExtract, "确认下周灰度上线", 6_000)
}

#[test]
fn complete_ok() {
    let transport = support::FakeTransport::new(vec![Step::Json(support::envelope(
        &support::insight_body(),
    ))]);
    let (gateway, _observer) = support::gateway(support::test_config(), transport.clone());
    let outcome = gateway
        .complete_structured(&insight_request())
        .expect("调用成功");
    assert!(!outcome.degraded);
    assert_eq!(outcome.attempts, 1);
    assert_eq!(
        outcome.schema_version,
        LlmTask::InsightExtract.schema_version()
    );
    assert!(matches!(outcome.body, Some(StructuredBody::Insight(_))));
    assert_eq!(transport.requests(), 1);
}

#[test]
fn stream_ok() {
    let item = "{\"kind\":\"key_info\",\"text\":\"确认下周灰度上线\",\"sources\":[\"utt-1\"],\"confidence\":0.8}";
    let chunks = vec![
        "data: {\"choices\":[{\"delta\":{\"content\":\"{\\\"insights\\\":[\"}}]}\n\n".to_owned(),
        format!(
            "data: {}\n\n",
            serde_json::json!({"choices": [{"delta": {"content": item}}]})
        ),
        "data: {\"choices\":[{\"delta\":{\"content\":\"]}\"}}]}\n\n".to_owned(),
        "data: [DONE]\n\n".to_owned(),
    ];
    let transport = support::FakeTransport::new(vec![Step::Sse(chunks)]);
    let (gateway, _observer) = support::gateway(support::test_config(), transport);
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = events.clone();
    let mut sink = move |event: StreamEvent| {
        captured.lock().expect("锁").push(event);
    };
    let summary = gateway
        .stream_structured(&insight_request(), &CancelToken::new(), &mut sink)
        .expect("流式调用");
    assert!(summary.terminated);
    assert!(summary.first_token_ms.is_some());
    let events = events.lock().expect("锁").clone();
    let sequences: Vec<u32> = events.iter().map(|event| event.sequence).collect();
    assert_eq!(sequences, vec![1, 2, 3, 4]);
    assert!(matches!(
        events.last().map(|event| &event.kind),
        Some(StreamEventKind::Completed(_))
    ));
}

#[test]
fn non_blocking_capture() {
    let transport = support::FakeTransport::new(vec![Step::DelayThen(
        Duration::from_secs(1),
        Box::new(Step::Json(support::envelope(&support::insight_body()))),
    )]);
    let (gateway, _observer) = support::gateway(support::test_config(), transport);
    let request = insight_request();
    let worker = std::thread::spawn(move || gateway.complete_structured(&request));

    let mut samples = Vec::new();
    for _ in 0..100 {
        let started = Instant::now();
        std::thread::sleep(Duration::from_millis(1));
        samples.push(started.elapsed());
    }
    samples.sort();
    let p95 = samples[(samples.len() * 95 / 100).saturating_sub(1)];
    assert!(p95 <= Duration::from_millis(200), "p95={p95:?}");
    let outcome = worker.join().expect("线程").expect("结果");
    assert!(!outcome.degraded);
}

#[test]
fn loopback_endpoint_roundtrip() {
    let server = support::MockServer::start(support::envelope(&support::insight_body()));
    let mut config = support::test_config();
    config.endpoint = format!("http://127.0.0.1:{}/v1/chat/completions", server.port());
    let transport = Arc::new(LoopbackTransport::new(
        config.domain_allowlist.clone(),
        config.max_response_bytes,
    ));
    let (gateway, _observer) = support::gateway(config, transport);
    let outcome = gateway
        .complete_structured(&insight_request())
        .expect("回环调用");
    assert!(!outcome.degraded, "failure={:?}", outcome.failure);
    assert_eq!(server.requests(), 1);
}

#[test]
fn transport_failure_is_classified_without_panic() {
    let transport = support::FakeTransport::new(vec![
        Step::Fail(TransportError::NetworkUnreachable),
        Step::Fail(TransportError::NetworkUnreachable),
        Step::Fail(TransportError::NetworkUnreachable),
    ]);
    let (gateway, _observer) = support::gateway(support::test_config(), transport);
    let outcome = gateway
        .complete_structured(&insight_request())
        .expect("降级结果");
    assert!(outcome.degraded);
    assert_eq!(
        outcome.failure,
        Some(meeting_copilot_domain::model::llm::FailureClass::Offline)
    );
    assert!(outcome.body.is_some());
    let _ = LocalObserver::new();
    let _ = FakeClock::new();
}

#[test]
fn request_body_field_set() {
    use meeting_copilot_domain::model::llm::{
        DeadlineMs, LlmGenerationRequest, OutputBudget, RedactedPayload, SummaryPurpose,
        TemperatureMilli,
    };
    use meeting_copilot_domain::vo::TaskId;
    use meeting_copilot_infrastructure::llm::gateway::{request_body, THINKING_FIELD};

    let request = LlmGenerationRequest::new(
        TaskId::new("req-body").expect("id"),
        RedactedPayload::new("上下文", LlmTask::Summary, 8_000).expect("payload"),
        Some(SummaryPurpose::RollingWindow),
        OutputBudget::new(512).expect("output"),
        TemperatureMilli::new(200).expect("temperature"),
        DeadlineMs::new(8_000).expect("deadline"),
    )
    .expect("request");
    let body = request_body("test-model", &request, false);
    for field in [
        "model",
        "temperature",
        "max_tokens",
        "stream",
        "messages",
        "response_format",
    ] {
        assert!(body.get(field).is_some(), "缺少字段 {field}");
    }
    assert_eq!(body[THINKING_FIELD]["type"], "disabled");
}

#[test]
fn request_body_byte_budget_rejects_without_send() {
    let mut config = support::test_config();
    config.max_request_bytes = 1024;
    let transport = support::FakeTransport::new(vec![]);
    let (gateway, _observer) = support::gateway(config, transport.clone());
    let request = support::request(LlmTask::InsightExtract, &"字".repeat(8_000), 6_000);
    let error = gateway.complete_structured(&request).expect_err("必须拒绝");
    assert!(format!("{error}").contains("payload_too_large"));
    assert_eq!(transport.requests(), 0);
}
