//! 启用源注册表（SPEC 8.2-4、6.9-5/10）。
//!
//! 只返回 `enabled && health != Unavailable` 的源；注册表初始为空，
//! 不提供任何兜底源。文件变更后由 [`Registry::refresh`] 在下一次检索生效。

use crate::search::profile_store::{ProfileError, ProfileStore, ProviderEntry};

#[derive(Debug, Default)]
pub struct Registry {
    entries: Vec<ProviderEntry>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_entries(entries: Vec<ProviderEntry>) -> Self {
        Self { entries }
    }

    pub fn entries(&self) -> &[ProviderEntry] {
        &self.entries
    }

    /// 仅启用且健康状态非 `Unavailable`。
    pub fn usable(&self) -> Vec<&ProviderEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.profile.is_usable())
            .collect()
    }

    /// 启用源为 0 即零源模式。
    pub fn is_zero_source(&self) -> bool {
        self.usable().is_empty()
    }

    pub fn replace(&mut self, entries: Vec<ProviderEntry>) {
        self.entries = entries;
    }

    /// 文件变更时整体替换；返回是否发生了重新加载。
    pub fn refresh(&mut self, store: &ProfileStore) -> Result<bool, ProfileError> {
        let entries = store.load_if_changed()?;
        if entries == self.entries {
            return Ok(false);
        }
        self.entries = entries;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::profile_store::parse_document;

    const DOC: &str = r#"{
        "providers": [
            {
                "id": "enabled-healthy", "name": "启用", "kind": "custom",
                "priority": 1, "weight": 50, "enabled": "enabled", "timeout": 3000,
                "rateLimit": {"perMinute": 10, "perDay": 100},
                "adapter": "http_json", "endpoint": "https://a.example.com/api"
            },
            {
                "id": "disabled", "name": "停用", "kind": "custom",
                "priority": 2, "weight": 50, "enabled": "disabled", "timeout": 3000,
                "rateLimit": {"perMinute": 10, "perDay": 100},
                "adapter": "http_json", "endpoint": "https://b.example.com/api"
            },
            {
                "id": "unavailable", "name": "不可用", "kind": "custom",
                "priority": 3, "weight": 50, "enabled": "enabled", "timeout": 3000,
                "rateLimit": {"perMinute": 10, "perDay": 100}, "health": "unavailable",
                "adapter": "http_json", "endpoint": "https://c.example.com/api"
            }
        ]
    }"#;

    #[test]
    fn empty_registry_is_zero_source() {
        let registry = Registry::new();
        assert!(registry.usable().is_empty());
        assert!(registry.is_zero_source());
    }

    #[test]
    fn only_enabled_and_healthy_entries_are_usable() {
        let registry = Registry::from_entries(parse_document(DOC).expect("配置"));
        let usable = registry.usable();
        assert_eq!(usable.len(), 1);
        assert_eq!(usable[0].profile.id.as_str(), "enabled-healthy");
        assert!(!registry.is_zero_source());
    }
}
