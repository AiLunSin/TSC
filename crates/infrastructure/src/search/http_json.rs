//! 单源 HTTP JSON 适配器（SPEC 8.3-5/6、8.4）。
//!
//! 仅 GET、可选 Bearer、8 秒硬超时、响应 ≤2 MiB；不记录 URL 查询串与响应正文。
//! 传输复用 [`crate::llm::transport`]，不新增 HTTP 依赖。

use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use meeting_copilot_domain::model::card::AbsoluteUrl;
use meeting_copilot_domain::model::enums::ProviderHealth;
use meeting_copilot_domain::model::enums::{ProviderKind, TrustTier};
use meeting_copilot_domain::model::provider::ProviderProfile;
use meeting_copilot_domain::port::llm::CancelToken;
use meeting_copilot_domain::port::search::{
    ProviderFailure, ProviderRequest, ProviderResponse, SearchHit, SearchProvider,
};
use meeting_copilot_domain::vo::{ProviderId, TitleText, WallClock};
use serde::Deserialize;

use crate::llm::transport::{HttpRequest, HttpTransport};
use crate::search::health::classify_transport;
use crate::search::profile_store::{
    parse_https_endpoint, HttpsEndpoint, ProfileError, ProviderEntry,
};

/// SPEC 8.3-6：响应体上限 2 MiB。
pub const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

pub struct HttpJsonProvider {
    profile: ProviderProfile,
    endpoint: HttpsEndpoint,
    auth_ref: Option<String>,
    transport: Option<Arc<dyn HttpTransport>>,
}

impl HttpJsonProvider {
    /// 生产构造：`llm-https` feature 下使用系统 TLS；否则任何检索都返回不可达。
    pub fn from_entry(entry: &ProviderEntry) -> Result<Self, ProfileError> {
        Self::with_default_transport(entry)
    }

    /// 测试注入传输。
    pub fn with_transport(
        entry: &ProviderEntry,
        transport: Arc<dyn HttpTransport>,
    ) -> Result<Self, ProfileError> {
        let endpoint = parse_https_endpoint(&entry.endpoint).map_err(|detail| ProfileError {
            field: "providers.endpoint".to_owned(),
            detail,
        })?;
        Ok(Self {
            profile: entry.profile.clone(),
            endpoint,
            auth_ref: entry.auth_ref.clone(),
            transport: Some(transport),
        })
    }

    #[cfg(feature = "llm-https")]
    fn with_default_transport(entry: &ProviderEntry) -> Result<Self, ProfileError> {
        let endpoint = parse_https_endpoint(&entry.endpoint).map_err(|detail| ProfileError {
            field: "providers.endpoint".to_owned(),
            detail,
        })?;
        let transport: Arc<dyn HttpTransport> = Arc::new(
            crate::llm::transport_tls::TlsTransport::with_allowed_hosts(&[endpoint.host.as_str()]),
        );
        Ok(Self {
            profile: entry.profile.clone(),
            endpoint,
            auth_ref: entry.auth_ref.clone(),
            transport: Some(transport),
        })
    }

    #[cfg(not(feature = "llm-https"))]
    fn with_default_transport(entry: &ProviderEntry) -> Result<Self, ProfileError> {
        let endpoint = parse_https_endpoint(&entry.endpoint).map_err(|detail| ProfileError {
            field: "providers.endpoint".to_owned(),
            detail,
        })?;
        Ok(Self {
            profile: entry.profile.clone(),
            endpoint,
            auth_ref: entry.auth_ref.clone(),
            transport: None,
        })
    }

    pub fn endpoint_url(&self) -> String {
        format!("https://{}{}", self.endpoint.host, self.endpoint.path)
    }

    fn request_path(&self, request: &ProviderRequest) -> String {
        let separator = if self.endpoint.path.contains('?') {
            '&'
        } else {
            '?'
        };
        format!(
            "{}{separator}q={}&format=json",
            self.endpoint.path,
            percent_encode(request.query.as_str())
        )
    }
}

impl fmt::Debug for HttpJsonProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpJsonProvider")
            .field("provider", &self.profile.id)
            .field("host", &self.endpoint.host)
            .field("auth", &self.auth_ref.is_some())
            .finish_non_exhaustive()
    }
}

impl SearchProvider for HttpJsonProvider {
    fn profile_id(&self) -> &ProviderId {
        &self.profile.id
    }

    fn kind(&self) -> ProviderKind {
        self.profile.kind
    }

    fn health(&self) -> ProviderHealth {
        self.profile.health
    }

    fn search(&self, request: &ProviderRequest) -> Result<ProviderResponse, ProviderFailure> {
        let Some(transport) = &self.transport else {
            return Err(ProviderFailure::Unreachable);
        };
        let started = Instant::now();
        let deadline = started + Duration::from_millis(request.timeout.as_millis());
        let cancel = CancelToken::new();

        let mut headers = vec![("Accept".to_owned(), "application/json".to_owned())];
        if let Some(auth_ref) = &self.auth_ref {
            let token = std::env::var(auth_ref).map_err(|_| ProviderFailure::AuthFailed)?;
            if token.trim().is_empty() {
                return Err(ProviderFailure::AuthFailed);
            }
            headers.push(("Authorization".to_owned(), format!("Bearer {token}")));
        }

        let http = HttpRequest {
            method: "GET".to_owned(),
            host: self.endpoint.host.clone(),
            port: self.endpoint.port,
            path: self.request_path(request),
            headers,
            body: Vec::new(),
            secure: true,
        };

        let response = transport
            .send(&http, deadline, &cancel)
            .map_err(|error| classify_transport(&error))?;

        if !(200..300).contains(&response.status) {
            return Err(match response.status {
                401 | 403 => ProviderFailure::AuthFailed,
                429 => ProviderFailure::RateLimited,
                _ => ProviderFailure::MalformedResponse,
            });
        }
        if response.body.len() > MAX_RESPONSE_BYTES {
            return Err(ProviderFailure::MalformedResponse);
        }

        let payload: RawPayload = serde_json::from_slice(&response.body)
            .map_err(|_| ProviderFailure::MalformedResponse)?;
        let raw_count = payload.results.len();
        let mut hits = Vec::new();
        for raw in payload.results {
            let url = match AbsoluteUrl::new(raw.url) {
                Ok(url) => url,
                Err(_) => continue,
            };
            let domain = url.domain().unwrap_or_default().to_owned();
            if request.is_domain_blocked(&domain) {
                continue;
            }
            let title = TitleText::new(truncate_chars(&raw.title, TitleText::LIMIT))
                .map_err(|_| ProviderFailure::MalformedResponse)?;
            hits.push(SearchHit {
                title,
                url,
                domain,
                snippet: None,
                published_at: raw.published_date.as_ref().and_then(parse_published_at),
                trust: TrustTier::GeneralWeb,
            });
        }
        if hits.is_empty() && raw_count > 0 {
            return Err(ProviderFailure::MalformedResponse);
        }
        hits.truncate(usize::from(request.limit));

        let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        Ok(ProviderResponse {
            provider: self.profile.id.clone(),
            hits,
            elapsed_ms,
        })
    }
}

#[derive(Debug, Deserialize)]
struct RawPayload {
    #[serde(default)]
    results: Vec<RawResult>,
}

#[derive(Debug, Deserialize)]
struct RawResult {
    url: String,
    #[serde(default)]
    title: String,
    #[serde(default, rename = "publishedDate")]
    published_date: Option<serde_json::Value>,
}

fn truncate_chars(raw: &str, limit: usize) -> String {
    raw.chars().take(limit).collect()
}

/// 仅保留 RFC 3986 非保留字符，其余按 UTF-8 字节百分号编码。
fn percent_encode(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for byte in raw.as_bytes() {
        let keep = byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'.' | b'_' | b'~');
        if keep {
            out.push(*byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn parse_published_at(value: &serde_json::Value) -> Option<WallClock> {
    match value {
        serde_json::Value::Number(number) => number.as_i64().map(WallClock::from_unix_millis),
        serde_json::Value::String(text) => date_to_millis(text).map(WallClock::from_unix_millis),
        _ => None,
    }
}

/// 解析 `YYYY-MM-DD`（允许带时间后缀），UTC 零点；解析失败返回 `None`。
fn date_to_millis(raw: &str) -> Option<i64> {
    let date = raw.get(..10)?;
    let mut parts = date.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let adjusted_year = if month <= 2 { year - 1 } else { year };
    let era = if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    } / 400;
    let year_of_era = adjusted_year - era * 400;
    let month_prime = (month + 9) % 12;
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    Some(days * 86_400_000)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::llm::transport::{HttpResponse, StreamReader, TransportError};
    use crate::search::profile_store::parse_document;
    use meeting_copilot_domain::model::provider::ProviderTimeout;
    use meeting_copilot_domain::vo::SanitizedQuery;

    #[derive(Default)]
    struct MockTransport {
        responses: Mutex<Vec<Result<HttpResponse, TransportError>>>,
        requests: Mutex<Vec<String>>,
    }

    impl MockTransport {
        fn with(status: u16, body: &str) -> Arc<Self> {
            let transport = Self::default();
            transport
                .responses
                .lock()
                .expect("lock")
                .push(Ok(HttpResponse {
                    status,
                    headers: Vec::new(),
                    body: body.as_bytes().to_vec(),
                }));
            Arc::new(transport)
        }

        fn failing(error: TransportError) -> Arc<Self> {
            let transport = Self::default();
            transport.responses.lock().expect("lock").push(Err(error));
            Arc::new(transport)
        }
    }

    impl HttpTransport for MockTransport {
        fn send(
            &self,
            request: &HttpRequest,
            _deadline: Instant,
            _cancel: &CancelToken,
        ) -> Result<HttpResponse, TransportError> {
            self.requests
                .lock()
                .expect("lock")
                .push(request.path.clone());
            self.responses
                .lock()
                .expect("lock")
                .pop()
                .unwrap_or(Err(TransportError::PeerClosed))
        }

        fn open_stream(
            &self,
            _request: &HttpRequest,
            _deadline: Instant,
            _cancel: &CancelToken,
        ) -> Result<Box<dyn StreamReader>, TransportError> {
            Err(TransportError::Malformed)
        }
    }

    fn entry(auth_ref: Option<&str>) -> ProviderEntry {
        let auth = auth_ref.map_or(String::new(), |value| format!(r#", "authRef": "{value}""#));
        let raw = format!(
            r#"{{"providers": [{{
                "id": "search-1", "name": "检索", "kind": "custom", "priority": 1,
                "weight": 50, "enabled": "enabled", "timeout": 8000,
                "rateLimit": {{"perMinute": 10, "perDay": 100}},
                "adapter": "http_json", "endpoint": "https://search.example.com/api"{auth}
            }}]}}"#
        );
        parse_document(&raw).expect("配置").pop().expect("条目")
    }

    fn provider(transport: Arc<MockTransport>, auth_ref: Option<&str>) -> HttpJsonProvider {
        HttpJsonProvider::with_transport(&entry(auth_ref), transport).expect("适配器")
    }

    fn request() -> ProviderRequest {
        ProviderRequest::new(
            SanitizedQuery::try_from("模型 上下文".to_owned()).expect("查询"),
            5,
            ProviderTimeout::from_millis(8_000).expect("超时"),
            vec![],
        )
        .expect("请求")
    }

    #[test]
    fn maps_auth_rate_limit_and_malformed() {
        for (status, expected) in [
            (401, ProviderFailure::AuthFailed),
            (403, ProviderFailure::AuthFailed),
            (429, ProviderFailure::RateLimited),
            (500, ProviderFailure::MalformedResponse),
        ] {
            let transport = MockTransport::with(status, "{}");
            let result = provider(transport, None).search(&request());
            assert_eq!(result.expect_err("失败"), expected, "status={status}");
        }

        let transport = MockTransport::with(200, "not-json");
        assert_eq!(
            provider(transport, None)
                .search(&request())
                .expect_err("解析"),
            ProviderFailure::MalformedResponse
        );
    }

    #[test]
    fn maps_transport_errors() {
        for (error, expected) in [
            (TransportError::ReadTimeout, ProviderFailure::Timeout),
            (TransportError::TlsFailure, ProviderFailure::TlsFailure),
            (TransportError::DnsFailure, ProviderFailure::Unreachable),
            (TransportError::TooLarge, ProviderFailure::MalformedResponse),
        ] {
            let transport = MockTransport::failing(error);
            assert_eq!(
                provider(transport, None)
                    .search(&request())
                    .expect_err("传输"),
                expected
            );
        }
    }

    #[test]
    fn parses_results_filters_blocked_domains_and_encodes_query() {
        let body = r#"{"results":[
            {"url":"https://docs.example.com/a","title":"模型上下文窗口说明","publishedDate":"2026-11-20"},
            {"url":"https://internal.example.com/b","title":"内部资料"},
            {"url":"javascript:alert(1)","title":"非法"}
        ]}"#;
        let transport = MockTransport::with(200, body);
        let mut req = request();
        req.blocked_domains = vec![meeting_copilot_domain::model::provider::DomainPattern::new(
            "*.internal.example.com",
        )
        .expect("pattern")];
        let response = provider(Arc::clone(&transport), None)
            .search(&req)
            .expect("检索");
        assert_eq!(response.hits.len(), 1);
        assert_eq!(response.hits[0].domain, "docs.example.com");
        assert_eq!(
            response.hits[0].published_at,
            Some(WallClock::from_unix_millis(1_795_132_800_000))
        );
        let path = transport.requests.lock().expect("lock")[0].clone();
        assert!(path.contains("q=%E6%A8%A1%E5%9E%8B%20%E4%B8%8A%E4%B8%8B%E6%96%87"));
        assert!(path.contains("format=json"));
    }

    #[test]
    fn response_over_two_mib_is_malformed() {
        let huge = "x".repeat(MAX_RESPONSE_BYTES + 1);
        let transport = MockTransport::with(200, &huge);
        assert_eq!(
            provider(transport, None)
                .search(&request())
                .expect_err("超限"),
            ProviderFailure::MalformedResponse
        );
    }

    #[test]
    fn missing_credential_maps_to_auth_failed_without_leaking() {
        std::env::remove_var("U09_TEST_MISSING_TOKEN");
        let transport = MockTransport::with(200, r#"{"results":[]}"#);
        let provider = provider(transport, Some("U09_TEST_MISSING_TOKEN"));
        assert_eq!(
            provider.search(&request()).expect_err("凭据缺失"),
            ProviderFailure::AuthFailed
        );
        assert!(!format!("{provider:?}").contains("Bearer"));
    }

    #[test]
    fn debug_and_error_do_not_expose_query() {
        let transport = MockTransport::with(401, "{}");
        let provider = provider(transport, None);
        let debug = format!("{provider:?}");
        assert!(!debug.contains("模型"));
        let error = provider.search(&request()).expect_err("失败");
        assert!(!error.to_string().contains("模型"));
    }
}
