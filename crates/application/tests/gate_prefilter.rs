//! G1 预筛与门控输入契约（AC-B4-01~08）。

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use meeting_copilot_application::gate::{
    detect_gaps, detect_gaps_with_counters, evaluate_gate, FixedClock, GapScanInput, GateClock,
    GateInput, GateSignals, GateSnapshot, NoopSink, PassthroughSanitizer,
};
use meeting_copilot_application::understand::types::NormalizedFragment;
use meeting_copilot_domain::model::card::GapCandidate;
use meeting_copilot_domain::model::enums::{
    AutoSuggestionSwitch, ConsentMode, GapKind, SessionState, TriggerKind,
};
use meeting_copilot_domain::model::gate::{
    AuthorizationStatus, GateAuthorization, GateBudget, GateCounter, GateCounters, GateDecision,
    GatePolicy, GateRejectReason, GateStage, LlmCapabilityView, NetworkState, PerStageResults,
    ProviderView, StageResult,
};
use meeting_copilot_domain::model::gate_replay::ClockFixture;
use meeting_copilot_domain::model::governance::EffectivePolicy;
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::model::utterance::SpeakerRef;
use meeting_copilot_domain::vo::{Confidence, SessionId, TraceId, UtteranceRef};

static PASS_SANITIZER: PassthroughSanitizer = PassthroughSanitizer;
static NOOP_SINK: NoopSink = NoopSink;

struct TestClock {
    now_ms: AtomicU64,
}

impl TestClock {
    fn new(now_ms: u64) -> Self {
        Self {
            now_ms: AtomicU64::new(now_ms),
        }
    }
}

impl GateClock for TestClock {
    fn now_ms(&self) -> u64 {
        self.now_ms.load(Ordering::SeqCst)
    }
}

struct Harness {
    session: SessionId,
    trace: TraceId,
    policy: EffectivePolicy,
    authorization: GateAuthorization,
    fragments: Vec<NormalizedFragment>,
    lexicon: Vec<String>,
    explained: Vec<String>,
    whitelist: Vec<String>,
    sensitive: Vec<String>,
    stable: BTreeMap<UtteranceRef, u64>,
    answered: BTreeMap<UtteranceRef, u64>,
    now_ms: u64,
    session_state: SessionState,
    silent_period: bool,
    screen_share: bool,
    consent_confirmed: bool,
    auto_suggestion_enabled: bool,
    session_suppressed: bool,
    duplicate_detected: bool,
    provider_enabled: u8,
    global_used: u8,
    outbound_used: u16,
}

impl Harness {
    fn new(now_ms: u64) -> Self {
        let mut policy = EffectivePolicy::fail_closed();
        policy.auto_suggestion = AutoSuggestionSwitch::Enabled;
        policy.consent = ConsentMode::Explicit;
        policy.screen_share_protection_default = true;
        Self {
            session: SessionId::new("session-1").expect("session"),
            trace: TraceId::new("trace-1").expect("trace"),
            policy,
            authorization: GateAuthorization {
                status: AuthorizationStatus::Granted,
                granted_at_ms: Some(0),
                content_types: vec![
                    meeting_copilot_domain::model::gate::GateContentType::TextQuery,
                ],
                max_query_chars: Some(200),
                provider_ids: Vec::new(),
                notice_version: 1,
                revoked_at_ms: None,
            },
            fragments: Vec::new(),
            lexicon: Vec::new(),
            explained: Vec::new(),
            whitelist: Vec::new(),
            sensitive: Vec::new(),
            stable: BTreeMap::new(),
            answered: BTreeMap::new(),
            now_ms,
            session_state: SessionState::Listening,
            silent_period: false,
            screen_share: false,
            consent_confirmed: true,
            auto_suggestion_enabled: true,
            session_suppressed: false,
            duplicate_detected: false,
            provider_enabled: 1,
            global_used: 0,
            outbound_used: 0,
        }
    }

    fn push(&mut self, text: &str, confidence: f64, start_ms: u64, end_ms: u64) -> UtteranceRef {
        let reference = UtteranceRef::new(format!("u{}", self.fragments.len() + 1)).expect("ref");
        self.fragments.push(NormalizedFragment {
            session: self.session.clone(),
            reference: reference.clone(),
            parent_reference: reference.clone(),
            span: TextRange { start_ms, end_ms },
            text: meeting_copilot_domain::vo::UtteranceText::new(text).expect("text"),
            speaker: SpeakerRef::anonymous("A"),
            confidence: Confidence::new(confidence).expect("confidence"),
            dual_channel: false,
            origin_pair: None,
        });
        self.stable.insert(reference.clone(), 0);
        reference
    }

    fn scan(&self) -> GapScanInput<'_> {
        GapScanInput {
            session: &self.session,
            fragments: &self.fragments,
            insights: &[],
            lexicon_names: &self.lexicon,
            explained_terms: &self.explained,
            acronym_whitelist: &self.whitelist,
            sensitive_terms: &self.sensitive,
            question_wait_ms: GapScanInput::QUESTION_WAIT_MS,
            stable_prefix_ms: &self.stable,
            answered_at_ms: &self.answered,
            now_ms: self.now_ms,
        }
    }

    fn input(&self) -> GateInput<'_> {
        GateInput {
            session: &self.session,
            trace_id: &self.trace,
            candidate_ref: None,
            insight_ref: None,
            trigger: TriggerKind::Auto,
            insights: &[],
            fragments: &self.fragments,
            policy: &self.policy,
            authorization: &self.authorization,
            provider: ProviderView {
                enabled_count: self.provider_enabled,
                provider_ids: Vec::new(),
            },
            llm: LlmCapabilityView {
                cooling: false,
                cooling_until_ms: None,
            },
            network: NetworkState::Online,
            session_state: self.session_state,
            silent_period: self.silent_period,
            screen_share: self.screen_share,
            consent_confirmed: self.consent_confirmed,
            auto_suggestion_enabled: self.auto_suggestion_enabled,
            session_suppressed: self.session_suppressed,
            duplicate_detected: self.duplicate_detected,
            now_ms: self.now_ms,
            budget: GateBudget {
                session_outbound_used: self.outbound_used,
                global_window_used: self.global_used,
                topic_window_used: BTreeMap::new(),
                entity_last_seen_ms: BTreeMap::new(),
                inflight: 0,
                tokens_milli: 0,
                window_start_ms: 0,
            },
            audit_available: true,
            scan: self.scan(),
            signals: GateSignals::default(),
            sanitizer: &PASS_SANITIZER,
            audit_sink: &NOOP_SINK,
            query_seed: "默认查询",
        }
    }

    fn decision(&self) -> GateDecision {
        evaluate_gate(
            &self.input(),
            &GatePolicy::default(),
            &TestClock::new(self.now_ms),
        )
    }
}

fn candidates(scan: &GapScanInput<'_>) -> Vec<GapCandidate> {
    detect_gaps(scan)
}

fn assert_deny(decision: &GateDecision, reason: GateRejectReason) {
    match decision {
        GateDecision::Deny {
            reason: actual,
            stopped_at,
            per_stage,
            cooldown_until_ms,
        } => {
            assert_eq!(*actual, reason);
            assert_eq!(*stopped_at, GateStage::G1);
            assert!(cooldown_until_ms.is_none());
            assert_eq!(per_stage.0[0].calls, 1);
            assert_eq!(per_stage.0[0].result, StageResult::Deny);
            assert_eq!(per_stage.0[0].reason, Some(reason));
            for trace in &per_stage.0[1..] {
                assert_eq!(trace.calls, 0);
                assert_eq!(trace.result, StageResult::NotReached);
            }
        }
        other => panic!("expected Deny {reason:?}, got {other:?}"),
    }
}

#[test]
fn gate_input_has_no_insight_draft() {
    let harness = Harness::new(10_000);
    let input = harness.input();
    assert!(input.insights.is_empty());
    assert_eq!(input.fragments.len(), 0);
}

#[test]
fn gate_input_is_single_source() {
    let harness = Harness::new(10_000);
    let snapshot = GateSnapshot {
        session: &harness.session,
        trace_id: &harness.trace,
        candidate_ref: None,
        insight_ref: None,
        trigger: TriggerKind::Auto,
        insights: &[],
        fragments: &harness.fragments,
        policy: &harness.policy,
        authorization: &harness.authorization,
        provider: ProviderView {
            enabled_count: 1,
            provider_ids: Vec::new(),
        },
        llm: LlmCapabilityView {
            cooling: false,
            cooling_until_ms: None,
        },
        network: NetworkState::Online,
        session_state: SessionState::Listening,
        silent_period: false,
        screen_share: false,
        consent_confirmed: true,
        auto_suggestion_enabled: true,
        session_suppressed: false,
        duplicate_detected: false,
        now_ms: 10_000,
        budget: GateBudget {
            session_outbound_used: 0,
            global_window_used: 0,
            topic_window_used: BTreeMap::new(),
            entity_last_seen_ms: BTreeMap::new(),
            inflight: 0,
            tokens_milli: 0,
            window_start_ms: 0,
        },
        audit_available: true,
        scan: harness.scan(),
        signals: GateSignals::default(),
        sanitizer: &PASS_SANITIZER,
        audit_sink: &NOOP_SINK,
        query_seed: "默认查询",
    };
    let input = snapshot.into_input();
    assert!(std::ptr::eq(input.fragments, input.scan.fragments));
    assert!(std::ptr::eq(input.insights, input.scan.insights));
}

#[test]
fn prefilter_scores_six_gap_rules() {
    let mut harness = Harness::new(10_000);
    harness.lexicon.push("张伟".to_owned());
    harness.push("张伟负责该模块。", 0.9, 100, 200);
    harness.push("本次讨论 OKR 指标。", 0.9, 300, 400);
    harness.push("中国科学院参与项目。", 0.9, 500, 600);
    harness.push("依据 GB/T 12345 执行。", 0.9, 700, 800);
    harness.push("转化率提升 30%。", 0.9, 900, 1_000);
    harness.push("这个参数怎么设置？", 0.9, 0, 0);

    let scan = harness.scan();
    let found = candidates(&scan);
    assert_eq!(found.len(), 6);
    let scores: Vec<f64> = found.iter().map(|item| item.confidence.value()).collect();
    assert_eq!(scores, vec![0.85, 0.8, 0.75, 0.7, 0.65, 0.6]);
    let kinds: Vec<GapKind> = found.iter().map(|item| item.kind).collect();
    assert!(kinds.contains(&GapKind::UnansweredQuestion));
    assert!(kinds.contains(&GapKind::PolicyStandard));
    assert!(kinds.contains(&GapKind::Metric));
    assert!(kinds.contains(&GapKind::ProperNoun));
    assert!(kinds.contains(&GapKind::Abbreviation));
}

#[test]
fn acronym_whitelist_is_configurable() {
    let mut harness = Harness::new(10_000);
    harness.push("本次讨论 OKR 指标。", 0.9, 0, 100);
    let scan = harness.scan();
    assert_eq!(candidates(&scan).len(), 1);

    harness.whitelist.push("OKR".to_owned());
    let scan = harness.scan();
    assert_eq!(candidates(&scan).len(), 0);
}

#[test]
fn first_seen_suppresses_second_candidate() {
    let mut harness = Harness::new(10_000);
    harness.lexicon.push("张伟".to_owned());
    harness.push("张伟负责该模块。", 0.9, 0, 100);
    harness.push("张伟再次说明。", 0.9, 200, 300);
    let scan = harness.scan();
    assert_eq!(candidates(&scan).len(), 1);

    let mut empty_lexicon = Harness::new(10_000);
    empty_lexicon.push("李四负责该模块。", 0.9, 0, 100);
    let scan = empty_lexicon.scan();
    assert_eq!(candidates(&scan).len(), 0);
}

#[test]
fn sensitive_term_is_not_a_seed() {
    let mut harness = Harness::new(10_000);
    harness.lexicon.push("机密项目".to_owned());
    harness.sensitive.push("机密项目".to_owned());
    harness.push("机密项目讨论。", 0.9, 0, 100);
    let scan = harness.scan();
    let mut counters = GateCounters::default();
    let found = detect_gaps_with_counters(&scan, &mut counters);
    assert_eq!(found.len(), 0);
    assert_eq!(counters.get(GateCounter::GateL0SensitiveSeedBlocked), 1);
}

#[test]
fn unanswered_question_yields_after_ten_seconds() {
    let mut harness = Harness::new(9_999);
    harness.push("这个参数怎么设置？", 0.9, 0, 0);
    let scan = harness.scan();
    assert_eq!(candidates(&scan).len(), 0);

    harness.now_ms = 10_000;
    let scan = harness.scan();
    assert_eq!(candidates(&scan).len(), 1);
}

#[test]
fn answered_question_is_skipped() {
    let mut harness = Harness::new(10_000);
    let reference = harness.push("这个参数怎么设置？", 0.9, 0, 0);
    harness.answered.insert(reference, 5_000);
    let scan = harness.scan();
    let mut counters = GateCounters::default();
    let found = detect_gaps_with_counters(&scan, &mut counters);
    assert_eq!(found.len(), 0);
    assert_eq!(counters.get(GateCounter::GateL0QuestionAnsweredSkipped), 1);
}

#[test]
fn below_half_gap_confidence_yields_no_candidate() {
    let mut harness = Harness::new(10_000);
    harness.lexicon.push("张伟".to_owned());
    harness.push("张伟负责该模块。", 0.49, 0, 100);
    let scan = harness.scan();
    let mut counters = GateCounters::default();
    let found = detect_gaps_with_counters(&scan, &mut counters);
    assert_eq!(found.len(), 0);
    assert_eq!(counters.get(GateCounter::GateL0LowConfidence), 1);
}

#[test]
fn exactly_half_gap_confidence_passes() {
    let mut harness = Harness::new(10_000);
    harness.lexicon.push("张伟".to_owned());
    harness.push("张伟负责该模块。", 0.5, 0, 100);
    let scan = harness.scan();
    assert_eq!(candidates(&scan).len(), 1);
}

#[test]
fn candidate_is_ready_within_two_seconds_of_stable_prefix() {
    let mut harness = Harness::new(1_999);
    harness.lexicon.push("张伟".to_owned());
    let reference = harness.push("张伟负责该模块。", 0.9, 0, 0);
    harness.stable.insert(reference.clone(), 0);
    let scan = harness.scan();
    assert_eq!(candidates(&scan).len(), 0);

    harness.now_ms = 2_000;
    harness.stable.insert(reference, 0);
    let scan = harness.scan();
    assert_eq!(candidates(&scan).len(), 1);
}

#[test]
fn gap_kind_is_closed_enum_and_institution_maps_to_proper_noun() {
    let variants = [
        GapKind::ProperNoun,
        GapKind::Abbreviation,
        GapKind::PolicyStandard,
        GapKind::Metric,
        GapKind::UnansweredQuestion,
        GapKind::Acronym,
    ];
    assert_eq!(variants.len(), 6);
    let mut harness = Harness::new(10_000);
    harness.push("中国科学院参与项目。", 0.9, 0, 100);
    let scan = harness.scan();
    let found = candidates(&scan);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, GapKind::ProperNoun);
}

#[test]
fn dedupe_key_is_32_hex() {
    let mut harness = Harness::new(10_000);
    harness.push("转化率提升 30%。", 0.9, 0, 100);
    let scan = harness.scan();
    let found = candidates(&scan);
    assert_eq!(found.len(), 1);
    let key = found[0].dedupe_key.as_str();
    assert_eq!(key.len(), 32);
    assert!(key.chars().all(|character| character.is_ascii_hexdigit()));
    assert!(found[0].trigger_span.start_ms < found[0].trigger_span.end_ms);
}

#[test]
fn prefilter_is_pure_and_deterministic() {
    let mut harness = Harness::new(10_000);
    harness.lexicon.push("张伟".to_owned());
    harness.push("张伟负责该模块，依据 GB/T 12345，提升 30%。", 0.9, 0, 100);
    let first = candidates(&harness.scan());
    for _ in 0..100 {
        assert_eq!(candidates(&harness.scan()), first);
    }
}

#[test]
fn invalid_policy_fails_closed() {
    let harness = Harness::new(10_000);
    let policy = GatePolicy {
        session_outbound_cap: 601,
        ..GatePolicy::default()
    };
    let decision = evaluate_gate(&harness.input(), &policy, &TestClock::new(10_000));
    match decision {
        GateDecision::Deny {
            reason,
            stopped_at,
            per_stage,
            ..
        } => {
            assert_eq!(reason, GateRejectReason::PolicyUnavailable);
            assert_eq!(stopped_at, GateStage::G1);
            assert_eq!(per_stage, PerStageResults::empty());
        }
        other => panic!("expected policy deny, got {other:?}"),
    }
}

#[test]
fn hard_gate_priority_returns_first_only() {
    let mut harness = Harness::new(10_000);
    harness.push("张伟负责该模块。", 0.9, 0, 100);
    harness.auto_suggestion_enabled = false;
    harness.consent_confirmed = false;
    assert_deny(&harness.decision(), GateRejectReason::AutoDisabled);

    harness.auto_suggestion_enabled = true;
    harness.consent_confirmed = true;
    harness.session_state = SessionState::Paused;
    harness.silent_period = true;
    assert_deny(&harness.decision(), GateRejectReason::Paused);
}

#[test]
fn prefilter_denies_eleven_hard_cases() {
    let mut reasons: Vec<GateRejectReason> = Vec::new();
    let cases: [(&str, GateRejectReason); 11] = [
        ("auto", GateRejectReason::AutoDisabled),
        ("consent", GateRejectReason::ConsentMissing),
        ("paused", GateRejectReason::Paused),
        ("silent", GateRejectReason::SilentPeriod),
        ("screen", GateRejectReason::ScreenShareProtection),
        ("low", GateRejectReason::PreFilterLowConfidence),
        ("suppressed", GateRejectReason::Suppressed),
        ("quota", GateRejectReason::QuotaExhausted),
        ("duplicate", GateRejectReason::Duplicate),
        ("no_provider", GateRejectReason::NoProviderEnabled),
        ("sensitive", GateRejectReason::SensitiveTerm),
    ];
    for (name, expected) in cases {
        let mut harness = Harness::new(10_000);
        harness.lexicon.push("张伟".to_owned());
        harness.push("张伟负责该模块。", 0.9, 0, 100);
        match name {
            "auto" => harness.auto_suggestion_enabled = false,
            "consent" => harness.consent_confirmed = false,
            "paused" => harness.session_state = SessionState::Paused,
            "silent" => harness.silent_period = true,
            "screen" => harness.screen_share = true,
            "low" => {
                harness.fragments.clear();
                harness.push("张伟负责该模块。", 0.49, 0, 100);
            }
            "suppressed" => harness.session_suppressed = true,
            "quota" => harness.global_used = 5,
            "duplicate" => harness.duplicate_detected = true,
            "no_provider" => harness.provider_enabled = 0,
            "sensitive" => {
                harness.sensitive.push("张伟".to_owned());
            }
            _ => unreachable!(),
        }
        let decision = harness.decision();
        assert_deny(&decision, expected);
        reasons.push(expected);
    }
    assert_eq!(reasons.len(), 11);
}

#[test]
fn deny_short_circuits_later_stages() {
    let mut harness = Harness::new(10_000);
    harness.push("张伟负责该模块。", 0.9, 0, 100);
    harness.session_state = SessionState::Paused;
    let decision = harness.decision();
    assert_deny(&decision, GateRejectReason::Paused);
    if let GateDecision::Deny { per_stage, .. } = decision {
        assert_eq!(per_stage.0.len(), 5);
        for trace in &per_stage.0[1..] {
            assert_eq!(trace.calls, 0);
            assert_eq!(trace.result, StageResult::NotReached);
        }
    }
}

#[test]
fn zero_candidate_path_allow_has_no_outbound() {
    let harness = Harness::new(10_000);
    let decision = harness.decision();
    match decision {
        GateDecision::Allow {
            candidates,
            outbound,
            stopped_at,
            per_stage,
            ..
        } => {
            assert!(candidates.is_empty());
            assert!(outbound.is_none());
            assert_eq!(stopped_at, Some(GateStage::G1));
            assert_eq!(per_stage.0[0].calls, 1);
            assert_eq!(per_stage.0[0].result, StageResult::NoCandidate);
            for trace in &per_stage.0[1..] {
                assert_eq!(trace.calls, 0);
                assert_eq!(trace.result, StageResult::NotReached);
            }
        }
        other => panic!("expected allow, got {other:?}"),
    }
}

#[test]
fn clock_fixture_matches_fixed_clock_sequence() {
    let fixture = ClockFixture {
        anchor_ms: 1_000,
        tick_ms: 250,
    };
    let clock = FixedClock::new(fixture.anchor_ms, fixture.tick_ms);
    let sequence: Vec<u64> = (0..3).map(|_| clock.now_ms()).collect();
    assert_eq!(sequence, vec![1_000, 1_250, 1_500]);
}
