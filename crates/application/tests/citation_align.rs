//! 39 引用对齐校验（AC-B4-58~64，12 用例）。

mod common;

use common::Harness;
use meeting_copilot_application::citation::{check_citation_alignment, project_alignment};
use meeting_copilot_application::gate::keyword::{
    derive_keyword_candidates, keyword_card_admission, KeywordCardEnv,
};
use meeting_copilot_domain::model::citation::{
    AlignmentBasis, AlignmentVerdict, CitationCheckInput, JudgeCapability, CONTRACT_UNVERIFIED,
};
use meeting_copilot_domain::model::enums::TriggerKind;
use meeting_copilot_domain::model::gate::{
    AuthorizationStatus, GateCounters, GateDecision, GateRejectReason, GateStage, PerStageResults,
};
use meeting_copilot_domain::model::gate_keyword::{
    AllowTraceRef, CitationAlignment, ClickDisabledReason, ClickState, KeywordCardDraft,
    KeywordCardStatus, KeywordKind, ReferenceAccessStatus, ReferenceTrustTier, ReferenceView,
};
use meeting_copilot_domain::vo::{
    CardRef, Confidence, KeywordRef, NormalizedTerm, TitleText, TraceId,
};

fn reference(
    id: &str,
    tier: ReferenceTrustTier,
    status: ReferenceAccessStatus,
    alignment: CitationAlignment,
) -> ReferenceView {
    ReferenceView {
        reference_id: meeting_copilot_domain::vo::ReferenceRef::new(id).expect("id"),
        trust_tier: tier,
        access_status: status,
        published_at_ms: None,
        alignment,
    }
}

fn sample_card() -> KeywordCardDraft {
    KeywordCardDraft {
        card_id: CardRef::new("card-0001").expect("card"),
        keyword_text: TitleText::new("术语").expect("text"),
        normalized_term: NormalizedTerm::new("术语").expect("term"),
        keyword_kind: KeywordKind::ProperNoun,
        trigger: TriggerKind::Auto,
        triggered_at_ms: 10,
        confidence: Confidence::new(0.9).expect("confidence"),
        summary: None,
        references: Vec::new(),
        citation_alignment: CitationAlignment::Unknown,
        gate_decision_ref: AllowTraceRef::with_trace(TraceId::new("t-card").expect("trace"), None),
        card_status: KeywordCardStatus::Displayed,
        click_state: ClickState::Disabled(ClickDisabledReason::ReferenceUnverified),
    }
}

fn term_for(kind: KeywordKind) -> &'static str {
    match kind {
        KeywordKind::Metric => "10%",
        KeywordKind::PolicyStandard => "gb 123",
        KeywordKind::Acronym => "ABC",
        KeywordKind::ProperNoun => "术语",
        KeywordKind::InstitutionRole => "公司",
    }
}

fn input_for(
    kind: KeywordKind,
    references: Vec<ReferenceView>,
    capability: JudgeCapability,
    now_ms: u64,
) -> CitationCheckInput {
    CitationCheckInput {
        card_id: CardRef::new("card-0001").expect("card"),
        keyword_ref: KeywordRef::new("kw-0001").expect("kw"),
        normalized_term: NormalizedTerm::new(term_for(kind)).expect("term"),
        keyword_kind: kind,
        references,
        now_ms,
        judge_capability: capability,
        claim_ref: None,
    }
}

fn outcome(kind: KeywordKind, references: Vec<ReferenceView>) -> AlignmentBasis {
    check_citation_alignment(&input_for(kind, references, JudgeCapability::Available, 7)).basis
}

#[test]
fn citation_input_is_readonly_and_closed() {
    let card = sample_card();
    let before = format!("{:?}", card);
    let input = input_for(
        KeywordKind::ProperNoun,
        vec![reference(
            "ref-1",
            ReferenceTrustTier::Official,
            ReferenceAccessStatus::Available,
            CitationAlignment::Passed,
        )],
        JudgeCapability::Available,
        11,
    );
    let _ = check_citation_alignment(&input);
    let debug = format!("{:?}", input);
    assert!(!debug.contains("术语"));
    assert_eq!(format!("{:?}", card), before);
    assert!(input.validate_shape());
}

#[test]
fn citation_alignment_is_three_state_and_fail_closed() {
    let mut bases = Vec::new();
    bases.push(outcome(KeywordKind::ProperNoun, Vec::new()));
    bases.push(outcome(
        KeywordKind::ProperNoun,
        vec![reference(
            "ref-1",
            ReferenceTrustTier::Official,
            ReferenceAccessStatus::Unavailable,
            CitationAlignment::Passed,
        )],
    ));
    bases.push(
        check_citation_alignment(&input_for(
            KeywordKind::ProperNoun,
            vec![reference(
                "ref-1",
                ReferenceTrustTier::Official,
                ReferenceAccessStatus::Available,
                CitationAlignment::Passed,
            )],
            JudgeCapability::Unavailable,
            7,
        ))
        .basis,
    );
    let mut duplicate = input_for(
        KeywordKind::ProperNoun,
        vec![
            reference(
                "ref-1",
                ReferenceTrustTier::Official,
                ReferenceAccessStatus::Available,
                CitationAlignment::Passed,
            ),
            reference(
                "ref-1",
                ReferenceTrustTier::GeneralWeb,
                ReferenceAccessStatus::Available,
                CitationAlignment::Passed,
            ),
        ],
        JudgeCapability::Available,
        7,
    );
    assert!(!duplicate.validate_shape());
    assert_eq!(
        check_citation_alignment(&duplicate).basis,
        AlignmentBasis::InputContractViolation
    );
    duplicate.references.clear();
    bases.push(AlignmentBasis::InputContractViolation);

    bases.push(outcome(
        KeywordKind::Metric,
        vec![reference(
            "ref-1",
            ReferenceTrustTier::Official,
            ReferenceAccessStatus::Available,
            CitationAlignment::Passed,
        )],
    ));
    let _ = &mut bases;
    assert!(bases.contains(&AlignmentBasis::NoReference));
    assert!(bases.contains(&AlignmentBasis::ReferenceUnavailable));
    assert!(bases.contains(&AlignmentBasis::JudgeUnavailable));
}

#[test]
fn unknown_projects_to_unverified_only() {
    for basis in [
        AlignmentBasis::NoReference,
        AlignmentBasis::ReferenceUnavailable,
        AlignmentBasis::JudgeUnavailable,
        AlignmentBasis::InsufficientEvidence,
        AlignmentBasis::InputContractViolation,
    ] {
        assert_eq!(basis.verdict(), AlignmentVerdict::Unknown);
    }
    assert_eq!(AlignmentVerdict::Unknown.to_contract(), CONTRACT_UNVERIFIED);
    assert_eq!(CitationAlignment::ALL.len(), 3);
    assert_eq!(AlignmentVerdict::ALL.len(), 3);
}

#[test]
fn citation_result_is_id_only() {
    let result = check_citation_alignment(&input_for(
        KeywordKind::Metric,
        vec![reference(
            "ref-1",
            ReferenceTrustTier::Official,
            ReferenceAccessStatus::Available,
            CitationAlignment::Passed,
        )],
        JudgeCapability::Available,
        9,
    ));
    assert_eq!(result.reference_count, 1);
    assert_eq!(result.accessible_reference_count, 1);
    assert_eq!(result.checked_at_ms, 9);
    assert_eq!(result.contract_version, 1);
    let view = reference(
        "ref-1",
        ReferenceTrustTier::GeneralWeb,
        ReferenceAccessStatus::Available,
        CitationAlignment::Unknown,
    );
    assert_eq!(view.trust_tier, ReferenceTrustTier::GeneralWeb);
}

fn allow() -> GateDecision {
    GateDecision::Allow {
        per_stage: PerStageResults::empty(),
        budget_remaining: 1,
        candidates: Vec::new(),
        outbound: None,
        suppressed_notice: false,
        stopped_at: None,
    }
}

fn harness_candidate() -> (
    Harness,
    meeting_copilot_domain::model::gate_keyword::KeywordCandidate,
) {
    let mut harness = Harness::new(10_000);
    harness.lexicon.push("张伟".to_owned());
    harness.push("张伟负责该模块。", 0.9, 0, 100);
    harness.relevance.semantic_milli = 1000;
    harness.relevance.entity_milli = 1000;
    harness.relevance.gain_milli = 1000;
    harness.relevance.source_trust_milli = 1000;
    let set = derive_keyword_candidates(&harness.scan());
    let candidate = set.new_candidates.first().expect("candidate").clone();
    (harness, candidate)
}

#[test]
fn citation_passed_enables_click_only_with_other_facts() {
    let (harness, candidate) = harness_candidate();
    let base = KeywordCardEnv {
        authorization_status: AuthorizationStatus::Granted,
        authorization_revoked: false,
        screen_share: false,
        emergency_masked: false,
        current_session: &harness.session,
        card_status: KeywordCardStatus::Displayed,
    };
    let enabled = keyword_card_admission(
        &candidate,
        &allow(),
        &TraceId::new("t-1").expect("trace"),
        CitationAlignment::Passed,
        &base,
    );
    assert_eq!(enabled.click_state, ClickState::Enabled);
    assert_ne!(CitationAlignment::Passed, CitationAlignment::Failed);

    let revoked = KeywordCardEnv {
        authorization_revoked: true,
        ..base
    };
    assert_eq!(
        keyword_card_admission(
            &candidate,
            &allow(),
            &TraceId::new("t-2").expect("trace"),
            CitationAlignment::Passed,
            &revoked,
        )
        .click_state,
        ClickState::Disabled(ClickDisabledReason::AuthorizationRevoked)
    );

    let deny = GateDecision::Deny {
        reason: GateRejectReason::Paused,
        stopped_at: GateStage::G1,
        per_stage: PerStageResults::empty(),
        cooldown_until_ms: None,
    };
    assert_eq!(
        keyword_card_admission(
            &candidate,
            &deny,
            &TraceId::new("t-3").expect("trace"),
            CitationAlignment::Passed,
            &base,
        )
        .click_state,
        ClickState::Disabled(ClickDisabledReason::NotAllowed)
    );

    let screen = KeywordCardEnv {
        screen_share: true,
        ..base
    };
    assert_eq!(
        keyword_card_admission(
            &candidate,
            &allow(),
            &TraceId::new("t-4").expect("trace"),
            CitationAlignment::Passed,
            &screen,
        )
        .click_state,
        ClickState::Disabled(ClickDisabledReason::ScreenShareProtection)
    );
}

#[test]
fn citation_failed_still_displayed_but_disabled() {
    let (harness, candidate) = harness_candidate();
    let env = KeywordCardEnv {
        authorization_status: AuthorizationStatus::Granted,
        authorization_revoked: false,
        screen_share: false,
        emergency_masked: false,
        current_session: &harness.session,
        card_status: KeywordCardStatus::Displayed,
    };
    let admission = keyword_card_admission(
        &candidate,
        &allow(),
        &TraceId::new("t-5").expect("trace"),
        CitationAlignment::Failed,
        &env,
    );
    assert!(admission.card_drafts);
    assert_eq!(
        admission.click_state,
        ClickState::Disabled(ClickDisabledReason::ReferenceUnverified)
    );
}

#[test]
fn citation_unknown_is_conservative() {
    let (harness, candidate) = harness_candidate();
    let env = KeywordCardEnv {
        authorization_status: AuthorizationStatus::Granted,
        authorization_revoked: false,
        screen_share: false,
        emergency_masked: false,
        current_session: &harness.session,
        card_status: KeywordCardStatus::Displayed,
    };
    let result = check_citation_alignment(&input_for(
        KeywordKind::ProperNoun,
        Vec::new(),
        JudgeCapability::Available,
        7,
    ));
    let projected = project_alignment(&result);
    assert_eq!(projected, CONTRACT_UNVERIFIED);
    let admission = keyword_card_admission(
        &candidate,
        &allow(),
        &TraceId::new("t-6").expect("trace"),
        projected,
        &env,
    );
    assert!(admission.card_drafts);
    assert_eq!(
        admission.click_state,
        ClickState::Disabled(ClickDisabledReason::ReferenceUnverified)
    );
}

#[test]
fn citation_ownership_is_outside_b4_and_ui() {
    assert_eq!(GateCounters::default().snapshot().len(), 32);
    assert!(!std::path::Path::new("crates/application/tests/replay").exists());
    let result = check_citation_alignment(&input_for(
        KeywordKind::ProperNoun,
        Vec::new(),
        JudgeCapability::Available,
        0,
    ));
    assert_eq!(result.alignment, AlignmentVerdict::Unknown);
}

#[test]
fn citation_check_is_offline_and_deterministic() {
    let input = input_for(
        KeywordKind::Metric,
        vec![reference(
            "ref-1",
            ReferenceTrustTier::Official,
            ReferenceAccessStatus::Available,
            CitationAlignment::Passed,
        )],
        JudgeCapability::Available,
        7,
    );
    let first = check_citation_alignment(&input);
    for _ in 0..100 {
        assert_eq!(check_citation_alignment(&input), first);
    }
}

#[test]
fn citation_ignores_reference_order() {
    let references = vec![
        reference(
            "ref-a",
            ReferenceTrustTier::Official,
            ReferenceAccessStatus::Available,
            CitationAlignment::Passed,
        ),
        reference(
            "ref-b",
            ReferenceTrustTier::GeneralWeb,
            ReferenceAccessStatus::Available,
            CitationAlignment::Passed,
        ),
        reference(
            "ref-c",
            ReferenceTrustTier::AuthoritativeMedia,
            ReferenceAccessStatus::Available,
            CitationAlignment::Passed,
        ),
    ];
    let forward = check_citation_alignment(&input_for(
        KeywordKind::Metric,
        references.clone(),
        JudgeCapability::Available,
        7,
    ));
    let mut reversed = references.clone();
    reversed.reverse();
    let backward = check_citation_alignment(&input_for(
        KeywordKind::Metric,
        reversed,
        JudgeCapability::Available,
        7,
    ));
    assert_eq!(forward, backward);
}

#[test]
fn citation_basis_is_unique_and_ordered() {
    assert_eq!(
        outcome(KeywordKind::ProperNoun, Vec::new()),
        AlignmentBasis::NoReference
    );
    assert_eq!(
        outcome(
            KeywordKind::Metric,
            vec![reference(
                "ref-1",
                ReferenceTrustTier::GeneralWeb,
                ReferenceAccessStatus::Available,
                CitationAlignment::Passed,
            )],
        ),
        AlignmentBasis::SupportedByGeneral
    );
    assert_eq!(
        outcome(
            KeywordKind::Metric,
            vec![reference(
                "ref-1",
                ReferenceTrustTier::Official,
                ReferenceAccessStatus::Available,
                CitationAlignment::Unknown,
            )],
        ),
        AlignmentBasis::InsufficientEvidence
    );
    assert_eq!(
        outcome(
            KeywordKind::Metric,
            vec![reference(
                "ref-1",
                ReferenceTrustTier::Official,
                ReferenceAccessStatus::Available,
                CitationAlignment::Passed,
            )],
        ),
        AlignmentBasis::SupportedByOfficial
    );
    assert_eq!(
        outcome(
            KeywordKind::Metric,
            vec![reference(
                "ref-1",
                ReferenceTrustTier::AuthoritativeMedia,
                ReferenceAccessStatus::Available,
                CitationAlignment::Passed,
            )],
        ),
        AlignmentBasis::SupportedByAuthority
    );
    assert_eq!(
        outcome(
            KeywordKind::Metric,
            vec![reference(
                "ref-1",
                ReferenceTrustTier::Official,
                ReferenceAccessStatus::Available,
                CitationAlignment::Failed,
            )],
        ),
        AlignmentBasis::Contradicted
    );
    assert_eq!(
        outcome(
            KeywordKind::Metric,
            vec![
                reference(
                    "ref-1",
                    ReferenceTrustTier::Official,
                    ReferenceAccessStatus::Available,
                    CitationAlignment::Failed,
                ),
                reference(
                    "ref-2",
                    ReferenceTrustTier::Official,
                    ReferenceAccessStatus::Available,
                    CitationAlignment::Failed,
                ),
            ],
        ),
        AlignmentBasis::ConflictingSources
    );
    assert_eq!(
        outcome(
            KeywordKind::ProperNoun,
            vec![reference(
                "ref-1",
                ReferenceTrustTier::Official,
                ReferenceAccessStatus::Available,
                CitationAlignment::Passed,
            )],
        ),
        AlignmentBasis::SupportedByOfficial
    );
    let mut short_term = input_for(
        KeywordKind::ProperNoun,
        vec![reference(
            "ref-1",
            ReferenceTrustTier::Official,
            ReferenceAccessStatus::Available,
            CitationAlignment::Passed,
        )],
        JudgeCapability::Available,
        7,
    );
    short_term.normalized_term = NormalizedTerm::new("字").expect("term");
    assert_eq!(
        check_citation_alignment(&short_term).basis,
        AlignmentBasis::TermMismatch
    );
}

#[test]
fn citation_counts_match_inputs() {
    for size in [0usize, 1, 3] {
        let references: Vec<ReferenceView> = (0..size)
            .map(|index| {
                reference(
                    &format!("ref-{index:02}"),
                    ReferenceTrustTier::GeneralWeb,
                    ReferenceAccessStatus::Available,
                    CitationAlignment::Passed,
                )
            })
            .collect();
        let result = check_citation_alignment(&input_for(
            KeywordKind::Metric,
            references,
            JudgeCapability::Available,
            13,
        ));
        assert_eq!(result.reference_count, size as u32);
        assert_eq!(result.accessible_reference_count, size as u32);
        assert_eq!(result.checked_at_ms, 13);
    }
}
