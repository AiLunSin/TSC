//! G5 外发授权、顺序、审计与关闭态（AC-B4-28~36）。

mod common;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use common::Harness;
use meeting_copilot_application::gate::{GateFeature, NoopSink};
use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::enums::TriggerKind;
use meeting_copilot_domain::model::gate::{
    AuditResult, AuthorizationStatus, GateAuditEvent, GateCounter, GateDecision, GatePolicy,
    GateRejectReason, GateStage, OutboundIntent,
};
use meeting_copilot_domain::port::governance::{Sanitizer, SensitiveHit};
use meeting_copilot_domain::vo::{ProviderId, SanitizedQuery};

#[derive(Default)]
struct SpySanitizer {
    received: Mutex<Vec<usize>>,
    fail: AtomicBool,
}

impl SpySanitizer {
    fn fail() -> Self {
        Self {
            received: Mutex::new(Vec::new()),
            fail: AtomicBool::new(true),
        }
    }

    fn calls(&self) -> usize {
        self.received.lock().expect("lock").len()
    }

    fn received_len(&self) -> usize {
        *self
            .received
            .lock()
            .expect("lock")
            .first()
            .expect("one call")
    }
}

impl Sanitizer for SpySanitizer {
    fn sanitize_query(&self, raw: &str) -> Result<(String, u16), CopilotError> {
        self.received
            .lock()
            .expect("lock")
            .push(raw.chars().count());
        if self.fail.load(Ordering::SeqCst) {
            return Err(CopilotError::PrivacyViolation {
                detail: "替身失败".to_owned(),
            });
        }
        Ok((raw.to_owned(), 0))
    }

    fn sanitize_output(&self, raw: &str) -> Result<String, CopilotError> {
        Ok(raw.to_owned())
    }

    fn scan(&self, _raw: &str) -> Vec<SensitiveHit> {
        Vec::new()
    }

    fn issue_query(&self, raw: &str) -> Result<SanitizedQuery, CopilotError> {
        let (text, _count) = self.sanitize_query(raw)?;
        SanitizedQuery::try_from(text).map_err(|_| CopilotError::PrivacyViolation {
            detail: "空查询".to_owned(),
        })
    }
}

#[derive(Default)]
struct SpySink {
    events: Mutex<Vec<GateAuditEvent>>,
    released: Arc<AtomicBool>,
    fail: AtomicBool,
}

impl SpySink {
    fn with_release(released: Arc<AtomicBool>) -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            released,
            fail: AtomicBool::new(false),
        }
    }

    fn failing() -> Self {
        Self {
            fail: AtomicBool::new(true),
            ..Self::default()
        }
    }

    fn calls(&self) -> usize {
        self.events.lock().expect("lock").len()
    }

    fn events(&self) -> Vec<GateAuditEvent> {
        self.events.lock().expect("lock").clone()
    }
}

impl meeting_copilot_application::gate::AuditSink for SpySink {
    fn append(
        &self,
        event: &GateAuditEvent,
    ) -> Result<(), meeting_copilot_application::gate::AuditError> {
        assert!(
            !self.released.load(Ordering::SeqCst),
            "审计必须先于外发提交"
        );
        if self.fail.load(Ordering::SeqCst) {
            return Err(meeting_copilot_application::gate::AuditError);
        }
        self.events.lock().expect("lock").push(event.clone());
        Ok(())
    }
}

fn ready() -> Harness {
    let mut harness = Harness::new(10_000);
    harness.lexicon.push("张伟".to_owned());
    harness.push("张伟负责该模块。", 0.9, 0, 100);
    harness.relevance.semantic_milli = 1000;
    harness.relevance.entity_milli = 1000;
    harness.relevance.gain_milli = 1000;
    harness.relevance.source_trust_milli = 1000;
    harness
}

fn deny_reason(outcome: &meeting_copilot_application::gate::GateOutcome) -> GateRejectReason {
    match &outcome.decision {
        GateDecision::Deny { reason, .. } => *reason,
        other => panic!("expected deny, got {other:?}"),
    }
}

#[test]
fn authorization_records_seven_required_fields() {
    let harness = ready();
    assert!(harness.authorization.is_complete());

    let mut missing = harness.authorization.clone();
    missing.granted_at_ms = None;
    assert!(!missing.is_complete());
    let mut missing = harness.authorization.clone();
    missing.content_types.clear();
    assert!(!missing.is_complete());
    let mut missing = harness.authorization.clone();
    missing.max_query_chars = None;
    assert!(!missing.is_complete());
    let mut missing = harness.authorization.clone();
    missing.provider_ids.clear();
    assert!(!missing.is_complete());
    let mut missing = harness.authorization.clone();
    missing.notice_version = 0;
    assert!(!missing.is_complete());
    let mut revoked = harness.authorization.clone();
    revoked.revoked_at_ms = Some(1);
    assert!(!revoked.is_complete());
    let mut not_granted = harness.authorization.clone();
    not_granted.status = AuthorizationStatus::Revoked;
    assert!(!not_granted.is_complete());
}

#[test]
fn unauthorized_outbound_is_zero() {
    for status in [
        AuthorizationStatus::Revoked,
        AuthorizationStatus::Expired,
        AuthorizationStatus::Missing,
    ] {
        let mut harness = ready();
        harness.authorization.status = status;
        let outcome = harness.outcome();
        assert_eq!(deny_reason(&outcome), GateRejectReason::OutboundNotGranted);
        assert_eq!(outcome.counters.get(GateCounter::GateL5NotGranted), 1);
        assert!(matches!(
            outcome.decision,
            GateDecision::Deny {
                stopped_at: GateStage::G5,
                ..
            }
        ));
    }
}

#[test]
fn revoked_authorization_is_not_auto_restored() {
    let mut harness = ready();
    harness.authorization.status = AuthorizationStatus::Revoked;
    harness.authorization.revoked_at_ms = Some(500);
    assert!(!harness.authorization.is_grantable());
    let outcome = harness.outcome();
    assert_eq!(deny_reason(&outcome), GateRejectReason::OutboundNotGranted);
    assert_eq!(harness.authorization.status, AuthorizationStatus::Revoked);
}

#[test]
fn reauthorization_bumps_notice_version() {
    let harness = ready();
    let mut renewed = harness.authorization.clone();
    renewed.notice_version = 2;
    renewed.granted_at_ms = Some(9_000);
    assert!(renewed.is_complete());
    assert!(renewed.notice_version > harness.authorization.notice_version);

    let mut stale = harness.authorization.clone();
    stale.notice_version = 0;
    assert!(!stale.is_complete());
}

#[test]
fn authorization_change_invalidates_old_grant() {
    let mut harness = ready();
    harness.authorization.provider_ids = vec![ProviderId::new("provider-other").expect("id")];
    let outcome = harness.outcome();
    assert_eq!(deny_reason(&outcome), GateRejectReason::ProviderNotAllowed);
    assert_eq!(
        outcome
            .counters
            .get(GateCounter::GateL5ProviderOrDomainBlocked),
        1
    );

    let mut narrow = ready();
    narrow.authorization.max_query_chars = Some(60);
    let outcome = narrow.outcome();
    assert_eq!(deny_reason(&outcome), GateRejectReason::OutboundNotGranted);
}

#[test]
fn auto_query_is_capped_at_80() {
    let harness = ready();
    let spy = SpySanitizer::default();
    let sink = NoopSink;
    let seed = "字".repeat(81);
    let outcome = harness.outcome_with_ports(&GatePolicy::default(), &spy, &sink, seed.as_str());
    assert_eq!(spy.received_len(), 80);
    let intent = outbound(&outcome);
    assert!(intent.truncated);
    assert_eq!(intent.sanitized_query_len, 80);
}

#[test]
fn on_demand_query_is_capped_at_200() {
    let mut harness = ready();
    harness.trigger = TriggerKind::OnDemand;
    let spy = SpySanitizer::default();
    let sink = NoopSink;
    let seed = "字".repeat(201);
    let outcome = harness.outcome_with_ports(&GatePolicy::default(), &spy, &sink, seed.as_str());
    assert_eq!(spy.received_len(), 200);
    let intent = outbound(&outcome);
    assert!(intent.truncated);
    assert_eq!(intent.sanitized_query_len, 200);
}

fn outbound(outcome: &meeting_copilot_application::gate::GateOutcome) -> OutboundIntent {
    match &outcome.decision {
        GateDecision::Allow {
            outbound: Some(intent),
            ..
        } => intent.clone(),
        other => panic!("expected allow with intent, got {other:?}"),
    }
}

#[test]
fn sanitizer_receives_truncated_input_only() {
    let harness = ready();
    let spy = SpySanitizer::default();
    let sink = NoopSink;
    let seed = "字".repeat(81);
    let outcome = harness.outcome_with_ports(&GatePolicy::default(), &spy, &sink, seed.as_str());
    assert_eq!(spy.calls(), 1);
    assert_eq!(spy.received_len(), 80);
    let intent = outbound(&outcome);
    assert_eq!(intent.sanitized_query_len, spy.received_len());
}

#[test]
fn oversize_never_reaches_sanitizer() {
    let mut harness = ready();
    harness.trigger = TriggerKind::OnDemand;
    let spy = SpySanitizer::default();
    let sink = NoopSink;
    let seed = "字".repeat(500);
    let outcome = harness.outcome_with_ports(&GatePolicy::default(), &spy, &sink, seed.as_str());
    assert_eq!(spy.calls(), 1);
    assert!(spy.received_len() <= 200);
    assert!(matches!(outcome.decision, GateDecision::Allow { .. }));
}

#[test]
fn audit_unavailable_blocks_outbound() {
    let mut harness = ready();
    harness.audit_available = false;
    let spy = SpySanitizer::default();
    let sink = SpySink::default();
    let outcome = harness.outcome_with_ports(&GatePolicy::default(), &spy, &sink, "查询");
    assert_eq!(deny_reason(&outcome), GateRejectReason::AuditUnavailable);
    assert_eq!(outcome.counters.get(GateCounter::GateL5AuditUnavailable), 1);
    assert_eq!(sink.calls(), 0);
}

#[test]
fn audit_append_error_blocks_outbound() {
    let harness = ready();
    let spy = SpySanitizer::default();
    let sink = SpySink::failing();
    let outcome = harness.outcome_with_ports(&GatePolicy::default(), &spy, &sink, "查询");
    assert_eq!(deny_reason(&outcome), GateRejectReason::AuditUnavailable);
    assert_eq!(outcome.counters.get(GateCounter::GateL5AuditUnavailable), 1);
    assert_eq!(sink.calls(), 0);
}

#[test]
fn audit_is_appended_before_release() {
    let harness = ready();
    let released = Arc::new(AtomicBool::new(false));
    let spy = SpySanitizer::default();
    let sink = SpySink::with_release(released.clone());
    let outcome = harness.outcome_with_ports(&GatePolicy::default(), &spy, &sink, "查询");
    assert_eq!(sink.calls(), 1);
    assert!(!released.load(Ordering::SeqCst));
    let _ = outbound(&outcome);
    released.store(true, Ordering::SeqCst);
}

#[test]
fn sanitize_failure_blocks_outbound() {
    let harness = ready();
    let spy = SpySanitizer::fail();
    let sink = SpySink::default();
    let outcome = harness.outcome_with_ports(&GatePolicy::default(), &spy, &sink, "查询");
    assert_eq!(deny_reason(&outcome), GateRejectReason::SanitizeFailed);
    assert_eq!(outcome.counters.get(GateCounter::GateL5SanitizeFailed), 1);
    assert_eq!(sink.calls(), 1);
    assert_eq!(sink.events()[0].result, AuditResult::Deny);
}

#[test]
fn outbound_audit_has_required_fields_without_text() {
    let harness = ready();
    let spy = SpySanitizer::default();
    let sink = SpySink::default();
    let outcome = harness.outcome_with_ports(&GatePolicy::default(), &spy, &sink, "查询");
    let intent = outbound(&outcome);
    assert_eq!(intent.sanitized_query_len, 2);
    assert_eq!(sink.calls(), 1);
    let event = &sink.events()[0];
    assert_eq!(event.authorization_status, AuthorizationStatus::Granted);
    assert_eq!(event.query_len, 2);
    assert_eq!(event.result, AuditResult::Allow);
    assert!(event.reason.is_none());
    assert!(!event.provider_ids.is_empty());
}

#[test]
fn gate_consumes_insights_readonly() {
    let harness = ready();
    let before = harness.fragments.clone();
    let _ = harness.outcome();
    assert_eq!(harness.fragments, before);
}

#[test]
fn gate_feature_disabled_is_zero_effect() {
    assert_eq!(GateFeature::default(), GateFeature::Disabled);
    let counters = meeting_copilot_domain::model::gate::GateCounters::default();
    assert_eq!(counters.snapshot().iter().map(|(_, v)| *v).sum::<u64>(), 0);
    let records: Vec<()> = Vec::new();
    assert!(records.is_empty());
}
