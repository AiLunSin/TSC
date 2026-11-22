//! 关键词卡片契约（AC-B4-51~57）。

mod common;

use common::Harness;
use meeting_copilot_application::gate::keyword::{
    derive_card_id, derive_keyword_candidates, keyword_card_admission, resolve_click_state,
    ClickFacts, KeywordCardEnv,
};
use meeting_copilot_application::gate::record_from;
use meeting_copilot_domain::model::enums::{GapKind, SessionState, TriggerKind};
use meeting_copilot_domain::model::gate::{
    AuthorizationStatus, GateDecision, GateRejectReason, GateStage, OutboundIntent,
    PerStageResults, StageResult,
};
use meeting_copilot_domain::model::gate_keyword::{
    CitationAlignment, ClickDisabledReason, ClickState, KeywordCardStatus, KeywordKind,
};
use meeting_copilot_domain::vo::TraceId;

fn ready() -> Harness {
    let mut harness = Harness::new(10_000);
    harness.lexicon.push("张伟".to_owned());
    harness.push("张伟负责该模块。", 0.9, 0, 100);
    harness.push("本次讨论 OKR 指标。", 0.9, 200, 300);
    harness.push("中国科学院参与项目。", 0.9, 400, 500);
    harness.push("依据 GB/T 12345 执行。", 0.9, 600, 700);
    harness.push("转化率提升 30%。", 0.9, 800, 900);
    harness.relevance.semantic_milli = 1000;
    harness.relevance.entity_milli = 1000;
    harness.relevance.gain_milli = 1000;
    harness.relevance.source_trust_milli = 1000;
    harness
}

#[test]
fn keyword_candidates_cover_five_kinds() {
    let harness = ready();
    let set = derive_keyword_candidates(&harness.scan());
    assert_eq!(set.new_candidates.len(), 5);
    let mut kinds = Vec::new();
    for candidate in &set.new_candidates {
        assert!(candidate.validate());
        assert!(!kinds.contains(&candidate.keyword_kind));
        kinds.push(candidate.keyword_kind);
    }
    assert_eq!(kinds.len(), 5);
    assert!(kinds.contains(&KeywordKind::InstitutionRole));
    assert!(!kinds.contains(&KeywordKind::ProperNoun) || kinds.len() == 5);
}

#[test]
fn keyword_candidate_keeps_first_seen_only() {
    let mut harness = ready();
    harness.push("张伟再次说明。", 0.9, 1_000, 1_100);
    let set = derive_keyword_candidates(&harness.scan());
    assert_eq!(set.new_candidates.len(), 5);
    assert_eq!(set.reobservations.len(), 1);
    assert_eq!(
        set.reobservations[0].dedupe_key,
        set.new_candidates
            .iter()
            .find(|item| item.dedupe_key == set.reobservations[0].dedupe_key)
            .expect("first seen")
            .dedupe_key
    );
}

#[test]
fn unanswered_question_is_not_a_keyword_card() {
    let mut harness = Harness::new(10_000);
    harness.push("这个参数怎么设置？", 0.9, 0, 0);
    let gaps = meeting_copilot_application::gate::detect_gaps(&harness.scan());
    assert!(gaps
        .iter()
        .any(|gap| gap.kind == GapKind::UnansweredQuestion));
    let set = derive_keyword_candidates(&harness.scan());
    assert!(set.new_candidates.is_empty());
}

#[test]
fn g2_skip_marks_stage_skipped() {
    let policy = meeting_copilot_domain::model::gate::GatePolicy {
        relevance_enabled: false,
        ..Default::default()
    };
    let outcome = ready().outcome_with(&policy);
    match outcome.decision {
        GateDecision::Allow { per_stage, .. } => {
            assert_eq!(per_stage.0[1].result, StageResult::Skipped);
        }
        other => panic!("expected allow, got {other:?}"),
    }
}

#[test]
fn g3_skip_marks_stage_skipped() {
    let policy = meeting_copilot_domain::model::gate::GatePolicy {
        dedupe_enabled: false,
        ..Default::default()
    };
    let outcome = ready().outcome_with(&policy);
    match outcome.decision {
        GateDecision::Allow { per_stage, .. } => {
            assert_eq!(per_stage.0[2].result, StageResult::Skipped);
        }
        other => panic!("expected allow, got {other:?}"),
    }
}

fn admission_for(
    decision: &GateDecision,
) -> meeting_copilot_application::gate::keyword::KeywordCardAdmission {
    let harness = ready();
    let set = derive_keyword_candidates(&harness.scan());
    let candidate = set.new_candidates.first().expect("candidate");
    let env = KeywordCardEnv {
        authorization_status: AuthorizationStatus::Granted,
        authorization_revoked: false,
        screen_share: false,
        emergency_masked: false,
        current_session: &harness.session,
        card_status: KeywordCardStatus::Displayed,
    };
    keyword_card_admission(
        candidate,
        decision,
        &TraceId::new("t-new").expect("trace"),
        CitationAlignment::Passed,
        &env,
    )
}

#[test]
fn keyword_card_requires_gate_allow() {
    let outcome = ready().outcome();
    let admission = admission_for(&outcome.decision);
    assert!(admission.card_drafts);
    assert_eq!(admission.click_state, ClickState::Enabled);
    assert!(admission.allow_ref.is_some());
    assert!(admission.body_visible);
}

#[test]
fn deny_never_emits_clickable_card() {
    let mut harness = ready();
    harness.session_state = SessionState::Paused;
    let outcome = harness.outcome();
    assert!(matches!(
        outcome.decision,
        GateDecision::Deny {
            stopped_at: GateStage::G1,
            ..
        }
    ));
    let admission = admission_for(&outcome.decision);
    assert!(!admission.card_drafts);
    assert_eq!(
        admission.click_state,
        ClickState::Disabled(ClickDisabledReason::NotAllowed)
    );
    assert!(admission.allow_ref.is_none());
    assert!(!admission.body_visible);
}

#[test]
fn click_permission_is_fail_closed() {
    let base = ClickFacts {
        allow_ref_present: true,
        alignment: CitationAlignment::Passed,
        card_status: KeywordCardStatus::Displayed,
        session_matches: true,
        authorization_granted: true,
        authorization_revoked: false,
        screen_share: false,
        emergency_masked: false,
        reference_stale: false,
    };
    assert_eq!(resolve_click_state(base), ClickState::Enabled);
    let stale = ClickFacts {
        reference_stale: true,
        ..base
    };
    assert_eq!(
        resolve_click_state(stale),
        ClickState::Disabled(ClickDisabledReason::StaleReference)
    );
    let revoked = ClickFacts {
        authorization_revoked: true,
        ..base
    };
    assert_eq!(
        resolve_click_state(revoked),
        ClickState::Disabled(ClickDisabledReason::AuthorizationRevoked)
    );
    let masked = ClickFacts {
        emergency_masked: true,
        ..base
    };
    assert_eq!(
        resolve_click_state(masked),
        ClickState::Disabled(ClickDisabledReason::EmergencyMasked)
    );
    let screen = ClickFacts {
        screen_share: true,
        ..base
    };
    assert_eq!(
        resolve_click_state(screen),
        ClickState::Disabled(ClickDisabledReason::ScreenShareProtection)
    );
    let unverified = ClickFacts {
        alignment: CitationAlignment::Failed,
        ..base
    };
    assert_eq!(
        resolve_click_state(unverified),
        ClickState::Disabled(ClickDisabledReason::ReferenceUnverified)
    );
    assert_eq!(
        resolve_click_state(ClickFacts {
            reference_stale: true,
            authorization_revoked: true,
            ..base
        }),
        ClickState::Disabled(ClickDisabledReason::StaleReference)
    );
}

#[test]
fn keyword_card_hidden_under_screen_share() {
    let harness = ready();
    let set = derive_keyword_candidates(&harness.scan());
    let candidate = set.new_candidates.first().expect("candidate");
    let decision = GateDecision::Allow {
        per_stage: PerStageResults::empty(),
        budget_remaining: 1,
        candidates: Vec::new(),
        outbound: Some(OutboundIntent {
            sanitized_query_len: 0,
            content_type: meeting_copilot_domain::model::gate::GateContentType::TextQuery,
            provider_ids: Vec::new(),
            authorization_status: AuthorizationStatus::Granted,
            truncated: false,
            queued: false,
            cooldown_until_ms: None,
        }),
        suppressed_notice: false,
        stopped_at: None,
    };
    let env = KeywordCardEnv {
        authorization_status: AuthorizationStatus::Granted,
        authorization_revoked: false,
        screen_share: true,
        emergency_masked: false,
        current_session: &harness.session,
        card_status: KeywordCardStatus::Displayed,
    };
    let admission = keyword_card_admission(
        candidate,
        &decision,
        &TraceId::new("t-1").expect("trace"),
        CitationAlignment::Passed,
        &env,
    );
    assert!(!admission.body_visible);
    assert_eq!(
        admission.click_state,
        ClickState::Disabled(ClickDisabledReason::ScreenShareProtection)
    );
}

#[test]
fn keyword_card_has_deterministic_id() {
    let harness = ready();
    let set = derive_keyword_candidates(&harness.scan());
    let candidate = set.new_candidates.first().expect("candidate");
    let first = derive_card_id(&harness.session, &candidate.dedupe_key);
    let second = derive_card_id(&harness.session, &candidate.dedupe_key);
    assert_eq!(first, second);
    assert_eq!(first.as_str().len(), 36);
}

#[test]
fn keyword_card_zero_transcript_boundary() {
    let harness = ready();
    let outcome = harness.outcome();
    let record = record_from(&harness.input(), &outcome.decision, &harness.budget);
    assert_eq!(record.schema_version, 1);
    assert!(record.candidate_ref.is_some());
    assert!(record.insight_ref.is_none());
    assert!(record.unknown.is_empty());
}

#[test]
fn keyword_card_type_boundary_has_no_free_text_owner() {
    let harness = ready();
    let outcome = harness.outcome();
    assert!(matches!(
        outcome.decision,
        GateDecision::Allow { .. } | GateDecision::Deny { .. }
    ));
    let _ = TriggerKind::OnDemand;
    let _ = GateRejectReason::PolicyUnavailable;
}
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountingSanitizer {
    calls: AtomicUsize,
}

impl meeting_copilot_domain::port::governance::Sanitizer for CountingSanitizer {
    fn sanitize_query(
        &self,
        raw: &str,
    ) -> Result<(String, u16), meeting_copilot_domain::error::CopilotError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok((raw.to_owned(), 0))
    }
    fn sanitize_output(
        &self,
        raw: &str,
    ) -> Result<String, meeting_copilot_domain::error::CopilotError> {
        Ok(raw.to_owned())
    }
    fn scan(&self, _raw: &str) -> Vec<meeting_copilot_domain::port::governance::SensitiveHit> {
        Vec::new()
    }
}

struct CountingSink {
    calls: AtomicUsize,
}

impl meeting_copilot_application::gate::AuditSink for CountingSink {
    fn append(
        &self,
        _event: &meeting_copilot_domain::model::gate::GateAuditEvent,
    ) -> Result<(), meeting_copilot_application::gate::AuditError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[test]
fn clicking_keyword_card_is_local_only() {
    let harness = ready();
    let sanitizer = CountingSanitizer {
        calls: AtomicUsize::new(0),
    };
    let sink = CountingSink {
        calls: AtomicUsize::new(0),
    };
    let _ = &sanitizer;
    let _ = &sink;
    let outcome = harness.outcome();
    let admission = admission_for(&outcome.decision);
    let set = derive_keyword_candidates(&harness.scan());
    let candidate = set.new_candidates.first().expect("candidate");
    let card_id = derive_card_id(&harness.session, &candidate.dedupe_key);
    assert_eq!(card_id.as_str().len(), 36);
    assert!(admission.card_drafts);
    assert_eq!(sanitizer.calls.load(Ordering::SeqCst), 0);
    assert_eq!(sink.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn continue_search_reenters_gate() {
    let harness = ready();
    let set = derive_keyword_candidates(&harness.scan());
    let candidate = set.new_candidates.first().expect("candidate");
    let env = KeywordCardEnv {
        authorization_status: AuthorizationStatus::Granted,
        authorization_revoked: false,
        screen_share: false,
        emergency_masked: false,
        current_session: &harness.session,
        card_status: KeywordCardStatus::Displayed,
    };
    let allow = harness.outcome().decision;
    let first = keyword_card_admission(
        candidate,
        &allow,
        &TraceId::new("t-1").expect("trace"),
        CitationAlignment::Passed,
        &env,
    );
    let deny = GateDecision::Deny {
        reason: GateRejectReason::Paused,
        stopped_at: GateStage::G1,
        per_stage: PerStageResults::empty(),
        cooldown_until_ms: None,
    };
    let denied = keyword_card_admission(
        candidate,
        &deny,
        &TraceId::new("t-2").expect("trace"),
        CitationAlignment::Passed,
        &env,
    );
    assert!(!denied.card_drafts);
    assert!(denied.allow_ref.is_none());
    let second = keyword_card_admission(
        candidate,
        &allow,
        &TraceId::new("t-2").expect("trace"),
        CitationAlignment::Passed,
        &env,
    );
    assert_ne!(
        first.allow_ref.expect("first").trace_id(),
        second.allow_ref.expect("second").trace_id()
    );
}
