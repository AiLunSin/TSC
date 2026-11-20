use crate::error::CopilotError;
use crate::model::card::{AbsoluteUrl, Reference};
use crate::model::enums::{ProviderKind, TrustTier};
use crate::model::provider::{DomainPattern, ProviderTimeout};
use crate::vo::{ProviderId, SanitizedQuery, TitleText, WallClock};

/// 单次检索结果条数上限 10（SPEC 5.3.1）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub title: TitleText,
    pub url: AbsoluteUrl,
    pub domain: String,
    pub snippet: Option<String>,
    pub trust: TrustTier,
}

/// 检索请求。`query` 必须是 [`SanitizedQuery`]，从类型上阻断未脱敏外发。
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderRequest {
    pub query: SanitizedQuery,
    pub limit: u8,
    pub timeout: ProviderTimeout,
    pub blocked_domains: Vec<DomainPattern>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RequestError {
    #[error("检索条数必须落在 1–10，实际为 {0}")]
    InvalidLimit(u8),
}

impl ProviderRequest {
    pub const MAX_LIMIT: u8 = 10;

    pub fn new(
        query: SanitizedQuery,
        limit: u8,
        timeout: ProviderTimeout,
        blocked_domains: Vec<DomainPattern>,
    ) -> Result<Self, RequestError> {
        if !(1..=Self::MAX_LIMIT).contains(&limit) {
            return Err(RequestError::InvalidLimit(limit));
        }
        Ok(Self {
            query,
            limit,
            timeout,
            blocked_domains,
        })
    }

    pub fn is_domain_blocked(&self, domain: &str) -> bool {
        self.blocked_domains.iter().any(|pattern| pattern.matches(domain))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProviderResponse {
    pub provider: ProviderId,
    pub hits: Vec<SearchHit>,
    pub elapsed_ms: u64,
}

/// 抓取结果。非 `Ok` 时不得产出"有结论但无来源"的卡片。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchOutcome {
    Ok {
        title: TitleText,
        text: String,
        published_at: Option<WallClock>,
    },
    Skipped {
        reason: String,
    },
    Failed {
        reason: String,
        retryable: bool,
    },
}

impl FetchOutcome {
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok { .. })
    }
}

pub trait SearchProvider: Send + Sync {
    fn profile_id(&self) -> &ProviderId;

    fn kind(&self) -> ProviderKind;

    fn test_connection(&self) -> Result<(), CopilotError>;

    fn search(&self, request: &ProviderRequest) -> Result<ProviderResponse, CopilotError>;
}

/// 检索调度：按优先级与权重选择可用源，失败时按备用顺序降级。
pub trait SearchBroker: Send + Sync {
    fn search(&self, request: &ProviderRequest) -> Vec<ProviderResponse>;
}

pub trait PageFetcher: Send + Sync {
    fn fetch(&self, url: &AbsoluteUrl, timeout: ProviderTimeout) -> FetchOutcome;

    /// 把命中结果转换为领域引用，供提示卡携带来源。
    fn to_reference(&self, hit: &SearchHit, fetched_at: WallClock) -> Reference;
}