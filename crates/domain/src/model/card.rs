use std::fmt;

use serde::{Deserialize, Serialize};

use crate::model::enums::{
    AccessStatus, CardStatus, FeedbackStatus, GapKind, GapState, PresentationMode, TriggerKind,
    TrustTier, WithdrawReason,
};
use crate::model::session::TextRange;
use crate::vo::{
    CardRef, Confidence, ConclusionText, ProviderId, ReferenceRef, SanitizedQuery, TitleText,
    TraceId, UtteranceRef, WallClock,
};

/// 去重键：`SHA-256(topic_cluster ‖ entities ‖ domain)` 前 16 字节的十六进制。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DedupeKey(String);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("去重键必须为 32 个十六进制字符")]
pub struct DedupeKeyError;

impl DedupeKey {
    pub fn new(raw: impl Into<String>) -> Result<Self, DedupeKeyError> {
        let value = raw.into();
        if value.len() != 32 || !value.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(DedupeKeyError);
        }
        Ok(Self(value.to_ascii_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for DedupeKey {
    type Error = DedupeKeyError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<DedupeKey> for String {
    fn from(value: DedupeKey) -> Self {
        value.0
    }
}

/// 绝对 URL。禁止 `javascript:` / `data:` 等非 http(s) 协议。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AbsoluteUrl(String);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("仅接受 http(s) 绝对 URL，且长度不超过 2048")]
pub struct UrlError;

impl AbsoluteUrl {
    pub const LIMIT: usize = 2048;

    pub fn new(raw: impl Into<String>) -> Result<Self, UrlError> {
        let value = raw.into();
        let lower = value.to_ascii_lowercase();
        if value.len() > Self::LIMIT
            || !(lower.starts_with("https://") || lower.starts_with("http://"))
            || value.chars().any(char::is_whitespace)
        {
            return Err(UrlError);
        }
        Ok(Self(value))
    }

    pub fn domain(&self) -> Option<&str> {
        let rest = self.0.split_once("://")?.1;
        let host = rest.split(['/', '?', '#']).next()?;
        Some(host.split(':').next().unwrap_or(host))
    }
}

impl TryFrom<String> for AbsoluteUrl {
    type Error = UrlError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<AbsoluteUrl> for String {
    fn from(value: AbsoluteUrl) -> Self {
        value.0
    }
}

impl fmt::Display for AbsoluteUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 信息缺口候选。一个候选最多产出一张卡。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GapCandidate {
    pub kind: GapKind,
    pub trigger_span: TextRange,
    pub confidence: Confidence,
    pub dedupe_key: DedupeKey,
    pub state: GapState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reference {
    pub reference: ReferenceRef,
    pub title: TitleText,
    pub url: AbsoluteUrl,
    pub access: AccessStatus,
    pub fetched_at: WallClock,
    pub published_at: Option<WallClock>,
    pub trust: TrustTier,
    pub excerpt: Option<String>,
}

/// 冲突并列展示：不同来源结论不一致时不得择一隐瞒。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictView {
    pub alternatives: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Freshness {
    pub fetched_at: WallClock,
    pub cache_hit: bool,
}

/// 溯源链。每张卡必须携带，用于回答"这条提示从哪来"。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceChain {
    pub id: TraceId,
    pub trigger: UtteranceRef,
    pub attempted_providers: Vec<ProviderId>,
    pub query: SanitizedQuery,
    pub references: Vec<ReferenceRef>,
    pub shown_at: WallClock,
    pub feedback: Option<FeedbackStatus>,
}

/// 提示卡。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuggestionCard {
    pub identity: CardRef,
    pub trigger: TriggerKind,
    pub title: TitleText,
    pub conclusion: Option<ConclusionText>,
    pub confidence: Confidence,
    pub tier: TrustTier,
    pub status: CardStatus,
    pub feedback: FeedbackStatus,
    pub dedupe_key: DedupeKey,
    pub freshness: Freshness,
    pub conflict: Option<ConflictView>,
    pub shown_as: PresentationMode,
    pub references: Vec<Reference>,
    pub trace: TraceChain,
    pub withdraw_reason: Option<WithdrawReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CardError {
    #[error("无来源时结论必须为空（SPEC 6.4-5）")]
    ConclusionWithoutReference,
    #[error("缺少溯源链")]
    MissingTrace,
}

impl SuggestionCard {
    pub fn validate(&self) -> Result<(), CardError> {
        if self.references.is_empty() && self.conclusion.is_some() {
            return Err(CardError::ConclusionWithoutReference);
        }
        if self.trace.references.is_empty() && !self.references.is_empty() {
            return Err(CardError::MissingTrace);
        }
        Ok(())
    }

    pub fn withdraw(&mut self, reason: WithdrawReason) {
        self.status = CardStatus::Withdrawn;
        self.withdraw_reason = Some(reason);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupe_key_shape() {
        assert!(DedupeKey::new("a".repeat(32)).is_ok());
        assert!(DedupeKey::new("a".repeat(31)).is_err());
        assert!(DedupeKey::new("z".repeat(32)).is_err());
    }

    #[test]
    fn url_rejects_dangerous_schemes() {
        assert!(AbsoluteUrl::new("https://example.com/a").is_ok());
        assert!(AbsoluteUrl::new("javascript:alert(1)").is_err());
        assert!(AbsoluteUrl::new("data:text/html,hi").is_err());
        assert!(AbsoluteUrl::new("https://exa mple.com").is_err());
    }

    #[test]
    fn url_domain_extraction() {
        let url = AbsoluteUrl::new("https://docs.example.com:8443/path?q=1").expect("url");
        assert_eq!(url.domain(), Some("docs.example.com"));
    }
}