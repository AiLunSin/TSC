//! 抽取链路的集成用例：预筛、能力、Schema、失败与正常路径。

use meeting_copilot_application::understand::{
    run_extraction, NormalizedFragment, UnderstandingCounters,
};
use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::enums::InsightKind;
use meeting_copilot_domain::model::llm::{
    FailureClass, InsightExtract, InsightItem, LlmGenerationRequest, LlmTask, StreamEvent,
    StructuredBody, StructuredOutcome,
};
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::model::utterance::SpeakerRef;
use meeting_copilot_domain::port::llm::{
    CancelToken, LlmCapability, StreamOutcomeSummary, StructuredLlmGateway,
};
use meeting_copilot_domain::vo::{Confidence, SessionId, ShortText, UtteranceRef, UtteranceText};

struct StubGateway {
    endpoint_configured: bool,
    body: Option<StructuredBody>,
    fail: bool,
    failure: Option<FailureClass>,
}

impl StructuredLlmGateway for StubGateway {
    fn complete_structured(
        &self,
        request: &LlmGenerationRequest,
    ) -> Result<StructuredOutcome, CopilotError> {
        if self.fail {
            return Err(CopilotError::NetworkOffline);
        }
        Ok(StructuredOutcome {
            request_id: request.request_id.clone(),
            task: request.task,
            body: self.body.clone(),
            model: "stub".into(),
            tokens_used: None,
            degraded: false,
            degrade_cause: None,
            failure: self.failure,
            attempts: 1,
            truncated: false,
            schema_version: "v1",
            http_status: None,
            normalization: Vec::new(),
        })
    }

    fn stream_structured(
        &self,
        _request: &LlmGenerationRequest,
        _cancel: &CancelToken,
        _sink: &mut (dyn FnMut(StreamEvent) + Send),
    ) -> Result<StreamOutcomeSummary, CopilotError> {
        Err(CopilotError::NetworkOffline)
    }

    fn capability(&self) -> LlmCapability {
        LlmCapability {
            task: LlmTask::InsightExtract,
            endpoint_configured: self.endpoint_configured,
            model_configured: true,
            cooling: false,
        }
    }
}

fn fragment(text: &str) -> NormalizedFragment {
    NormalizedFragment {
        session: SessionId::new("s1").expect("session"),
        reference: UtteranceRef::new("u1").expect("ref"),
        parent_reference: UtteranceRef::new("u1").expect("ref"),
        span: TextRange {
            start_ms: 0,
            end_ms: 1_000,
        },
        text: UtteranceText::new(text).expect("text"),
        speaker: SpeakerRef::unknown(),
        confidence: Confidence::new(0.9).expect("conf"),
        dual_channel: false,
        origin_pair: None,
    }
}

fn extract_with(sources: Vec<&str>) -> InsightExtract {
    InsightExtract {
        insights: vec![InsightItem {
            kind: InsightKind::KeyInfo,
            text: ShortText::new("确认下周上线").expect("text"),
            sources: sources
                .into_iter()
                .map(|name| UtteranceRef::new(name).expect("ref"))
                .collect(),
            confidence: Confidence::new(0.9).expect("conf"),
            decision: None,
            todo: None,
        }],
    }
}

fn gateway(endpoint: bool, body: Option<StructuredBody>, fail: bool) -> StubGateway {
    StubGateway {
        endpoint_configured: endpoint,
        body,
        fail,
        failure: None,
    }
}

#[test]
fn prefilter_miss_skips_gateway_call() {
    let counters = UnderstandingCounters::new();
    let drafts = run_extraction(
        &fragment("嗯好的"),
        &gateway(true, None, false),
        &counters,
        None,
    );
    assert!(drafts.is_empty());
    assert_eq!(counters.snapshot().prefilter_miss, 1);
    assert_eq!(counters.snapshot().gateway_calls, 0);
}

#[test]
fn missing_endpoint_falls_back_to_rules() {
    let counters = UnderstandingCounters::new();
    let drafts = run_extraction(
        &fragment("确认下周灰度上线"),
        &gateway(false, None, false),
        &counters,
        None,
    );
    assert_eq!(drafts.len(), 1);
    assert_eq!(counters.snapshot().rule_fallback_count, 1);
    assert_eq!(counters.snapshot().gateway_calls, 0);
}

#[test]
fn schema_violation_falls_back_to_rules() {
    let counters = UnderstandingCounters::new();
    let drafts = run_extraction(
        &fragment("确认下周灰度上线"),
        &gateway(true, None, false),
        &counters,
        None,
    );
    assert_eq!(drafts.len(), 1);
    assert_eq!(counters.snapshot().schema_violation, 1);
    assert_eq!(counters.snapshot().gateway_calls, 1);
}

#[test]
fn gateway_error_falls_back_to_rules() {
    let counters = UnderstandingCounters::new();
    let drafts = run_extraction(
        &fragment("确认下周灰度上线"),
        &gateway(true, None, true),
        &counters,
        None,
    );
    assert_eq!(drafts.len(), 1);
    assert_eq!(counters.snapshot().gateway_cooling, 1);
    assert_eq!(counters.snapshot().rule_fallback_count, 1);
}

#[test]
fn valid_extract_returns_drafts() {
    let counters = UnderstandingCounters::new();
    let body = StructuredBody::Insight(extract_with(vec!["u1"]));
    let drafts = run_extraction(
        &fragment("确认下周灰度上线"),
        &gateway(true, Some(body), false),
        &counters,
        None,
    );
    assert_eq!(drafts.len(), 1);
    assert_eq!(drafts[0].sources.len(), 1);
    assert_eq!(counters.snapshot().gateway_calls, 1);
    assert_eq!(counters.snapshot().schema_violation, 0);
}

#[test]
fn unavailable_failure_falls_back_to_rules() {
    let counters = UnderstandingCounters::new();
    let mut stub = gateway(true, None, false);
    stub.failure = Some(FailureClass::Unavailable);
    let drafts = run_extraction(&fragment("确认下周灰度上线"), &stub, &counters, None);
    assert_eq!(drafts.len(), 1);
    assert_eq!(counters.snapshot().gateway_cooling, 1);
    assert_eq!(counters.snapshot().schema_violation, 0);
}

#[test]
fn invalid_schema_failure_is_counted() {
    let counters = UnderstandingCounters::new();
    let mut stub = gateway(true, None, false);
    stub.failure = Some(FailureClass::InvalidSchema);
    let drafts = run_extraction(&fragment("确认下周灰度上线"), &stub, &counters, None);
    assert_eq!(drafts.len(), 1);
    assert_eq!(counters.snapshot().schema_violation, 1);
    assert_eq!(counters.snapshot().gateway_cooling, 0);
}
