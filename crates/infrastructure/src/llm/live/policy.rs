//! 服务商锁定策略（AC-28/29/30；DESIGN 2.10）。
//!
//! 纯函数、零 IO、零凭据读取。host 必须**逐字节**命中允许集合，禁止后缀匹配。

/// SPEC 11.9.1-3 唯一放行的具体 host。
pub const DEEPSEEK_HOSTS: &[&str] = &["api.deepseek.com"];

/// C1 已确认的唯一路径段（不含 host）。
pub const PATH_CONSTANT: &str = "/chat/completions";

/// 仅允许与常量逐字节相等；不 trim、不做大小写与尾斜杠归一化。
pub fn path_confirmed(path: &str) -> bool {
    path.as_bytes() == PATH_CONSTANT.as_bytes()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostVerdict {
    Allowed,
    NotAllowed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemeVerdict {
    Https,
    InsecureRejected,
    Unparsable,
}

/// 放行后传给 `AdmissionTicket` 的协议标记（只可能为 HTTPS）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Https,
}

fn authority_of(raw_url: &str) -> Option<&str> {
    let rest = raw_url
        .strip_prefix("https://")
        .or_else(|| raw_url.strip_prefix("http://"))?;
    Some(rest.split(['/', '?', '#']).next().unwrap_or(""))
}

fn is_ip_literal(host: &str) -> bool {
    if host.starts_with('[') {
        return true;
    }
    host.chars().all(|c| c.is_ascii_digit() || c == '.') && host.chars().any(|c| c == '.')
}

/// 允许集合内的 host 必须逐字节相等；端口只允许 443。
pub fn host_is_locked(raw_url: &str, allowed: &[&str]) -> HostVerdict {
    let Some(authority) = authority_of(raw_url) else {
        return HostVerdict::NotAllowed;
    };
    if authority.is_empty() || authority.contains('@') {
        return HostVerdict::NotAllowed;
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => match port.parse::<u16>() {
            Ok(port) => (host, Some(port)),
            Err(_) => return HostVerdict::NotAllowed,
        },
        None => (authority, None),
    };
    if port.is_some_and(|port| port != 443) {
        return HostVerdict::NotAllowed;
    }
    if host.is_empty() || host.ends_with('.') || is_ip_literal(host) {
        return HostVerdict::NotAllowed;
    }
    let normalized = host.to_ascii_lowercase();
    if normalized.starts_with("xn--") || normalized.contains(".xn--") {
        return HostVerdict::NotAllowed;
    }
    if allowed.iter().any(|item| normalized == *item) {
        HostVerdict::Allowed
    } else {
        HostVerdict::NotAllowed
    }
}

/// 返回归一化后的 host；仅在 [`host_is_locked`] 放行时返回。
pub fn locked_host(raw_url: &str) -> Option<String> {
    if host_is_locked(raw_url, DEEPSEEK_HOSTS) != HostVerdict::Allowed {
        return None;
    }
    let authority = authority_of(raw_url)?;
    let host = match authority.rsplit_once(':') {
        Some((host, _)) => host,
        None => authority,
    };
    Some(host.to_ascii_lowercase())
}

pub fn scheme_verdict(raw_url: &str) -> SchemeVerdict {
    if raw_url.starts_with("https://") {
        SchemeVerdict::Https
    } else if raw_url.starts_with("http://") {
        SchemeVerdict::InsecureRejected
    } else {
        SchemeVerdict::Unparsable
    }
}

/// 逐字节比较；不 trim、不改大小写；未确认（None）一律 false。
pub fn model_confirmed(actual: &str, confirmed: Option<&str>) -> bool {
    match confirmed {
        Some(confirmed) => actual.as_bytes() == confirmed.as_bytes(),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOST: &str = "api.deepseek.com";

    #[test]
    fn host_is_locked_is_exact_not_suffix() {
        let https = format!("https://{HOST}");
        for suffix in ["", "/v1/chat", ":443/v1/chat"] {
            assert_eq!(
                host_is_locked(&format!("{https}{suffix}"), &[HOST]),
                HostVerdict::Allowed,
                "{suffix}"
            );
        }
        assert_eq!(
            host_is_locked(
                &format!("https://{}/v1", HOST.to_ascii_uppercase()),
                &[HOST]
            ),
            HostVerdict::Allowed
        );
        for raw in [
            format!("https://evil.{HOST}/v1"),
            format!("https://{HOST}.evil.com/v1"),
            format!("https://x{HOST}/v1"),
            "https://api.deepseek.co/v1".to_owned(),
            format!("https://{HOST}:8443/v1"),
            format!("https://user@{HOST}/v1"),
            format!("https://{HOST}./v1"),
            "https://127.0.0.1/v1".to_owned(),
            "https://[::1]/v1".to_owned(),
            format!("https://xn--{HOST}/v1"),
            format!("ftp://{HOST}/v1"),
            format!("{HOST}/v1"),
        ] {
            assert_eq!(
                host_is_locked(&raw, &[HOST]),
                HostVerdict::NotAllowed,
                "{raw}"
            );
        }
    }

    #[test]
    fn scheme_verdict_three_states() {
        assert_eq!(
            scheme_verdict(&format!("https://{HOST}/v1")),
            SchemeVerdict::Https
        );
        assert_eq!(
            scheme_verdict(&format!("http://{HOST}/v1")),
            SchemeVerdict::InsecureRejected
        );
        assert_eq!(
            scheme_verdict(&format!("{HOST}/v1")),
            SchemeVerdict::Unparsable
        );
    }

    #[test]
    fn model_confirmed_five_values() {
        assert!(!model_confirmed("", Some("m")));
        assert!(!model_confirmed("   ", Some("m")));
        assert!(!model_confirmed("candidate?", Some("m")));
        assert!(!model_confirmed("M", Some("m")));
        assert!(model_confirmed("m", Some("m")));
        assert!(!model_confirmed("m", None));
    }

    #[test]
    fn locked_host_is_none_on_any_rejection() {
        assert_eq!(
            locked_host(&format!("https://{HOST}/v1")),
            Some(HOST.to_owned())
        );
        assert!(locked_host(&format!("https://evil.{HOST}/v1")).is_none());
    }

    #[test]
    fn only_https_maps_to_ticket_scheme() {
        assert_eq!(scheme_verdict("https://x"), SchemeVerdict::Https);
        let scheme = Scheme::Https;
        assert_eq!(scheme, Scheme::Https);
    }

    #[test]
    fn path_confirmed_five_values() {
        assert!(path_confirmed(PATH_CONSTANT));
        for path in ["/", "/v1/chat/completions", "", "/chat/completions/"] {
            assert!(!path_confirmed(path), "{path:?}");
        }
    }
}
