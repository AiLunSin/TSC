//! 39 引用对齐契约：只读、离线、确定性。

use serde::{Deserialize, Serialize};

use crate::model::gate_keyword::{
    CitationAlignment, KeywordKind, ReferenceAccessStatus, ReferenceTrustTier, ReferenceView,
};
use crate::vo::{CardRef, KeywordRef, NormalizedTerm};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentVerdict {
    Passed,
    Failed,
    Unknown,
}

impl AlignmentVerdict {
    pub const ALL: [Self; 3] = [Self::Passed, Self::Failed, Self::Unknown];

    pub const fn to_contract(self) -> CitationAlignment {
        match self {
            Self::Passed => CitationAlignment::Passed,
            Self::Failed => CitationAlignment::Failed,
            Self::Unknown => CONTRACT_UNVERIFIED,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentBasis {
    SupportedByOfficial,
    SupportedByAuthority,
    SupportedByGeneral,
    Contradicted,
    TermMismatch,
    ConflictingSources,
    NoReference,
    ReferenceUnavailable,
    InsufficientEvidence,
    JudgeUnavailable,
    InputContractViolation,
}

impl AlignmentBasis {
    pub const ALL: [Self; 11] = [
        Self::SupportedByOfficial,
        Self::SupportedByAuthority,
        Self::SupportedByGeneral,
        Self::Contradicted,
        Self::TermMismatch,
        Self::ConflictingSources,
        Self::NoReference,
        Self::ReferenceUnavailable,
        Self::InsufficientEvidence,
        Self::JudgeUnavailable,
        Self::InputContractViolation,
    ];

    pub const fn verdict(self) -> AlignmentVerdict {
        match self {
            Self::SupportedByOfficial | Self::SupportedByAuthority => AlignmentVerdict::Passed,
            Self::Contradicted | Self::TermMismatch | Self::ConflictingSources => {
                AlignmentVerdict::Failed
            }
            _ => AlignmentVerdict::Unknown,
        }
    }

    pub const fn index(self) -> usize {
        match self {
            Self::SupportedByOfficial => 0,
            Self::SupportedByAuthority => 1,
            Self::SupportedByGeneral => 2,
            Self::Contradicted => 3,
            Self::TermMismatch => 4,
            Self::ConflictingSources => 5,
            Self::NoReference => 6,
            Self::ReferenceUnavailable => 7,
            Self::InsufficientEvidence => 8,
            Self::JudgeUnavailable => 9,
            Self::InputContractViolation => 10,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JudgeCapability {
    Available,
    Unavailable,
}

/// `Unknown` 的唯一契约投影（语义名 Unverified，代码标识 Unknown）。
pub const CONTRACT_UNVERIFIED: CitationAlignment = CitationAlignment::Unknown;

impl CitationAlignment {
    pub const ALL: [Self; 3] = [Self::Passed, Self::Failed, Self::Unknown];
}

pub const fn is_official_tier(tier: ReferenceTrustTier) -> bool {
    matches!(tier, ReferenceTrustTier::Official)
}

pub const fn is_authority_tier(tier: ReferenceTrustTier) -> bool {
    matches!(
        tier,
        ReferenceTrustTier::Official | ReferenceTrustTier::AuthoritativeMedia
    )
}

pub const fn is_general_tier(tier: ReferenceTrustTier) -> bool {
    matches!(
        tier,
        ReferenceTrustTier::GeneralWeb | ReferenceTrustTier::UserGenerated
    )
}

pub const fn is_accessible(status: ReferenceAccessStatus) -> bool {
    matches!(status, ReferenceAccessStatus::Available)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimRefError {
    Empty,
    Whitespace,
    AtSign,
    SchemeSeparator,
    TooLong,
}

impl core::fmt::Display for ClaimRefError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::Empty => "claim 引用为空",
            Self::Whitespace => "claim 引用含空白",
            Self::AtSign => "claim 引用含 @",
            Self::SchemeSeparator => "claim 引用含 scheme 分隔符",
            Self::TooLong => "claim 引用超出长度上限",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClaimRef(String);

impl ClaimRef {
    pub fn new(raw: impl Into<String>) -> Result<Self, ClaimRefError> {
        let value = raw.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(ClaimRefError::Empty);
        }
        if trimmed.chars().count() > 128 {
            return Err(ClaimRefError::TooLong);
        }
        if trimmed.chars().any(char::is_whitespace) {
            return Err(ClaimRefError::Whitespace);
        }
        if trimmed.contains('@') {
            return Err(ClaimRefError::AtSign);
        }
        if trimmed.contains("://") {
            return Err(ClaimRefError::SchemeSeparator);
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub struct CitationCheckInput {
    pub card_id: CardRef,
    pub keyword_ref: KeywordRef,
    pub normalized_term: NormalizedTerm,
    pub keyword_kind: KeywordKind,
    pub references: Vec<ReferenceView>,
    pub now_ms: u64,
    pub judge_capability: JudgeCapability,
    pub claim_ref: Option<ClaimRef>,
}

impl core::fmt::Debug for CitationCheckInput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CitationCheckInput")
            .field("card_id", &self.card_id)
            .field("keyword_ref", &self.keyword_ref)
            .field("keyword_kind", &self.keyword_kind)
            .field("reference_count", &self.references.len())
            .field("now_ms", &self.now_ms)
            .field("judge_capability", &self.judge_capability)
            .field("has_claim_ref", &self.claim_ref.is_some())
            .finish()
    }
}

impl CitationCheckInput {
    /// 只判引用 ID 不重复；不排序、不去重、不读术语。
    pub fn validate_shape(&self) -> bool {
        let mut ids = std::collections::BTreeSet::new();
        self.references
            .iter()
            .all(|view| ids.insert(view.reference_id.as_str().to_owned()))
            && ids.len() == self.references.len()
    }
}

pub const CITATION_CONTRACT_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationAlignmentResult {
    pub alignment: AlignmentVerdict,
    pub basis: AlignmentBasis,
    pub reference_count: u32,
    pub accessible_reference_count: u32,
    pub checked_at_ms: u64,
    pub contract_version: u16,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::gate_keyword::{
        CitationAlignment, ReferenceAccessStatus, ReferenceTrustTier,
    };

    fn view(id: &str, tier: ReferenceTrustTier, status: ReferenceAccessStatus) -> ReferenceView {
        ReferenceView {
            reference_id: crate::vo::ReferenceRef::new(id).expect("id"),
            trust_tier: tier,
            access_status: status,
            published_at_ms: None,
            alignment: CitationAlignment::Passed,
        }
    }

    fn input(references: Vec<ReferenceView>) -> CitationCheckInput {
        CitationCheckInput {
            card_id: CardRef::new("card-1").expect("card"),
            keyword_ref: KeywordRef::new("kw-1").expect("kw"),
            normalized_term: NormalizedTerm::new("术语").expect("term"),
            keyword_kind: KeywordKind::Metric,
            references,
            now_ms: 7,
            judge_capability: JudgeCapability::Available,
            claim_ref: None,
        }
    }

    #[test]
    fn citation_input_has_eight_fields() {
        let value = input(Vec::new());
        assert_eq!(value.references.len(), 0);
        assert!(value.validate_shape());
    }

    #[test]
    fn citation_result_has_six_fields() {
        let result = CitationAlignmentResult {
            alignment: AlignmentVerdict::Unknown,
            basis: AlignmentBasis::InsufficientEvidence,
            reference_count: 0,
            accessible_reference_count: 0,
            checked_at_ms: 0,
            contract_version: CITATION_CONTRACT_VERSION,
        };
        let value = serde_json::to_value(result).expect("value");
        assert_eq!(value.as_object().expect("object").len(), 6);
    }

    #[test]
    fn citation_basis_has_eleven_variants() {
        assert_eq!(AlignmentBasis::ALL.len(), 11);
        for (index, basis) in AlignmentBasis::ALL.into_iter().enumerate() {
            assert_eq!(basis.index(), index);
            assert_eq!(basis.verdict(), basis.verdict());
        }
        assert_eq!(
            AlignmentBasis::SupportedByOfficial.verdict(),
            AlignmentVerdict::Passed
        );
        assert_eq!(
            AlignmentBasis::Contradicted.verdict(),
            AlignmentVerdict::Failed
        );
        assert_eq!(
            AlignmentBasis::NoReference.verdict(),
            AlignmentVerdict::Unknown
        );
    }

    #[test]
    fn claim_ref_rejects_illegal_shapes() {
        assert_eq!(ClaimRef::new(""), Err(ClaimRefError::Empty));
        assert_eq!(ClaimRef::new("has space"), Err(ClaimRefError::Whitespace));
        assert_eq!(ClaimRef::new("user@host"), Err(ClaimRefError::AtSign));
        assert_eq!(ClaimRef::new("a://b"), Err(ClaimRefError::SchemeSeparator));
        assert!(ClaimRef::new("claim-0001").is_ok());
    }

    #[test]
    fn alignment_projection_is_injective() {
        assert_eq!(AlignmentVerdict::Unknown.to_contract(), CONTRACT_UNVERIFIED);
        assert_eq!(CitationAlignment::ALL.len(), 3);
        assert_eq!(AlignmentVerdict::ALL.len(), 3);
        assert_ne!(
            AlignmentVerdict::Passed.to_contract(),
            AlignmentVerdict::Failed.to_contract()
        );
    }

    #[test]
    fn citation_tier_predicates_match_alias_table() {
        assert!(is_official_tier(ReferenceTrustTier::Official));
        assert!(is_authority_tier(ReferenceTrustTier::AuthoritativeMedia));
        assert!(is_general_tier(ReferenceTrustTier::UserGenerated));
        assert!(is_accessible(ReferenceAccessStatus::Available));
        assert!(!is_accessible(ReferenceAccessStatus::Unavailable));
        let _ = view(
            "ref-1",
            ReferenceTrustTier::Official,
            ReferenceAccessStatus::Available,
        );
    }
}
