//! 连接测试与失败分类（SPEC 8.3-4、6.9-10）。
//!
//! 六类失败固定为 `Timeout / RateLimited / AuthFailed / MalformedResponse /
//! TlsFailure / Unreachable`。连续失败达到阈值置 `Unavailable`，成功即恢复。

use std::collections::HashMap;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use meeting_copilot_domain::model::enums::ProviderHealth;
use meeting_copilot_domain::port::search::{
    OutboundPolicy, ProviderFailure, ProviderRequest, SearchProvider,
};
use meeting_copilot_domain::vo::ProviderId;

use crate::llm::transport::TransportError;

/// 连续失败达到该次数即标记不可用。
pub const FAILURE_THRESHOLD: u8 = 3;

/// 到 `host:port` 的 TCP 可达性：只建连接、立即关闭，不发请求、不消耗检索额度。
///
/// 供断网恢复识别使用；外发是否成功仍以真实检索结果为准。
pub fn tcp_reachable(host: &str, port: u16, timeout: Duration) -> bool {
    let Ok(addresses) = (host, port).to_socket_addrs() else {
        return false;
    };
    addresses
        .into_iter()
        .any(|address| TcpStream::connect_timeout(&address, timeout).is_ok())
}

#[derive(Debug, Default)]
pub struct HealthMonitor {
    failures: HashMap<ProviderId, u8>,
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn health(&self, provider: &ProviderId) -> ProviderHealth {
        match self.failures.get(provider).copied().unwrap_or(0) {
            0 => ProviderHealth::Healthy,
            count if count >= FAILURE_THRESHOLD => ProviderHealth::Unavailable,
            _ => ProviderHealth::Degraded,
        }
    }

    pub fn record_success(&mut self, provider: &ProviderId) {
        self.failures.remove(provider);
    }

    pub fn record_failure(&mut self, provider: &ProviderId) -> ProviderHealth {
        let count = self.failures.entry(provider.clone()).or_insert(0);
        *count = count.saturating_add(1);
        self.health(provider)
    }
}

/// 传输错误到六类失败的唯一映射。
pub fn classify_transport(error: &TransportError) -> ProviderFailure {
    match error {
        TransportError::DnsFailure
        | TransportError::NetworkUnreachable
        | TransportError::ConnectRefused
        | TransportError::ConnectTimeout => ProviderFailure::Unreachable,
        TransportError::ReadTimeout | TransportError::BudgetExhausted => ProviderFailure::Timeout,
        TransportError::TlsFailure => ProviderFailure::TlsFailure,
        TransportError::TooLarge | TransportError::Malformed | TransportError::PeerClosed => {
            ProviderFailure::MalformedResponse
        }
        TransportError::Cancelled => ProviderFailure::Timeout,
    }
}

/// 连接测试：先经过外发白名单，再发起一次最小检索。
pub fn test_connection(
    provider: &dyn SearchProvider,
    policy: &dyn OutboundPolicy,
    endpoint: &str,
    request: &ProviderRequest,
) -> Result<(), ProviderFailure> {
    policy.check(
        provider.profile_id().as_str(),
        endpoint,
        &request.resolved_ips,
    )?;
    provider.search(request).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    use meeting_copilot_domain::model::enums::ProviderKind;
    use meeting_copilot_domain::model::provider::ProviderTimeout;
    use meeting_copilot_domain::port::search::ProviderResponse;
    use meeting_copilot_domain::vo::SanitizedQuery;

    fn provider_id(raw: &str) -> ProviderId {
        ProviderId::new(raw).expect("id")
    }

    struct NeverProvider {
        id: ProviderId,
    }

    impl SearchProvider for NeverProvider {
        fn profile_id(&self) -> &ProviderId {
            &self.id
        }

        fn kind(&self) -> ProviderKind {
            ProviderKind::Custom
        }

        fn health(&self) -> ProviderHealth {
            ProviderHealth::Healthy
        }

        fn search(&self, _request: &ProviderRequest) -> Result<ProviderResponse, ProviderFailure> {
            panic!("白名单拒绝后不得发起检索");
        }
    }

    struct DenyPolicy;

    impl OutboundPolicy for DenyPolicy {
        fn check(
            &self,
            _provider_profile_id: &str,
            _url: &str,
            _resolved_ips: &[IpAddr],
        ) -> Result<(), ProviderFailure> {
            Err(ProviderFailure::Unreachable)
        }
    }

    fn request() -> ProviderRequest {
        ProviderRequest::new(
            SanitizedQuery::try_from("模型".to_owned()).expect("query"),
            1,
            ProviderTimeout::from_millis(1_000).expect("timeout"),
            Vec::new(),
        )
        .expect("request")
    }

    #[test]
    fn six_failure_classes_are_stable() {
        for (error, expected) in [
            (TransportError::ReadTimeout, ProviderFailure::Timeout),
            (TransportError::TlsFailure, ProviderFailure::TlsFailure),
            (TransportError::TooLarge, ProviderFailure::MalformedResponse),
            (
                TransportError::Malformed,
                ProviderFailure::MalformedResponse,
            ),
            (
                TransportError::PeerClosed,
                ProviderFailure::MalformedResponse,
            ),
            (TransportError::DnsFailure, ProviderFailure::Unreachable),
            (TransportError::ConnectRefused, ProviderFailure::Unreachable),
        ] {
            assert_eq!(classify_transport(&error), expected);
        }
        for failure in [
            ProviderFailure::Timeout,
            ProviderFailure::RateLimited,
            ProviderFailure::AuthFailed,
            ProviderFailure::MalformedResponse,
            ProviderFailure::TlsFailure,
            ProviderFailure::Unreachable,
        ] {
            assert!(!failure.as_str().is_empty());
        }
    }

    #[test]
    fn connection_test_goes_through_outbound_policy() {
        let provider = NeverProvider {
            id: provider_id("p1"),
        };
        let result = test_connection(
            &provider,
            &DenyPolicy,
            "https://search.example.com/api",
            &request(),
        );
        assert_eq!(result, Err(ProviderFailure::Unreachable));
    }

    #[test]
    fn three_consecutive_failures_mark_unavailable_and_success_recovers() {
        let mut monitor = HealthMonitor::new();
        let id = provider_id("p1");
        assert_eq!(monitor.health(&id), ProviderHealth::Healthy);

        assert_eq!(monitor.record_failure(&id), ProviderHealth::Degraded);
        assert_eq!(monitor.record_failure(&id), ProviderHealth::Degraded);
        assert_eq!(monitor.record_failure(&id), ProviderHealth::Unavailable);
        assert_eq!(monitor.health(&id), ProviderHealth::Unavailable);

        monitor.record_success(&id);
        assert_eq!(monitor.health(&id), ProviderHealth::Healthy);
    }
}
