//! 设置向导端口：凭据存储、非敏感配置、连通性探针。
//!
//! 本模块只定义纯数据与 trait，不含平台 API、文件 IO、网络与时钟调用；
//! 具体实现在平台层与装配层。

/// 非敏感配置默认 schema 版本。
pub const SETTINGS_DEFAULT_SCHEMA_VERSION: u32 = 1;
/// 非敏感配置默认超时（毫秒）。
pub const SETTINGS_DEFAULT_TIMEOUT_MS: u64 = 6_000;
/// 超时允许区间下界（毫秒，闭区间）。
pub const SETTINGS_TIMEOUT_MIN_MS: u64 = 100;
/// 超时允许区间上界（毫秒，闭区间）。
pub const SETTINGS_TIMEOUT_MAX_MS: u64 = 8_000;

/// 端口统一错误：恰 4 类，不带任何载荷文本。
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PortError {
    /// 当前平台或能力未启用。
    #[error("port unsupported")]
    Unsupported,
    /// 系统拒绝访问。
    #[error("port denied")]
    Denied,
    /// 系统暂不可用。
    #[error("port unavailable")]
    Unavailable,
    /// 读取内容非法或损坏。
    #[error("port corrupted")]
    Corrupted,
}

/// 一次性凭据字节：无序列化、无展示、无长度访问器。
pub struct SecretBytes {
    bytes: Vec<u8>,
}

impl SecretBytes {
    /// 以原始字节构造；调用方必须在进程内一次性使用。
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    /// 仅在端口实现内部一次性借用字节；返回值不得存留。
    pub fn with_bytes<R>(&self, f: impl FnOnce(&[u8]) -> R) -> R {
        f(&self.bytes)
    }
}

impl core::fmt::Debug for SecretBytes {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("SecretBytes(***)")
    }
}

/// 非敏感配置：恰 4 项，默认 `1/0/0/6000`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingsConfig {
    /// 配置 schema 版本，只接受 1。
    pub schema_version: u32,
    /// 模型下标（越界归 0）。
    pub model_index: usize,
    /// 端点下标（越界归 0）。
    pub endpoint_index: usize,
    /// 超时毫秒（越界归 6 000）。
    pub timeout_ms: u64,
}

impl Default for SettingsConfig {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_DEFAULT_SCHEMA_VERSION,
            model_index: 0,
            endpoint_index: 0,
            timeout_ms: SETTINGS_DEFAULT_TIMEOUT_MS,
        }
    }
}

impl SettingsConfig {
    /// 按清单规模归一：越界下标归 0，超时越界归默认值。
    pub fn normalized(&self, model_count: usize, endpoint_count: usize) -> Self {
        Self {
            schema_version: SETTINGS_DEFAULT_SCHEMA_VERSION,
            model_index: if self.model_index < model_count {
                self.model_index
            } else {
                0
            },
            endpoint_index: if self.endpoint_index < endpoint_count {
                self.endpoint_index
            } else {
                0
            },
            timeout_ms: self.normalized_timeout_ms(),
        }
    }

    /// 超时单项归一。
    pub fn normalized_timeout_ms(&self) -> u64 {
        if (SETTINGS_TIMEOUT_MIN_MS..=SETTINGS_TIMEOUT_MAX_MS).contains(&self.timeout_ms) {
            self.timeout_ms
        } else {
            SETTINGS_DEFAULT_TIMEOUT_MS
        }
    }
}

/// 一次探针调用：只在单次命令栈内存在。
pub struct ProbeCall {
    /// 内存凭据。
    pub credential: SecretBytes,
    /// 目录清单内的模型显示名。
    pub model_name: String,
    /// 端点下标。
    pub endpoint_index: usize,
    /// 已归一的超时毫秒。
    pub timeout_ms: u64,
}

/// 一次探针回执：恰 4 项。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeAttempt {
    /// 是否成功。
    pub succeeded: bool,
    /// 成功时为空，失败时为 8 类之一。
    pub failure: Option<ProbeFailure>,
    /// 耗时毫秒。
    pub elapsed_ms: u64,
    /// 冷却剩余毫秒（相对值）；大于 0 才表示冷却。
    pub remaining_ms: Option<u64>,
}

/// 探针失败类别：恰 8 态，与 51 `ProbeFailureKind` 同名同序。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeFailure {
    /// 缺少 Key。
    MissingKey,
    /// Key 被拒绝。
    KeyRejected,
    /// 端点不可达。
    EndpointUnreachable,
    /// 超时。
    Timeout,
    /// 限流。
    RateLimited,
    /// 额度耗尽。
    QuotaExceeded,
    /// 服务端错误。
    ServerError,
    /// 未知错误。
    Unknown,
}

impl ProbeFailure {
    /// 对外 JSON 字面量（snake_case），与 DTO 一一对应。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingKey => "missing_key",
            Self::KeyRejected => "key_rejected",
            Self::EndpointUnreachable => "endpoint_unreachable",
            Self::Timeout => "timeout",
            Self::RateLimited => "rate_limited",
            Self::QuotaExceeded => "quota_exceeded",
            Self::ServerError => "server_error",
            Self::Unknown => "unknown",
        }
    }
}

/// 唯一凭据持久化端口；同一实现目标名恒定，删除幂等。
pub trait SecureStore: Send + Sync {
    /// 写入（同名覆盖，不新增条目）。
    fn write(&self, secret: &SecretBytes) -> Result<(), PortError>;
    /// 读取；不存在返回 `None`。
    fn read(&self) -> Result<Option<SecretBytes>, PortError>;
    /// 删除；不存在视为成功。
    fn delete(&self) -> Result<(), PortError>;
}

/// 非敏感配置原子读写端口；损坏/版本不符由实现降级为默认。
pub trait SettingsConfigStore: Send + Sync {
    /// 读取配置。
    fn load(&self) -> Result<SettingsConfig, PortError>;
    /// 原子写入配置。
    fn save(&self, config: &SettingsConfig) -> Result<(), PortError>;
    /// 写回默认 4 键。
    fn reset(&self) -> Result<(), PortError>;
}

/// 唯一出网端口；生产实现复用既有网关。
pub trait ProbeExecutor: Send + Sync {
    /// 执行一次探针。
    fn execute(&self, call: &ProbeCall) -> Result<ProbeAttempt, PortError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = include_str!("ports.rs");

    fn field_count(name: &str) -> usize {
        let needle = format!("pub struct {name} {{");
        let start = SOURCE.find(&needle).expect("struct present");
        let mut inside = false;
        let mut fields = 0usize;
        for line in SOURCE[start..].lines() {
            if !inside {
                inside = true;
                continue;
            }
            if line.trim() == "}" {
                break;
            }
            if line.starts_with("    pub ") {
                fields += 1;
            }
        }
        fields
    }

    #[test]
    fn application_declares_three_ports() {
        for name in [
            "pub trait SecureStore",
            "pub trait SettingsConfigStore",
            "pub trait ProbeExecutor",
        ] {
            assert!(SOURCE.contains(name), "missing {name}");
        }
    }

    #[test]
    fn settings_config_rejects_extra_keys() {
        assert_eq!(field_count("SettingsConfig"), 4);

        let default = SettingsConfig::default();
        assert_eq!(default.schema_version, 1);
        assert_eq!(default.model_index, 0);
        assert_eq!(default.endpoint_index, 0);
        assert_eq!(default.timeout_ms, 6_000);

        let out_of_range = SettingsConfig {
            schema_version: 1,
            model_index: 9,
            endpoint_index: 9,
            timeout_ms: 99,
        }
        .normalized(2, 1);
        assert_eq!(out_of_range.model_index, 0);
        assert_eq!(out_of_range.endpoint_index, 0);
        assert_eq!(out_of_range.timeout_ms, 6_000);

        let upper = SettingsConfig {
            schema_version: 1,
            model_index: 1,
            endpoint_index: 0,
            timeout_ms: 8_000,
        }
        .normalized(2, 1);
        assert_eq!(upper.model_index, 1);
        assert_eq!(upper.timeout_ms, 8_000);
    }

    #[test]
    fn probe_failure_has_exactly_eight_kinds() {
        let kinds = [
            ProbeFailure::MissingKey,
            ProbeFailure::KeyRejected,
            ProbeFailure::EndpointUnreachable,
            ProbeFailure::Timeout,
            ProbeFailure::RateLimited,
            ProbeFailure::QuotaExceeded,
            ProbeFailure::ServerError,
            ProbeFailure::Unknown,
        ];
        assert_eq!(kinds.len(), 8);
        let mut names: Vec<&str> = kinds.iter().map(|kind| kind.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 8);
    }

    #[test]
    fn secret_bytes_debug_is_masked() {
        let secret = SecretBytes::new(b"test-key-plain".to_vec());
        let rendered = format!("{secret:?}");
        assert_eq!(rendered, "SecretBytes(***)");
        assert!(!rendered.contains("test-key-plain"));
    }
}
