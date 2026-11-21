mod support;

use meeting_copilot_domain::model::llm::{LlmTask, ALLOWED_LLM_LOG_FIELDS};
use meeting_copilot_infrastructure::llm::transport::TransportError;
use meeting_copilot_infrastructure::llm::LocalObserver;
use support::Step;

fn run_degraded(
    transport: Step,
) -> (
    meeting_copilot_domain::model::llm::StructuredOutcome,
    std::sync::Arc<LocalObserver>,
) {
    let transport =
        support::FakeTransport::new(vec![transport.clone(), transport.clone(), transport]);
    let (gateway, observer) = support::gateway(support::test_config(), transport);
    let request = support::request(
        LlmTask::InsightExtract,
        &format!("{} 的会议上下文 sk-test-0000", support::SENSITIVE),
        6_000,
    );
    let outcome = gateway.complete_structured(&request).expect("调用");
    (outcome, observer)
}

#[test]
fn logs_and_audit_never_contain_payload_or_credentials() {
    let (outcome, observer) = run_degraded(Step::Fail(TransportError::NetworkUnreachable));
    assert!(outcome.degraded);

    let logs = observer.logs();
    assert!(!logs.is_empty());
    for record in &logs {
        assert!(
            ALLOWED_LLM_LOG_FIELDS.contains(&record.field()),
            "非白名单字段 {}",
            record.field()
        );
        assert!(!record.value().contains(support::SENSITIVE));
        assert!(!record.value().contains("sk-test-0000"));
    }

    let audits = observer.audit_records();
    assert_eq!(audits.len(), 2);
    for record in &audits {
        assert!(!record.summary.contains(support::SENSITIVE));
        assert!(!record.summary.contains("sk-test-0000"));
        assert!(!record.summary.contains("http://"));
        assert!(!record.summary.contains("https://"));
        assert!(record.summary.contains("task=insight_extract"));
        assert!(record.summary.contains("chars="));
        assert!(record.summary.contains("attempts="));
    }
}

#[test]
fn successful_audit_contains_metadata_only() {
    let transport = support::FakeTransport::new(vec![Step::Json(support::envelope(
        &support::insight_body(),
    ))]);
    let (gateway, observer) = support::gateway(support::test_config(), transport);
    let request = support::request(LlmTask::InsightExtract, support::SENSITIVE, 6_000);
    let outcome = gateway.complete_structured(&request).expect("调用");
    assert!(!outcome.degraded);
    let audits = observer.audit_records();
    assert_eq!(audits.len(), 2);
    assert!(audits[0].summary.contains("phase=intent"));
    assert!(audits[1].summary.contains("phase=result"));
    assert!(audits[1].summary.contains("outcome=ok"));
    assert!(!audits[1].summary.contains(support::SENSITIVE));
}
