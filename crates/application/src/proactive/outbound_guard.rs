//! 按需补全的网页正文外发白名单（SPEC 7.3.1-3、DESIGN 2.10.3）。
//!
//! 只允许已启用的检索源、HTTPS、目标主机与配置一致，并拒绝环回、私网与
//! 链路本地地址；不做 DNS 解析，解析结果由调用方传入。

use std::net::IpAddr;

const REQUIRED_SCHEME: &str = "https";
const SCHEME_SEPARATOR: &str = "://";
const MAX_SAME_HOST_REDIRECTS: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowedEndpoint {
    pub provider_profile_id: String,
    pub host: String,
}

impl AllowedEndpoint {
    pub fn new(provider_profile_id: impl Into<String>, host: impl Into<String>) -> Self {
        Self {
            provider_profile_id: provider_profile_id.into(),
            host: host
                .into()
                .trim()
                .trim_end_matches('.')
                .to_ascii_lowercase(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboundTarget {
    pub provider_profile_id: String,
    pub provider_enabled: bool,
    pub url: String,
    /// 调用方解析出的地址；为空时只校验字面主机。
    pub resolved_ips: Vec<IpAddr>,
    pub cross_host_redirects: u8,
    pub same_host_redirects: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboundReject {
    ProviderDisabled,
    ProviderUnknown,
    NotHttps,
    Malformed,
    HostMismatch,
    LoopbackAddress,
    PrivateAddress,
    LinkLocalAddress,
    RedirectCrossHost,
    RedirectTooMany,
}

impl OutboundReject {
    /// 供审计写入的稳定标签，不包含 URL 或主机。
    pub const fn audit_label(self) -> &'static str {
        match self {
            Self::ProviderDisabled => "provider_disabled",
            Self::ProviderUnknown => "provider_unknown",
            Self::NotHttps => "not_https",
            Self::Malformed => "malformed",
            Self::HostMismatch => "host_mismatch",
            Self::LoopbackAddress => "loopback",
            Self::PrivateAddress => "private_address",
            Self::LinkLocalAddress => "link_local",
            Self::RedirectCrossHost => "redirect_cross_host",
            Self::RedirectTooMany => "redirect_too_many",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct OutboundGuard {
    endpoints: Vec<AllowedEndpoint>,
}

impl OutboundGuard {
    pub fn new(endpoints: Vec<AllowedEndpoint>) -> Self {
        Self { endpoints }
    }

    pub fn check(&self, target: &OutboundTarget) -> Result<(), OutboundReject> {
        if !target.provider_enabled {
            return Err(OutboundReject::ProviderDisabled);
        }
        let endpoint = self
            .endpoints
            .iter()
            .find(|endpoint| endpoint.provider_profile_id == target.provider_profile_id)
            .ok_or(OutboundReject::ProviderUnknown)?;

        if target.cross_host_redirects > 0 {
            return Err(OutboundReject::RedirectCrossHost);
        }
        if target.same_host_redirects > MAX_SAME_HOST_REDIRECTS {
            return Err(OutboundReject::RedirectTooMany);
        }

        let (scheme, rest) = target
            .url
            .split_once(SCHEME_SEPARATOR)
            .ok_or(OutboundReject::Malformed)?;
        if !scheme.eq_ignore_ascii_case(REQUIRED_SCHEME) {
            return Err(OutboundReject::NotHttps);
        }
        let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
        if authority.is_empty() || authority.contains('@') {
            return Err(OutboundReject::Malformed);
        }
        let host = authority_host(authority).ok_or(OutboundReject::Malformed)?;
        if !host.eq_ignore_ascii_case(&endpoint.host) {
            return Err(OutboundReject::HostMismatch);
        }
        if is_localhost_name(host) {
            return Err(OutboundReject::LoopbackAddress);
        }
        if let Ok(ip) = host.parse::<IpAddr>() {
            classify_ip(ip)?;
        }
        for ip in &target.resolved_ips {
            classify_ip(*ip)?;
        }
        Ok(())
    }
}

fn authority_host(authority: &str) -> Option<&str> {
    if let Some(rest) = authority.strip_prefix('[') {
        let host = rest.split(']').next()?;
        return if host.is_empty() { None } else { Some(host) };
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    if let Some(port) = port {
        if !port.is_empty() && port.parse::<u16>().is_err() {
            return None;
        }
    }
    if host.is_empty() {
        None
    } else {
        Some(host)
    }
}

fn is_localhost_name(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost") || host.to_ascii_lowercase().ends_with(".localhost")
}

fn classify_ip(ip: IpAddr) -> Result<(), OutboundReject> {
    match ip {
        IpAddr::V4(ipv4) => {
            if ipv4.is_loopback() {
                Err(OutboundReject::LoopbackAddress)
            } else if ipv4.is_private() {
                Err(OutboundReject::PrivateAddress)
            } else if ipv4.is_link_local() {
                Err(OutboundReject::LinkLocalAddress)
            } else if ipv4.is_unspecified() || ipv4.is_broadcast() {
                Err(OutboundReject::PrivateAddress)
            } else {
                Ok(())
            }
        }
        IpAddr::V6(ipv6) => {
            if ipv6.is_loopback() {
                Err(OutboundReject::LoopbackAddress)
            } else if ipv6.is_unspecified() {
                Err(OutboundReject::PrivateAddress)
            } else if let Some(mapped) = ipv6.to_ipv4_mapped() {
                classify_ip(IpAddr::V4(mapped))
            } else {
                let segments = ipv6.segments();
                if segments[0] & 0xfe00 == 0xfc00 {
                    Err(OutboundReject::PrivateAddress)
                } else if segments[0] & 0xffc0 == 0xfe80 {
                    Err(OutboundReject::LinkLocalAddress)
                } else {
                    Ok(())
                }
            }
        }
    }
}
