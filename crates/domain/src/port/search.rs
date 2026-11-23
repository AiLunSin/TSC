use std::net::IpAddr;

use crate::model::card::{AbsoluteUrl, Reference};
use crate::model::enums::{ProviderHealth, ProviderKind, TrustTier};
use crate::model::provider::{DomainPattern, ProviderTimeout};
use crate::vo::{ProviderId, SanitizedQuery, TitleText, WallClock};

/// 单次检索结果条数上限 10（SPEC 5.3.1）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub title: TitleText,
    pub url: AbsoluteUrl,
    pub domain: String,
    pub snippet: Option<String>,
    pub published_at: Option<WallClock>,
    pub trust: TrustTier,
}

/// 检索请求。`query` 必须是 [`SanitizedQuery`]，从类型上阻断未脱敏外发。
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderRequest {
    pub query: SanitizedQuery,
    pub limit: u8,
    pub timeout: ProviderTimeout,
    pub blocked_domains: Vec<DomainPattern>,
    /// 调用方解析出的地址，供外发白名单拒绝私网/环回/链路本地。
    pub resolved_ips: Vec<IpAddr>,
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
            resolved_ips: Vec::new(),
        })
    }

    pub fn with_resolved_ips(mut self, resolved_ips: Vec<IpAddr>) -> Self {
        self.resolved_ips = resolved_ips;
        self
    }

    pub fn is_domain_blocked(&self, domain: &str) -> bool {
        self.blocked_domains
            .iter()
            .any(|pattern| pattern.matches(domain))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProviderResponse {
    pub provider: ProviderId,
    pub hits: Vec<SearchHit>,
    pub elapsed_ms: u64,
}

/// 适配器失败分类固定为六类（SPEC 8.3-4），任何失败都不得 panic。
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ProviderFailure {
    #[error("检索超时")]
    Timeout,
    #[error("检索被限流")]
    RateLimited,
    #[error("检索鉴权失败")]
    AuthFailed,
    #[error("检索响应非法")]
    MalformedResponse,
    #[error("检索 TLS 失败")]
    TlsFailure,
    #[error("检索不可达")]
    Unreachable,
}

impl ProviderFailure {
    /// 供审计与日志写入的稳定标签，不包含查询、URL 或响应正文。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Timeout => "timeout",
            Self::RateLimited => "rate_limited",
            Self::AuthFailed => "auth_failed",
            Self::MalformedResponse => "malformed_response",
            Self::TlsFailure => "tls_failure",
            Self::Unreachable => "unreachable",
        }
    }
}

/// 外发白名单端口。基础设施在执行连接测试或检索前调用，避免反向依赖应用层。
pub trait OutboundPolicy: Send + Sync {
    fn check(
        &self,
        provider_profile_id: &str,
        url: &str,
        resolved_ips: &[IpAddr],
    ) -> Result<(), ProviderFailure>;
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

    fn health(&self) -> ProviderHealth;

    fn search(&self, request: &ProviderRequest) -> Result<ProviderResponse, ProviderFailure>;
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
