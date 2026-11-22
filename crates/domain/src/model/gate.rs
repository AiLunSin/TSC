//! C-01 码表：23 项 `GateRejectReason` 为唯一对外标识，12 项投影到既有 11 项 `DenyCode`。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::DenyCode;
use crate::model::card::{DedupeKey, GapCandidate};
use crate::model::enums::TriggerKind;
use crate::vo::{InsightRef, ProviderId, SessionId, TraceId};

/// 对外可见性。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Public,
    Internal,
}

/// 门控拒绝原因（唯一对外码，23 项，顺序即断言口径）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateRejectReason {
    PreFilterLowConfidence,
    ConsentMissing,
    AutoDisabled,
    Paused,
    SilentPeriod,
    ScreenShareProtection,
    SensitiveTerm,
    Duplicate,
    Suppressed,
    QuotaExhausted,
    NoProviderEnabled,
    LowRelevance,
    Stale,
    BudgetExhausted,
    GatewayCooling,
    Offline,
    OutboundNotGranted,
    SanitizeFailed,
    AuditUnavailable,
    ProviderNotAllowed,
    DomainBlocked,
    PolicyUnavailable,
    RetentionExpired,
}

impl GateRejectReason {
    /// 顺序与 design §2.2.3 表 #1..#23 完全一致。
    pub const ALL: [Self; 23] = [
        Self::PreFilterLowConfidence,
        Self::ConsentMissing,
        Self::AutoDisabled,
        Self::Paused,
        Self::SilentPeriod,
        Self::ScreenShareProtection,
        Self::SensitiveTerm,
        Self::Duplicate,
        Self::Suppressed,
        Self::QuotaExhausted,
        Self::NoProviderEnabled,
        Self::LowRelevance,
        Self::Stale,
        Self::BudgetExhausted,
        Self::GatewayCooling,
        Self::Offline,
        Self::OutboundNotGranted,
        Self::SanitizeFailed,
        Self::AuditUnavailable,
        Self::ProviderNotAllowed,
        Self::DomainBlocked,
        Self::PolicyUnavailable,
        Self::RetentionExpired,
    ];

    /// serde 值，同时用于 `ui_message_key` 与审计 `reason`。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PreFilterLowConfidence => "pre_filter_low_confidence",
            Self::ConsentMissing => "consent_missing",
            Self::AutoDisabled => "auto_disabled",
            Self::Paused => "paused",
            Self::SilentPeriod => "silent_period",
            Self::ScreenShareProtection => "screen_share_protection",
            Self::SensitiveTerm => "sensitive_term",
            Self::Duplicate => "duplicate",
            Self::Suppressed => "suppressed",
            Self::QuotaExhausted => "quota_exhausted",
            Self::NoProviderEnabled => "no_provider_enabled",
            Self::LowRelevance => "low_relevance",
            Self::Stale => "stale",
            Self::BudgetExhausted => "budget_exhausted",
            Self::GatewayCooling => "gateway_cooling",
            Self::Offline => "offline",
            Self::OutboundNotGranted => "outbound_not_granted",
            Self::SanitizeFailed => "sanitize_failed",
            Self::AuditUnavailable => "audit_unavailable",
            Self::ProviderNotAllowed => "provider_not_allowed",
            Self::DomainBlocked => "domain_blocked",
            Self::PolicyUnavailable => "policy_unavailable",
            Self::RetentionExpired => "retention_expired",
        }
    }

    /// 12 项投影到既有 `DenyCode`；11 项为纯内部码。
    pub const fn to_deny_code(self) -> Option<DenyCode> {
        match self {
            Self::ConsentMissing => Some(DenyCode::ConsentMissing),
            Self::SilentPeriod => Some(DenyCode::SilentPeriod),
            Self::ScreenShareProtection => Some(DenyCode::ScreenShareProtection),
            Self::SensitiveTerm => Some(DenyCode::SensitiveTermBlocked),
            Self::QuotaExhausted | Self::BudgetExhausted => Some(DenyCode::RateLimited),
            Self::OutboundNotGranted => Some(DenyCode::OutboundNotGranted),
            Self::SanitizeFailed => Some(DenyCode::SanitizeRequired),
            Self::ProviderNotAllowed => Some(DenyCode::ProviderNotAllowed),
            Self::DomainBlocked => Some(DenyCode::DomainBlocked),
            Self::PolicyUnavailable => Some(DenyCode::PolicyUnavailable),
            Self::RetentionExpired => Some(DenyCode::RetentionExpired),
            Self::PreFilterLowConfidence
            | Self::AutoDisabled
            | Self::Paused
            | Self::Duplicate
            | Self::Suppressed
            | Self::NoProviderEnabled
            | Self::LowRelevance
            | Self::Stale
            | Self::GatewayCooling
            | Self::Offline
            | Self::AuditUnavailable => None,
        }
    }

    /// UI 文案键统一为 `gate.reason.<serde 值>`。
    pub fn ui_message_key(self) -> String {
        format!("gate.reason.{}", self.as_str())
    }

    pub const fn visibility(self) -> Visibility {
        match self {
            Self::ConsentMissing
            | Self::SilentPeriod
            | Self::ScreenShareProtection
            | Self::SensitiveTerm
            | Self::QuotaExhausted
            | Self::BudgetExhausted
            | Self::OutboundNotGranted
            | Self::SanitizeFailed
            | Self::ProviderNotAllowed
            | Self::DomainBlocked
            | Self::PolicyUnavailable
            | Self::RetentionExpired => Visibility::Public,
            Self::PreFilterLowConfidence
            | Self::AutoDisabled
            | Self::Paused
            | Self::Duplicate
            | Self::Suppressed
            | Self::NoProviderEnabled
            | Self::LowRelevance
            | Self::Stale
            | Self::GatewayCooling
            | Self::Offline
            | Self::AuditUnavailable => Visibility::Internal,
        }
    }

    /// 原因码在“产生级”对应的分项计数；非产生级返回 `None`。
    pub const fn counter_at(self, stage: GateStage) -> Option<GateCounter> {
        match (self, stage) {
            (Self::PreFilterLowConfidence, GateStage::G1) => Some(GateCounter::GateL1LowConfidence),
            (Self::ConsentMissing, GateStage::G1) => Some(GateCounter::GateL1ConsentMissing),
            (Self::AutoDisabled, GateStage::G1) => Some(GateCounter::GateL1AutoDisabled),
            (Self::Paused, GateStage::G1) => Some(GateCounter::GateL1Paused),
            (Self::SilentPeriod, GateStage::G1) => Some(GateCounter::GateL1SilentPeriod),
            (Self::ScreenShareProtection, GateStage::G1) => Some(GateCounter::GateL1ScreenShare),
            (Self::SensitiveTerm, GateStage::G1) => Some(GateCounter::GateL1SensitiveTerm),
            (Self::Duplicate, GateStage::G1) => Some(GateCounter::GateL1Duplicate),
            (Self::Duplicate, GateStage::G3) => Some(GateCounter::GateL3Duplicate),
            (Self::Suppressed, GateStage::G1) => Some(GateCounter::GateL1Suppressed),
            (Self::Suppressed, GateStage::G3) => Some(GateCounter::GateL3Suppressed),
            (Self::QuotaExhausted, GateStage::G1) => Some(GateCounter::GateL1QuotaExhausted),
            (Self::QuotaExhausted, GateStage::G3) => Some(GateCounter::GateL3Quota),
            (Self::NoProviderEnabled, GateStage::G1) => Some(GateCounter::GateL1NoProvider),
            (Self::LowRelevance, GateStage::G2) => Some(GateCounter::GateL2LowRelevance),
            (Self::Stale, GateStage::G3) => Some(GateCounter::GateL3Stale),
            (Self::BudgetExhausted, GateStage::G4) => Some(GateCounter::GateL4BudgetExhausted),
            (Self::GatewayCooling, GateStage::G4) => Some(GateCounter::GateL4GatewayCooling),
            (Self::Offline, GateStage::G4) => Some(GateCounter::GateL4Offline),
            (Self::OutboundNotGranted, GateStage::G5) => Some(GateCounter::GateL5NotGranted),
            (Self::SanitizeFailed, GateStage::G5) => Some(GateCounter::GateL5SanitizeFailed),
            (Self::AuditUnavailable, GateStage::G5) => Some(GateCounter::GateL5AuditUnavailable),
            (Self::ProviderNotAllowed, GateStage::G5) | (Self::DomainBlocked, GateStage::G5) => {
                Some(GateCounter::GateL5ProviderOrDomainBlocked)
            }
            _ => None,
        }
    }
}

/// 执行漏斗的级。固定 5 项，顺序 G1..G5 即数组序。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateStage {
    G1,
    G2,
    G3,
    G4,
    G5,
}

impl GateStage {
    pub const ALL: [Self; 5] = [Self::G1, Self::G2, Self::G3, Self::G4, Self::G5];
}

/// 单级结果；`NotReached` 与 `Skipped` 均可判定。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageResult {
    Allow,
    Deny,
    Skipped,
    NoCandidate,
    NotReached,
}

/// L0 六类规则的内部标签（7 变体：Institution 是 ProperNoun 的子标签）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GapRule {
    ProperNoun,
    Abbreviation,
    Institution,
    PolicyStandard,
    Metric,
    UnansweredQuestion,
    Acronym,
}

impl GapRule {
    pub const ALL: [Self; 7] = [
        Self::ProperNoun,
        Self::Abbreviation,
        Self::Institution,
        Self::PolicyStandard,
        Self::Metric,
        Self::UnansweredQuestion,
        Self::Acronym,
    ];

    /// 基础分（毫值）。
    pub const fn base_milli(self) -> u16 {
        match self {
            Self::ProperNoun => 700,
            Self::Abbreviation => 650,
            Self::Institution => 600,
            Self::PolicyStandard => 800,
            Self::Metric => 750,
            Self::UnansweredQuestion => 850,
            Self::Acronym => 650,
        }
    }

    /// 对外映射；Institution 归入 `proper_noun`。
    pub const fn gap_kind_name(self) -> &'static str {
        match self {
            Self::ProperNoun | Self::Institution => "proper_noun",
            Self::Abbreviation => "abbreviation",
            Self::PolicyStandard => "policy_standard",
            Self::Metric => "metric",
            Self::UnansweredQuestion => "unanswered_question",
            Self::Acronym => "acronym",
        }
    }
}

/// 六分项相关性（全部毫值，0..=1000，无浮点）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelevanceScore {
    pub semantic_milli: u16,
    pub entity_milli: u16,
    pub gain_milli: u16,
    pub source_trust_milli: u16,
    pub decay_milli: u16,
    pub score_milli: u16,
}

/// 相关性阈值档位：720 / 620 / 520。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThresholdProfile {
    Strict,
    #[default]
    Balanced,
    Loose,
}

impl ThresholdProfile {
    pub const fn milli(self) -> u16 {
        match self {
            Self::Strict => 720,
            Self::Balanced => 620,
            Self::Loose => 520,
        }
    }

    /// 命中 520/620/720；其余返回 `ThresholdProfileInvalid`，使该变体可达。
    pub fn from_milli(milli: u16) -> Result<Self, GatePolicyError> {
        match milli {
            720 => Ok(Self::Strict),
            620 => Ok(Self::Balanced),
            520 => Ok(Self::Loose),
            other => Err(GatePolicyError::ThresholdProfileInvalid(other)),
        }
    }
}

/// 频率档位：每 600 000 ms 1 / 3 / 5 次。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrequencyProfile {
    Low,
    #[default]
    Medium,
    High,
}

impl FrequencyProfile {
    pub const fn per_window(self) -> u8 {
        match self {
            Self::Low => 1,
            Self::Medium => 3,
            Self::High => 5,
        }
    }
}

/// 门控策略。强制级无 skip 字段，从类型上杜绝非法配置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GatePolicy {
    pub relevance_enabled: bool,
    pub dedupe_enabled: bool,
    pub threshold_profile: ThresholdProfile,
    pub frequency_profile: FrequencyProfile,
    pub session_outbound_cap: u16,
    pub auto_query_limit: u16,
    pub on_demand_query_limit: u16,
    pub max_cache_age_ms: u64,
    #[serde(default, flatten)]
    pub unknown: BTreeMap<String, serde_json::Value>,
}

impl Default for GatePolicy {
    fn default() -> Self {
        Self {
            relevance_enabled: true,
            dedupe_enabled: true,
            threshold_profile: ThresholdProfile::Balanced,
            frequency_profile: FrequencyProfile::Medium,
            session_outbound_cap: 120,
            auto_query_limit: 80,
            on_demand_query_limit: 200,
            max_cache_age_ms: 86_400_000,
            unknown: BTreeMap::new(),
        }
    }
}

impl GatePolicy {
    pub const fn threshold_milli(&self) -> u16 {
        self.threshold_profile.milli()
    }

    /// 先拒绝 `skip_*`，再校验数值——保证安全优先。
    pub fn validate(&self) -> Result<(), GatePolicyError> {
        for key in self.unknown.keys() {
            if key == "skip_g1" {
                return Err(GatePolicyError::MandatoryStageCannotSkip { stage: 1 });
            }
            if key == "skip_g4" {
                return Err(GatePolicyError::MandatoryStageCannotSkip { stage: 4 });
            }
            if key == "skip_g5" {
                return Err(GatePolicyError::MandatoryStageCannotSkip { stage: 5 });
            }
            if key.starts_with("skip_") {
                return Err(GatePolicyError::MandatoryStageCannotSkip { stage: 0 });
            }
        }
        if self.session_outbound_cap > 600 {
            return Err(GatePolicyError::SessionCapOutOfRange(
                self.session_outbound_cap,
            ));
        }
        if self.auto_query_limit != 80 || self.on_demand_query_limit != 200 {
            return Err(GatePolicyError::QueryLimitNotFixed {
                auto: self.auto_query_limit,
                on_demand: self.on_demand_query_limit,
            });
        }
        let threshold = self.threshold_profile.milli();
        if !matches!(threshold, 520 | 620 | 720) {
            return Err(GatePolicyError::ThresholdProfileInvalid(threshold));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GatePolicyError {
    #[error("会话外发上限 {0} 超出 0..=600")]
    SessionCapOutOfRange(u16),
    #[error("阈值档位非法：{0}")]
    ThresholdProfileInvalid(u16),
    #[error("自动/按需长度上限固定为 80/200，实际 {auto}/{on_demand}")]
    QueryLimitNotFixed { auto: u16, on_demand: u16 },
    #[error("强制级 G{stage} 不可跳过")]
    MandatoryStageCannotSkip { stage: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorizationStatus {
    Granted,
    Revoked,
    Expired,
    Missing,
}

/// 外发内容类型：只允许 `TextQuery`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateContentType {
    TextQuery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryLimit {
    Auto,
    OnDemand,
}

impl QueryLimit {
    pub const fn chars(self) -> usize {
        match self {
            Self::Auto => 80,
            Self::OnDemand => 200,
        }
    }
}

/// 授权记录 7 字段。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateAuthorization {
    pub status: AuthorizationStatus,
    #[serde(default)]
    pub granted_at_ms: Option<i64>,
    #[serde(default)]
    pub content_types: Vec<GateContentType>,
    #[serde(default)]
    pub max_query_chars: Option<u16>,
    #[serde(default)]
    pub provider_ids: Vec<ProviderId>,
    #[serde(default)]
    pub notice_version: u16,
    #[serde(default)]
    pub revoked_at_ms: Option<i64>,
}

impl GateAuthorization {
    /// 7 字段齐备且可外发；任一缺失或非 Granted 一律 false（fail-closed）。
    pub fn is_complete(&self) -> bool {
        self.status == AuthorizationStatus::Granted
            && self.granted_at_ms.is_some()
            && self.content_types == [GateContentType::TextQuery]
            && self.max_query_chars.is_some()
            && !self.provider_ids.is_empty()
            && self.notice_version >= 1
            && self.revoked_at_ms.is_none()
    }

    /// 撤销不可自恢复：`Revoked` 或已有 `revoked_at_ms` 一律不可直接授权。
    pub fn is_grantable(&self) -> bool {
        self.status != AuthorizationStatus::Revoked && self.revoked_at_ms.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkState {
    Online,
    Offline,
    Unknown,
}

/// 实体键：32 位十六进制文本，不携带实体原文。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct EntityKey(String);

impl EntityKey {
    /// 恰 32 位十六进制；大小写归一为小写，否则 `None`。
    pub fn parse(raw: &str) -> Option<Self> {
        if raw.len() != 32 || !raw.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        Some(Self(raw.to_ascii_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EntityKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// 门控预算 7 字段。判定外 `commit` 推进。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateBudget {
    pub session_outbound_used: u16,
    pub global_window_used: u8,
    pub topic_window_used: BTreeMap<u32, u8>,
    pub entity_last_seen_ms: BTreeMap<EntityKey, u64>,
    pub inflight: u8,
    #[serde(default)]
    pub tokens_milli: u16,
    #[serde(default)]
    pub window_start_ms: u64,
}

impl GateBudget {
    pub fn session_remaining_with(&self, cap: u16) -> u16 {
        cap.saturating_sub(self.session_outbound_used)
    }

    pub fn is_within_limits(&self) -> bool {
        self.session_outbound_used <= 600
            && self.global_window_used <= 5
            && self.inflight <= 3
            && self.tokens_milli <= 3000
            && self.topic_window_used.values().all(|used| *used <= 2)
    }

    /// 仅 `Allow` 且携带外发意图时推进，`Deny` 零变化。
    pub fn commit(&mut self, decision: &GateDecision) {
        if let GateDecision::Allow {
            outbound: Some(_), ..
        } = decision
        {
            self.session_outbound_used = self.session_outbound_used.saturating_add(1);
            self.global_window_used = self.global_window_used.saturating_add(1);
            self.tokens_milli = self.tokens_milli.saturating_sub(1000);
        }
    }
}

/// 32 项固定计数；顺序即 spec §4.4 顺序。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GateCounter {
    #[serde(rename = "gate_l0_candidates")]
    GateL0Candidates,
    #[serde(rename = "gate_l0_low_confidence")]
    GateL0LowConfidence,
    #[serde(rename = "gate_l0_question_answered_skipped")]
    GateL0QuestionAnsweredSkipped,
    #[serde(rename = "gate_l0_sensitive_seed_blocked")]
    GateL0SensitiveSeedBlocked,
    #[serde(rename = "gate_allow_total")]
    GateAllowTotal,
    #[serde(rename = "gate_deny_total")]
    GateDenyTotal,
    #[serde(rename = "gate_stage_skipped_total")]
    GateStageSkippedTotal,
    #[serde(rename = "gate_l1_auto_disabled")]
    GateL1AutoDisabled,
    #[serde(rename = "gate_l1_consent_missing")]
    GateL1ConsentMissing,
    #[serde(rename = "gate_l1_paused")]
    GateL1Paused,
    #[serde(rename = "gate_l1_silent_period")]
    GateL1SilentPeriod,
    #[serde(rename = "gate_l1_screen_share")]
    GateL1ScreenShare,
    #[serde(rename = "gate_l1_low_confidence")]
    GateL1LowConfidence,
    #[serde(rename = "gate_l1_duplicate")]
    GateL1Duplicate,
    #[serde(rename = "gate_l1_suppressed")]
    GateL1Suppressed,
    #[serde(rename = "gate_l1_quota_exhausted")]
    GateL1QuotaExhausted,
    #[serde(rename = "gate_l1_no_provider")]
    GateL1NoProvider,
    #[serde(rename = "gate_l1_sensitive_term")]
    GateL1SensitiveTerm,
    #[serde(rename = "gate_l2_low_relevance")]
    GateL2LowRelevance,
    #[serde(rename = "gate_l3_duplicate")]
    GateL3Duplicate,
    #[serde(rename = "gate_l3_suppressed")]
    GateL3Suppressed,
    #[serde(rename = "gate_l3_stale")]
    GateL3Stale,
    #[serde(rename = "gate_l3_quota")]
    GateL3Quota,
    #[serde(rename = "gate_l4_budget_exhausted")]
    GateL4BudgetExhausted,
    #[serde(rename = "gate_l4_gateway_cooling")]
    GateL4GatewayCooling,
    #[serde(rename = "gate_l4_offline")]
    GateL4Offline,
    #[serde(rename = "gate_l5_not_granted")]
    GateL5NotGranted,
    #[serde(rename = "gate_l5_sanitize_failed")]
    GateL5SanitizeFailed,
    #[serde(rename = "gate_l5_audit_unavailable")]
    GateL5AuditUnavailable,
    #[serde(rename = "gate_l5_provider_or_domain_blocked")]
    GateL5ProviderOrDomainBlocked,
    #[serde(rename = "gate_replay_runs")]
    GateReplayRuns,
    #[serde(rename = "gate_replay_mismatch")]
    GateReplayMismatch,
}

impl GateCounter {
    pub const ALL: [Self; 32] = [
        Self::GateL0Candidates,
        Self::GateL0LowConfidence,
        Self::GateL0QuestionAnsweredSkipped,
        Self::GateL0SensitiveSeedBlocked,
        Self::GateAllowTotal,
        Self::GateDenyTotal,
        Self::GateStageSkippedTotal,
        Self::GateL1AutoDisabled,
        Self::GateL1ConsentMissing,
        Self::GateL1Paused,
        Self::GateL1SilentPeriod,
        Self::GateL1ScreenShare,
        Self::GateL1LowConfidence,
        Self::GateL1Duplicate,
        Self::GateL1Suppressed,
        Self::GateL1QuotaExhausted,
        Self::GateL1NoProvider,
        Self::GateL1SensitiveTerm,
        Self::GateL2LowRelevance,
        Self::GateL3Duplicate,
        Self::GateL3Suppressed,
        Self::GateL3Stale,
        Self::GateL3Quota,
        Self::GateL4BudgetExhausted,
        Self::GateL4GatewayCooling,
        Self::GateL4Offline,
        Self::GateL5NotGranted,
        Self::GateL5SanitizeFailed,
        Self::GateL5AuditUnavailable,
        Self::GateL5ProviderOrDomainBlocked,
        Self::GateReplayRuns,
        Self::GateReplayMismatch,
    ];

    pub const fn index(self) -> usize {
        match self {
            Self::GateL0Candidates => 0,
            Self::GateL0LowConfidence => 1,
            Self::GateL0QuestionAnsweredSkipped => 2,
            Self::GateL0SensitiveSeedBlocked => 3,
            Self::GateAllowTotal => 4,
            Self::GateDenyTotal => 5,
            Self::GateStageSkippedTotal => 6,
            Self::GateL1AutoDisabled => 7,
            Self::GateL1ConsentMissing => 8,
            Self::GateL1Paused => 9,
            Self::GateL1SilentPeriod => 10,
            Self::GateL1ScreenShare => 11,
            Self::GateL1LowConfidence => 12,
            Self::GateL1Duplicate => 13,
            Self::GateL1Suppressed => 14,
            Self::GateL1QuotaExhausted => 15,
            Self::GateL1NoProvider => 16,
            Self::GateL1SensitiveTerm => 17,
            Self::GateL2LowRelevance => 18,
            Self::GateL3Duplicate => 19,
            Self::GateL3Suppressed => 20,
            Self::GateL3Stale => 21,
            Self::GateL3Quota => 22,
            Self::GateL4BudgetExhausted => 23,
            Self::GateL4GatewayCooling => 24,
            Self::GateL4Offline => 25,
            Self::GateL5NotGranted => 26,
            Self::GateL5SanitizeFailed => 27,
            Self::GateL5AuditUnavailable => 28,
            Self::GateL5ProviderOrDomainBlocked => 29,
            Self::GateReplayRuns => 30,
            Self::GateReplayMismatch => 31,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GateCounters {
    counts: [u64; 32],
}

impl GateCounters {
    /// 唯一写入口：饱和 +1，不回绕、不清零。
    pub fn bump(&mut self, counter: GateCounter) {
        let slot = &mut self.counts[counter.index()];
        *slot = slot.saturating_add(1);
    }

    pub fn get(&self, counter: GateCounter) -> u64 {
        self.counts[counter.index()]
    }

    pub fn snapshot(&self) -> [(GateCounter, u64); 32] {
        let mut out = [(GateCounter::GateL0Candidates, 0u64); 32];
        for (index, counter) in GateCounter::ALL.into_iter().enumerate() {
            out[index] = (counter, self.counts[index]);
        }
        out
    }

    pub fn from_snapshot(snapshot: [(GateCounter, u64); 32]) -> Self {
        let mut counts = [0u64; 32];
        for (counter, value) in snapshot {
            counts[counter.index()] = value;
        }
        Self { counts }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageTrace {
    pub calls: u16,
    pub result: StageResult,
    pub reason: Option<GateRejectReason>,
    pub elapsed_ms: u32,
}

/// 固定 5 元素数组，保证序列化顺序恒定。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerStageResults(pub [StageTrace; 5]);

impl PerStageResults {
    pub fn empty() -> Self {
        Self(
            [StageTrace {
                calls: 0,
                result: StageResult::NotReached,
                reason: None,
                elapsed_ms: 0,
            }; 5],
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClusterSource {
    Injected,
    Fallback,
}

/// 候选 + 评分 + 主题/实体；不改 `GapCandidate`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoredCandidate {
    pub candidate: GapCandidate,
    pub rule: GapRule,
    pub relevance: RelevanceScore,
    pub topic_cluster_id: u32,
    pub entity_keys: Vec<EntityKey>,
    pub topic_cluster_source: ClusterSource,
}

/// 外发意图（无原文）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundIntent {
    pub sanitized_query_len: usize,
    pub content_type: GateContentType,
    pub provider_ids: Vec<ProviderId>,
    pub authorization_status: AuthorizationStatus,
    pub truncated: bool,
    pub queued: bool,
    #[serde(default)]
    pub cooldown_until_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GateDecision {
    Allow {
        per_stage: PerStageResults,
        budget_remaining: u16,
        candidates: Vec<ScoredCandidate>,
        outbound: Option<OutboundIntent>,
        suppressed_notice: bool,
        stopped_at: Option<GateStage>,
    },
    Deny {
        reason: GateRejectReason,
        stopped_at: GateStage,
        per_stage: PerStageResults,
        cooldown_until_ms: Option<u64>,
    },
}

/// 记录层的轻量决策标签。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionKind {
    Allow,
    Deny,
}

/// 门控记录：14 业务字段 + 透传容器。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateRecord {
    pub schema_version: u16,
    pub trace_id: TraceId,
    pub session_id: SessionId,
    #[serde(default)]
    pub candidate_ref: Option<DedupeKey>,
    #[serde(default)]
    pub insight_ref: Option<InsightRef>,
    pub trigger: TriggerKind,
    pub per_stage: PerStageResults,
    pub decision: DecisionKind,
    pub stopped_at: Option<GateStage>,
    pub budget_remaining: u16,
    #[serde(default)]
    pub cooldown_until_ms: Option<u64>,
    pub authorization_status: AuthorizationStatus,
    #[serde(default)]
    pub skipped_stages: Vec<GateStage>,
    pub decided_at_ms: u64,
    #[serde(default, flatten)]
    pub unknown: BTreeMap<String, serde_json::Value>,
}

impl GateRecord {
    /// 空集合合法；非空时逐项 ∈ {G2, G3}。
    pub fn skipped_stages_are_valid(&self) -> bool {
        self.skipped_stages
            .iter()
            .all(|stage| matches!(stage, GateStage::G2 | GateStage::G3))
    }

    pub fn refs_are_valid(&self) -> bool {
        self.candidate_ref.is_some() || self.insight_ref.is_some()
    }

    /// `Deny` 与 `stopped_at.is_some()` 必须同真同假。
    pub fn decision_tag_matches_stopped_at(&self) -> bool {
        (self.decision == DecisionKind::Deny) == self.stopped_at.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditResult {
    Allow,
    Deny,
}

/// 审计事件（7 字段 + 发生时间），不写 `stopped_at`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateAuditEvent {
    pub authorization_status: AuthorizationStatus,
    #[serde(default)]
    pub granted_at_ms: Option<i64>,
    pub content_type: GateContentType,
    pub query_len: usize,
    pub provider_ids: Vec<ProviderId>,
    pub result: AuditResult,
    #[serde(default)]
    pub reason: Option<GateRejectReason>,
    pub occurred_at_ms: u64,
}

/// 网关能力快照；不含模型名/端点。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmCapabilityView {
    pub cooling: bool,
    #[serde(default)]
    pub cooling_until_ms: Option<u64>,
}

/// 检索源视图；不含端点/密钥。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderView {
    pub enabled_count: u8,
    #[serde(default)]
    pub provider_ids: Vec<ProviderId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_reject_reason_all_is_23_in_table_order() {
        assert_eq!(GateRejectReason::ALL.len(), 23);
        assert_eq!(
            GateRejectReason::ALL[0],
            GateRejectReason::PreFilterLowConfidence
        );
        assert_eq!(
            GateRejectReason::ALL[22],
            GateRejectReason::RetentionExpired
        );
    }

    #[test]
    fn reject_reason_rejects_unknown_string() {
        assert!(serde_json::from_str::<GateRejectReason>("\"not_a_reason\"").is_err());
        assert_eq!(
            serde_json::from_str::<GateRejectReason>("\"consent_missing\"").expect("known"),
            GateRejectReason::ConsentMissing
        );
    }

    #[test]
    fn deny_code_mapping_has_exactly_12_some() {
        let mapped = GateRejectReason::ALL
            .into_iter()
            .filter(|reason| reason.to_deny_code().is_some())
            .count();
        assert_eq!(mapped, 12);
    }

    #[test]
    fn deny_code_mapping_covers_11_with_rate_limited_twice() {
        let mut covered = Vec::new();
        let mut rate_limited = 0;
        for reason in GateRejectReason::ALL {
            if let Some(code) = reason.to_deny_code() {
                if code == DenyCode::RateLimited {
                    rate_limited += 1;
                }
                if !covered.contains(&code) {
                    covered.push(code);
                }
            }
        }
        assert_eq!(covered.len(), 11);
        assert_eq!(rate_limited, 2);
    }

    #[test]
    fn ui_message_key_is_23_and_matches_serde_value() {
        let mut keys = Vec::new();
        for reason in GateRejectReason::ALL {
            let key = reason.ui_message_key();
            assert_eq!(key, format!("gate.reason.{}", reason.as_str()));
            assert!(!keys.contains(&key));
            keys.push(key);
        }
        assert_eq!(keys.len(), 23);
    }

    #[test]
    fn visibility_is_12_public_11_internal() {
        let public = GateRejectReason::ALL
            .into_iter()
            .filter(|reason| reason.visibility() == Visibility::Public)
            .count();
        let internal = GateRejectReason::ALL
            .into_iter()
            .filter(|reason| reason.visibility() == Visibility::Internal)
            .count();
        assert_eq!(public, 12);
        assert_eq!(internal, 11);
    }

    struct RecordField;

    impl RecordField {
        const ALL: [&'static str; 14] = [
            "schema_version",
            "trace_id",
            "session_id",
            "candidate_ref",
            "insight_ref",
            "trigger",
            "per_stage",
            "decision",
            "stopped_at",
            "budget_remaining",
            "cooldown_until_ms",
            "authorization_status",
            "skipped_stages",
            "decided_at_ms",
        ];
    }

    const COUNTER_NAMES: [&str; 32] = [
        "gate_l0_candidates",
        "gate_l0_low_confidence",
        "gate_l0_question_answered_skipped",
        "gate_l0_sensitive_seed_blocked",
        "gate_allow_total",
        "gate_deny_total",
        "gate_stage_skipped_total",
        "gate_l1_auto_disabled",
        "gate_l1_consent_missing",
        "gate_l1_paused",
        "gate_l1_silent_period",
        "gate_l1_screen_share",
        "gate_l1_low_confidence",
        "gate_l1_duplicate",
        "gate_l1_suppressed",
        "gate_l1_quota_exhausted",
        "gate_l1_no_provider",
        "gate_l1_sensitive_term",
        "gate_l2_low_relevance",
        "gate_l3_duplicate",
        "gate_l3_suppressed",
        "gate_l3_stale",
        "gate_l3_quota",
        "gate_l4_budget_exhausted",
        "gate_l4_gateway_cooling",
        "gate_l4_offline",
        "gate_l5_not_granted",
        "gate_l5_sanitize_failed",
        "gate_l5_audit_unavailable",
        "gate_l5_provider_or_domain_blocked",
        "gate_replay_runs",
        "gate_replay_mismatch",
    ];

    fn sample_record() -> GateRecord {
        GateRecord {
            schema_version: 1,
            trace_id: TraceId::new("trace-1").expect("trace"),
            session_id: SessionId::new("session-1").expect("session"),
            candidate_ref: Some(DedupeKey::new("a".repeat(32)).expect("dedupe")),
            insight_ref: None,
            trigger: TriggerKind::Auto,
            per_stage: PerStageResults::empty(),
            decision: DecisionKind::Deny,
            stopped_at: Some(GateStage::G1),
            budget_remaining: 120,
            cooldown_until_ms: None,
            authorization_status: AuthorizationStatus::Missing,
            skipped_stages: Vec::new(),
            decided_at_ms: 1_000,
            unknown: BTreeMap::new(),
        }
    }

    fn deny_decision(reason: GateRejectReason, stopped_at: GateStage) -> GateDecision {
        GateDecision::Deny {
            reason,
            stopped_at,
            per_stage: PerStageResults::empty(),
            cooldown_until_ms: None,
        }
    }

    #[test]
    fn gate_contract_types_are_31() {
        assert_eq!(GateStage::ALL.len(), 5);
        assert_eq!(GateCounter::ALL.len(), 32);
        assert_eq!(GateRejectReason::ALL.len(), 23);
        assert_eq!(RecordField::ALL.len(), 14);
        let mut names = Vec::new();
        for field in RecordField::ALL {
            assert!(!names.contains(&field));
            names.push(field);
        }
    }

    #[test]
    fn per_stage_serializes_exactly_five_entries() {
        let empty = PerStageResults::empty();
        assert_eq!(empty.0.len(), 5);
        assert!(empty.0.iter().all(|trace| trace.calls == 0
            && trace.result == StageResult::NotReached
            && trace.reason.is_none()));
        let encoded = serde_json::to_value(&empty).expect("serialize");
        assert_eq!(encoded.as_array().map(|items| items.len()), Some(5));
    }

    #[test]
    fn gate_record_roundtrips_with_closed_fields() {
        let record = sample_record();
        assert!(record.decision_tag_matches_stopped_at());
        let first = serde_json::to_string(&record).expect("first");
        let decoded: GateRecord = serde_json::from_str(&first).expect("decode");
        let second = serde_json::to_string(&decoded).expect("second");
        assert_eq!(first, second);
        let value: serde_json::Value = serde_json::from_str(&first).expect("value");
        let object = value.as_object().expect("object");
        for field in RecordField::ALL {
            assert!(object.contains_key(field), "missing {field}");
        }
        assert_eq!(object.len(), RecordField::ALL.len());
    }

    #[test]
    fn skipped_stages_only_g2_g3() {
        let mut record = sample_record();
        for stages in [
            vec![],
            vec![GateStage::G2],
            vec![GateStage::G3],
            vec![GateStage::G2, GateStage::G3],
        ] {
            record.skipped_stages = stages;
            assert!(record.skipped_stages_are_valid());
        }
        for stage in [GateStage::G1, GateStage::G4, GateStage::G5] {
            record.skipped_stages = vec![stage];
            assert!(!record.skipped_stages_are_valid());
        }
    }

    #[test]
    fn outbound_intent_and_audit_event_have_no_content_fields() {
        let intent = OutboundIntent {
            sanitized_query_len: 80,
            content_type: GateContentType::TextQuery,
            provider_ids: Vec::new(),
            authorization_status: AuthorizationStatus::Granted,
            truncated: true,
            queued: false,
            cooldown_until_ms: None,
        };
        let event = GateAuditEvent {
            authorization_status: AuthorizationStatus::Granted,
            granted_at_ms: Some(1),
            content_type: GateContentType::TextQuery,
            query_len: 80,
            provider_ids: Vec::new(),
            result: AuditResult::Allow,
            reason: None,
            occurred_at_ms: 2,
        };
        let forbidden = ["text", "url", "request_id", "model", "api_key"];
        let intent_value = serde_json::to_value(&intent).expect("intent");
        for key in intent_value.as_object().expect("object").keys() {
            assert!(!forbidden.contains(&key.as_str()), "forbidden {key}");
            assert_ne!(key, "query");
        }
        let event_value = serde_json::to_value(&event).expect("event");
        for key in event_value.as_object().expect("object").keys() {
            assert!(!forbidden.contains(&key.as_str()), "forbidden {key}");
            assert_ne!(key, "stopped_at");
        }
        assert!(serde_json::from_str::<GateContentType>("\"audio\"").is_err());
        assert!(serde_json::from_str::<GateContentType>("\"text_query\"").is_ok());
    }

    #[test]
    fn record_rejects_empty_refs() {
        let mut record = sample_record();
        record.candidate_ref = None;
        record.insight_ref = None;
        assert!(!record.refs_are_valid());
        record.insight_ref = Some(InsightRef::new("insight-1").expect("insight"));
        assert!(record.refs_are_valid());
    }

    #[test]
    fn budget_rejects_corrupted_snapshot_and_saturates_remaining() {
        let budget = GateBudget {
            session_outbound_used: 121,
            global_window_used: 0,
            topic_window_used: BTreeMap::new(),
            entity_last_seen_ms: BTreeMap::new(),
            inflight: 0,
            tokens_milli: 0,
            window_start_ms: 0,
        };
        assert_eq!(budget.session_remaining_with(120), 0);
        assert!(budget.is_within_limits());

        let illegal = GateBudget {
            session_outbound_used: 601,
            global_window_used: 6,
            topic_window_used: BTreeMap::from([(1, 3)]),
            entity_last_seen_ms: BTreeMap::new(),
            inflight: 4,
            tokens_milli: 3001,
            window_start_ms: 0,
        };
        assert!(!illegal.is_within_limits());

        let mut fresh = GateBudget {
            session_outbound_used: 1,
            global_window_used: 1,
            topic_window_used: BTreeMap::new(),
            entity_last_seen_ms: BTreeMap::new(),
            inflight: 0,
            tokens_milli: 2000,
            window_start_ms: 0,
        };
        let before = fresh.clone();
        fresh.commit(&deny_decision(GateRejectReason::Duplicate, GateStage::G3));
        assert_eq!(fresh, before);
    }

    #[test]
    fn gate_counter_all_is_32_in_spec_order() {
        assert_eq!(GateCounter::ALL.len(), 32);
        assert_eq!(GateCounter::ALL[0], GateCounter::GateL0Candidates);
        assert_eq!(GateCounter::ALL[31], GateCounter::GateReplayMismatch);
        for (index, counter) in GateCounter::ALL.into_iter().enumerate() {
            assert_eq!(counter.index(), index);
        }
    }

    #[test]
    fn gate_counters_bump_is_monotonic_and_snapshot_is_32() {
        let mut counters = GateCounters::default();
        for _ in 0..3 {
            counters.bump(GateCounter::GateL1Paused);
        }
        assert_eq!(counters.get(GateCounter::GateL1Paused), 3);
        let snapshot = counters.snapshot();
        assert_eq!(snapshot.len(), 32);
        let restored = GateCounters::from_snapshot(snapshot);
        assert_eq!(restored, counters);
        assert_eq!(restored.get(GateCounter::GateL1Paused), 3);
    }

    #[test]
    fn gate_counter_names_match_spec_field_names() {
        for (index, counter) in GateCounter::ALL.into_iter().enumerate() {
            let encoded = serde_json::to_string(&counter).expect("serde");
            assert_eq!(encoded, format!("\"{}\"", COUNTER_NAMES[index]));
        }
    }

    #[test]
    fn threshold_profile_from_milli_is_reachable() {
        assert_eq!(
            ThresholdProfile::from_milli(720),
            Ok(ThresholdProfile::Strict)
        );
        assert_eq!(
            ThresholdProfile::from_milli(620),
            Ok(ThresholdProfile::Balanced)
        );
        assert_eq!(
            ThresholdProfile::from_milli(520),
            Ok(ThresholdProfile::Loose)
        );
        assert_eq!(
            ThresholdProfile::from_milli(500),
            Err(GatePolicyError::ThresholdProfileInvalid(500))
        );
        assert_eq!(ThresholdProfile::default().milli(), 620);
        assert_eq!(FrequencyProfile::default().per_window(), 3);
    }

    #[test]
    fn policy_validate_rejects_out_of_range_and_fixes_query_limits() {
        let from_empty: GatePolicy = serde_json::from_str("{}").expect("default");
        assert_eq!(from_empty, GatePolicy::default());
        assert_eq!(from_empty.validate(), Ok(()));
        assert_eq!(from_empty.session_outbound_cap, 120);
        assert_eq!(from_empty.auto_query_limit, 80);
        assert_eq!(from_empty.on_demand_query_limit, 200);
        assert_eq!(from_empty.max_cache_age_ms, 86_400_000);

        let policy = GatePolicy {
            session_outbound_cap: 601,
            ..GatePolicy::default()
        };
        assert_eq!(
            policy.validate(),
            Err(GatePolicyError::SessionCapOutOfRange(601))
        );

        let policy = GatePolicy {
            session_outbound_cap: 0,
            ..GatePolicy::default()
        };
        assert_eq!(policy.validate(), Ok(()));

        let policy = GatePolicy {
            auto_query_limit: 81,
            ..GatePolicy::default()
        };
        assert!(matches!(
            policy.validate(),
            Err(GatePolicyError::QueryLimitNotFixed { auto: 81, .. })
        ));

        let policy = GatePolicy {
            on_demand_query_limit: 201,
            ..GatePolicy::default()
        };
        assert!(matches!(
            policy.validate(),
            Err(GatePolicyError::QueryLimitNotFixed { on_demand: 201, .. })
        ));
    }

    #[test]
    fn policy_rejects_skip_g1_g4_g5() {
        for (key, stage) in [("skip_g1", 1), ("skip_g4", 4), ("skip_g5", 5)] {
            let raw = format!("{{\"{key}\":true}}");
            let policy: GatePolicy = serde_json::from_str(&raw).expect("decode");
            assert_eq!(
                policy.validate(),
                Err(GatePolicyError::MandatoryStageCannotSkip { stage })
            );
            assert!(policy.unknown.contains_key(key));
        }
        let policy: GatePolicy = serde_json::from_str("{\"skip_foo\":true}").expect("decode");
        assert_eq!(
            policy.validate(),
            Err(GatePolicyError::MandatoryStageCannotSkip { stage: 0 })
        );
        let policy: GatePolicy =
            serde_json::from_str("{\"skip_g1\":true,\"session_outbound_cap\":601}")
                .expect("decode");
        assert_eq!(
            policy.validate(),
            Err(GatePolicyError::MandatoryStageCannotSkip { stage: 1 })
        );
    }

    #[test]
    fn policy_keeps_unknown_fields_roundtrip() {
        let raw = "{\"future_flag\":1,\"future_nested\":{\"a\":\"b\"}}";
        let policy: GatePolicy = serde_json::from_str(raw).expect("decode");
        assert!(policy.unknown.contains_key("future_flag"));
        assert!(policy.unknown.contains_key("future_nested"));
        let encoded = serde_json::to_value(&policy).expect("encode");
        let object = encoded.as_object().expect("object");
        assert_eq!(object.get("future_flag"), Some(&serde_json::json!(1)));
        assert_eq!(
            object.get("future_nested"),
            Some(&serde_json::json!({"a": "b"}))
        );
    }

    fn expected_counter(reason: GateRejectReason, stage: GateStage) -> Option<GateCounter> {
        use GateCounter::*;
        use GateRejectReason as R;
        use GateStage::*;
        match (reason, stage) {
            (R::PreFilterLowConfidence, G1) => Some(GateL1LowConfidence),
            (R::ConsentMissing, G1) => Some(GateL1ConsentMissing),
            (R::AutoDisabled, G1) => Some(GateL1AutoDisabled),
            (R::Paused, G1) => Some(GateL1Paused),
            (R::SilentPeriod, G1) => Some(GateL1SilentPeriod),
            (R::ScreenShareProtection, G1) => Some(GateL1ScreenShare),
            (R::SensitiveTerm, G1) => Some(GateL1SensitiveTerm),
            (R::Duplicate, G1) => Some(GateL1Duplicate),
            (R::Duplicate, G3) => Some(GateL3Duplicate),
            (R::Suppressed, G1) => Some(GateL1Suppressed),
            (R::Suppressed, G3) => Some(GateL3Suppressed),
            (R::QuotaExhausted, G1) => Some(GateL1QuotaExhausted),
            (R::QuotaExhausted, G3) => Some(GateL3Quota),
            (R::NoProviderEnabled, G1) => Some(GateL1NoProvider),
            (R::LowRelevance, G2) => Some(GateL2LowRelevance),
            (R::Stale, G3) => Some(GateL3Stale),
            (R::BudgetExhausted, G4) => Some(GateL4BudgetExhausted),
            (R::GatewayCooling, G4) => Some(GateL4GatewayCooling),
            (R::Offline, G4) => Some(GateL4Offline),
            (R::OutboundNotGranted, G5) => Some(GateL5NotGranted),
            (R::SanitizeFailed, G5) => Some(GateL5SanitizeFailed),
            (R::AuditUnavailable, G5) => Some(GateL5AuditUnavailable),
            (R::ProviderNotAllowed, G5) | (R::DomainBlocked, G5) => {
                Some(GateL5ProviderOrDomainBlocked)
            }
            _ => None,
        }
    }

    #[test]
    fn bump_by_reason_matches_static_table() {
        let mut combinations = 0;
        for reason in GateRejectReason::ALL {
            for stage in GateStage::ALL {
                combinations += 1;
                assert_eq!(
                    reason.counter_at(stage),
                    expected_counter(reason, stage),
                    "{reason:?} at {stage:?}"
                );
            }
        }
        assert_eq!(combinations, 115);
    }

    #[test]
    fn duplicate_splits_by_stage() {
        assert_eq!(
            GateRejectReason::Duplicate.counter_at(GateStage::G1),
            Some(GateCounter::GateL1Duplicate)
        );
        assert_eq!(
            GateRejectReason::Duplicate.counter_at(GateStage::G3),
            Some(GateCounter::GateL3Duplicate)
        );
        assert_eq!(GateRejectReason::Duplicate.counter_at(GateStage::G4), None);
        assert_eq!(
            GateRejectReason::PolicyUnavailable.counter_at(GateStage::G1),
            None
        );
        for stage in GateStage::ALL {
            assert_eq!(GateRejectReason::PolicyUnavailable.counter_at(stage), None);
            assert_eq!(GateRejectReason::RetentionExpired.counter_at(stage), None);
        }
    }

    #[test]
    fn no_provider_is_counted_once() {
        assert_eq!(
            GateRejectReason::NoProviderEnabled.counter_at(GateStage::G1),
            Some(GateCounter::GateL1NoProvider)
        );
        for stage in GateStage::ALL {
            if stage == GateStage::G1 {
                continue;
            }
            assert_eq!(GateRejectReason::NoProviderEnabled.counter_at(stage), None);
        }
    }
}
