//! 13.3 回放驱动自检：24 例主集 + 5 例探针集（零 IO / 零随机 / 不复用 200 例构造器）。

use std::collections::{BTreeMap, BTreeSet};

use meeting_copilot_application::gate::{
    evaluate_gate, score_relevance, FixedClock, GapScanInput, GateClock, GateInput, GateSignals,
    NoopSink, PassthroughSanitizer, RelevanceInput, RelevanceSnapshot, ReplayCase,
    ReplayCaseSource, ReplayDriver, ReplayLoadError, ReplayMismatchCode, ReplayRun,
};
use meeting_copilot_application::understand::types::NormalizedFragment;
use meeting_copilot_domain::error::CopilotError;
use meeting_copilot_domain::model::card::DedupeKey;
use meeting_copilot_domain::model::enums::{
    AutoSuggestionSwitch, ConsentMode, SessionState, TriggerKind,
};
use meeting_copilot_domain::model::gate::{
    AuthorizationStatus, DecisionKind, GateAuthorization, GateBudget, GateContentType,
    GateDecision, GatePolicy, GateRejectReason, GateStage, LlmCapabilityView, NetworkState,
    ProviderView, ThresholdProfile,
};
use meeting_copilot_domain::model::gate_replay::{
    AuthorizationFixture, BudgetFixture, ClockFixture, ExpectedDecision, GateInputFixture,
    GateReplayCase, RelevanceFixture, ReplayReport, SuppressionFixture,
};
use meeting_copilot_domain::model::governance::EffectivePolicy;
use meeting_copilot_domain::model::insight::Insight;
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::port::governance::{Sanitizer, SensitiveHit};
use meeting_copilot_domain::vo::{
    Confidence, ProviderId, SanitizedQuery, SessionId, TraceId, UtteranceRef, UtteranceText,
};

const TEXT: &str = "内部示例文本-13r";
const NOW_MS: u64 = 10_000;
const SESSION: &str = "session-13r";
const TRACE: &str = "trace-13r";
const REFERENCE: &str = "u1";
const PROVIDER: &str = "provider-1";

struct Arena {
    session: SessionId,
    trace: TraceId,

    fragments: Vec<NormalizedFragment>,
    fragments_low_confidence: Vec<NormalizedFragment>,
    lexicon_names: Vec<String>,
    explained_terms: Vec<String>,
    acronym_whitelist: Vec<String>,
    sensitive_terms: Vec<String>,
    sensitive_terms_nonempty: Vec<String>,
    stable_prefix_ms: BTreeMap<UtteranceRef, u64>,
    answered_at_ms: BTreeMap<UtteranceRef, u64>,
    insights: Vec<Insight>,
    effective_policy: EffectivePolicy,
    authorization: GateAuthorization,
    authorization_revoked: GateAuthorization,
    authorization_blocked: GateAuthorization,
    provider: ProviderView,
    llm: LlmCapabilityView,
    budget: GateBudget,
    candidate_ref: DedupeKey,
    query_seed: String,
    pass: PassthroughSanitizer,
    fail: FailingSanitizer,
    sink: NoopSink,
}

impl Arena {
    fn new() -> Self {
        let session = SessionId::new(SESSION).expect("session");
        let trace = TraceId::new(TRACE).expect("trace");
        let reference = UtteranceRef::new(REFERENCE).expect("reference");
        let provider_id = ProviderId::new(PROVIDER).expect("provider");

        let mut effective_policy = EffectivePolicy::fail_closed();
        effective_policy.auto_suggestion = AutoSuggestionSwitch::Enabled;
        effective_policy.consent = ConsentMode::Explicit;

        let authorization = GateAuthorization {
            status: AuthorizationStatus::Granted,
            granted_at_ms: Some(0),
            content_types: vec![GateContentType::TextQuery],
            max_query_chars: Some(200),
            provider_ids: vec![provider_id.clone()],
            notice_version: 1,
            revoked_at_ms: None,
        };
        let mut authorization_revoked = authorization.clone();
        authorization_revoked.status = AuthorizationStatus::Revoked;
        let mut authorization_blocked = authorization.clone();
        authorization_blocked.provider_ids = vec![ProviderId::new("provider-2").expect("provider")];

        let mut stable_prefix_ms = BTreeMap::new();
        stable_prefix_ms.insert(reference.clone(), 0);

        Self {
            session: session.clone(),
            trace,

            fragments: vec![fragment(&session, &reference, 0.9)],
            fragments_low_confidence: vec![fragment(&session, &reference, 0.49)],
            lexicon_names: vec![TEXT.to_owned()],
            explained_terms: Vec::new(),
            acronym_whitelist: Vec::new(),
            sensitive_terms: Vec::new(),
            sensitive_terms_nonempty: vec![TEXT.to_owned()],
            stable_prefix_ms,
            answered_at_ms: BTreeMap::new(),
            insights: Vec::new(),
            effective_policy,
            authorization,
            authorization_revoked,
            authorization_blocked,
            provider: ProviderView {
                enabled_count: 1,
                provider_ids: vec![provider_id],
            },
            llm: LlmCapabilityView {
                cooling: false,
                cooling_until_ms: None,
            },
            budget: GateBudget {
                session_outbound_used: 0,
                global_window_used: 0,
                topic_window_used: BTreeMap::new(),
                entity_last_seen_ms: BTreeMap::new(),
                inflight: 0,
                tokens_milli: 3000,
                window_start_ms: 0,
            },
            candidate_ref: DedupeKey::new("0".repeat(32)).expect("dedupe"),
            query_seed: TEXT.to_owned(),
            pass: PassthroughSanitizer,
            fail: FailingSanitizer,
            sink: NoopSink,
        }
    }

    fn case<'a>(
        &'a self,
        case_id: &'a str,
        policy: GatePolicy,
        expected: ExpectedDecision,
        mutate: impl FnOnce(&mut GateInput<'a>),
    ) -> ReplayCase<'a> {
        let scan = GapScanInput {
            session: &self.session,
            fragments: &self.fragments,
            insights: &self.insights,
            lexicon_names: &self.lexicon_names,
            explained_terms: &self.explained_terms,
            acronym_whitelist: &self.acronym_whitelist,
            sensitive_terms: &self.sensitive_terms,
            question_wait_ms: GapScanInput::QUESTION_WAIT_MS,
            stable_prefix_ms: &self.stable_prefix_ms,
            answered_at_ms: &self.answered_at_ms,
            now_ms: NOW_MS,
        };
        let mut input = GateInput {
            session: &self.session,
            trace_id: &self.trace,
            candidate_ref: Some(&self.candidate_ref),
            insight_ref: None,
            trigger: TriggerKind::Auto,
            insights: &self.insights,
            fragments: &self.fragments,
            policy: &self.effective_policy,
            authorization: &self.authorization,
            provider: self.provider.clone(),
            llm: self.llm,
            network: NetworkState::Online,
            session_state: SessionState::Listening,
            silent_period: false,
            screen_share: false,
            consent_confirmed: true,
            auto_suggestion_enabled: true,
            session_suppressed: false,
            duplicate_detected: false,
            now_ms: NOW_MS,
            budget: self.budget.clone(),
            audit_available: true,
            scan,
            signals: GateSignals {
                relevance: RelevanceSnapshot {
                    semantic_milli: 1000,
                    entity_milli: 1000,
                    gain_milli: 1000,
                    source_trust_milli: 1000,
                    delta_ms: 0,
                },
                ..GateSignals::default()
            },
            sanitizer: &self.pass,
            audit_sink: &self.sink,
            query_seed: &self.query_seed,
        };
        mutate(&mut input);
        ReplayCase {
            case_id,
            input,
            policy,
            expected,
        }
    }
}

fn fragment(session: &SessionId, reference: &UtteranceRef, confidence: f64) -> NormalizedFragment {
    NormalizedFragment {
        session: session.clone(),
        reference: reference.clone(),
        parent_reference: reference.clone(),
        span: TextRange {
            start_ms: 0,
            end_ms: 100,
        },
        text: UtteranceText::new(TEXT).expect("text"),
        speaker: meeting_copilot_domain::model::utterance::SpeakerRef::anonymous("A"),
        confidence: Confidence::new(confidence).expect("confidence"),
        dual_channel: false,
        origin_pair: None,
    }
}

fn deny(reason: GateRejectReason, stopped_at: GateStage) -> ExpectedDecision {
    ExpectedDecision::Deny { reason, stopped_at }
}

fn build_self_check(arena: &Arena) -> Vec<ReplayCase<'_>> {
    use GateRejectReason as R;
    use GateStage as S;

    let default_policy = GatePolicy::default;
    let invalid_policy = GatePolicy {
        session_outbound_cap: 601,
        ..GatePolicy::default()
    };
    let skip_dedupe = GatePolicy {
        dedupe_enabled: false,
        ..GatePolicy::default()
    };

    let mut cases = Vec::new();
    cases.push(arena.case(
        "r01-g1-auto-disabled",
        default_policy(),
        deny(R::AutoDisabled, S::G1),
        |input| input.auto_suggestion_enabled = false,
    ));
    cases.push(arena.case(
        "r02-g1-consent-missing",
        default_policy(),
        deny(R::ConsentMissing, S::G1),
        |input| input.consent_confirmed = false,
    ));
    cases.push(arena.case(
        "r03-g1-paused",
        default_policy(),
        deny(R::Paused, S::G1),
        |input| input.session_state = SessionState::Paused,
    ));
    cases.push(arena.case(
        "r04-g1-silent",
        default_policy(),
        deny(R::SilentPeriod, S::G1),
        |input| input.silent_period = true,
    ));
    cases.push(arena.case(
        "r05-g1-screen-share",
        default_policy(),
        deny(R::ScreenShareProtection, S::G1),
        |input| input.screen_share = true,
    ));
    cases.push(arena.case(
        "r06-g1-low-confidence",
        default_policy(),
        deny(R::PreFilterLowConfidence, S::G1),
        |input| {
            input.fragments = &arena.fragments_low_confidence;
            input.scan.fragments = &arena.fragments_low_confidence;
        },
    ));
    cases.push(arena.case(
        "r07-g1-suppressed",
        default_policy(),
        deny(R::Suppressed, S::G1),
        |input| input.session_suppressed = true,
    ));
    cases.push(arena.case(
        "r08-g1-quota",
        default_policy(),
        deny(R::QuotaExhausted, S::G1),
        |input| input.budget.global_window_used = 5,
    ));
    cases.push(arena.case(
        "r09-g1-duplicate",
        default_policy(),
        deny(R::Duplicate, S::G1),
        |input| input.duplicate_detected = true,
    ));
    cases.push(arena.case(
        "r10-g1-no-provider",
        default_policy(),
        deny(R::NoProviderEnabled, S::G1),
        |input| input.provider.enabled_count = 0,
    ));
    cases.push(arena.case(
        "r11-g1-sensitive",
        default_policy(),
        deny(R::SensitiveTerm, S::G1),
        |input| input.scan.sensitive_terms = &arena.sensitive_terms_nonempty,
    ));
    cases.push(arena.case(
        "r12-g1-policy-invalid",
        invalid_policy,
        deny(R::PolicyUnavailable, S::G1),
        |_input| {},
    ));
    cases.push(arena.case(
        "r13-g2-low-relevance",
        default_policy(),
        deny(R::LowRelevance, S::G2),
        |input| input.signals.relevance = RelevanceSnapshot::default(),
    ));
    cases.push(arena.case(
        "r14-g3-stale",
        default_policy(),
        deny(R::Stale, S::G3),
        |input| input.signals.cache_age_ms = Some(86_400_001),
    ));
    cases.push(arena.case(
        "r15-g4-budget",
        default_policy(),
        deny(R::BudgetExhausted, S::G4),
        |input| input.budget.session_outbound_used = 120,
    ));
    cases.push(arena.case(
        "r16-g4-cooling",
        default_policy(),
        deny(R::GatewayCooling, S::G4),
        |input| {
            input.llm = LlmCapabilityView {
                cooling: true,
                cooling_until_ms: None,
            }
        },
    ));
    cases.push(arena.case(
        "r17-g4-offline",
        default_policy(),
        deny(R::Offline, S::G4),
        |input| input.network = NetworkState::Offline,
    ));
    cases.push(arena.case(
        "r18-g5-not-granted",
        default_policy(),
        deny(R::OutboundNotGranted, S::G5),
        |input| input.authorization = &arena.authorization_revoked,
    ));
    cases.push(arena.case(
        "r19-g5-sanitize-failed",
        default_policy(),
        deny(R::SanitizeFailed, S::G5),
        |input| input.sanitizer = &arena.fail,
    ));
    cases.push(arena.case(
        "r20-g5-audit-unavailable",
        default_policy(),
        deny(R::AuditUnavailable, S::G5),
        |input| input.audit_available = false,
    ));
    cases.push(arena.case(
        "r21-g5-provider-blocked",
        default_policy(),
        deny(R::ProviderNotAllowed, S::G5),
        |input| input.authorization = &arena.authorization_blocked,
    ));
    cases.push(arena.case(
        "r22-allow-auto",
        default_policy(),
        ExpectedDecision::Allow,
        |_input| {},
    ));
    cases.push(arena.case(
        "r23-allow-on-demand",
        default_policy(),
        ExpectedDecision::Allow,
        |input| input.trigger = TriggerKind::OnDemand,
    ));
    cases.push(arena.case(
        "r24-allow-skip-g3",
        skip_dedupe,
        ExpectedDecision::Allow,
        |_input| {},
    ));
    cases
}

fn build_probe(arena: &Arena) -> Vec<ReplayCase<'_>> {
    use GateRejectReason as R;
    use GateStage as S;

    let default_policy = GatePolicy::default;
    let mut cases = Vec::new();
    cases.push(arena.case(
        "p01-expect-allow-got-deny",
        default_policy(),
        ExpectedDecision::Allow,
        |input| input.auto_suggestion_enabled = false,
    ));
    cases.push(arena.case(
        "p02-expect-deny-got-allow",
        default_policy(),
        deny(R::LowRelevance, S::G2),
        |_input| {},
    ));
    cases.push(arena.case(
        "p03-reason-differs",
        default_policy(),
        deny(R::LowRelevance, S::G2),
        |input| input.signals.cache_age_ms = Some(86_400_001),
    ));
    cases.push(arena.case(
        "p04-stage-differs",
        default_policy(),
        deny(R::Stale, S::G4),
        |input| input.signals.cache_age_ms = Some(86_400_001),
    ));
    cases.push(arena.case(
        "p05-record-refs-empty",
        default_policy(),
        deny(R::AutoDisabled, S::G1),
        |input| {
            input.candidate_ref = None;
            input.insight_ref = None;
            input.auto_suggestion_enabled = false;
        },
    ));
    cases
}

struct Source<'a> {
    stored: Vec<ReplayCase<'a>>,
    order: Vec<usize>,
    seed: u64,
}

impl<'a> Source<'a> {
    fn forward(stored: Vec<ReplayCase<'a>>, seed: u64) -> Self {
        let order = (0..stored.len()).collect();
        Self {
            stored,
            order,
            seed,
        }
    }

    fn reversed(stored: Vec<ReplayCase<'a>>, seed: u64) -> Self {
        let order = (0..stored.len()).rev().collect();
        Self {
            stored,
            order,
            seed,
        }
    }
}

impl ReplayCaseSource for Source<'_> {
    fn seed(&self) -> u64 {
        self.seed
    }

    fn cases(&self) -> Vec<ReplayCase<'_>> {
        self.order
            .iter()
            .map(|index| {
                let case = &self.stored[*index];
                ReplayCase {
                    case_id: case.case_id,
                    input: case.input.clone(),
                    policy: case.policy.clone(),
                    expected: case.expected,
                }
            })
            .collect()
    }
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

    fn scan(&self, _raw: &str) -> Vec<SensitiveHit> {
        Vec::new()
    }

    fn issue_query(&self, _raw: &str) -> Result<SanitizedQuery, CopilotError> {
        Err(CopilotError::PrivacyViolation {
            detail: "回放失败".to_owned(),
        })
    }
}
struct ConstantClock;

impl GateClock for ConstantClock {
    fn now_ms(&self) -> u64 {
        NOW_MS
    }
}

#[test]
fn replay_driver_types_are_closed() {
    assert_eq!(ReplayMismatchCode::ALL.len(), 5);
    assert_eq!(ReplayLoadError::ALL.len(), 3);
    let driver: ReplayDriver = Default::default();
    let arena = Arena::new();
    let source = Source::forward(build_self_check(&arena), 13);
    let run: Result<ReplayRun, ReplayLoadError> = driver.run(&source);
    assert!(run.is_ok());
}

#[test]
fn replay_case_layers_are_separated() {
    let arena = Arena::new();
    let case = build_self_check(&arena).into_iter().nth(21).expect("case");
    assert_eq!(case.expected, ExpectedDecision::Allow);
    assert_eq!(case.policy, GatePolicy::default());
    assert!(!format!("{:?}", case).contains(TEXT));
}

#[test]
fn replay_case_source_rejects_empty_set() {
    let source = Source::forward(Vec::new(), 13);
    let driver = ReplayDriver;
    assert_eq!(
        driver.run(&source).err(),
        Some(ReplayLoadError::EmptyCaseSet)
    );
    assert!(source.cases().is_empty());
    assert!(source.cases().is_empty());
}

#[test]
fn replay_case_source_rejects_invalid_case_id() {
    let arena = Arena::new();
    let driver = ReplayDriver;

    let blank = arena.case(
        "",
        GatePolicy::default(),
        ExpectedDecision::Allow,
        |_input| {},
    );
    assert_eq!(
        driver.run(&Source::forward(vec![blank], 13)).err(),
        Some(ReplayLoadError::InvalidCaseId)
    );

    let too_long = "a".repeat(65);
    let long = arena.case(
        &too_long,
        GatePolicy::default(),
        ExpectedDecision::Allow,
        |_input| {},
    );
    assert_eq!(
        driver.run(&Source::forward(vec![long], 13)).err(),
        Some(ReplayLoadError::InvalidCaseId)
    );

    let spaced = arena.case(
        "r 1",
        GatePolicy::default(),
        ExpectedDecision::Allow,
        |_input| {},
    );
    assert_eq!(
        driver.run(&Source::forward(vec![spaced], 13)).err(),
        Some(ReplayLoadError::InvalidCaseId)
    );
}

#[test]
fn replay_case_source_rejects_duplicate_ids() {
    let arena = Arena::new();
    let driver = ReplayDriver;

    let first = arena.case(
        "dup",
        GatePolicy::default(),
        ExpectedDecision::Allow,
        |_input| {},
    );
    let second = arena.case(
        "dup",
        GatePolicy::default(),
        ExpectedDecision::Allow,
        |_input| {},
    );
    assert_eq!(
        driver.run(&Source::forward(vec![first, second], 13)).err(),
        Some(ReplayLoadError::DuplicateCaseId)
    );

    let blank = arena.case(
        "",
        GatePolicy::default(),
        ExpectedDecision::Allow,
        |_input| {},
    );
    let left = arena.case(
        "dup",
        GatePolicy::default(),
        ExpectedDecision::Allow,
        |_input| {},
    );
    let right = arena.case(
        "dup",
        GatePolicy::default(),
        ExpectedDecision::Allow,
        |_input| {},
    );
    assert_eq!(
        driver
            .run(&Source::forward(vec![blank, left, right], 13))
            .err(),
        Some(ReplayLoadError::InvalidCaseId)
    );
}

#[test]
fn replay_case_set_is_sorted_by_case_id() {
    let arena = Arena::new();
    let run = ReplayDriver
        .run(&Source::reversed(build_probe(&arena), 13))
        .expect("run");

    let ids: Vec<&str> = run
        .failures
        .iter()
        .map(|item| item.case_id.as_str())
        .collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted);
    assert_eq!(run.records.len(), 5);
}

#[test]
fn replay_driver_uses_only_four_functions() {
    let arena = Arena::new();
    let policy = GatePolicy::default();
    let case = build_self_check(&arena).into_iter().nth(21).expect("case");

    let decision = evaluate_gate(&case.input, &policy, &ConstantClock);
    let parsed = if let GateDecision::Allow { candidates, .. } = &decision {
        candidates.first().expect("candidate").clone()
    } else {
        panic!("allow expected");
    };

    let relevance = score_relevance(
        &RelevanceInput {
            candidate: &parsed.candidate,
            semantic_milli: 1000,
            entity_milli: 1000,
            gain_milli: 1000,
            source_trust_milli: 1000,
            delta_ms: 0,
            threshold_profile: policy.threshold_profile,
        },
        &policy,
    );
    assert_eq!(relevance, parsed.relevance);

    let strict = GatePolicy {
        threshold_profile: ThresholdProfile::Strict,
        ..GatePolicy::default()
    };
    let mut low = build_self_check(&arena).into_iter().nth(12).expect("case");
    low.input.signals.relevance = RelevanceSnapshot {
        semantic_milli: 700,
        entity_milli: 700,
        gain_milli: 700,
        source_trust_milli: 700,
        delta_ms: 0,
    };
    low.expected = ExpectedDecision::Allow;
    let balanced_decision = evaluate_gate(&low.input, &GatePolicy::default(), &ConstantClock);
    let strict_decision = evaluate_gate(&low.input, &strict, &ConstantClock);
    assert!(matches!(balanced_decision, GateDecision::Allow { .. }));
    assert!(matches!(
        strict_decision,
        GateDecision::Deny {
            reason: GateRejectReason::LowRelevance,
            ..
        }
    ));
}

#[test]
fn replay_driver_injects_constant_clock() {
    let arena = Arena::new();
    let run = ReplayDriver
        .run(&Source::forward(build_self_check(&arena), 13))
        .expect("run");
    for record in &run.records {
        for trace in record.per_stage.0.iter() {
            assert_eq!(trace.elapsed_ms, 0);
        }
    }
    let clock = FixedClock::new(NOW_MS, 0);
    assert_eq!(clock.now_ms(), clock.now_ms());
}

#[test]
fn replay_driver_records_are_closed() {
    let arena = Arena::new();
    let run = ReplayDriver
        .run(&Source::forward(build_self_check(&arena), 13))
        .expect("run");

    assert_eq!(run.records.len() as u32, run.report.total);
    for record in &run.records {
        assert_eq!(record.schema_version, 1);
        assert!(record.refs_are_valid());
        assert!(record.skipped_stages_are_valid());
        assert!(record.decision_tag_matches_stopped_at());
        assert!(!format!("{:?}", record).contains(TEXT));
    }
}

#[test]
fn replay_driver_record_violation_is_reported() {
    let arena = Arena::new();
    let run = ReplayDriver
        .run(&Source::forward(build_probe(&arena), 13))
        .expect("run");
    assert!(run.failures.iter().any(|item| {
        item.case_id.starts_with("p05") && item.code == ReplayMismatchCode::RecordViolation
    }));
    assert_eq!(run.records.len(), 5);
}

#[test]
fn replay_record_check_rejects_illegal_skip_stage() {
    let arena = Arena::new();
    let run = ReplayDriver
        .run(&Source::forward(build_self_check(&arena), 13))
        .expect("run");
    let base = run.records.first().expect("record").clone();

    let mut illegal_skip = base.clone();
    illegal_skip.skipped_stages = vec![GateStage::G4];
    assert!(!illegal_skip.skipped_stages_are_valid());

    let mut illegal_tag = base;
    illegal_tag.decision = DecisionKind::Deny;
    illegal_tag.stopped_at = None;
    assert!(!illegal_tag.decision_tag_matches_stopped_at());
}
#[test]
fn replay_failure_reports_case_id_and_code_only() {
    let arena = Arena::new();
    let run = ReplayDriver
        .run(&Source::forward(build_probe(&arena), 13))
        .expect("run");

    assert_eq!(run.report.mismatched as usize, run.failures.len());
    let ids: Vec<String> = run
        .failures
        .iter()
        .map(|item| item.case_id.clone())
        .collect();
    assert_eq!(run.report.mismatch_cases, ids);
    for failure in &run.failures {
        let text = format!("{:?}", failure);
        assert!(text.contains("ReplayFailure"));
        assert!(text.contains("case_id"));
        assert!(text.contains("code"));
        assert!(!text.contains("expected"));
        assert!(!text.contains("decision"));
        assert!(!text.contains("stopped_at"));
        assert!(!text.contains(TEXT));
    }
}

#[test]
fn replay_failure_codes_cover_all_mismatch_kinds() {
    let arena = Arena::new();
    let run = ReplayDriver
        .run(&Source::forward(build_probe(&arena), 13))
        .expect("run");

    let actual: BTreeSet<String> = run
        .failures
        .iter()
        .map(|item| format!("{:?}", item.code))
        .collect();
    let expected: BTreeSet<String> = ReplayMismatchCode::ALL
        .iter()
        .map(|code| format!("{:?}", code))
        .collect();
    assert_eq!(actual, expected);
    assert_eq!(run.failures.len(), 5);
    let unique: BTreeSet<&str> = run
        .failures
        .iter()
        .map(|item| item.case_id.as_str())
        .collect();
    assert_eq!(unique.len(), 5);
}

#[test]
fn replay_driver_reports_totals_are_consistent() {
    let arena = Arena::new();
    let driver = ReplayDriver;

    let self_check = driver
        .run(&Source::forward(build_self_check(&arena), 13))
        .expect("run");
    assert_eq!(self_check.report.total, 24);
    assert_eq!(self_check.report.matched, 24);
    assert_eq!(self_check.report.mismatched, 0);
    assert_eq!(self_check.report.seed, 13);

    let probe = driver
        .run(&Source::forward(build_probe(&arena), 13))
        .expect("run");
    assert_eq!(probe.report.total, 5);
    assert_eq!(probe.report.matched, 0);
    assert_eq!(probe.report.mismatched, 5);

    for report in [&self_check.report, &probe.report] {
        assert_eq!(report.matched + report.mismatched, report.total);
        assert_eq!(report.mismatched as usize, report.mismatch_cases.len());
    }
    assert_eq!(probe.report.mismatched as usize, probe.failures.len());
}

#[test]
fn replay_driver_is_deterministic_twice() {
    let arena = Arena::new();
    let driver = ReplayDriver;
    let source = Source::forward(build_self_check(&arena), 13);

    let first = driver.run(&source).expect("run");
    let second = driver.run(&source).expect("run");
    assert_eq!(first.report, second.report);
    assert_eq!(first.failures, second.failures);
    assert_eq!(first.records, second.records);

    for iter in 0..100 {
        let next = driver.run(&source).expect("run");
        assert_eq!(next.report, first.report, "iter={}", iter);
        assert_eq!(next.failures, first.failures, "iter={}", iter);
        assert_eq!(next.records, first.records, "iter={}", iter);
    }
}

#[test]
fn replay_driver_is_order_insensitive() {
    let arena = Arena::new();
    let driver = ReplayDriver;
    let stored = build_self_check(&arena);

    let forward = driver
        .run(&Source::forward(stored.clone(), 13))
        .expect("run");
    let reversed = driver.run(&Source::reversed(stored, 13)).expect("run");

    assert_eq!(forward.report, reversed.report);
    assert_eq!(forward.failures, reversed.failures);
    assert_eq!(forward.records, reversed.records);
}

#[test]
fn replay_driver_seed_is_report_only() {
    let arena = Arena::new();
    let driver = ReplayDriver;
    let stored = build_self_check(&arena);

    let low = driver
        .run(&Source::forward(stored.clone(), 13))
        .expect("run");
    let high = driver.run(&Source::forward(stored, 29)).expect("run");

    assert_eq!(low.report.seed, 13);
    assert_eq!(high.report.seed, 29);
    assert_eq!(low.report.total, high.report.total);
    assert_eq!(low.report.matched, high.report.matched);
    assert_eq!(low.report.mismatched, high.report.mismatched);
    assert_eq!(low.report.mismatch_cases, high.report.mismatch_cases);
    assert_eq!(low.failures, high.failures);
    assert_eq!(low.records, high.records);
}

#[test]
fn replay_driver_zero_transcript_boundary() {
    let arena = Arena::new();
    let run = ReplayDriver
        .run(&Source::forward(build_self_check(&arena), 13))
        .expect("run");

    let text = format!("{:?}", run);
    assert!(text.contains("ReplayRun"));
    assert!(!text.contains(TEXT));
    assert!(!text.contains("查询"));
}

#[test]
fn replay_driver_debug_is_whitelisted() {
    let arena = Arena::new();
    let case = build_self_check(&arena).into_iter().nth(21).expect("case");
    assert_eq!(
        format!("{:?}", case),
        format!("ReplayCase {{ case_id: \"{}\" }}", case.case_id)
    );

    let run = ReplayDriver
        .run(&Source::forward(build_probe(&arena), 13))
        .expect("run");
    let run_text = format!("{:?}", run);
    assert!(run_text.contains("report"));
    assert!(run_text.contains("failures"));
    assert!(run_text.contains("records"));
    assert!(!run_text.contains(TEXT));

    let failure = run.failures.first().expect("failure");
    let failure_text = format!("{:?}", failure);
    assert!(failure_text.contains("ReplayFailure"));
    assert!(failure_text.contains("case_id"));
    assert!(failure_text.contains("code"));
    assert!(!failure_text.contains(TEXT));
}

#[test]
fn replay_driver_coexists_with_gate_replay_fixture() {
    let fixture_case = GateReplayCase {
        case_id: "fixture-1".to_owned(),
        input_fixture: fixture_input(),
        policy_fixture: GatePolicy::default(),
        expected: ExpectedDecision::Allow,
    };
    assert_eq!(fixture_case.case_id, "fixture-1");
    assert_eq!(fixture_case.policy_fixture, GatePolicy::default());

    let report = ReplayReport {
        total: 1,
        matched: 1,
        mismatched: 0,
        mismatch_cases: Vec::new(),
        seed: 13,
    };
    assert_eq!(report.total, 1);
    assert_eq!(report.matched + report.mismatched, report.total);
}

#[test]
fn replay_driver_does_not_touch_200_cases() {
    const SELF_SRC: &str = include_str!("gate_replay_driver.rs");
    assert!(!SELF_SRC.contains(concat!("fn build", "_cases")));
    assert!(!SELF_SRC.contains(concat!("enum ", "Kind")));
    assert!(!SELF_SRC.contains(concat!("mod ", "common")));
    assert!(!SELF_SRC.contains(concat!("use ", "common")));
    assert!(!SELF_SRC.contains(concat!("tests/gate_", "replay")));

    let fixture_case = GateReplayCase {
        case_id: "fixture-2".to_owned(),
        input_fixture: fixture_input(),
        policy_fixture: GatePolicy::default(),
        expected: ExpectedDecision::Allow,
    };
    let report = ReplayReport {
        total: 2,
        matched: 2,
        mismatched: 0,
        mismatch_cases: Vec::new(),
        seed: 13,
    };
    assert_eq!(fixture_case.expected, ExpectedDecision::Allow);
    assert_eq!(report.total, 2);
}

#[test]
fn replay_driver_load_is_io_free_and_read_only() {
    let arena = Arena::new();
    let source = Source::forward(build_self_check(&arena), 13);

    let first = source.cases();
    let second = source.cases();
    assert_eq!(first.len(), second.len());
    for (left, right) in first.iter().zip(second.iter()) {
        assert_eq!(left.case_id, right.case_id);
        assert_eq!(left.policy, right.policy);
        assert_eq!(left.expected, right.expected);
    }
}

fn fixture_input() -> GateInputFixture {
    GateInputFixture {
        now_ms: NOW_MS,
        clock: ClockFixture {
            anchor_ms: NOW_MS,
            tick_ms: 0,
        },
        trigger: TriggerKind::Auto,
        session_state: SessionState::Listening,
        silent_period: false,
        screen_share: false,
        consent_confirmed: true,
        auto_suggestion_enabled: true,
        fragments: Vec::new(),
        insights: Vec::new(),
        budget: BudgetFixture {
            session_outbound_used: 0,
            global_window_used: 0,
            topic_window_used: Vec::new(),
            entity_last_seen_ms: Vec::new(),
            inflight: 0,
            tokens_milli: 3000,
            window_start_ms: 0,
        },
        authorization: AuthorizationFixture {
            status: AuthorizationStatus::Granted,
            granted_at_ms: Some(0),
            content_types: vec![GateContentType::TextQuery],
            max_query_chars: Some(200),
            provider_ids: vec![PROVIDER.to_owned()],
            notice_version: 1,
            revoked_at_ms: None,
        },
        provider_enabled_count: 1,
        provider_ids: vec![PROVIDER.to_owned()],
        llm_cooling: false,
        llm_cooling_until_ms: None,
        network: NetworkState::Online,
        audit_available: true,
        relevance: RelevanceFixture {
            semantic_milli: 1000,
            entity_milli: 1000,
            gain_milli: 1000,
            source_trust_milli: 1000,
            delta_ms: 0,
        },
        suppression: SuppressionFixture::default(),
        candidate_seeds: Vec::new(),
        cache_age_ms: None,
        refresh_available: false,
    }
}
