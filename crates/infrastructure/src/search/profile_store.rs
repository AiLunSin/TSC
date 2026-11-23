//! `providers.json` 读取与字段级校验（SPEC 8.2、6.9）。
//!
//! 文件缺失或启用列表为空即零源；没有任何默认源或示例源。
//! `authRef` 只保存环境变量名，凭据值不进入配置、数据库、缓存、日志与审计。

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use serde::Deserialize;

use meeting_copilot_domain::model::enums::{EnabledState, ProviderHealth, ProviderKind};
use meeting_copilot_domain::model::provider::{
    DomainPattern, ProviderProfile, ProviderTimeout, RateLimit, Weight,
};
use meeting_copilot_domain::vo::ProviderId;

pub const DEFAULT_SCHEMA_VERSION: &str = "1.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileError {
    pub field: String,
    pub detail: String,
}

impl ProfileError {
    fn new(field: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            detail: detail.into(),
        }
    }
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.detail)
    }
}

impl std::error::Error for ProfileError {}

/// 本增量只支持 `http_json`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAdapter {
    HttpJson,
}

impl SearchAdapter {
    pub const fn as_str(self) -> &'static str {
        "http_json"
    }

    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "http_json" => Some(Self::HttpJson),
            _ => None,
        }
    }
}

/// 已校验的检索源配置。`profile` 复用领域模型，另带本增量需要的适配器字段。
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderEntry {
    /// SPEC 6.9-2：仅用于配置与统计展示，不超过 50 字。
    pub name: String,
    pub profile: ProviderProfile,
    pub adapter: SearchAdapter,
    /// 必须为 HTTPS 绝对地址，不含凭据。
    pub endpoint: String,
    /// 环境变量名；不是密钥本身。
    pub auth_ref: Option<String>,
}

impl ProviderEntry {
    pub fn profile_id(&self) -> &ProviderId {
        &self.profile.id
    }
}

/// 解析后的 HTTPS 端点；`path` 可能已带查询串。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpsEndpoint {
    pub host: String,
    pub port: u16,
    pub path: String,
}

/// 只接受 `https://` 绝对地址，拒绝 userinfo、片段与空白。
pub fn parse_https_endpoint(raw: &str) -> Result<HttpsEndpoint, String> {
    let trimmed = raw.trim();
    if trimmed.chars().any(char::is_whitespace) {
        return Err("不允许空白".to_owned());
    }
    let rest = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("HTTPS://"))
        .ok_or_else(|| "必须为 https:// 地址".to_owned())?;
    if rest.is_empty() {
        return Err("缺少主机".to_owned());
    }
    if rest.contains('#') {
        return Err("不允许 URL 片段".to_owned());
    }
    let authority_end = rest.find(['/', '?']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    if authority.is_empty() || authority.contains('@') {
        return Err("主机非法".to_owned());
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => {
            let port = port
                .parse::<u16>()
                .ok()
                .filter(|port| *port != 0)
                .ok_or_else(|| "端口非法".to_owned())?;
            (host, port)
        }
        None => (authority, 443),
    };
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty() || host.contains(char::is_whitespace) {
        return Err("主机非法".to_owned());
    }
    let tail = &rest[authority_end..];
    let path = if tail.is_empty() {
        "/".to_owned()
    } else if tail.starts_with('/') {
        tail.to_owned()
    } else {
        format!("/{tail}")
    };
    Ok(HttpsEndpoint { host, port, path })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawDocument {
    #[serde(default = "default_schema_version")]
    schema_version: String,
    #[serde(default)]
    providers: Vec<RawProvider>,
}

fn default_schema_version() -> String {
    DEFAULT_SCHEMA_VERSION.to_owned()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawProvider {
    id: String,
    name: String,
    kind: ProviderKind,
    priority: i32,
    weight: f64,
    #[serde(default)]
    enabled: EnabledState,
    timeout: u64,
    rate_limit: RawRateLimit,
    #[serde(default)]
    blocked_domains: Vec<String>,
    #[serde(default)]
    health: ProviderHealth,
    adapter: String,
    endpoint: String,
    #[serde(default)]
    auth_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawRateLimit {
    per_minute: u16,
    per_day: u32,
}

impl RawProvider {
    fn into_entry(self, index: usize) -> Result<ProviderEntry, ProfileError> {
        let field = |name: &str| format!("providers[{index}].{name}");

        let name = self.name.trim().to_owned();
        if name.is_empty() || name.chars().count() > 50 {
            return Err(ProfileError::new(field("name"), "必填且不超过 50 字"));
        }

        let id = ProviderId::new(self.id)
            .map_err(|error| ProfileError::new(field("id"), error.to_string()))?;

        if self.priority < 0 {
            return Err(ProfileError::new(field("priority"), "不得为负"));
        }

        // SPEC 6.9-7：权重 0–100；领域权重为 (0, 1]，0 视为非法。
        if !self.weight.is_finite() || self.weight <= 0.0 || self.weight > 100.0 {
            return Err(ProfileError::new(field("weight"), "必须落在 (0, 100]"));
        }
        let weight = Weight::new(self.weight / 100.0)
            .map_err(|error| ProfileError::new(field("weight"), error.to_string()))?;

        let timeout = ProviderTimeout::from_millis(self.timeout)
            .map_err(|error| ProfileError::new(field("timeout"), error.to_string()))?;

        let adapter = SearchAdapter::parse(&self.adapter)
            .ok_or_else(|| ProfileError::new(field("adapter"), "本增量仅支持 http_json"))?;

        parse_https_endpoint(&self.endpoint)
            .map_err(|detail| ProfileError::new(field("endpoint"), detail))?;

        if let Some(auth_ref) = &self.auth_ref {
            validate_env_name(auth_ref)
                .map_err(|detail| ProfileError::new(field("authRef"), detail))?;
        }

        let blocked_domains = self
            .blocked_domains
            .into_iter()
            .map(|raw| {
                DomainPattern::new(raw)
                    .map_err(|error| ProfileError::new(field("blockedDomains"), error.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ProviderEntry {
            name,
            profile: ProviderProfile {
                id,
                kind: self.kind,
                priority: self.priority,
                weight,
                enabled: self.enabled,
                timeout,
                rate_limit: RateLimit {
                    per_minute: self.rate_limit.per_minute,
                    per_day: self.rate_limit.per_day,
                },
                blocked_domains,
                health: self.health,
                endpoint_ref: None,
            },
            adapter,
            endpoint: self.endpoint,
            auth_ref: self.auth_ref,
        })
    }
}

/// 只接受环境变量名，避免把密钥误写进配置。
fn validate_env_name(raw: &str) -> Result<(), String> {
    let mut chars = raw.chars();
    let Some(first) = chars.next() else {
        return Err("不能为空".to_owned());
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err("必须是环境变量名".to_owned());
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("必须是环境变量名".to_owned());
    }
    Ok(())
}

/// 解析整份文档；成功即返回全部条目（可能为空）。
pub fn parse_document(raw: &str) -> Result<Vec<ProviderEntry>, ProfileError> {
    let document: RawDocument = serde_json::from_str(raw)
        .map_err(|error| ProfileError::new("providers.json", error.to_string()))?;
    if document.schema_version != DEFAULT_SCHEMA_VERSION {
        return Err(ProfileError::new(
            "schemaVersion",
            format!("仅支持 {DEFAULT_SCHEMA_VERSION}"),
        ));
    }
    if document.providers.len() > 64 {
        return Err(ProfileError::new("providers", "最多 64 条"));
    }

    let mut entries = Vec::with_capacity(document.providers.len());
    let mut ids = Vec::new();
    let mut priorities = Vec::new();
    for (index, provider) in document.providers.into_iter().enumerate() {
        let entry = provider.into_entry(index)?;
        if ids.iter().any(|id| id == &entry.profile.id) {
            return Err(ProfileError::new(
                format!("providers[{index}].id"),
                "同一客户端内不得重复",
            ));
        }
        if priorities.contains(&entry.profile.priority) {
            return Err(ProfileError::new(
                format!("providers[{index}].priority"),
                "同一配置内不得重复",
            ));
        }
        ids.push(entry.profile.id.clone());
        priorities.push(entry.profile.priority);
        entries.push(entry);
    }
    Ok(entries)
}

#[derive(Default)]
struct Cached {
    stamp: Option<(SystemTime, u64)>,
    entries: Vec<ProviderEntry>,
}

/// 配置文件读取器。文件缺失视为空列表（零源）。
pub struct ProfileStore {
    path: PathBuf,
    cached: Mutex<Cached>,
}

impl ProfileStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            cached: Mutex::new(Cached::default()),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 无条件读取并解析。
    pub fn load(&self) -> Result<Vec<ProviderEntry>, ProfileError> {
        match fs::read_to_string(&self.path) {
            Ok(raw) => parse_document(&raw),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(ProfileError::new(
                "providers.json",
                format!("读取失败：{}", error.kind()),
            )),
        }
    }

    /// 文件时间戳/长度变化时才重新解析，实现"下一次检索生效"。
    pub fn load_if_changed(&self) -> Result<Vec<ProviderEntry>, ProfileError> {
        let stamp = fs::metadata(&self.path)
            .ok()
            .and_then(|meta| Some((meta.modified().ok()?, meta.len())));

        let mut cached = self
            .cached
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if stamp.is_some() && stamp == cached.stamp {
            return Ok(cached.entries.clone());
        }
        let entries = self.load()?;
        cached.stamp = stamp;
        cached.entries = entries.clone();
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"{
        "schemaVersion": "1.0",
        "providers": [{
            "id": "corp-search",
            "name": "公司检索",
            "kind": "custom",
            "priority": 1,
            "weight": 60,
            "enabled": "enabled",
            "timeout": 8000,
            "rateLimit": { "perMinute": 30, "perDay": 1000 },
            "blockedDomains": ["*.internal.example.com"],
            "health": "healthy",
            "adapter": "http_json",
            "endpoint": "https://search.example.com/api",
            "authRef": "SEARCH_API_TOKEN"
        }]
    }"#;

    fn replace(from: &str, to: &str) -> String {
        VALID.replace(from, to)
    }

    #[test]
    fn valid_document_is_parsed_without_secret_values() {
        let entries = parse_document(VALID).expect("合法配置");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "公司检索");
        assert_eq!(entries[0].profile.priority, 1);
        assert!((entries[0].profile.weight.value() - 0.6).abs() < f64::EPSILON);
        assert_eq!(entries[0].profile.timeout.as_millis(), 8_000);
        assert_eq!(entries[0].auth_ref.as_deref(), Some("SEARCH_API_TOKEN"));
        assert_eq!(entries[0].endpoint, "https://search.example.com/api");
    }

    #[test]
    fn empty_providers_is_valid_zero_source() {
        assert!(parse_document(r#"{"providers": []}"#)
            .expect("零源")
            .is_empty());
    }

    #[test]
    fn missing_file_is_zero_source() {
        let store = ProfileStore::new("definitely-missing-providers.json");
        assert!(store.load().expect("缺失即零源").is_empty());
        assert!(store.load_if_changed().expect("缺失即零源").is_empty());
    }

    #[test]
    fn duplicate_id_is_rejected_with_field_path() {
        let raw = VALID.replace(
            r#""providers": ["#,
            r#""providers": [{"id": "corp-search", "name": "重复", "kind": "custom",
                "priority": 2, "weight": 10, "enabled": "enabled", "timeout": 3000,
                "rateLimit": {"perMinute": 1, "perDay": 10}, "adapter": "http_json",
                "endpoint": "https://other.example.com/api"}, "#,
        );
        let error = parse_document(&raw).expect_err("重复 id");
        assert_eq!(error.field, "providers[1].id");
    }

    #[test]
    fn duplicate_priority_and_out_of_range_values_are_rejected() {
        let raw = VALID.replace(
            r#""providers": ["#,
            r#""providers": [{"id": "second", "name": "第二个", "kind": "custom",
                "priority": 1, "weight": 10, "enabled": "enabled", "timeout": 3000,
                "rateLimit": {"perMinute": 1, "perDay": 10}, "adapter": "http_json",
                "endpoint": "https://other.example.com/api"}, "#,
        );
        assert_eq!(
            parse_document(&raw).expect_err("重复 priority").field,
            "providers[1].priority"
        );

        let too_long = replace(r#""timeout": 8000"#, r#""timeout": 8001"#);
        assert_eq!(
            parse_document(&too_long).expect_err("超时越界").field,
            "providers[0].timeout"
        );

        let zero_weight = replace(r#""weight": 60"#, r#""weight": 0"#);
        assert_eq!(
            parse_document(&zero_weight).expect_err("权重为 0").field,
            "providers[0].weight"
        );
    }

    #[test]
    fn non_https_endpoint_is_rejected() {
        let raw = replace(
            "https://search.example.com/api",
            "http://search.example.com/api",
        );
        assert_eq!(
            parse_document(&raw).expect_err("非 HTTPS").field,
            "providers[0].endpoint"
        );
    }

    #[test]
    fn auth_ref_must_be_environment_variable_name() {
        let raw = replace("SEARCH_API_TOKEN", "https://token@example.com");
        assert_eq!(
            parse_document(&raw).expect_err("密钥字面量").field,
            "providers[0].authRef"
        );
    }

    #[test]
    fn endpoint_parser_rejects_userinfo_and_fragment() {
        assert!(parse_https_endpoint("https://user:pw@example.com/a").is_err());
        assert!(parse_https_endpoint("https://example.com/a#frag").is_err());
        let parsed = parse_https_endpoint("https://Example.COM:8443/api?x=1").expect("合法");
        assert_eq!(parsed.host, "example.com");
        assert_eq!(parsed.port, 8443);
        assert_eq!(parsed.path, "/api?x=1");
    }

    #[test]
    fn change_is_picked_up_on_next_load() {
        let stamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("providers-{}-{stamp}.json", std::process::id()));
        fs::write(&path, r#"{"providers": []}"#).expect("write");
        let store = ProfileStore::new(&path);
        assert!(store.load_if_changed().expect("first").is_empty());

        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(&path, VALID).expect("rewrite");
        assert_eq!(store.load_if_changed().expect("reload").len(), 1);
        let _ = fs::remove_file(&path);
    }
}
