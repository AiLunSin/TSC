//! 关键词卡片契约：候选、引用视图、卡片草案、点击态与 Allow 溯源。

use serde::{Deserialize, Serialize};

use crate::model::card::DedupeKey;
use crate::model::enums::{AccessStatus, CardStatus, GapState, TriggerKind, TrustTier};
use crate::model::gate::{GapRule, GateDecision, GateStage};
use crate::model::session::TextRange;
use crate::vo::{
    CardRef, ConclusionText, Confidence, InsightRef, KeywordRef, NormalizedTerm, ReferenceRef,
    SessionId, TitleText, TraceId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeywordKind {
    ProperNoun,
    Acronym,
    InstitutionRole,
    PolicyStandard,
    Metric,
}

impl KeywordKind {
    pub const ALL: [Self; 5] = [
        Self::ProperNoun,
        Self::Acronym,
        Self::InstitutionRole,
        Self::PolicyStandard,
        Self::Metric,
    ];
}

/// 五类映射；`UnansweredQuestion` 不出关键词卡。
pub const fn keyword_kind_of(rule: GapRule) -> Option<KeywordKind> {
    match rule {
        GapRule::ProperNoun => Some(KeywordKind::ProperNoun),
        GapRule::Abbreviation => Some(KeywordKind::Acronym),
        GapRule::Institution => Some(KeywordKind::InstitutionRole),
        GapRule::PolicyStandard => Some(KeywordKind::PolicyStandard),
        GapRule::Metric => Some(KeywordKind::Metric),
        GapRule::UnansweredQuestion => None,
        GapRule::Acronym => Some(KeywordKind::Acronym),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeywordCardStatus {
    Displayed,
    Collapsed,
    Withdrawn,
}

pub const fn card_status_from(status: CardStatus) -> KeywordCardStatus {
    match status {
        CardStatus::Shown => KeywordCardStatus::Displayed,
        CardStatus::Collapsed => KeywordCardStatus::Collapsed,
        CardStatus::Withdrawn => KeywordCardStatus::Withdrawn,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceTrustTier {
    Official,
    AuthoritativeMedia,
    GeneralWeb,
    UserGenerated,
}

pub const fn trust_tier_from(tier: TrustTier) -> ReferenceTrustTier {
    match tier {
        TrustTier::Official => ReferenceTrustTier::Official,
        TrustTier::AuthoritativeMedia => ReferenceTrustTier::AuthoritativeMedia,
        TrustTier::GeneralWeb => ReferenceTrustTier::GeneralWeb,
        TrustTier::UserGenerated => ReferenceTrustTier::UserGenerated,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceAccessStatus {
    Available,
    Unavailable,
}

pub const fn access_status_from(status: AccessStatus) -> ReferenceAccessStatus {
    match status {
        AccessStatus::Ok => ReferenceAccessStatus::Available,
        AccessStatus::LoginWall
        | AccessStatus::RobotsDenied
        | AccessStatus::Timeout
        | AccessStatus::Blocked => ReferenceAccessStatus::Unavailable,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CitationAlignment {
    Passed,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClickDisabledReason {
    StaleReference,
    AuthorizationRevoked,
    NotAllowed,
    EmergencyMasked,
    ScreenShareProtection,
    ReferenceUnverified,
}

impl ClickDisabledReason {
    pub const ALL: [Self; 6] = [
        Self::StaleReference,
        Self::AuthorizationRevoked,
        Self::NotAllowed,
        Self::EmergencyMasked,
        Self::ScreenShareProtection,
        Self::ReferenceUnverified,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClickState {
    Enabled,
    Disabled(ClickDisabledReason),
}

/// 只由 `Allow` 构造：Deny 路径无伪造入口。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllowTraceRef {
    trace_id: TraceId,
    stopped_at: Option<GateStage>,
}

impl AllowTraceRef {
    pub fn new(decision: &GateDecision) -> Option<Self> {
        match decision {
            GateDecision::Allow { stopped_at, .. } => Some(Self {
                trace_id: TraceId::new("allow-trace").ok()?,
                stopped_at: *stopped_at,
            }),
            GateDecision::Deny { .. } => None,
        }
    }

    pub fn with_trace(trace_id: TraceId, stopped_at: Option<GateStage>) -> Self {
        Self {
            trace_id,
            stopped_at,
        }
    }

    pub fn trace_id(&self) -> &TraceId {
        &self.trace_id
    }

    pub fn stopped_at(&self) -> Option<GateStage> {
        self.stopped_at
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeywordCandidate {
    pub keyword_ref: KeywordRef,
    pub keyword_text: TitleText,
    pub normalized_term: NormalizedTerm,
    pub keyword_kind: KeywordKind,
    pub trigger_span: TextRange,
    pub confidence: Confidence,
    pub dedupe_key: DedupeKey,
    pub state: GapState,
    pub session_id: SessionId,
    pub source_insight_ref: Option<InsightRef>,
}

impl KeywordCandidate {
    pub fn validate(&self) -> bool {
        self.confidence.value() >= 0.50
            && self.trigger_span.start_ms < self.trigger_span.end_ms
            && self.state != GapState::Pushed
            && !self.keyword_text.as_str().trim().is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceView {
    pub reference_id: ReferenceRef,
    pub trust_tier: ReferenceTrustTier,
    pub access_status: ReferenceAccessStatus,
    pub published_at_ms: Option<u64>,
    pub alignment: CitationAlignment,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeywordCardDraft {
    pub card_id: CardRef,
    pub keyword_text: TitleText,
    pub normalized_term: NormalizedTerm,
    pub keyword_kind: KeywordKind,
    pub trigger: TriggerKind,
    pub triggered_at_ms: u64,
    pub confidence: Confidence,
    pub summary: Option<ConclusionText>,
    pub references: Vec<ReferenceView>,
    pub citation_alignment: CitationAlignment,
    pub gate_decision_ref: AllowTraceRef,
    pub card_status: KeywordCardStatus,
    pub click_state: ClickState,
}

impl KeywordCardDraft {
    pub const NO_RELIABLE_SOURCE: &'static str = "未找到可靠来源";

    pub fn summary_is_consistent(&self) -> bool {
        match &self.summary {
            None => true,
            Some(summary) => {
                self.citation_alignment == CitationAlignment::Passed
                    || summary.as_str() == Self::NO_RELIABLE_SOURCE
            }
        }
    }

    pub fn click_state_is_consistent(&self) -> bool {
        match self.click_state {
            ClickState::Enabled => {
                self.citation_alignment == CitationAlignment::Passed
                    && self.card_status != KeywordCardStatus::Withdrawn
            }
            ClickState::Disabled(_) => true,
        }
    }

    pub fn is_withdrawn(&self) -> bool {
        self.card_status == KeywordCardStatus::Withdrawn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(confidence: f64) -> KeywordCandidate {
        KeywordCandidate {
            keyword_ref: KeywordRef::new("kw-1").expect("ref"),
            keyword_text: TitleText::new("示例术语").expect("text"),
            normalized_term: NormalizedTerm::new("示例术语").expect("term"),
            keyword_kind: KeywordKind::ProperNoun,
            trigger_span: TextRange {
                start_ms: 0,
                end_ms: 10,
            },
            confidence: Confidence::new(confidence).expect("confidence"),
            dedupe_key: DedupeKey::new("a".repeat(32)).expect("key"),
            state: GapState::Detected,
            session_id: SessionId::new("s-1").expect("session"),
            source_insight_ref: None,
        }
    }

    fn card(alignment: CitationAlignment) -> KeywordCardDraft {
        KeywordCardDraft {
            card_id: CardRef::new("card-1").expect("card"),
            keyword_text: TitleText::new("示例术语").expect("text"),
            normalized_term: NormalizedTerm::new("示例术语").expect("term"),
            keyword_kind: KeywordKind::Metric,
            trigger: TriggerKind::Auto,
            triggered_at_ms: 0,
            confidence: Confidence::new(0.9).expect("confidence"),
            summary: Some(ConclusionText::new("结论").expect("summary")),
            references: vec![ReferenceView {
                reference_id: ReferenceRef::new("ref-1").expect("ref"),
                trust_tier: ReferenceTrustTier::Official,
                access_status: ReferenceAccessStatus::Available,
                published_at_ms: Some(0),
                alignment,
            }],
            citation_alignment: alignment,
            gate_decision_ref: AllowTraceRef::with_trace(TraceId::new("t-1").expect("trace"), None),
            card_status: KeywordCardStatus::Displayed,
            click_state: ClickState::Disabled(ClickDisabledReason::ReferenceUnverified),
        }
    }

    #[test]
    fn keyword_card_has_thirteen_fields() {
        let mut values = serde_json::to_value(card(CitationAlignment::Passed)).expect("value");
        let object = values.as_object_mut().expect("object");
        assert_eq!(object.len(), 13);
        assert!(object.contains_key("gate_decision_ref"));
    }

    #[test]
    fn keyword_reference_view_has_five_fields() {
        let view = ReferenceView {
            reference_id: ReferenceRef::new("ref-1").expect("ref"),
            trust_tier: ReferenceTrustTier::GeneralWeb,
            access_status: ReferenceAccessStatus::Available,
            published_at_ms: None,
            alignment: CitationAlignment::Passed,
        };
        let value = serde_json::to_value(&view).expect("value");
        assert_eq!(value.as_object().expect("object").len(), 5);
    }

    #[test]
    fn keyword_candidate_has_ten_fields() {
        let value = serde_json::to_value(candidate(0.9)).expect("value");
        assert_eq!(value.as_object().expect("object").len(), 10);
    }

    #[test]
    fn keyword_enums_are_closed_and_projected() {
        assert_eq!(KeywordKind::ALL.len(), 5);
        assert_eq!(
            keyword_kind_of(GapRule::Institution),
            Some(KeywordKind::InstitutionRole)
        );
        assert_eq!(keyword_kind_of(GapRule::UnansweredQuestion), None);
        assert_eq!(ClickDisabledReason::ALL.len(), 6);
        assert_eq!(
            card_status_from(CardStatus::Shown),
            KeywordCardStatus::Displayed
        );
        assert_eq!(
            access_status_from(AccessStatus::LoginWall),
            ReferenceAccessStatus::Unavailable
        );
    }

    #[test]
    fn allow_trace_ref_is_only_from_allow() {
        let deny = GateDecision::Deny {
            reason: crate::model::gate::GateRejectReason::Paused,
            stopped_at: GateStage::G1,
            per_stage: crate::model::gate::PerStageResults::empty(),
            cooldown_until_ms: None,
        };
        assert!(AllowTraceRef::new(&deny).is_none());
        let allow = GateDecision::Allow {
            per_stage: crate::model::gate::PerStageResults::empty(),
            budget_remaining: 1,
            candidates: Vec::new(),
            outbound: None,
            suppressed_notice: false,
            stopped_at: Some(GateStage::G1),
        };
        assert!(AllowTraceRef::new(&allow).is_some());
    }

    #[test]
    fn keyword_card_predicates_fail_closed() {
        let failed = card(CitationAlignment::Failed);
        assert!(!failed.summary_is_consistent());
        let mut enabled = failed.clone();
        enabled.click_state = ClickState::Enabled;
        assert!(!enabled.click_state_is_consistent());
        let mut no_source = card(CitationAlignment::Failed);
        no_source.summary =
            Some(ConclusionText::new(KeywordCardDraft::NO_RELIABLE_SOURCE).expect("ok"));
        assert!(no_source.summary_is_consistent());
    }
}
