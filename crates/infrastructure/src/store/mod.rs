//! 本地存储（SPEC 4.3-3、DESIGN 2.3.1）。
//!
//! 打开前强制校验密钥引用，禁止静默降级为明文库。
//! 页级加密（SQLCipher）需要额外的构建链；当 `sqlcipher` feature 可用时
//! 由 rusqlite 的 SQLCipher 后端接管，否则退化为"密钥门禁 + 字段级加密"。

use meeting_copilot_domain::error::CopilotError;

pub mod repo;

pub use repo::SqliteStore;

pub const SCHEMA_VERSION: u32 = 1;

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

/// 打开参数。`key_ref` 只能是 OS 密钥库引用，不接受明文口令。
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