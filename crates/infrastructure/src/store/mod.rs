//! 本地存储（SPEC 4.3-3、DESIGN 2.3.1）。
//!
//! 打开前强制校验密钥引用，禁止静默降级为明文库。
//! 启用 `sqlcipher` feature 时（`cargo build --features sqlcipher`），由
//! SQLCipher 后端接管页级加密，`SqliteStore::open_with_key` 需要传入 L1 派生密钥；
//! 未启用时退化为"密钥门禁 + 字段级加密"。

use meeting_copilot_domain::error::CopilotError;

use crate::crypto::MASTER_KEY_LEN;

pub mod repo;

pub use repo::SqliteStore;

pub const SCHEMA_VERSION: u32 = 1;

/// L1 派生的 32 字节数据库密钥。仅内存持有，日志与 Debug 输出一律脱敏。
#[derive(Clone, PartialEq, Eq)]
pub struct StoreKey([u8; MASTER_KEY_LEN]);

impl StoreKey {
    pub fn new(bytes: [u8; MASTER_KEY_LEN]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; MASTER_KEY_LEN] {
        &self.0
    }

    /// SQLCipher 原始密钥字面量，使用 64 位十六进制避免口令 KDF 开销。
    #[cfg(feature = "sqlcipher")]
    fn to_hex(&self) -> String {
        self.0.iter().map(|byte| format!("{byte:02X}")).collect()
    }
}

impl std::fmt::Debug for StoreKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("StoreKey(<redacted>)")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StoreError {
    #[error("缺少数据库密钥，拒绝打开（禁止静默降级为明文）")]
    MissingKey,
    #[error("数据库密钥错误")]
    WrongKey,
    #[error("迁移失败：{0}")]
    MigrationFailed(String),
    #[error("数据库操作失败：{0}")]
    Sqlite(String),
    #[error("数据序列化失败：{0}")]
    Serialization(String),
    #[allow(dead_code)]
    #[error("注入的写失败（仅测试）")]
    InjectedFailure,
}

impl From<StoreError> for CopilotError {
    fn from(error: StoreError) -> Self {
        Self::StoreFailure {
            detail: error.to_string(),
        }
    }
}

/// 打开参数。`key_ref` 只能是 OS 密钥库引用，不接受明文口令；
/// 实际密钥在 `SqliteStore::open_with_key` 中单独传入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreConfig {
    pub path: String,
    pub key_ref: Option<String>,
    pub wal_enabled: bool,
}

impl StoreConfig {
    pub fn validate(&self) -> Result<(), StoreError> {
        match self.key_ref.as_deref() {
            None | Some("") => Err(StoreError::MissingKey),
            Some(value) if value.starts_with("dpapi://") || value.starts_with("keychain://") => {
                Ok(())
            }
            Some(_) => Err(StoreError::MissingKey),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(key_ref: Option<&str>) -> StoreConfig {
        StoreConfig {
            path: "meeting.db".into(),
            key_ref: key_ref.map(str::to_owned),
            wal_enabled: true,
        }
    }

    #[test]
    fn refuses_to_open_without_key() {
        assert_eq!(config(None).validate(), Err(StoreError::MissingKey));
        assert_eq!(config(Some("")).validate(), Err(StoreError::MissingKey));
        assert_eq!(
            config(Some("plaintext-password")).validate(),
            Err(StoreError::MissingKey)
        );
    }

    #[test]
    fn accepts_keychain_reference() {
        assert!(config(Some("keychain://meeting-copilot/db"))
            .validate()
            .is_ok());
    }
}
