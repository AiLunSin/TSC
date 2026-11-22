//! 51 视图模型：只读投影 13 字段卡片/11 项预览，点击只展开本地详情。

use meeting_copilot_application::cards::ComposedCard;
use meeting_copilot_domain::model::enums::TriggerKind;
use meeting_copilot_domain::model::gate_keyword::{
    CitationAlignment, ClickDisabledReason, ClickState, KeywordCardDraft, KeywordCardStatus,
    KeywordKind, ReferenceView,
};
use meeting_copilot_domain::vo::{CardRef, ConclusionText, ReferenceRef, TitleText};

#[derive(Clone, PartialEq, serde::Serialize)]
pub struct KeywordCardView {
    pub card_id: CardRef,
    pub keyword_kind: KeywordKind,
    pub trigger: TriggerKind,
    pub triggered_at_ms: u64,
    pub card_status: KeywordCardStatus,
    pub body_visible: bool,
    pub body: Option<CardBodyView>,
    pub click_state: ClickState,
    pub citation_alignment: CitationAlignment,
    pub badge_count: u32,
}

impl core::fmt::Debug for KeywordCardView {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("KeywordCardView")
            .field("card_id", &self.card_id)
            .field("keyword_kind", &self.keyword_kind)
            .field("trigger", &self.trigger)
            .field("card_status", &self.card_status)
            .field("body_visible", &self.body_visible)
            .field("has_body", &self.body.is_some())
            .field("click_state", &self.click_state)
            .field("citation_alignment", &self.citation_alignment)
            .field("badge_count", &self.badge_count)
            .finish()
    }
}

#[derive(Clone, PartialEq, serde::Serialize)]
pub struct CardBodyView {
    pub title: TitleText,
    pub conclusion: ConclusionView,
    pub confidence: meeting_copilot_domain::vo::Confidence,
    pub references: Vec<ReferenceView>,
}

impl core::fmt::Debug for CardBodyView {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CardBodyView")
            .field("has_title", &!self.title.as_str().is_empty())
            .field("conclusion", &self.conclusion)
            .field("confidence", &self.confidence)
            .field("reference_count", &self.references.len())
            .finish()
    }
}

#[derive(Clone, PartialEq, serde::Serialize)]
pub enum ConclusionView {
    Available(ConclusionText),
    NoReliableSource,
    Absent,
}

impl core::fmt::Debug for ConclusionView {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Available(_) => f.write_str("Available"),
            Self::NoReliableSource => f.write_str("NoReliableSource"),
            Self::Absent => f.write_str("Absent"),
        }
    }
}

#[derive(Clone, PartialEq, serde::Serialize)]
pub struct CardDetailView {
    pub card_id: CardRef,
    pub title: TitleText,
    pub conclusion: ConclusionView,
    pub reference_ids: Vec<ReferenceRef>,
    pub citation_alignment: CitationAlignment,
}

impl core::fmt::Debug for CardDetailView {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CardDetailView")
            .field("card_id", &self.card_id)
            .field("conclusion", &self.conclusion)
            .field("reference_count", &self.reference_ids.len())
            .field("citation_alignment", &self.citation_alignment)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewFacts {
    pub session_matches: bool,
    pub authorization_revoked: bool,
    pub reference_stale: bool,
    pub screen_share: bool,
    pub emergency_masked: bool,
    pub visible_card_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum ViewRejectReason {
    InputContractViolation,
    ReferenceContractViolation,
    CardWithdrawn,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CardViewOutcome {
    pub view: Option<KeywordCardView>,
    pub reason: Option<ViewRejectReason>,
}

impl CardViewOutcome {
    pub fn is_xor(&self) -> bool {
        self.view.is_some() != self.reason.is_some()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClickOutcome {
    pub detail: Option<CardDetailView>,
    pub reason: Option<ClickDisabledReason>,
}

impl ClickOutcome {
    pub fn is_xor(&self) -> bool {
        self.detail.is_some() != self.reason.is_some()
    }
}

fn sorted_references(references: &[ReferenceView]) -> Vec<ReferenceView> {
    let mut sorted = references.to_vec();
    sorted.sort_by(|left, right| left.reference_id.as_str().cmp(right.reference_id.as_str()));
    sorted
}

fn has_duplicate_reference_id(references: &[ReferenceView]) -> bool {
    let mut ids = std::collections::BTreeSet::new();
    let unique = references
        .iter()
        .all(|view| ids.insert(view.reference_id.as_str().to_owned()));
    !unique
}

fn validate_draft(card: &KeywordCardDraft) -> Option<ViewRejectReason> {
    if card.card_status == KeywordCardStatus::Withdrawn {
        return Some(ViewRejectReason::CardWithdrawn);
    }
    if !card.summary_is_consistent() {
        return Some(ViewRejectReason::InputContractViolation);
    }
    if card.citation_alignment == CitationAlignment::Passed && card.references.is_empty() {
        return Some(ViewRejectReason::InputContractViolation);
    }
    None
}

fn validate_composed(card: &ComposedCard) -> Option<ViewRejectReason> {
    if !card.summary_is_consistent() || card.card_status == KeywordCardStatus::Collapsed {
        return Some(ViewRejectReason::InputContractViolation);
    }
    if card.citation_alignment == CitationAlignment::Passed && card.references.is_empty() {
        return Some(ViewRejectReason::InputContractViolation);
    }
    None
}

fn conclusion_of(alignment: CitationAlignment, summary: &Option<ConclusionText>) -> ConclusionView {
    match summary {
        Some(text) if alignment == CitationAlignment::Passed => {
            ConclusionView::Available(text.clone())
        }
        Some(text) if text.as_str() == KeywordCardDraft::NO_RELIABLE_SOURCE => {
            ConclusionView::NoReliableSource
        }
        Some(_) => ConclusionView::NoReliableSource,
        None => ConclusionView::Absent,
    }
}

fn mask_state(facts: &ViewFacts) -> Option<ClickDisabledReason> {
    if facts.emergency_masked {
        Some(ClickDisabledReason::EmergencyMasked)
    } else if facts.screen_share {
        Some(ClickDisabledReason::ScreenShareProtection)
    } else {
        None
    }
}

fn resolve_ui_click_state(
    input: ClickState,
    alignment: CitationAlignment,
    card_status: KeywordCardStatus,
    facts: &ViewFacts,
) -> ClickState {
    if card_status == KeywordCardStatus::Withdrawn {
        return ClickState::Disabled(ClickDisabledReason::NotAllowed);
    }
    if facts.emergency_masked {
        return ClickState::Disabled(ClickDisabledReason::EmergencyMasked);
    }
    if facts.screen_share {
        return ClickState::Disabled(ClickDisabledReason::ScreenShareProtection);
    }
    if facts.authorization_revoked {
        return ClickState::Disabled(ClickDisabledReason::AuthorizationRevoked);
    }
    if !facts.session_matches || facts.reference_stale {
        return ClickState::Disabled(ClickDisabledReason::StaleReference);
    }
    if let ClickState::Disabled(reason) = input {
        return ClickState::Disabled(reason);
    }
    if alignment != CitationAlignment::Passed {
        return ClickState::Disabled(ClickDisabledReason::ReferenceUnverified);
    }
    ClickState::Enabled
}

fn rejected(reason: ViewRejectReason) -> CardViewOutcome {
    CardViewOutcome {
        view: None,
        reason: Some(reason),
    }
}

pub fn project_keyword_card_view(card: &KeywordCardDraft, facts: &ViewFacts) -> CardViewOutcome {
    if let Some(reason) = validate_draft(card) {
        return rejected(reason);
    }
    if has_duplicate_reference_id(&card.references) {
        return rejected(ViewRejectReason::ReferenceContractViolation);
    }
    if card.citation_alignment != CitationAlignment::Passed
        && card.summary.is_some()
        && card.summary.as_ref().map(|text| text.as_str())
            != Some(KeywordCardDraft::NO_RELIABLE_SOURCE)
    {
        return rejected(ViewRejectReason::InputContractViolation);
    }

    let references = sorted_references(&card.references);
    let conclusion = conclusion_of(card.citation_alignment, &card.summary);
    let mask = mask_state(facts);
    let click_state = resolve_ui_click_state(
        card.click_state,
        card.citation_alignment,
        card.card_status,
        facts,
    );
    let body = if mask.is_some() {
        None
    } else {
        Some(CardBodyView {
            title: card.keyword_text.clone(),
            conclusion,
            confidence: card.confidence,
            references,
        })
    };
    CardViewOutcome {
        view: Some(KeywordCardView {
            card_id: card.card_id.clone(),
            keyword_kind: card.keyword_kind,
            trigger: card.trigger,
            triggered_at_ms: card.triggered_at_ms,
            card_status: card.card_status,
            body_visible: body.is_some(),
            body,
            click_state,
            citation_alignment: card.citation_alignment,
            badge_count: facts.visible_card_count.max(1),
        }),
        reason: None,
    }
}

pub fn project_composed_card_view(card: &ComposedCard, facts: &ViewFacts) -> CardViewOutcome {
    if let Some(reason) = validate_composed(card) {
        return rejected(reason);
    }
    if has_duplicate_reference_id(&card.references) {
        return rejected(ViewRejectReason::ReferenceContractViolation);
    }
    let references = sorted_references(&card.references);
    let conclusion = conclusion_of(card.citation_alignment, &card.summary);
    let mask = mask_state(facts);
    let body = if mask.is_some() {
        None
    } else {
        Some(CardBodyView {
            title: card.keyword_text.clone(),
            conclusion,
            confidence: card.confidence,
            references,
        })
    };
    CardViewOutcome {
        view: Some(KeywordCardView {
            card_id: card.card_id.clone(),
            keyword_kind: card.keyword_kind,
            trigger: card.trigger,
            triggered_at_ms: card.triggered_at_ms,
            card_status: card.card_status,
            body_visible: body.is_some(),
            body,
            click_state: ClickState::Disabled(ClickDisabledReason::NotAllowed),
            citation_alignment: card.citation_alignment,
            badge_count: facts.visible_card_count.max(1),
        }),
        reason: None,
    }
}

pub fn open_keyword_card_detail(view: &KeywordCardView) -> ClickOutcome {
    match (&view.click_state, &view.body) {
        (ClickState::Enabled, Some(body)) => ClickOutcome {
            detail: Some(CardDetailView {
                card_id: view.card_id.clone(),
                title: body.title.clone(),
                conclusion: body.conclusion.clone(),
                reference_ids: body
                    .references
                    .iter()
                    .map(|view| view.reference_id.clone())
                    .collect(),
                citation_alignment: view.citation_alignment,
            }),
            reason: None,
        },
        (ClickState::Disabled(reason), _) => ClickOutcome {
            detail: None,
            reason: Some(*reason),
        },
        _ => ClickOutcome {
            detail: None,
            reason: Some(ClickDisabledReason::NotAllowed),
        },
    }
}
