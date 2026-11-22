//! 门控输入：一次判定所需的只读快照与同源子视图。

use std::collections::BTreeMap;

use meeting_copilot_domain::model::card::GapCandidate;
use meeting_copilot_domain::model::enums::{SessionState, TriggerKind};
use meeting_copilot_domain::model::gate::{
    GateAuthorization, GateBudget, LlmCapabilityView, NetworkState, ProviderView, ThresholdProfile,
};
use meeting_copilot_domain::model::governance::EffectivePolicy;
use meeting_copilot_domain::model::insight::Insight;
use meeting_copilot_domain::vo::{InsightRef, SessionId, TraceId, UtteranceRef};

use crate::understand::types::NormalizedFragment;

/// 装配方一次性注入的只读快照（与 `GateInput` 同源构造的唯一入口）。
#[derive(Debug, Clone)]
pub struct GateSnapshot<'a> {
    pub session: &'a SessionId,
    pub trace_id: &'a TraceId,
    pub candidate_ref: Option<&'a meeting_copilot_domain::model::card::DedupeKey>,
    pub insight_ref: Option<&'a InsightRef>,
    pub trigger: TriggerKind,
    pub insights: &'a [Insight],
    pub fragments: &'a [NormalizedFragment],
    pub policy: &'a EffectivePolicy,
    pub authorization: &'a GateAuthorization,
    pub provider: ProviderView,
    pub llm: LlmCapabilityView,
    pub network: NetworkState,
    pub session_state: SessionState,
    pub silent_period: bool,
    pub screen_share: bool,
    pub consent_confirmed: bool,
    pub auto_suggestion_enabled: bool,
    pub session_suppressed: bool,
    pub duplicate_detected: bool,
    pub now_ms: u64,
    pub budget: GateBudget,
    pub audit_available: bool,
    pub scan: GapScanInput<'a>,
    pub signals: GateSignals<'a>,
}

/// 门控输入；`insights`/`fragments` 为不可变借用，编译期禁止回写。
#[derive(Debug, Clone)]
pub struct GateInput<'a> {
    pub session: &'a SessionId,
    pub trace_id: &'a TraceId,
    pub candidate_ref: Option<&'a meeting_copilot_domain::model::card::DedupeKey>,
    pub insight_ref: Option<&'a InsightRef>,
    pub trigger: TriggerKind,
    pub insights: &'a [Insight],
    pub fragments: &'a [NormalizedFragment],
    pub policy: &'a EffectivePolicy,
    pub authorization: &'a GateAuthorization,
    pub provider: ProviderView,
    pub llm: LlmCapabilityView,
    pub network: NetworkState,
    pub session_state: SessionState,
    pub silent_period: bool,
    pub screen_share: bool,
    pub consent_confirmed: bool,
    pub auto_suggestion_enabled: bool,
    pub session_suppressed: bool,
    pub duplicate_detected: bool,
    pub now_ms: u64,
    pub budget: GateBudget,
    pub audit_available: bool,
    pub scan: GapScanInput<'a>,
    pub signals: GateSignals<'a>,
}

impl<'a> GateSnapshot<'a> {
    /// 唯一同源构造路径：四个视图共享同一借用切片。
    pub fn into_input(self) -> GateInput<'a> {
        debug_assert_eq!(self.fragments.len(), self.scan.fragments.len());
        debug_assert_eq!(self.insights.len(), self.scan.insights.len());
        GateInput {
            session: self.session,
            trace_id: self.trace_id,
            candidate_ref: self.candidate_ref,
            insight_ref: self.insight_ref,
            trigger: self.trigger,
            insights: self.insights,
            fragments: self.fragments,
            policy: self.policy,
            authorization: self.authorization,
            provider: self.provider,
            llm: self.llm,
            network: self.network,
            session_state: self.session_state,
            silent_period: self.silent_period,
            screen_share: self.screen_share,
            consent_confirmed: self.consent_confirmed,
            auto_suggestion_enabled: self.auto_suggestion_enabled,
            session_suppressed: self.session_suppressed,
            duplicate_detected: self.duplicate_detected,
            now_ms: self.now_ms,
            budget: self.budget,
            audit_available: self.audit_available,
            scan: self.scan,
            signals: self.signals,
        }
    }
}

/// L0 扫描输入（自给自足，保证 `detect_gaps` 单参签名可用）。
#[derive(Debug, Clone)]
pub struct GapScanInput<'a> {
    pub session: &'a SessionId,
    pub fragments: &'a [NormalizedFragment],
    pub insights: &'a [Insight],
    pub lexicon_names: &'a [String],
    pub explained_terms: &'a [String],
    pub acronym_whitelist: &'a [String],
    pub sensitive_terms: &'a [String],
    pub question_wait_ms: u64,
    pub stable_prefix_ms: &'a BTreeMap<UtteranceRef, u64>,
    pub answered_at_ms: &'a BTreeMap<UtteranceRef, u64>,
    pub now_ms: u64,
}

impl GapScanInput<'_> {
    pub const QUESTION_WAIT_MS: u64 = 10_000;
}

/// 相关性输入（单候选）。
#[derive(Debug, Clone, Copy)]
pub struct RelevanceInput<'a> {
    pub candidate: &'a GapCandidate,
    pub semantic_milli: u16,
    pub entity_milli: u16,
    pub gain_milli: u16,
    pub source_trust_milli: u16,
    pub delta_ms: u64,
    pub threshold_profile: ThresholdProfile,
}
/// G2 四注入值 + Δt 快照。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RelevanceSnapshot {
    pub semantic_milli: u16,
    pub entity_milli: u16,
    pub gain_milli: u16,
    pub source_trust_milli: u16,
    pub delta_ms: u64,
}

/// G2/G3/G4 的只读信号快照；缺失一律按保守默认。
#[derive(Debug, Clone, Copy, Default)]
pub struct GateSignals<'a> {
    pub relevance: RelevanceSnapshot,
    pub similarity_milli: Option<u16>,
    pub duplicate_last_seen_ms: Option<u64>,
    pub cluster_id: u32,
    pub entity_keys: &'a [String],
    pub suppression_topics: &'a [u32],
    pub suppression_providers: &'a [String],
    pub suppression_entities: &'a [String],
    pub cache_age_ms: Option<u64>,
    pub refresh_available: bool,
}
