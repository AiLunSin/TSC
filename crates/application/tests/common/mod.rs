//! P2 三个 target 共用的最小门控夹具。

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use meeting_copilot_application::gate::{
    evaluate_gate_with_counters, GateClock, GateInput, GateOutcome, GateSignals, RelevanceSnapshot,
};
use meeting_copilot_application::understand::types::NormalizedFragment;
use meeting_copilot_domain::model::enums::{
    AutoSuggestionSwitch, ConsentMode, SessionState, TriggerKind,
};
use meeting_copilot_domain::model::gate::{
    AuthorizationStatus, GateAuthorization, GateBudget, GateContentType, GatePolicy,
    LlmCapabilityView, NetworkState, ProviderView,
};
use meeting_copilot_domain::model::governance::EffectivePolicy;
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::model::utterance::SpeakerRef;
use meeting_copilot_domain::vo::{Confidence, SessionId, TraceId, UtteranceRef, UtteranceText};

pub struct TestClock {
    now_ms: AtomicU64,
}

impl TestClock {
    pub fn new(now_ms: u64) -> Self {
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

pub struct Harness {
    pub session: SessionId,
    pub trace: TraceId,
    pub policy: EffectivePolicy,
    pub authorization: GateAuthorization,
    pub fragments: Vec<NormalizedFragment>,
    pub lexicon: Vec<String>,
    pub whitelist: Vec<String>,
    pub sensitive: Vec<String>,
    pub stable: BTreeMap<UtteranceRef, u64>,
    pub answered: BTreeMap<UtteranceRef, u64>,
    pub now_ms: u64,
    pub session_state: SessionState,
    pub silent_period: bool,
    pub screen_share: bool,
    pub consent_confirmed: bool,
    pub auto_suggestion_enabled: bool,
    pub session_suppressed: bool,
    pub duplicate_detected: bool,
    pub provider_enabled: u8,
    pub budget: GateBudget,
    pub relevance: RelevanceSnapshot,
    pub similarity_milli: Option<u16>,
    pub duplicate_last_seen_ms: Option<u64>,
    pub cluster_id: u32,
    pub entity_keys: Vec<String>,
    pub suppression_topics: Vec<u32>,
    pub suppression_providers: Vec<String>,
    pub suppression_entities: Vec<String>,
    pub cache_age_ms: Option<u64>,
    pub refresh_available: bool,
    pub trigger: TriggerKind,
    pub llm_cooling: bool,
    pub llm_cooling_until_ms: Option<u64>,
    pub network: NetworkState,
}

impl Harness {
    pub fn new(now_ms: u64) -> Self {
        let mut policy = EffectivePolicy::fail_closed();
        policy.auto_suggestion = AutoSuggestionSwitch::Enabled;
        policy.consent = ConsentMode::Explicit;
        policy.screen_share_protection_default = true;
        Self {
            session: SessionId::new("session-p2").expect("session"),
            trace: TraceId::new("trace-p2").expect("trace"),
            policy,
            authorization: GateAuthorization {
                status: AuthorizationStatus::Granted,
                granted_at_ms: Some(0),
                content_types: vec![GateContentType::TextQuery],
                max_query_chars: Some(200),
                provider_ids: Vec::new(),
                notice_version: 1,
                revoked_at_ms: None,
            },
            fragments: Vec::new(),
            lexicon: Vec::new(),
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
            budget: GateBudget {
                session_outbound_used: 0,
                global_window_used: 0,
                topic_window_used: BTreeMap::new(),
                entity_last_seen_ms: BTreeMap::new(),
                inflight: 0,
                tokens_milli: 3000,
                window_start_ms: 0,
            },
            relevance: RelevanceSnapshot::default(),
            similarity_milli: None,
            duplicate_last_seen_ms: None,
            cluster_id: 1,
            entity_keys: Vec::new(),
            suppression_topics: Vec::new(),
            suppression_providers: Vec::new(),
            suppression_entities: Vec::new(),
            cache_age_ms: None,
            refresh_available: false,
            trigger: TriggerKind::Auto,
            llm_cooling: false,
            llm_cooling_until_ms: None,
            network: NetworkState::Online,
        }
    }

    pub fn push(&mut self, text: &str, confidence: f64, start_ms: u64, end_ms: u64) {
        let reference = UtteranceRef::new(format!("u{}", self.fragments.len() + 1)).expect("ref");
        self.fragments.push(NormalizedFragment {
            session: self.session.clone(),
            reference: reference.clone(),
            parent_reference: reference.clone(),
            span: TextRange { start_ms, end_ms },
            text: UtteranceText::new(text).expect("text"),
            speaker: SpeakerRef::anonymous("A"),
            confidence: Confidence::new(confidence).expect("confidence"),
            dual_channel: false,
            origin_pair: None,
        });
        self.stable.insert(reference, 0);
    }

    pub fn signals(&self) -> GateSignals<'_> {
        GateSignals {
            relevance: self.relevance,
            similarity_milli: self.similarity_milli,
            duplicate_last_seen_ms: self.duplicate_last_seen_ms,
            cluster_id: self.cluster_id,
            entity_keys: &self.entity_keys,
            suppression_topics: &self.suppression_topics,
            suppression_providers: &self.suppression_providers,
            suppression_entities: &self.suppression_entities,
            cache_age_ms: self.cache_age_ms,
            refresh_available: self.refresh_available,
        }
    }

    pub fn input(&self) -> GateInput<'_> {
        GateInput {
            session: &self.session,
            trace_id: &self.trace,
            candidate_ref: None,
            insight_ref: None,
            trigger: self.trigger,
            insights: &[],
            fragments: &self.fragments,
            policy: &self.policy,
            authorization: &self.authorization,
            provider: ProviderView {
                enabled_count: self.provider_enabled,
                provider_ids: Vec::new(),
            },
            llm: LlmCapabilityView {
                cooling: self.llm_cooling,
                cooling_until_ms: self.llm_cooling_until_ms,
            },
            network: self.network,
            session_state: self.session_state,
            silent_period: self.silent_period,
            screen_share: self.screen_share,
            consent_confirmed: self.consent_confirmed,
            auto_suggestion_enabled: self.auto_suggestion_enabled,
            session_suppressed: self.session_suppressed,
            duplicate_detected: self.duplicate_detected,
            now_ms: self.now_ms,
            budget: self.budget.clone(),
            audit_available: true,
            scan: self.scan(),
            signals: self.signals(),
        }
    }

    pub fn scan(&self) -> meeting_copilot_application::gate::GapScanInput<'_> {
        meeting_copilot_application::gate::GapScanInput {
            session: &self.session,
            fragments: &self.fragments,
            insights: &[],
            lexicon_names: &self.lexicon,
            explained_terms: &[],
            acronym_whitelist: &self.whitelist,
            sensitive_terms: &self.sensitive,
            question_wait_ms: 10_000,
            stable_prefix_ms: &self.stable,
            answered_at_ms: &self.answered,
            now_ms: self.now_ms,
        }
    }

    pub fn outcome(&self) -> GateOutcome {
        evaluate_gate_with_counters(
            &self.input(),
            &GatePolicy::default(),
            &TestClock::new(self.now_ms),
        )
    }

    #[allow(dead_code)]
    pub fn outcome_with(&self, policy: &GatePolicy) -> GateOutcome {
        evaluate_gate_with_counters(&self.input(), policy, &TestClock::new(self.now_ms))
    }
}
