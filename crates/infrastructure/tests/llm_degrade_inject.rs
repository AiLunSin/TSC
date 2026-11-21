mod support;

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use meeting_copilot_domain::model::llm::{
    DegradeCause, FailureClass, LlmTask, StructuredBody, Summary, SummaryPurpose,
};
use meeting_copilot_domain::vo::{UtteranceRef, UtteranceText};
use meeting_copilot_infrastructure::llm::degrade::FakeClock;
use meeting_copilot_infrastructure::llm::fallback::{FallbackInput, RuleFallback};
use meeting_copilot_infrastructure::llm::transport::TransportError;
use support::Step;

fn steps(repeat: usize, step: Step) -> Vec<Step> {
    (0..repeat).map(|_| step.clone()).collect()
}

fn run(
    task: LlmTask,
    steps: Vec<Step>,
    deadline_ms: u64,
) -> (
    meeting_copilot_domain::model::llm::StructuredOutcome,
    Arc<support::FakeTransport>,
) {
    let transport = support::FakeTransport::new(steps);
    let (gateway, _observer) = support::gateway(support::test_config(), transport.clone());
    let request = support::request(task, "会议上下文", deadline_ms);
    let outcome = gateway.complete_structured(&request).expect("调用");
    (outcome, transport)
}

fn run_with_clock(
    task: LlmTask,
    steps: Vec<Step>,
    deadline_ms: u64,
) -> (
    meeting_copilot_domain::model::llm::StructuredOutcome,
    Arc<support::FakeTransport>,
    Duration,
) {
    use meeting_copilot_infrastructure::llm::degrade::Clock;
    let transport = support::FakeTransport::new(steps);
    let clock = Arc::new(FakeClock::new());
    let before = clock.now();
    let (gateway, _observer) = support::gateway_with_config(
        Arc::new(RwLock::new(support::test_config())),
        transport.clone(),
        clock.clone(),
    );
    let request = support::request(task, "会议上下文", deadline_ms);
    let outcome = gateway.complete_structured(&request).expect("调用");
    let after = clock.now();
    (outcome, transport, after.duration_since(before))
}

#[test]
fn timeout_deadline() {
    let started = Instant::now();
    let (outcome, transport) = run(
        LlmTask::InsightExtract,
        steps(3, Step::Delay(Duration::from_secs(3))),
        500,
    );
    assert!(started.elapsed() <= Duration::from_millis(700));
    assert_eq!(outcome.failure, Some(FailureClass::Timeout));
    assert!(outcome.degraded);
    assert_eq!(transport.requests(), 3);
}

#[test]
fn retry_cap() {
    let (outcome, transport) = run(
        LlmTask::InsightExtract,
        steps(3, Step::Fail(TransportError::ReadTimeout)),
        6_000,
    );
    assert_eq!(outcome.attempts, 3);
    assert!(outcome.degraded);
    assert_eq!(transport.requests(), 3);
}

#[test]
fn rate_limit_backoff() {
    let (outcome, transport, waited) = run_with_clock(
        LlmTask::InsightExtract,
        vec![
            Step::Status(429, vec![("Retry-After".into(), "1".into())], String::new()),
            Step::Json(support::envelope(&support::insight_body())),
        ],
        6_000,
    );
    assert!(!outcome.degraded);
    assert_eq!(outcome.attempts, 2);
    assert_eq!(transport.requests(), 2);
    assert_eq!(waited, Duration::from_millis(1_000));
}

#[test]
fn rate_limit_backoff_jitter_without_retry_after() {
    let (outcome, transport, waited) = run_with_clock(
        LlmTask::InsightExtract,
        vec![
            Step::Status(429, vec![], String::new()),
            Step::Json(support::envelope(&support::insight_body())),
        ],
        6_000,
    );
    assert!(!outcome.degraded);
    assert_eq!(outcome.attempts, 2);
    assert_eq!(transport.requests(), 2);
    assert!(waited > Duration::ZERO, "等待必须为正: {waited:?}");
    assert!(
        waited >= Duration::from_millis(640),
        "抖动下界 640ms 未达成: {waited:?}"
    );
    assert!(
        waited <= Duration::from_millis(960),
        "抖动上界 960ms 越界: {waited:?}"
    );
}

#[test]
fn rate_limit_retry_after_exceeds_remaining_deadline() {
    let (outcome, transport) = run(
        LlmTask::InsightExtract,
        vec![
            Step::Status(429, vec![("Retry-After".into(), "1".into())], String::new()),
            Step::Json(support::envelope(&support::insight_body())),
        ],
        300,
    );
    assert!(outcome.degraded);
    assert_eq!(outcome.failure, Some(FailureClass::RateLimited));
    assert_eq!(outcome.attempts, 1);
    assert_eq!(transport.requests(), 1);
}

#[test]
fn rate_limit_beyond_remaining_budget_degrades() {
    let (outcome, transport) = run(
        LlmTask::InsightExtract,
        vec![
            Step::Status(429, vec![("Retry-After".into(), "5".into())], String::new()),
            Step::Json(support::envelope(&support::insight_body())),
        ],
        2_000,
    );
    assert!(outcome.degraded);
    assert_eq!(outcome.failure, Some(FailureClass::RateLimited));
    assert_eq!(outcome.attempts, 1);
    assert_eq!(transport.requests(), 1);
}

#[test]
fn invalid_schema_repair() {
    let (outcome, transport) = run(
        LlmTask::InsightExtract,
        vec![
            Step::Json("{}".into()),
            Step::Json(support::envelope(&support::insight_body())),
        ],
        6_000,
    );
    assert!(!outcome.degraded);
    assert_eq!(outcome.attempts, 2);
    assert_eq!(transport.requests(), 2);
}

#[test]
fn invalid_schema_twice_degrades_with_same_shape() {
    let (outcome, transport) = run(
        LlmTask::InsightExtract,
        vec![Step::Json("{}".into()), Step::Json("{}".into())],
        6_000,
    );
    assert!(outcome.degraded);
    assert_eq!(outcome.failure, Some(FailureClass::InvalidSchema));
    assert_eq!(outcome.attempts, 2);
    assert!(matches!(outcome.body, Some(StructuredBody::Insight(_))));
    assert!(matches!(
        outcome.body.as_ref().map(|body| body.task()),
        Some(LlmTask::InsightExtract)
    ));
    assert_eq!(transport.requests(), 2);
}

#[test]
fn circuit_open_blocks_requests() {
    let clock = Arc::new(FakeClock::new());
    let transport = support::FakeTransport::new(steps(9, Step::Fail(TransportError::ReadTimeout)));
    let config = Arc::new(RwLock::new(support::test_config()));
    let (gateway, _observer) = support::gateway_with_config(config, transport.clone(), clock);
    let request = support::request(LlmTask::InsightExtract, "会议上下文", 6_000);
    for _ in 0..3 {
        let outcome = gateway.complete_structured(&request).expect("调用");
        assert!(outcome.degraded);
    }
    assert_eq!(transport.requests(), 9);
    let outcome = gateway.complete_structured(&request).expect("冷却调用");
    assert_eq!(outcome.degrade_cause, Some(DegradeCause::Cooling));
    assert_eq!(transport.requests(), 9);
}

#[test]
fn circuit_recovers_after_cooling() {
    let clock = Arc::new(FakeClock::new());
    let mut all = steps(9, Step::Fail(TransportError::ReadTimeout));
    all.push(Step::Json(support::envelope(&support::insight_body())));
    let transport = support::FakeTransport::new(all);
    let config = Arc::new(RwLock::new(support::test_config()));
    let (gateway, _observer) =
        support::gateway_with_config(config, transport.clone(), clock.clone());
    let request = support::request(LlmTask::InsightExtract, "会议上下文", 6_000);
    for _ in 0..3 {
        assert!(
            gateway
                .complete_structured(&request)
                .expect("调用")
                .degraded
        );
    }
    clock.advance(Duration::from_secs(61));
    let outcome = gateway.complete_structured(&request).expect("恢复调用");
    assert!(!outcome.degraded);
    assert_eq!(transport.requests(), 10);
}

#[test]
fn payload_too_large_never_leaves_the_process() {
    let mut config = support::test_config();
    config.payload_char_limit = 10;
    let transport = support::FakeTransport::new(vec![]);
    let (gateway, _observer) = support::gateway(config, transport.clone());
    let request = support::request(LlmTask::Summary, "超过十个字符的会议上下文", 6_000);
    let outcome = gateway.complete_structured(&request).expect("调用");
    assert_eq!(outcome.degrade_cause, Some(DegradeCause::PayloadTooLarge));
    assert_eq!(transport.requests(), 0);
}

#[test]
fn all_failures_no_panic() {
    let cases = vec![
        (
            FailureClass::Timeout,
            steps(3, Step::Fail(TransportError::ReadTimeout)),
        ),
        (
            FailureClass::Offline,
            steps(3, Step::Fail(TransportError::NetworkUnreachable)),
        ),
        (
            FailureClass::Unavailable,
            steps(3, Step::Fail(TransportError::ConnectRefused)),
        ),
        (
            FailureClass::RateLimited,
            vec![
                Step::Status(429, vec![("Retry-After".into(), "1".into())], String::new()),
                Step::Status(429, vec![("Retry-After".into(), "1".into())], String::new()),
            ],
        ),
        (
            FailureClass::InvalidSchema,
            vec![Step::Json("{}".into()), Step::Json("{}".into())],
        ),
        (
            FailureClass::ContentFiltered,
            vec![Step::Status(400, Vec::new(), "content_filter".into())],
        ),
    ];
    for (expected, steps) in cases {
        let result = catch_unwind(AssertUnwindSafe(|| {
            run(LlmTask::InsightExtract, steps, 6_000)
        }));
        assert!(result.is_ok(), "{expected:?} 触发 panic");
        let (outcome, _) = result.expect("已检查");
        assert_eq!(outcome.failure, Some(expected));
        assert!(outcome.degraded);
    }
}

#[test]
fn degrade_same_shape_for_four_tasks() {
    for task in LlmTask::ALL {
        let (outcome, _) = run(
            task,
            steps(3, Step::Fail(TransportError::NetworkUnreachable)),
            6_000,
        );
        assert!(outcome.degraded);
        assert!(outcome.degrade_cause.is_some());
        assert_eq!(outcome.body.as_ref().map(StructuredBody::task), Some(task));
        assert!(outcome.invariants_hold());
    }
}

#[test]
fn fallback_insight_has_sources_and_low_confidence() {
    let input = FallbackInput {
        sources: vec![UtteranceRef::new("utt-1").expect("ref")],
        window_text: Some("确认下周灰度上线".into()),
        last_summary: None,
    };
    match RuleFallback::produce(LlmTask::InsightExtract, &input) {
        StructuredBody::Insight(result) => {
            assert!(!result.insights.is_empty());
            assert_eq!(result.insights[0].sources[0].as_str(), "utt-1");
            assert!(result.insights[0].confidence.value() <= 0.5);
        }
        other => panic!("类型错误：{other:?}"),
    }
}

#[test]
fn fallback_summary_uses_previous_when_present() {
    let previous = Summary {
        purpose: SummaryPurpose::RollingWindow,
        text: UtteranceText::new("上一版摘要").expect("text"),
        covered: None,
        version: 3,
        carried_over: false,
        produced: true,
    };
    let input = FallbackInput {
        last_summary: Some(previous),
        ..FallbackInput::default()
    };
    match RuleFallback::produce(LlmTask::Summary, &input) {
        StructuredBody::Summary(result) => {
            assert!(result.carried_over);
            assert_eq!(result.version, 3);
        }
        other => panic!("类型错误：{other:?}"),
    }
    match RuleFallback::produce(LlmTask::Summary, &FallbackInput::default()) {
        StructuredBody::Summary(result) => assert!(!result.produced),
        other => panic!("类型错误：{other:?}"),
    }
}

#[test]
fn fallback_conclusion_is_empty_and_explained() {
    match RuleFallback::produce(LlmTask::CardConclusion, &FallbackInput::default()) {
        StructuredBody::Conclusion(result) => {
            assert!(result.conclusion.is_none());
            assert!(result.notice.is_some());
        }
        other => panic!("类型错误：{other:?}"),
    }
}
