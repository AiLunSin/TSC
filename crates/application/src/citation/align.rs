//! 39 引用对齐：只读、离线、确定性；不生成卡片、不写记录。

use meeting_copilot_domain::model::citation::{
    is_accessible, is_authority_tier, is_general_tier, is_official_tier, AlignmentBasis,
    AlignmentVerdict, CitationAlignmentResult, CitationCheckInput, JudgeCapability,
    CITATION_CONTRACT_VERSION,
};
use meeting_copilot_domain::model::gate_keyword::{CitationAlignment, KeywordKind, ReferenceView};

#[derive(Default)]
pub struct CitationDiagnostics {
    pub checks: u32,
    pub passed: u32,
    pub failed: u32,
    pub unknown: u32,
    pub bases: [u32; 11],
}

impl CitationDiagnostics {
    pub fn record(&mut self, result: &CitationAlignmentResult) {
        self.checks = self.checks.saturating_add(1);
        match result.alignment {
            AlignmentVerdict::Passed => self.passed = self.passed.saturating_add(1),
            AlignmentVerdict::Failed => self.failed = self.failed.saturating_add(1),
            AlignmentVerdict::Unknown => self.unknown = self.unknown.saturating_add(1),
        }
        let index = result.basis.index();
        self.bases[index] = self.bases[index].saturating_add(1);
    }
}

fn sorted_references(references: &[ReferenceView]) -> Vec<ReferenceView> {
    let mut sorted = references.to_vec();
    sorted.sort_by(|left, right| left.reference_id.as_str().cmp(right.reference_id.as_str()));
    sorted
}

fn term_mismatch(input: &CitationCheckInput) -> bool {
    let term = input.normalized_term.as_str();
    let has_digit = term.chars().any(|c| c.is_ascii_digit())
        || term.chars().any(|c| {
            matches!(
                c,
                '一' | '二'
                    | '三'
                    | '四'
                    | '五'
                    | '六'
                    | '七'
                    | '八'
                    | '九'
                    | '十'
                    | '百'
                    | '千'
                    | '万'
                    | '亿'
            )
        });
    let has_institution = [
        "公司", "大学", "学院", "院", "局", "委", "部", "中心", "主任", "经理", "教授",
    ]
    .iter()
    .any(|suffix| term.contains(suffix));
    match input.keyword_kind {
        KeywordKind::Metric => !has_digit,
        KeywordKind::PolicyStandard => {
            let lower = term.to_ascii_lowercase();
            let marker = ["gb", "iso", "iec", "t/", "q/"]
                .iter()
                .any(|mark| lower.contains(mark));
            !(marker && has_digit || term.contains('第') && term.contains('号'))
        }
        KeywordKind::Acronym => {
            let count = term.chars().count();
            !((2..=10).contains(&count) && term.chars().all(|c| c.is_ascii_alphabetic()))
        }
        KeywordKind::ProperNoun => {
            term.chars().count() < 2 || term.chars().all(|c| c.is_ascii_digit())
        }
        KeywordKind::InstitutionRole => !has_institution,
    }
}

fn resolve(
    input: &CitationCheckInput,
    references: &[ReferenceView],
) -> (AlignmentVerdict, AlignmentBasis) {
    if !input.validate_shape() {
        return (
            AlignmentVerdict::Unknown,
            AlignmentBasis::InputContractViolation,
        );
    }
    if references.is_empty() {
        return (AlignmentVerdict::Unknown, AlignmentBasis::NoReference);
    }
    let accessible: Vec<&ReferenceView> = references
        .iter()
        .filter(|view| is_accessible(view.access_status))
        .collect();
    if accessible.is_empty() {
        return (
            AlignmentVerdict::Unknown,
            AlignmentBasis::ReferenceUnavailable,
        );
    }
    if input.judge_capability == JudgeCapability::Unavailable {
        return (AlignmentVerdict::Unknown, AlignmentBasis::JudgeUnavailable);
    }
    if term_mismatch(input) {
        return (AlignmentVerdict::Failed, AlignmentBasis::TermMismatch);
    }

    let failed = accessible
        .iter()
        .filter(|view| view.alignment == CitationAlignment::Failed)
        .count();
    let passed = accessible
        .iter()
        .filter(|view| view.alignment == CitationAlignment::Passed)
        .count();
    if failed == 1 && passed == 0 {
        return (AlignmentVerdict::Failed, AlignmentBasis::Contradicted);
    }
    if failed >= 2 || (failed >= 1 && passed >= 1) {
        return (AlignmentVerdict::Failed, AlignmentBasis::ConflictingSources);
    }
    if accessible
        .iter()
        .all(|view| is_general_tier(view.trust_tier))
    {
        return (
            AlignmentVerdict::Unknown,
            AlignmentBasis::SupportedByGeneral,
        );
    }
    if accessible
        .iter()
        .all(|view| view.alignment == CitationAlignment::Unknown)
    {
        return (
            AlignmentVerdict::Unknown,
            AlignmentBasis::InsufficientEvidence,
        );
    }
    if accessible.iter().any(|view| {
        is_official_tier(view.trust_tier) && view.alignment == CitationAlignment::Passed
    }) {
        return (
            AlignmentVerdict::Passed,
            AlignmentBasis::SupportedByOfficial,
        );
    }
    if accessible.iter().any(|view| {
        is_authority_tier(view.trust_tier) && view.alignment == CitationAlignment::Passed
    }) {
        return (
            AlignmentVerdict::Passed,
            AlignmentBasis::SupportedByAuthority,
        );
    }
    (
        AlignmentVerdict::Unknown,
        AlignmentBasis::InsufficientEvidence,
    )
}

pub fn check_citation_alignment(input: &CitationCheckInput) -> CitationAlignmentResult {
    let references = sorted_references(&input.references);
    let (alignment, basis) = resolve(input, &references);
    let accessible_reference_count = u32::try_from(
        references
            .iter()
            .filter(|view| is_accessible(view.access_status))
            .count(),
    )
    .unwrap_or(u32::MAX);
    CitationAlignmentResult {
        alignment,
        basis,
        reference_count: u32::try_from(references.len()).unwrap_or(u32::MAX),
        accessible_reference_count,
        checked_at_ms: input.now_ms,
        contract_version: CITATION_CONTRACT_VERSION,
    }
}

/// 38 只能调用本函数回填 `citation_alignment` 一项。
pub fn project_alignment(result: &CitationAlignmentResult) -> CitationAlignment {
    result.alignment.to_contract()
}
