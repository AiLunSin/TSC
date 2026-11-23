//! 本地卡片入口的 B4 门控装配：手动/实时入口均按显式按需操作处理。

use std::collections::BTreeMap;

use meeting_copilot_application::gate::{
    evaluate_gate, GapScanInput, GateInput, GateSignals, NoopSink, PassthroughSanitizer,
    RelevanceSnapshot,
};
use meeting_copilot_application::mirror_clock::MirrorClock;
use meeting_copilot_application::understand::types::NormalizedFragment;
use meeting_copilot_domain::model::enums::{
    AutoSuggestionSwitch, ConsentMode, SessionState, TriggerKind,
};
use meeting_copilot_domain::model::gate::{
    AuthorizationStatus, GateAuthorization, GateBudget, GateContentType, GateDecision, GatePolicy,
    LlmCapabilityView, NetworkState, ProviderView,
};
use meeting_copilot_domain::model::governance::EffectivePolicy;
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::model::utterance::SpeakerRef;
use meeting_copilot_domain::vo::{
    Confidence, ProviderId, SessionId, TraceId, UtteranceRef, UtteranceText,
};

static SANITIZER: PassthroughSanitizer = PassthroughSanitizer;
static AUDIT_SINK: NoopSink = NoopSink;
const LOCAL_PROVIDER: &str = "local-desktop";

pub(super) struct LocalGate {
    pub decision: GateDecision,
    pub trace_id: TraceId,
    pub session_id: SessionId,
}

/// 对一段本地文本执行完整 B4 判定；装配失败一律返回 `None`（fail-closed）。
pub(super) fn evaluate(text: &str, sequence: u64, now_ms: u64) -> Option<LocalGate> {
    let session_id = SessionId::new("desktop-local-session").ok()?;
    // 授权时间戳在领域模型里是毫秒 `i64`，门控时钟是 `u64`。
    let granted_at_ms = i64::try_from(now_ms).ok()?;
    let trace_id = TraceId::new(format!("trace-local-{sequence:016}")).ok()?;
    let reference = UtteranceRef::new(format!("local-{sequence:016}")).ok()?;
    let provider_id = ProviderId::new(LOCAL_PROVIDER).ok()?;
    let span = TextRange {
        start_ms: now_ms.saturating_sub(4_000),
        end_ms: now_ms.saturating_sub(3_000),
    };
    let fragment_text = UtteranceText::new(text.chars().take(2_000).collect::<String>()).ok()?;
    let fragments = vec![NormalizedFragment {
        session: session_id.clone(),
        reference: reference.clone(),
        parent_reference: reference.clone(),
        span,
        text: fragment_text,
        speaker: SpeakerRef::anonymous("local"),
        confidence: Confidence::new(0.9).ok()?,
        dual_channel: false,
        origin_pair: None,
    }];
    let mut stable = BTreeMap::new();
    stable.insert(reference, span.end_ms);
    let answered = BTreeMap::new();

    let policy = EffectivePolicy {
        scope: meeting_copilot_domain::model::enums::PolicyScope::Personal,
        consent: ConsentMode::Explicit,
        mandatory_sanitize: true,
        retention_days: 1,
        auto_suggestion: AutoSuggestionSwitch::Enabled,
        outbound_grant: None,
        audit_retention_days: EffectivePolicy::MIN_AUDIT_RETENTION_DAYS,
        screen_share_protection_default: true,
    };
    let authorization = GateAuthorization {
        status: AuthorizationStatus::Granted,
        granted_at_ms: Some(granted_at_ms),
        content_types: vec![GateContentType::TextQuery],
        max_query_chars: Some(200),
        provider_ids: vec![provider_id.clone()],
        notice_version: 1,
        revoked_at_ms: None,
    };
    let budget = GateBudget {
        session_outbound_used: 0,
        global_window_used: 0,
        topic_window_used: BTreeMap::new(),
        entity_last_seen_ms: BTreeMap::new(),
        inflight: 0,
        tokens_milli: 3_000,
        window_start_ms: now_ms,
    };
    let scan = GapScanInput {
        session: &session_id,
        fragments: &fragments,
        insights: &[],
        lexicon_names: &[],
        explained_terms: &[],
        acronym_whitelist: &[],
        sensitive_terms: &[],
        question_wait_ms: GapScanInput::QUESTION_WAIT_MS,
        stable_prefix_ms: &stable,
        answered_at_ms: &answered,
        now_ms,
    };
    let signals = GateSignals {
        relevance: RelevanceSnapshot::default(),
        similarity_milli: None,
        duplicate_last_seen_ms: None,
        cluster_id: 1,
        entity_keys: &[],
        suppression_topics: &[],
        suppression_providers: &[],
        suppression_entities: &[],
        cache_age_ms: None,
        refresh_available: false,
    };
    let input = GateInput {
        session: &session_id,
        trace_id: &trace_id,
        candidate_ref: None,
        insight_ref: None,
        trigger: TriggerKind::OnDemand,
        insights: &[],
        fragments: &fragments,
        policy: &policy,
        authorization: &authorization,
        provider: ProviderView {
            enabled_count: 1,
            provider_ids: vec![provider_id],
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
        now_ms,
        budget,
        audit_available: true,
        scan,
        signals,
        sanitizer: &SANITIZER,
        audit_sink: &AUDIT_SINK,
        query_seed: text,
    };
    let policy = GatePolicy {
        relevance_enabled: false,
        dedupe_enabled: false,
        ..GatePolicy::default()
    };
    // 生产时钟 `MirrorClock` 是唯一实现 `GateClock` 的类型；此处每判定一次新建一份，
    // 锚点即当前毫秒，门控内部读到的耗时恒为 0。
    let clock = MirrorClock::new(now_ms);
    let decision = evaluate_gate(&input, &policy, &clock);
    Some(LocalGate {
        decision,
        trace_id,
        session_id,
    })
}
