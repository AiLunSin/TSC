//! 38 卡片合成（AC-B4-65~74，14 用例）。

use meeting_copilot_application::cards::{
    compose_keyword_card, next_card_status, CardComposeInput, CardStatusSignal,
    ComposeRejectReason, ComposedCard,
};
use meeting_copilot_domain::model::card::DedupeKey;
use meeting_copilot_domain::model::citation::{
    AlignmentBasis, AlignmentVerdict, CitationAlignmentResult, CITATION_CONTRACT_VERSION,
    CONTRACT_UNVERIFIED,
};
use meeting_copilot_domain::model::enums::{GapState, TriggerKind};
use meeting_copilot_domain::model::gate::GateCounters;
use meeting_copilot_domain::model::gate_keyword::{
    AllowTraceRef, CitationAlignment, KeywordCandidate, KeywordCardStatus, KeywordKind,
    ReferenceAccessStatus, ReferenceTrustTier, ReferenceView,
};
use meeting_copilot_domain::model::session::TextRange;
use meeting_copilot_domain::vo::{
    CardRef, Confidence, KeywordRef, NormalizedTerm, ReferenceRef, SessionId, TitleText, TraceId,
};

fn candidate(term: &str, confidence: f64, end_ms: u64) -> KeywordCandidate {
    KeywordCandidate {
        keyword_ref: KeywordRef::new("kw-1").expect("kw"),
        keyword_text: TitleText::new(term).expect("text"),
        normalized_term: NormalizedTerm::new(term).expect("term"),
        keyword_kind: KeywordKind::Metric,
        trigger_span: TextRange {
            start_ms: 0,
            end_ms,
        },
        confidence: Confidence::new(confidence).expect("conf"),
        dedupe_key: DedupeKey::new("0".repeat(32)).expect("key"),
        state: GapState::Detected,
        session_id: SessionId::new("s-1").expect("session"),
        source_insight_ref: None,
    }
}

fn reference(id: &str, alignment: CitationAlignment) -> ReferenceView {
    ReferenceView {
        reference_id: ReferenceRef::new(id).expect("id"),
        trust_tier: ReferenceTrustTier::Official,
        access_status: ReferenceAccessStatus::Available,
        published_at_ms: None,
        alignment,
    }
}

fn result(verdict: AlignmentVerdict, basis: AlignmentBasis) -> CitationAlignmentResult {
    CitationAlignmentResult {
        alignment: verdict,
        basis,
        reference_count: 1,
        accessible_reference_count: 1,
        checked_at_ms: 7,
        contract_version: CITATION_CONTRACT_VERSION,
    }
}

fn input_for(
    term: &str,
    confidence: f64,
    end_ms: u64,
    verdict: AlignmentVerdict,
    alignment: CitationAlignment,
    status: KeywordCardStatus,
) -> CardComposeInput {
    CardComposeInput {
        card_id: CardRef::new("card-1").expect("card"),
        candidate: candidate(term, confidence, end_ms),
        allow_ref: Some(AllowTraceRef::with_trace(
            TraceId::new("t-1").expect("trace"),
            None,
        )),
        alignment_result: result(verdict, AlignmentBasis::SupportedByOfficial),
        references: vec![reference("ref-1", alignment)],
        current_card_status: status,
        suppressed_notice: false,
    }
}

fn default_input() -> CardComposeInput {
    input_for(
        "术语1",
        0.9,
        10,
        AlignmentVerdict::Passed,
        CitationAlignment::Passed,
        KeywordCardStatus::Displayed,
    )
}

#[test]
fn card_compose_input_is_readonly_and_closed() {
    let input = default_input();
    let before = format!("{:?}", input);
    let card = compose_keyword_card(&input).card.expect("card");
    assert_eq!(format!("{:?}", input), before);
    assert_eq!(card.card_id, input.card_id);
}

#[test]
fn card_compose_title_and_summary_limits() {
    let card = compose_keyword_card(&default_input()).card.expect("card");
    assert!(card.keyword_text.as_str().chars().count() <= 40);
    let summary = card.summary.clone().expect("summary");
    assert!(summary.as_str().chars().count() <= 80);
    assert!(card.summary_is_consistent());

    let mut long = default_input();
    long.candidate = candidate(&"字".repeat(40), 0.9, 10);
    let card = compose_keyword_card(&long).card.expect("card");
    assert_eq!(card.keyword_text.as_str().chars().count(), 40);
    if let Some(summary) = &card.summary {
        assert!(summary.as_str().chars().count() <= 80);
    }
}

#[test]
fn card_compose_preserves_confidence_and_trigger() {
    let input = input_for(
        "术语1",
        0.87,
        42,
        AlignmentVerdict::Passed,
        CitationAlignment::Passed,
        KeywordCardStatus::Displayed,
    );
    let card = compose_keyword_card(&input).card.expect("card");
    assert_eq!(card.confidence.value(), 0.87);
    assert_eq!(card.triggered_at_ms, 42);
    assert_eq!(card.trigger, TriggerKind::Auto);
    assert_eq!(card.keyword_kind, input.candidate.keyword_kind);
}

#[test]
fn card_compose_requires_gate_allow() {
    let mut input = default_input();
    input.allow_ref = None;
    assert_eq!(
        compose_keyword_card(&input).reason,
        Some(ComposeRejectReason::GateNotAllowed)
    );
    assert!(compose_keyword_card(&default_input()).card.is_some());
}

#[test]
fn card_compose_is_idempotent_per_candidate() {
    let input = default_input();
    let first = compose_keyword_card(&input).card.expect("card");
    let second = compose_keyword_card(&input).card.expect("card");
    assert_eq!(first, second);
}

#[test]
fn card_compose_withdrawn_is_not_revived() {
    let input = input_for(
        "术语1",
        0.9,
        10,
        AlignmentVerdict::Passed,
        CitationAlignment::Passed,
        KeywordCardStatus::Withdrawn,
    );
    assert_eq!(
        compose_keyword_card(&input).reason,
        Some(ComposeRejectReason::CardWithdrawn)
    );
    assert_eq!(
        next_card_status(KeywordCardStatus::Withdrawn, CardStatusSignal::Present),
        KeywordCardStatus::Withdrawn
    );
}

#[test]
fn card_compose_aligns_unknown_to_unverified() {
    let input = input_for(
        "术语1",
        0.9,
        10,
        AlignmentVerdict::Unknown,
        CitationAlignment::Passed,
        KeywordCardStatus::Displayed,
    );
    let card = compose_keyword_card(&input).card.expect("card");
    assert_eq!(card.citation_alignment, CONTRACT_UNVERIFIED);
}

#[test]
fn card_compose_leaves_click_state_to_b4() {
    let card = compose_keyword_card(&default_input()).card.expect("card");
    let debug = format!("{:?}", card);
    assert!(!debug.contains("Enabled"));
}

#[test]
fn card_compose_failed_stays_displayed_without_conclusion() {
    let input = input_for(
        "术语1",
        0.9,
        10,
        AlignmentVerdict::Failed,
        CitationAlignment::Failed,
        KeywordCardStatus::Displayed,
    );
    let card = compose_keyword_card(&input).card.expect("card");
    assert_eq!(card.card_status, KeywordCardStatus::Displayed);
    assert!(card.summary.is_none());
    assert_eq!(card.citation_alignment, CitationAlignment::Failed);
}

#[test]
fn card_compose_references_are_id_only_and_sorted() {
    let mut input = default_input();
    input.references = vec![
        reference("ref-b", CitationAlignment::Passed),
        reference("ref-a", CitationAlignment::Passed),
    ];
    let card = compose_keyword_card(&input).card.expect("card");
    assert_eq!(card.references[0].reference_id.as_str(), "ref-a");
    assert_eq!(card.references[1].reference_id.as_str(), "ref-b");
}

#[test]
fn card_compose_is_offline_and_deterministic() {
    let input = default_input();
    let first = compose_keyword_card(&input);
    for _ in 0..100 {
        assert_eq!(compose_keyword_card(&input), first);
    }
}

#[test]
fn card_compose_ignores_reference_order() {
    let base = default_input();
    let mut forward = base.clone();
    forward.references = vec![
        reference("ref-a", CitationAlignment::Passed),
        reference("ref-b", CitationAlignment::Passed),
        reference("ref-c", CitationAlignment::Passed),
    ];
    let mut reversed = forward.clone();
    reversed.references.reverse();
    let mut swapped = forward.clone();
    swapped.references.swap(0, 2);
    let first = compose_keyword_card(&forward);
    assert_eq!(compose_keyword_card(&reversed), first);
    assert_eq!(compose_keyword_card(&swapped), first);
}

#[test]
fn card_compose_zero_transcript_boundary() {
    let input = default_input();
    let debug = format!("{:?}", input);
    assert!(!debug.contains(input.candidate.normalized_term.as_str()));
    assert!(!debug.contains(input.candidate.keyword_text.as_str()));
    let card: ComposedCard = compose_keyword_card(&input).card.expect("card");
    assert_eq!(card.references.len(), 1);
}

#[test]
fn card_compose_ownership_is_outside_gate_and_ui() {
    assert_eq!(GateCounters::default().snapshot().len(), 32);
    assert!(!std::path::Path::new("crates/application/tests/replay").exists());
    assert!(!include_str!("../src/cards/compose.rs")
        .contains(concat!("ui-bridge/src/", "keyword_card")));
    assert_eq!(ComposeRejectReason::ALL.len(), 6);
    assert_eq!(ComposeRejectReason::ALL[0].index(), 0);
    assert_eq!(ComposeRejectReason::ALL[5].index(), 5);
}
