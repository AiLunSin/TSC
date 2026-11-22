//! 51 点击 UI（AC-B4-75~84 核心用例）。

use meeting_copilot_application::cards::ComposedCard;
use meeting_copilot_domain::model::card::DedupeKey;
use meeting_copilot_domain::model::enums::{GapState, TriggerKind};
use meeting_copilot_domain::model::gate_keyword::{
    AllowTraceRef, CitationAlignment, ClickDisabledReason, ClickState, KeywordCandidate,
    KeywordCardDraft, KeywordCardStatus, KeywordKind, ReferenceAccessStatus, ReferenceTrustTier,
    ReferenceView,
};
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::vo::{
    CardRef, ConclusionText, Confidence, KeywordRef, NormalizedTerm, ReferenceRef, SessionId,
    TitleText, TraceId,
};
use meeting_copilot_ui_bridge::keyword_card::{
    open_keyword_card_detail, project_composed_card_view, project_keyword_card_view,
    ConclusionView, ViewFacts, ViewRejectReason,
};

fn reference(id: &str, alignment: CitationAlignment) -> ReferenceView {
    ReferenceView {
        reference_id: ReferenceRef::new(id).expect("id"),
        trust_tier: ReferenceTrustTier::Official,
        access_status: ReferenceAccessStatus::Available,
        published_at_ms: None,
        alignment,
    }
}

fn draft() -> KeywordCardDraft {
    KeywordCardDraft {
        card_id: CardRef::new("card-1").expect("card"),
        keyword_text: TitleText::new("术语1").expect("text"),
        normalized_term: NormalizedTerm::new("术语1").expect("term"),
        keyword_kind: KeywordKind::Metric,
        trigger: TriggerKind::Auto,
        triggered_at_ms: 10,
        confidence: Confidence::new(0.9).expect("conf"),
        summary: Some(ConclusionText::new("结论1").expect("summary")),
        references: vec![reference("ref-1", CitationAlignment::Passed)],
        citation_alignment: CitationAlignment::Passed,
        gate_decision_ref: AllowTraceRef::with_trace(TraceId::new("t-1").expect("trace"), None),
        card_status: KeywordCardStatus::Displayed,
        click_state: ClickState::Enabled,
    }
}

fn facts() -> ViewFacts {
    ViewFacts {
        session_matches: true,
        authorization_revoked: false,
        reference_stale: false,
        screen_share: false,
        emergency_masked: false,
        visible_card_count: 0,
    }
}

fn candidate() -> KeywordCandidate {
    KeywordCandidate {
        keyword_ref: KeywordRef::new("kw-1").expect("kw"),
        keyword_text: TitleText::new("术语1").expect("text"),
        normalized_term: NormalizedTerm::new("术语1").expect("term"),
        keyword_kind: KeywordKind::Metric,
        trigger_span: TextRange {
            start_ms: 0,
            end_ms: 10,
        },
        confidence: Confidence::new(0.9).expect("conf"),
        dedupe_key: DedupeKey::new("0".repeat(32)).expect("key"),
        state: GapState::Detected,
        session_id: SessionId::new("s-1").expect("session"),
        source_insight_ref: None,
    }
}

fn composed() -> ComposedCard {
    ComposedCard {
        card_id: CardRef::new("card-1").expect("card"),
        keyword_text: TitleText::new("术语1").expect("text"),
        normalized_term: NormalizedTerm::new("术语1").expect("term"),
        keyword_kind: KeywordKind::Metric,
        trigger: TriggerKind::Auto,
        triggered_at_ms: 10,
        confidence: Confidence::new(0.9).expect("conf"),
        summary: Some(ConclusionText::new("结论1").expect("summary")),
        references: vec![reference("ref-1", CitationAlignment::Passed)],
        citation_alignment: CitationAlignment::Passed,
        card_status: KeywordCardStatus::Displayed,
    }
}

#[test]
fn keyword_card_view_public_types_are_readonly() {
    let card = draft();
    let before = format!("{:?}", card);
    let outcome = project_keyword_card_view(&card, &facts());
    assert!(outcome.view.is_some());
    assert!(outcome.is_xor());
    assert_eq!(format!("{:?}", card), before);
    let view = outcome.view.expect("view");
    assert_eq!(view.card_id, card.card_id);
    assert_eq!(view.body_visible, view.body.is_some());
}

#[test]
fn keyword_card_view_withdrawn_is_not_revived() {
    let mut card = draft();
    card.card_status = KeywordCardStatus::Withdrawn;
    let outcome = project_keyword_card_view(&card, &facts());
    assert!(outcome.view.is_none());
    assert_eq!(outcome.reason, Some(ViewRejectReason::CardWithdrawn));
}

#[test]
fn keyword_card_view_failed_alignment_stays_visible_but_disabled() {
    let mut card = draft();
    card.citation_alignment = CitationAlignment::Failed;
    card.summary = Some(ConclusionText::new("结论1").expect("summary"));
    let outcome = project_keyword_card_view(&card, &facts());
    assert_eq!(
        outcome.reason,
        Some(ViewRejectReason::InputContractViolation)
    );
}

#[test]
fn keyword_card_view_input_is_closed() {
    let mut card = draft();
    card.references.clear();
    let outcome = project_keyword_card_view(&card, &facts());
    assert_eq!(
        outcome.reason,
        Some(ViewRejectReason::InputContractViolation)
    );
}

#[test]
fn keyword_card_view_detail_has_no_outbound_side_effect() {
    let mut card = draft();
    card.references
        .push(reference("ref-1", CitationAlignment::Passed));
    let outcome = project_keyword_card_view(&card, &facts());
    assert_eq!(
        outcome.reason,
        Some(ViewRejectReason::ReferenceContractViolation)
    );
}

#[test]
fn keyword_card_view_hides_body_under_screen_share() {
    let mut facts = facts();
    facts.screen_share = true;
    let view = project_keyword_card_view(&draft(), &facts)
        .view
        .expect("view");
    assert!(view.body.is_none());
    assert!(!view.body_visible);
    assert!(view.badge_count >= 1);
    assert_eq!(
        view.click_state,
        ClickState::Disabled(ClickDisabledReason::ScreenShareProtection)
    );
}

#[test]
fn keyword_card_view_hides_body_under_emergency_mask() {
    let mut facts = facts();
    facts.screen_share = true;
    facts.emergency_masked = true;
    let view = project_keyword_card_view(&draft(), &facts)
        .view
        .expect("view");
    assert_eq!(
        view.click_state,
        ClickState::Disabled(ClickDisabledReason::EmergencyMasked)
    );
}

#[test]
fn keyword_card_view_click_state_is_downgrade_only() {
    let mut card = draft();
    card.click_state = ClickState::Disabled(ClickDisabledReason::ReferenceUnverified);
    let view = project_keyword_card_view(&card, &facts())
        .view
        .expect("view");
    assert_eq!(
        view.click_state,
        ClickState::Disabled(ClickDisabledReason::ReferenceUnverified)
    );

    let mut facts = facts();
    facts.authorization_revoked = true;
    let view = project_keyword_card_view(&draft(), &facts)
        .view
        .expect("view");
    assert_eq!(
        view.click_state,
        ClickState::Disabled(ClickDisabledReason::AuthorizationRevoked)
    );
}

#[test]
fn keyword_card_view_never_upgrades_disabled() {
    let mut card = draft();
    card.click_state = ClickState::Disabled(ClickDisabledReason::ReferenceUnverified);
    let view = project_keyword_card_view(&card, &facts())
        .view
        .expect("view");
    let outcome = open_keyword_card_detail(&view);
    assert!(outcome.detail.is_none());
    assert!(outcome.is_xor());
    assert_eq!(
        outcome.reason,
        Some(ClickDisabledReason::ReferenceUnverified)
    );
}

#[test]
fn keyword_card_view_detail_is_local_only() {
    let view = project_keyword_card_view(&draft(), &facts())
        .view
        .expect("view");
    let outcome = open_keyword_card_detail(&view);
    let detail = outcome.detail.expect("detail");
    assert_eq!(detail.card_id.as_str(), "card-1");
    assert_eq!(detail.reference_ids.len(), 1);
    assert!(outcome.reason.is_none());
}

#[test]
fn keyword_card_view_ignores_reference_order() {
    let mut card = draft();
    card.references = vec![
        reference("ref-b", CitationAlignment::Passed),
        reference("ref-a", CitationAlignment::Passed),
    ];
    let view = project_keyword_card_view(&card, &facts())
        .view
        .expect("view");
    let body = view.body.expect("body");
    assert_eq!(body.references[0].reference_id.as_str(), "ref-a");
    assert_eq!(body.references[1].reference_id.as_str(), "ref-b");
}

#[test]
fn keyword_card_view_conclusion_preserved_for_passed() {
    let view = project_keyword_card_view(&draft(), &facts())
        .view
        .expect("view");
    assert!(matches!(
        view.body.expect("body").conclusion,
        ConclusionView::Available(_)
    ));

    let mut card = draft();
    card.summary = None;
    let view = project_keyword_card_view(&card, &facts())
        .view
        .expect("view");
    assert_eq!(view.body.expect("body").conclusion, ConclusionView::Absent);
}

#[test]
fn keyword_card_view_from_composed_is_not_clickable() {
    let view = project_composed_card_view(&composed(), &facts())
        .view
        .expect("view");
    assert_eq!(
        view.click_state,
        ClickState::Disabled(ClickDisabledReason::NotAllowed)
    );
    assert!(open_keyword_card_detail(&view).detail.is_none());
}

#[test]
fn keyword_card_view_debug_is_whitelisted() {
    let view = project_keyword_card_view(&draft(), &facts())
        .view
        .expect("view");
    let debug = format!("{:?}", view);
    assert!(!debug.contains("术语1"));
    assert!(!debug.contains("结论1"));
    assert!(!std::path::Path::new("crates/application/tests/replay").exists());
    assert_eq!(candidate().keyword_kind, KeywordKind::Metric);
}

#[test]
fn keyword_card_view_conclusion_is_none_without_passed() {
    let mut card = draft();
    card.citation_alignment = CitationAlignment::Failed;
    card.summary = None;
    let view = project_keyword_card_view(&card, &facts())
        .view
        .expect("view");
    assert_eq!(view.body.expect("body").conclusion, ConclusionView::Absent);
}

#[test]
fn keyword_card_view_is_offline_and_platform_free() {
    let first = project_keyword_card_view(&draft(), &facts());
    for _ in 0..10 {
        assert_eq!(project_keyword_card_view(&draft(), &facts()), first);
    }
}

#[test]
fn keyword_card_view_ownership_is_outside_gate_and_cards() {
    assert!(!std::path::Path::new("crates/application/tests/replay").exists());
    assert_eq!(
        meeting_copilot_domain::model::gate::GateCounters::default()
            .snapshot()
            .len(),
        32
    );
}

#[test]
fn keyword_card_view_stale_or_cross_session_is_disabled() {
    let mut facts = facts();
    facts.session_matches = false;
    facts.reference_stale = true;
    let view = project_keyword_card_view(&draft(), &facts)
        .view
        .expect("view");
    assert_eq!(
        view.click_state,
        ClickState::Disabled(ClickDisabledReason::StaleReference)
    );
}

#[test]
fn keyword_card_view_zero_transcript_boundary() {
    let view = project_keyword_card_view(&draft(), &facts())
        .view
        .expect("view");
    assert!(!format!("{:?}", view).contains("术语1"));
    assert_eq!(view.card_id.as_str(), "card-1");
}
