//! 回放面收敛：4 函数 + 3 结构（AC-B4-46/48/49）。

mod common;

use common::Harness;
use meeting_copilot_application::gate::{detect_gaps, FixedClock};
use meeting_copilot_domain::model::gate_replay::ReplayReport;

#[test]
fn replay_interface_is_io_free() {
    let harness = Harness::new(10_000);
    let scan = harness.scan();
    let first = detect_gaps(&scan);
    for _ in 0..10 {
        assert_eq!(detect_gaps(&scan), first);
    }
    let decision = harness.outcome().decision;
    assert_eq!(harness.outcome().decision, decision);
}

#[test]
fn fixed_clock_is_deterministic_sequence() {
    let clock = FixedClock::new(1_000, 250);
    let first: Vec<u64> = (0..3).map(|_| clock.now_ms()).collect();
    let clock = FixedClock::new(1_000, 250);
    let second: Vec<u64> = (0..3).map(|_| clock.now_ms()).collect();
    assert_eq!(first, second);
    assert_eq!(first, vec![1_000, 1_250, 1_500]);
}

#[test]
fn replay_report_has_closed_fields() {
    let report = ReplayReport {
        total: 200,
        matched: 200,
        mismatched: 0,
        mismatch_cases: Vec::new(),
        seed: 7,
    };
    assert_eq!(report.total, 200);
    assert_eq!(report.mismatched, 0);
    assert_eq!(report.mismatch_cases.len(), 0);
    assert_eq!(report.seed, 7);
}

// ---- AC-B4-48：方案 A 的 200 例回放（测试内构造，文本不出进程）----

use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::enums::SessionState;
use meeting_copilot_domain::model::gate::{
    AuthorizationStatus, GateCounter, GateDecision, GatePolicy, GateRejectReason, GateStage,
    NetworkState,
};
use meeting_copilot_domain::port::governance::Sanitizer;
use meeting_copilot_domain::vo::{ProviderId, SanitizedQuery};

const SEED: u64 = 7;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    AutoDisabled,
    ConsentMissing,
    Paused,
    SilentPeriod,
    ScreenShareProtection,
    LowConfidence,
    Suppressed,
    Quota,
    Duplicate,
    NoProvider,
    Sensitive,
    PolicyUnavailable,
    LowRelevance,
    Stale,
    Budget,
    Cooling,
    Offline,
    NotGranted,
    SanitizeFailed,
    AuditUnavailable,
    ProviderBlocked,
    AllowAuto,
    AllowOnDemand,
    AllowSkipG3,
    AllowQueued,
}

const KINDS: [Kind; 25] = [
    Kind::AutoDisabled,
    Kind::ConsentMissing,
    Kind::Paused,
    Kind::SilentPeriod,
    Kind::ScreenShareProtection,
    Kind::LowConfidence,
    Kind::Suppressed,
    Kind::Quota,
    Kind::Duplicate,
    Kind::NoProvider,
    Kind::Sensitive,
    Kind::PolicyUnavailable,
    Kind::LowRelevance,
    Kind::Stale,
    Kind::Budget,
    Kind::Cooling,
    Kind::Offline,
    Kind::NotGranted,
    Kind::SanitizeFailed,
    Kind::AuditUnavailable,
    Kind::ProviderBlocked,
    Kind::AllowAuto,
    Kind::AllowOnDemand,
    Kind::AllowSkipG3,
    Kind::AllowQueued,
];

struct ReplayCase {
    case_id: String,
    kind: Kind,
}

fn build_cases() -> Vec<ReplayCase> {
    (0..200)
        .map(|index| ReplayCase {
            case_id: format!("case-{index:03}"),
            kind: KINDS[index % KINDS.len()],
        })
        .collect()
}

struct FailingSanitizer;

impl Sanitizer for FailingSanitizer {
    fn sanitize_query(&self, _raw: &str) -> Result<(String, u16), CopilotError> {
        Err(CopilotError::PrivacyViolation {
            detail: "回放失败".to_owned(),
        })
    }
    fn sanitize_output(&self, raw: &str) -> Result<String, CopilotError> {
        Ok(raw.to_owned())
    }
    fn scan(&self, _raw: &str) -> Vec<meeting_copilot_domain::port::governance::SensitiveHit> {
        Vec::new()
    }
    fn issue_query(&self, _raw: &str) -> Result<SanitizedQuery, CopilotError> {
        Err(CopilotError::PrivacyViolation {
            detail: "回放失败".to_owned(),
        })
    }
}

struct NoSink;
impl meeting_copilot_application::gate::AuditSink for NoSink {
    fn append(
        &self,
        _event: &meeting_copilot_domain::model::gate::GateAuditEvent,
    ) -> Result<(), meeting_copilot_application::gate::AuditError> {
        Ok(())
    }
}

fn base() -> Harness {
    let mut harness = Harness::new(10_000);
    harness.lexicon.push("张伟".to_owned());
    harness.push("张伟负责该模块。", 0.9, 0, 100);
    harness.relevance.semantic_milli = 1000;
    harness.relevance.entity_milli = 1000;
    harness.relevance.gain_milli = 1000;
    harness.relevance.source_trust_milli = 1000;
    harness
}

fn expected_of(kind: Kind) -> Option<(GateRejectReason, GateStage)> {
    use GateRejectReason as R;
    use GateStage as S;
    match kind {
        Kind::AutoDisabled => Some((R::AutoDisabled, S::G1)),
        Kind::ConsentMissing => Some((R::ConsentMissing, S::G1)),
        Kind::Paused => Some((R::Paused, S::G1)),
        Kind::SilentPeriod => Some((R::SilentPeriod, S::G1)),
        Kind::ScreenShareProtection => Some((R::ScreenShareProtection, S::G1)),
        Kind::LowConfidence => Some((R::PreFilterLowConfidence, S::G1)),
        Kind::Suppressed => Some((R::Suppressed, S::G1)),
        Kind::Quota => Some((R::QuotaExhausted, S::G1)),
        Kind::Duplicate => Some((R::Duplicate, S::G1)),
        Kind::NoProvider => Some((R::NoProviderEnabled, S::G1)),
        Kind::Sensitive => Some((R::SensitiveTerm, S::G1)),
        Kind::PolicyUnavailable => Some((R::PolicyUnavailable, S::G1)),
        Kind::LowRelevance => Some((R::LowRelevance, S::G2)),
        Kind::Stale => Some((R::Stale, S::G3)),
        Kind::Budget => Some((R::BudgetExhausted, S::G4)),
        Kind::Cooling => Some((R::GatewayCooling, S::G4)),
        Kind::Offline => Some((R::Offline, S::G4)),
        Kind::NotGranted => Some((R::OutboundNotGranted, S::G5)),
        Kind::SanitizeFailed => Some((R::SanitizeFailed, S::G5)),
        Kind::AuditUnavailable => Some((R::AuditUnavailable, S::G5)),
        Kind::ProviderBlocked => Some((R::ProviderNotAllowed, S::G5)),
        Kind::AllowAuto | Kind::AllowOnDemand | Kind::AllowSkipG3 | Kind::AllowQueued => None,
    }
}

fn run_kind(kind: Kind) -> GateDecision {
    let mut harness = base();
    let mut policy = GatePolicy::default();
    let no_sink = NoSink;
    let pass = meeting_copilot_application::gate::PassthroughSanitizer;
    let fail = FailingSanitizer;
    match kind {
        Kind::AutoDisabled => harness.auto_suggestion_enabled = false,
        Kind::ConsentMissing => harness.consent_confirmed = false,
        Kind::Paused => harness.session_state = SessionState::Paused,
        Kind::SilentPeriod => harness.silent_period = true,
        Kind::ScreenShareProtection => harness.screen_share = true,
        Kind::LowConfidence => {
            harness.fragments.clear();
            harness.push("张伟负责该模块。", 0.49, 0, 100);
        }
        Kind::Suppressed => harness.session_suppressed = true,
        Kind::Quota => harness.budget.global_window_used = 5,
        Kind::Duplicate => harness.duplicate_detected = true,
        Kind::NoProvider => harness.provider_enabled = 0,
        Kind::Sensitive => harness.sensitive.push("张伟".to_owned()),
        Kind::PolicyUnavailable => policy.session_outbound_cap = 601,
        Kind::LowRelevance => harness.relevance = Default::default(),
        Kind::Stale => harness.cache_age_ms = Some(86_400_001),
        Kind::Budget => harness.budget.session_outbound_used = 120,
        Kind::Cooling => harness.llm_cooling = true,
        Kind::Offline => harness.network = NetworkState::Offline,
        Kind::NotGranted => harness.authorization.status = AuthorizationStatus::Revoked,
        Kind::SanitizeFailed => {
            return harness
                .outcome_with_ports(&policy, &fail, &no_sink, "查询")
                .decision;
        }
        Kind::AuditUnavailable => harness.audit_available = false,
        Kind::ProviderBlocked => {
            harness.authorization.provider_ids =
                vec![ProviderId::new("provider-other").expect("id")];
        }
        Kind::AllowOnDemand => {
            harness.trigger = meeting_copilot_domain::model::enums::TriggerKind::OnDemand
        }
        Kind::AllowSkipG3 => policy.dedupe_enabled = false,
        Kind::AllowQueued => harness.budget.inflight = 3,
        Kind::AllowAuto => {}
    }
    harness
        .outcome_with_ports(&policy, &pass, &no_sink, "查询")
        .decision
}

fn run(cases: &[ReplayCase]) -> ReplayReport {
    let mut matched = 0;
    let mut mismatch_cases = Vec::new();
    for case in cases {
        let decision = run_kind(case.kind);
        let ok = match (expected_of(case.kind), &decision) {
            (None, GateDecision::Allow { .. }) => true,
            (
                Some((reason, stage)),
                GateDecision::Deny {
                    reason: actual,
                    stopped_at,
                    ..
                },
            ) => *actual == reason && *stopped_at == stage,
            _ => false,
        };
        if ok {
            matched += 1;
        } else {
            mismatch_cases.push(case.case_id.clone());
        }
    }
    let total = u32::try_from(cases.len()).unwrap_or(0);
    ReplayReport {
        total,
        matched,
        mismatched: total - matched,
        mismatch_cases,
        seed: SEED,
    }
}

#[test]
fn replay_200_cases_is_byte_identical() {
    let cases = build_cases();
    assert_eq!(cases.len(), 200);
    let first = run(&cases);
    let second = run(&cases);
    assert_eq!(first.total, 200);
    assert_eq!(first.mismatched, 0, "{:?}", first.mismatch_cases);
    assert_eq!(first.matched, 200);
    assert_eq!(first.seed, 7);
    assert_eq!(first.mismatch_cases, second.mismatch_cases);
    assert_eq!(first.matched, second.matched);
    assert_eq!(first.mismatched, second.mismatched);
    assert_eq!(first.seed, second.seed);
}

#[test]
fn replay_200_cases_cover_all_reject_reasons() {
    let cases = build_cases();
    let mut reasons = std::collections::BTreeSet::new();
    let mut allows = 0;
    for case in &cases {
        match run_kind(case.kind) {
            GateDecision::Deny { reason, .. } => {
                reasons.insert(format!("{reason:?}"));
            }
            GateDecision::Allow { .. } => {
                allows += 1;
            }
        }
    }
    assert_eq!(reasons.len(), 21);
    assert!(allows >= 1);
    let _ = (GateCounter::GateAllowTotal, GateStage::G5);
    let _ = std::mem::size_of::<GateRejectReason>();
}
#[test]
fn replay_allow_variants_are_explicit() {
    match run_kind(Kind::AllowSkipG3) {
        GateDecision::Allow { per_stage, .. } => {
            assert_eq!(
                per_stage.0[2].result,
                meeting_copilot_domain::model::gate::StageResult::Skipped
            );
        }
        other => panic!("expected allow, got {other:?}"),
    }
    match run_kind(Kind::AllowQueued) {
        GateDecision::Allow {
            outbound: Some(intent),
            ..
        } => assert!(intent.queued),
        other => panic!("expected queued allow, got {other:?}"),
    }
    assert!(matches!(
        run_kind(Kind::AllowOnDemand),
        GateDecision::Allow { .. }
    ));
    assert!(matches!(
        run_kind(Kind::AllowAuto),
        GateDecision::Allow { .. }
    ));
}
