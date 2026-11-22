//! 38 卡片合成：只读 7 项、确定性 11 项、不判定点击。

use meeting_copilot_domain::model::citation::CitationAlignmentResult;
use meeting_copilot_domain::model::enums::TriggerKind;
use meeting_copilot_domain::model::gate_keyword::{
    AllowTraceRef, CitationAlignment, KeywordCandidate, KeywordCardStatus, ReferenceView,
};
use meeting_copilot_domain::vo::{CardRef, ConclusionText, TitleText};

use crate::citation::align::project_alignment;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposeRejectReason {
    GateNotAllowed,
    CandidateInvalid,
    TitleTooLong,
    DuplicateCard,
    CardWithdrawn,
    InputContractViolation,
}

impl ComposeRejectReason {
    pub const ALL: [Self; 6] = [
        Self::GateNotAllowed,
        Self::CandidateInvalid,
        Self::TitleTooLong,
        Self::DuplicateCard,
        Self::CardWithdrawn,
        Self::InputContractViolation,
    ];

    pub const fn index(self) -> usize {
        match self {
            Self::GateNotAllowed => 0,
            Self::CandidateInvalid => 1,
            Self::TitleTooLong => 2,
            Self::DuplicateCard => 3,
            Self::CardWithdrawn => 4,
            Self::InputContractViolation => 5,
        }
    }
}

#[derive(Clone)]
pub struct CardComposeInput {
    pub card_id: CardRef,
    pub candidate: KeywordCandidate,
    pub allow_ref: Option<AllowTraceRef>,
    pub alignment_result: CitationAlignmentResult,
    pub references: Vec<ReferenceView>,
    pub current_card_status: KeywordCardStatus,
    pub suppressed_notice: bool,
}

impl core::fmt::Debug for CardComposeInput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CardComposeInput")
            .field("card_id", &self.card_id)
            .field("keyword_ref", &self.candidate.keyword_ref)
            .field("keyword_kind", &self.candidate.keyword_kind)
            .field("reference_count", &self.references.len())
            .field("has_allow_ref", &self.allow_ref.is_some())
            .field("current_card_status", &self.current_card_status)
            .field("suppressed_notice", &self.suppressed_notice)
            .field("alignment", &self.alignment_result.alignment)
            .field("basis", &self.alignment_result.basis)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ComposedCard {
    pub card_id: CardRef,
    pub keyword_text: TitleText,
    pub normalized_term: meeting_copilot_domain::vo::NormalizedTerm,
    pub keyword_kind: meeting_copilot_domain::model::gate_keyword::KeywordKind,
    pub trigger: TriggerKind,
    pub triggered_at_ms: u64,
    pub confidence: meeting_copilot_domain::vo::Confidence,
    pub summary: Option<ConclusionText>,
    pub references: Vec<ReferenceView>,
    pub citation_alignment: CitationAlignment,
    pub card_status: KeywordCardStatus,
}

impl ComposedCard {
    pub fn summary_is_consistent(&self) -> bool {
        match &self.summary {
            None => true,
            Some(_) => self.citation_alignment == CitationAlignment::Passed,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CardComposeOutcome {
    pub card: Option<ComposedCard>,
    pub reason: Option<ComposeRejectReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardStatusSignal {
    Present,
    Withdraw,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CardComposeDiagnostics {
    pub composes: u32,
    pub succeeded: u32,
    pub rejected: u32,
    pub reasons: [u32; 6],
}

impl CardComposeDiagnostics {
    pub fn record(&mut self, outcome: &CardComposeOutcome) {
        self.composes = self.composes.saturating_add(1);
        if outcome.card.is_some() {
            self.succeeded = self.succeeded.saturating_add(1);
        } else {
            self.rejected = self.rejected.saturating_add(1);
            if let Some(reason) = outcome.reason {
                let index = reason.index();
                self.reasons[index] = self.reasons[index].saturating_add(1);
            }
        }
    }
}

pub fn next_card_status(current: KeywordCardStatus, signal: CardStatusSignal) -> KeywordCardStatus {
    match (current, signal) {
        (KeywordCardStatus::Withdrawn, _) => KeywordCardStatus::Withdrawn,
        (_, CardStatusSignal::Withdraw) => KeywordCardStatus::Withdrawn,
        (KeywordCardStatus::Displayed, CardStatusSignal::Present)
        | (KeywordCardStatus::Collapsed, CardStatusSignal::Present) => KeywordCardStatus::Displayed,
    }
}

fn validate_references(references: &[ReferenceView]) -> bool {
    let mut ids = std::collections::BTreeSet::new();
    references
        .iter()
        .all(|view| ids.insert(view.reference_id.as_str().to_owned()))
        && ids.len() == references.len()
}

fn sorted_references(references: &[ReferenceView]) -> Vec<ReferenceView> {
    let mut sorted = references.to_vec();
    sorted.sort_by(|left, right| left.reference_id.as_str().cmp(right.reference_id.as_str()));
    sorted
}

fn compose_summary(
    alignment: CitationAlignment,
    term: &meeting_copilot_domain::vo::NormalizedTerm,
    references: &[ReferenceView],
) -> Option<ConclusionText> {
    if alignment != CitationAlignment::Passed || references.is_empty() {
        return None;
    }
    let text = format!("{SUMMARY_TEMPLATE_PREFIX}{}", term.as_str());
    if text.chars().count() > ConclusionText::LIMIT {
        return None;
    }
    ConclusionText::new(text).ok()
}

pub const SUMMARY_TEMPLATE_PREFIX: &str = "该术语可参考可靠来源：";

fn rejected(reason: ComposeRejectReason) -> CardComposeOutcome {
    CardComposeOutcome {
        card: None,
        reason: Some(reason),
    }
}

pub fn compose_keyword_card(input: &CardComposeInput) -> CardComposeOutcome {
    if !validate_references(&input.references) {
        return rejected(ComposeRejectReason::InputContractViolation);
    }
    if input.allow_ref.is_none() {
        return rejected(ComposeRejectReason::GateNotAllowed);
    }
    if !input.candidate.validate() {
        return rejected(ComposeRejectReason::CandidateInvalid);
    }
    if input.current_card_status == KeywordCardStatus::Withdrawn {
        return rejected(ComposeRejectReason::CardWithdrawn);
    }
    if input.suppressed_notice {
        return rejected(ComposeRejectReason::DuplicateCard);
    }
    if input.candidate.keyword_text.as_str().chars().count() > TitleText::LIMIT {
        return rejected(ComposeRejectReason::TitleTooLong);
    }

    let citation_alignment = project_alignment(&input.alignment_result);
    let references = sorted_references(&input.references);
    let summary = compose_summary(
        citation_alignment,
        &input.candidate.normalized_term,
        &references,
    );
    let card_status = next_card_status(input.current_card_status, CardStatusSignal::Present);

    CardComposeOutcome {
        card: Some(ComposedCard {
            card_id: input.card_id.clone(),
            keyword_text: input.candidate.keyword_text.clone(),
            normalized_term: input.candidate.normalized_term.clone(),
            keyword_kind: input.candidate.keyword_kind,
            trigger: TriggerKind::Auto,
            triggered_at_ms: input.candidate.trigger_span.end_ms,
            confidence: input.candidate.confidence,
            summary,
            references,
            citation_alignment,
            card_status,
        }),
        reason: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use meeting_copilot_domain::model::card::DedupeKey;
    use meeting_copilot_domain::model::citation::{AlignmentBasis, AlignmentVerdict};
    use meeting_copilot_domain::model::enums::GapState;
    use meeting_copilot_domain::model::session::TextRange;
    use meeting_copilot_domain::vo::{
        Confidence, KeywordRef, NormalizedTerm, ReferenceRef, SessionId, TraceId,
    };

    fn candidate() -> KeywordCandidate {
        KeywordCandidate {
            keyword_ref: KeywordRef::new("kw-1").expect("kw"),
            keyword_text: TitleText::new("术语").expect("text"),
            normalized_term: NormalizedTerm::new("术语").expect("term"),
            keyword_kind: meeting_copilot_domain::model::gate_keyword::KeywordKind::Metric,
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

    fn reference(id: &str) -> ReferenceView {
        ReferenceView {
            reference_id: ReferenceRef::new(id).expect("id"),
            trust_tier: meeting_copilot_domain::model::gate_keyword::ReferenceTrustTier::Official,
            access_status:
                meeting_copilot_domain::model::gate_keyword::ReferenceAccessStatus::Available,
            published_at_ms: None,
            alignment: CitationAlignment::Passed,
        }
    }

    fn input(status: KeywordCardStatus, suppressed: bool) -> CardComposeInput {
        CardComposeInput {
            card_id: CardRef::new("card-1").expect("card"),
            candidate: candidate(),
            allow_ref: Some(AllowTraceRef::with_trace(
                TraceId::new("t-1").expect("trace"),
                None,
            )),
            alignment_result: CitationAlignmentResult {
                alignment: AlignmentVerdict::Passed,
                basis: AlignmentBasis::SupportedByOfficial,
                reference_count: 1,
                accessible_reference_count: 1,
                checked_at_ms: 1,
                contract_version: 1,
            },
            references: vec![reference("ref-1")],
            current_card_status: status,
            suppressed_notice: suppressed,
        }
    }

    #[test]
    fn next_card_status_truth_table() {
        use KeywordCardStatus::*;
        assert_eq!(
            next_card_status(Displayed, CardStatusSignal::Present),
            Displayed
        );
        assert_eq!(
            next_card_status(Displayed, CardStatusSignal::Withdraw),
            Withdrawn
        );
        assert_eq!(
            next_card_status(Collapsed, CardStatusSignal::Present),
            Displayed
        );
        assert_eq!(
            next_card_status(Collapsed, CardStatusSignal::Withdraw),
            Withdrawn
        );
        assert_eq!(
            next_card_status(Withdrawn, CardStatusSignal::Present),
            Withdrawn
        );
        assert_eq!(
            next_card_status(Withdrawn, CardStatusSignal::Withdraw),
            Withdrawn
        );
    }

    #[test]
    fn compose_s0_rejects_duplicate_reference_id() {
        let mut value = input(KeywordCardStatus::Displayed, false);
        value.references.push(reference("ref-1"));
        assert_eq!(
            compose_keyword_card(&value).reason,
            Some(ComposeRejectReason::InputContractViolation)
        );
    }

    #[test]
    fn compose_s1_requires_allow_trace() {
        let mut value = input(KeywordCardStatus::Displayed, false);
        value.allow_ref = None;
        assert_eq!(
            compose_keyword_card(&value).reason,
            Some(ComposeRejectReason::GateNotAllowed)
        );
        assert!(
            compose_keyword_card(&input(KeywordCardStatus::Displayed, false))
                .card
                .is_some()
        );
    }

    #[test]
    fn compose_s2_rejects_invalid_candidate() {
        let mut value = input(KeywordCardStatus::Displayed, false);
        value.candidate.confidence = Confidence::new(0.49).expect("conf");
        assert_eq!(
            compose_keyword_card(&value).reason,
            Some(ComposeRejectReason::CandidateInvalid)
        );
    }

    #[test]
    fn compose_s3_withdrawn_precedes_duplicate() {
        assert_eq!(
            compose_keyword_card(&input(KeywordCardStatus::Withdrawn, true)).reason,
            Some(ComposeRejectReason::CardWithdrawn)
        );
        assert_eq!(
            compose_keyword_card(&input(KeywordCardStatus::Displayed, true)).reason,
            Some(ComposeRejectReason::DuplicateCard)
        );
    }

    #[test]
    fn title_limit_is_defensive() {
        assert_eq!(TitleText::LIMIT, 40);
        assert!(TitleText::new("字".repeat(40)).is_ok());
        assert!(TitleText::new("字".repeat(41)).is_err());
    }

    #[test]
    fn references_are_sorted_by_id() {
        let mut value = input(KeywordCardStatus::Displayed, false);
        value.references = vec![reference("ref-b"), reference("ref-a")];
        let card = compose_keyword_card(&value).card.expect("card");
        assert_eq!(card.references[0].reference_id.as_str(), "ref-a");
    }

    #[test]
    fn summary_requires_passed_and_refs() {
        let card = compose_keyword_card(&input(KeywordCardStatus::Displayed, false))
            .card
            .expect("card");
        assert!(card.summary.is_some());
        assert!(card.summary_is_consistent());
        let mut value = input(KeywordCardStatus::Displayed, false);
        value.references.clear();
        assert!(compose_keyword_card(&value)
            .card
            .expect("card")
            .summary
            .is_none());
    }

    #[test]
    fn assemble_card_passes_through_six_fields() {
        let card = compose_keyword_card(&input(KeywordCardStatus::Displayed, false))
            .card
            .expect("card");
        assert_eq!(card.triggered_at_ms, 10);
        assert_eq!(card.confidence.value(), 0.9);
        assert_eq!(card.trigger, TriggerKind::Auto);
        assert_eq!(card.keyword_kind, candidate().keyword_kind);
    }

    #[test]
    fn diagnostics_counts_match_inputs() {
        let mut diagnostics = CardComposeDiagnostics::default();
        diagnostics.record(&compose_keyword_card(&input(
            KeywordCardStatus::Displayed,
            false,
        )));
        let mut missing = input(KeywordCardStatus::Displayed, false);
        missing.allow_ref = None;
        diagnostics.record(&compose_keyword_card(&missing));
        assert_eq!(diagnostics.composes, 2);
        assert_eq!(diagnostics.succeeded, 1);
        assert_eq!(diagnostics.rejected, 1);
        assert_eq!(diagnostics.reasons.iter().sum::<u32>(), 1);
        assert_eq!(ComposeRejectReason::ALL.len(), 6);
    }
}
